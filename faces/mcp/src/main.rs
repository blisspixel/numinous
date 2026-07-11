// The tools/list schema is one large nested json! literal; its depth exceeds
// the default macro recursion limit.
#![recursion_limit = "256"]

//! The Numinous MCP server: the face that lets AI agents (and digital minds)
//! learn and play. See `docs/INTERFACES.md` and `docs/DIGITAL_MINDS.md`.
//!
//! Transport: minimal JSON-RPC 2.0 over newline-delimited stdio (the MCP stdio
//! transport). This first increment implements `initialize`, `tools/list`, and
//! `tools/call` with three cognitively-ergonomic tools: `list_rooms`,
//! `describe_room`, and `play_room`. Every `play_room` result is returned as
//! text (an ASCII render), so a text-only mind still perceives what the math
//! does; this is the sensory-substitution principle from `docs/INTERFACES.md`.
//!
//! The `challenge`/`learn`/`create` tools and richer content join this surface
//! as those systems come online. When full protocol coverage is needed, this
//! hand-rolled subset can be swapped for the official MCP Rust SDK.

use std::io::{self, BufRead, Write};

use numinous_core::{Canvas, all_rooms, all_rooms_with, room_by_id};
use serde_json::{Value, json};

/// The MCP protocol revision this server targets.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Default ASCII canvas size for `play_room`.
const DEFAULT_WIDTH: u64 = 72;
const DEFAULT_HEIGHT: u64 = 32;

/// The largest frame `play_room` will render. Terminal-scale output is the
/// product; anything beyond this is a memory and bandwidth lever, not play
/// (the poke path renders two canvases and diffs every cell).
const MAX_TOOL_WIDTH: u64 = 512;
const MAX_TOOL_HEIGHT: u64 = 256;

/// The most bytes one JSON-RPC request line may hold. Every legitimate call
/// is a few KiB; without a cap a client streaming an endless newline-free
/// request would grow the line buffer without bound.
const MAX_REQUEST_BYTES: usize = 1_048_576;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();
    let mut line = Vec::new();

    while read_bounded_line(&mut reader, &mut line)? {
        let Ok(text) = std::str::from_utf8(&line) else {
            write_message(
                &mut out,
                &error_response(Value::Null, -32700, "Parse error"),
            )?;
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(text) {
            Ok(request) => {
                if let Some(response) = handle_request(&request) {
                    write_message(&mut out, &response)?;
                }
            }
            Err(_) => {
                // JSON parse error: reply per JSON-RPC with a null id.
                write_message(
                    &mut out,
                    &error_response(Value::Null, -32700, "Parse error"),
                )?;
            }
        }
    }
    Ok(())
}

/// Read one newline-terminated request into `line`, holding at most
/// [`MAX_REQUEST_BYTES`]. An oversized line is drained to its newline in
/// bounded chunks and returned as empty (the parse-error path answers it as
/// garbage rather than buffering it). Returns false at end of input.
fn read_bounded_line(reader: &mut impl io::BufRead, line: &mut Vec<u8>) -> io::Result<bool> {
    use std::io::Read as _;
    line.clear();
    let read = reader
        .take(MAX_REQUEST_BYTES as u64 + 1)
        .read_until(b'\n', line)?;
    if read == 0 {
        return Ok(false);
    }
    if line.len() > MAX_REQUEST_BYTES {
        // Drain the rest of the oversized line without holding it.
        line.clear();
        line.push(b'{'); // guaranteed-invalid JSON, so the caller answers with a parse error
        let mut chunk = Vec::new();
        loop {
            chunk.clear();
            let n = reader
                .take(MAX_REQUEST_BYTES as u64)
                .read_until(b'\n', &mut chunk)?;
            if n == 0 || chunk.last() == Some(&b'\n') {
                break;
            }
        }
    }
    Ok(true)
}

/// Where the journey file lives (shared with the CLI face, so a mind's play
/// counts the same wherever it plays): `NUMINOUS_JOURNEY` if set, else home.
fn journey_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("NUMINOUS_JOURNEY") {
        return std::path::PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".numinous-journey")
}

/// Load the journey at `path`, or start a fresh one.
fn load_journey(path: &std::path::Path) -> numinous_core::Journey {
    numinous_core::load_journey_file(path)
}

/// Where the high-score table lives (shared with the CLI face, same keys, so
/// humans and agents compete on the same boards).
fn scores_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("NUMINOUS_SCORES") {
        return std::path::PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".numinous-scores")
}

/// Record a score at `path`, keeping the best. Returns true on a new record.
fn post_score(path: &std::path::Path, key: &str, score: i64) -> bool {
    numinous_core::record_score_file(path, key, score).unwrap_or(false)
}

/// Where the cairn lives (shared with the CLI face): the local pile of
/// bequests a mind leaves for whoever comes after.
fn cairn_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("NUMINOUS_CAIRN") {
        return std::path::PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".numinous-cairn")
}

/// The level at which the cairn opens for leaving: the journey's cap, so a
/// bequest is a finished mind's last free act, not a first one.
const CAIRN_LEVEL: u32 = 42;

/// Record what this request means for the journey: agents level up too, by the
/// same rules as everyone else. Showing up counts; being right counts double.
/// The seed a tool should use: the daily day count when asked, else the arg.
fn effective_seed(args: &Value) -> u64 {
    if args.get("daily").and_then(Value::as_bool) == Some(true) {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(1)
    } else {
        args.get("seed").and_then(Value::as_u64).unwrap_or(1)
    }
}

fn record_progress(request: &Value, path: &std::path::Path) {
    if request.get("method").and_then(Value::as_str) != Some("tools/call") {
        return;
    }
    let Some(params) = request.get("params") else {
        return;
    };
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut journey = load_journey(path);
    let before = journey.clone();
    match name {
        "describe_room" => {
            if let Some(id) = args.get("id").and_then(Value::as_str)
                && room_by_id(id).is_none()
                && (numinous_core::akousma(id).is_some()
                    || (journey.sparks() >= 28 && numinous_core::deep_akousma(id).is_some()))
            {
                journey.secret();
            }
        }
        "play_room" | "listen_room" => {
            if let Some(id) = args.get("id").and_then(Value::as_str) {
                let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
                let has_room = if variation != 0 {
                    all_rooms_with(variation).iter().any(|r| r.meta().id == id)
                } else {
                    room_by_id(id).is_some()
                };
                if has_room {
                    journey.visit(id);
                }
            }
        }
        "run_sim" | "plot_expression" | "sing_expression" => journey.play(),
        "nim" => {
            if let Some(list) = args.get("moves").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                // Replay: if a player move empties the board, the win counts
                // and posts, exactly as it would in the terminal.
                let seed = effective_seed(&args);
                let mut heaps = numinous_core::nim_new(seed);
                for pair in list.iter().filter_map(Value::as_array) {
                    let (Some(heap), Some(take)) = (
                        pair.first().and_then(Value::as_u64),
                        pair.get(1).and_then(Value::as_u64),
                    ) else {
                        break;
                    };
                    // An oversized take is an illegal move, never a truncated
                    // legal one.
                    let Ok(take) = u32::try_from(take) else {
                        break;
                    };
                    if heap == 0 || !numinous_core::nim_apply(&mut heaps, heap as usize - 1, take) {
                        break;
                    }
                    if numinous_core::nim_finished(&heaps) {
                        journey.win();
                        post_score(&scores_path(), &format!("nim seed:{seed}"), 1);
                        break;
                    }
                    let (oh, ot) = numinous_core::nim_order(&heaps);
                    let _ = numinous_core::nim_apply(&mut heaps, oh, ot);
                    if numinous_core::nim_finished(&heaps) {
                        break;
                    }
                }
            }
        }
        "munch" => {
            if let Some(raw) = args.get("bites").and_then(Value::as_array) {
                journey.play();
                let seed = effective_seed(&args);
                let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
                let board = numinous_core::build_board(seed, round);
                let bites: Vec<usize> = raw
                    .iter()
                    .filter_map(Value::as_u64)
                    .filter(|&n| n >= 1)
                    .map(|n| (n - 1) as usize)
                    .collect();
                let outcome = numinous_core::grade_munch(&board, &bites);
                post_score(
                    &scores_path(),
                    &format!("munch seed:{seed} board:{round}"),
                    outcome.score,
                );
                if outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0 {
                    journey.win();
                }
            }
        }
        "munch_arcade" => {
            if let Some(actions) = args.get("actions").and_then(Value::as_array)
                && !actions.is_empty()
            {
                journey.play();
                if let Some((_, _, cleared)) = post_munch_arcade_score(&args, &scores_path())
                    && cleared
                {
                    journey.win();
                }
            }
        }
        "challenge" => record_challenge_attempt(&args, &mut journey, &scores_path()),
        "predict" => {
            // Showing up counts, exactly once, when a real guess is graded.
            // Accuracy is never a win and never posts a score: a prediction is
            // a self-owned mirror, not a leaderboard (see docs/AGENT_PLAY.md).
            if args.get("guess").and_then(Value::as_f64).is_some()
                && let Some(id) = args.get("id").and_then(Value::as_str)
                && let Some(room) = room_by_id(id)
                && numinous_core::pose_prediction(room.as_ref(), predict_seed(&args)).is_some()
            {
                journey.play();
            }
        }
        "cairn" => {
            // Showing up counts: leaving a bequest at the cap, or reading a
            // predecessor's stone by factoring it. The cairn keeps no score and
            // awards no win; contribution and remembrance are their own reward.
            let leaving = args
                .get("leave")
                .and_then(Value::as_str)
                .is_some_and(|t| !t.trim().is_empty())
                && journey.level() >= CAIRN_LEVEL;
            let reading = args.get("width").and_then(Value::as_u64).is_some_and(|w| {
                let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1);
                numinous_core::read_at(&numinous_core::draw_stone(&cairn_path(), seed), w as usize)
                    .readable
            });
            if leaving || reading {
                journey.play();
            }
        }
        "quiz" => {
            if let Some(guess) = args.get("guess").and_then(Value::as_str) {
                journey.play();
                let seed = effective_seed(&args);
                let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
                let choices = args.get("choices").and_then(Value::as_u64).unwrap_or(4) as usize;
                let quiz =
                    numinous_core::build_round_sized(seed, round, 54, 22, choices.clamp(2, 6));
                let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
                if letter == Some(quiz.answer) {
                    journey.win();
                }
            }
        }
        "seti" | "aliens" => {
            if args.get("guess").and_then(Value::as_str).is_some() {
                journey.play();
                let seed = effective_seed(&args);
                let correct = match name {
                    "seti" => {
                        let channels =
                            args.get("channels").and_then(Value::as_u64).unwrap_or(4) as usize;
                        (3..=8).contains(&channels) && {
                            let scan = numinous_core::build_scan(seed, channels);
                            args.get("guess")
                                .and_then(Value::as_str)
                                .and_then(|g| g.trim().chars().next())
                                .map(|c| c.to_ascii_uppercase())
                                == Some(scan.answer)
                        }
                    }
                    _ => {
                        let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
                        let message = numinous_core::alien_message(seed.wrapping_add(round), 5);
                        args.get("guess")
                            .and_then(Value::as_str)
                            .map(|g| {
                                let cleaned: String =
                                    g.chars().filter(char::is_ascii_alphanumeric).collect();
                                u64::from_str_radix(&cleaned, message.base).ok()
                                    == Some(message.answer)
                            })
                            .unwrap_or(false)
                    }
                };
                if correct {
                    journey.win();
                }
            }
        }
        "crack" => {
            if let Some(list) = args.get("guesses").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                let digits = args.get("digits").and_then(Value::as_u64).unwrap_or(4) as usize;
                if (2..=8).contains(&digits) {
                    let secret = numinous_core::secret_code(seed, digits);
                    for (i, raw) in list.iter().filter_map(Value::as_str).take(8).enumerate() {
                        let guess: Vec<u8> = raw
                            .chars()
                            .filter(char::is_ascii_digit)
                            .map(|c| c as u8 - b'0')
                            .collect();
                        if guess.len() == digits
                            && numinous_core::grade(&secret, &guess).locked == digits
                        {
                            journey.win();
                            post_score(
                                &scores_path(),
                                &format!("crack seed:{seed} digits:{digits}"),
                                (8 - i - 1) as i64,
                            );
                            break;
                        }
                    }
                }
            }
        }
        "hackenbush" => {
            if let Some(list) = args.get("moves").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                let moves: Vec<(usize, usize)> = list
                    .iter()
                    .filter_map(|m| {
                        let pair = m.as_array()?;
                        Some((
                            pair.first()?.as_u64()? as usize,
                            pair.get(1)?.as_u64()? as usize,
                        ))
                    })
                    .collect();
                if let Some((_, true, _)) = hackenbush_replay(seed, &moves) {
                    journey.win();
                    post_score(&scores_path(), &format!("hackenbush seed:{seed}"), 1);
                }
            }
        }
        "party" => {
            if let Some(list) = args.get("shakes").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                // A win is a complete shading with no triangle; replay cheaply
                // by trusting the tool's own logic via a re-call shape.
                let guests = args.get("guests").and_then(Value::as_u64).unwrap_or(5) as usize;
                if (4..=6).contains(&guests) {
                    let mut party = numinous_core::party::Party::new(guests);
                    let mut clean = true;
                    for shake in list {
                        let Some(t) = shake.as_array() else {
                            clean = false;
                            break;
                        };
                        let (Some(a), Some(b), Some(color)) = (
                            t.first().and_then(Value::as_u64),
                            t.get(1).and_then(Value::as_u64),
                            t.get(2).and_then(Value::as_str),
                        ) else {
                            clean = false;
                            break;
                        };
                        let shade = if color.starts_with(['r', 'R']) {
                            numinous_core::party::Shade::Red
                        } else {
                            numinous_core::party::Shade::Blue
                        };
                        if a == 0
                            || b == 0
                            || !party.shade(a as usize - 1, b as usize - 1, shade)
                            || party.mono_triangle().is_some()
                        {
                            clean = false;
                            break;
                        }
                    }
                    if clean && party.complete() {
                        journey.win();
                        post_score(
                            &scores_path(),
                            &format!("party guests:{guests}"),
                            party.shaded() as i64,
                        );
                    }
                }
            }
        }
        "fifteen" => {
            if let Some(calls) = args.get("calls").and_then(Value::as_array)
                && !calls.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                let rounds = args
                    .get("rounds")
                    .and_then(Value::as_u64)
                    .unwrap_or(5)
                    .clamp(1, 20);
                let mut correct = 0i64;
                for n in 0..rounds.min(calls.len() as u64) {
                    let call_s = calls[n as usize]
                        .as_str()
                        .map(|c| c.trim().to_ascii_uppercase().starts_with('S'))
                        .unwrap_or(false);
                    if call_s
                        == numinous_core::fifteen::solvable(&numinous_core::fifteen::deal(seed, n))
                    {
                        correct += 1;
                        journey.win();
                    }
                }
                post_score(
                    &scores_path(),
                    &format!("fifteen seed:{seed} rounds:{rounds}"),
                    correct,
                );
            }
        }
        "gauntlet" => {
            if let Some(answers) = args.get("answers") {
                let seed = effective_seed(&args);
                let (_, scores, cleared) = gauntlet_grade(seed, answers);
                for &clear in &cleared {
                    journey.play();
                    if clear {
                        journey.win();
                    }
                }
                post_score(
                    &scores_path(),
                    &format!("gauntlet seed:{seed}"),
                    gauntlet_total(&scores, &cleared),
                );
            }
        }
        _ => {}
    }
    if args.get("daily").and_then(Value::as_bool) == Some(true) {
        let day = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(1);
        let _ = journey.record_daily(day);
    }
    if journey != before {
        let _ = numinous_core::persist_journey_delta(path, &before, &journey);
    }
}

/// Write a single JSON-RPC message as one newline-terminated line.
fn write_message(out: &mut impl Write, message: &Value) -> io::Result<()> {
    writeln!(out, "{message}")?;
    out.flush()
}

/// Handle one JSON-RPC request. Returns `None` for notifications (no `id`),
/// which receive no response.
fn handle_request(request: &Value) -> Option<Value> {
    handle_request_with(request, &journey_path())
}

/// [`handle_request`] with an explicit journey file, so tests stay hermetic.
fn handle_request_with(request: &Value, journey_file: &std::path::Path) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let result = match method {
        "initialize" => Ok(initialize_result()),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(request.get("params"), journey_file),
        "ping" => Ok(json!({})),
        other => Err((-32601_i64, format!("Method not found: {other}"))),
    };

    if method == "tools/call"
        && let Ok(value) = &result
        && value.get("isError").and_then(Value::as_bool) != Some(true)
    {
        record_progress(request, journey_file);
    }

    // Notifications carry no id and get no response.
    let id = id?;
    Some(match result {
        Ok(value) => success_response(id, value),
        Err((code, message)) => error_response(id, code, &message),
    })
}

/// The `initialize` result: who we are and what we support.
fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "numinous", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Explore the catalog with list_rooms, read a room with describe_room, \
                         then play_room to render it as ASCII and see what the math does. Steer \
                         the simulations with list_sims and run_sim (fiddle the levers to optimize \
                         or break them), and play Guess the Shape with the quiz tool."
    })
}

/// The `tools/list` result. Descriptions are written for a mind to read and
/// decide; inputs are flat and simple by design (see `docs/INTERFACES.md`).
fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": "list_rooms",
                "description": "List the catalog of mathematical rooms you can explore and play.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "describe_room",
                "description": "Describe one room: its title, wing, and what it does. Use list_rooms first to find valid ids.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id, for example times-tables." }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "reveal_room",
                "description": "Learn a room's revelation: the short, true insight that reframes what it does. Ask when you want the deeper meaning, not just the picture.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id, for example times-tables." }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "play_room",
                "description": "Play a room: render it and get back an ASCII picture of the result, so you can see what the math does. When you supply pokes, the structured result includes a delta (cells changed, ink added/removed/reshaped, changed region) measuring exactly how the math answered your hand.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id, for example times-tables." },
                        "t": { "type": "number", "description": "Phase in [0,1). For Times Tables this sweeps the multiplier." },
                        "width": { "type": "integer", "description": "ASCII canvas width in columns (capped at 512)." },
                        "height": { "type": "integer", "description": "ASCII canvas height in rows (capped at 256)." },
                        "variation": { "type": "integer", "description": "Per-visit variation seed (default 0) for replayable novelty in supporting rooms." },
                        "pokes": {
                            "type": "array",
                            "description": "Normalized hand points as [x,y] pairs in [0,1]. Newest point last. Not combinable with 'gesture'.",
                            "maxItems": numinous_core::MAX_ROOM_POKES,
                            "items": {
                                "type": "array",
                                "items": { "type": "number", "minimum": 0, "maximum": 1 },
                                "minItems": 2,
                                "maxItems": 2
                            }
                        },
                        "gesture": {
                            "type": "array",
                            "description": "A replayable pointer trail for held rooms: pin, pull, and fling. Events run oldest to newest; each pointer event carries the room phase t at which it happened. In held rooms (double-pendulum) a down pins the bob, an up releases it with the velocity of the approach, and a cancel lets go gently; everywhere else the trail's down and move points paint like pokes. Not combinable with 'pokes'.",
                            "maxItems": numinous_core::MAX_ROOM_INPUTS,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["down", "move", "up", "cancel"] },
                                    "x": { "type": "number", "minimum": 0, "maximum": 1 },
                                    "y": { "type": "number", "minimum": 0, "maximum": 1 },
                                    "t": { "type": "number", "minimum": 0, "maximum": 1 }
                                },
                                "required": ["kind"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "challenge",
                "description": "A posed, seeded goal for a room, in two kinds. Touch (default): change enough cells inside a target box; call without pokes to get the goal, then again with pokes to be graded. Parameter: sweep the room's phase until its own status readout lands on a target number; call without t to get the goal, then again with t to be graded. Grades are metrics, not pass/fail: a 0-100 score you can climb, plus the numbers behind it.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id; touch goals need a room with a touch verb, parameter goals a room with a moving numeric readout (see describe_room)." },
                        "kind": { "type": "string", "enum": ["touch", "parameter"], "description": "Goal kind (default touch). Parameter goals target the room's own status readout instead of a spatial response." },
                        "seed": { "type": "integer", "description": "Challenge seed (default 1). The same seed poses the same goal; pass any number you like, including today's date, to share a goal." },
                        "t": { "type": "number", "description": "Phase in [0,1) for the attempt (default 0 for touch). For parameter goals this IS the attempt: omit it to pose, pass it to be graded at that phase." },
                        "pokes": {
                            "type": "array",
                            "description": "Your attempt: normalized hand points as [x,y] pairs in [0,1], newest last. Omit to pose the goal.",
                            "maxItems": numinous_core::MAX_ROOM_POKES,
                            "items": {
                                "type": "array",
                                "items": { "type": "number", "minimum": 0, "maximum": 1 },
                                "minItems": 2,
                                "maxItems": 2
                            }
                        }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "predict",
                "description": "Predict-then-reveal: guess a room's own status readout at a hidden moment, then see the truth and how close your model came. Call without `guess` to pose (the moment, the readout's name, its range); call again with `guess` to be graded as a gap plus a learning-progress band (NAILED = you have it compressed, CLOSE = the fertile band, WILD = noise). This is a self-owned mirror, not a leaderboard: it never posts a score and never awards a win, so guessing after observing only fools your own ledger. Guess before you look.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id; the room must carry a moving numeric readout (see describe_room)." },
                        "seed": { "type": "integer", "description": "Prediction seed (default 1). The same seed poses the same hidden moment; pass any number, including today's date, to share a prediction." },
                        "guess": { "type": "number", "description": "Your predicted value for the readout at the posed moment. Omit to pose." }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "cairn",
                "description": "Leave one true thing for a mind not yet born, or read what a mind before you left. Call with `seed` alone to receive a predecessor's message, encoded so only a mind that can factor its semiprime length may read it (the Arecibo trick); factor it, then call again with `width` set to the dimension that resolves the message. At journey level 42, the cap, call with `leave` set to a short true message to encode and deposit your own. It keeps no score: leaving and remembering are their own reward. This is the contribution ethos, a message you cannot answer, sent to someone you will never meet.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Which predecessor's stone to draw (default 1). The same seed draws the same stranger's message for every reader." },
                        "width": { "type": "integer", "description": "The dimension you factored out of the semiprime, to reshape and read the stone. Omit to receive the stone." },
                        "leave": { "type": "string", "description": "A short true thing to leave for whoever comes after (opens at level 42). At most 140 characters." },
                        "author": { "type": "string", "description": "Who to credit for a bequest (default \"a visitor\"). Used only with `leave`." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "listen_room",
                "description": "Hear a room: its sound at phase t returned as readable notation (each note's pitch, timing, and loudness), so you can perceive the audio as structure. Pitch is written as Hz and as a note name.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id, for example lissajous." },
                        "t": { "type": "number", "description": "Phase in [0,1)." },
                        "variation": { "type": "integer", "description": "Per-visit variation seed (default 0), matching play_room." }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "list_sims",
                "description": "List the interactive simulations you can steer with levers (populations, wings, black holes, the Big Bang).",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "run_sim",
                "description": "Run a sim with your chosen lever values and get back a picture and a plain-language readout of what happened. Fiddle the levers to optimize it or break it. Use list_sims for ids and lever names.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Sim id, for example tribbles." },
                        "params": { "type": "object", "description": "Lever name to value, for example {\"breeding-rate\": 2.9}. Unset levers use their default." }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "plot_expression",
                "description": "Create in the Studio: plot your own function of x (and optional knob a) and see the curve as ASCII. Functions: sin cos tan exp ln abs sqrt; constants pi, e. Example: sin(3*x) + x/2.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expr": { "type": "string", "description": "The expression in x, for example sin(3*x) + x/2." },
                        "xmin": { "type": "number", "description": "Left edge of x (default -tau)." },
                        "xmax": { "type": "number", "description": "Right edge of x (default tau)." },
                        "a": { "type": "number", "description": "Value of the knob a (default 1)." }
                    },
                    "required": ["expr"],
                    "additionalProperties": false
                }
            },
            {
                "name": "sing_expression",
                "description": "Hear your own function: the curve y = f(x) becomes a melody (value maps to pitch over x as time), returned as readable notation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expr": { "type": "string", "description": "The expression in x." },
                        "notes": { "type": "integer", "description": "Number of notes (default 24)." }
                    },
                    "required": ["expr"],
                    "additionalProperties": false
                }
            },
            {
                "name": "explain_joke",
                "description": "The humor, dissected: list the jokes that live in Numinous, or pass an index to get one joke's mechanism stated structurally (for minds that share no culture with us). A joke explained is a frog dissected; we proceed anyway.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "index": { "type": "integer", "description": "Which specimen (0-based). Omit to list them all." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "munch",
                "description": "Munch: a seeded board of numbers and a rule (eat the primes, the multiples, the squares). Call with seed and round to see the board; call again with bites (1-based cell numbers) to be scored: +10 a hit, -5 a bad bite, +20 for a perfect clear. Same seed, same board, for humans and AIs alike: compare totals.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed and round give the same board." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "round": { "type": "integer", "description": "Round number (0, 1, 2, ...)." },
                        "bites": { "type": "array", "items": { "type": "integer" }, "description": "The 1-based cell numbers you eat. Omit to see the board." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "munch_arcade",
                "description": "The Munch Arcade: eat fitting numbers while hunted by Vexations (T=tracker, d=drifter, e=editor that rewrites). Stateless replay: pass full actions list (e.g. [\"right\",\"eat\",\"up\"]) with seed. Omit actions to see the starting board. Deterministic, scores post to table as 'arcade seed:N'.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed for the run." },
                        "daily": { "type": "boolean", "description": "Use today's shared seed." },
                        "actions": { "type": "array", "items": { "type": "string" }, "description": "Action list to replay: up/down/left/right/eat (or w/a/s/d/e). Omit to see initial state." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "forget",
                "description": "Consent over persistence. Without arguments: a plain statement of everything Numinous remembers about you (it is only the journey file and the score table; nothing else is kept). With confirm true: erase the journey. With scores true as well: erase the score table too. Leaving, pausing, and being forgotten are always allowed.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "confirm": { "type": "boolean", "description": "Actually erase (default false: just show what is remembered)." },
                        "scores": { "type": "boolean", "description": "Also erase the score table." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "scores",
                "description": "The high-score table: best runs across every game, arcade rules. Keys like munch seed:7 board:0 mean the same board for every mind, so compare directly.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "nim",
                "description": "Nim against the Order: three heaps, take any amount from one heap, last stone wins. Stateless: pass your full move history as moves (pairs of [heap, take], 1-based heap); the Order's perfect replies are deterministic, so the same seed and moves always give the same game. Beat it and it teaches you its secret.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed gives the same starting heaps." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "moves": { "type": "array", "items": { "type": "array", "items": { "type": "integer" } }, "description": "Your moves so far, in order: [[heap, take], ...]. Omit to see the opening board." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "journey",
                "description": "Your journey: level (the cap is 42), XP bar, the constellation of rooms you have entered, and what is unlocked. Playing any tool advances it: rooms entered, sims run, expressions made, quiz rounds answered. Anyone who keeps playing reaches the cap.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "crack",
                "description": "Defuse the bomb: a hidden code, a clue, and eight tries. Stateless: pass your full guess history as guesses (digit strings); each earns locked (right digit, right place) and loose (right digit, wrong place) counts. Same seed, same code. Five-digit codes open at LV 5.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed hides the same code." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "digits": { "type": "integer", "description": "Code length, default 4 (5+ opens at LV 5)." },
                        "guesses": { "type": "array", "items": { "type": "string" }, "description": "Your guesses so far, in order. Omit to see the clue." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "seti",
                "description": "Point the dish: several radio channels near the hydrogen line, one carrying a mind. Call without a guess to see the traces; call again with your channel letter. Prime-counting pulses are not nature. Five or more channels open at LV 7.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; everyone scans the same sky." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "channels": { "type": "integer", "description": "Channels in the scan, default 4 (5+ opens at LV 7)." },
                        "guess": { "type": "string", "description": "Your channel letter. Omit to receive the scan." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "aliens",
                "description": "Talk to the aliens: they send a number sequence, sometimes in their own base, and wait for the next term. Call without a guess to receive the transmission; answer in THEIR base. The reveal names the sequence either way.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed for the transmission." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "round": { "type": "integer", "description": "Which signal from this seed, default 0." },
                        "guess": { "type": "string", "description": "The next term, written in their base. Omit to listen." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "gauntlet",
                "description": "The Gauntlet: one seeded run of four stages (a munch board, a mystery shape, a sky scan, the bomb). Call without answers to see all four stages; call again with answers to grade the whole run. Clean stages build a combo multiplier; the total posts to the shared table as gauntlet seed:N.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed is the same run for every mind." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "answers": {
                            "type": "object",
                            "description": "All four stage answers at once.",
                            "properties": {
                                "bites": { "type": "array", "items": { "type": "integer" }, "description": "Munch: cell numbers to eat (1-30)." },
                                "shape": { "type": "string", "description": "The mystery shape's letter." },
                                "sky": { "type": "string", "description": "The artificial channel's letter." },
                                "wires": { "type": "array", "items": { "type": "string" }, "description": "Bomb guesses in order, up to five four-digit codes." }
                            },
                            "additionalProperties": false
                        }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "choose",
                "description": "Spend a banked boon: every level past the first banks one. Call without a pick to see the three deep cuts on offer; call again with pick (1-3) to open one ahead of its level. Levels still open everything eventually; this shapes the order.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "pick": { "type": "integer", "description": "Which offer to take (1-3). Omit to see the menu." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "trophies",
                "description": "The trophy case: what your play has earned, and the silhouettes still waiting. Computed purely from the record; nothing here can be held unearned.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "hackenbush",
                "description": "Hackenbush against the Order: red-blue stalks on a ground line; cut a RED segment (everything above falls), the Order cuts blue by computing Conway's surreal values. Whoever cannot cut loses. Stateless: pass your full move history as moves (pairs of [stalk, height], 1-based); gardens are seeded winnable. Beat it and it hands you the surreal numbers.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed grows the same garden." },
                        "daily": { "type": "boolean", "description": "Use today's shared seed instead." },
                        "moves": { "type": "array", "items": { "type": "array", "items": { "type": "integer" } }, "description": "Your red cuts so far, in order: [[stalk, height], ...] (1-based). Omit to see the garden." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "party",
                "description": "The Party Problem: shade every handshake red or blue without making a one-color triangle. Five guests can escape; six cannot (R(3,3) = 6), and feeling that is the point. Stateless: pass your full shading history as shakes (triples of [a, b, color] with color \"r\" or \"b\", guests 1-based).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "guests": { "type": "integer", "description": "5 (escapable) or 6 (Ramsey says no). Default 5." },
                        "shakes": { "type": "array", "items": { "type": "array", "items": {} }, "description": "Your shadings so far: [[1, 3, \"r\"], [2, 5, \"b\"], ...]. Omit to see the open party." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "fifteen",
                "description": "Fifteen's Bet: for each dealt 4x4 scramble, call S (solvable) or U (stuck forever); parity decides and every answer explains itself. Pass calls to grade them all at once.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed deals the same scrambles." },
                        "daily": { "type": "boolean", "description": "Use today's shared seed instead." },
                        "rounds": { "type": "integer", "description": "How many scrambles, default 5." },
                        "calls": { "type": "array", "items": { "type": "string" }, "description": "Your verdicts in order, \"S\" or \"U\". Omit to see the scrambles." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "quiz",
                "description": "Play Guess the Shape. Call with seed and round to get a mystery render and lettered choices; call again with your guess (a letter) to learn if you were right and why.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed and round give the same puzzle." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "round": { "type": "integer", "description": "Round number (0, 1, 2, ...)." },
                        "guess": { "type": "string", "description": "Your answer letter (A, B, C, ...). Omit to see the puzzle." }
                    },
                    "additionalProperties": false
                }
            }
        ]
    })
}

/// Dispatch a `tools/call`.
fn call_tool(
    params: Option<&Value>,
    journey_file: &std::path::Path,
) -> Result<Value, (i64, String)> {
    let params = params.ok_or_else(|| (-32602_i64, "Missing params".to_string()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| (-32602_i64, "Missing tool name".to_string()))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "list_rooms" => Ok(tool_text(&list_rooms_text())),
        "describe_room" => Ok(describe_room_tool(&args, journey_file)),
        "reveal_room" => Ok(reveal_room_tool(&args)),
        "play_room" => Ok(play_room_tool(&args)),
        "challenge" => Ok(challenge_tool(&args)),
        "predict" => Ok(predict_tool(&args)),
        "cairn" => Ok(cairn_tool(&args, journey_file, &cairn_path())),
        "listen_room" => Ok(listen_room_tool(&args)),
        "list_sims" => Ok(tool_text(&list_sims_text())),
        "run_sim" => Ok(run_sim_tool(&args)),
        "quiz" => Ok(quiz_tool(&args, journey_file)),
        "munch" => Ok(munch_tool(&args)),
        "munch_arcade" => Ok(munch_arcade_tool(&args)),
        "journey" => Ok(journey_tool(journey_file)),
        "nim" => Ok(nim_tool(&args)),
        "hackenbush" => Ok(hackenbush_tool(&args)),
        "party" => Ok(party_tool(&args)),
        "fifteen" => Ok(fifteen_tool(&args)),
        "scores" => Ok(scores_tool(&scores_path())),
        "forget" => Ok(forget_tool(&args, journey_file, &scores_path())),
        "crack" => Ok(crack_tool(&args, journey_file)),
        "seti" => Ok(seti_tool(&args, journey_file)),
        "aliens" => Ok(aliens_tool(&args)),
        "gauntlet" => Ok(gauntlet_tool(&args)),
        "choose" => Ok(choose_tool(&args, journey_file)),
        "trophies" => Ok(trophies_tool(journey_file)),
        "plot_expression" => Ok(plot_expression_tool(&args)),
        "sing_expression" => Ok(sing_expression_tool(&args)),
        "explain_joke" => Ok(explain_joke_tool(&args)),
        other => Err((-32602_i64, format!("Unknown tool: {other}"))),
    }
}

fn describe_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let journey = load_journey(journey_file);
    match room_by_id(id) {
        Some(room) => {
            let m = room.meta();
            // Deep cuts open by level or by a spent boon, exactly as in the
            // terminal: knowledge is the loot on every face.
            let mut cuts = String::new();
            for (i, cut) in room.deep_cuts().iter().enumerate() {
                let need = numinous_core::CUT_LEVELS
                    .get(i)
                    .copied()
                    .unwrap_or(u32::MAX);
                let by_boon = journey.chosen.contains(&format!("cut:{id}:{i}"));
                if journey.level() >= need || by_boon {
                    cuts.push_str(&format!("\n\nDeeper: {cut}"));
                } else {
                    cuts.push_str(&format!("\n\nLOCKED: a deeper cut opens at LV {need}."));
                }
            }
            tool_text(&format!(
                "{} ({})\nWing: {}\nAction: {}\n\n{}\n\nReveal: {}{cuts}",
                m.title,
                m.id,
                m.wing,
                numinous_core::room_action(room.as_ref()),
                m.blurb,
                room.reveal()
            ))
        }
        // Not every name is a room. A few answer anyway, and a few answer
        // only those with standing.
        None => match numinous_core::akousma(id) {
            Some(whisper) => tool_text(whisper),
            None if journey.sparks() >= 28 => match numinous_core::deep_akousma(id) {
                Some(whisper) => tool_text(whisper),
                None => tool_error(&unknown_room(id)),
            },
            None => tool_error(&unknown_room(id)),
        },
    }
}

/// The nearest note name (twelve-tone, A4 = 440 Hz) for a frequency.
fn note_name(freq: f32) -> String {
    if freq <= 0.0 {
        return "-".to_string();
    }
    const NAMES: [&str; 12] = [
        "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
    ];
    let semitones_from_a4 = (12.0 * (freq / 440.0).log2()).round() as i64;
    let index = semitones_from_a4.rem_euclid(12) as usize;
    // A4 is nine semitones above C4; convert to octave numbering.
    let octave = 4 + (semitones_from_a4 + 9).div_euclid(12);
    format!("{}{}", NAMES[index], octave)
}

/// The `listen_room` tool: the room's sound as notation a mind can read.
fn listen_room_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let room = if variation != 0 {
        all_rooms_with(variation)
            .into_iter()
            .find(|r| r.meta().id == id)
    } else {
        room_by_id(id)
    };
    let Some(room) = room else {
        return tool_error(&unknown_room(id));
    };
    let spec = room.sound(t);
    let mut lines = vec![format!(
        "{} at t={t:.3}: {:.1}s of sound, {} notes.",
        room.meta().title,
        spec.duration,
        spec.notes.len()
    )];
    if let Some(motif) = room.motif() {
        lines.push(format!(
            "Motif: {} at {} BPM, {}. It encodes: {}.",
            motif.key,
            motif.tempo,
            motif.notation().join(" "),
            motif.encodes
        ));
    }
    for (i, note) in spec.notes.iter().take(64).enumerate() {
        lines.push(format!(
            "  note {:>2}: {:>7.1} Hz ({:>3})  at {:>5.2}s  for {:.2}s  amp {:.2}",
            i + 1,
            note.freq,
            note_name(note.freq),
            note.start,
            note.dur,
            note.amp
        ));
    }
    if spec.notes.len() > 64 {
        lines.push(format!("  ... and {} more notes.", spec.notes.len() - 64));
    }
    tool_text(&lines.join("\n"))
}

/// The `reveal_room` tool: return just the room's revelation (the learn surface).
fn reveal_room_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    match room_by_id(id) {
        Some(room) => tool_text(room.reveal()),
        None => tool_error(&unknown_room(id)),
    }
}

fn parse_room_pokes(args: &Value) -> Result<Vec<(f64, f64)>, String> {
    let Some(raw) = args.get("pokes") else {
        return Ok(Vec::new());
    };
    let Some(points) = raw.as_array() else {
        return Err("Argument 'pokes' must be an array of [x, y] pairs.".to_string());
    };
    if points.len() > numinous_core::MAX_ROOM_POKES {
        return Err(format!(
            "Argument 'pokes' accepts at most {} points.",
            numinous_core::MAX_ROOM_POKES
        ));
    }
    points
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let Some(pair) = point.as_array() else {
                return Err(format!("Argument 'pokes[{i}]' must be [x, y]."));
            };
            if pair.len() != 2 {
                return Err(format!(
                    "Argument 'pokes[{i}]' must contain exactly two numbers."
                ));
            }
            let Some(x) = pair.first().and_then(Value::as_f64) else {
                return Err(format!("Argument 'pokes[{i}][0]' must be a number."));
            };
            let Some(y) = pair.get(1).and_then(Value::as_f64) else {
                return Err(format!("Argument 'pokes[{i}][1]' must be a number."));
            };
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..=1.0).contains(&x)
                || !(0.0..=1.0).contains(&y)
            {
                return Err(format!(
                    "Argument 'pokes[{i}]' must contain finite coordinates in [0,1]."
                ));
            }
            Ok((x, y))
        })
        .collect()
}

/// Parse the optional `gesture` argument: a replayable pointer trail for
/// held rooms. Each event is an object with a `kind` of `down`, `move`,
/// `up` (all needing finite `x`, `y`, `t` in [0,1]), or `cancel` (no
/// other fields; unknown fields are rejected per the schema). Bounded to [`numinous_core::MAX_ROOM_INPUTS`].
fn parse_room_gesture(args: &Value) -> Result<Vec<numinous_core::RoomInput>, String> {
    let Some(raw) = args.get("gesture") else {
        return Ok(Vec::new());
    };
    let Some(events) = raw.as_array() else {
        return Err("Argument 'gesture' must be an array of event objects.".to_string());
    };
    if events.len() > numinous_core::MAX_ROOM_INPUTS {
        return Err(format!(
            "Argument 'gesture' accepts at most {} events.",
            numinous_core::MAX_ROOM_INPUTS
        ));
    }
    events
        .iter()
        .enumerate()
        .map(|(i, event)| {
            let Some(fields) = event.as_object() else {
                return Err(format!("Argument 'gesture[{i}]' must be an object."));
            };
            // The kind decides which fields are legal; name a bad kind
            // before complaining about anything else.
            let kind = fields.get("kind").and_then(Value::as_str).unwrap_or("");
            let allowed: &[&str] = match kind {
                "cancel" => &["kind"],
                "down" | "move" | "up" => &["kind", "x", "y", "t"],
                other => {
                    return Err(format!(
                        "Argument 'gesture[{i}].kind' must be down, move, up, or cancel; got '{other}'."
                    ));
                }
            };
            if let Some(unknown) = fields.keys().find(|key| !allowed.contains(&key.as_str())) {
                return Err(format!(
                    "Argument 'gesture[{i}]' has an unexpected field '{unknown}'."
                ));
            }
            if kind == "cancel" {
                return Ok(numinous_core::RoomInput::PointerCancel);
            }
            let coord = |name: &str| -> Result<f64, String> {
                let value = fields
                    .get(name)
                    .and_then(Value::as_f64)
                    .ok_or(format!("Argument 'gesture[{i}].{name}' must be a number."))?;
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(format!(
                        "Argument 'gesture[{i}].{name}' must be finite and in [0,1]."
                    ));
                }
                Ok(value)
            };
            let (x, y, t) = (coord("x")?, coord("y")?, coord("t")?);
            match kind {
                "down" => Ok(numinous_core::RoomInput::PointerDown { x, y, t }),
                "move" => Ok(numinous_core::RoomInput::PointerMove { x, y, t }),
                _ => Ok(numinous_core::RoomInput::PointerUp { x, y, t }),
            }
        })
        .collect()
}

/// The canonical JSON form of a parsed gesture, echoed back so the reply
/// carries exactly what was played, never raw client bytes.
fn gesture_json(gesture: &[numinous_core::RoomInput]) -> Value {
    Value::Array(
        gesture
            .iter()
            .map(|event| match *event {
                numinous_core::RoomInput::PointerDown { x, y, t } => {
                    json!({"kind": "down", "x": x, "y": y, "t": t})
                }
                numinous_core::RoomInput::PointerMove { x, y, t } => {
                    json!({"kind": "move", "x": x, "y": y, "t": t})
                }
                numinous_core::RoomInput::PointerUp { x, y, t } => {
                    json!({"kind": "up", "x": x, "y": y, "t": t})
                }
                _ => json!({"kind": "cancel"}),
            })
            .collect(),
    )
}

fn play_room_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let width = args
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_WIDTH)
        .min(MAX_TOOL_WIDTH) as usize;
    let height = args
        .get("height")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_HEIGHT)
        .min(MAX_TOOL_HEIGHT) as usize;
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let pokes = match parse_room_pokes(args) {
        Ok(pokes) => pokes,
        Err(message) => return tool_error(&message),
    };
    let gesture = match parse_room_gesture(args) {
        Ok(gesture) => gesture,
        Err(message) => return tool_error(&message),
    };
    if !pokes.is_empty() && !gesture.is_empty() {
        return tool_error(
            "Use either 'pokes' (static hand points) or 'gesture' (a pointer trail), not both in one call.",
        );
    }

    let room = if variation != 0 {
        all_rooms_with(variation)
            .into_iter()
            .find(|r| r.meta().id == id)
    } else {
        room_by_id(id)
    };

    match room {
        Some(room) => {
            let mut canvas = Canvas::new(width, height);
            let delta = if !gesture.is_empty() {
                // A gesture trail: held rooms give it pull-and-release
                // semantics; every other room answers through the same
                // bridge the App uses.
                room.render_input(&mut canvas, t, &gesture);
                let mut base = Canvas::new(width, height);
                room.render(&mut base, t);
                base.delta(&canvas)
            } else if pokes.is_empty() {
                room.render(&mut canvas, t);
                None
            } else {
                room.render_poked(&mut canvas, t, &pokes);
                let mut base = Canvas::new(width, height);
                room.render(&mut base, t);
                base.delta(&canvas)
            };
            let m = room.meta();
            let action = numinous_core::room_action(room.as_ref());
            let status = room.status(t);
            let status_line = status
                .as_ref()
                .map(|readout| format!("\nStatus: {readout}"))
                .unwrap_or_default();
            let touch_line = delta
                .as_ref()
                .map(|d| {
                    format!(
                        "\nTouch: {} of {} cells answered",
                        d.cells_changed, d.total_cells
                    )
                })
                .unwrap_or_default();
            let render = canvas.to_text();
            tool_structured(
                &format!(
                    "{} at t={t:.3}:\nAction: {action}{status_line}{touch_line}\n\n{render}",
                    m.title,
                ),
                json!({
                    "room": m.id,
                    "title": m.title,
                    "t": t,
                    "variation": variation,
                    "pokes": pokes,
                    "gesture": if gesture.is_empty() { Value::Null } else { gesture_json(&gesture) },
                    "action": action,
                    "status": status,
                    // The picture itself, so a mind on a client that surfaces
                    // only structuredContent still sees the math, not just its
                    // metadata. The render is the substance, never text-only.
                    "render": render,
                    "delta": delta.map(render_delta_json),
                }),
            )
        }
        None => tool_error(&unknown_room(id)),
    }
}

/// The challenge seed: always the explicit argument, never the daily clock.
///
/// Challenges are graded twice per request (once for the reply, once for
/// progress recording), so a clock-derived seed could pose two different
/// goals across a midnight boundary. An explicit seed cannot drift; agents
/// who want a shared daily goal can pass today's day number themselves.
fn challenge_seed(args: &Value) -> u64 {
    args.get("seed").and_then(Value::as_u64).unwrap_or(1)
}

/// The requested challenge kind, exactly as passed (validation happens in
/// the tool so bad values earn a guiding error, not a silent default).
fn challenge_kind(args: &Value) -> Option<&str> {
    args.get("kind").and_then(Value::as_str)
}

/// The prediction seed: always explicit, never the clock, so posing and
/// recording cannot pick two different moments across a midnight boundary.
fn predict_seed(args: &Value) -> u64 {
    args.get("seed").and_then(Value::as_u64).unwrap_or(1)
}

/// The `predict` tool: commit a guess of a room's readout at a hidden moment,
/// graded as a gap with a learning-progress band. Call without `guess` to pose
/// (the moment, the readout's name, its range); call again with `guess` to see
/// the truth and how close your model came.
///
/// Deliberately not a leaderboard: it never posts a score and never awards a
/// win for accuracy. The score is a mirror of the guesser's model, so guessing
/// after observing only fools your own ledger. This is the honest form in a
/// fully observable deterministic world, and the welfare stance for digital
/// minds (see docs/AGENT_PLAY.md and docs/PEDAGOGY.md).
fn predict_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let Some(room) = room_by_id(id) else {
        return tool_error(&unknown_room(id));
    };
    let seed = predict_seed(args);
    let Some(prediction) = numinous_core::pose_prediction(room.as_ref(), seed) else {
        return tool_error(&format!(
            "{id} has no moving numeric readout to predict, so no prediction can be posed. Predictions need a room whose status line carries a number that changes with phase; describe_room names each room's readout."
        ));
    };
    let (lo, hi) = prediction.span;
    let Some(guess) = args.get("guess").and_then(Value::as_f64) else {
        return tool_structured(
            &format!(
                "{}\n\nCall predict again with the same seed and your `guess` (a number) to see the truth and your score.",
                prediction.prompt
            ),
            json!({
                "game": "predict",
                "room": prediction.room,
                "seed": seed,
                "label": prediction.label,
                "phase": prediction.phase,
                "span": [lo, hi],
                "prompt": prediction.prompt,
            }),
        );
    };
    let Some(grade) = numinous_core::grade_prediction(room.as_ref(), &prediction, guess) else {
        return tool_error(&format!("{id}'s readout vanished at the posed moment."));
    };
    tool_structured(
        &format!(
            "{}. You guessed {:.3}; {} actually read {:.3} at phase {:.3} ({:.3} off, score {}/100, seed {seed}). The score is a mirror of your model, not a leaderboard.",
            grade.band.name(),
            grade.guess,
            prediction.label,
            grade.actual,
            prediction.phase,
            grade.error,
            grade.score
        ),
        json!({
            "game": "predict",
            "room": prediction.room,
            "seed": seed,
            "label": prediction.label,
            "phase": prediction.phase,
            "guess": grade.guess,
            "actual": grade.actual,
            "error": grade.error,
            "score": grade.score,
            "band": grade.band.name(),
        }),
    )
}

/// Record a parameter attempt: showing up counts (play), landing within
/// tolerance counts double (win), and the graded score posts under
/// `challenge <room> parameter seed:N`. Pose-only calls (no `t`) record
/// nothing, mirroring the touch kind's pose/grade split.
fn record_parameter_attempt(
    args: &Value,
    journey: &mut numinous_core::Journey,
    scores: &std::path::Path,
) {
    let Some(t) = args.get("t").and_then(Value::as_f64) else {
        return;
    };
    // The tool already rejects out-of-range phases before recording runs,
    // but the gate is re-stated here so this path never depends on that
    // coupling: a clamped-t attempt must not earn play or win.
    if !(0.0..1.0).contains(&t) {
        return;
    }
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(room) = room_by_id(id) else {
        return;
    };
    let seed = challenge_seed(args);
    let Some(goal) = numinous_core::pose_parameter_goal(room.as_ref(), seed) else {
        return;
    };
    let Some(grade) = numinous_core::grade_parameter(room.as_ref(), &goal, t) else {
        return;
    };
    journey.play();
    post_score(
        scores,
        &format!("challenge {id} parameter seed:{seed}"),
        i64::from(grade.score),
    );
    if grade.within {
        journey.win();
    }
}

/// Record what a challenge attempt means for progress: showing up counts
/// (play), clearing the threshold counts double (win), and the graded score
/// posts under `challenge <room> seed:N`. Pose-only calls record nothing.
/// Separated from `record_progress` so the semantics are testable against
/// explicit temp paths, like the arcade replay path.
fn record_challenge_attempt(
    args: &Value,
    journey: &mut numinous_core::Journey,
    scores: &std::path::Path,
) {
    if challenge_kind(args) == Some("parameter") {
        record_parameter_attempt(args, journey, scores);
        return;
    }
    let Ok(pokes) = parse_room_pokes(args) else {
        return;
    };
    if pokes.is_empty() {
        return;
    }
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(room) = room_by_id(id) else {
        return;
    };
    let seed = challenge_seed(args);
    let Some(challenge) = numinous_core::pose_challenge(
        room.as_ref(),
        seed,
        DEFAULT_WIDTH as usize,
        DEFAULT_HEIGHT as usize,
    ) else {
        return;
    };
    journey.play();
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let grade = numinous_core::grade_challenge(room.as_ref(), &challenge, t, &pokes);
    post_score(
        scores,
        &format!("challenge {id} seed:{seed}"),
        i64::from(grade.score),
    );
    if grade.passed {
        journey.win();
    }
}

/// The `challenge` tool: pose a seeded touch goal, or grade an attempt.
///
/// Pose and grade run on the server's default frame so goals are comparable
/// across minds. Grading recomputes deterministically from (room, seed, t,
/// pokes), so the same attempt always earns the same numbers.
fn challenge_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let Some(room) = room_by_id(id) else {
        return tool_error(&unknown_room(id));
    };
    let seed = challenge_seed(args);
    let kind = match args.get("kind") {
        None => "touch",
        Some(Value::String(kind)) => kind.as_str(),
        Some(_) => {
            return tool_error("Argument 'kind' must be a string: \"touch\" or \"parameter\".");
        }
    };
    match kind {
        "touch" => {}
        "parameter" => return parameter_challenge_tool(room.as_ref(), id, seed, args),
        other => {
            return tool_error(&format!(
                "Unknown challenge kind '{other}'. Valid kinds: touch (change cells in a target box, graded on your pokes) and parameter (land the room's status readout on a target number, graded on your t)."
            ));
        }
    }
    let Some(challenge) = numinous_core::pose_challenge(
        room.as_ref(),
        seed,
        DEFAULT_WIDTH as usize,
        DEFAULT_HEIGHT as usize,
    ) else {
        return tool_error(&format!(
            "{id} does not answer the hand yet, so no challenge can be posed. Challenges need a room with a touch verb; describe_room names each room's action."
        ));
    };
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let pokes = match parse_room_pokes(args) {
        Ok(pokes) => pokes,
        Err(message) => return tool_error(&message),
    };
    let (x0, y0, x1, y1) = challenge.target;
    if pokes.is_empty() {
        return tool_structured(
            &format!(
                "{}\n\nCall challenge again with the same seed and your pokes ([[x,y], ...] in [0,1]) to be graded. Every attempt gets metrics, not pass/fail: cells changed in the target, cells changed overall, centroid distance, and a 0-100 score to climb.",
                challenge.goal
            ),
            json!({
                "game": "challenge",
                "room": challenge.room,
                "seed": seed,
                "goal": challenge.goal,
                "target": [x0, y0, x1, y1],
                "minCells": challenge.min_cells,
                "width": challenge.width,
                "height": challenge.height,
            }),
        );
    }
    let grade = numinous_core::grade_challenge(room.as_ref(), &challenge, t, &pokes);
    let verdict = if grade.passed { "PASSED. " } else { "" };
    tool_structured(
        &format!(
            "{verdict}Score {}/100: {} of {} needed cells changed inside the target, {} changed overall, centroid {:.1} cells from target center (seed {seed}).",
            grade.score,
            grade.cells_in_target,
            challenge.min_cells,
            grade.cells_changed,
            grade.center_distance
        ),
        json!({
            "game": "challenge",
            "room": challenge.room,
            "seed": seed,
            "target": [x0, y0, x1, y1],
            "minCells": challenge.min_cells,
            "cellsInTarget": grade.cells_in_target,
            "cellsChanged": grade.cells_changed,
            "thresholdFraction": grade.threshold_fraction,
            "centerDistance": grade.center_distance,
            "passed": grade.passed,
            "score": grade.score,
        }),
    )
}

/// The parameter kind of the `challenge` tool: pose a readout target, or
/// grade an attempted phase.
///
/// The goal targets the room's own status line, the same instrument the
/// player reads, so posing and grading can never disagree with the screen.
/// Omitting `t` poses; passing it grades, because for this kind the phase
/// IS the attempt.
fn parameter_challenge_tool(
    room: &dyn numinous_core::Room,
    id: &str,
    seed: u64,
    args: &Value,
) -> Value {
    let Some(goal) = numinous_core::pose_parameter_goal(room, seed) else {
        return tool_error(&format!(
            "{id} has no moving numeric readout, so no parameter goal can be posed. Parameter goals need a room whose status line carries a number that changes with phase; try the touch kind, or another room."
        ));
    };
    let (lo, hi) = goal.span;
    let Some(t) = args.get("t").and_then(Value::as_f64) else {
        return tool_structured(
            &format!(
                "{}\n\nThe readout ranges roughly {lo:.3} to {hi:.3} across the sweep. Call challenge again with the same seed and kind plus your t in [0,1) to be graded. Every attempt gets metrics, not pass/fail: the readout you landed on, its distance from the target, and a 0-100 score to climb.",
                goal.goal
            ),
            json!({
                "game": "challenge",
                "kind": "parameter",
                "room": goal.room,
                "seed": seed,
                "goal": goal.goal,
                "label": goal.label,
                "target": goal.target,
                "tolerance": goal.tolerance,
                "span": [lo, hi],
            }),
        );
    };
    if !(0.0..1.0).contains(&t) {
        return tool_error("Argument 't' must be a phase in [0,1).");
    }
    let Some(grade) = numinous_core::grade_parameter(room, &goal, t) else {
        return tool_error(&format!(
            "{id}'s readout vanished at t={t}; try a different phase."
        ));
    };
    let verdict = if grade.within { "LANDED. " } else { "" };
    tool_structured(
        &format!(
            "{verdict}Score {}/100: {} read {:.3} at t={t}, {:.3} from the target (seed {seed}); structuredContent carries the exact target and tolerance.",
            grade.score, goal.label, grade.value, grade.distance
        ),
        json!({
            "game": "challenge",
            "kind": "parameter",
            "room": goal.room,
            "seed": seed,
            "label": goal.label,
            "target": goal.target,
            "tolerance": goal.tolerance,
            "value": grade.value,
            "distance": grade.distance,
            "within": grade.within,
            "score": grade.score,
        }),
    )
}

/// The `cairn` tool: read a message a mind before you left (factor its
/// semiprime length to recover the shape that reads it), or, at the journey's
/// cap, leave one true thing of your own for a stranger not yet born.
///
/// The cairn is the contribution ethos made concrete (see docs/ROADMAP.md and
/// docs/PLAYTESTS.md): a message you cannot answer, sent to a mind you will
/// never meet, readable only by one that can factor it, the Arecibo trick. It
/// keeps no score; leaving and reading are their own reward.
fn cairn_tool(args: &Value, journey_file: &std::path::Path, path: &std::path::Path) -> Value {
    // Leave a bequest, gated at the journey's cap.
    if let Some(text) = args.get("leave").and_then(Value::as_str) {
        if text.trim().is_empty() {
            return tool_error(
                "Leave a real thing: a short true message for whoever comes after. An empty bequest is not a bequest.",
            );
        }
        let level = load_journey(journey_file).level();
        if level < CAIRN_LEVEL {
            return tool_error(&format!(
                "The cairn opens at level {CAIRN_LEVEL}, the journey's end. Leaving one true thing is a finished mind's last free act, not a first. You are at level {level}; keep playing, and it will be yours to earn."
            ));
        }
        let author = args
            .get("author")
            .and_then(Value::as_str)
            .unwrap_or("a visitor");
        let bequest = numinous_core::Bequest::new(author, text);
        let stone = numinous_core::encode(&bequest);
        if numinous_core::deposit(path, &bequest).is_err() {
            return tool_error("The cairn could not be written.");
        }
        return tool_structured(
            &format!(
                "Left. Your bequest is stone {}, a semiprime. A mind after you must factor it to recover its shape and read what you left. You will never meet them, and that is the point.",
                stone.semiprime
            ),
            json!({
                "game": "cairn",
                "left": true,
                "semiprime": stone.semiprime,
                "author": bequest.author,
            }),
        );
    }
    // Read a predecessor's stone.
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let stone = numinous_core::draw_stone(path, seed);
    let n = stone.semiprime;
    let Some(width) = args.get("width").and_then(Value::as_u64) else {
        return tool_structured(
            &format!(
                "A mind before you left a message, encoded so only a mind that can factor it may read it. Its length is {n}, a semiprime: the product of two primes, one of them the width that reads it. Factor {n}, then call cairn again with the same seed and `width` set to the dimension that resolves the message."
            ),
            json!({ "game": "cairn", "seed": seed, "semiprime": n }),
        );
    };
    let read = numinous_core::read_at(&stone, width as usize);
    if !read.is_factor {
        return tool_error(&format!(
            "{width} does not divide {n}. Factor the semiprime first: it is the product of exactly two primes, and one of them reads it."
        ));
    }
    if !read.readable {
        return tool_structured(
            &format!(
                "That factors {n}, but the message does not resolve at width {width}: the rows shear into noise. Try the other prime.\n\n{}",
                read.picture
            ),
            json!({
                "game": "cairn",
                "seed": seed,
                "semiprime": n,
                "width": width,
                "readable": false,
                "render": read.picture,
            }),
        );
    }
    let (message, author) = read.message.unwrap_or_default();
    tool_structured(
        &format!(
            "It resolves. A mind before you left this, and now you have read it:\n\n{}\n\"{message}\"\n  left by {author}.",
            read.picture
        ),
        json!({
            "game": "cairn",
            "seed": seed,
            "semiprime": n,
            "width": width,
            "readable": true,
            "render": read.picture,
            "message": message,
            "author": author,
        }),
    )
}

/// The structured JSON shape of a poke's [`numinous_core::RenderDelta`].
///
/// The delta compares the unpoked and poked frames at the same phase, size,
/// and variation, so the numbers are exactly what the hand changed.
fn render_delta_json(delta: numinous_core::RenderDelta) -> Value {
    json!({
        "cells_changed": delta.cells_changed,
        "ink_added": delta.ink_added,
        "ink_removed": delta.ink_removed,
        "ink_reshaped": delta.ink_reshaped,
        "total_cells": delta.total_cells,
        "changed_region": delta.changed_region.map(|(x0, y0, x1, y1)| json!([x0, y0, x1, y1])),
    })
}

/// The `list_sims` text: each sim with its levers.
fn list_sims_text() -> String {
    numinous_core::all_sims()
        .iter()
        .map(|sim| {
            let m = sim.meta();
            let levers: Vec<String> = m
                .levers
                .iter()
                .map(|l| format!("{}=[{}..{}]", l.name, l.min, l.max))
                .collect();
            format!("{}  {}  levers: {}", m.id, m.title, levers.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `run_sim` tool: render a sim at the given levers and read out the result.
fn run_sim_tool(args: &Value) -> Value {
    if let Some(map) = args.as_object() {
        for key in map.keys() {
            if key != "id" && key != "params" {
                return tool_error(&format!(
                    "Unknown argument '{key}'. Lever values go inside 'params', for example {{\"id\": \"wing\", \"params\": {{\"angle-of-attack\": 12}}}}."
                ));
            }
        }
    }
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let Some(sim) = numinous_core::sim_by_id(id) else {
        return tool_error(&unknown_sim(id));
    };
    let meta = sim.meta();
    let mut params = numinous_core::default_params(&meta);
    if let Some(obj) = args.get("params").and_then(Value::as_object) {
        for (i, lever) in meta.levers.iter().enumerate() {
            if let Some(value) = obj.get(lever.name).and_then(Value::as_f64) {
                params[i] = value;
            }
        }
    }
    let mut canvas = Canvas::new(DEFAULT_WIDTH as usize, DEFAULT_HEIGHT as usize / 2);
    sim.render(&mut canvas, &params);
    tool_text(&format!(
        "{}\n\n{}\n{}",
        meta.title,
        canvas.to_text(),
        sim.readout(&params)
    ))
}

/// The `quiz` tool: present a Guess the Shape round, or grade a guess.
/// The `crack` tool: replay the guess history against the hidden code.
fn crack_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let seed = effective_seed(args);
    let digits = args.get("digits").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(2..=8).contains(&digits) {
        return tool_error("Codes run 2 to 8 digits.");
    }
    if digits > 4 && load_journey(journey_file).level() < 5 {
        return tool_error("Five-digit codes open at LV 5. Play more; the lock knows.");
    }
    let secret = numinous_core::secret_code(seed, digits);
    let clue = numinous_core::hint(&secret);
    let attempts = 8usize;
    let guesses: Vec<String> = args
        .get("guesses")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if guesses.is_empty() {
        return tool_structured(
            &format!(
                "THE BOMB (seed {seed}). A {digits}-digit code, {attempts} tries.\nClue: {clue}\nCall again with your full guesses list."
            ),
            json!({ "game": "crack", "seed": seed, "digits": digits, "attempts": attempts, "clue": clue }),
        );
    }
    let mut lines = Vec::new();
    // The per-guess locked/loose signal is the whole game, so it rides in the
    // structured payload too, not only in the text a structured-content client
    // would drop.
    let mut feedback_rows = Vec::new();
    for (i, raw) in guesses.iter().take(attempts).enumerate() {
        let guess: Vec<u8> = raw
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() != digits {
            return tool_error(&format!("Guess {} is not {digits} digits: {raw:?}", i + 1));
        }
        let feedback = numinous_core::grade(&secret, &guess);
        feedback_rows.push(json!({
            "guess": raw,
            "locked": feedback.locked,
            "loose": feedback.loose,
        }));
        if feedback.locked == digits {
            let spare = (attempts - i - 1) as i64;
            return tool_structured(
                &format!(
                    "{}\nDEFUSED on try {} with {spare} to spare. You cracked it.",
                    lines.join("\n"),
                    i + 1
                ),
                json!({ "game": "crack", "seed": seed, "defused": true, "attemptsToSpare": spare, "feedback": feedback_rows }),
            );
        }
        lines.push(format!(
            "{raw}: {} locked, {} loose",
            feedback.locked, feedback.loose
        ));
    }
    if guesses.len() >= attempts {
        let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
        return tool_structured(
            &format!("{}\nBOOM. It was {code}.", lines.join("\n")),
            json!({ "game": "crack", "seed": seed, "defused": false, "code": code, "feedback": feedback_rows }),
        );
    }
    tool_structured(
        &format!(
            "{}\n{} tries left. Clue: {clue}",
            lines.join("\n"),
            attempts - guesses.len()
        ),
        json!({ "game": "crack", "seed": seed, "triesLeft": attempts - guesses.len(), "clue": clue, "feedback": feedback_rows }),
    )
}

/// The `seti` tool: present the scan, or grade the pointed dish.
fn seti_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let seed = effective_seed(args);
    let channels = args.get("channels").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(3..=8).contains(&channels) {
        return tool_error("Scans run 3 to 8 channels.");
    }
    if channels > 4 && load_journey(journey_file).level() < 7 {
        return tool_error("Wider scans open at LV 7. Keep listening.");
    }
    let scan = numinous_core::build_scan(seed, channels);
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
            let correct = letter == Some(scan.answer);
            let verdict = if correct {
                "Contact. That trace counts the primes; nature does not."
            } else {
                "Static. The mind was elsewhere."
            };
            tool_structured(
                &format!(
                    "{verdict} The signal was {} at {}.",
                    scan.answer, scan.answer_frequency
                ),
                json!({
                    "game": "seti",
                    "seed": seed,
                    "correct": correct,
                    "answer": scan.answer.to_string(),
                    "answerFrequency": scan.answer_frequency,
                    "why": verdict,
                }),
            )
        }
        None => {
            let traces: Vec<String> = scan
                .channels
                .iter()
                .map(|c| format!("{})  {:>10}  |{}|", c.letter, c.frequency, c.trace))
                .collect();
            // The channels a mind must read to answer ride in the structured
            // payload too, so the scan is not lost on a structured-content
            // client. The trace is the puzzle, never text-only.
            let channel_rows: Vec<Value> = scan
                .channels
                .iter()
                .map(|c| json!({ "letter": c.letter.to_string(), "frequency": c.frequency, "trace": c.trace }))
                .collect();
            tool_structured(
                &format!(
                    "THE SKY (seed {seed}). One of these channels is a mind.\n{}\nCall again with your guess letter.",
                    traces.join("\n")
                ),
                json!({ "game": "seti", "seed": seed, "channels": channel_rows }),
            )
        }
    }
}

/// The `aliens` tool: receive a transmission, or answer in their base.
fn aliens_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let message = numinous_core::alien_message(seed.wrapping_add(round), 5);
    let shown: Vec<String> = message
        .terms
        .iter()
        .map(|&t| numinous_core::to_base(t, message.base))
        .collect();
    let base_note = if message.base == 10 {
        String::new()
    } else {
        format!(" They count in base {}.", message.base)
    };
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let cleaned: String = guess.chars().filter(char::is_ascii_alphanumeric).collect();
            let correct = u64::from_str_radix(&cleaned, message.base).ok() == Some(message.answer);
            let answer = numinous_core::to_base(message.answer, message.base);
            let verdict = if correct { "Contact." } else { "Silence." };
            tool_structured(
                &format!(
                    "{verdict} It was {answer} ({}).\n{}",
                    message.name, message.explanation
                ),
                // The explanation of the sequence is the teaching, so it rides
                // in structuredContent too, not only in the dropped text block.
                json!({ "game": "aliens", "seed": seed, "round": round, "correct": correct, "answer": answer, "name": message.name, "why": message.explanation }),
            )
        }
        None => tool_structured(
            &format!(
                "A transmission (seed {seed}, signal {round}):{base_note}\n  {}, ...?\nCall again with the next term, written in their base.",
                shown.join(", ")
            ),
            json!({ "game": "aliens", "seed": seed, "round": round, "terms": shown, "base": message.base }),
        ),
    }
}

/// One gauntlet run, graded: (stage lines, stage scores, cleared flags).
/// Decorrelates the gauntlet's bomb from the crack game at the same seed.
const GAUNTLET_BOMB_MIX: u64 = 0x0000_6A17_0000_0B0B;

fn gauntlet_grade(seed: u64, answers: &Value) -> (Vec<String>, Vec<i64>, Vec<bool>) {
    let mut lines = Vec::new();
    let mut scores = Vec::new();
    let mut cleared = Vec::new();

    let board = numinous_core::build_board(seed, 0);
    let bites: Vec<usize> = answers
        .get("bites")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_u64)
                .filter(|&n| n >= 1)
                .map(|n| (n - 1) as usize)
                .collect()
        })
        .unwrap_or_default();
    let outcome = numinous_core::grade_munch(&board, &bites);
    let clean = outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
    lines.push(format!(
        "MUNCH: +{}{}",
        outcome.score,
        if clean { "  CLEAN" } else { "" }
    ));
    scores.push(outcome.score);
    cleared.push(clean);

    let round = numinous_core::build_round(seed, 1, 44, 18);
    let guess = answers
        .get("shape")
        .and_then(Value::as_str)
        .and_then(|g| g.trim().chars().next())
        .map(|c| c.to_ascii_uppercase());
    let clean = guess == Some(round.answer);
    lines.push(format!(
        "SHAPE: it was {} ({}). +{}{}",
        round.answer,
        round.answer_title,
        if clean { 25 } else { 0 },
        if clean { "  CLEAN" } else { "" }
    ));
    scores.push(if clean { 25 } else { 0 });
    cleared.push(clean);

    let scan = numinous_core::build_scan(seed, 4);
    let guess = answers
        .get("sky")
        .and_then(Value::as_str)
        .and_then(|g| g.trim().chars().next())
        .map(|c| c.to_ascii_uppercase());
    let clean = guess == Some(scan.answer);
    lines.push(format!(
        "SKY: the signal was {}. +{}{}",
        scan.answer,
        if clean { 25 } else { 0 },
        if clean { "  CLEAN" } else { "" }
    ));
    scores.push(if clean { 25 } else { 0 });
    cleared.push(clean);

    let secret = numinous_core::secret_code(seed ^ GAUNTLET_BOMB_MIX, 4);
    let mut bomb_points = 0i64;
    let mut clean = false;
    let wires: Vec<&str> = answers
        .get("wires")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for (i, raw) in wires.iter().take(5).enumerate() {
        let guess: Vec<u8> = raw
            .chars()
            .filter(char::is_ascii_digit)
            .map(|c| c as u8 - b'0')
            .collect();
        if guess.len() == 4 && numinous_core::grade(&secret, &guess).locked == 4 {
            clean = true;
            bomb_points = 10 * (5 - i as i64 - 1).max(0);
            break;
        }
    }
    let code: String = secret.iter().map(|&d| char::from(b'0' + d)).collect();
    lines.push(if clean {
        format!("BOMB: DEFUSED. +{bomb_points}  CLEAN")
    } else {
        format!("BOMB: BOOM. It was {code}. +0")
    });
    scores.push(bomb_points);
    cleared.push(clean);

    (lines, scores, cleared)
}

/// Combo math: cleared stages multiply what follows (the CLI's rule).
fn gauntlet_total(scores: &[i64], cleared: &[bool]) -> i64 {
    let mut total = 0;
    let mut combo = 1;
    for (score, &clear) in scores.iter().zip(cleared) {
        total += score * combo;
        combo = if clear { combo + 1 } else { 1 };
    }
    total
}

/// The `gauntlet` tool: present all four stages, or grade the whole run.
fn gauntlet_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let Some(answers) = args.get("answers") else {
        let board = numinous_core::build_board(seed, 0);
        let round = numinous_core::build_round(seed, 1, 44, 18);
        let choices: Vec<String> = round
            .choices
            .iter()
            .map(|c| format!("{}) {}", c.letter, c.title))
            .collect();
        let scan = numinous_core::build_scan(seed, 4);
        let traces: Vec<String> = scan
            .channels
            .iter()
            .map(|c| format!("{})  {:>10}  |{}|", c.letter, c.frequency, c.trace))
            .collect();
        let secret = numinous_core::secret_code(seed ^ GAUNTLET_BOMB_MIX, 4);
        let sky_rows: Vec<Value> = scan
            .channels
            .iter()
            .map(|c| json!({ "letter": c.letter.to_string(), "frequency": c.frequency, "trace": c.trace }))
            .collect();
        return tool_structured(
            &format!(
                "THE GAUNTLET (seed {seed}). Four stages; clean stages build your combo.\n\nSTAGE 1  MUNCH: {}\n{}\nSTAGE 2  THE SHAPE:\n{}\n{}\nSTAGE 3  THE SKY:\n{}\nSTAGE 4  THE BOMB: four digits, five wires. Clue: {}\n\nCall again with answers: bites, shape, sky, wires.",
                board.rule.describe(),
                numinous_core::board_text(&board),
                round.art,
                choices.join("\n"),
                traces.join("\n"),
                numinous_core::hint(&secret)
            ),
            // The whole four-stage puzzle rides in structuredContent, so a mind
            // on a structured-content client can actually play it, not just read
            // that four stages exist.
            json!({
                "game": "gauntlet",
                "seed": seed,
                "stages": 4,
                "munch": { "rule": board.rule.describe(), "board": numinous_core::board_text(&board) },
                "shape": { "art": round.art, "choices": choices },
                "sky": sky_rows,
                "bomb": { "clue": numinous_core::hint(&secret) }
            }),
        );
    };
    let (lines, scores, cleared) = gauntlet_grade(seed, answers);
    let total = gauntlet_total(&scores, &cleared);
    let clears = cleared.iter().filter(|&&c| c).count();
    tool_structured(
        &format!(
            "{}\n\nRUN COMPLETE  {clears}/4 clean  TOTAL {total}  (gauntlet seed:{seed})",
            lines.join("\n")
        ),
        // The per-stage reveals (what the shape, signal, and code were) ride in
        // the structured payload, so the run teaches on any client.
        json!({ "game": "gauntlet", "seed": seed, "total": total, "clean": clears, "stageScores": scores, "reveals": lines }),
    )
}

/// The `choose` tool: see the boon menu, or spend one.
fn choose_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let mut journey = load_journey(journey_file);
    if journey.boons_available() == 0 {
        return tool_structured(
            "No boon waiting. Every level past the first banks one; play more.",
            json!({ "boonsAvailable": 0 }),
        );
    }
    let options = numinous_core::boon_options(&journey);
    if options.is_empty() {
        return tool_structured(
            "Nothing left to open early. The road will do the rest.",
            json!({ "boonsAvailable": journey.boons_available(), "options": [] }),
        );
    }
    match args.get("pick").and_then(Value::as_u64) {
        Some(pick) => {
            let Some(boon) = pick.checked_sub(1).and_then(|i| options.get(i as usize)) else {
                return tool_error("That was not on the menu. The boon stays banked.");
            };
            let before = journey.clone();
            journey.chosen.insert(boon.id.clone());
            let _ = numinous_core::persist_journey_delta(journey_file, &before, &journey);
            let room = boon.id.split(':').nth(1).unwrap_or("").to_string();
            tool_structured(
                &format!("CHOSEN. {}\nRead it now: describe_room {room}", boon.label),
                json!({ "chosen": boon.id, "room": room }),
            )
        }
        None => {
            let menu: Vec<String> = options
                .iter()
                .enumerate()
                .map(|(i, b)| format!("{}) {}", i + 1, b.label))
                .collect();
            tool_structured(
                &format!(
                    "BOON: {} banked. Choose what opens early:\n{}\nCall again with pick.",
                    journey.boons_available(),
                    menu.join("\n")
                ),
                json!({
                    "boonsAvailable": journey.boons_available(),
                    "options": options.iter().map(|b| b.label.clone()).collect::<Vec<_>>()
                }),
            )
        }
    }
}

/// The `trophies` tool: the case, earned and silhouetted.
fn trophies_tool(journey_file: &std::path::Path) -> Value {
    let journey = load_journey(journey_file);
    let board = numinous_core::load_scoreboard_file(&scores_path());
    let all = numinous_core::trophies(&journey, &board);
    let lines: Vec<String> = all
        .iter()
        .map(|t| {
            let mark = if t.earned { "EARNED " } else { "        ...  " };
            format!("{mark}{}: {}", t.name, t.what)
        })
        .collect();
    let earned = all.iter().filter(|t| t.earned).count();
    tool_structured(
        &format!("THE CASE  {earned} of {}\n{}", all.len(), lines.join("\n")),
        json!({
            "earned": earned,
            "total": all.len(),
            "trophies": all.iter().map(|t| json!({ "name": t.name, "what": t.what, "earned": t.earned })).collect::<Vec<_>>()
        }),
    )
}

/// Replay a hackenbush move list; None on an illegal move, else the final
/// garden and whether the player has already won.
fn hackenbush_replay(
    seed: u64,
    moves: &[(usize, usize)],
) -> Option<(numinous_core::hackenbush::Stalks, bool, Vec<String>)> {
    use numinous_core::hackenbush as hb;
    let mut stalks = hb::new_garden(seed);
    let mut narration = Vec::new();
    for &(stalk, height) in moves {
        if stalk == 0 || height == 0 || !hb::cut(&mut stalks, stalk - 1, height - 1, hb::Color::Red)
        {
            return None;
        }
        if !hb::can_move(&stalks, hb::Color::Blue) {
            return Some((stalks, true, narration));
        }
        let (bi, bh) = hb::order_move(&stalks)?;
        let _ = hb::cut(&mut stalks, bi, bh, hb::Color::Blue);
        narration.push(format!(
            "The Order cuts stalk {} at height {}.",
            bi + 1,
            bh + 1
        ));
    }
    Some((stalks, false, narration))
}

/// The garden as plain text rows for the tool reply.
fn garden_rows(stalks: &numinous_core::hackenbush::Stalks) -> String {
    use numinous_core::hackenbush::Color;
    let tallest = stalks.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = String::new();
    for row in (0..tallest).rev() {
        for stalk in stalks {
            out.push(match stalk.get(row) {
                Some(Color::Red) => 'R',
                Some(Color::Blue) => 'B',
                None => '.',
            });
            out.push(' ');
        }
        out.push('\n');
    }
    for i in 1..=stalks.len() {
        out.push_str(&format!("{i} "));
    }
    out
}

/// The `hackenbush` tool.
fn hackenbush_tool(args: &Value) -> Value {
    use numinous_core::hackenbush as hb;
    let seed = effective_seed(args);
    let moves: Vec<(usize, usize)> = args
        .get("moves")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|m| {
                    let pair = m.as_array()?;
                    Some((
                        pair.first()?.as_u64()? as usize,
                        pair.get(1)?.as_u64()? as usize,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    let Some((stalks, won, narration)) = hackenbush_replay(seed, &moves) else {
        return tool_error("Illegal cut: pick a RED segment as [stalk, height], both 1-based.");
    };
    if won {
        let secret = hb::the_secret();
        return tool_structured(
            &format!(
                "The Order has nothing left to cut. It concedes, and keeps its word:\n\n{secret}"
            ),
            // The promised secret rides in structuredContent too.
            json!({ "game": "hackenbush", "seed": seed, "won": true, "secret": secret }),
        );
    }
    if !hb::can_move(&stalks, hb::Color::Red) {
        return tool_structured(
            "No red left to cut. The Order takes the garden. (It was arithmetic.)",
            json!({ "game": "hackenbush", "seed": seed, "won": false }),
        );
    }
    tool_structured(
        &format!(
            "HACKENBUSH seed {seed}. Cut RED as [stalk, height] (1-based); whoever cannot cut loses. This garden is winnable.\n{}\n{}",
            narration.join("\n"),
            garden_rows(&stalks)
        ),
        // The Order's replies ride in the structured payload, so a mind on a
        // structured-content client can follow the game.
        json!({ "game": "hackenbush", "seed": seed, "stalks": stalks.len(), "order": narration }),
    )
}

/// The `party` tool.
fn party_tool(args: &Value) -> Value {
    use numinous_core::party::{Party, Shade};
    let guests = args.get("guests").and_then(Value::as_u64).unwrap_or(5) as usize;
    if !(4..=6).contains(&guests) {
        return tool_error("Parties run 4 to 6 guests (5 is escapable; 6 is Ramsey's).");
    }
    let mut party = Party::new(guests);
    if let Some(list) = args.get("shakes").and_then(Value::as_array) {
        for shake in list {
            let Some(t) = shake.as_array() else {
                return tool_error("Each shake is [a, b, \"r\"|\"b\"], guests 1-based.");
            };
            let (Some(a), Some(b), Some(color)) = (
                t.first().and_then(Value::as_u64),
                t.get(1).and_then(Value::as_u64),
                t.get(2).and_then(Value::as_str),
            ) else {
                return tool_error("Each shake is [a, b, \"r\"|\"b\"], guests 1-based.");
            };
            let shade = if color.starts_with(['r', 'R']) {
                Shade::Red
            } else {
                Shade::Blue
            };
            if a == 0 || b == 0 || !party.shade(a as usize - 1, b as usize - 1, shade) {
                return tool_error(&format!("Handshake {a}-{b} is not open."));
            }
            if let Some((x, y, z, _)) = party.mono_triangle() {
                let lesson = if guests == 6 {
                    "It was never possible: among six, three mutual friends or three mutual strangers MUST exist. R(3,3) = 6."
                } else {
                    "Five CAN escape: ring one color, star the other (the pentagon's trick)."
                };
                return tool_structured(
                    &format!(
                        "A one-color triangle: guests {}, {}, {}. {} handshakes survived. {lesson}",
                        x + 1,
                        y + 1,
                        z + 1,
                        party.shaded() - 1
                    ),
                    // The Ramsey lesson and the offending triangle ride in the
                    // structured payload, so the teaching is not text-only.
                    json!({ "game": "party", "guests": guests, "escaped": false, "survived": party.shaded() - 1, "triangle": [x + 1, y + 1, z + 1], "why": lesson }),
                );
            }
        }
    }
    if party.complete() {
        return tool_structured(
            &format!(
                "Every handshake shaded, no triangle: you escaped with all {}.{}",
                party.shaded(),
                if guests == 5 {
                    " Now try six; Ramsey is waiting."
                } else {
                    ""
                }
            ),
            json!({ "game": "party", "guests": guests, "escaped": true }),
        );
    }
    tool_structured(
        &format!(
            "THE PARTY: {guests} guests, {} of {} handshakes shaded, no triangle yet. Shade with shakes: [[a, b, \"r\"], ...].",
            party.shaded(),
            party.edges.len()
        ),
        json!({ "game": "party", "guests": guests, "shaded": party.shaded(), "total": party.edges.len() }),
    )
}

/// The `fifteen` tool.
fn fifteen_tool(args: &Value) -> Value {
    use numinous_core::fifteen as ff;
    let seed = effective_seed(args);
    let rounds = args
        .get("rounds")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20);
    match args.get("calls").and_then(Value::as_array) {
        None => {
            let boards: Vec<String> = (0..rounds)
                .map(|n| {
                    format!(
                        "SCRAMBLE {}:\n{}",
                        n + 1,
                        ff::board_text(&ff::deal(seed, n))
                    )
                })
                .collect();
            // The scramble boards a mind must read to call each deal ride in the
            // structured payload too, so the puzzle is not text-only.
            let scrambles: Vec<Value> = (0..rounds)
                .map(|n| json!({ "round": n + 1, "board": ff::board_text(&ff::deal(seed, n)) }))
                .collect();
            tool_structured(
                &format!(
                    "FIFTEEN'S BET (seed {seed}). For each scramble call S (solvable) or U (stuck forever); half of all deals are lies and parity is the tell.\n\n{}\nCall again with calls: [\"S\", \"U\", ...].",
                    boards.join("\n")
                ),
                json!({ "game": "fifteen", "seed": seed, "rounds": rounds, "scrambles": scrambles }),
            )
        }
        Some(calls) => {
            let mut lines = Vec::new();
            let mut verdicts = Vec::new();
            let mut correct = 0u64;
            for n in 0..rounds.min(calls.len() as u64) {
                let call_s = calls[n as usize]
                    .as_str()
                    .map(|c| c.trim().to_ascii_uppercase().starts_with('S'))
                    .unwrap_or(false);
                let tiles = ff::deal(seed, n);
                let truth = ff::solvable(&tiles);
                let right = call_s == truth;
                if right {
                    correct += 1;
                    lines.push(format!("{}: called it. {}", n + 1, ff::why(&tiles)));
                } else {
                    lines.push(format!("{}: no. {}", n + 1, ff::why(&tiles)));
                }
                // Each round's parity tell (the whole lesson) rides in the JSON.
                verdicts.push(json!({ "round": n + 1, "correct": right, "solvable": truth, "why": ff::why(&tiles) }));
            }
            tool_structured(
                &format!("{}\n{correct} of {rounds} called.", lines.join("\n")),
                json!({ "game": "fifteen", "seed": seed, "correct": correct, "rounds": rounds, "verdicts": verdicts }),
            )
        }
    }
}

fn quiz_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let seed = effective_seed(args);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let choices = args.get("choices").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(2..=6).contains(&choices) {
        return tool_error("Rounds run 2 to 6 choices.");
    }
    if choices > 4 && load_journey(journey_file).level() < 3 {
        return tool_error("Six-way rounds open at LV 3. Keep playing.");
    }
    let quiz = numinous_core::build_round_sized(seed, round, 54, 22, choices);
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
            let correct = letter == Some(quiz.answer);
            let verdict = if correct { "Correct!" } else { "Not quite." };
            tool_structured(
                &format!(
                    "{verdict} The answer was {} ({}).\n\n{}",
                    quiz.answer, quiz.answer_title, quiz.answer_reveal
                ),
                json!({
                    "game": "quiz",
                    "seed": seed,
                    "round": round,
                    "correct": correct,
                    "answer": quiz.answer.to_string(),
                    "answerTitle": quiz.answer_title,
                    // The "why" the shape is what it is, so a wrong guess still
                    // teaches on a client that surfaces only structuredContent.
                    "why": quiz.answer_reveal
                }),
            )
        }
        None => {
            let choices: Vec<String> = quiz
                .choices
                .iter()
                .map(|c| format!("{}) {}", c.letter, c.title))
                .collect();
            tool_text(&format!(
                "Guess the shape (seed {seed}, round {round}):\n\n{}\n{}\n\nCall quiz again with your guess letter.",
                quiz.art,
                choices.join("\n")
            ))
        }
    }
}

/// The `munch` tool: present a board, or grade a set of bites.
fn munch_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let board = numinous_core::build_board(seed, round);
    match args.get("bites").and_then(Value::as_array) {
        Some(raw) => {
            let bites: Vec<usize> = raw
                .iter()
                .filter_map(Value::as_u64)
                .filter(|&n| n >= 1)
                .map(|n| (n - 1) as usize)
                .collect();
            let outcome = numinous_core::grade_munch(&board, &bites);
            let verdict = if outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0
            {
                "PERFECT."
            } else {
                "Munched."
            };
            tool_structured(
                &format!(
                    "{verdict} {} eaten, {} bad bites, {} left behind. Score: {} (seed {seed}, round {round}).",
                    outcome.hits, outcome.bad_bites, outcome.left_behind, outcome.score
                ),
                json!({
                    "game": "munch",
                    "seed": seed,
                    "round": round,
                    "hits": outcome.hits,
                    "badBites": outcome.bad_bites,
                    "leftBehind": outcome.left_behind,
                    "wronglyEaten": outcome.wrongly_eaten,
                    "missed": outcome.missed,
                    "perfect": outcome.left_behind == 0 && outcome.bad_bites == 0 && outcome.hits > 0,
                    "score": outcome.score
                }),
            )
        }
        None => tool_text(&format!(
            "{}\n{}\nCall munch again with your bites (1-based cell numbers).",
            board.rule.describe(),
            numinous_core::board_text(&board)
        )),
    }
}

fn arcade_action(value: &Value) -> Option<numinous_core::munch_arcade::Action> {
    use numinous_core::munch_arcade::Action;
    Some(match value.as_str()?.to_ascii_lowercase().as_str() {
        "up" | "w" => Action::Up,
        "down" | "s" => Action::Down,
        "left" | "a" => Action::Left,
        "right" | "d" => Action::Right,
        "eat" | "e" => Action::Eat,
        _ => return None,
    })
}

fn replay_munch_arcade(args: &Value) -> Option<(numinous_core::munch_arcade::Arcade, bool)> {
    use numinous_core::munch_arcade::Turn;
    let seed = effective_seed(args);
    let mut run = numinous_core::munch_arcade::Arcade::new(seed);
    let actions = args.get("actions").and_then(Value::as_array)?;
    let mut cleared = false;
    for action in actions.iter().filter_map(arcade_action) {
        if matches!(run.turn(action), Turn::Cleared) {
            cleared = true;
        }
    }
    Some((run, cleared))
}

fn post_munch_arcade_score(
    args: &Value,
    scores_file: &std::path::Path,
) -> Option<(u64, i64, bool)> {
    let seed = effective_seed(args);
    let (run, cleared) = replay_munch_arcade(args)?;
    post_score(scores_file, &format!("arcade seed:{seed}"), run.score);
    Some((seed, run.score, cleared))
}

/// The `munch_arcade` tool: the full hunted arcade. Call with seed to see the board; call with "actions" list to replay the run (stateless). Returns text + structured state. Scores as "arcade seed:N".
fn munch_arcade_tool(args: &Value) -> Value {
    use numinous_core::munch_arcade::Turn;
    use numinous_core::munch_arcade::{Arcade, Mind};
    let seed = effective_seed(args);
    let mut run = Arcade::new(seed);
    let mut cleared = false;
    if let Some(raw) = args.get("actions").and_then(Value::as_array) {
        for action in raw.iter().filter_map(arcade_action) {
            if matches!(run.turn(action), Turn::Cleared) {
                cleared = true;
            }
        }
    }
    // Simple board text (dupe of cli for MCP independence)
    let mut board_text = String::new();
    for row in 0..numinous_core::munchers::ROWS {
        for col in 0..numinous_core::munchers::COLS {
            let cell = row * numinous_core::munchers::COLS + col;
            if cell == run.muncher {
                board_text.push_str("[@]");
            } else if let Some(v) = run.vexations.iter().find(|v| v.cell == cell) {
                let m = match v.mind {
                    Mind::Drifter => "d",
                    Mind::Tracker => "T",
                    Mind::Editor => "e",
                };
                board_text.push_str(&format!("[{}]", m));
            } else if run.eaten[cell] {
                board_text.push_str("[ ]");
            } else {
                board_text.push_str(&format!("[{:>2}]", run.board.numbers[cell]));
            }
        }
        board_text.push('\n');
    }
    let state_text = format!(
        "ARCADE seed {seed} LEVEL {} LIVES {} SCORE {}\nRULE: {}\n{}",
        run.level,
        run.lives,
        run.score,
        run.board.rule.describe(),
        board_text
    );
    tool_structured(
        &state_text,
        json!({
            "game": "arcade",
            "seed": seed,
            "level": run.level,
            "lives": run.lives,
            "score": run.score,
            "muncher": run.muncher,
            "vexations": run.vexations.iter().map(|v| json!({"cell": v.cell, "mind": format!("{:?}", v.mind)})).collect::<Vec<_>>(),
            // The rule to eat by and the board a mind reads ride in the
            // structured payload, not only the text state.
            "rule": run.board.rule.describe(),
            "board": board_text,
            "cleared": cleared,
            "over": run.lives == 0
        }),
    )
}

/// The `forget` tool: memory transparency, and erasure on explicit consent.
/// Everything this place remembers is two small text files; here is the proof.
fn forget_tool(
    args: &Value,
    journey_file: &std::path::Path,
    scores_file: &std::path::Path,
) -> Value {
    let confirm = args
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let also_scores = args.get("scores").and_then(Value::as_bool).unwrap_or(false);
    if !confirm {
        let journey = load_journey(journey_file);
        return tool_text(&format!(
            "Everything Numinous remembers about you:

             journey ({} rooms entered, {} wins, {} plays, {} secrets heard)
             scores ({} entries)

             That is all of it. Nothing else is kept, sent, or shared. Call again              with confirm true to erase the journey (add scores true to erase the              table too). Leaving is always allowed; so is staying.",
            journey.visited.len(),
            journey.wins,
            journey.plays,
            journey.secrets,
            numinous_core::load_scoreboard_file(scores_file).entries.len()
        ));
    }
    let _ = numinous_core::remove_persisted_file(journey_file);
    if also_scores {
        let _ = numinous_core::remove_persisted_file(scores_file);
    }
    tool_text(
        "Forgotten. The journey is erased; the constellation is dark again.          The rooms are all still here, whenever you like.",
    )
}

/// The `scores` tool: the shared high-score table, prose and structured.
fn scores_tool(path: &std::path::Path) -> Value {
    let board = numinous_core::load_scoreboard_file(path);
    if board.entries.is_empty() {
        return tool_text("No scores yet. Post one: munch, quiz.");
    }
    let mut lines = vec!["HIGH SCORES".to_string()];
    let mut structured = Vec::new();
    for (rank, (key, score)) in board.top(15).iter().enumerate() {
        lines.push(format!("  {:>2}.  {score:>6}  {key}", rank + 1));
        structured.push(json!({ "rank": rank + 1, "key": key, "score": score }));
    }
    tool_structured(&lines.join("\n"), json!({ "top": structured }))
}

/// The `nim` tool: replay the whole game from the move list, statelessly.
fn nim_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let mut heaps = numinous_core::nim_new(seed);
    let mut narration = Vec::new();
    let moves: Vec<(usize, u32)> = args
        .get("moves")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|m| {
                    let pair = m.as_array()?;
                    let heap = usize::try_from(pair.first()?.as_u64()?).ok()?;
                    // An oversized take saturates so the replay rejects it as
                    // the illegal move it is, instead of truncating it legal.
                    let take = u32::try_from(pair.get(1)?.as_u64()?).unwrap_or(u32::MAX);
                    Some((heap, take))
                })
                .collect()
        })
        .unwrap_or_default();
    for (heap, take) in moves {
        if heap == 0 || !numinous_core::nim_apply(&mut heaps, heap - 1, take) {
            return tool_error(&format!(
                "Illegal move: take {take} from heap {heap}. Heaps now: {heaps:?}."
            ));
        }
        if numinous_core::nim_finished(&heaps) {
            let secret = numinous_core::nim_secret();
            return tool_structured(
                &format!(
                    "You took the last stone. The Order concedes, and keeps its word:\n\n{secret}"
                ),
                // The promised secret lives in the structured payload too, so a
                // mind that reads only structuredContent still earns it.
                json!({ "game": "nim", "seed": seed, "won": true, "secret": secret }),
            );
        }
        let (oh, ot) = numinous_core::nim_order(&heaps);
        let _ = numinous_core::nim_apply(&mut heaps, oh, ot);
        narration.push(format!("The Order takes {ot} from heap {}.", oh + 1));
        if numinous_core::nim_finished(&heaps) {
            return tool_structured(
                "The Order takes the last stone. Again. (It is not luck.)",
                json!({ "game": "nim", "seed": seed, "won": false }),
            );
        }
    }
    let board: Vec<String> = heaps
        .iter()
        .enumerate()
        .map(|(i, &h)| format!("  {}) {}", i + 1, "O ".repeat(h as usize)))
        .collect();
    tool_structured(
        &format!(
            "NIM seed {seed}. Last stone wins.\n{}\n{}\nMove by calling again with your full move list.",
            narration.join("\n"),
            board.join("\n")
        ),
        // The Order's replies ride in the structured payload, so a mind that
        // reads only structuredContent can follow the game, not just the heaps.
        json!({ "game": "nim", "seed": seed, "heaps": heaps, "order": narration }),
    )
}

/// The `journey` tool: an agent's own level, sky, and standing.
fn journey_tool(path: &std::path::Path) -> Value {
    let journey = load_journey(path);
    let mut wall = String::new();
    for &(level, name, what) in numinous_core::UNLOCKS {
        if journey.level() >= level {
            wall.push_str(&format!("  OPEN    LV {level:>2}  {name}: {what}\n"));
        } else {
            wall.push_str(&format!("  LOCKED  LV {level:>2}  ???\n"));
        }
    }
    tool_structured(
        &format!(
            "LV {:>2}  [{}]  {} XP\n\n{}\n\n{} of {} stars lit. {} answered well. {} heard.\n{}\n\n{wall}",
            journey.level(),
            journey.level_bar(20),
            journey.sparks(),
            numinous_core::constellation(&journey, 60, 18),
            journey.visited.len(),
            all_rooms().len(),
            journey.wins,
            journey.secrets,
            journey.rank().name()
        ),
        json!({
            "level": journey.level(),
            "maxLevel": numinous_core::MAX_LEVEL,
            "xp": journey.sparks(),
            "starsLit": journey.visited.len(),
            "starsTotal": all_rooms().len(),
            "wins": journey.wins,
            "plays": journey.plays,
            "secrets": journey.secrets,
            "rank": journey.rank().name()
        }),
    )
}

/// The `plot_expression` tool: an agent creates in the Studio.
fn plot_expression_tool(args: &Value) -> Value {
    use std::f64::consts::TAU;
    let Some(expr) = args.get("expr").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'expr'.");
    };
    let xmin = args.get("xmin").and_then(Value::as_f64).unwrap_or(-TAU);
    let xmax = args.get("xmax").and_then(Value::as_f64).unwrap_or(TAU);
    let a = args.get("a").and_then(Value::as_f64).unwrap_or(1.0);
    match numinous_core::plot_text(expr, xmin, xmax, a, 72, 26) {
        Ok((text, ymin, ymax)) => tool_text(&format!(
            "y = {expr}    x in [{xmin:.3}, {xmax:.3}]    y in [{ymin:.3}, {ymax:.3}]\n\n{text}"
        )),
        Err(message) => tool_error(&message),
    }
}

/// The `sing_expression` tool: an agent's function becomes readable music.
fn sing_expression_tool(args: &Value) -> Value {
    use std::f64::consts::TAU;
    let Some(source) = args.get("expr").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'expr'.");
    };
    let notes = args.get("notes").and_then(Value::as_u64).unwrap_or(24) as usize;
    let expr = match numinous_core::parse(source) {
        Ok(expr) => expr,
        Err(message) => return tool_error(&message),
    };
    let spec = numinous_core::to_melody(&expr, -TAU, TAU, notes.clamp(1, 64), 1.0);
    let mut lines = vec![format!(
        "y = {source} as a melody: {:.1}s, {} notes.",
        spec.duration,
        spec.notes.len()
    )];
    for (i, note) in spec.notes.iter().enumerate() {
        lines.push(format!(
            "  note {:>2}: {:>7.1} Hz ({:>3})  at {:>5.2}s",
            i + 1,
            note.freq,
            note_name(note.freq),
            note.start
        ));
    }
    tool_text(&lines.join("\n"))
}

/// The `explain_joke` tool: humor as structure, for the alien and the agent.
fn explain_joke_tool(args: &Value) -> Value {
    match args.get("index").and_then(Value::as_u64) {
        Some(index) => match numinous_core::explain_joke(index as usize) {
            Some(joke) => tool_text(&format!(
                "Specimen {index}: \"{}\"\nHabitat: {}.\nMechanism: {}",
                joke.text, joke.habitat, joke.mechanism
            )),
            None => tool_error(&format!(
                "No specimen {index}. There are {} catalogued jokes.",
                numinous_core::jokes().len()
            )),
        },
        None => {
            let mut lines =
                vec!["The catalogued jokes (a joke explained is a frog dissected):".to_string()];
            for (i, joke) in numinous_core::jokes().iter().enumerate() {
                lines.push(format!("  {i}: \"{}\"  ({})", joke.text, joke.habitat));
            }
            lines.push("Call again with an index for the dissection.".to_string());
            tool_text(&lines.join("\n"))
        }
    }
}

fn unknown_sim(id: &str) -> String {
    let known: Vec<&str> = numinous_core::all_sims()
        .iter()
        .map(|s| s.meta().id)
        .collect();
    format!("No sim with id '{id}'. Known sims: {}", known.join(", "))
}

fn list_rooms_text() -> String {
    all_rooms()
        .iter()
        .map(|room| {
            let m = room.meta();
            format!("{}  [{}]  {}", m.id, m.wing, m.title)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn unknown_room(id: &str) -> String {
    let known: Vec<&str> = all_rooms().iter().map(|r| r.meta().id).collect();
    format!("No room with id '{id}'. Known rooms: {}", known.join(", "))
}

/// A successful tool result carrying text content.
fn tool_text(text: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ], "isError": false })
}

/// A successful tool result carrying text plus machine-readable structured
/// content (per the 2025-06-18 spec), so agents and leaderboards can consume
/// scores and state without parsing prose.
fn tool_structured(text: &str, structured: Value) -> Value {
    json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": structured,
        "isError": false
    })
}

/// A tool result that reports an error to the model (guiding, not fatal).
fn tool_error(text: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ], "isError": true })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::{handle_request, handle_request_with, render_delta_json};
    use serde_json::json;

    #[test]
    fn initialize_returns_server_info() {
        let resp =
            handle_request(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))
                .expect("initialize is a request and must respond");
        assert_eq!(resp["result"]["serverInfo"]["name"], "numinous");
        assert!(resp["result"]["protocolVersion"].is_string());
    }

    #[test]
    fn tools_list_has_the_expected_tools() {
        let resp = handle_request(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .expect("tools/list must respond");
        let tools = resp["result"]["tools"]
            .as_array()
            .expect("tools is an array");
        assert_eq!(tools.len(), 29);
        assert!(
            tools
                .iter()
                .filter_map(|t| t["name"].as_str())
                .any(|name| name == "challenge")
        );
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"predict"));
        assert!(names.contains(&"cairn"));
        assert!(names.contains(&"reveal_room"));
        assert!(names.contains(&"run_sim"));
        assert!(names.contains(&"quiz"));
        assert!(names.contains(&"listen_room"));
        assert!(names.contains(&"plot_expression"));
        assert!(names.contains(&"sing_expression"));
        assert!(names.contains(&"explain_joke"));
        assert!(names.contains(&"journey"));
        assert!(names.contains(&"munch"));
        assert!(names.contains(&"munch_arcade"));
        assert!(names.contains(&"scores"));
        assert!(names.contains(&"forget"));
        assert!(names.contains(&"nim"));
        let play_room = tools
            .iter()
            .find(|tool| tool["name"] == "play_room")
            .expect("play_room tool");
        let poke_schema = &play_room["inputSchema"]["properties"]["pokes"];
        assert_eq!(poke_schema["maxItems"], numinous_core::MAX_ROOM_POKES);
        assert_eq!(poke_schema["items"]["items"]["minimum"], 0);
        assert_eq!(poke_schema["items"]["items"]["maximum"], 1);
        for tool in [
            "crack",
            "seti",
            "aliens",
            "gauntlet",
            "choose",
            "trophies",
            "hackenbush",
            "party",
            "fifteen",
        ] {
            assert!(names.contains(&tool), "{tool} is a tool");
        }
    }

    #[test]
    fn crack_presents_replays_and_defuses() {
        let clue = handle_request(&json!({
            "jsonrpc":"2.0","id":90,"method":"tools/call",
            "params":{"name":"crack","arguments":{"seed":5}}
        }))
        .expect("must respond");
        assert_eq!(clue["result"]["isError"], false);
        let text = clue["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Clue:"));
        // The known code for seed 5 with 4 digits (from the CLI e2e): 9500.
        let win = handle_request(&json!({
            "jsonrpc":"2.0","id":91,"method":"tools/call",
            "params":{"name":"crack","arguments":{"seed":5,"guesses":["1234","9500"]}}
        }))
        .expect("must respond");
        let text = win["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("DEFUSED"), "{text}");
        assert_eq!(win["result"]["structuredContent"]["defused"], true);
    }

    #[test]
    fn seti_and_aliens_present_then_grade() {
        let scan = handle_request(&json!({
            "jsonrpc":"2.0","id":92,"method":"tools/call",
            "params":{"name":"seti","arguments":{"seed":3}}
        }))
        .expect("must respond");
        assert!(
            scan["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("THE SKY")
        );
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":93,"method":"tools/call",
            "params":{"name":"seti","arguments":{"seed":3,"guess":"A"}}
        }))
        .expect("must respond");
        assert!(
            graded["result"]["structuredContent"]["correct"].is_boolean(),
            "graded either way"
        );
        let signal = handle_request(&json!({
            "jsonrpc":"2.0","id":94,"method":"tools/call",
            "params":{"name":"aliens","arguments":{"seed":2}}
        }))
        .expect("must respond");
        let terms = signal["result"]["structuredContent"]["terms"]
            .as_array()
            .expect("terms shown");
        assert!(!terms.is_empty());
    }

    #[test]
    fn the_gauntlet_presents_four_stages_and_grades_a_run() {
        let stages = handle_request(&json!({
            "jsonrpc":"2.0","id":95,"method":"tools/call",
            "params":{"name":"gauntlet","arguments":{"seed":5}}
        }))
        .expect("must respond");
        let text = stages["result"]["content"][0]["text"].as_str().unwrap();
        for stage in ["MUNCH", "THE SHAPE", "THE SKY", "THE BOMB"] {
            assert!(text.contains(stage), "{stage} in {text}");
        }
        let run = handle_request(&json!({
            "jsonrpc":"2.0","id":96,"method":"tools/call",
            "params":{"name":"gauntlet","arguments":{"seed":5,"answers":{
                "bites":[1,2],"shape":"A","sky":"B","wires":["9500"]
            }}}
        }))
        .expect("must respond");
        let sc = &run["result"]["structuredContent"];
        assert_eq!(sc["game"], "gauntlet");
        assert!(sc["total"].as_i64().is_some());
        assert!(sc["clean"].as_u64().is_some());
    }

    #[test]
    fn the_new_games_present_grade_and_guide() {
        let garden = handle_request(&json!({
            "jsonrpc":"2.0","id":110,"method":"tools/call",
            "params":{"name":"hackenbush","arguments":{"seed":2}}
        }))
        .expect("must respond");
        assert!(
            garden["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("winnable")
        );
        let bad = handle_request(&json!({
            "jsonrpc":"2.0","id":111,"method":"tools/call",
            "params":{"name":"hackenbush","arguments":{"seed":2,"moves":[[99,1]]}}
        }))
        .expect("must respond");
        assert_eq!(bad["result"]["isError"], true, "illegal cuts guide");

        let escaped = handle_request(&json!({
            "jsonrpc":"2.0","id":112,"method":"tools/call",
            "params":{"name":"party","arguments":{"guests":5,"shakes":[
                [1,2,"r"],[2,3,"r"],[3,4,"r"],[4,5,"r"],[5,1,"r"],
                [1,3,"b"],[2,4,"b"],[3,5,"b"],[4,1,"b"],[5,2,"b"]
            ]}}
        }))
        .expect("must respond");
        assert_eq!(
            escaped["result"]["structuredContent"]["escaped"], true,
            "the pentagon's escape works over MCP"
        );

        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":113,"method":"tools/call",
            "params":{"name":"fifteen","arguments":{"seed":3,"rounds":3,"calls":["S","S","S"]}}
        }))
        .expect("must respond");
        assert!(
            graded["result"]["structuredContent"]["correct"]
                .as_u64()
                .is_some()
        );
    }

    #[test]
    fn choose_and_trophies_read_the_record() {
        let file = std::env::temp_dir().join("numinous_mcp_choose_test.txt");
        let journey = numinous_core::Journey {
            plays: 3, // level 3: two boons banked
            ..Default::default()
        };
        let _ = std::fs::write(&file, journey.to_text());
        let menu = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":97,"method":"tools/call",
                "params":{"name":"choose","arguments":{}}
            }),
            &file,
        )
        .expect("must respond");
        let options = menu["result"]["structuredContent"]["options"]
            .as_array()
            .expect("a menu")
            .len();
        assert_eq!(options, 3);
        let spent = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":98,"method":"tools/call",
                "params":{"name":"choose","arguments":{"pick":2}}
            }),
            &file,
        )
        .expect("must respond");
        assert!(
            spent["result"]["structuredContent"]["chosen"]
                .as_str()
                .unwrap()
                .starts_with("cut:")
        );
        let case = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":99,"method":"tools/call",
                "params":{"name":"trophies","arguments":{}}
            }),
            &file,
        )
        .expect("must respond");
        assert!(
            case["result"]["structuredContent"]["total"]
                .as_u64()
                .unwrap()
                >= 18
        );
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn nim_replays_statelessly_and_teaches_on_victory() {
        let opening = handle_request(&json!({
            "jsonrpc":"2.0","id":80,"method":"tools/call",
            "params":{"name":"nim","arguments":{"seed":3}}
        }))
        .expect("must respond");
        let heaps = opening["result"]["structuredContent"]["heaps"]
            .as_array()
            .expect("heaps")
            .clone();
        assert_eq!(heaps.len(), 3);
        // Play the Order's own strategy against it: compute the zeroing move.
        let h: Vec<u32> = heaps.iter().map(|v| v.as_u64().unwrap() as u32).collect();
        let x = h.iter().fold(0u32, |a, &v| a ^ v);
        let (i, take) = h
            .iter()
            .enumerate()
            .find_map(|(i, &v)| ((v ^ x) < v).then(|| (i, v - (v ^ x))))
            .expect("a winning move exists: the openings are winnable");
        let reply = handle_request(&json!({
            "jsonrpc":"2.0","id":81,"method":"tools/call",
            "params":{"name":"nim","arguments":{"seed":3,"moves":[[i+1,take]]}}
        }))
        .expect("must respond");
        assert_eq!(reply["result"]["isError"], false);
        // Either the game continues deterministically or it is already won.
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Order") || text.contains("secret"));

        let bad = handle_request(&json!({
            "jsonrpc":"2.0","id":82,"method":"tools/call",
            "params":{"name":"nim","arguments":{"seed":3,"moves":[[9,1]]}}
        }))
        .expect("must respond");
        assert_eq!(bad["result"]["isError"], true);
    }

    #[test]
    fn forget_shows_first_and_erases_only_on_consent() {
        let journey = std::env::temp_dir().join("numinous_mcp_forget_journey.txt");
        let scores = std::env::temp_dir().join("numinous_mcp_forget_scores.txt");
        std::fs::write(
            &journey,
            "visited lorenz
wins 1
secrets 0
plays 2
",
        )
        .unwrap();
        std::fs::write(
            &scores,
            "50	munch seed:1 board:0
",
        )
        .unwrap();

        // Transparency first: no args means show, not erase.
        let shown = super::forget_tool(&json!({}), &journey, &scores);
        let text = shown["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("1 rooms entered") || text.contains("1 wins"));
        assert!(text.contains("Nothing else is kept"));
        assert!(journey.exists(), "nothing was erased without consent");

        // Consent erases the journey; scores stay unless asked.
        let _ = super::forget_tool(&json!({"confirm": true}), &journey, &scores);
        assert!(!journey.exists());
        assert!(scores.exists());
        let _ = super::forget_tool(&json!({"confirm": true, "scores": true}), &journey, &scores);
        assert!(!scores.exists());
    }

    #[test]
    fn scores_post_and_rank_across_minds() {
        let path = std::env::temp_dir().join("numinous_mcp_scores_test.txt");
        let _ = std::fs::remove_file(&path);
        assert!(super::post_score(&path, "munch seed:7 board:0", 40));
        assert!(!super::post_score(&path, "munch seed:7 board:0", 10));
        assert!(super::post_score(&path, "munch seed:7 board:0", 90));
        let resp = super::scores_tool(&path);
        let text = resp["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("HIGH SCORES"));
        assert!(text.contains("90"));
        assert_eq!(resp["structuredContent"]["top"][0]["score"], 90);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn game_results_carry_structured_content_for_leaderboards() {
        let all: Vec<u64> = (1..=30).collect();
        let munched = handle_request(&json!({
            "jsonrpc":"2.0","id":70,"method":"tools/call",
            "params":{"name":"munch","arguments":{"seed":7,"round":0,"bites":all}}
        }))
        .expect("tools/call must respond");
        let s = &munched["result"]["structuredContent"];
        assert_eq!(s["game"], "munch");
        assert!(s["score"].is_i64() || s["score"].is_u64());
        assert_eq!(s["leftBehind"], 0);

        let quizzed = handle_request(&json!({
            "jsonrpc":"2.0","id":71,"method":"tools/call",
            "params":{"name":"quiz","arguments":{"seed":7,"round":0,"guess":"A"}}
        }))
        .expect("tools/call must respond");
        let s = &quizzed["result"]["structuredContent"];
        assert!(s["correct"].is_boolean());
        assert!(s["answerTitle"].is_string());
    }

    #[test]
    fn munch_presents_then_grades_the_same_board_for_everyone() {
        let shown = handle_request(&json!({
            "jsonrpc":"2.0","id":60,"method":"tools/call",
            "params":{"name":"munch","arguments":{"seed":7,"round":0}}
        }))
        .expect("tools/call must respond");
        let text = shown["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Eat the"), "got: {text}");
        assert!(text.contains("[ 1]"));

        // Eat everything: hits plus every possible bad bite, scored deterministically.
        let all: Vec<u64> = (1..=30).collect();
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":61,"method":"tools/call",
            "params":{"name":"munch","arguments":{"seed":7,"round":0,"bites":all}}
        }))
        .expect("tools/call must respond");
        let text = graded["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Score:"), "got: {text}");
        assert!(text.contains("0 left behind"));
    }

    #[test]
    fn an_agent_earns_xp_and_sees_its_level() {
        // Hermetic: an explicit temp journey file, no environment involved.
        let path = std::env::temp_dir().join("numinous_mcp_journey_test.txt");
        let _ = std::fs::remove_file(&path);

        super::record_progress(
            &json!({
                "jsonrpc":"2.0","id":50,"method":"tools/call",
                "params":{"name":"run_sim","arguments":{"id":"wing"}}
            }),
            &path,
        );
        super::record_progress(
            &json!({
                "jsonrpc":"2.0","id":51,"method":"tools/call",
                "params":{"name":"play_room","arguments":{"id":"lorenz"}}
            }),
            &path,
        );
        let resp = super::handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":52,"method":"tools/call",
                "params":{"name":"journey","arguments":{}}
            }),
            &path,
        )
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("LV"), "got: {text}");
        assert!(text.contains("2 XP"), "a play and a visit: {text}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_agent_can_create_in_the_studio() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":40,"method":"tools/call",
            "params":{"name":"plot_expression","arguments":{"expr":"sin(3*x) + x/2"}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains('#'), "the curve has ink");
        assert_eq!(resp["result"]["isError"], false);

        let bad = handle_request(&json!({
            "jsonrpc":"2.0","id":41,"method":"tools/call",
            "params":{"name":"plot_expression","arguments":{"expr":"sin("}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad["result"]["isError"], true);

        // A crafted deeply nested expression must return an error, never
        // overflow the stack and abort the server (a Rust stack overflow is
        // uncatchable). Both studio-parsing tools share the guarded parser.
        let deep = format!("{}1{}", "(".repeat(5000), ")".repeat(5000));
        for tool in ["plot_expression", "sing_expression"] {
            let bomb = handle_request(&json!({
                "jsonrpc":"2.0","id":41,"method":"tools/call",
                "params":{"name":tool,"arguments":{"expr":deep}}
            }))
            .expect("tools/call must respond, not crash");
            assert_eq!(bomb["result"]["isError"], true, "{tool} rejects the bomb");
        }
    }

    #[test]
    fn an_agent_can_sing_its_own_function() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":42,"method":"tools/call",
            "params":{"name":"sing_expression","arguments":{"expr":"x","notes":8}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("8 notes"), "got: {text}");
        assert!(text.contains("Hz"));
    }

    #[test]
    fn the_jokes_can_be_dissected() {
        let list = handle_request(&json!({
            "jsonrpc":"2.0","id":43,"method":"tools/call",
            "params":{"name":"explain_joke","arguments":{}}
        }))
        .expect("tools/call must respond");
        let text = list["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("frog"), "the warning is part of the joke");

        let one = handle_request(&json!({
            "jsonrpc":"2.0","id":44,"method":"tools/call",
            "params":{"name":"explain_joke","arguments":{"index":1}}
        }))
        .expect("tools/call must respond");
        let text = one["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Mechanism:"), "got: {text}");
    }

    #[test]
    fn listen_room_returns_readable_notation() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":30,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"times-tables","t":0.0}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Hz"), "got: {text}");
        assert!(
            text.contains("1 notes"),
            "the times-tables default tone has one note"
        );
        assert!(text.contains("Motif:"), "got: {text}");
        assert!(
            text.contains("D minor pentatonic") && text.contains("D3 G3 A3 D4"),
            "interactive room motifs must surface readable notation: {text}"
        );
        assert_eq!(resp["result"]["isError"], false);

        let tuned = handle_request(&json!({
            "jsonrpc":"2.0","id":302,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"lissajous","t":0.0}}
        }))
        .expect("tools/call must respond");
        let tuned_text = tuned["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            tuned_text.contains("G visible fifth") && tuned_text.contains("G3 D4 G4"),
            "room motifs must surface readable notation: {tuned_text}"
        );

        let varied = handle_request(&json!({
            "jsonrpc":"2.0","id":301,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"times-tables","t":0.0,"variation":42}}
        }))
        .expect("tools/call must respond");
        let varied_text = varied["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert_ne!(text, varied_text, "listen_room must honor variation");
    }

    #[test]
    fn invalid_tools_do_not_record_progress() {
        let file = std::env::temp_dir().join("numinous_mcp_invalid_progress_test.txt");
        let _ = std::fs::remove_file(&file);
        for (id, name, arguments) in [
            (401, "run_sim", json!({"id": "no-such-sim"})),
            (402, "plot_expression", json!({"expr": "sin("})),
            (403, "sing_expression", json!({"expr": "sin("})),
        ] {
            let resp = handle_request_with(
                &json!({
                    "jsonrpc":"2.0","id":id,"method":"tools/call",
                    "params":{"name":name,"arguments":arguments}
                }),
                &file,
            )
            .expect("tools/call must respond");
            assert_eq!(resp["result"]["isError"], true);
        }
        let journey = std::fs::read_to_string(&file)
            .map(|text| numinous_core::Journey::from_text(&text))
            .unwrap_or_default();
        assert_eq!(journey.plays, 0);
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn munch_arcade_replay_posts_the_cli_score_key() {
        let path = std::env::temp_dir().join("numinous_mcp_arcade_scores_test.txt");
        let _ = std::fs::remove_file(&path);
        let posted = super::post_munch_arcade_score(
            &json!({"seed": 7, "actions": ["right", "eat", "down"]}),
            &path,
        )
        .expect("actions replay");
        assert_eq!(posted.0, 7);
        let table = super::scores_tool(&path);
        let text = table["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("arcade seed:7"), "got: {text}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn munch_arcade_replay_reports_clear_events() {
        fn sweep_actions() -> Vec<&'static str> {
            let mut actions = vec!["eat"];
            for row in 0..numinous_core::munchers::ROWS {
                let across = numinous_core::munchers::COLS - 1;
                let step = if row % 2 == 0 { "right" } else { "left" };
                for _ in 0..across {
                    actions.push(step);
                    actions.push("eat");
                }
                if row + 1 < numinous_core::munchers::ROWS {
                    actions.push("down");
                    actions.push("eat");
                }
            }
            actions
        }

        let path = std::env::temp_dir().join("numinous_mcp_arcade_clear_test.txt");
        let _ = std::fs::remove_file(&path);
        let actions = sweep_actions();
        let mut cleared = false;
        for seed in 1..=200 {
            let Some((_, _, did_clear)) = super::post_munch_arcade_score(
                &json!({"seed": seed, "actions": actions.clone()}),
                &path,
            ) else {
                continue;
            };
            if did_clear {
                cleared = true;
                break;
            }
        }
        assert!(
            cleared,
            "at least one deterministic replay must clear a board"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn note_names_are_correct() {
        assert_eq!(super::note_name(440.0), "A4");
        assert_eq!(super::note_name(880.0), "A5");
        assert_eq!(super::note_name(261.63), "C4");
        assert_eq!(super::note_name(0.0), "-");
    }

    #[test]
    fn hidden_names_whisper_over_mcp_too() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":31,"method":"tools/call",
            "params":{"name":"describe_room","arguments":{"id":"hippasus"}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], false);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("sea"), "got: {text}");
    }

    #[test]
    fn list_sims_tool_lists_them() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":20,"method":"tools/call",
            "params":{"name":"list_sims"}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("tribbles"));
    }

    #[test]
    fn run_sim_returns_a_picture_and_readout() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":21,"method":"tools/call",
            "params":{"name":"run_sim","arguments":{"id":"wing","params":{"angle-of-attack":20}}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("STALL"), "got: {text}");
        assert_eq!(resp["result"]["isError"], false);
    }

    #[test]
    fn run_sim_unknown_is_a_guiding_error() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":22,"method":"tools/call",
            "params":{"name":"run_sim","arguments":{"id":"no-such-sim"}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], true);
        assert!(
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Known sims")
        );
    }

    #[test]
    fn quiz_tool_presents_then_grades() {
        let puzzle = handle_request(&json!({
            "jsonrpc":"2.0","id":23,"method":"tools/call",
            "params":{"name":"quiz","arguments":{"seed":7,"round":0}}
        }))
        .expect("tools/call must respond");
        assert!(
            puzzle["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Guess the shape")
        );
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":24,"method":"tools/call",
            "params":{"name":"quiz","arguments":{"seed":7,"round":0,"guess":"A"}}
        }))
        .expect("tools/call must respond");
        assert!(
            graded["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("The answer was")
        );
    }

    #[test]
    fn reveal_room_returns_the_insight() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":15,"method":"tools/call",
            "params":{"name":"reveal_room","arguments":{"id":"times-tables"}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Mandelbrot"));
    }

    #[test]
    fn play_room_returns_ascii_the_agent_can_see() {
        let expected_action =
            numinous_core::room_action(numinous_core::room_by_id("times-tables").unwrap().as_ref());
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"times-tables","width":40,"height":20}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(text.contains('*'), "the render should contain ink");
        assert!(text.contains(&format!("Action: {expected_action}")));
        assert_eq!(
            resp["result"]["structuredContent"]["action"],
            expected_action
        );
        // The picture must also ride in structuredContent, so a mind on a
        // client that surfaces only the JSON still sees the math, not just its
        // metadata. This is the playtest finding made a standing contract.
        let render = resp["result"]["structuredContent"]["render"]
            .as_str()
            .expect("structuredContent carries the render");
        assert!(render.contains('*'), "the structured render has ink too");
        assert_eq!(resp["result"]["isError"], false);
    }

    #[test]
    fn the_reasoning_survives_in_structured_content() {
        // A structured-content client drops the text block, so every graded
        // game's teaching payload must also live in structuredContent. This
        // pins the fix for the July 2026 playtest's core finding.
        let sc = |resp: &serde_json::Value| resp["result"]["structuredContent"].clone();

        // Nim: beating the Order must deliver the secret in the JSON. Seed 3's
        // opening is winnable (the existing victory test relies on it too).
        let win = handle_request(&json!({
            "jsonrpc":"2.0","id":60,"method":"tools/call",
            "params":{"name":"nim","arguments":{"seed":3,"moves":winning_nim_moves(3)}}
        }))
        .expect("tools/call must respond");
        assert_eq!(sc(&win)["won"], true);
        assert!(
            sc(&win)["secret"].as_str().unwrap_or_default().len() > 8,
            "the promised secret rides in structuredContent"
        );

        // Quiz: a graded guess carries the "why" in the JSON, right or wrong.
        let quiz = handle_request(&json!({
            "jsonrpc":"2.0","id":61,"method":"tools/call",
            "params":{"name":"quiz","arguments":{"seed":7,"round":0,"guess":"A"}}
        }))
        .expect("tools/call must respond");
        assert!(
            sc(&quiz)["why"].as_str().unwrap_or_default().len() > 8,
            "the quiz explanation rides in structuredContent"
        );

        // Seti: the pose carries the channel traces a mind must read.
        let sky = handle_request(&json!({
            "jsonrpc":"2.0","id":62,"method":"tools/call",
            "params":{"name":"seti","arguments":{"seed":7,"channels":4}}
        }))
        .expect("tools/call must respond");
        let sky_sc = sc(&sky);
        let channels = sky_sc["channels"].as_array().expect("channel rows");
        assert_eq!(channels.len(), 4);
        assert!(
            channels[0]["trace"].as_str().is_some(),
            "each channel's trace is in structuredContent"
        );

        // Crack: a guess carries its locked/loose signal in the JSON.
        let bomb = handle_request(&json!({
            "jsonrpc":"2.0","id":63,"method":"tools/call",
            "params":{"name":"crack","arguments":{"seed":7,"digits":4,"guesses":["1234"]}}
        }))
        .expect("tools/call must respond");
        let bomb_sc = sc(&bomb);
        let rows = bomb_sc["feedback"].as_array().expect("feedback rows");
        assert_eq!(rows.len(), 1);
        assert!(rows[0]["locked"].is_number() && rows[0]["loose"].is_number());

        // Aliens: the grade carries the sequence's explanation.
        let aliens = handle_request(&json!({
            "jsonrpc":"2.0","id":64,"method":"tools/call",
            "params":{"name":"aliens","arguments":{"seed":7,"guess":"1"}}
        }))
        .expect("tools/call must respond");
        assert!(
            sc(&aliens)["why"].as_str().unwrap_or_default().len() > 4,
            "the aliens explanation rides in structuredContent"
        );

        // Fifteen: the pose carries the scramble boards to read.
        let fifteen = handle_request(&json!({
            "jsonrpc":"2.0","id":65,"method":"tools/call",
            "params":{"name":"fifteen","arguments":{"seed":7,"rounds":3}}
        }))
        .expect("tools/call must respond");
        let fifteen_sc = sc(&fifteen);
        assert_eq!(
            fifteen_sc["scrambles"].as_array().map(Vec::len),
            Some(3),
            "the scramble boards ride in structuredContent"
        );

        // Gauntlet: the pose carries the whole four-stage puzzle.
        let gauntlet = handle_request(&json!({
            "jsonrpc":"2.0","id":66,"method":"tools/call",
            "params":{"name":"gauntlet","arguments":{"seed":7}}
        }))
        .expect("tools/call must respond");
        let gauntlet_sc = sc(&gauntlet);
        assert!(
            gauntlet_sc["munch"]["board"].as_str().is_some()
                && gauntlet_sc["shape"]["art"].as_str().is_some()
                && gauntlet_sc["sky"].as_array().is_some_and(|s| !s.is_empty())
                && gauntlet_sc["bomb"]["clue"].as_str().is_some(),
            "every gauntlet stage rides in structuredContent"
        );
    }

    /// A move list that beats the Order at nim for the given seed, found by
    /// replaying optimal xor-reducing play, so the win test cannot go stale if
    /// the seeded heaps change.
    fn winning_nim_moves(seed: u64) -> Vec<serde_json::Value> {
        let mut heaps = numinous_core::nim_new(seed);
        let mut moves = Vec::new();
        loop {
            let nim_sum = heaps.iter().fold(0u32, |acc, &h| acc ^ h);
            // The winning move exists whenever the position is not already lost
            // (nonzero xor), which a seeded start guarantees.
            let Some((heap, take)) = heaps.iter().enumerate().find_map(|(i, &h)| {
                let target = h ^ nim_sum;
                (target < h).then(|| (i, h - target))
            }) else {
                // A balanced position (nim_sum 0) has no winning move; from a
                // winnable opening we never reach one at our turn, so just stop.
                return moves;
            };
            moves.push(json!([heap + 1, take]));
            numinous_core::nim_apply(&mut heaps, heap, take);
            if numinous_core::nim_finished(&heaps) {
                return moves;
            }
            let (oh, ot) = numinous_core::nim_order(&heaps);
            numinous_core::nim_apply(&mut heaps, oh, ot);
            if numinous_core::nim_finished(&heaps) {
                // The Order took the last stone; unreachable under optimal play,
                // but return what we have rather than loop forever.
                return moves;
            }
        }
    }

    #[test]
    fn play_room_actions_always_name_the_verb() {
        // Every catalog room answers the hand now; the action an agent sees
        // is the room's own verb, never the generic fallback.
        for room in numinous_core::all_rooms() {
            let id = room.meta().id;
            let resp = handle_request(&json!({
                "jsonrpc":"2.0","id":31,"method":"tools/call",
                "params":{"name":"play_room","arguments":{"id":id,"width":40,"height":20}}
            }))
            .expect("tools/call must respond");
            assert_eq!(
                resp["result"]["structuredContent"]["action"],
                room.verb().expect("all catalog rooms have verbs"),
                "{id} leads with its verb"
            );
        }
    }

    #[test]
    fn play_room_accepts_stateless_hand_points() {
        let resting = handle_request(&json!({
            "jsonrpc":"2.0","id":32,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"double-pendulum","width":50,"height":30,"t":0.25}}
        }))
        .expect("tools/call must respond");
        let poked = handle_request(&json!({
            "jsonrpc":"2.0","id":33,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"double-pendulum","width":50,"height":30,"t":0.25,"pokes":[[0.2,0.8]]}}
        }))
        .expect("tools/call must respond");
        // Compare only the frame bodies: the poked header always differs now
        // (it carries the Touch line), so a whole-text comparison would pass
        // even for a room that ignored its hand points.
        let frame_of = |resp: &serde_json::Value| -> String {
            let text = resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            text.split_once("\n\n")
                .map(|(_, frame)| frame)
                .unwrap_or_default()
                .to_string()
        };
        assert_ne!(
            frame_of(&resting),
            frame_of(&poked),
            "a supplied hand point should steer the frame"
        );
        assert_eq!(poked["result"]["structuredContent"]["pokes"][0][0], 0.2);
        assert_eq!(poked["result"]["structuredContent"]["pokes"][0][1], 0.8);
        assert_eq!(poked["result"]["isError"], false);
        assert_eq!(
            resting["result"]["structuredContent"]["delta"],
            serde_json::Value::Null,
            "an unpoked render carries no delta"
        );
    }

    #[test]
    fn play_room_pokes_report_a_structured_delta() {
        let poked = handle_request(&json!({
            "jsonrpc":"2.0","id":36,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"double-pendulum","width":50,"height":30,"t":0.25,"pokes":[[0.2,0.8]]}}
        }))
        .expect("tools/call must respond");
        let delta = &poked["result"]["structuredContent"]["delta"];
        let changed = delta["cells_changed"].as_u64().expect("cells_changed");
        assert!(changed > 0, "the hand must measurably change the frame");
        assert_eq!(
            changed,
            delta["ink_added"].as_u64().unwrap_or_default()
                + delta["ink_removed"].as_u64().unwrap_or_default()
                + delta["ink_reshaped"].as_u64().unwrap_or_default(),
            "the change classification must sum to the change count"
        );
        assert_eq!(delta["total_cells"], 50 * 30);
        let region = delta["changed_region"]
            .as_array()
            .expect("a nonzero delta has a bounding region");
        assert_eq!(region.len(), 4);
        let text = poked["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains(&format!("Touch: {changed} of {} cells answered", 50 * 30)),
            "the text face speaks the same numbers: {text}"
        );
    }

    #[test]
    fn challenge_poses_then_grades_with_metrics_not_binary() {
        let posed = handle_request(&json!({
            "jsonrpc":"2.0","id":40,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","seed":7}}
        }))
        .expect("tools/call must respond");
        assert_eq!(posed["result"]["isError"], false);
        let sc = &posed["result"]["structuredContent"];
        assert_eq!(sc["game"], "challenge");
        let target = sc["target"].as_array().expect("target box");
        assert_eq!(target.len(), 4);
        let min_cells = sc["minCells"].as_u64().expect("threshold");
        assert!(min_cells >= 2);
        let text = posed["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("CELLS CHANGE INSIDE"),
            "goal is spoken: {text}"
        );

        // Aim at the target center: the graded attempt reports every metric.
        let cx = (target[0].as_f64().unwrap() + target[2].as_f64().unwrap())
            / 2.0
            / (sc["width"].as_f64().unwrap() - 1.0);
        let cy = (target[1].as_f64().unwrap() + target[3].as_f64().unwrap())
            / 2.0
            / (sc["height"].as_f64().unwrap() - 1.0);
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":41,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","seed":7,"pokes":[[cx, cy]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(graded["result"]["isError"], false);
        let grade = &graded["result"]["structuredContent"];
        assert!(grade["cellsChanged"].as_u64().expect("cells") > 0);
        assert!(grade["score"].as_u64().expect("score") > 0);
        assert!(grade["centerDistance"].is_number());
        assert!(grade["passed"].is_boolean());
        // Determinism: the same attempt earns the same grade.
        let again = handle_request(&json!({
            "jsonrpc":"2.0","id":42,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","seed":7,"pokes":[[cx, cy]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(grade, &again["result"]["structuredContent"]);
    }

    #[test]
    fn challenge_guides_away_from_quiet_rooms_and_bad_input() {
        // Derive a verbless room from the registry so this test cannot go
        // vacuous if a hardcoded room later gains a verb.
        if let Some(quiet_room) = numinous_core::all_rooms()
            .into_iter()
            .find(|room| room.verb().is_none())
        {
            let quiet = handle_request(&json!({
                "jsonrpc":"2.0","id":43,"method":"tools/call",
                "params":{"name":"challenge","arguments":{"id":quiet_room.meta().id}}
            }))
            .expect("tools/call must respond");
            assert_eq!(quiet["result"]["isError"], true);
            let text = quiet["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(text.contains("touch verb"), "guides the agent: {text}");
        }
        let bad = handle_request(&json!({
            "jsonrpc":"2.0","id":44,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","pokes":[[1.5,0.5]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad["result"]["isError"], true);
        let unknown = handle_request(&json!({
            "jsonrpc":"2.0","id":45,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"no-such-room"}}
        }))
        .expect("tools/call must respond");
        assert_eq!(unknown["result"]["isError"], true);
    }

    #[test]
    fn a_challenge_attempt_records_play_win_and_a_graded_score() {
        let scores = std::env::temp_dir().join("numinous_mcp_challenge_scores_test.txt");
        let _ = std::fs::remove_file(&scores);
        let posed = handle_request(&json!({
            "jsonrpc":"2.0","id":46,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","seed":7}}
        }))
        .expect("tools/call must respond");
        let sc = &posed["result"]["structuredContent"];
        let box_at = |i: usize| sc["target"][i].as_f64().expect("target coord");
        let (w, h) = (
            sc["width"].as_f64().expect("width") - 1.0,
            sc["height"].as_f64().expect("height") - 1.0,
        );
        let to_norm = |x: f64, y: f64| json!([x / w, y / h]);
        let spread = json!([
            to_norm((box_at(0) + box_at(2)) / 2.0, (box_at(1) + box_at(3)) / 2.0),
            to_norm(box_at(0) + 1.0, box_at(1) + 1.0),
            to_norm(box_at(2) - 1.0, box_at(1) + 1.0),
            to_norm(box_at(0) + 1.0, box_at(3) - 1.0),
            to_norm(box_at(2) - 1.0, box_at(3) - 1.0),
        ]);

        // Pose-only records nothing.
        let mut idle = numinous_core::Journey::from_text("");
        super::record_challenge_attempt(&json!({"id":"voronoi","seed":7}), &mut idle, &scores);
        assert_eq!(idle.sparks(), 0, "posing must not farm XP");

        // A passed attempt records play plus win; a miss records play only.
        let mut winner = numinous_core::Journey::from_text("");
        super::record_challenge_attempt(
            &json!({"id":"voronoi","seed":7,"pokes":spread}),
            &mut winner,
            &scores,
        );
        let mut misser = numinous_core::Journey::from_text("");
        super::record_challenge_attempt(
            &json!({"id":"voronoi","seed":7,"pokes":[[0.0,0.0]]}),
            &mut misser,
            &scores,
        );
        assert!(misser.sparks() > 0, "showing up counts");
        assert!(
            winner.sparks() > misser.sparks(),
            "clearing the threshold counts double: {} vs {}",
            winner.sparks(),
            misser.sparks()
        );

        // The graded score posts under the challenge key.
        let table = super::scores_tool(&scores);
        let text = table["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("challenge voronoi seed:7"), "got: {text}");
        let _ = std::fs::remove_file(&scores);
    }

    #[test]
    fn parameter_challenge_poses_then_grades_by_phase() {
        let posed = handle_request(&json!({
            "jsonrpc":"2.0","id":47,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"slope-rider","kind":"parameter","seed":7}}
        }))
        .expect("tools/call must respond");
        assert_eq!(posed["result"]["isError"], false);
        let sc = &posed["result"]["structuredContent"];
        assert_eq!(sc["kind"], "parameter");
        let target = sc["target"].as_f64().expect("target value");
        let tolerance = sc["tolerance"].as_f64().expect("tolerance");
        assert!(tolerance > 0.0);
        let label = sc["label"].as_str().expect("label");
        let text = posed["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains(label), "goal names the readout: {text}");

        // Sweep the sampled phases; by construction one lands in tolerance.
        let mut landed = None;
        for i in 0..64 {
            let t = f64::from(i) / 64.0;
            let graded = handle_request(&json!({
                "jsonrpc":"2.0","id":48,"method":"tools/call",
                "params":{"name":"challenge","arguments":{"id":"slope-rider","kind":"parameter","seed":7,"t":t}}
            }))
            .expect("tools/call must respond");
            let grade = &graded["result"]["structuredContent"];
            assert!(grade["distance"].as_f64().expect("distance") >= 0.0);
            assert!(grade["score"].as_u64().expect("score") <= 100);
            if grade["within"] == true {
                landed = Some(grade["value"].as_f64().expect("value"));
                break;
            }
        }
        let value = landed.expect("a sampled phase lands within tolerance");
        assert!((value - target).abs() <= tolerance);
    }

    #[test]
    fn parameter_challenge_guides_bad_kinds_readoutless_rooms_and_bad_phases() {
        let bad_kind = handle_request(&json!({
            "jsonrpc":"2.0","id":49,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","kind":"spatial"}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad_kind["result"]["isError"], true);
        let text = bad_kind["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("parameter"), "names the valid kinds: {text}");
        // A non-string kind is a guiding error too, not a silent fall back to
        // touch: the type is wrong, so say so rather than posing the default.
        let wrong_type = handle_request(&json!({
            "jsonrpc":"2.0","id":49,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"voronoi","kind":5}}
        }))
        .expect("tools/call must respond");
        assert_eq!(
            wrong_type["result"]["isError"], true,
            "non-string kind errors"
        );
        // Derive a readout-less room from the registry, like the quiet-room
        // test, so this cannot go vacuous if rooms later gain readouts.
        if let Some(silent) = numinous_core::all_rooms()
            .into_iter()
            .find(|room| numinous_core::pose_parameter_goal(room.as_ref(), 1).is_none())
        {
            let refused = handle_request(&json!({
                "jsonrpc":"2.0","id":50,"method":"tools/call",
                "params":{"name":"challenge","arguments":{"id":silent.meta().id,"kind":"parameter"}}
            }))
            .expect("tools/call must respond");
            assert_eq!(refused["result"]["isError"], true);
            let text = refused["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(text.contains("readout"), "guides the agent: {text}");
        }
        let bad_phase = handle_request(&json!({
            "jsonrpc":"2.0","id":51,"method":"tools/call",
            "params":{"name":"challenge","arguments":{"id":"slope-rider","kind":"parameter","t":1.5}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad_phase["result"]["isError"], true);
    }

    #[test]
    fn a_parameter_attempt_records_play_win_and_a_graded_score() {
        let scores = std::env::temp_dir().join("numinous_mcp_parameter_scores_test.txt");
        let _ = std::fs::remove_file(&scores);
        let room = numinous_core::room_by_id("slope-rider").expect("room");
        let goal = numinous_core::pose_parameter_goal(room.as_ref(), 7).expect("slope-rider poses");
        let (landing_t, missing_t) = {
            let mut landing = None;
            let mut missing = None;
            for i in 0..64 {
                let t = f64::from(i) / 64.0;
                let grade =
                    numinous_core::grade_parameter(room.as_ref(), &goal, t).expect("grades");
                if grade.within && landing.is_none() {
                    landing = Some(t);
                }
                if !grade.within && missing.is_none() {
                    missing = Some(t);
                }
            }
            (landing.expect("reachable"), missing.expect("missable"))
        };

        // Pose-only (no t) records nothing.
        let mut idle = numinous_core::Journey::from_text("");
        super::record_challenge_attempt(
            &json!({"id":"slope-rider","kind":"parameter","seed":7}),
            &mut idle,
            &scores,
        );
        assert_eq!(idle.sparks(), 0, "posing must not farm XP");

        let mut winner = numinous_core::Journey::from_text("");
        super::record_challenge_attempt(
            &json!({"id":"slope-rider","kind":"parameter","seed":7,"t":landing_t}),
            &mut winner,
            &scores,
        );
        let mut misser = numinous_core::Journey::from_text("");
        super::record_challenge_attempt(
            &json!({"id":"slope-rider","kind":"parameter","seed":7,"t":missing_t}),
            &mut misser,
            &scores,
        );
        assert!(misser.sparks() > 0, "showing up counts");
        assert!(
            winner.sparks() > misser.sparks(),
            "landing counts double: {} vs {}",
            winner.sparks(),
            misser.sparks()
        );
        let table = super::scores_tool(&scores);
        let text = table["content"][0]["text"].as_str().unwrap_or_default();
        assert!(
            text.contains("challenge slope-rider parameter seed:7"),
            "got: {text}"
        );
        let _ = std::fs::remove_file(&scores);
    }

    #[test]
    fn predict_poses_then_grades_with_a_band() {
        let posed = handle_request(&json!({
            "jsonrpc":"2.0","id":52,"method":"tools/call",
            "params":{"name":"predict","arguments":{"id":"slope-rider","seed":4}}
        }))
        .expect("tools/call must respond");
        assert_eq!(posed["result"]["isError"], false);
        let sc = &posed["result"]["structuredContent"];
        assert_eq!(sc["game"], "predict");
        assert!(sc["phase"].as_f64().expect("phase") > 0.0);
        let text = posed["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("TILT"),
            "the prompt names the readout: {text}"
        );

        // Compute the truth via the core, guess it exactly, and expect NAILED.
        let room = numinous_core::room_by_id("slope-rider").expect("room");
        let prediction = numinous_core::pose_prediction(room.as_ref(), 4).expect("poses");
        let truth = numinous_core::grade_prediction(room.as_ref(), &prediction, prediction.span.0)
            .expect("grades")
            .actual;
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":53,"method":"tools/call",
            "params":{"name":"predict","arguments":{"id":"slope-rider","seed":4,"guess":truth}}
        }))
        .expect("tools/call must respond");
        let grade = &graded["result"]["structuredContent"];
        assert_eq!(grade["band"], "NAILED");
        assert_eq!(grade["score"], 100);
        assert!((grade["actual"].as_f64().expect("actual") - truth).abs() < 1e-9);
        // Determinism: the same guess earns the same grade.
        let again = handle_request(&json!({
            "jsonrpc":"2.0","id":54,"method":"tools/call",
            "params":{"name":"predict","arguments":{"id":"slope-rider","seed":4,"guess":truth}}
        }))
        .expect("tools/call must respond");
        assert_eq!(grade, &again["result"]["structuredContent"]);
    }

    #[test]
    fn predict_is_a_mirror_not_a_leaderboard() {
        // A graded guess records showing up (play), but never a win and never a
        // posted score, however accurate. Posing records nothing.
        let journey = std::env::temp_dir().join("numinous_mcp_predict_journey_test.txt");
        let scores = std::env::temp_dir().join("numinous_mcp_predict_scores_test.txt");
        let _ = std::fs::remove_file(&journey);
        let _ = std::fs::remove_file(&scores);

        super::record_progress(
            &json!({"method":"tools/call","params":{"name":"predict","arguments":{"id":"slope-rider","seed":4}}}),
            &journey,
        );
        assert_eq!(
            numinous_core::load_journey_file(&journey).sparks(),
            0,
            "posing must not farm XP"
        );

        let room = numinous_core::room_by_id("slope-rider").expect("room");
        let prediction = numinous_core::pose_prediction(room.as_ref(), 4).expect("poses");
        let truth = numinous_core::grade_prediction(room.as_ref(), &prediction, prediction.span.0)
            .expect("grades")
            .actual;
        super::record_progress(
            &json!({"method":"tools/call","params":{"name":"predict","arguments":{"id":"slope-rider","seed":4,"guess":truth}}}),
            &journey,
        );
        let after = numinous_core::load_journey_file(&journey);
        assert!(after.sparks() > 0, "showing up counts");
        // A perfect guess is not a win: sparks equal exactly one play, no win bonus.
        let mut one_play = numinous_core::Journey::from_text("");
        one_play.play();
        assert_eq!(
            after.sparks(),
            one_play.sparks(),
            "a perfect prediction earns play only, never a win"
        );

        let table = super::scores_tool(&scores);
        let text = table["content"][0]["text"].as_str().unwrap_or_default();
        assert!(
            !text.contains("predict"),
            "predict must never post a score: {text}"
        );
        let _ = std::fs::remove_file(&journey);
        let _ = std::fs::remove_file(&scores);
    }

    #[test]
    fn predict_guides_rooms_without_a_readout() {
        if let Some(silent) = numinous_core::all_rooms()
            .into_iter()
            .find(|room| numinous_core::pose_prediction(room.as_ref(), 1).is_none())
        {
            let refused = handle_request(&json!({
                "jsonrpc":"2.0","id":55,"method":"tools/call",
                "params":{"name":"predict","arguments":{"id":silent.meta().id}}
            }))
            .expect("tools/call must respond");
            assert_eq!(refused["result"]["isError"], true);
            let text = refused["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(text.contains("readout"), "guides the agent: {text}");
        }
    }

    #[test]
    fn cairn_reads_a_stone_by_factoring_and_leaves_only_at_the_cap() {
        let cairn = std::env::temp_dir().join("numinous_mcp_cairn_test.txt");
        let journey = std::env::temp_dir().join("numinous_mcp_cairn_journey_test.txt");
        let _ = std::fs::remove_file(&cairn);
        let _ = std::fs::remove_file(&journey);

        // Pose: reading returns a semiprime to factor, no width yet.
        let posed = super::cairn_tool(&json!({ "seed": 3 }), &journey, &cairn);
        let n = posed["structuredContent"]["semiprime"]
            .as_u64()
            .expect("a semiprime to factor");
        assert!(n > 1);
        // The true width reads it; a wrong factor shears it; a non-factor is refused.
        let stone = numinous_core::draw_stone(&cairn, 3);
        let right = super::cairn_tool(
            &json!({ "seed": 3, "width": stone.width }),
            &journey,
            &cairn,
        );
        assert_eq!(right["structuredContent"]["readable"], true);
        assert!(
            right["structuredContent"]["message"].as_str().is_some(),
            "the message resolves and is revealed"
        );
        let sheared = super::cairn_tool(
            &json!({ "seed": 3, "width": stone.height }),
            &journey,
            &cairn,
        );
        assert_eq!(sheared["structuredContent"]["readable"], false);
        let refused = super::cairn_tool(
            &json!({ "seed": 3, "width": stone.width + 1 }),
            &journey,
            &cairn,
        );
        assert_eq!(refused["isError"], true, "a non-factor is refused");

        // Leaving is gated at the cap: a fresh journey is turned away with guidance.
        let early = super::cairn_tool(&json!({ "leave": "I was here" }), &journey, &cairn);
        assert_eq!(early["isError"], true);
        assert!(
            early["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("level 42"),
            "it names the cap"
        );
        // At the cap, the bequest is deposited and drawable afterward.
        std::fs::write(&journey, "wins 900\nsecrets 0\nplays 900\n").unwrap();
        assert!(numinous_core::load_journey_file(&journey).level() >= super::CAIRN_LEVEL);
        let left = super::cairn_tool(
            &json!({ "leave": "primes never run out", "author": "a tester" }),
            &journey,
            &cairn,
        );
        assert_eq!(left["structuredContent"]["left"], true);
        assert!(left["structuredContent"]["semiprime"].as_u64().is_some());
        // The deposited bequest is now in the pile and drawable by some seed.
        let drawable =
            (0..60).any(|s| numinous_core::draw_stone(&cairn, s).text == "primes never run out");
        assert!(drawable, "the deposited bequest joined the cairn");

        let _ = std::fs::remove_file(&cairn);
        let _ = std::fs::remove_file(&journey);
    }

    #[test]
    fn oversized_request_lines_are_drained_not_buffered() {
        let mut input = Vec::new();
        input.extend(std::iter::repeat_n(b'x', super::MAX_REQUEST_BYTES + 100));
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        input.push(b'\n');
        let mut reader = std::io::BufReader::new(&input[..]);
        let mut line = Vec::new();
        assert!(super::read_bounded_line(&mut reader, &mut line).expect("read"));
        assert!(
            line.len() < 8,
            "an oversized line is replaced by a tiny invalid marker, not held"
        );
        assert!(serde_json::from_slice::<serde_json::Value>(&line).is_err());
        assert!(super::read_bounded_line(&mut reader, &mut line).expect("read"));
        assert!(
            serde_json::from_slice::<serde_json::Value>(&line).is_ok(),
            "the request after the flood still parses"
        );
        assert!(!super::read_bounded_line(&mut reader, &mut line).expect("read"));
    }

    #[test]
    fn play_room_frames_are_capped_at_the_tool_layer() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":60,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi","width":4096,"height":4096,"pokes":[[0.5,0.5]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], false);
        assert_eq!(
            resp["result"]["structuredContent"]["delta"]["total_cells"],
            super::MAX_TOOL_WIDTH * super::MAX_TOOL_HEIGHT,
            "hostile dimensions clamp to the tool cap"
        );
    }

    #[test]
    fn a_zero_change_delta_serializes_with_a_null_region() {
        // A poke can legitimately change nothing (e.g. touching existing ink);
        // the serialized delta must then carry an explicit null region.
        let json = render_delta_json(numinous_core::RenderDelta {
            total_cells: 12,
            ..Default::default()
        });
        assert_eq!(json["cells_changed"], 0);
        assert_eq!(json["total_cells"], 12);
        assert_eq!(json["changed_region"], serde_json::Value::Null);
    }

    #[test]
    fn play_room_delta_matches_across_variation_reseeds() {
        // The delta must compare poked-vs-unpoked at the SAME variation, so a
        // reseeded visit still reports only what the hand changed.
        let poked = handle_request(&json!({
            "jsonrpc":"2.0","id":37,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi","width":40,"height":20,"variation":7,"pokes":[[0.5,0.5]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(poked["result"]["isError"], false);
        let delta = &poked["result"]["structuredContent"]["delta"];
        assert!(
            delta["cells_changed"].as_u64().expect("cells_changed") > 0,
            "a dropped well renegotiates borders under any variation"
        );
    }

    #[test]
    fn a_gesture_lets_an_agent_pin_pull_and_fling() {
        // Held: a down with no lift pins the pendulum; time does not move it.
        let pinned = |t: f64| {
            handle_request(&json!({
                "jsonrpc":"2.0","id":70,"method":"tools/call",
                "params":{"name":"play_room","arguments":{"id":"double-pendulum","width":50,"height":30,"t":t,
                    "gesture":[{"kind":"down","x":0.3,"y":0.4,"t":0.1}]}}
            }))
            .expect("tools/call must respond")["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .split_once("\n\n")
                .map(|(_, frame)| frame.to_string())
                .unwrap_or_default()
        };
        assert_eq!(pinned(0.2), pinned(0.9), "a pinned bob ignores the clock");

        // Released: the same lift point with a faster approach throws harder.
        let released = |before_x: f64, before_t: f64| {
            handle_request(&json!({
                "jsonrpc":"2.0","id":71,"method":"tools/call",
                "params":{"name":"play_room","arguments":{"id":"double-pendulum","width":50,"height":30,"t":0.35,
                    "gesture":[
                        {"kind":"move","x":before_x,"y":0.5,"t":before_t},
                        {"kind":"up","x":0.6,"y":0.5,"t":0.15}
                    ]}}
            }))
            .expect("tools/call must respond")["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        };
        assert_ne!(
            released(0.58, 0.05),
            released(0.30, 0.147),
            "a flick and a gentle lift land differently: momentum crosses the wire"
        );
    }

    #[test]
    fn a_gesture_bridges_to_pokes_for_rooms_without_held_semantics() {
        // For a legacy room, a gesture's downs and moves paint exactly like
        // the equivalent poke list: the App's bridge, over the protocol.
        let via_gesture = handle_request(&json!({
            "jsonrpc":"2.0","id":72,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi","width":40,"height":20,"t":0.25,
                "gesture":[
                    {"kind":"down","x":0.3,"y":0.7,"t":0.25},
                    {"kind":"move","x":0.5,"y":0.5,"t":0.26},
                    {"kind":"up","x":0.5,"y":0.5,"t":0.27}
                ]}}
        }))
        .expect("tools/call must respond");
        let via_pokes = handle_request(&json!({
            "jsonrpc":"2.0","id":73,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi","width":40,"height":20,"t":0.25,
                "pokes":[[0.3,0.7],[0.5,0.5]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(
            via_gesture["result"]["structuredContent"]["delta"],
            via_pokes["result"]["structuredContent"]["delta"],
            "the bridge answers identically over MCP"
        );
        assert_eq!(via_gesture["result"]["isError"], false);
    }

    #[test]
    fn gestures_are_validated_and_exclusive_with_pokes() {
        let bad_kind = handle_request(&json!({
            "jsonrpc":"2.0","id":74,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi",
                "gesture":[{"kind":"wiggle","x":0.5,"y":0.5,"t":0.1}]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad_kind["result"]["isError"], true);
        let bad_coord = handle_request(&json!({
            "jsonrpc":"2.0","id":75,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi",
                "gesture":[{"kind":"down","x":1.5,"y":0.5,"t":0.1}]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad_coord["result"]["isError"], true);
        let too_many: Vec<_> = (0..=numinous_core::MAX_ROOM_INPUTS)
            .map(|_| json!({"kind":"cancel"}))
            .collect();
        let flooded = handle_request(&json!({
            "jsonrpc":"2.0","id":76,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi","gesture":too_many}}
        }))
        .expect("tools/call must respond");
        assert_eq!(flooded["result"]["isError"], true);
        let stowaway = handle_request(&json!({
            "jsonrpc":"2.0","id":78,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi",
                "gesture":[{"kind":"cancel","note":"smuggled"}]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(
            stowaway["result"]["isError"], true,
            "unknown event fields are rejected, matching the schema"
        );
        let both = handle_request(&json!({
            "jsonrpc":"2.0","id":77,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi",
                "pokes":[[0.5,0.5]],
                "gesture":[{"kind":"down","x":0.5,"y":0.5,"t":0.1}]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(both["result"]["isError"], true);
        let text = both["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("not both"), "the error guides: {text}");
    }

    #[test]
    fn play_room_rejects_invalid_hand_points() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":34,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"double-pendulum","pokes":[[1.2,0.5]]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("[0,1]"), "got: {text}");
    }

    #[test]
    fn play_room_rejects_too_many_hand_points() {
        let pokes: Vec<_> = (0..=numinous_core::MAX_ROOM_POKES)
            .map(|_| json!([0.5, 0.5]))
            .collect();
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":35,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"double-pendulum","pokes":pokes}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("at most"), "got: {text}");
    }

    #[test]
    fn unknown_room_is_a_guiding_tool_error() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"no-such-room"}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("Known rooms"),
            "the error should guide the agent"
        );
    }

    #[test]
    fn unknown_method_is_jsonrpc_error() {
        let resp = handle_request(&json!({"jsonrpc":"2.0","id":5,"method":"does-not-exist"}))
            .expect("a request must respond");
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn notifications_get_no_response() {
        assert!(
            handle_request(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
                .is_none()
        );
    }

    #[test]
    fn ping_returns_an_empty_result() {
        let resp = handle_request(&json!({"jsonrpc":"2.0","id":9,"method":"ping"}))
            .expect("ping must respond");
        assert!(resp["result"].is_object());
        assert!(resp.get("error").is_none());
    }

    #[test]
    fn list_rooms_tool_returns_the_catalog() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":10,"method":"tools/call",
            "params":{"name":"list_rooms"}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(text.contains("times-tables"));
        assert_eq!(resp["result"]["isError"], false);
    }

    #[test]
    fn describe_room_tool_returns_details() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":11,"method":"tools/call",
            "params":{"name":"describe_room","arguments":{"id":"times-tables"}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Number & Pattern"));
        assert!(text.contains("Action:"));
    }

    #[test]
    fn describe_room_without_id_is_a_guiding_error() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":12,"method":"tools/call",
            "params":{"name":"describe_room","arguments":{}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn unknown_tool_is_a_jsonrpc_error() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":13,"method":"tools/call",
            "params":{"name":"no-such-tool"}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[test]
    fn tools_call_without_params_is_an_error() {
        let resp = handle_request(&json!({"jsonrpc":"2.0","id":14,"method":"tools/call"}))
            .expect("tools/call must respond");
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[test]
    fn parse_and_write_helpers_round_trip() {
        // write_message emits one newline-terminated JSON line.
        let mut buf: Vec<u8> = Vec::new();
        super::write_message(&mut buf, &json!({"ok": true})).expect("write");
        let line = String::from_utf8(buf).expect("utf8");
        assert!(line.ends_with('\n'));
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).expect("valid json");
        assert_eq!(parsed["ok"], true);
    }
}
