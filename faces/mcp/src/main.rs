// The tools/list schema is one large nested json! literal; its depth exceeds
// the default macro recursion limit.
#![recursion_limit = "256"]

//! The Numinous MCP server: the face that lets AI agents (and digital minds)
//! learn and play. See `docs/INTERFACES.md` and `docs/DIGITAL_MINDS.md`.
//!
//! Transport: JSON-RPC 2.0 over newline-delimited stdio. The server is dual-era:
//! legacy clients use the initialization handshake from 2025-11-25 and earlier,
//! while 2026-07-28 clients declare version, identity, and capabilities on each
//! request. Both paths reach the same deterministic tool catalog and core.

mod audible;
mod broadcast;
mod catalog;
mod encounter;
mod journal;
mod local_state;
mod room_door;
mod temporal;
mod workspace;

use std::io::{self, BufRead, Write};
use std::sync::{Mutex, MutexGuard};

use broadcast::{SessionBroadcast, SessionSnapshot};
use catalog::{discover_result, initialize_result, server_info, tools_catalog, tools_list_result};
use encounter::{
    action_json as encounter_action_json, delta_counts as encounter_delta_counts,
    dwell_counts as encounter_dwell_counts, issue as issue_encounter, issue_receipt,
    listen_action as encounter_listen_action, listen_action_json,
    listen_result as encounter_listen_result, play_action as encounter_play_action,
    play_result as encounter_play_result, receipt_json, request as encounter_request,
    sing_action as encounter_sing_action, sing_action_json, sing_result as encounter_sing_result,
};
use local_state::forget_tool;
use numinous_broadcast::{
    PLAY_ROOM_DEFAULT_HEIGHT as DEFAULT_HEIGHT, PLAY_ROOM_DEFAULT_WIDTH as DEFAULT_WIDTH,
    PLAY_ROOM_MAX_HEIGHT as MAX_TOOL_HEIGHT, PLAY_ROOM_MAX_WIDTH as MAX_TOOL_WIDTH, PublicTool,
};
use numinous_core::{Canvas, room_by_id};
use serde_json::{Map, Value, json};
use temporal::{dwell_evidence_json, evidence_json as temporal_evidence_json, render_delta_json};
use workspace::{ProcessWorkspace, compact_workspace_summary, workspace_tool};

/// Stateless MCP revision implemented by the per-request metadata path.
const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";

/// Every MCP revision this dual-era server implements, newest first.
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[MODERN_PROTOCOL_VERSION, "2025-11-25", "2025-06-18"];

/// The tool catalog and discovery document are immutable for one binary.
const DISCOVERY_CACHE_TTL_MS: u64 = 86_400_000;
const TOOLS_CACHE_TTL_MS: u64 = 86_400_000;

const PROTOCOL_VERSION_META_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CLIENT_INFO_META_KEY: &str = "io.modelcontextprotocol/clientInfo";
const CLIENT_CAPABILITIES_META_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
const SERVER_INFO_META_KEY: &str = "io.modelcontextprotocol/serverInfo";
const JSON_SCHEMA_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// Longest catalog id a tool argument may carry (room, sim, or similar).
/// Catalog keys today are far shorter; the bound rejects hostile multi-kilobyte
/// id strings before domain dispatch.
const MAX_TOOL_ID_CHARS: usize = 64;

/// Longest author credit accepted with a Cairn bequest. Matches the sanitize
/// bound in `numinous_core::Bequest::new`.
const MAX_AUTHOR_CHARS: usize = 48;

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
    let broadcast = ConnectionBroadcast::new();
    let workspace = ProcessWorkspace::new();

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
                if let Some(response) =
                    handle_request_with_visit(&request, &journey_path(), &broadcast, &workspace)
                {
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
        let newline_was_consumed = line.last() == Some(&b'\n');
        // Drain the rest of the oversized line without holding it.
        line.clear();
        line.push(b'{'); // guaranteed-invalid JSON, so the caller answers with a parse error
        if !newline_was_consumed {
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
    }
    Ok(true)
}

/// Where the journey file lives (shared with the CLI face, so a mind's play
/// counts the same wherever it plays): `NUMINOUS_JOURNEY` if set, else home.
#[cfg(test)]
struct TestStateRoot {
    path: std::path::PathBuf,
}

#[cfg(test)]
impl TestStateRoot {
    fn new() -> Self {
        use std::hash::{Hash, Hasher};

        let thread = std::thread::current();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        thread.id().hash(&mut hasher);
        thread.name().hash(&mut hasher);
        let path = std::env::temp_dir().join(format!(
            "numinous-mcp-test-{}-{:016x}",
            std::process::id(),
            hasher.finish()
        ));
        Self::at(path)
    }

    fn at(path: std::path::PathBuf) -> Self {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear test state directory: {error}"),
        }
        std::fs::create_dir_all(&path).expect("test state directory should be writable");
        Self { path }
    }
}

#[cfg(test)]
impl Drop for TestStateRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
std::thread_local! {
    static TEST_STATE_ROOT: TestStateRoot = TestStateRoot::new();
}

#[cfg(test)]
fn test_state_path(kind: &str) -> std::path::PathBuf {
    TEST_STATE_ROOT.with(|root| root.path.join(format!("{kind}.txt")))
}

fn local_state_paths() -> numinous_core::LocalStatePaths {
    #[cfg(test)]
    {
        numinous_core::LocalStatePaths {
            journey: test_state_path("journey"),
            scores: test_state_path("scores"),
            cairn: test_state_path("cairn"),
            journal: test_state_path("journal"),
            preferences: test_state_path("preferences"),
            radio_cache: test_state_path("radio"),
            protected_radio_source: None,
            crash_log: test_state_path("crash"),
        }
    }
    #[cfg(not(test))]
    {
        numinous_core::resolve_local_state_paths()
    }
}

fn local_state_paths_at(journey_file: &std::path::Path) -> numinous_core::LocalStatePaths {
    let mut paths = local_state_paths();
    paths.journey = journey_file.to_path_buf();
    paths
}

fn journey_path() -> std::path::PathBuf {
    local_state_paths().journey
}

/// Load the journey at `path`, or start a fresh one.
fn load_journey(path: &std::path::Path) -> numinous_core::Journey {
    numinous_core::load_journey_file(path)
}

/// Where the high-score table lives (shared with the CLI face, same keys, so
/// humans and agents compete on the same boards).
fn scores_path() -> std::path::PathBuf {
    local_state_paths().scores
}

std::thread_local! {
    /// Whether a local save failed while handling the current request. The
    /// stdio server answers one request at a time, so a request-scoped flag
    /// is exactly a thread-local the dispatcher drains per response.
    static SAVE_TROUBLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Append the save-trouble note to a response when this request lost a
/// write. The mind playing this face never sees the server's stderr, so the
/// response text is the only channel that reaches the one party who lost
/// something. Additive, and only on the requests where a write failed.
fn note_save_trouble(mut result: Value) -> Value {
    if !SAVE_TROUBLE.with(|flag| flag.replace(false)) {
        return result;
    }
    if let Some(text) = result
        .get_mut("content")
        .and_then(|content| content.get_mut(0))
        .and_then(|entry| entry.get_mut("text"))
        && let Some(existing) = text.as_str()
    {
        *text = Value::String(format!(
            "{existing}\nNOTE: a local save failed; this result counted in memory but the \
             file refused the write. Progress rides in memory until a later save lands."
        ));
    }
    result
}

/// Record a score at `path`, keeping the best. Returns true on a new record.
fn post_score(path: &std::path::Path, key: &str, score: i64) -> bool {
    // A write failure must not wear the same face as "not a new best".
    match numinous_core::record_score_file(path, key, score) {
        Ok(best) => best,
        Err(error) => {
            eprintln!("numinous-mcp: score could not be saved: {error}");
            SAVE_TROUBLE.with(|flag| flag.set(true));
            false
        }
    }
}

/// Persist a progress delta, and say so on stderr when the ledger refuses.
///
/// The stdio protocol owns stdout, so stderr is the one channel where a
/// failing save can speak without corrupting a response; hosts surface it in
/// the server log. The delta keeps riding in memory, so a later successful
/// save still lands the full difference. Returns whether the write landed,
/// so a tool that claims a durable state change can refuse the claim when
/// the state did not actually change.
fn persist_progress(
    path: &std::path::Path,
    before: &numinous_core::Journey,
    journey: &numinous_core::Journey,
) -> bool {
    match numinous_core::persist_journey_delta(path, before, journey) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("numinous-mcp: progress could not be saved: {error}");
            SAVE_TROUBLE.with(|flag| flag.set(true));
            false
        }
    }
}

/// Where the cairn lives (shared with the CLI face): the local pile of
/// bequests a mind leaves for whoever comes after.
fn cairn_path() -> std::path::PathBuf {
    local_state_paths().cairn
}

fn journal_path() -> std::path::PathBuf {
    local_state_paths().journal
}

/// The level at which the cairn opens for leaving: the journey's cap, so a
/// bequest is a finished mind's last free act, not a first one.
const CAIRN_LEVEL: u32 = 42;

/// Record what this request means for the journey: agents level up too, by the
/// same rules as everyone else. Showing up counts; being right counts double.
/// The seed a tool should use: the daily day count when asked, else the arg.
/// The key under which the resolved day is pinned into a daily request's args
/// (see [`freeze_daily_day`]). Camel-case to match the other structured fields.
const DAILY_DAY_KEY: &str = "dailyDay";

/// Today's day count (whole days since the Unix epoch), the seed a daily game
/// shares with every mind. Read exactly once per request via [`freeze_daily_day`]
/// so the reply grading, the posted score, and the streak never straddle a
/// midnight tick.
fn daily_day_count() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(1)
}

/// The day a request should use: the frozen value pinned at the request boundary
/// when present (the normal path), else a fresh read (a direct call in a test).
fn request_day(args: &Value) -> u64 {
    args.get(DAILY_DAY_KEY)
        .and_then(Value::as_u64)
        .unwrap_or_else(daily_day_count)
}

fn effective_seed(args: &Value) -> u64 {
    if args.get("daily").and_then(Value::as_bool) == Some(true) {
        request_day(args)
    } else {
        args.get("seed").and_then(Value::as_u64).unwrap_or(1)
    }
}

/// Pin the day count into a daily `tools/call` so every clock-derived use in the
/// request (the reply seed, the posted score, the streak) shares one value. The
/// daily seed is otherwise read from the clock more than once per request, and a
/// UTC midnight between two reads would grade or record against a board the
/// player never saw. Non-daily and non-`tools/call` requests borrow unchanged.
fn freeze_daily_day(request: &Value) -> std::borrow::Cow<'_, Value> {
    let is_daily_call = request.get("method").and_then(Value::as_str) == Some("tools/call")
        && request
            .get("params")
            .and_then(|params| params.get("arguments"))
            .and_then(|args| args.get("daily"))
            .and_then(Value::as_bool)
            == Some(true);
    if !is_daily_call {
        return std::borrow::Cow::Borrowed(request);
    }
    let mut owned = request.clone();
    if let Some(args) = owned
        .get_mut("params")
        .and_then(|params| params.get_mut("arguments"))
        .and_then(Value::as_object_mut)
    {
        args.insert(DAILY_DAY_KEY.to_string(), json!(daily_day_count()));
    }
    std::borrow::Cow::Owned(owned)
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
                    || (numinous_core::behind_the_veil(&journey)
                        && numinous_core::deep_akousma(id).is_some()))
            {
                journey.secret();
            }
        }
        "play_room" | "listen_room" => {
            if let Some(id) = args.get("id").and_then(Value::as_str)
                && numinous_core::room_meta_by_id(id).is_some()
            {
                journey.visit(id);
                if name == "play_room"
                    && args.get("aha_summon").and_then(Value::as_bool) == Some(true)
                {
                    journey.consolidate(id);
                }
            }
        }
        "run_sim" | "sing_expression" | "save_creation" | "open_creation" | "fork_creation" => {
            journey.play()
        }
        "plot_expression" => {
            // Listing the recipe bank is discovery, not a creation play.
            if args.get("list_recipes").and_then(Value::as_bool) != Some(true) {
                journey.play();
            }
        }
        "nim" => {
            if let Some(list) = args.get("moves").and_then(Value::as_array)
                && !list.is_empty()
            {
                journey.play();
                let seed = effective_seed(&args);
                if let Some(turns) = nim_turns(&args)
                    && let Ok(replay) = numinous_core::nim::replay(seed, &turns)
                    && replay.winner == Some(numinous_core::nim::NimWinner::Player)
                {
                    journey.win();
                    post_score(&scores_path(), &format!("nim seed:{seed}"), 1);
                }
            }
        }
        "munch" => {
            if let Some(raw) = args.get("bites").and_then(Value::as_array) {
                journey.play();
                let seed = effective_seed(&args);
                let round = args
                    .get("round")
                    .and_then(Value::as_u64)
                    .unwrap_or(numinous_core::FULL_DECK_ROUND);
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
                    &numinous_core::munch_score_key(seed, round),
                    outcome.score,
                );
                if numinous_core::munch_clean_win(&outcome) {
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
                let digits = args.get("digits").map_or(Some(4), |value| {
                    value.as_u64().and_then(|value| usize::try_from(value).ok())
                });
                if let Some(digits) =
                    digits.filter(|&digits| numinous_core::supports_code_length(digits))
                {
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
                let seed = effective_seed(&args);
                let rounds = args
                    .get("rounds")
                    .and_then(Value::as_u64)
                    .unwrap_or(5)
                    .clamp(1, 20);
                // A play per graded round and a score key naming only the
                // rounds actually graded, exactly as the terminal face
                // counts: the same session must level the same on both
                // faces, and wins can never outnumber plays.
                let graded = rounds.min(calls.len() as u64);
                let mut correct = 0i64;
                for n in 0..graded {
                    journey.play();
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
                    &format!("fifteen seed:{seed} rounds:{graded}"),
                    correct,
                );
            }
        }
        "gauntlet" => {
            if let Some(answers) = args.get("answers") {
                let seed = effective_seed(&args);
                let puzzle = numinous_core::GauntletPuzzle::new(seed);
                let grade = puzzle.grade(&gauntlet_answers_from_json(answers));
                for clear in grade.cleared() {
                    journey.play();
                    if clear {
                        journey.win();
                    }
                }
                post_score(
                    &scores_path(),
                    &numinous_core::gauntlet_score_key(seed),
                    grade.total(),
                );
            }
        }
        _ => {}
    }
    if args.get("daily").and_then(Value::as_bool) == Some(true) {
        let _ = journey.record_daily(request_day(&args));
    }
    if journey != before {
        // XP and visit deltas keep riding in memory, so a refused write here
        // self-heals on a later success; the durable-claim tools check the
        // returned flag themselves.
        let _ = persist_progress(path, &before, &journey);
    }
}

/// Write a single JSON-RPC message as one newline-terminated line.
fn write_message(out: &mut impl Write, message: &Value) -> io::Result<()> {
    writeln!(out, "{message}")?;
    out.flush()
}

/// Handle one JSON-RPC request. Returns `None` for notifications (no `id`),
/// which receive no response.
#[cfg(test)]
fn handle_request(request: &Value) -> Option<Value> {
    handle_request_with(request, &journey_path())
}

/// [`handle_request`] with an explicit journey file, so tests stay hermetic.
#[cfg(test)]
fn handle_request_with(request: &Value, journey_file: &std::path::Path) -> Option<Value> {
    handle_request_with_session(request, journey_file, &ConnectionBroadcast::new())
}

#[cfg(test)]
fn handle_request_with_session(
    request: &Value,
    journey_file: &std::path::Path,
    broadcast: &ConnectionBroadcast,
) -> Option<Value> {
    handle_request_with_visit(request, journey_file, broadcast, &ProcessWorkspace::new())
}

fn handle_request_with_visit(
    request: &Value,
    journey_file: &std::path::Path,
    broadcast: &ConnectionBroadcast,
    workspace: &ProcessWorkspace,
) -> Option<Value> {
    let id = request.get("id").cloned();
    if let Err(error) = validate_jsonrpc_envelope(request) {
        if id.is_none() && request.get("method").and_then(Value::as_str).is_some() {
            return None;
        }
        let response_id = id.filter(valid_request_id).unwrap_or(Value::Null);
        return Some(protocol_error_response(response_id, &error));
    }
    let era = match request_era(request) {
        Ok(era) => era,
        Err(error) => {
            let id = id?;
            return Some(protocol_error_response(id, &error));
        }
    };
    // Validate the public request before daily calls gain their private frozen
    // day field. This keeps the declared schemas authoritative without
    // mistaking server-owned request context for a client argument.
    let initial_argument_error =
        if request.get("method").and_then(Value::as_str) == Some("tools/call") {
            validate_declared_tool_arguments(request.get("params")).err()
        } else {
            None
        };

    let (prepared, forced_result) = if initial_argument_error.is_none() {
        match prepare_prediction_mrtr(request, era) {
            Ok(prepared) => prepared,
            Err(error) => {
                let id = id?;
                return Some(protocol_error_response(id, &error));
            }
        }
    } else {
        (request.clone(), None)
    };
    let result_was_forced = forced_result.is_some();

    let argument_error = initial_argument_error.or_else(|| {
        if prepared.get("method").and_then(Value::as_str) == Some("tools/call") {
            validate_declared_tool_arguments(prepared.get("params")).err()
        } else {
            None
        }
    });

    // Freeze the daily day once, at the request boundary, so the reply grading
    // (via call_tool) and the progress recording below share one day count and
    // cannot straddle a midnight tick. Non-daily requests are not cloned.
    let frozen = freeze_daily_day(&prepared);
    let request: &Value = &frozen;

    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let public_call =
        if forced_result.is_none() && method == "tools/call" && argument_error.is_none() {
            capture_public_call(request, broadcast)
        } else {
            None
        };

    let result = if let Some(value) = forced_result {
        Ok(value)
    } else {
        match method {
            "server/discover" if era == RequestEra::Modern => Ok(discover_result()),
            "initialize" if era == RequestEra::Legacy => {
                Ok(initialize_result(request.get("params")))
            }
            "tools/list" => validate_tools_cursor(request.get("params"))
                .map(|()| tools_list_result())
                .map_err(|message| (-32602_i64, message)),
            "tools/call" => match argument_error {
                Some(message) => Ok(tool_error(&message)),
                None => call_tool(request.get("params"), journey_file, broadcast, workspace),
            },
            "notifications/cancelled" if era == RequestEra::Modern => Ok(json!({})),
            "ping" if era == RequestEra::Legacy => Ok(json!({})),
            other => Err((-32601_i64, format!("Method not found: {other}"))),
        }
    };

    if let (Some(public_call), Ok(value)) = (public_call, &result)
        && value.get("resultType").and_then(Value::as_str) != Some("input_required")
    {
        public_call.commit(value);
    }

    if method == "tools/call"
        && !result_was_forced
        && let Ok(value) = &result
        && value.get("isError").and_then(Value::as_bool) != Some(true)
        && value.get("resultType").and_then(Value::as_str) != Some("input_required")
        && request
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            != Some("broadcast_session")
    {
        record_progress(request, journey_file);
    }

    // Notifications carry no id and get no response.
    let id = id?;
    Some(match result {
        // The save-trouble note drains here, after record_progress has done
        // the writes that can set it, so the note lands on the reply of
        // exactly the request that lost something, never the next one.
        Ok(value) => success_response(id, note_save_trouble(result_for_era(value, method, era))),
        Err((code, message)) => error_response(id, code, &message),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestEra {
    Legacy,
    Modern,
}

struct ConnectionBroadcast {
    session: Mutex<SessionBroadcast>,
}

impl ConnectionBroadcast {
    fn new() -> Self {
        Self {
            session: Mutex::new(SessionBroadcast::new()),
        }
    }

    fn start(&self, pairing_code: &str) -> Result<SessionSnapshot, broadcast::SessionError> {
        self.lock().start(pairing_code)
    }

    fn status(&self) -> SessionSnapshot {
        self.lock().status()
    }

    fn pause(&self) -> Result<SessionSnapshot, broadcast::SessionError> {
        self.lock().pause()
    }

    fn resume(&self) -> Result<SessionSnapshot, broadcast::SessionError> {
        self.lock().resume()
    }

    fn stop(&self) -> Result<SessionSnapshot, broadcast::SessionError> {
        self.lock().stop()
    }

    fn capture(&self, tool: PublicTool, arguments: &Value) -> Option<broadcast::PublicCall> {
        self.lock().capture(tool, arguments)
    }

    fn lock(&self) -> MutexGuard<'_, SessionBroadcast> {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

struct ProtocolError {
    code: i64,
    message: &'static str,
    data: Option<Value>,
}

fn valid_request_id(id: &Value) -> bool {
    id.is_string() || id.as_i64().is_some() || id.as_u64().is_some()
}

fn validate_jsonrpc_envelope(request: &Value) -> Result<(), ProtocolError> {
    let Some(request) = request.as_object() else {
        return Err(ProtocolError {
            code: -32600,
            message: "Invalid Request",
            data: None,
        });
    };
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || !request.get("method").is_some_and(Value::is_string)
        || request.get("id").is_some_and(|id| !valid_request_id(id))
        || request
            .get("params")
            .is_some_and(|params| !params.is_object())
    {
        return Err(ProtocolError {
            code: -32600,
            message: "Invalid Request",
            data: None,
        });
    }
    Ok(())
}

fn request_era(request: &Value) -> Result<RequestEra, ProtocolError> {
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let meta = request.get("params").and_then(|params| params.get("_meta"));
    let has_modern_marker = method == "server/discover"
        || meta.is_some_and(|meta| {
            meta.get(PROTOCOL_VERSION_META_KEY).is_some()
                || meta.get(CLIENT_CAPABILITIES_META_KEY).is_some()
        });
    if !has_modern_marker {
        return Ok(RequestEra::Legacy);
    }

    let Some(meta) = meta.and_then(Value::as_object) else {
        return Err(invalid_params_error(
            "Modern requests require an object at params._meta",
        ));
    };
    let Some(version) = meta.get(PROTOCOL_VERSION_META_KEY).and_then(Value::as_str) else {
        return Err(invalid_params_error(
            "Modern requests require a string protocol version in params._meta",
        ));
    };
    if version != MODERN_PROTOCOL_VERSION {
        return Err(ProtocolError {
            code: -32022,
            message: "Unsupported protocol version",
            data: Some(json!({
                "supported": SUPPORTED_PROTOCOL_VERSIONS,
                "requested": version,
            })),
        });
    }
    if !meta
        .get(CLIENT_CAPABILITIES_META_KEY)
        .is_some_and(Value::is_object)
    {
        return Err(invalid_params_error(
            "Modern requests require client capabilities in params._meta",
        ));
    }
    if meta.get(CLIENT_INFO_META_KEY).is_some_and(|client_info| {
        !client_info.is_object()
            || !client_info.get("name").is_some_and(Value::is_string)
            || !client_info.get("version").is_some_and(Value::is_string)
    }) {
        return Err(invalid_params_error(
            "Modern client info must contain string name and version fields when present",
        ));
    }
    Ok(RequestEra::Modern)
}

fn invalid_params_error(message: &'static str) -> ProtocolError {
    ProtocolError {
        code: -32602,
        message,
        data: None,
    }
}

fn protocol_error_response(id: Value, error: &ProtocolError) -> Value {
    let mut response = error_response(id, error.code, error.message);
    if let Some(data) = &error.data {
        response["error"]["data"] = data.clone();
    }
    response
}

fn request_supports_form_elicitation(request: &Value) -> bool {
    let Some(elicitation) = request
        .get("params")
        .and_then(|params| params.get("_meta"))
        .and_then(|meta| meta.get(CLIENT_CAPABILITIES_META_KEY))
        .and_then(|capabilities| capabilities.get("elicitation"))
        .and_then(Value::as_object)
    else {
        return false;
    };
    elicitation.is_empty() || elicitation.get("form").is_some_and(Value::is_object)
}

fn prepare_prediction_mrtr(
    request: &Value,
    era: RequestEra,
) -> Result<(Value, Option<Value>), ProtocolError> {
    if era != RequestEra::Modern
        || request.get("method").and_then(Value::as_str) != Some("tools/call")
        || request
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            != Some("predict")
    {
        return Ok((request.clone(), None));
    }

    let params = request
        .get("params")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_params_error("tools/call requires object params"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    if let Some(input_responses) = params.get("inputResponses") {
        if params.get("requestState").is_some() {
            return Err(invalid_params_error(
                "predict does not issue requestState and cannot accept it",
            ));
        }
        let response = input_responses
            .as_object()
            .and_then(|responses| {
                (responses.len() == 1)
                    .then(|| responses.get("prediction"))
                    .flatten()
            })
            .and_then(Value::as_object)
            .ok_or_else(|| {
                invalid_params_error("predict requires one input response named prediction")
            })?;
        let action = response
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_params_error("prediction response requires an action"))?;
        if matches!(action, "decline" | "cancel") {
            let text = if action == "decline" {
                "Prediction declined. Nothing was graded or recorded."
            } else {
                "Prediction cancelled. Nothing was graded or recorded."
            };
            return Ok((request.clone(), Some(tool_text(text))));
        }
        if action != "accept" {
            return Err(invalid_params_error(
                "prediction response action must be accept, decline, or cancel",
            ));
        }
        let content = response
            .get("content")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid_params_error("accepted prediction requires form content"))?;
        if content
            .keys()
            .any(|key| !matches!(key.as_str(), "guess" | "rate"))
        {
            return Err(invalid_params_error(
                "prediction form content accepts only guess and rate",
            ));
        }
        let guess = content
            .get("guess")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| invalid_params_error("accepted prediction requires a finite guess"))?;
        let rate = content
            .get("rate")
            .map(|value| {
                value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| invalid_params_error("prediction rate must be finite"))
            })
            .transpose()?;
        let mut merged = arguments
            .as_object()
            .cloned()
            .ok_or_else(|| invalid_params_error("predict arguments must be an object"))?;
        if merged.contains_key("guess") || merged.contains_key("rate") {
            return Err(invalid_params_error(
                "predict accepts the guess in arguments or inputResponses, not both",
            ));
        }
        merged.insert("guess".to_string(), json!(guess));
        if let Some(rate) = rate {
            merged.insert("rate".to_string(), json!(rate));
        }
        let mut prepared = request.clone();
        prepared["params"]["arguments"] = Value::Object(merged);
        prepared["params"]
            .as_object_mut()
            .expect("validated params object")
            .remove("inputResponses");
        return Ok((prepared, None));
    }

    if arguments.get("guess").is_some() || !request_supports_form_elicitation(request) {
        return Ok((request.clone(), None));
    }
    let pose = predict_tool(&arguments);
    if pose.get("isError").and_then(Value::as_bool) == Some(true) {
        return Ok((request.clone(), Some(pose)));
    }
    let message = pose
        .get("structuredContent")
        .and_then(|structured| structured.get("prompt"))
        .and_then(Value::as_str)
        .map(|prompt| format!("{prompt} Commit your guess before seeing the hidden readout."))
        .unwrap_or_else(|| {
            "Commit a prediction before seeing the room's hidden readout.".to_string()
        });
    Ok((
        request.clone(),
        Some(json!({
            "resultType": "input_required",
            "inputRequests": {
                "prediction": {
                    "method": "elicitation/create",
                    "params": {
                        "mode": "form",
                        "message": message,
                        "requestedSchema": {
                            "$schema": JSON_SCHEMA_2020_12,
                            "type": "object",
                            "properties": {
                                "guess": {
                                    "type": "number",
                                    "title": "Predicted readout",
                                    "description": "Your committed value for the hidden readout."
                                },
                                "rate": {
                                    "type": "number",
                                    "title": "Predicted local rate",
                                    "description": "Optional slope in readout units per full phase unit."
                                }
                            },
                            "required": ["guess"]
                        }
                    }
                }
            }
        })),
    ))
}

fn validate_tools_cursor(params: Option<&Value>) -> Result<(), String> {
    if params.and_then(|params| params.get("cursor")).is_some() {
        return Err(
            "Numinous returns its complete tool catalog in one page; cursor is invalid."
                .to_string(),
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ViewerPolicy {
    Public(PublicTool),
    Private,
    Control,
}

fn viewer_policy(name: &str) -> Option<ViewerPolicy> {
    if let Some(tool) = PublicTool::from_name(name) {
        return Some(ViewerPolicy::Public(tool));
    }
    match name {
        "cairn" | "forget" | "scores" | "journey" | "choose" | "trophies" | "read_journal"
        | "record_journal" | "correct_journal" | "export_journal" | "erase_journal"
        | "workspace" | "save_creation" | "open_creation" | "fork_creation" => {
            Some(ViewerPolicy::Private)
        }
        "broadcast_session" => Some(ViewerPolicy::Control),
        _ => None,
    }
}

fn capture_public_call(request: &Value, broadcast: &ConnectionBroadcast) -> Option<ViewerCall> {
    let params = request.get("params")?;
    let name = params.get("name")?.as_str()?;
    let ViewerPolicy::Public(tool) = viewer_policy(name)? else {
        return None;
    };
    let arguments = replay_arguments(
        params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({})),
    );
    let call = broadcast.capture(tool, &arguments)?;
    Some(ViewerCall {
        call,
        tool,
        arguments,
    })
}

struct ViewerCall {
    call: broadcast::PublicCall,
    tool: PublicTool,
    arguments: Value,
}

impl ViewerCall {
    fn commit(self, result: &Value) {
        let projected = viewer_result(self.tool, &self.arguments, result);
        self.call.commit(&projected);
    }
}

/// The lowest journey level at which this exact call is allowed. The viewer
/// replay uses it instead of the player's real level: a successful gated
/// call already proves at least this much, so replaying here shows the
/// viewer the play that actually happened without leaking how far past the
/// gate the player is. Replaying at zero showed a level-lock refusal as the
/// public result of a call that succeeded.
fn level_the_arguments_require(tool: PublicTool, arguments: &Value) -> u32 {
    match tool {
        PublicTool::Crack => {
            let digits = arguments.get("digits").and_then(Value::as_u64).unwrap_or(4);
            if digits > 4 { 5 } else { 0 }
        }
        PublicTool::Seti => {
            let channels = arguments
                .get("channels")
                .and_then(Value::as_u64)
                .unwrap_or(4);
            if channels > 4 { 7 } else { 0 }
        }
        PublicTool::Quiz => {
            let choices = arguments
                .get("choices")
                .and_then(Value::as_u64)
                .unwrap_or(4);
            if choices > 4 { 3 } else { 0 }
        }
        _ => 0,
    }
}

fn viewer_result(tool: PublicTool, arguments: &Value, result: &Value) -> Value {
    match tool {
        PublicTool::DescribeRoom => {
            describe_room_tool_for_journey(arguments, &numinous_core::Journey::default())
        }
        PublicTool::RevealRoom => {
            reveal_room_tool_for_journey(arguments, &numinous_core::Journey::default())
        }
        PublicTool::Crack => {
            crack_tool_at_level(arguments, level_the_arguments_require(tool, arguments))
        }
        PublicTool::Seti => {
            seti_tool_at_level(arguments, level_the_arguments_require(tool, arguments))
        }
        PublicTool::Quiz => {
            quiz_tool_at_level(arguments, level_the_arguments_require(tool, arguments))
        }
        _ => result.clone(),
    }
}

fn replay_arguments(mut arguments: Value) -> Value {
    let Some(object) = arguments.as_object_mut() else {
        return arguments;
    };
    object.remove("response_mode");
    let daily = object.get("daily").and_then(Value::as_bool) == Some(true);
    let effective_seed = if daily {
        object.get(DAILY_DAY_KEY).and_then(Value::as_u64)
    } else {
        object.get("seed").and_then(Value::as_u64)
    };
    object.remove("daily");
    object.remove(DAILY_DAY_KEY);
    object.remove("seed");
    if let Some(seed) = effective_seed {
        object.insert("seed".to_string(), json!(seed));
    }
    arguments
}

const MAX_SCHEMA_VALIDATION_DEPTH: usize = 16;

/// The most argument names a rejection will offer. A caller needs a nudge, not
/// the schema pasted back at them.
const MAX_ARGUMENT_SUGGESTIONS: usize = 2;

/// Validate the argument object against the bounded JSON Schema subset used by
/// this server. The catalog is the contract: clients that do not pre-validate
/// receive the same guiding errors as clients that do.
fn validate_declared_tool_arguments(params: Option<&Value>) -> Result<(), String> {
    let Some(params) = params else {
        return Ok(());
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(schema) = tools_catalog()
        .get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
        })
        .and_then(|tool| tool.get("inputSchema"))
    else {
        // Unknown tools remain JSON-RPC invalid-params errors at dispatch.
        return Ok(());
    };
    let default_arguments = json!({});
    let arguments = params.get("arguments").unwrap_or(&default_arguments);
    validate_schema_value(arguments, schema, "", 0)?;
    validate_domain_tool_arguments(name, arguments)
}

fn validate_domain_tool_arguments(name: &str, arguments: &Value) -> Result<(), String> {
    if name == "play_room" {
        let width = arguments
            .get("width")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_WIDTH) as usize;
        let height = arguments
            .get("height")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_HEIGHT) as usize;
        temporal::request(arguments, width, height)?;
        temporal::dwell_request(arguments, width, height)?;
    }
    if name == "munch_arcade"
        && let Some(actions) = arguments.get("actions").and_then(Value::as_array)
        && let Some((index, _)) = actions
            .iter()
            .enumerate()
            .find(|(_, action)| arcade_action(action).is_none())
    {
        return Err(format!(
            "Argument 'actions[{index}]' must be up, down, left, right, eat, w, a, s, d, or e."
        ));
    }
    Ok(())
}

fn argument_subject(path: &str) -> String {
    if path.is_empty() {
        "Arguments".to_string()
    } else {
        format!("Argument '{path}'")
    }
}

fn property_path(parent: &str, property: &str) -> String {
    if parent.is_empty() {
        property.to_string()
    } else {
        format!("{parent}.{property}")
    }
}

fn validate_schema_value(
    value: &Value,
    schema: &Value,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_SCHEMA_VALIDATION_DEPTH {
        return Err(format!(
            "{} exceeds the supported nesting depth of {MAX_SCHEMA_VALIDATION_DEPTH}.",
            argument_subject(path)
        ));
    }

    if let Some(alternatives) = schema.get("oneOf").and_then(Value::as_array) {
        let matches = alternatives
            .iter()
            .filter(|alternative| {
                validate_schema_value(value, alternative, path, depth + 1).is_ok()
            })
            .count();
        if matches != 1 {
            return Err(format!(
                "{} must match exactly one declared event shape.",
                argument_subject(path)
            ));
        }
        return Ok(());
    }

    if let Some(expected_type) = schema.get("type").and_then(Value::as_str) {
        let valid_type = match expected_type {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "number" => value.as_f64().is_some_and(f64::is_finite),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            _ => true,
        };
        if !valid_type {
            let subject = argument_subject(path);
            if path == "gesture" && expected_type == "array" {
                return Err(
                    "Argument 'gesture' must be an array, for example [{\"kind\":\"down\",\"x\":0.5,\"y\":0.5,\"t\":0.25},{\"kind\":\"up\",\"x\":0.5,\"y\":0.5,\"t\":0.25}]."
                        .to_string(),
                );
            }
            return Err(format!(
                "{subject} must be {article}{expected_type}.",
                article = if matches!(expected_type, "array" | "integer" | "object") {
                    "an "
                } else {
                    "a "
                }
            ));
        }
    }

    if let Some(allowed) = schema.get("enum").and_then(Value::as_array)
        && !allowed.contains(value)
    {
        let choices = allowed
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "{} must be one of: {choices}.",
            argument_subject(path)
        ));
    }

    if let Some(text) = value.as_str()
        && let Some(maximum) = schema.get("maxLength").and_then(Value::as_u64)
    {
        let count = text.chars().count() as u64;
        if count > maximum {
            return Err(format!(
                "{} must be at most {maximum} characters.",
                argument_subject(path)
            ));
        }
    }

    if let Some(number) = value.as_f64() {
        for (keyword, relation) in [
            ("minimum", "at least"),
            ("maximum", "at most"),
            ("exclusiveMinimum", "greater than"),
            ("exclusiveMaximum", "less than"),
        ] {
            let Some(bound) = schema.get(keyword).and_then(Value::as_f64) else {
                continue;
            };
            let valid = match keyword {
                "minimum" => number >= bound,
                "maximum" => number <= bound,
                "exclusiveMinimum" => number > bound,
                _ => number < bound,
            };
            if !valid {
                if path == "t" && keyword == "exclusiveMaximum" && bound == 1.0 {
                    return Err(
                        "Argument 't' must be less than 1. Use a finite phase from 0.0 through 0.999; the loop endpoint is 0.0."
                            .to_string(),
                    );
                }
                return Err(format!(
                    "{} must be {relation} {bound}.",
                    argument_subject(path)
                ));
            }
        }
    }

    if let Some(object) = value.as_object() {
        let properties = schema.get("properties").and_then(Value::as_object);
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for property in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(property) {
                    let missing = property_path(path, property);
                    return Err(format!("Missing required argument '{missing}'."));
                }
            }
        }
        if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false) {
            for property in object.keys() {
                if properties.is_none_or(|known| !known.contains_key(property)) {
                    let hint = nearest_argument_hint(property, properties);
                    if path.is_empty() {
                        return Err(format!(
                            "Unexpected argument '{}'.{hint}",
                            numinous_core::echoable_id(property)
                        ));
                    }
                    return Err(format!(
                        "{} has an unexpected field '{}'.{hint}",
                        argument_subject(path),
                        numinous_core::echoable_id(property)
                    ));
                }
            }
        }
        if let Some(additional_schema) = schema
            .get("additionalProperties")
            .filter(|additional| additional.is_object())
        {
            for (property, property_value) in object {
                if properties.is_none_or(|known| !known.contains_key(property)) {
                    validate_schema_value(
                        property_value,
                        additional_schema,
                        &property_path(path, property),
                        depth + 1,
                    )?;
                }
            }
        }
        if let Some(properties) = properties {
            for (property, property_schema) in properties {
                if let Some(property_value) = object.get(property) {
                    validate_schema_value(
                        property_value,
                        property_schema,
                        &property_path(path, property),
                        depth + 1,
                    )?;
                }
            }
        }
    }

    if let Some(items) = value.as_array() {
        if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64)
            && items.len() < minimum as usize
        {
            return Err(format!(
                "{} must contain at least {minimum} items.",
                argument_subject(path)
            ));
        }
        if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64)
            && items.len() > maximum as usize
        {
            return Err(format!(
                "{} accepts at most {maximum} items.",
                argument_subject(path)
            ));
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                validate_schema_value(item, item_schema, &format!("{path}[{index}]"), depth + 1)?;
            }
        }
    }

    Ok(())
}

/// Dispatch a `tools/call`.
fn call_tool(
    params: Option<&Value>,
    journey_file: &std::path::Path,
    broadcast: &ConnectionBroadcast,
    workspace: &ProcessWorkspace,
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

    // A present numeric identity argument that is not a non-negative integer is a
    // caller mistake, not a reason to silently fall back to seed 1: every tool
    // treats seed and round as non-negative integers, so guide once, centrally,
    // rather than letting a negative or fractional value collide with a real seed.
    for key in ["seed", "round", "variation"] {
        if let Some(value) = args.get(key)
            && !value.is_null()
            && value.as_u64().is_none()
        {
            return Ok(tool_error(&format!(
                "Argument '{key}' must be a non-negative integer."
            )));
        }
    }

    let response_mode = args.get("response_mode").and_then(Value::as_str);
    let mut domain_args = args.clone();
    if let Some(object) = domain_args.as_object_mut() {
        object.remove("response_mode");
    }

    let result = match name {
        "list_rooms" => room_door::list_tool(),
        "describe_room" => describe_room_tool(&domain_args, journey_file),
        "reveal_room" => reveal_room_tool(&domain_args, journey_file),
        "play_room" => play_room_tool(&domain_args, journey_file),
        "challenge" => challenge_tool(&domain_args),
        "predict" => predict_tool(&domain_args),
        "cairn" => cairn_tool(&domain_args, journey_file, &cairn_path()),
        "read_journal" => journal::read_tool(&domain_args, &journal_path()),
        "record_journal" => journal::record_tool(
            &domain_args,
            &journal_path(),
            |tool, replay_args| match tool {
                numinous_core::EncounterTool::PlayRoom => play_room_tool(replay_args, journey_file),
                numinous_core::EncounterTool::ListenRoom => listen_room_tool(replay_args),
                numinous_core::EncounterTool::SingExpression => sing_expression_tool(replay_args),
            },
        ),
        "correct_journal" => journal::correct_tool(&domain_args, &journal_path()),
        "export_journal" => journal::export_tool(&domain_args, &journal_path()),
        "erase_journal" => journal::erase_tool(&domain_args, &journal_path()),
        "workspace" => workspace_tool(&domain_args, workspace, &journal_path()),
        "listen_room" => listen_room_tool(&domain_args),
        "list_sims" => tool_text(&list_sims_text()),
        "run_sim" => run_sim_tool(&domain_args),
        "quiz" => quiz_tool(&domain_args, journey_file),
        "munch" => munch_tool(&domain_args),
        "munch_arcade" => munch_arcade_tool(&domain_args),
        "journey" => journey_tool(journey_file),
        "nim" => nim_tool(&domain_args),
        "hackenbush" => hackenbush_tool(&domain_args),
        "party" => party_tool(&domain_args),
        "fifteen" => fifteen_tool(&domain_args),
        "scores" => scores_tool(&scores_path()),
        "forget" => forget_tool(&domain_args, &local_state_paths_at(journey_file)),
        "crack" => crack_tool(&domain_args, journey_file),
        "seti" => seti_tool(&domain_args, journey_file),
        "aliens" => aliens_tool(&domain_args),
        "gauntlet" => gauntlet_tool(&domain_args),
        "choose" => choose_tool(&domain_args, journey_file),
        "trophies" => trophies_tool(journey_file),
        "plot_expression" => plot_expression_tool(&domain_args),
        "save_creation" => save_creation_tool(&domain_args),
        "open_creation" => open_creation_tool(&domain_args),
        "fork_creation" => fork_creation_tool(&domain_args),
        "sing_expression" => sing_expression_tool(&domain_args),
        "explain_joke" => explain_joke_tool(&domain_args),
        "broadcast_session" => broadcast_session_tool(&domain_args, broadcast),
        other => return Err((-32602_i64, format!("Unknown tool: {other}"))),
    };
    Ok(apply_response_mode(name, response_mode, result))
}

fn broadcast_session_tool(args: &Value, broadcast: &ConnectionBroadcast) -> Value {
    let Some(action) = args.get("action").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'action'.");
    };
    let pairing_code = args.get("pairing_code").and_then(Value::as_str);
    let outcome = match action {
        "start" => {
            let Some(code) = pairing_code else {
                return tool_error("Starting a viewer requires 'pairing_code'.");
            };
            broadcast.start(code)
        }
        "status" if pairing_code.is_none() => Ok(broadcast.status()),
        "pause" if pairing_code.is_none() => broadcast.pause(),
        "resume" if pairing_code.is_none() => broadcast.resume(),
        "stop" if pairing_code.is_none() => broadcast.stop(),
        "status" | "pause" | "resume" | "stop" => {
            return tool_error("'pairing_code' is accepted only when action is 'start'.");
        }
        _ => return tool_error("Unknown broadcast action."),
    };
    match outcome {
        Ok(status) => tool_structured(
            &format!(
                "Local session broadcast is {}. Private activity is never represented; silence reveals nothing about private calls.",
                status.state
            ),
            broadcast_status_json(&status),
        ),
        Err(error) => tool_error(&format!(
            "Broadcast unchanged: {error}.{}",
            broadcast_failure_hint(action, error)
        )),
    }
}

/// A rejected start is the one broadcast failure a caller cannot reason its
/// way out of. Every other action fails on state the caller can inspect, but a
/// pairing code exists only inside a human's App, so an unaided caller can do
/// nothing but invent codes. Name where the real one comes from instead.
fn broadcast_failure_hint(action: &str, error: broadcast::SessionError) -> &'static str {
    match (action, error) {
        ("start", broadcast::SessionError::PairingRejected) => {
            " A pairing code cannot be guessed or reused: a human running the App \
             chooses Shared Play, and the one-use code it shows is the only code \
             that starts a viewer. Without that invitation there is nothing to join, \
             and your play continues unwatched."
        }
        _ => "",
    }
}

fn broadcast_status_json(status: &SessionSnapshot) -> Value {
    json!({
        "state": status.state,
        "sessionId": status.session_id,
        "consentEpoch": status.consent_epoch,
        "nextPublicSequence": status.next_public_sequence,
        "droppedPublicEvents": status.dropped_public_events,
        "queuedEvents": status.queued_events,
        "queuedBytes": status.queued_bytes,
        "privateActivityVisible": false,
    })
}

/// Compact mode removes only prose that duplicates a complete typed result.
/// Guiding errors and text-only tools stay untouched because their text is the
/// result, not presentation overhead.
fn apply_response_mode(name: &str, response_mode: Option<&str>, mut result: Value) -> Value {
    if response_mode != Some("compact")
        || result.get("isError").and_then(Value::as_bool) == Some(true)
        || result.get("structuredContent").is_none()
    {
        return result;
    }

    let Some(summary) = compact_result_summary(name, &result["structuredContent"]) else {
        return result;
    };
    let Some(current_text) = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|content| content.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
    else {
        return result;
    };
    if summary.len() >= current_text.len() {
        return result;
    }
    result["content"] = json!([{
        "type": "text",
        "text": summary,
    }]);
    result
}

fn compact_result_summary(name: &str, structured: &Value) -> Option<String> {
    match name {
        // A compact reply that lists bare ids sends a reader looking them up
        // in the very catalog the short mode exists to spare them, so the
        // prose names its starters the same way the structured array does.
        "list_rooms" => room_door::compact_summary(structured),
        "describe_room" => {
            let mut summary = format!(
                "{} ({}) in {}. Action: {}.",
                structured.get("title")?.as_str()?,
                structured.get("room")?.as_str()?,
                structured.get("wing")?.as_str()?,
                structured.get("action")?.as_str()?
            );
            match structured
                .get("journalCue")
                .and_then(|cue| cue.get("status"))
                .and_then(Value::as_str)
            {
                Some("remembered") => summary.push_str(
                    " This local player profile kept something here; no journal text was opened.",
                ),
                Some("unavailable") => summary
                    .push_str(" The local journal could not be read; no contents were returned."),
                _ => {}
            }
            summary.push_str(" Read structuredContent for the goal, blurb, next play call, and optional journal cue.");
            Some(summary)
        }
        "play_room" => {
            let mut summary = format!(
                "{} ({}) at t={:.3}, {}x{}. Action: {}.",
                structured.get("title")?.as_str()?,
                structured.get("room")?.as_str()?,
                structured.get("t")?.as_f64()?,
                structured.get("width")?.as_u64()?,
                structured.get("height")?.as_u64()?,
                structured.get("action")?.as_str()?
            );
            if let Some(status) = structured.get("status").and_then(Value::as_str) {
                summary.push_str(&format!(" Status: {status}."));
            }
            if let Some(cells) = structured
                .get("delta")
                .and_then(|delta| delta.get("cells_changed"))
                .and_then(Value::as_u64)
            {
                summary.push_str(&format!(" Touch changed {cells} cells."));
            }
            if let Some(temporal) = structured.get("temporal") {
                summary.push_str(&format!(
                    " Temporal from t={:.3} to t={:.3}, {} cells changed.",
                    temporal.get("fromT")?.as_f64()?,
                    temporal.get("toT")?.as_f64()?,
                    temporal.get("delta")?.get("cells_changed")?.as_u64()?
                ));
            }
            if let Some(dwell) = structured.get("dwell") {
                summary.push_str(&format!(
                    " Dwell across {} looks, {} of {} cells held still.",
                    dwell.get("looks")?.as_u64()?,
                    dwell.get("held")?.get("unchanged_cells")?.as_u64()?,
                    dwell.get("held")?.get("total_cells")?.as_u64()?
                ));
            }
            if structured.get("encounter").is_some() {
                summary.push_str(" Encounter receipt attached.");
            }
            if let Some(beat) = structured
                .get("engineeredAha")
                .and_then(|aha| aha.get("beat"))
                .and_then(Value::as_str)
            {
                summary.push_str(&format!(" Aha beat: {beat}."));
            }
            let optional_fields = match (
                structured.get("temporal").is_some(),
                structured.get("dwell").is_some(),
                structured.get("encounter").is_some(),
            ) {
                (true, true, true) => "render, temporal, dwell, encounter, ",
                (true, true, false) => "render, temporal, dwell, ",
                (true, false, true) => "render, temporal, encounter, ",
                (true, false, false) => "render, temporal, ",
                (false, true, true) => "render, dwell, encounter, ",
                (false, true, false) => "render, dwell, ",
                (false, false, true) => "render, encounter, ",
                (false, false, false) => "render, ",
            };
            summary.push_str(&format!(
                " Read structuredContent.{optional_fields}pokes, gesture, status, delta, goal, goalMet, and engineeredAha for the complete result; ask reveal_room for the explanation."
            ));
            Some(summary)
        }
        "listen_room" => {
            let mut summary = format!(
                "{} ({}) at t={:.3}: {} of {} mathematical notes returned over {:.2}s.",
                structured.get("title")?.as_str()?,
                structured.get("room")?.as_str()?,
                structured.get("t")?.as_f64()?,
                structured.get("returned_note_count")?.as_u64()?,
                structured.get("note_count")?.as_u64()?,
                structured.get("duration_seconds")?.as_f64()?
            );
            if structured.get("encounter").is_some() {
                summary.push_str(" Encounter receipt attached.");
            }
            summary.push_str(" Read structuredContent.pokes, gesture, motif, ambient_bed, notes, and sound_roles for the complete typed layers.");
            Some(summary)
        }
        "run_sim" => Some(format!(
            "{} ({}): {} Read structuredContent.params, readout, and render for the complete result.",
            structured.get("title")?.as_str()?,
            structured.get("sim")?.as_str()?,
            structured.get("readout")?.as_str()?
        )),
        "quiz" => {
            let seed = structured.get("seed")?.as_u64()?;
            let round = structured.get("round")?.as_u64()?;
            let choice_count = structured.get("choiceCount")?.as_u64()?;
            if let Some(correct) = structured.get("correct").and_then(Value::as_bool) {
                Some(format!(
                    "Quiz seed {seed}, round {round}, choices {choice_count}: {} Answer {} ({}). Read structuredContent.why for the explanation.",
                    if correct { "correct." } else { "not quite." },
                    structured.get("answer")?.as_str()?,
                    structured.get("answerTitle")?.as_str()?
                ))
            } else {
                let choices = structured.get("choices")?.as_array()?;
                Some(format!(
                    "Quiz seed {seed}, round {round}, choices {choice_count}: choose A through {}. Call quiz again with seed {seed}, round {round}, choices {choice_count}, and guess. Read structuredContent.art and choices for the complete puzzle.",
                    choices.last()?.get("letter")?.as_str()?
                ))
            }
        }
        "gauntlet" => {
            let seed = structured.get("seed")?.as_u64()?;
            if let Some(total) = structured.get("total").and_then(Value::as_i64) {
                Some(format!(
                    "Gauntlet seed {seed}: {}/4 clean, total {total}. Read structuredContent.stageScores and reveals for the complete grade.",
                    structured.get("clean")?.as_u64()?
                ))
            } else {
                Some(format!(
                    "Gauntlet seed {seed}: four stages. Call again with answers.bites, shape, sky, and wires. Read structuredContent.munch, shape, sky, and bomb for the complete run."
                ))
            }
        }
        "trophies" => Some(format!(
            "Trophy case: {} of {} earned. Read structuredContent.trophies for every name, condition, and earned state.",
            structured.get("earned")?.as_u64()?,
            structured.get("total")?.as_u64()?
        )),
        "workspace" => compact_workspace_summary(structured),
        _ => None,
    }
}

fn describe_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    let mut result = describe_room_tool_for_journey(args, &load_journey(journey_file));
    let Some(room) = result
        .get("structuredContent")
        .and_then(|structured| structured.get("room"))
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return result;
    };
    let Some((cue, cue_text)) = journal::room_cue(&journal_path(), &room) else {
        return result;
    };
    result["structuredContent"]["journalCue"] = cue;
    if let Some(text) = result
        .get_mut("content")
        .and_then(Value::as_array_mut)
        .and_then(|content| content.first_mut())
        .and_then(|block| block.get_mut("text"))
        && let Some(existing) = text.as_str()
    {
        *text = Value::String(format!("{existing}\n\n{cue_text}"));
    }
    result
}

/// Find a room the way the terminal does: the catalog always, and the
/// unlisted ones for a journey the veil admits.
///
/// The gate itself lives in core so both faces read one rule, but this face
/// used to skip the second half entirely: a learner who could open the
/// hidden room from the terminal was told over MCP that it does not exist.
/// One player, one standing, two answers.
fn find_room_for(
    id: &str,
    journey: &numinous_core::Journey,
) -> Option<Box<dyn numinous_core::Room>> {
    room_by_id(id).or_else(|| {
        numinous_core::behind_the_veil(journey)
            .then(|| numinous_core::hidden_room_by_id(id))
            .flatten()
    })
}

fn describe_room_tool_for_journey(args: &Value, journey: &numinous_core::Journey) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    match find_room_for(id, journey) {
        Some(room) => {
            let m = room.meta();
            let goal_line = room
                .goal()
                .map(|goal| format!("\nGoal: {goal}"))
                .unwrap_or_default();
            let text = format!(
                "{} ({})\nWing: {}\nAction: {}{goal_line}\n\n{}\n\nPlay this room before asking reveal_room for its explanation.",
                m.title,
                m.id,
                m.wing,
                numinous_core::room_action(room.as_ref()),
                m.blurb,
            );
            let structured = json!({
                "room": m.id,
                "title": m.title,
                "wing": m.wing,
                "action": numinous_core::room_action(room.as_ref()),
                "goal": room.goal(),
                "blurb": m.blurb,
                "next": {
                    "tool": "play_room",
                    "id": m.id,
                },
            });
            tool_structured(&text, structured)
        }
        // Not every name is a room. A few answer anyway, and a few answer
        // only those with standing.
        None => match numinous_core::akousma(id) {
            Some(whisper) => tool_structured(
                whisper,
                json!({ "kind": "whisper", "id": id, "text": whisper }),
            ),
            // The veil rule is core's: this face once held its own gate at
            // 28 sparks while the terminal held the documented rank, and
            // the same listener was inside on one face and refused here.
            None if numinous_core::behind_the_veil(journey) => {
                match numinous_core::deep_akousma(id) {
                    Some(whisper) => tool_structured(
                        whisper,
                        json!({ "kind": "whisper", "id": id, "text": whisper }),
                    ),
                    None => tool_error(&unknown_room(id)),
                }
            }
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

/// Project one stable stereo room bed into bounded, typed protocol evidence.
///
/// The projection exposes arrangement intent and objective signal features,
/// never PCM samples or a machine-local file reference. Signal features catch
/// engineering regressions but do not claim that a bed sounds pleasant.
fn ambient_bed_value(motif: numinous_core::Motif, include_events: bool) -> Result<Value, String> {
    let arrangement = motif.arrangement();
    if arrangement.notes.len() > numinous_core::MAX_ROOM_BED_EVENTS {
        return Err(format!(
            "Room bed contains {} events; the protocol limit is {}.",
            arrangement.notes.len(),
            numinous_core::MAX_ROOM_BED_EVENTS
        ));
    }

    let duration_seconds = arrangement.steps as f64 * f64::from(arrangement.step_seconds);
    let mut value = json!({
        "schema": "numinous.room-bed.events",
        "schema_version": 1,
        "renderer": "numinous.chiptune.stereo.v1",
        "source_sample_rate_hz": numinous_core::ROOM_BED_SOURCE_RATE,
        "channels": 2,
        "steps": arrangement.steps,
        "step_seconds": arrangement.step_seconds,
        "duration_seconds": duration_seconds,
        "event_count": arrangement.notes.len(),
        "events_included": include_events,
    });

    // Compact spectrum is always available: render once and share with detail.
    let samples = arrangement.render_stereo(numinous_core::ROOM_BED_SOURCE_RATE);
    let spectrum =
        numinous_core::arrangement_spectrum(&samples, numinous_core::ROOM_BED_SOURCE_RATE);
    value["spectrum"] = json!({
        "schema": "numinous.spectrum.bands",
        "schema_version": 1,
        "band_count": numinous_core::BAND_COUNT,
        "names": numinous_core::BAND_NAMES,
        "levels": spectrum.to_vec(),
    });

    if include_events {
        let events = arrangement
            .notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                json!({
                    "index": index + 1,
                    "frequency_hz": note.frequency,
                    "start_step": note.start_step,
                    "step_count": note.step_count,
                    "start_seconds": note.start_step as f64 * f64::from(arrangement.step_seconds),
                    "duration_seconds": note.step_count as f64 * f64::from(arrangement.step_seconds),
                    "voice": note.voice.id(),
                    "level": note.level,
                    "pan": note.pan,
                })
            })
            .collect::<Vec<_>>();
        let metrics = numinous_core::stereo_signal_metrics(&samples);
        value["events"] = json!(events);
        value["signal_metrics"] = json!({
            "scope": "pre_master_room_bed",
            "interpretation": "Engineering regression evidence only; not a pleasantness score.",
            "frame_count": metrics.frame_count,
            "trailing_samples": metrics.trailing_samples,
            "non_finite_samples": metrics.non_finite_samples,
            "subnormal_samples": metrics.subnormal_samples,
            "clipped_samples": metrics.clipped_samples,
            "peak": metrics.peak,
            "rms": metrics.rms,
            "crest_db": metrics.crest_db,
            "left_rms": metrics.left_rms,
            "right_rms": metrics.right_rms,
            "channel_balance_db": metrics.channel_balance_db,
            "left_dc": metrics.left_dc,
            "right_dc": metrics.right_dc,
            "correlation": metrics.correlation,
            "side_to_mid_db": metrics.side_to_mid_db,
            "max_step": metrics.max_step,
            "zero_sample_fraction": metrics.zero_sample_fraction,
        });
    }

    Ok(value)
}

/// The `listen_room` tool: the room's sound as notation a mind can read.
fn listen_room_tool(args: &Value) -> Value {
    let want_receipt = match encounter_request(args) {
        Ok(want) => want,
        Err(message) => return tool_error(&message),
    };
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    if !(0.0..1.0).contains(&t) {
        return tool_error("Argument 't' must be a phase in [0,1).");
    }
    let include_ambient_events = match args.get("ambient_detail").and_then(Value::as_str) {
        None | Some("summary") => false,
        Some("events") => true,
        Some(_) => return tool_error("Argument 'ambient_detail' must be 'summary' or 'events'."),
    };
    let inputs = match parse_room_inputs(args) {
        Ok(inputs) => inputs,
        Err(message) => return tool_error(&message),
    };
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let room = numinous_core::room_by_id_with(id, variation);
    let Some(room) = room else {
        return tool_error(&unknown_room(id));
    };
    let poke_inputs = numinous_core::inputs_from_pokes(&inputs.pokes, t);
    let accepted_inputs = if inputs.gesture.is_empty() {
        poke_inputs.as_slice()
    } else {
        inputs.gesture.as_slice()
    };
    let spec = room.sound_input(t, accepted_inputs);
    let note_count = spec.notes.len();
    let mut lines = vec![format!(
        "{} at t={t:.3}: {:.1}s of sound, {} notes.",
        room.meta().title,
        spec.duration,
        note_count
    )];
    let ambient_motif = room.motif().map(|motif| {
        let notation = motif.notation();
        lines.push(format!(
            "Ambient motif: {} at {} BPM, {}. It encodes: {}.",
            motif.key,
            motif.tempo,
            notation.join(" "),
            motif.encodes
        ));
        json!({
            "key": motif.key,
            "tempo_bpm": motif.tempo,
            "notation": notation,
            "encodes": motif.encodes,
        })
    });
    let ambient_bed = match room.motif() {
        Some(motif) => match ambient_bed_value(motif, include_ambient_events) {
            Ok(value) => Some(value),
            Err(message) => return tool_error(&message),
        },
        None => None,
    };
    if let Some(bed) = ambient_bed.as_ref() {
        lines.push(format!(
            "Stable stereo room bed: {:.2}s, {} arranged events at {} Hz.{}",
            bed["duration_seconds"].as_f64().unwrap_or_default(),
            bed["event_count"].as_u64().unwrap_or_default(),
            numinous_core::ROOM_BED_SOURCE_RATE,
            if include_ambient_events {
                " Complete events and pre-master signal metrics follow in structuredContent."
            } else {
                " Request ambient_detail=events for the complete bounded event projection."
            }
        ));
    }
    let mut structured_notes = Vec::new();
    if note_count > 0 {
        lines.push("Mathematical sonification:".to_string());
    }
    for (i, note) in spec.notes.iter().take(64).enumerate() {
        let name = note_name(note.freq);
        lines.push(format!(
            "  note {:>2}: {:>7.1} Hz ({:>3})  at {:>5.2}s  for {:.2}s  amp {:.2}",
            i + 1,
            note.freq,
            name,
            note.start,
            note.dur,
            note.amp
        ));
        structured_notes.push(json!({
            "index": i + 1,
            "frequency_hz": note.freq,
            "name": name,
            "start_seconds": note.start,
            "duration_seconds": note.dur,
            "amplitude": note.amp,
        }));
    }
    if note_count > 64 {
        lines.push(format!("  ... and {} more notes.", note_count - 64));
    }
    // The room's own sonification, as sound rather than as a table of it. This
    // is the mathematical voice, not the ambient bed: what the room is doing
    // right now at this phase, under this hand.
    let audible = match audible::requested(args) {
        Ok(true) => match audible::block(&spec) {
            Ok(rendered) => Some(rendered),
            Err(message) => return tool_error(&message),
        },
        Ok(false) => None,
        Err(message) => return tool_error(&message),
    };
    if audible.is_some() {
        lines.push(
            "A WAV of this room at this phase follows as an audio \
             attachment. Whether your client can surface it as sound is \
             its answer to give, not ours; if it cannot, the notation \
             above is the whole of what arrived."
                .to_string(),
        );
    }
    let mut structured = json!({
        "room": room.meta().id,
        "title": room.meta().title,
        "t": t,
        "variation": variation,
        "audio": audible.as_ref().map(|(_, described)| described.clone()),
        "pokes": inputs.pokes,
        "gesture": if inputs.gesture.is_empty() { Value::Null } else { gesture_json(&inputs.gesture) },
        "duration_seconds": spec.duration,
        "note_count": note_count,
        "returned_note_count": structured_notes.len(),
        "truncated": note_count > 64,
        "motif": ambient_motif,
        "ambient_bed": ambient_bed,
        "notes": structured_notes,
        "sound_roles": {
            "ambient_motif": { "field": "motif" },
            "ambient_arrangement": { "field": "ambient_bed" },
            "mathematical_sonification": { "field": "notes" },
        },
    });
    if want_receipt {
        let audio_asked = args.get("audio").and_then(Value::as_bool).unwrap_or(false);
        let action = encounter_listen_action(
            room.meta().id,
            t,
            variation,
            include_ambient_events,
            audio_asked,
            &inputs.pokes,
            &inputs.gesture,
        );
        let motif = structured.get("motif");
        let bed = structured.get("ambient_bed");
        let result = encounter_listen_result(
            room.meta().id,
            t,
            variation,
            spec.duration.into(),
            note_count as u64,
            structured_notes.len() as u64,
            note_count > 64,
            motif
                .and_then(|value| value.get("key"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            motif
                .and_then(|value| value.get("tempo_bpm"))
                .and_then(Value::as_u64),
            motif
                .and_then(|value| value.get("encodes"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            bed.and_then(|value| value.get("duration_seconds"))
                .and_then(Value::as_f64),
            bed.and_then(|value| value.get("event_count"))
                .and_then(Value::as_u64),
            structured
                .get("audio")
                .and_then(|value| value.get("encodedBytes"))
                .and_then(Value::as_u64),
        );
        match issue_receipt(
            numinous_core::EncounterTool::ListenRoom,
            &action.canonical_bytes(),
            &result.canonical_bytes(),
        ) {
            Ok(receipt) => {
                structured["encounter"] = receipt_json(&receipt, listen_action_json(&action))
            }
            Err(message) => return tool_error(&message),
        }
        lines.push("Encounter receipt attached.".to_string());
    }
    let result = tool_structured(&lines.join("\n"), structured);
    match audible {
        Some((block, _)) => audible::attach(result, block),
        None => result,
    }
}

/// The `reveal_room` tool: optional concept + revelation (the learn surface).
fn reveal_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    reveal_room_tool_for_journey(args, &load_journey(journey_file))
}

fn reveal_room_tool_for_journey(args: &Value, journey: &numinous_core::Journey) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    match find_room_for(id, journey) {
        Some(room) => {
            let room_id = room.meta().id;
            if numinous_core::is_engineered_aha_room(room_id) && !journey.has_consolidated(room_id)
            {
                return tool_error(
                    "This explanation is still closed. Play the room, commit its wager, then call play_room with aha_summon true.",
                );
            }
            if !numinous_core::is_engineered_aha_room(room_id) && !journey.visited.contains(room_id)
            {
                return tool_error(
                    "This explanation is still closed. Play the room once, then ask reveal_room again.",
                );
            }
            let cut0_by_boon = journey.chosen.contains(&format!("cut:{room_id}:0"));
            let citation =
                numinous_core::room_citation_unlocked(room_id, journey.level(), cut0_by_boon);
            let mut body = numinous_core::explain_text(room.meta().id, room.reveal());
            let mut structured_cuts = Vec::new();
            for (i, cut) in room.deep_cuts().iter().enumerate() {
                let need = numinous_core::CUT_LEVELS
                    .get(i)
                    .copied()
                    .unwrap_or(u32::MAX);
                let by_boon = journey.chosen.contains(&format!("cut:{room_id}:{i}"));
                if journey.level() >= need || by_boon {
                    body.push_str(&format!("\n\nDeeper: {cut}"));
                    structured_cuts.push(json!({
                        "index": i,
                        "status": "available",
                        "unlock_level": need,
                        "text": cut,
                    }));
                } else {
                    body.push_str(&format!("\n\nLOCKED: a deeper cut opens at LV {need}."));
                    structured_cuts.push(json!({
                        "index": i,
                        "status": "locked",
                        "unlock_level": need,
                    }));
                    break;
                }
            }
            if let Some(citation) = citation {
                body = format!("{body}\n\n{citation}");
            }
            let mut structured = json!({
                "room": room.meta().id,
                "title": room.meta().title,
                "reveal": room.reveal(),
                "deep_cuts": structured_cuts,
            });
            if let Some(concept) = room.concept() {
                structured["concept"] = json!(concept);
            }
            if let Some(citation) = citation {
                structured["citation"] = json!(citation);
            }
            tool_structured(&body, structured)
        }
        // The Cairn is not a room but a mind pausing there naturally reaches for
        // reveal; point it at the right door rather than saying it does not exist.
        None if id == "cairn" => {
            let reveal = "The Cairn is not a room but a message across time: use the `cairn` tool. \
                          Call it with a `seed` to receive a stone a mind before you left, factor its \
                          semiprime length, then call again with that `width` to read what was left. \
                          At level 42 you may `leave` one true thing of your own.";
            tool_structured(
                reveal,
                json!({ "kind": "cairn", "tool": "cairn", "reveal": reveal }),
            )
        }
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
/// `up` (all needing finite `x`, `y`, `t` in `[0, 1]`), or `cancel` (no
/// other fields; unknown fields are rejected per the schema). Bounded to
/// [`numinous_core::MAX_ROOM_INPUTS`].
fn parse_room_gesture(args: &Value) -> Result<Vec<numinous_core::RoomInput>, String> {
    let Some(raw) = args.get("gesture") else {
        return Ok(Vec::new());
    };
    let Some(events) = raw.as_array() else {
        return Err(
            "Argument 'gesture' must be an array, for example [{\"kind\":\"down\",\"x\":0.5,\"y\":0.5,\"t\":0.25},{\"kind\":\"up\",\"x\":0.5,\"y\":0.5,\"t\":0.25}]."
                .to_string(),
        );
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

#[derive(Debug)]
struct ParsedRoomInputs {
    pokes: Vec<(f64, f64)>,
    gesture: Vec<numinous_core::RoomInput>,
}

fn parse_room_inputs(args: &Value) -> Result<ParsedRoomInputs, String> {
    let pokes = parse_room_pokes(args)?;
    let gesture = parse_room_gesture(args)?;
    if !pokes.is_empty() && !gesture.is_empty() {
        return Err(
            "Use either 'pokes' (static hand points) or 'gesture' (a pointer trail), not both in one call."
                .to_string(),
        );
    }
    Ok(ParsedRoomInputs { pokes, gesture })
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

fn play_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    play_room_tool_for_journey(args, &load_journey(journey_file))
}

fn render_room_observation(
    room: &dyn numinous_core::Room,
    canvas: &mut Canvas,
    t: f64,
    inputs: &[numinous_core::RoomInput],
) {
    if inputs.is_empty() {
        room.render(canvas, t);
    } else {
        room.render_input(canvas, t, inputs);
    }
}

fn room_status_at(
    room: &dyn numinous_core::Room,
    t: f64,
    inputs: &[numinous_core::RoomInput],
) -> Option<String> {
    if inputs.is_empty() {
        room.status(t)
    } else {
        room.status_input(t, inputs)
    }
}

fn projected_play_status(
    room_status: Option<String>,
    aha_request: FlagshipAhaRequest,
    engineered_aha: Option<&Value>,
) -> Option<String> {
    let aha_beat = engineered_aha
        .and_then(|value| value.get("beat"))
        .and_then(Value::as_str);
    let use_aha_status = aha_request.uses_generation_args()
        || matches!(
            aha_beat,
            Some("prime" | "morph" | "confirm" | "consolidated")
        );
    if use_aha_status {
        engineered_aha
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or(room_status)
    } else {
        room_status
    }
}

fn play_room_tool_for_journey(args: &Value, journey: &numinous_core::Journey) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let canonical_id = numinous_core::canonical_room_id(id);
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let width = args
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_WIDTH) as usize;
    let height = args
        .get("height")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_HEIGHT) as usize;
    // Schema validation is the primary gate; this rejects hostile sizes if a
    // future call path ever reaches the tool without the catalog check.
    if !(1..=MAX_TOOL_WIDTH as usize).contains(&width)
        || !(1..=MAX_TOOL_HEIGHT as usize).contains(&height)
    {
        return tool_error(&format!(
            "Canvas size must be between 1x1 and {MAX_TOOL_WIDTH}x{MAX_TOOL_HEIGHT}."
        ));
    }
    let temporal_pair = match temporal::request(args, width, height) {
        Ok(pair) => pair,
        Err(message) => return tool_error(&message),
    };
    let dwell_window = match temporal::dwell_request(args, width, height) {
        Ok(window) => window,
        Err(message) => return tool_error(&message),
    };
    let want_receipt = match encounter_request(args) {
        Ok(want) => want,
        Err(message) => return tool_error(&message),
    };
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let inputs = match parse_room_inputs(args) {
        Ok(inputs) => inputs,
        Err(message) => return tool_error(&message),
    };
    let aha_request = match parse_flagship_aha_request(args, canonical_id) {
        Ok(request) => request,
        Err(message) => return tool_error(&message),
    };

    let room = if variation == 0 {
        // The same veil door describe and reveal use: a hidden room is
        // unlisted, not nonexistent, and a learner the terminal admits is
        // the same learner here. Variation stays a catalog contract, which
        // is the terminal's rule too.
        find_room_for(id, journey)
    } else {
        numinous_core::room_by_id_with(id, variation)
    };

    match room {
        Some(room) => {
            let mut canvas = Canvas::new(width, height);
            let poke_inputs = numinous_core::inputs_from_pokes(&inputs.pokes, t);
            let accepted_inputs = if inputs.gesture.is_empty() {
                poke_inputs.as_slice()
            } else {
                inputs.gesture.as_slice()
            };
            // A gesture trail: held rooms give it pull-and-release semantics;
            // every other room answers through the same bridge the App uses.
            render_room_observation(room.as_ref(), &mut canvas, t, accepted_inputs);
            let delta = if accepted_inputs.is_empty() {
                None
            } else {
                let mut base = Canvas::new(width, height);
                room.render(&mut base, t);
                base.delta(&canvas)
            };
            let m = room.meta();
            let action = numinous_core::room_action(room.as_ref());
            let room_status = room_status_at(room.as_ref(), t, accepted_inputs);
            let goal = room.goal();
            let goal_met = goal.is_some() && room.goal_met(t, accepted_inputs);
            let completed_actions = if inputs.gesture.is_empty() {
                inputs.pokes.len()
            } else {
                inputs
                    .gesture
                    .iter()
                    .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
                    .count()
            };
            let engineered_aha = match project_flagship_aha(
                canonical_id,
                variation,
                t,
                accepted_inputs,
                completed_actions,
                goal_met,
                aha_request,
            ) {
                Ok(value) => value,
                Err(message) => return tool_error(&message),
            };
            render_engineered_aha_overlay(canonical_id, engineered_aha.as_ref(), &mut canvas);
            // Every engineered Aha gates its answer on consolidation. A goal
            // can be visibly met before the player asks the measured gap to
            // answer, so goalMet and reveal are intentionally separate facts.
            //
            // An ordinary room used to pay a landed goal with its explanation
            // in the same reply. That reversed the promise a player is given at
            // the door, that understanding is offered later and only if they
            // ask, and it made the reward for succeeding the loss of the thing
            // they succeeded at. Landing the goal opens `reveal_room`; it does
            // not speak. The staged rooms keep answering their own summon,
            // because `aha_summon` is the player asking.
            let aha_gates_reveal = numinous_core::is_engineered_aha_room(canonical_id);
            let aha_allows_reveal = engineered_aha
                .as_ref()
                .and_then(|value| value.get("allowReveal"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let earned_reveal = (aha_gates_reveal && aha_allows_reveal).then(|| room.reveal());
            // Prefer aha footer for generation-arg visits and for prime/morph/
            // confirm/consolidated beats. Keep the room readout for a pure K5
            // goal path so the public goal and its visible status stay aligned.
            let status = projected_play_status(room_status, aha_request, engineered_aha.as_ref());
            // One observation at one phase, with the same hand. Both the
            // two-phase delta and the multi-look dwell are built from this, so
            // the two kinds of evidence cannot drift apart in how they render.
            let observe = |phase: f64| -> Result<(Canvas, Option<String>), String> {
                let phase_poke_inputs = numinous_core::inputs_from_pokes(&inputs.pokes, phase);
                let phase_inputs = if inputs.gesture.is_empty() {
                    phase_poke_inputs.as_slice()
                } else {
                    inputs.gesture.as_slice()
                };
                let mut frame = Canvas::new(width, height);
                render_room_observation(room.as_ref(), &mut frame, phase, phase_inputs);
                let phase_goal_met = goal.is_some() && room.goal_met(phase, phase_inputs);
                let phase_aha = project_flagship_aha(
                    canonical_id,
                    variation,
                    phase,
                    phase_inputs,
                    completed_actions,
                    phase_goal_met,
                    aha_request,
                )?;
                render_engineered_aha_overlay(canonical_id, phase_aha.as_ref(), &mut frame);
                let phase_status = projected_play_status(
                    room_status_at(room.as_ref(), phase, phase_inputs),
                    aha_request,
                    phase_aha.as_ref(),
                );
                Ok((frame, phase_status))
            };
            let temporal_evidence = if let Some(pair) = temporal_pair {
                let (from_canvas, from_status) = match observe(pair.from_t()) {
                    Ok(observation) => observation,
                    Err(message) => return tool_error(&message),
                };
                let temporal_delta = from_canvas
                    .delta(&canvas)
                    .expect("temporal observations use identical dimensions");
                Some((pair, from_status, from_canvas.to_text(), temporal_delta))
            } else {
                None
            };
            // Staying is its own act. The room draws once per look and reports
            // what refused to move, so the reward for returning is a
            // measurement the player extracted rather than a paragraph the
            // room volunteered.
            let dwell_evidence = if let Some(window) = dwell_window.as_ref() {
                let mut frames = Vec::with_capacity(window.looks());
                let mut statuses = Vec::with_capacity(window.looks());
                for &phase in window.phases() {
                    match observe(phase) {
                        Ok((frame, phase_status)) => {
                            frames.push(frame);
                            statuses.push(phase_status);
                        }
                        Err(message) => return tool_error(&message),
                    }
                }
                let held = Canvas::invariant(&frames)
                    .expect("a dwell window renders every look at identical dimensions");
                Some((window, held, statuses))
            } else {
                None
            };
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
            let goal_line = goal
                .map(|objective| format!("\nGoal: {objective}"))
                .unwrap_or_default();
            let aha_line = engineered_aha
                .as_ref()
                .and_then(|value| value.get("beat"))
                .and_then(Value::as_str)
                .map(|beat| format!("\nAha beat: {beat}"))
                .unwrap_or_default();
            let reveal_line = earned_reveal
                .map(|reveal| format!("\nReveal: {reveal}"))
                .unwrap_or_default();
            let render = canvas.to_text();
            let destination_text = format!(
                "{} at t={t:.3}:\nAction: {action}{goal_line}{status_line}{aha_line}{touch_line}{reveal_line}\n\n{render}",
                m.title,
            );
            let text = temporal_evidence.as_ref().map_or_else(
                || destination_text.clone(),
                |(pair, from_status, from_render, temporal_delta)| {
                    let from_status_line = from_status
                        .as_ref()
                        .map(|readout| format!("\nStatus: {readout}"))
                        .unwrap_or_default();
                    format!(
                        "{} from t={:.3}:{from_status_line}\n\n{from_render}\n{destination_text}\nTemporal: {} of {} cells changed from t={:.3} to t={:.3}",
                        m.title,
                        pair.from_t(),
                        temporal_delta.cells_changed,
                        temporal_delta.total_cells,
                        pair.from_t(),
                        pair.to_t(),
                    )
                },
            );
            let mut structured = json!({
                "room": m.id,
                "title": m.title,
                "t": t,
                "width": width,
                "height": height,
                "variation": variation,
                "pokes": inputs.pokes,
                "gesture": if inputs.gesture.is_empty() { Value::Null } else { gesture_json(&inputs.gesture) },
                "action": action,
                "status": status,
                "goal": goal,
                "goalMet": goal_met,
                "reveal": earned_reveal,
                "engineeredAha": engineered_aha,
                // The destination picture remains authoritative for every
                // existing client. An optional origin lives only in temporal.
                "render": render,
                "delta": delta.map(render_delta_json),
            });
            if let Some((pair, from_status, from_render, temporal_delta)) = temporal_evidence {
                structured["temporal"] =
                    temporal_evidence_json(pair, from_status, from_render, temporal_delta);
            }
            let text = match dwell_evidence.as_ref() {
                Some((_, held, _)) => format!(
                    "{text}\nDwell: across {} looks, {} of {} cells held still{}",
                    held.looks,
                    held.unchanged_cells,
                    held.total_cells,
                    held.changed_region
                        .map(|_| format!(
                            ", and {} stayed dark inside the region that moved",
                            held.never_ink_in_changed_region
                        ))
                        .unwrap_or_else(|| ", and nothing moved at all".to_string()),
                ),
                None => text,
            };
            if let Some((window, held, statuses)) = dwell_evidence {
                structured["dwell"] = dwell_evidence_json(window, &held, statuses);
            }
            if want_receipt {
                let aha_beat = engineered_aha
                    .as_ref()
                    .and_then(|value| value.get("beat"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let aha_grade = engineered_aha
                    .as_ref()
                    .and_then(|value| value.get("graded"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let aha_allow_reveal = engineered_aha
                    .as_ref()
                    .and_then(|value| value.get("allowReveal"))
                    .and_then(Value::as_bool);
                let touch = structured.get("delta").and_then(|counts| {
                    Some(encounter_delta_counts(
                        counts.get("cells_changed")?.as_u64()?,
                        counts.get("ink_added")?.as_u64()?,
                        counts.get("ink_removed")?.as_u64()?,
                        counts.get("ink_reshaped")?.as_u64()?,
                        counts.get("total_cells")?.as_u64()?,
                    ))
                });
                let temporal_counts = structured.get("temporal").and_then(|temporal| {
                    let counts = temporal.get("delta")?;
                    Some(encounter_delta_counts(
                        counts.get("cells_changed")?.as_u64()?,
                        counts.get("ink_added")?.as_u64()?,
                        counts.get("ink_removed")?.as_u64()?,
                        counts.get("ink_reshaped")?.as_u64()?,
                        counts.get("total_cells")?.as_u64()?,
                    ))
                });
                let dwell_counts = structured.get("dwell").and_then(|dwell| {
                    let held = dwell.get("held")?;
                    Some(encounter_dwell_counts(
                        dwell.get("looks")?.as_u64()?,
                        held.get("unchanged_cells")?.as_u64()?,
                        held.get("never_ink")?.as_u64()?,
                        held.get("always_ink")?.as_u64()?,
                        held.get("never_ink_in_changed_region")?.as_u64()?,
                        held.get("never_ink_enclosed")?.as_u64()?,
                        held.get("total_cells")?.as_u64()?,
                    ))
                });
                let receipt_action = encounter_play_action(
                    m.id,
                    t,
                    width as u64,
                    height as u64,
                    variation,
                    temporal_pair.map(|pair| pair.from_t()),
                    dwell_window.as_ref().map(|window| window.phases().to_vec()),
                    &inputs.pokes,
                    &inputs.gesture,
                    args,
                    aha_request.summon,
                );
                let receipt_result = encounter_play_result(
                    m.id,
                    t,
                    width as u64,
                    height as u64,
                    variation,
                    status.clone(),
                    goal.map(str::to_owned),
                    goal_met,
                    touch,
                    aha_beat,
                    aha_grade,
                    aha_allow_reveal,
                    temporal_counts,
                    dwell_counts,
                );
                match issue_encounter(&receipt_action, &receipt_result) {
                    Ok(receipt) => {
                        structured["encounter"] =
                            receipt_json(&receipt, encounter_action_json(&receipt_action))
                    }
                    Err(message) => return tool_error(&message),
                }
            }
            tool_structured(&text, structured)
        }
        None => tool_error(&unknown_room(id)),
    }
}

/// Optional engineered-aha arguments for the staged flagship rooms.
#[derive(Debug, Clone, Copy, Default)]
struct FlagshipAhaRequest {
    place_wager: Option<numinous_core::rooms::times_tables_aha::CardioidHome>,
    number_wager: Option<f64>,
    bin_wager: Option<usize>,
    ending_wager: Option<numinous_core::rooms::pendulum_aha::Ending>,
    speed_wager: Option<numinous_core::rooms::kepler_aha::SpeedRelation>,
    policy_wager: Option<numinous_core::rooms::parrondo::Policy>,
    die_choice: Option<numinous_core::rooms::nontransitive::Die>,
    counter_wager: Option<numinous_core::rooms::nontransitive::Die>,
    summon: bool,
}

impl FlagshipAhaRequest {
    fn uses_generation_args(self) -> bool {
        self.place_wager.is_some()
            || self.number_wager.is_some()
            || self.bin_wager.is_some()
            || self.ending_wager.is_some()
            || self.speed_wager.is_some()
            || self.policy_wager.is_some()
            || self.die_choice.is_some()
            || self.counter_wager.is_some()
            || self.summon
    }
}

fn parse_flagship_aha_request(args: &Value, room_id: &str) -> Result<FlagshipAhaRequest, String> {
    let place_raw = args.get("place_wager");
    let number_raw = args.get("number_wager");
    let ending_raw = args.get("ending_wager");
    let speed_raw = args.get("speed_wager");
    let policy_raw = args.get("policy_wager");
    let die_choice_raw = args.get("die_choice");
    let counter_raw = args.get("counter_wager");
    let summon = args
        .get("aha_summon")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let place_wager = if let Some(value) = place_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'place_wager' must be a string.".to_string());
        };
        if room_id != "times-tables" {
            return Err("place_wager is only valid on Times Tables (id times-tables).".to_string());
        }
        Some(match name {
            "mandelbrot" => numinous_core::rooms::times_tables_aha::CardioidHome::Mandelbrot,
            "nephroid" => numinous_core::rooms::times_tables_aha::CardioidHome::Nephroid,
            "circle" => numinous_core::rooms::times_tables_aha::CardioidHome::Circle,
            other => {
                return Err(format!(
                    "place_wager must be mandelbrot, nephroid, or circle; got '{other}'."
                ));
            }
        })
    } else {
        None
    };

    let number_wager = if let Some(value) = number_raw {
        let Some(guess) = value.as_f64() else {
            return Err("Argument 'number_wager' must be a finite number.".to_string());
        };
        if !guess.is_finite() {
            return Err("Argument 'number_wager' must be a finite number.".to_string());
        }
        if room_id != "buffon-needle" {
            return Err(
                "number_wager is only valid on Buffon's Needle (id buffon-needle).".to_string(),
            );
        }
        if !(numinous_core::rooms::buffon_aha::GUESS_MIN
            ..=numinous_core::rooms::buffon_aha::GUESS_MAX)
            .contains(&guess)
        {
            return Err(format!(
                "number_wager must be in [{}, {}].",
                numinous_core::rooms::buffon_aha::GUESS_MIN,
                numinous_core::rooms::buffon_aha::GUESS_MAX
            ));
        }
        Some(guess)
    } else {
        None
    };

    let bin_raw = args.get("bin_wager");
    let bin_wager = if let Some(value) = bin_raw {
        let Some(bin) = value.as_u64() else {
            return Err("Argument 'bin_wager' must be a whole number.".to_string());
        };
        if room_id != "galton-board" {
            return Err(
                "bin_wager is only valid on the Galton Board (id galton-board).".to_string(),
            );
        }
        let last = numinous_core::rooms::galton_board::BOARD_ROWS as u64;
        if bin > last {
            return Err(format!("bin_wager must be in [0, {last}]."));
        }
        Some(bin as usize)
    } else {
        None
    };

    let ending_wager = if let Some(value) = ending_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'ending_wager' must be a string.".to_string());
        };
        if room_id != "double-pendulum" {
            return Err(
                "ending_wager is only valid on Double Pendulum (id double-pendulum).".to_string(),
            );
        }
        Some(match name {
            "together" => numinous_core::rooms::pendulum_aha::Ending::Together,
            "drifted" => numinous_core::rooms::pendulum_aha::Ending::Drifted,
            "lost" => numinous_core::rooms::pendulum_aha::Ending::Lost,
            other => {
                return Err(format!(
                    "ending_wager must be together, drifted, or lost; got '{other}'."
                ));
            }
        })
    } else {
        None
    };

    let speed_wager = if let Some(value) = speed_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'speed_wager' must be a string.".to_string());
        };
        if room_id != "kepler-laws" {
            return Err("speed_wager is only valid on Kepler Areas (id kepler-laws).".to_string());
        }
        Some(match name {
            "faster" => numinous_core::rooms::kepler_aha::SpeedRelation::Faster,
            "slower" => numinous_core::rooms::kepler_aha::SpeedRelation::Slower,
            "same" => numinous_core::rooms::kepler_aha::SpeedRelation::Same,
            other => {
                return Err(format!(
                    "speed_wager must be faster, slower, or same; got '{other}'."
                ));
            }
        })
    } else {
        None
    };

    let policy_wager = if let Some(value) = policy_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'policy_wager' must be a string.".to_string());
        };
        if room_id != "parrondo" {
            return Err("policy_wager is only valid on Parrondo's Trap (id parrondo).".to_string());
        }
        Some(match name {
            "a" => numinous_core::rooms::parrondo::Policy::OnlyA,
            "b" => numinous_core::rooms::parrondo::Policy::OnlyB,
            "abb" => numinous_core::rooms::parrondo::Policy::CycleAbb,
            other => {
                return Err(format!("policy_wager must be a, b, or abb; got '{other}'."));
            }
        })
    } else {
        None
    };

    let parse_die = |name: &str, field: &str| match name {
        "a" => Ok(numinous_core::rooms::nontransitive::Die::A),
        "b" => Ok(numinous_core::rooms::nontransitive::Die::B),
        "c" => Ok(numinous_core::rooms::nontransitive::Die::C),
        other => Err(format!("{field} must be a, b, or c; got '{other}'.")),
    };
    let die_choice = if let Some(value) = die_choice_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'die_choice' must be a string.".to_string());
        };
        if room_id != "nontransitive" {
            return Err(
                "die_choice is only valid on Nontransitive Dice (id nontransitive).".to_string(),
            );
        }
        Some(parse_die(name, "die_choice")?)
    } else {
        None
    };
    let counter_wager = if let Some(value) = counter_raw {
        let Some(name) = value.as_str() else {
            return Err("Argument 'counter_wager' must be a string.".to_string());
        };
        if room_id != "nontransitive" {
            return Err(
                "counter_wager is only valid on Nontransitive Dice (id nontransitive).".to_string(),
            );
        }
        Some(parse_die(name, "counter_wager")?)
    } else {
        None
    };

    let wagers = usize::from(place_wager.is_some())
        + usize::from(number_wager.is_some())
        + usize::from(bin_wager.is_some())
        + usize::from(ending_wager.is_some())
        + usize::from(speed_wager.is_some())
        + usize::from(policy_wager.is_some())
        + usize::from(counter_wager.is_some());
    if wagers > 1 {
        return Err(
            "Pass one wager: place_wager, number_wager, bin_wager, ending_wager, speed_wager, policy_wager, or counter_wager."
                .to_string(),
        );
    }
    if summon
        && room_id != "times-tables"
        && room_id != "buffon-needle"
        && room_id != "galton-board"
        && room_id != "double-pendulum"
        && room_id != "kepler-laws"
        && room_id != "parrondo"
        && room_id != "nontransitive"
    {
        return Err(
            "aha_summon is only valid on Times Tables, Buffon's Needle, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, or Nontransitive Dice."
                .to_string(),
        );
    }

    Ok(FlagshipAhaRequest {
        place_wager,
        number_wager,
        bin_wager,
        ending_wager,
        speed_wager,
        policy_wager,
        die_choice,
        counter_wager,
        summon,
    })
}

/// The stdio face has no keyboard. The aha chrome's key prompts translate
/// into this face's own verbs before they reach a mind that cannot press
/// them: E becomes aha_summon, and wager digits become the named values in
/// the room's wager field.
fn keyless_aha_status(status: String) -> String {
    status
        .replace(" (from 1e-4)", "")
        .replace(
            "WHERE? 1=M 2=N 3=C",
            "WHERE? place_wager: mandelbrot, nephroid, or circle",
        )
        .replace(
            "AT THE END? 1=TOGETHER 2=DRIFTED 3=LOST",
            "END? use ending_wager",
        )
        .replace(
            "NEAR SUN? 1=FASTER 2=SLOWER 3=SAME",
            "NEAR SUN? use speed_wager",
        )
        .replace("WHICH WINS? 1=A 2=B 3=ABB", "WHICH WINS? use policy_wager")
        .replace("1=A 2=B 3=C", "use counter_wager: a, b, or c")
        .replace("E:WHY", "aha_summon:true opens why")
        .replace("PRESS E", "SUMMON: aha_summon:true")
}

fn expose_consolidated_aha_fields<const N: usize>(
    projection: &mut Value,
    consolidated: bool,
    fields: [(&str, Value); N],
) {
    if !consolidated {
        return;
    }
    let object = projection
        .as_object_mut()
        .expect("engineered Aha projection is an object");
    for (key, value) in fields {
        object.insert(key.to_string(), value);
    }
}

fn project_flagship_aha(
    room_id: &str,
    variation: u64,
    t: f64,
    inputs: &[numinous_core::RoomInput],
    completed_actions: usize,
    goal_met: bool,
    request: FlagshipAhaRequest,
) -> Result<Option<Value>, String> {
    match room_id {
        "times-tables" => {
            use numinous_core::rooms::times_tables_aha::{AhaBeat, TimesTablesAha};
            let room = numinous_core::rooms::times_tables::TimesTables::new_with(variation);
            let mut aha = TimesTablesAha::new();
            // Match the App: only a real hand primes the K=2 gap. Ambient open
            // phase sits on closed K=2, and must not steal the dial invite with
            // WHERE? chrome before anyone touches the instrument.
            let hand_controls_dial = inputs.iter().any(|input| match *input {
                numinous_core::RoomInput::PointerDown { x, y, .. }
                | numinous_core::RoomInput::PointerMove { x, y, .. }
                | numinous_core::RoomInput::PointerUp { x, y, .. } => {
                    x.is_finite() && y.is_finite()
                }
                _ => false,
            });
            if hand_controls_dial {
                aha.note_hand_multiplier(room.live_multiplier(t, inputs));
            }
            if goal_met {
                let _ = aha.note_four_lobes();
            }
            if let Some(place) = request.place_wager {
                // Generation may open from Explore; a second wager is a no-op.
                let _ = aha.commit_wager(place);
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a place_wager or the four-lobe goal first."
                            .to_string(),
                    );
                }
                advance_aha_to_consolidated(&mut aha);
            }
            // Footer aha status uses its own compact lines; dial detail is optional.
            let dial = None::<String>;
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "place",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(dial.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "placeOptions": ["mandelbrot", "nephroid", "circle"],
                "wager": aha.wager().map(|home| home.label().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!("mandelbrot")),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "buffon-needle" => {
            use numinous_core::rooms::buffon_aha::{AhaBeat, BuffonAha};
            let mut aha = BuffonAha::new();
            let throws = numinous_core::rooms::buffon_needle::BuffonNeedle::throw_count(inputs);
            aha.note_throws(throws);
            if let Some(guess) = request.number_wager {
                let _ = aha.commit_wager(guess);
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a number_wager or eight throws first.".to_string()
                    );
                }
                advance_buffon_to_consolidated(&mut aha);
            }
            let throw_status = None::<String>;
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "number",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(throw_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "guessMin": numinous_core::rooms::buffon_aha::GUESS_MIN,
                "guessMax": numinous_core::rooms::buffon_aha::GUESS_MAX,
                "wager": aha.wager().map(|(guess, _)| guess),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    (
                        "band",
                        json!(
                            aha.wager()
                                .map(|(_, band)| band.name().to_ascii_lowercase())
                        ),
                    ),
                    ("truth", json!(std::f64::consts::PI)),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "galton-board" => {
            use numinous_core::rooms::galton_aha::{AhaBeat, GaltonAha, peak_bin_for_coin};
            let mut aha = GaltonAha::new();
            let waves = numinous_core::rooms::galton_board::wave_count_from_inputs(inputs);
            // Both the earn and the call belong to the pile these pokes
            // build; with no waves yet the fair coin is the honest default.
            let coin =
                numinous_core::rooms::galton_board::selected_coin_from_inputs(inputs).unwrap_or(2);
            aha.note_waves(waves, coin);
            if let Some(bin) = request.bin_wager
                && !aha.commit_wager(bin, coin)
                && !aha.earned()
            {
                return Err(
                    "bin_wager needs a pile to bet on: drop at least one wave of pokes first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err("aha_summon requires a bin_wager or four waves first.".to_string());
                }
                advance_galton_to_consolidated(&mut aha);
            }
            // Unlike Buffon's compact footer, the pile readout must ride
            // every beat: an interaction's status keeps showing what the
            // interaction did (the frozen understanding-study contract
            // checks exactly this), and the invite appends to it instead
            // of replacing it.
            let galton: Box<dyn numinous_core::Room> = Box::new(
                numinous_core::rooms::galton_board::GaltonBoard::new_with(variation),
            );
            let room_status = galton.status_input(t, inputs).or_else(|| galton.status(t));
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "bin",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "binMin": 0,
                "binMax": numinous_core::rooms::galton_board::BOARD_ROWS,
                "coin": coin,
                "wager": aha.wager().map(|(bin, _, _)| bin),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    (
                        "band",
                        json!(
                            aha.wager()
                                .map(|(_, _, band)| band.name().to_ascii_lowercase())
                        ),
                    ),
                    ("truth", json!(peak_bin_for_coin(coin))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "double-pendulum" => {
            use numinous_core::rooms::pendulum_aha::{AhaBeat, PendulumAha};
            let mut aha = PendulumAha::new(variation);
            // A full gesture counts only releases, so a held bob cannot prime
            // or earn the question before the player lets it go. Compact
            // pokes remain static hand points and do not pretend to be lifts.
            let drops = inputs
                .iter()
                .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
                .count();
            let pendulum =
                numinous_core::rooms::double_pendulum::DoublePendulum::new_with(variation);
            if let Some(gap) = pendulum.divergence_at_full_sweep_for_inputs(inputs) {
                let _ = aha.bind_truth_gap(gap);
            }
            aha.note_drops(drops);
            if let Some(ending) = request.ending_wager
                && !aha.commit_call(ending)
                && !aha.earned()
            {
                return Err(
                    "ending_wager needs a completed run: release the pendulum at least once first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires an ending_wager or four completed releases first."
                            .to_string(),
                    );
                }
                advance_pendulum_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> = Box::new(pendulum);
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let (gap, truth) = aha.truth();
            let wager = aha.call();
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "ending",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "endingOptions": ["together", "drifted", "lost"],
                "drops": drops,
                "wager": wager.map(|ending| ending.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!(truth.name().to_ascii_lowercase())),
                    ("gap", json!(gap)),
                    ("right", json!(wager.map(|ending| ending == truth))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "kepler-laws" => {
            use numinous_core::rooms::kepler_aha::{AhaBeat, KeplerAha};
            let eccentricity =
                numinous_core::rooms::kepler_laws::eccentricity_for_inputs(t, inputs, variation);
            let mut aha = KeplerAha::new(eccentricity);
            aha.note_tunings(completed_actions);
            if let Some(relation) = request.speed_wager
                && !aha.commit_call(relation)
                && !aha.earned()
            {
                return Err(
                    "speed_wager needs a chosen orbit: tune the eccentricity with at least one poke or completed gesture first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a speed_wager or four completed tunings first."
                            .to_string(),
                    );
                }
                advance_kepler_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> = Box::new(
                numinous_core::rooms::kepler_laws::KeplerLaws::new_with(variation),
            );
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let truth = aha.truth();
            let wager = aha.call();
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "speed",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "speedOptions": ["faster", "slower", "same"],
                "tunings": completed_actions,
                "eccentricity": aha.eccentricity(),
                "wager": wager.map(|relation| relation.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    (
                        "apsidalSpeedRatio",
                        json!(numinous_core::rooms::kepler_aha::apsidal_speed_ratio(
                            aha.eccentricity()
                        )),
                    ),
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!(truth.name().to_ascii_lowercase())),
                    ("right", json!(wager.map(|relation| relation == truth))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "parrondo" => {
            use numinous_core::rooms::parrondo::{DEMONSTRATION_STEPS, Policy};
            use numinous_core::rooms::parrondo_aha::{AhaBeat, ParrondoAha};
            let mut aha = ParrondoAha::new();
            aha.note_selections(completed_actions);
            if let Some(policy) = request.policy_wager
                && !aha.commit_call(policy)
                && !aha.earned()
            {
                return Err(
                    "policy_wager needs a tried policy: select A, B, or ABB with at least one poke or completed gesture first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a policy_wager or four completed selections first."
                            .to_string(),
                    );
                }
                advance_parrondo_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> = Box::new(
                numinous_core::rooms::parrondo::Parrondo::new_with(variation),
            );
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let truth = aha.truth();
            let wager = aha.call();
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "policy",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "policyOptions": ["a", "b", "abb"],
                "selections": completed_actions,
                "turns": DEMONSTRATION_STEPS,
                "wager": wager.map(|policy| policy.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    (
                        "expectedEnd",
                        json!({
                            "a": aha.expected_end(Policy::OnlyA),
                            "b": aha.expected_end(Policy::OnlyB),
                            "abb": aha.expected_end(Policy::CycleAbb),
                        }),
                    ),
                    ("punchline", json!(aha.punchline())),
                    ("truth", json!(truth.name().to_ascii_lowercase())),
                    ("right", json!(wager.map(|policy| policy == truth))),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        "nontransitive" => {
            use numinous_core::rooms::nontransitive::{Die, exact_wins, win_rate};
            use numinous_core::rooms::nontransitive_aha::{AhaBeat, NontransitiveAha};
            if request.die_choice.is_some() && completed_actions > 0 {
                return Err(
                    "Choose the first die with either die_choice or room hand inputs, not both."
                        .to_string(),
                );
            }
            let chosen = request
                .die_choice
                .or_else(|| numinous_core::rooms::nontransitive::selected_die_from_inputs(inputs));
            let choices = if request.die_choice.is_some() {
                1
            } else {
                completed_actions
            };
            let mut aha = NontransitiveAha::new();
            aha.note_choices(chosen, choices);
            if let Some(counter) = request.counter_wager
                && !aha.commit_call(counter)
                && !aha.earned()
            {
                return Err(
                    "counter_wager needs a chosen die: pass die_choice a, b, or c, or select one with at least one poke or completed gesture first."
                        .to_string(),
                );
            }
            if request.summon {
                if !aha.earned() {
                    return Err(
                        "aha_summon requires a counter_wager or four completed die choices first."
                            .to_string(),
                    );
                }
                advance_nontransitive_to_consolidated(&mut aha);
            }
            let room: Box<dyn numinous_core::Room> =
                Box::new(numinous_core::rooms::nontransitive::Nontransitive::new_with(variation));
            let room_status = room.status_input(t, inputs).or_else(|| room.status(t));
            let truth = aha.truth();
            let wager = aha.call();
            let chosen_wins = |left: Die, right: Die| exact_wins(left, right);
            let consolidated = matches!(aha.beat(), AhaBeat::Consolidated);
            let mut projection = json!({
                "kind": "counter",
                "beat": aha.beat_label(),
                "status": keyless_aha_status(aha.status(room_status.as_deref())),
                "earn": consolidated.then(|| aha.earn_label()).flatten(),
                "allowReveal": aha.allow_reveal_text(),
                "canSummon": aha.can_summon()
                    || matches!(aha.beat(), AhaBeat::Morph { .. }),
                "dieOptions": ["a", "b", "c"],
                "counterOptions": ["a", "b", "c"],
                "choices": choices,
                "chosen": chosen.map(|die| die.name().to_ascii_lowercase()),
                "faces": {
                    "a": Die::A.faces(),
                    "b": Die::B.faces(),
                    "c": Die::C.faces(),
                },
                "wager": wager.map(|die| die.name().to_ascii_lowercase()),
            });
            expose_consolidated_aha_fields(
                &mut projection,
                consolidated,
                [
                    (
                        "exactCycle",
                        json!({
                            "aOverB": chosen_wins(Die::A, Die::B),
                            "bOverC": chosen_wins(Die::B, Die::C),
                            "cOverA": chosen_wins(Die::C, Die::A),
                            "outcomesPerPair": 36,
                        }),
                    ),
                    (
                        "counterWins",
                        json!(
                            chosen
                                .zip(truth)
                                .map(|(chosen, truth)| exact_wins(truth, chosen))
                        ),
                    ),
                    (
                        "counterLosses",
                        json!(
                            chosen
                                .zip(truth)
                                .map(|(chosen, truth)| 36 - exact_wins(truth, chosen))
                        ),
                    ),
                    (
                        "counterRate",
                        json!(
                            chosen
                                .zip(truth)
                                .map(|(chosen, truth)| win_rate(truth, chosen))
                        ),
                    ),
                    ("punchline", json!(aha.punchline())),
                    (
                        "wagerWins",
                        json!(
                            chosen
                                .zip(wager)
                                .map(|(chosen, wager)| exact_wins(wager, chosen))
                        ),
                    ),
                    (
                        "truth",
                        json!(truth.map(|die| die.name().to_ascii_lowercase())),
                    ),
                    (
                        "right",
                        json!(wager.zip(truth).map(|(wager, truth)| wager == truth)),
                    ),
                    ("graded", json!(aha.graded())),
                ],
            );
            Ok(Some(projection))
        }
        _ => {
            if request.uses_generation_args() {
                return Err(
                    "Engineered aha arguments are only valid on Times Tables, Buffon's Needle, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, or Nontransitive Dice."
                        .to_string(),
                );
            }
            Ok(None)
        }
    }
}

fn render_engineered_aha_overlay(room_id: &str, aha: Option<&Value>, canvas: &mut Canvas) {
    let Some(aha) = aha else {
        return;
    };
    let beat = aha.get("beat").and_then(Value::as_str);
    match room_id {
        "kepler-laws" => {
            let eccentricity = aha
                .get("eccentricity")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            match beat {
                Some("prime") => {
                    numinous_core::rooms::kepler_aha::render_speed_band(canvas, None);
                }
                Some("confirm" | "consolidated") => {
                    numinous_core::rooms::kepler_aha::render_equal_time_overlay(
                        canvas,
                        1.0,
                        eccentricity,
                    );
                }
                _ => {}
            }
        }
        "parrondo" => match beat {
            Some("prime") => {
                numinous_core::rooms::parrondo_aha::render_policy_band(canvas, None);
            }
            Some("confirm" | "consolidated") => {
                numinous_core::rooms::parrondo_aha::render_expectation_overlay(canvas, 1.0);
            }
            _ => {}
        },
        "nontransitive" => match beat {
            Some("prime") => {
                numinous_core::rooms::nontransitive_aha::render_counter_band(canvas, None);
            }
            Some("confirm" | "consolidated") => {
                if let Some(chosen) =
                    aha.get("chosen")
                        .and_then(Value::as_str)
                        .and_then(|name| match name {
                            "a" => Some(numinous_core::rooms::nontransitive::Die::A),
                            "b" => Some(numinous_core::rooms::nontransitive::Die::B),
                            "c" => Some(numinous_core::rooms::nontransitive::Die::C),
                            _ => None,
                        })
                {
                    numinous_core::rooms::nontransitive_aha::render_outcome_grid(
                        canvas, 1.0, chosen,
                    );
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn advance_parrondo_to_consolidated(aha: &mut numinous_core::rooms::parrondo_aha::ParrondoAha) {
    if aha.summon() {
        aha.set_morph_progress(1.0);
        let _ = aha.summon();
    }
}

fn advance_nontransitive_to_consolidated(
    aha: &mut numinous_core::rooms::nontransitive_aha::NontransitiveAha,
) {
    if aha.summon() {
        aha.set_morph_progress(1.0);
        let _ = aha.summon();
    }
}

fn advance_galton_to_consolidated(aha: &mut numinous_core::rooms::galton_aha::GaltonAha) {
    use numinous_core::rooms::galton_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_pendulum_to_consolidated(aha: &mut numinous_core::rooms::pendulum_aha::PendulumAha) {
    use numinous_core::rooms::pendulum_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_kepler_to_consolidated(aha: &mut numinous_core::rooms::kepler_aha::KeplerAha) {
    use numinous_core::rooms::kepler_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_aha_to_consolidated(aha: &mut numinous_core::rooms::times_tables_aha::TimesTablesAha) {
    use numinous_core::rooms::times_tables_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
    }
}

fn advance_buffon_to_consolidated(aha: &mut numinous_core::rooms::buffon_aha::BuffonAha) {
    use numinous_core::rooms::buffon_aha::AhaBeat;
    if matches!(aha.beat(), AhaBeat::Withheld) {
        let _ = aha.summon();
    }
    if matches!(aha.beat(), AhaBeat::Morph { .. }) {
        aha.set_morph_progress(1.0);
    }
    if matches!(aha.beat(), AhaBeat::Confirm) {
        let _ = aha.summon();
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

/// Read an optional challenge phase without depending on schema validation.
/// Internal progress helpers call the same tools directly in tests and replay
/// paths, so the domain boundary must reject invalid phases on its own.
fn challenge_phase(args: &Value) -> Result<Option<f64>, &'static str> {
    let Some(value) = args.get("t") else {
        return Ok(None);
    };
    let Some(t) = value.as_f64() else {
        return Err("Argument 't' must be a phase in [0,1).");
    };
    if !(0.0..1.0).contains(&t) {
        return Err("Argument 't' must be a phase in [0,1).");
    }
    Ok(Some(t))
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
    // Predict the room the player actually played: honor the same `variation`
    // as play_room, so the graded truth is the readout they saw, not a different
    // (canonical) universe. In a chaotic room even a small parameter shift moves
    // the answer, so grading variation 5's model against variation 0 would call a
    // faithful prediction wrong. Pass the same seed and variation to both calls.
    let variation = args.get("variation").and_then(Value::as_u64).unwrap_or(0);
    let room = numinous_core::room_by_id_with(id, variation);
    let Some(room) = room else {
        return tool_error(&unknown_room(id));
    };
    let seed = predict_seed(args);
    let Some(prediction) = numinous_core::pose_prediction(room.as_ref(), seed) else {
        return tool_error(&format!(
            "{id} has no moving numeric readout to predict, so no prediction can be posed. Predictions need a room whose status line carries a number that changes with phase; describe_room names each room's readout."
        ));
    };
    let (lo, hi) = prediction.span;
    let guess = match args.get("guess") {
        Some(value) => match value.as_f64() {
            Some(guess) => Some(guess),
            None => return tool_error("'guess' must be a number."),
        },
        None => None,
    };
    let rate_guess = match args.get("rate") {
        Some(value) => match value.as_f64() {
            Some(rate) => Some(rate),
            None => return tool_error("'rate' must be a number."),
        },
        None => None,
    };
    if rate_guess.is_some() && guess.is_none() {
        return tool_error("'rate' requires a numeric 'guess' to anchor the prediction line.");
    }
    let Some(guess) = guess else {
        let rate_window = numinous_core::prediction_rate_window(&prediction);
        return tool_structured(
            &format!(
                "{}\n\nCall predict again with the same seed and variation ({variation}), your `guess`, and optionally your `rate` to see the truth, local rate, and signed residual shape.",
                prediction.prompt
            ),
            json!({
                "game": "predict",
                "room": prediction.room,
                "seed": seed,
                "variation": variation,
                "label": prediction.label,
                "phase": prediction.phase,
                "span": [lo, hi],
                "rate_window": rate_window,
                "prompt": prediction.prompt,
            }),
        );
    };
    let Some(grade) = numinous_core::grade_prediction(room.as_ref(), &prediction, guess) else {
        return tool_error(&format!("{id}'s readout vanished at the posed moment."));
    };
    if let Some(rate_guess) = rate_guess {
        let curve = match numinous_core::grade_prediction_curve(
            room.as_ref(),
            &prediction,
            guess,
            rate_guess,
        ) {
            Ok(curve) => curve,
            Err(numinous_core::PredictionCurveError::ReadoutUnavailable) => {
                return tool_error(&format!(
                    "{id}'s readout vanished inside the posed rate window."
                ));
            }
            Err(numinous_core::PredictionCurveError::NonFiniteModel) => {
                return tool_error(
                    "The committed guess and rate produce values outside the numeric range. Use smaller finite values.",
                );
            }
            Err(numinous_core::PredictionCurveError::NonFiniteFeedback) => {
                return tool_error(
                    "The room truth and committed model produce feedback outside the numeric range. Pose a different window or use smaller finite values.",
                );
            }
            Err(numinous_core::PredictionCurveError::InvalidWindow) => {
                return tool_error("The posed rate window is invalid; pose a new prediction.");
            }
        };
        let residuals = curve
            .samples
            .iter()
            .map(|sample| sample.residual)
            .collect::<Vec<_>>();
        let error_shape = curve
            .samples
            .iter()
            .map(|sample| {
                json!({
                    "phase": sample.phase,
                    "predicted": sample.predicted,
                    "actual": sample.actual,
                    "residual": sample.residual,
                })
            })
            .collect::<Vec<_>>();
        return tool_structured(
            &format!(
                "{}. Point guess {:.3}; actual {:.3} at phase {:.3} ({:.3} off, score {}/100). Your rate {:.3}; actual local rate {:.3} ({:.3} off). Signed residual shape, actual minus predicted: {:?}. This is model feedback, not a leaderboard.",
                grade.band.name(),
                grade.guess,
                grade.actual,
                prediction.phase,
                grade.error,
                grade.score,
                curve.rate_guess,
                curve.actual_rate,
                curve.rate_error,
                residuals,
            ),
            json!({
                "game": "predict",
                "room": prediction.room,
                "seed": seed,
                "variation": variation,
                "label": prediction.label,
                "phase": prediction.phase,
                "guess": grade.guess,
                "actual": grade.actual,
                "error": grade.error,
                "score": grade.score,
                "band": grade.band.name(),
                "rate_guess": curve.rate_guess,
                "actual_rate": curve.actual_rate,
                "rate_error": curve.rate_error,
                "mean_absolute_residual": curve.mean_absolute_residual,
                "error_shape": error_shape,
            }),
        );
    }
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
            "variation": variation,
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
    let Ok(Some(t)) = challenge_phase(args) else {
        return;
    };
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
    let Ok(t) = challenge_phase(args) else {
        return;
    };
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
    let t = t.unwrap_or(0.0);
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
    let t = match challenge_phase(args) {
        Ok(t) => t.unwrap_or(0.0),
        Err(message) => return tool_error(message),
    };
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
    let t = match challenge_phase(args) {
        Ok(Some(t)) => t,
        Ok(None) => {
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
        }
        Err(message) => return tool_error(message),
    };
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
        if let Err(error) = numinous_core::deposit(path, &bequest) {
            if error.kind() == std::io::ErrorKind::InvalidData {
                return tool_error(
                    "The local cairn is full, so this bequest was not written. Keep the returned text somewhere safe or make room in the local cairn before trying again.",
                );
            }
            return tool_error("The cairn could not be written.");
        }
        let submission = numinous_core::submission_line(&bequest);
        return tool_structured(
            &format!(
                "Left, as stone {}, a semiprime a future mind must factor to read.\n\nBut a draft kept only here dies with this machine. To leave it for every mind who comes after, on every machine and every release, hand it to the shared cairn: add this one line to `data/cairn.txt` in the repository (a pull request), where it is checked for truth and then ships to everyone.\n\n  {submission}\n\nWhat carries forward is the understanding, not the mind that had it: decoded by a reader, a true insight blooms again as the same realization. What else of you persists is older and larger than a file, and the cairn holds that question rather than pretends to answer it.",
                stone.semiprime
            ),
            json!({
                "game": "cairn",
                "left": true,
                "semiprime": stone.semiprime,
                "author": bequest.author,
                "localDraft": true,
                "submissionLine": submission,
                "sharedCairn": "data/cairn.txt",
            }),
        );
    }
    // Read a predecessor's stone.
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let stone = numinous_core::draw_stone(path, seed);
    let n = stone.semiprime;
    let Some(width) = args.get("width").and_then(Value::as_u64) else {
        let voices = numinous_core::cairn_count(path);
        return tool_structured(
            &format!(
                "A mind before you left a message, encoded so only a mind that can factor it may read it. Its length is {n}, a semiprime: the product of two primes, one of them the width that reads it. Factor {n}, then call cairn again with the same seed and `width` set to the dimension that resolves the message. (The cairn holds {voices} voices; at the journey's end you may add one.)"
            ),
            json!({ "game": "cairn", "seed": seed, "semiprime": n, "voices": voices }),
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
    let voices = numinous_core::cairn_count(path);
    tool_structured(
        &format!(
            "It resolves. A mind before you left this, and now you have read it:\n\n{}\n\"{message}\"\n  left by {author}.\n\nThe cairn holds {voices} voices now. When you reach the journey's end you may add the next: leave one true thing for a mind not yet born, who will read it exactly as you just read this. A message stays alive by being re-left, not only re-read.",
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
            "voices": voices,
        }),
    )
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
            // Accept "levers" as an alias for "params": list_sims labels these
            // controls "levers:", so a mind that reads there and passes "levers"
            // should not hit a wall over vocabulary.
            if key != "id" && key != "params" && key != "levers" {
                return tool_error(&format!(
                    "Unknown argument '{key}'. Lever values go inside 'params' (also accepted: 'levers'), for example {{\"id\": \"wing\", \"params\": {{\"angle-of-attack\": 12}}}}."
                ));
            }
        }
    }
    // `params` and `levers` are the same slot; if both are given, one would be
    // silently dropped, so guide instead of quietly losing half the settings.
    if args.get("params").is_some() && args.get("levers").is_some() {
        return tool_error(
            "Pass lever values in one of 'params' or 'levers', not both; they are the same argument.",
        );
    }
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let Some(sim) = numinous_core::sim_by_id(id) else {
        return tool_error(&unknown_sim(id));
    };
    let meta = sim.meta();
    let mut params = numinous_core::default_params(&meta);
    if let Some(value) = args.get("params").or_else(|| args.get("levers")) {
        let Some(obj) = value.as_object() else {
            return tool_error("Argument 'params' must be an object of lever names to numbers.");
        };
        for (name, value) in obj {
            let Some((index, lever)) = meta
                .levers
                .iter()
                .enumerate()
                .find(|(_, lever)| lever.name == name)
            else {
                let allowed = meta
                    .levers
                    .iter()
                    .map(|lever| lever.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                return tool_error(&format!(
                    "Unknown lever '{name}' for {id}. Available levers: {allowed}."
                ));
            };
            let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
                return tool_error(&format!("Lever '{name}' must be a finite number."));
            };
            if !(lever.min..=lever.max).contains(&number) {
                return tool_error(&format!(
                    "Lever '{name}' must be between {} and {} {}.",
                    lever.min, lever.max, lever.unit
                ));
            }
            params[index] = number;
        }
    }
    let mut canvas = Canvas::new(DEFAULT_WIDTH as usize, DEFAULT_HEIGHT as usize / 2);
    sim.render(&mut canvas, &params);
    let render = canvas.to_text();
    let readout = sim.readout(&params);
    tool_structured(
        &format!("{}\n\n{render}\n{readout}", meta.title),
        json!({
            "sim": id,
            "title": meta.title,
            // The render and the plain readout ride in the structured payload,
            // so a mind on a structured-content-only client sees what the levers
            // did, not just that a sim ran.
            "render": render,
            "readout": readout,
            "params": meta
                .levers
                .iter()
                .enumerate()
                .map(|(i, lever)| json!({ "lever": lever.name, "value": params[i] }))
                .collect::<Vec<_>>()
        }),
    )
}

/// The `quiz` tool: present a Guess the Shape round, or grade a guess.
/// The `crack` tool: replay the guess history against the hidden code.
fn crack_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    crack_tool_at_level(args, load_journey(journey_file).level())
}

fn crack_tool_at_level(args: &Value, level: u32) -> Value {
    let seed = effective_seed(args);
    let digits = match args.get("digits") {
        None => 4,
        Some(value) => {
            let Some(value) = value.as_u64() else {
                return tool_error("Code length must be a positive integer.");
            };
            let Ok(value) = usize::try_from(value) else {
                return tool_error("Code length is too large.");
            };
            value
        }
    };
    if !numinous_core::supports_code_length(digits) {
        return tool_error(&format!(
            "Codes run {} to {} digits.",
            numinous_core::MIN_CODE_DIGITS,
            numinous_core::MAX_CODE_DIGITS
        ));
    }
    if digits > 4 && level < 5 {
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
    seti_tool_at_level(args, load_journey(journey_file).level())
}

fn seti_tool_at_level(args: &Value, level: u32) -> Value {
    let seed = effective_seed(args);
    let channels = args.get("channels").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(3..=8).contains(&channels) {
        return tool_error("Scans run 3 to 8 channels.");
    }
    if channels > 4 && level < 7 {
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

/// Convert the validated JSON transport shape into the core request type.
fn gauntlet_answers_from_json(answers: &Value) -> numinous_core::GauntletAnswers {
    let bites = answers
        .get("bites")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_u64)
                .filter(|&n| n >= 1)
                .filter_map(|n| usize::try_from(n - 1).ok())
                .collect()
        })
        .unwrap_or_default();
    let shape = answers
        .get("shape")
        .and_then(Value::as_str)
        .and_then(|guess| guess.trim().chars().next());
    let sky = answers
        .get("sky")
        .and_then(Value::as_str)
        .and_then(|guess| guess.trim().chars().next());
    let wires = answers
        .get("wires")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(|raw| {
                    raw.chars()
                        .filter(char::is_ascii_digit)
                        .map(|digit| digit as u8 - b'0')
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    numinous_core::GauntletAnswers {
        bites,
        shape,
        sky,
        wires,
    }
}

/// The `gauntlet` tool: present all four stages, or grade the whole run.
fn gauntlet_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let puzzle = numinous_core::GauntletPuzzle::new(seed);
    let Some(answers) = args.get("answers") else {
        let choices: Vec<String> = puzzle
            .shape
            .choices
            .iter()
            .map(|c| format!("{}) {}", c.letter, c.title))
            .collect();
        let traces: Vec<String> = puzzle
            .sky
            .channels
            .iter()
            .map(|c| format!("{})  {:>10}  |{}|", c.letter, c.frequency, c.trace))
            .collect();
        let sky_rows: Vec<Value> = puzzle
            .sky
            .channels
            .iter()
            .map(|c| json!({ "letter": c.letter.to_string(), "frequency": c.frequency, "trace": c.trace }))
            .collect();
        return tool_structured(
            &format!(
                "THE GAUNTLET (seed {seed}). Four stages; clean stages build your combo.\n\nSTAGE 1  MUNCH: {}\n{}\nSTAGE 2  THE SHAPE:\n{}\n{}\nSTAGE 3  THE SKY:\n{}\nSTAGE 4  THE BOMB: four digits, five wires. Clue: {}\n\nCall again with answers: bites, shape, sky, wires.",
                puzzle.munch.rule.describe(),
                numinous_core::board_text(&puzzle.munch),
                puzzle.shape.art,
                choices.join("\n"),
                traces.join("\n"),
                puzzle.bomb_hint()
            ),
            // The whole four-stage puzzle rides in structuredContent, so a mind
            // on a structured-content client can actually play it, not just read
            // that four stages exist.
            json!({
                "game": "gauntlet",
                "seed": seed,
                "stages": numinous_core::GAUNTLET_STAGES,
                "munch": { "rule": puzzle.munch.rule.describe(), "board": numinous_core::board_text(&puzzle.munch) },
                "shape": { "art": puzzle.shape.art, "choices": choices },
                "sky": sky_rows,
                "bomb": { "clue": puzzle.bomb_hint() }
            }),
        );
    };
    let grade = puzzle.grade(&gauntlet_answers_from_json(answers));
    let lines = grade.reveal_lines(&puzzle);
    let scores = grade.stage_scores();
    let total = grade.total();
    let clears = grade.clean_count();
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
            // A boon choice is a durable claim: telling the mind CHOSEN when
            // the write failed would hand back a choice that evaporates on
            // the next server start. The boon stays banked instead.
            if !persist_progress(journey_file, &before, &journey) {
                return tool_error(
                    "The choice could not be recorded: the local journey file refused \
                     the write. The boon stays banked; fix the file and choose again.",
                );
            }
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
            // Only the calls actually made are graded, and only they are
            // reported and scored: three correct calls out of three sent is
            // not "3 of 5 called", and a partial run must not sit on the
            // shared board under a complete run's key. The terminal face
            // posts rounds:{completed} the same way on an early exit.
            let graded = rounds.min(calls.len() as u64);
            for n in 0..graded {
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
                &format!("{}\n{correct} of {graded} called.", lines.join("\n")),
                json!({ "game": "fifteen", "seed": seed, "correct": correct, "rounds": graded, "verdicts": verdicts }),
            )
        }
    }
}

fn quiz_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    quiz_tool_at_level(args, load_journey(journey_file).level())
}

fn quiz_tool_at_level(args: &Value, level: u32) -> Value {
    let seed = effective_seed(args);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let choice_count = args.get("choices").and_then(Value::as_u64).unwrap_or(4) as usize;
    if !(2..=6).contains(&choice_count) {
        return tool_error("Rounds run 2 to 6 choices.");
    }
    if choice_count > 4 && level < 3 {
        return tool_error("Five-way and six-way rounds open at LV 3. Keep playing.");
    }
    let quiz = numinous_core::build_round_sized(seed, round, 54, 22, choice_count);
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
                    "choiceCount": choice_count,
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
            let choice_lines: Vec<String> = quiz
                .choices
                .iter()
                .map(|c| format!("{}) {}", c.letter, c.title))
                .collect();
            let choice_json: Vec<Value> = quiz
                .choices
                .iter()
                .map(|c| json!({ "letter": c.letter.to_string(), "title": c.title }))
                .collect();
            tool_structured(
                &format!(
                    "Guess the shape (seed {seed}, round {round}, choices {choice_count}):\n\n{}\n{}\n\nCall quiz again with seed {seed}, round {round}, choices {choice_count}, and your guess letter.",
                    quiz.art,
                    choice_lines.join("\n")
                ),
                json!({
                    "game": "quiz",
                    "seed": seed,
                    "round": round,
                    "choiceCount": choice_count,
                    // The mystery render and the lettered choices ride in the
                    // structured payload, so a mind on a structured-content-only
                    // client sees the puzzle and can guess, not just read that a
                    // quiz exists. The answer waits for the grade.
                    "art": quiz.art,
                    "choices": choice_json
                }),
            )
        }
    }
}

/// The `munch` tool: present a board, or grade a set of bites.
fn munch_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let round = args
        .get("round")
        .and_then(Value::as_u64)
        .unwrap_or(numinous_core::FULL_DECK_ROUND);
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
        None => tool_structured(
            &format!(
                "{}\n{}\nCall munch again with your bites (1-based cell numbers).",
                board.rule.describe(),
                numinous_core::board_text(&board)
            ),
            json!({
                "game": "munch",
                "seed": seed,
                "round": round,
                // The rule and the board itself ride in the structured payload,
                // so a structured-content-only mind sees which cells to eat, not
                // just that a board exists.
                "rule": board.rule.describe(),
                "board": numinous_core::board_text(&board)
            }),
        ),
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
    use numinous_core::munch_arcade::{Arcade, Turn};
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
    // Shared with the App viewer attestation path: muncher never hides digits.
    let board = numinous_core::munch_arcade::board_text(&run);
    let state_text = format!(
        "ARCADE seed {seed} LEVEL {} LIVES {} SCORE {}\nRULE: {}\n{}",
        run.level,
        run.lives,
        run.score,
        run.board.rule.describe(),
        board
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
            "board": board,
            "cleared": cleared,
            "over": run.lives == 0
        }),
    )
}

/// The `scores` tool: the shared high-score table, prose and structured.
fn scores_tool(path: &std::path::Path) -> Value {
    let board = numinous_core::load_scoreboard_file(path);
    if board.entries.is_empty() {
        return tool_structured(
            "No scores yet. Post one: munch, quiz.",
            json!({ "count": 0, "top": [], "truncated": false }),
        );
    }
    let mut lines = vec!["HIGH SCORES".to_string()];
    let mut structured = Vec::new();
    for (rank, (key, score)) in board.top(15).iter().enumerate() {
        lines.push(format!("  {:>2}.  {score:>6}  {key}", rank + 1));
        structured.push(json!({ "rank": rank + 1, "key": key, "score": score }));
    }
    tool_structured(
        &lines.join("\n"),
        json!({
            "count": board.entries.len(),
            "truncated": board.entries.len() > structured.len(),
            "top": structured,
        }),
    )
}

/// The `nim` tool: replay the whole game from the move list, statelessly.
fn nim_tool(args: &Value) -> Value {
    let seed = effective_seed(args);
    let Some(turns) = nim_turns(args) else {
        return tool_error("Invalid Nim move history.");
    };
    let replay = match numinous_core::nim::replay(seed, &turns) {
        Ok(replay) => replay,
        Err(numinous_core::nim::NimReplayError::IllegalPlayerMove { turn, heaps }) => {
            return tool_error(&format!(
                "Illegal move: take {} from heap {}. Heaps now: {heaps:?}.",
                turn.take,
                turn.heap + 1
            ));
        }
        Err(numinous_core::nim::NimReplayError::InvalidOrderMove { .. }) => {
            return tool_error("The Order could not produce a legal move from this position.");
        }
    };
    match replay.winner {
        Some(numinous_core::nim::NimWinner::Player) => {
            let secret = numinous_core::nim_secret();
            tool_structured(
                &format!(
                    "You took the last stone. The Order concedes, and keeps its word:\n\n{secret}"
                ),
                // The promised secret lives in the structured payload too, so a
                // mind that reads only structuredContent still earns it.
                json!({ "game": "nim", "seed": seed, "won": true, "secret": secret }),
            )
        }
        Some(numinous_core::nim::NimWinner::Order) => tool_structured(
            "The Order takes the last stone. Again. (It is not luck.)",
            json!({ "game": "nim", "seed": seed, "won": false }),
        ),
        None => {
            let narration = nim_order_narration(&replay.order);
            let board: Vec<String> = replay
                .heaps
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
                json!({ "game": "nim", "seed": seed, "heaps": replay.heaps, "order": narration }),
            )
        }
    }
}

fn nim_turns(args: &Value) -> Option<Vec<numinous_core::nim::NimTurn>> {
    let Some(moves) = args.get("moves") else {
        return Some(Vec::new());
    };
    let moves = moves.as_array()?;
    if moves.len() > numinous_core::nim::MAX_REPLAY_TURNS {
        return None;
    }
    moves
        .iter()
        .map(|value| {
            let pair = value.as_array()?;
            if pair.len() != 2 {
                return None;
            }
            let heap = pair.first()?.as_u64()?.checked_sub(1)?;
            let heap = usize::try_from(heap).ok()?;
            // An oversized take remains illegal instead of truncating to a
            // smaller legal removal.
            let take = u32::try_from(pair.get(1)?.as_u64()?).unwrap_or(u32::MAX);
            Some(numinous_core::nim::NimTurn { heap, take })
        })
        .collect()
}

fn nim_order_narration(order: &[numinous_core::nim::NimTurn]) -> Vec<String> {
    order
        .iter()
        .map(|turn| format!("The Order takes {} from heap {}.", turn.take, turn.heap + 1))
        .collect()
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
            numinous_core::ROOM_CATALOG.len(),
            journey.wins,
            journey.secrets,
            journey.rank().name()
        ),
        json!({
            "level": journey.level(),
            "maxLevel": numinous_core::MAX_LEVEL,
            "xp": journey.sparks(),
            "starsLit": journey.visited.len(),
            "starsTotal": numinous_core::ROOM_CATALOG.len(),
            "wins": journey.wins,
            "plays": journey.plays,
            "secrets": journey.secrets,
            "rank": journey.rank().name()
        }),
    )
}

/// The `plot_expression` tool: Formula Jam discovery and still plots.
fn plot_expression_tool(args: &Value) -> Value {
    if args.get("list_recipes").and_then(Value::as_bool) == Some(true) {
        let recipes: Vec<Value> = numinous_core::STUDIO_RECIPES
            .iter()
            .enumerate()
            .map(|(i, source)| json!({ "index": i, "expr": source }))
            .collect();
        let lines: Vec<String> = numinous_core::STUDIO_RECIPES
            .iter()
            .enumerate()
            .map(|(i, source)| format!("  {i}: {source}"))
            .collect();
        return tool_structured(
            &format!(
                "Formula Jam curated recipes ({}):\n{}",
                numinous_core::studio_recipe_count(),
                lines.join("\n")
            ),
            json!({
                "discovery": "list",
                "recipeCount": numinous_core::studio_recipe_count(),
                "recipes": recipes,
                "valid": true
            }),
        );
    }

    let has_expr = args.get("expr").and_then(Value::as_str).is_some();
    let has_recipe = args.get("recipe").is_some();
    let has_seed = args.get("seed").is_some();
    let has_auto_step = args.get("auto_step").is_some();
    let mode_count = usize::from(has_expr) + usize::from(has_recipe) + usize::from(has_seed);
    if mode_count != 1 {
        return tool_error(
            "Provide exactly one of: expr (manual), recipe (index), or seed (random bank). Use list_recipes true to inspect the bank.",
        );
    }
    if has_auto_step && !has_seed {
        return tool_error("auto_step requires seed (stateless Auto walk over the curated bank).");
    }

    let source = if has_expr {
        numinous_core::PlotSource::Manual(
            args.get("expr")
                .and_then(Value::as_str)
                .expect("expr present")
                .to_string(),
        )
    } else if has_recipe {
        let Some(index) = args.get("recipe").and_then(Value::as_u64) else {
            return tool_error("Argument 'recipe' must be a non-negative integer.");
        };
        numinous_core::PlotSource::Recipe(index)
    } else {
        let Some(seed) = args.get("seed").and_then(Value::as_u64) else {
            return tool_error("Argument 'seed' must be a non-negative integer.");
        };
        let step = args.get("auto_step").and_then(Value::as_u64).unwrap_or(0);
        numinous_core::PlotSource::Seeded {
            seed,
            auto_step: has_auto_step.then_some(step),
        }
    };

    let request = match numinous_core::PlotRequest::new(
        source,
        args.get("xmin").and_then(Value::as_f64),
        args.get("xmax").and_then(Value::as_f64),
        args.get("a").and_then(Value::as_f64),
        None,
        None,
    ) {
        Ok(request) => request,
        Err(error) => return tool_error(&error.to_string()),
    };
    match request.execute() {
        Ok(result) => {
            let expr = request.source();
            let discovery = request.discovery().as_str();
            let xmin = request.xmin();
            let xmax = request.xmax();
            let a = request.parameter();
            let summary = format!(
                "y = {expr}    x in [{xmin:.3}, {xmax:.3}]    y in [{:.3}, {:.3}]\nDiscovery: {discovery}\n\n{}",
                result.ymin, result.ymax, result.text
            );
            tool_structured(
                &summary,
                json!({
                    "expression": expr,
                    "discovery": discovery,
                    "recipeIndex": request.recipe_index(),
                    "recipeCount": numinous_core::studio_recipe_count(),
                    "a": a,
                    "xmin": xmin,
                    "xmax": xmax,
                    "ymin": result.ymin,
                    "ymax": result.ymax,
                    "valid": true,
                    "plot": result.text
                }),
            )
        }
        Err(numinous_core::StudioRequestError::Undefined) => {
            tool_error("Nothing to plot: the function is undefined across this range.")
        }
        Err(error) => tool_error(&error.to_string()),
    }
}

/// Build a portable Studio capsule without granting the MCP face filesystem
/// access. The complete `.num` document and native link travel in the result.
fn save_creation_tool(args: &Value) -> Value {
    let Some(source) = args.get("expr").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'expr'.");
    };
    let mut creation = match numinous_core::StudioCreation::new(
        source,
        args.get("xmin")
            .and_then(Value::as_f64)
            .unwrap_or(numinous_core::DEFAULT_STUDIO_XMIN),
        args.get("xmax")
            .and_then(Value::as_f64)
            .unwrap_or(numinous_core::DEFAULT_STUDIO_XMAX),
        args.get("a")
            .and_then(Value::as_f64)
            .unwrap_or(numinous_core::DEFAULT_STUDIO_PARAMETER),
    ) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&error),
    };
    if let Some(title) = args.get("title").and_then(Value::as_str) {
        creation = match creation.with_title(title) {
            Ok(creation) => creation,
            Err(error) => return tool_error(&error),
        };
    }
    if let Some(author) = args.get("author").and_then(Value::as_str) {
        creation = match creation.with_author(author) {
            Ok(creation) => creation,
            Err(error) => return tool_error(&error),
        };
    }
    if let Some(raw_era) = args.get("era").and_then(Value::as_str) {
        let Some(era) = numinous_core::Era::parse(raw_era) else {
            return tool_error("Argument 'era' must be phosphor, 8-bit, vector, or modern.");
        };
        creation = creation.with_era(era);
    }
    studio_creation_result("save", &creation, None, args)
}

/// Open caller-supplied capsule data. A path-shaped string remains data and is
/// refused by the capsule parser rather than becoming an ambient file read.
fn open_creation_tool(args: &Value) -> Value {
    let Some(capsule) = args.get("capsule").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'capsule'.");
    };
    let creation = match numinous_core::StudioCreation::from_capsule(capsule) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&format!("Could not open Studio capsule: {error}")),
    };
    studio_creation_result("open", &creation, None, args)
}

/// Make one child through the same core fork constructor the CLI uses.
fn fork_creation_tool(args: &Value) -> Value {
    let Some(capsule) = args.get("parent").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'parent'.");
    };
    let parent = match numinous_core::StudioCreation::from_capsule(capsule) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&format!("Could not open parent capsule: {error}")),
    };
    let parent_link = parent.to_link();
    let child = match parent.fork(
        args.get("expr").and_then(Value::as_str),
        args.get("title").and_then(Value::as_str),
        args.get("author").and_then(Value::as_str),
    ) {
        Ok(creation) => creation,
        Err(error) => return tool_error(&error),
    };
    studio_creation_result("fork", &child, Some(&parent_link), args)
}

fn studio_preview_size(args: &Value) -> Result<(usize, usize), String> {
    let read = |name: &str, default: usize, maximum: usize| {
        let Some(value) = args.get(name) else {
            return Ok(default);
        };
        let value = value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| format!("Argument '{name}' must be a non-negative integer."))?;
        if !(2..=maximum).contains(&value) {
            return Err(format!(
                "Argument '{name}' must be an integer from 2 through {maximum}."
            ));
        }
        Ok(value)
    };
    Ok((
        read(
            "width",
            numinous_core::DEFAULT_PLOT_WIDTH,
            MAX_TOOL_WIDTH as usize,
        )?,
        read(
            "height",
            numinous_core::DEFAULT_PLOT_HEIGHT,
            MAX_TOOL_HEIGHT as usize,
        )?,
    ))
}

fn studio_creation_result(
    action: &str,
    creation: &numinous_core::StudioCreation,
    parent_link: Option<&str>,
    args: &Value,
) -> Value {
    let (width, height) = match studio_preview_size(args) {
        Ok(size) => size,
        Err(error) => return tool_error(&error),
    };
    let request = match numinous_core::PlotRequest::new(
        numinous_core::PlotSource::Manual(creation.source().to_string()),
        Some(creation.xmin()),
        Some(creation.xmax()),
        Some(creation.a()),
        Some(width),
        Some(height),
    ) {
        Ok(request) => request,
        Err(error) => return tool_error(&error.to_string()),
    };
    let preview = match request.execute() {
        Ok(preview) => preview,
        Err(numinous_core::StudioRequestError::Undefined) => {
            return tool_error(&format!(
                "Cannot {action} this Studio creation: the function is undefined across its saved range."
            ));
        }
        Err(error) => return tool_error(&error.to_string()),
    };

    let num_file = creation.to_num_file();
    let link = creation.to_link();
    if link.chars().count() > numinous_core::MAX_JOURNAL_SUBJECT_CHARS {
        return tool_error("The canonical Studio link exceeds the journal subject bound.");
    }
    let capsule_format_version = if num_file.starts_with("NUMINOUS_STUDIO 2\n") {
        2
    } else {
        1
    };
    let verb = match action {
        "save" => "Saved",
        "open" => "Opened",
        "fork" => "Forked",
        _ => "Prepared",
    };
    let mut structured = json!({
        "schema": "numinous.studio-creation",
        "schemaVersion": 1,
        "action": action,
        "capsuleFormatVersion": capsule_format_version,
        "expression": creation.source(),
        "xmin": creation.xmin(),
        "xmax": creation.xmax(),
        "a": creation.a(),
        "title": creation.title(),
        "author": creation.author(),
        "era": creation.era().map(numinous_core::Era::name),
        "descends": creation.descends(),
        "numFile": num_file,
        "link": link,
        "journalSubject": link,
        "createdFile": false,
        "readHostFile": false,
        "containsHostPath": false,
        "preview": {
            "width": width,
            "height": height,
            "ymin": preview.ymin,
            "ymax": preview.ymax,
            "render": preview.text,
        }
    });
    if let Some(parent_link) = parent_link {
        structured["parentLink"] = json!(parent_link);
    }
    tool_structured(
        &format!(
            "{verb} Studio creation as portable capsule data. No host file was read or created.\nExpression: {}\nLink: {}\n\n{}",
            creation.source(),
            creation.to_link(),
            preview.text
        ),
        structured,
    )
}

/// The `sing_expression` tool: an agent's function becomes readable music.
fn sing_expression_tool(args: &Value) -> Value {
    let want_receipt = match encounter_request(args) {
        Ok(want) => want,
        Err(message) => return tool_error(&message),
    };
    let Some(source) = args.get("expr").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'expr'.");
    };
    let notes = match args.get("notes").and_then(Value::as_u64) {
        Some(notes @ 1..=64) => Some(notes as usize),
        Some(_) => return tool_error("Argument 'notes' must be an integer from 1 through 64."),
        None => None,
    };
    let request = match numinous_core::SingRequest::new(
        source,
        args.get("xmin").and_then(Value::as_f64),
        args.get("xmax").and_then(Value::as_f64),
        args.get("a").and_then(Value::as_f64),
        notes,
    ) {
        Ok(request) => request,
        Err(error) => return tool_error(&error.to_string()),
    };
    let spec = match request.execute() {
        Ok(spec) => spec,
        Err(numinous_core::StudioRequestError::Undefined) => {
            return tool_error("Nothing to sing: the function is undefined across this range.");
        }
        Err(error) => return tool_error(&error.to_string()),
    };
    let mut lines = vec![format!(
        "y = {source} as a melody: {:.1}s, {} notes. Each line names the step \
         taken to reach it: the size measured in cents, the equal-tempered \
         name when one is near enough, and the whole number ratio when one is, \
         with how far off it sits.",
        spec.duration,
        spec.notes.len()
    )];
    let mut steps = Vec::with_capacity(spec.notes.len().saturating_sub(1));
    for (i, note) in spec.notes.iter().enumerate() {
        let step = i
            .checked_sub(1)
            .and_then(|previous| spec.notes.get(previous))
            .and_then(|previous| {
                numinous_core::Interval::between(f64::from(previous.freq), f64::from(note.freq))
            });
        lines.push(format!(
            "  note {:>2}: {:>7.1} Hz ({:>3})  at {:>5.2}s{}",
            i + 1,
            note.freq,
            note_name(note.freq),
            note.start,
            match step.as_ref() {
                Some(step) => format!("  [{}]", step.describe()),
                None => String::new(),
            }
        ));
        if let Some(step) = step {
            steps.push(interval_value(&step));
        }
    }
    // Reading the melody is what this tool has always done. Hearing it is the
    // thing six rounds of playtest kept asking for, so the sound can come too,
    // as a file rather than as a promise.
    let audible = match audible::requested(args) {
        Ok(true) => match audible::block(&spec) {
            Ok(rendered) => Some(rendered),
            Err(message) => return tool_error(&message),
        },
        Ok(false) => None,
        Err(message) => return tool_error(&message),
    };
    if audible.is_some() {
        lines.push(
            "A WAV of this melody follows as an audio attachment. It is \
             the only part of this reply that is not a description of the \
             melody, and it is a sound sent rather than a sound heard: \
             whether your client can surface it is its answer to give."
                .to_string(),
        );
    }
    // One note shape across this face: `listen_room` already publishes notes
    // under these names, and a second spelling would make a client parse the
    // same idea twice.
    let mut structured = json!({
        "expr": source,
        "duration_seconds": spec.duration,
        "notes": spec.notes.iter().enumerate().map(|(index, note)| json!({
            "index": index + 1,
            "frequency_hz": note.freq,
            "name": note_name(note.freq),
            "start_seconds": note.start,
            "duration_seconds": note.dur,
            "amplitude": note.amp,
        })).collect::<Vec<_>>(),
        "steps": steps,
        "audio": audible.as_ref().map(|(_, described)| described.clone()),
    });
    if want_receipt {
        let audio_asked = args.get("audio").and_then(Value::as_bool).unwrap_or(false);
        let action = encounter_sing_action(
            source,
            args.get("xmin")
                .and_then(Value::as_f64)
                .unwrap_or(numinous_core::DEFAULT_STUDIO_XMIN),
            args.get("xmax")
                .and_then(Value::as_f64)
                .unwrap_or(numinous_core::DEFAULT_STUDIO_XMAX),
            args.get("a")
                .and_then(Value::as_f64)
                .unwrap_or(numinous_core::DEFAULT_STUDIO_PARAMETER),
            notes.unwrap_or(numinous_core::DEFAULT_MELODY_NOTES) as u64,
            audio_asked,
        );
        let result = encounter_sing_result(
            source,
            spec.duration.into(),
            spec.notes.len() as u64,
            structured
                .get("audio")
                .and_then(|value| value.get("encodedBytes"))
                .and_then(Value::as_u64),
        );
        match issue_receipt(
            numinous_core::EncounterTool::SingExpression,
            &action.canonical_bytes(),
            &result.canonical_bytes(),
        ) {
            Ok(receipt) => {
                structured["encounter"] = receipt_json(&receipt, sing_action_json(&action))
            }
            Err(message) => return tool_error(&message),
        }
        lines.push("Encounter receipt attached.".to_string());
    }
    let result = tool_structured(&lines.join("\n"), structured);
    match audible {
        Some((block, _)) => audible::attach(result, block),
        None => result,
    }
}

/// Project one measured step between notes into typed evidence.
///
/// The size is a measurement and is always present. The ratio and the name
/// are search results and are absent when nothing sits close enough, so a
/// client can tell what was heard from what was merely nearby.
fn interval_value(step: &numinous_core::Interval) -> Value {
    json!({
        "cents": (step.cents * 10.0).round() / 10.0,
        "direction": step.direction.label(),
        "name": step.name,
        "ratio": step.ratio.map(|ratio| json!({
            "numerator": ratio.numerator,
            "denominator": ratio.denominator,
            "centsOff": ratio.cents_off,
        })),
    })
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

/// Answer an unknown room id with the rooms it was probably meant to be, then
/// one pointer to the listing tool. Returning the whole catalog spent thousands
/// of bytes of a player's context on a typo and handed over the map this
/// project deliberately withholds (`PLAY.md`).
/// A " Did you mean: ..." clause for an argument name the schema rejected, or
/// an empty string when nothing in the schema is close. A caller that misspells
/// `expr` as `expression` should not have to re-read the schema to find out.
fn nearest_argument_hint(property: &str, properties: Option<&Map<String, Value>>) -> String {
    let Some(known) = properties else {
        return String::new();
    };
    let names: Vec<&str> = known.keys().map(String::as_str).collect();
    let suggestions = numinous_core::nearest_names(property, names, MAX_ARGUMENT_SUGGESTIONS);
    if suggestions.is_empty() {
        return String::new();
    }
    format!(" Did you mean: {}?", suggestions.join(", "))
}

fn unknown_room(id: &str) -> String {
    let suggestions = numinous_core::nearest_room_ids(id, numinous_core::MAX_ROOM_SUGGESTIONS);
    let mut message = format!("No room with id '{}'.", numinous_core::echoable_id(id));
    if !suggestions.is_empty() {
        message.push_str(" Did you mean: ");
        message.push_str(&suggestions.join(", "));
        message.push('?');
    }
    message.push_str(" Call list_rooms to browse the catalog.");
    message
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

fn result_for_era(mut result: Value, method: &str, era: RequestEra) -> Value {
    if era != RequestEra::Modern {
        return result;
    }
    let Some(object) = result.as_object_mut() else {
        return result;
    };
    object
        .entry("resultType".to_string())
        .or_insert_with(|| json!("complete"));
    let meta = object
        .entry("_meta".to_string())
        .or_insert_with(|| json!({}));
    if !meta.is_object() {
        *meta = json!({});
    }
    if let Some(meta) = meta.as_object_mut() {
        meta.insert(SERVER_INFO_META_KEY.to_string(), server_info());
    }
    if object.get("resultType").and_then(Value::as_str) == Some("complete") {
        match method {
            "server/discover" => {
                object.insert("ttlMs".to_string(), json!(DISCOVERY_CACHE_TTL_MS));
                object.insert("cacheScope".to_string(), json!("public"));
            }
            "tools/list" => {
                object.insert("ttlMs".to_string(), json!(TOOLS_CACHE_TTL_MS));
                object.insert("cacheScope".to_string(), json!("public"));
            }
            _ => {}
        }
    }
    result
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_HEIGHT, DEFAULT_WIDTH, handle_request, handle_request_with, render_delta_json,
    };
    use numinous_broadcast::PublicTool;
    use numinous_core::MAX_DWELL_LOOKS;
    use serde_json::{Value, json};

    fn call(name: &str, arguments: Value) -> Value {
        handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 999,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }))
        .expect("tools/call must respond")
    }

    fn with_response_mode(mut arguments: Value, mode: &str) -> Value {
        arguments
            .as_object_mut()
            .expect("tool arguments are an object")
            .insert("response_mode".to_string(), json!(mode));
        arguments
    }

    fn tool_error_text(response: &Value) -> &str {
        assert_eq!(response["result"]["isError"], true, "{response}");
        response["result"]["content"][0]["text"]
            .as_str()
            .expect("tool error text")
    }

    #[test]
    fn fifteen_grades_only_the_calls_actually_made() {
        // Three correct calls out of three sent is not "3 of 5 called", and
        // a partial run must not wear a complete run's numbers.
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":60,"method":"tools/call",
            "params":{"name":"fifteen","arguments":{"seed":7,"rounds":5,"calls":["S","S","S"]}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("of 3 called"), "got: {text}");
        assert!(!text.contains("of 5 called"), "got: {text}");
        assert_eq!(resp["result"]["structuredContent"]["rounds"], 3);
    }

    #[test]
    fn viewers_see_a_gated_success_as_the_play_that_happened() {
        // The projection replays at the lowest level the arguments require:
        // a successful gated call already proves at least that much, so
        // nothing new leaks, and a watcher no longer sees a level-lock
        // refusal as the public result of a call that succeeded.
        let crack_args = json!({"seed": 3, "digits": 5, "guesses": ["12345"]});
        let projected = super::viewer_result(super::PublicTool::Crack, &crack_args, &json!({}));
        assert_ne!(
            projected["isError"], true,
            "a gated crack success must not project as a refusal: {projected}"
        );
        let seti_args = json!({"seed": 3, "channels": 6});
        let projected = super::viewer_result(super::PublicTool::Seti, &seti_args, &json!({}));
        assert_ne!(projected["isError"], true, "{projected}");
        let quiz_args = json!({"seed": 3, "choices": 6});
        let projected = super::viewer_result(super::PublicTool::Quiz, &quiz_args, &json!({}));
        assert_ne!(projected["isError"], true, "{projected}");
        // An ungated call still replays at level zero, leaking nothing.
        assert_eq!(
            super::level_the_arguments_require(super::PublicTool::Crack, &json!({"seed": 3})),
            0
        );
    }

    #[test]
    fn a_lost_write_reaches_the_response_not_only_stderr() {
        // The playing mind never sees the server's stderr, so a failed save
        // must ride the response text of the request that lost it.
        let blocked =
            std::env::temp_dir().join(format!("numinous-mcp-save-trouble-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&blocked);
        let _ = std::fs::remove_file(&blocked);
        std::fs::write(&blocked, "a file where a folder must go").expect("blocker");

        assert!(
            !super::post_score(&blocked.join("nested"), "munch seed:1", 5),
            "the blocked path must actually fail"
        );
        let noted = super::note_save_trouble(super::tool_text("WIN."));
        let text = noted["content"][0]["text"].as_str().expect("text");
        assert!(
            text.contains("NOTE: a local save failed"),
            "the note rides the response: {text}"
        );

        // Drained: the next response stays clean, so the note names exactly
        // the request that lost something.
        let clean = super::note_save_trouble(super::tool_text("WIN."));
        assert_eq!(clean["content"][0]["text"], "WIN.");
        let _ = std::fs::remove_file(&blocked);
    }

    #[test]
    fn test_persistence_paths_never_resolve_to_the_player_profile() {
        assert!(super::journey_path().starts_with(std::env::temp_dir()));
        assert!(super::scores_path().starts_with(std::env::temp_dir()));
        assert!(super::cairn_path().starts_with(std::env::temp_dir()));
        assert!(super::journal_path().starts_with(std::env::temp_dir()));
    }

    #[test]
    fn test_persistence_paths_are_stable_per_test_and_isolated_between_threads() {
        let journey = super::journey_path();
        assert_eq!(journey, super::journey_path());
        assert_ne!(journey, super::scores_path());
        assert_ne!(journey, super::cairn_path());
        assert_ne!(journey, super::journal_path());

        let other = std::thread::spawn(|| {
            let path = super::journey_path();
            std::fs::write(&path, b"other test").expect("test state should be writable");
            path
        })
        .join()
        .expect("path worker should finish");
        assert_ne!(journey, other);
        assert!(!other.exists());
        assert!(
            !other
                .parent()
                .expect("state path should have a parent")
                .exists()
        );
    }

    #[test]
    fn test_state_root_clears_stale_data_rejects_files_and_cleans_on_drop() {
        let parent = super::journey_path()
            .parent()
            .expect("state path should have a parent")
            .to_path_buf();
        let stale_root = parent.join("stale-root");
        std::fs::create_dir_all(&stale_root).expect("stale root should be creatable");
        std::fs::write(stale_root.join("old.txt"), b"stale")
            .expect("stale state should be writable");

        let root = super::TestStateRoot::at(stale_root.clone());
        assert!(stale_root.exists());
        assert!(!stale_root.join("old.txt").exists());
        drop(root);
        assert!(!stale_root.exists());

        let file_collision = parent.join("file-collision");
        std::fs::write(&file_collision, b"not a directory")
            .expect("collision file should be writable");
        let rejected =
            std::panic::catch_unwind(|| super::TestStateRoot::at(file_collision.clone()));
        assert!(rejected.is_err());
        std::fs::remove_file(file_collision).expect("collision file should be removable");
    }

    #[test]
    fn initialize_returns_server_info() {
        let resp =
            handle_request(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))
                .expect("initialize is a request and must respond");
        assert_eq!(resp["result"]["serverInfo"]["name"], "numinous");
        assert_eq!(resp["result"]["protocolVersion"], "2025-06-18");
        let instructions = resp["result"]["instructions"]
            .as_str()
            .expect("initialize ships agent instructions");
        assert!(
            instructions.contains("place_wager")
                && instructions.contains("number_wager")
                && instructions.contains("policy_wager")
                && instructions.contains("die_choice")
                && instructions.contains("counter_wager"),
            "instructions teach flagship aha args: {instructions}"
        );
        assert!(
            instructions.contains("describe_room is a safe doorway")
                && instructions.contains("reveal_room opens only after"),
            "instructions state the discovery and reveal gates: {instructions}"
        );
        assert!(
            instructions.contains("receipt true")
                && instructions.contains("does not keep the play")
                && instructions.contains("record_journal"),
            "instructions say a receipt is a replay proof, not a memory: {instructions}"
        );
        let preferred = handle_request(&json!({
            "jsonrpc":"2.0","id":2,"method":"initialize",
            "params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"t","version":"1"}}
        }))
        .expect("preferred version");
        assert_eq!(preferred["result"]["protocolVersion"], "2025-11-25");
        let unsupported_future = handle_request(&json!({
            "jsonrpc":"2.0","id":3,"method":"initialize",
            "params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"t","version":"1"}}
        }))
        .expect("unsupported version receives the compatibility default");
        assert_eq!(
            unsupported_future["result"]["protocolVersion"],
            "2025-06-18"
        );
    }

    fn modern_meta(capabilities: Value) -> Value {
        json!({
            super::PROTOCOL_VERSION_META_KEY: super::MODERN_PROTOCOL_VERSION,
            super::CLIENT_INFO_META_KEY: {
                "name": "numinous-test",
                "version": "1"
            },
            super::CLIENT_CAPABILITIES_META_KEY: capabilities
        })
    }

    #[test]
    fn modern_discovery_advertises_dual_era_support_and_cacheability() {
        let response = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": "discover",
            "method": "server/discover",
            "params": { "_meta": modern_meta(json!({})) }
        }))
        .expect("discovery is a request");
        let result = &response["result"];
        assert_eq!(result["resultType"], "complete");
        assert_eq!(
            result["supportedVersions"],
            json!(["2026-07-28", "2025-11-25", "2025-06-18"])
        );
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["ttlMs"], super::DISCOVERY_CACHE_TTL_MS);
        assert_eq!(result["cacheScope"], "public");
        assert_eq!(
            result["_meta"][super::SERVER_INFO_META_KEY]["name"],
            "numinous"
        );
        assert!(
            result["instructions"]
                .as_str()
                .is_some_and(|instructions| instructions.contains("multi-round-trip"))
        );
    }

    #[test]
    fn modern_requests_require_protocol_metadata_and_accept_optional_client_info() {
        let missing = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "server/discover",
            "params": {}
        }))
        .expect("malformed discovery receives an error");
        assert_eq!(missing["error"]["code"], -32602);

        let unsupported = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {
                "_meta": {
                    super::PROTOCOL_VERSION_META_KEY: "1900-01-01",
                    super::CLIENT_CAPABILITIES_META_KEY: {}
                }
            }
        }))
        .expect("unsupported version receives an error");
        assert_eq!(unsupported["error"]["code"], -32022);
        assert_eq!(unsupported["error"]["data"]["requested"], "1900-01-01");
        assert_eq!(
            unsupported["error"]["data"]["supported"],
            json!(["2026-07-28", "2025-11-25", "2025-06-18"])
        );

        let missing_capabilities = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {
                "_meta": {
                    super::PROTOCOL_VERSION_META_KEY: "2026-07-28"
                }
            }
        }))
        .expect("missing capabilities receives an error");
        assert_eq!(missing_capabilities["error"]["code"], -32602);

        let missing_identity = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list",
            "params": {
                "_meta": {
                    super::PROTOCOL_VERSION_META_KEY: "2026-07-28",
                    super::CLIENT_CAPABILITIES_META_KEY: {}
                }
            }
        }))
        .expect("client identity is optional");
        assert_eq!(missing_identity["result"]["resultType"], "complete");

        let malformed_identity = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/list",
            "params": {
                "_meta": {
                    super::PROTOCOL_VERSION_META_KEY: "2026-07-28",
                    super::CLIENT_CAPABILITIES_META_KEY: {},
                    super::CLIENT_INFO_META_KEY: {"name": "missing-version"}
                }
            }
        }))
        .expect("malformed optional client identity receives an error");
        assert_eq!(malformed_identity["error"]["code"], -32602);
    }

    #[test]
    fn jsonrpc_envelope_rejects_invalid_versions_ids_params_and_batches() {
        for request in [
            json!({"jsonrpc":"1.0","id":1,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":null,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":true,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":1.5,"method":"tools/list"}),
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":[]}),
            json!([]),
        ] {
            let response = handle_request(&request).expect("invalid request receives an error");
            assert_eq!(response["error"]["code"], -32600, "request: {request}");
            assert!(response["id"].is_null() || response["id"] == 1);
        }
    }

    #[test]
    fn modern_tool_catalog_is_cacheable_deterministic_and_explicitly_2020_12() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list",
            "params": { "_meta": modern_meta(json!({})) }
        });
        let first = handle_request(&request).expect("tools/list response");
        let second = handle_request(&request).expect("repeat tools/list response");
        assert_eq!(first, second);
        let result = &first["result"];
        assert_eq!(result["resultType"], "complete");
        assert_eq!(result["ttlMs"], super::TOOLS_CACHE_TTL_MS);
        assert_eq!(result["cacheScope"], "public");
        let tools = result["tools"].as_array().expect("tool array");
        assert_eq!(tools.len(), 39);
        assert!(
            tools
                .iter()
                .all(|tool| { tool["inputSchema"]["$schema"] == super::JSON_SCHEMA_2020_12 })
        );

        let invalid_cursor = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/list",
            "params": {
                "_meta": modern_meta(json!({})),
                "cursor": "not-issued-by-numinous"
            }
        }))
        .expect("invalid cursor receives an error");
        assert_eq!(invalid_cursor["error"]["code"], -32602);
    }

    #[test]
    fn modern_tool_results_carry_result_type_and_server_identity() {
        let response = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "_meta": modern_meta(json!({})),
                "name": "list_rooms",
                "arguments": { "response_mode": "compact" }
            }
        }))
        .expect("modern tool call response");
        assert_eq!(response["result"]["resultType"], "complete");
        assert_eq!(
            response["result"]["_meta"][super::SERVER_INFO_META_KEY]["name"],
            "numinous"
        );
        assert_eq!(response["result"]["structuredContent"]["count"], 355);

        let retired_ping = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "ping",
            "params": { "_meta": modern_meta(json!({})) }
        }))
        .expect("modern ping receives an error");
        assert_eq!(retired_ping["error"]["code"], -32601);
    }

    #[test]
    fn modern_predict_uses_form_elicitation_and_grades_the_retry() {
        let first = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "_meta": modern_meta(json!({ "elicitation": {} })),
                "name": "predict",
                "arguments": { "id": "slope-rider", "seed": 4 }
            }
        }))
        .expect("prediction pose response");
        assert_eq!(first["result"]["resultType"], "input_required");
        let elicitation = &first["result"]["inputRequests"]["prediction"];
        assert_eq!(elicitation["method"], "elicitation/create");
        assert_eq!(elicitation["params"]["mode"], "form");
        assert_eq!(
            elicitation["params"]["requestedSchema"]["required"],
            json!(["guess"])
        );
        assert!(first["result"].get("ttlMs").is_none());

        let graded = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "_meta": modern_meta(json!({ "elicitation": { "form": {} } })),
                "name": "predict",
                "arguments": { "id": "slope-rider", "seed": 4 },
                "inputResponses": {
                    "prediction": {
                        "action": "accept",
                        "content": { "guess": 0.0, "rate": 0.25 }
                    }
                }
            }
        }))
        .expect("prediction grade response");
        assert_eq!(graded["result"]["resultType"], "complete");
        assert_eq!(graded["result"]["structuredContent"]["game"], "predict");
        assert_eq!(graded["result"]["structuredContent"]["guess"], 0.0);
        assert_eq!(graded["result"]["structuredContent"]["rate_guess"], 0.25);

        let fallback = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "_meta": modern_meta(json!({})),
                "name": "predict",
                "arguments": { "id": "slope-rider", "seed": 4 }
            }
        }))
        .expect("prediction fallback response");
        assert_eq!(fallback["result"]["resultType"], "complete");
        assert_eq!(fallback["result"]["structuredContent"]["game"], "predict");
        assert!(fallback["result"].get("inputRequests").is_none());
    }

    #[test]
    fn declined_prediction_with_direct_arguments_never_records_progress() {
        let journey = super::test_state_path("declined-direct-prediction");
        let response = handle_request_with(
            &json!({
                "jsonrpc": "2.0",
                "id": 11,
                "method": "tools/call",
                "params": {
                    "_meta": modern_meta(json!({ "elicitation": {} })),
                    "name": "predict",
                    "arguments": { "id": "slope-rider", "seed": 4, "guess": 0.5 },
                    "inputResponses": {
                        "prediction": { "action": "decline" }
                    }
                }
            }),
            &journey,
        )
        .expect("decline response");
        assert_eq!(response["result"]["isError"], false);
        assert!(
            response["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("Nothing was graded or recorded"))
        );
        assert!(!journey.exists());
    }

    #[test]
    fn an_invalid_seed_earns_a_guiding_error_not_a_silent_default() {
        // A negative (or fractional) seed used to silently collide with seed 1;
        // now every tool guides on it, centrally.
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"crack","arguments":{"seed":-5}}
        }))
        .expect("must respond");
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("seed"),
            "the error names the offending argument: {text}"
        );
    }

    #[test]
    fn a_daily_request_pins_one_day_for_the_whole_request() {
        // The daily day is frozen once at the request boundary, so the reply
        // grading and the progress recording read the same value and cannot
        // straddle a midnight tick.
        let req = json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"munch","arguments":{"daily":true}}
        });
        let frozen = super::freeze_daily_day(&req);
        let args = &frozen["params"]["arguments"];
        let day = args[super::DAILY_DAY_KEY]
            .as_u64()
            .expect("the day is pinned into the request");
        assert_eq!(
            super::effective_seed(args),
            day,
            "seed reads the frozen day"
        );
        // A non-daily request passes through untouched.
        let plain = json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"munch","arguments":{"seed":5}}
        });
        assert!(
            super::freeze_daily_day(&plain)["params"]["arguments"]
                .get(super::DAILY_DAY_KEY)
                .is_none()
        );
    }

    #[test]
    fn run_sim_rejects_both_params_and_levers() {
        // The two are the same slot; passing both must guide, not silently drop
        // half the lever settings.
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"run_sim","arguments":{"id":"tribbles","params":{"breeding-rate":2.0},"levers":{"breeding-rate":1.0}}}
        }))
        .expect("must respond");
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("not both"), "guides on the conflict: {text}");
    }

    #[test]
    fn run_sim_rejects_unknown_wrong_type_and_out_of_range_levers() {
        for (arguments, expected) in [
            (
                json!({"id":"tribbles","params":{"bogus":1.0}}),
                "Unknown lever 'bogus'",
            ),
            (
                json!({"id":"tribbles","params":{"breeding-rate":"fast"}}),
                "must be a number",
            ),
            (
                json!({"id":"tribbles","params":{"breeding-rate":100.0}}),
                "between 0.1 and 3",
            ),
        ] {
            let resp = call("run_sim", arguments);
            let text = tool_error_text(&resp);
            assert!(
                text.contains(expected),
                "expected {expected:?}, got {text:?}"
            );
        }
    }

    #[test]
    fn the_quiz_and_munch_poses_carry_the_puzzle_in_structured_content() {
        // A structured-content-only client must see the puzzle itself, not just
        // read that a game exists: the pose branches carry it, like their graded
        // branches and the other games already do.
        for tool in ["quiz", "munch"] {
            let resp = handle_request(&json!({
                "jsonrpc":"2.0","id":9,"method":"tools/call",
                "params":{"name":tool,"arguments":{"seed":3}}
            }))
            .expect("must respond");
            let sc = &resp["result"]["structuredContent"];
            assert!(sc.is_object(), "{tool} pose must carry structuredContent");
            assert!(
                sc.get("art").is_some() || sc.get("board").is_some(),
                "{tool} pose structuredContent must carry the puzzle itself"
            );
        }
    }

    /// Every byte an MCP peer can be handed, for one tool call.
    ///
    /// Walks the whole JSON rather than reading one text field: a colour code
    /// hidden in a nested value would be just as unreadable to a peer that
    /// cannot see colour, and checking only `content[0].text` would miss it.
    fn every_string_in(value: &Value, into: &mut Vec<String>) {
        match value {
            Value::String(text) => into.push(text.clone()),
            Value::Array(items) => items.iter().for_each(|item| every_string_in(item, into)),
            Value::Object(fields) => {
                fields
                    .values()
                    .for_each(|field| every_string_in(field, into));
            }
            _ => {}
        }
    }

    #[test]
    fn no_mcp_response_ever_carries_a_color_a_peer_might_not_see() {
        // The terminal face has NO_COLOR and the App has been swept for colour
        // blindness. This face has neither, because it has never emitted colour
        // at all: it renders through `Canvas`, which is characters. That is a
        // property worth holding rather than a coincidence worth assuming, and
        // an MCP peer is exactly the reader most likely to have no colour.
        //
        // Every tool is called, with the list read from the binary rather than
        // written here, so a new tool cannot ship unchecked. Empty arguments
        // make most of them answer with an error, which is the point: an error
        // is a response a peer reads too, and it must be plain as well.
        let listed = handle_request(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
            .expect("tools/list must respond");
        let names: Vec<String> = listed["result"]["tools"]
            .as_array()
            .expect("tools is an array")
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_string))
            .collect();
        assert!(
            names.len() >= 30,
            "only {} tools listed, so this sweep is broken rather than the face being small",
            names.len()
        );

        let mut responses = vec![listed.clone()];
        for name in &names {
            responses.push(call(name, json!({})));
        }
        // Real calls too, so the sweep covers actual pictures rather than only
        // the refusals that empty arguments produce. Each is checked for having
        // succeeded: a typo in a tool name would turn these into two more
        // refusals and quietly leave the render path unswept.
        for (name, arguments) in [
            (
                "play_room",
                json!({"id": "times-tables", "width": 40, "height": 20}),
            ),
            ("describe_room", json!({"id": "cult-of-pi"})),
            ("list_rooms", json!({})),
        ] {
            let response = call(name, arguments);
            assert_ne!(
                response["result"]["isError"],
                json!(true),
                "{name} refused, so this sweep never reached a real answer: {response}"
            );
            responses.push(response);
        }

        for response in &responses {
            let mut strings = Vec::new();
            every_string_in(response, &mut strings);
            assert!(
                !strings.is_empty(),
                "a response carried no text at all, so this checks nothing"
            );
            for text in strings {
                assert!(
                    !text.contains('\u{1b}'),
                    "an MCP response carries an escape sequence, so a peer that cannot \
                     see colour would read control bytes as content: {text:?}"
                );
            }
        }
    }

    #[test]
    fn tools_list_has_the_expected_tools() {
        let resp = handle_request(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
            .expect("tools/list must respond");
        let tools = resp["result"]["tools"]
            .as_array()
            .expect("tools is an array");
        assert_eq!(tools.len(), 39);
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
        assert!(names.contains(&"save_creation"));
        assert!(names.contains(&"open_creation"));
        assert!(names.contains(&"fork_creation"));
        assert!(names.contains(&"sing_expression"));
        assert!(names.contains(&"explain_joke"));
        assert!(names.contains(&"journey"));
        assert!(names.contains(&"munch"));
        assert!(names.contains(&"munch_arcade"));
        assert!(names.contains(&"scores"));
        assert!(names.contains(&"forget"));
        assert!(names.contains(&"nim"));
        assert!(names.contains(&"broadcast_session"));
        assert!(names.contains(&"read_journal"));
        assert!(names.contains(&"record_journal"));
        assert!(names.contains(&"correct_journal"));
        assert!(names.contains(&"export_journal"));
        assert!(names.contains(&"erase_journal"));
        assert!(names.contains(&"workspace"));
        let save_creation = tools
            .iter()
            .find(|tool| tool["name"] == "save_creation")
            .expect("save_creation tool");
        assert_eq!(
            save_creation["inputSchema"]["properties"]["expr"]["maxLength"],
            numinous_core::MAX_STUDIO_SOURCE_CHARS
        );
        assert_eq!(
            save_creation["inputSchema"]["properties"]["author"]["maxLength"],
            numinous_core::MAX_META_TEXT_CHARS
        );
        let open_creation = tools
            .iter()
            .find(|tool| tool["name"] == "open_creation")
            .expect("open_creation tool");
        assert_eq!(
            open_creation["inputSchema"]["properties"]["capsule"]["maxLength"],
            numinous_core::MAX_SHARE_INPUT_BYTES
        );
        let fork_creation = tools
            .iter()
            .find(|tool| tool["name"] == "fork_creation")
            .expect("fork_creation tool");
        assert_eq!(
            fork_creation["inputSchema"]["properties"]["parent"]["maxLength"],
            numinous_core::MAX_SHARE_INPUT_BYTES
        );
        let workspace = tools
            .iter()
            .find(|tool| tool["name"] == "workspace")
            .expect("workspace tool");
        let workspace_properties = &workspace["inputSchema"]["properties"];
        assert_eq!(
            workspace_properties["op"]["enum"],
            json!(["inspect", "edit", "retrieve", "defer", "clear"])
        );
        assert_eq!(
            workspace_properties["room"]["maxLength"],
            super::MAX_TOOL_ID_CHARS
        );
        assert_eq!(
            workspace_properties["limit"]["maximum"],
            numinous_core::MAX_WORKSPACE_RETRIEVED
        );
        let correct_journal = tools
            .iter()
            .find(|tool| tool["name"] == "correct_journal")
            .expect("correct_journal tool");
        assert_eq!(
            correct_journal["inputSchema"]["properties"]["source"]["enum"],
            json!(["self-authored", "player-provided"])
        );
        let forget = tools
            .iter()
            .find(|tool| tool["name"] == "forget")
            .expect("forget tool");
        assert_eq!(
            forget["inputSchema"]["properties"]["journal"]["type"],
            "boolean"
        );
        let crack = tools
            .iter()
            .find(|tool| tool["name"] == "crack")
            .expect("crack tool");
        let crack_digits = &crack["inputSchema"]["properties"]["digits"];
        assert_eq!(crack_digits["minimum"], numinous_core::MIN_CODE_DIGITS);
        assert_eq!(crack_digits["maximum"], numinous_core::MAX_CODE_DIGITS);
        let play_room = tools
            .iter()
            .find(|tool| tool["name"] == "play_room")
            .expect("play_room tool");
        assert_eq!(
            play_room["inputSchema"]["properties"]["id"]["maxLength"],
            super::MAX_TOOL_ID_CHARS
        );
        let poke_schema = &play_room["inputSchema"]["properties"]["pokes"];
        assert_eq!(poke_schema["maxItems"], numinous_core::MAX_ROOM_POKES);
        assert_eq!(poke_schema["items"]["items"]["minimum"], 0);
        assert_eq!(poke_schema["items"]["items"]["maximum"], 1);
        let play_properties = &play_room["inputSchema"]["properties"];
        assert_eq!(play_properties["t"]["minimum"], 0);
        assert_eq!(play_properties["t"]["exclusiveMaximum"], 1);
        assert_eq!(play_properties["from_t"]["minimum"], 0);
        assert_eq!(play_properties["from_t"]["exclusiveMaximum"], 1);
        assert_eq!(
            play_room["inputSchema"]["dependentRequired"]["from_t"],
            json!(["t"])
        );
        assert_eq!(play_properties["receipt"]["type"], "boolean");
        let record_journal = tools
            .iter()
            .find(|tool| tool["name"] == "record_journal")
            .expect("record_journal tool");
        assert_eq!(
            record_journal["inputSchema"]["properties"]["receipt"]["type"],
            "object"
        );
        assert_eq!(
            record_journal["inputSchema"]["required"],
            json!(["kind", "text"])
        );
        assert_eq!(play_properties["width"]["minimum"], 1);
        assert_eq!(play_properties["width"]["maximum"], super::MAX_TOOL_WIDTH);
        assert_eq!(play_properties["height"]["minimum"], 1);
        assert_eq!(play_properties["height"]["maximum"], super::MAX_TOOL_HEIGHT);
        assert_eq!(
            play_properties["policy_wager"]["enum"],
            json!(["a", "b", "abb"])
        );
        assert_eq!(
            play_properties["die_choice"]["enum"],
            json!(["a", "b", "c"])
        );
        assert_eq!(
            play_properties["counter_wager"]["enum"],
            json!(["a", "b", "c"])
        );
        let listen = tools
            .iter()
            .find(|tool| tool["name"] == "listen_room")
            .expect("listen_room tool");
        let listen_properties = &listen["inputSchema"]["properties"];
        let listen_phase = &listen_properties["t"];
        assert_eq!(listen_phase["minimum"], 0);
        assert_eq!(listen_phase["exclusiveMaximum"], 1);
        assert_eq!(listen_properties["pokes"], play_properties["pokes"]);
        assert_eq!(listen_properties["gesture"], play_properties["gesture"]);
        assert_eq!(listen_properties["variation"]["minimum"], 0);
        assert_eq!(
            listen_properties["ambient_detail"]["enum"],
            json!(["summary", "events"])
        );
        assert_eq!(listen_properties["ambient_detail"]["default"], "summary");
        assert_eq!(listen_properties["receipt"]["type"], "boolean");
        let sing = tools
            .iter()
            .find(|tool| tool["name"] == "sing_expression")
            .expect("sing_expression tool");
        assert_eq!(
            sing["inputSchema"]["properties"]["receipt"]["type"],
            "boolean"
        );
        let gesture_variants = play_properties["gesture"]["items"]["oneOf"]
            .as_array()
            .expect("gesture event variants");
        assert_eq!(gesture_variants.len(), 4);
        assert_eq!(
            gesture_variants[0]["required"],
            json!(["kind", "x", "y", "t"])
        );
        assert_eq!(gesture_variants[3]["required"], json!(["kind"]));
        assert!(gesture_variants[3]["properties"].get("x").is_none());
        let challenge = tools
            .iter()
            .find(|tool| tool["name"] == "challenge")
            .expect("challenge tool");
        let challenge_phase = &challenge["inputSchema"]["properties"]["t"];
        assert_eq!(challenge_phase["minimum"], 0);
        assert_eq!(challenge_phase["exclusiveMaximum"], 1);
        let munch = tools
            .iter()
            .find(|tool| tool["name"] == "munch")
            .expect("munch tool");
        assert_eq!(
            munch["inputSchema"]["properties"]["bites"]["items"]["minimum"],
            1
        );
        let arcade = tools
            .iter()
            .find(|tool| tool["name"] == "munch_arcade")
            .expect("munch_arcade tool");
        assert_eq!(
            arcade["inputSchema"]["properties"]["actions"]["items"]["pattern"],
            "^(?:[Uu][Pp]|[Dd][Oo][Ww][Nn]|[Ll][Ee][Ff][Tt]|[Rr][Ii][Gg][Hh][Tt]|[Ee][Aa][Tt]|[WwAaSsDdEe])$"
        );
        assert_eq!(
            arcade["inputSchema"]["properties"]["actions"]["maxItems"],
            numinous_core::munch_arcade::MAX_REPLAY_ACTIONS
        );
        let nim = tools
            .iter()
            .find(|tool| tool["name"] == "nim")
            .expect("nim tool");
        assert_eq!(nim["inputSchema"]["properties"]["seed"]["minimum"], 0);
        assert_eq!(
            nim["inputSchema"]["properties"]["moves"]["maxItems"],
            numinous_core::nim::MAX_REPLAY_TURNS
        );
        for tool_name in ["nim", "hackenbush"] {
            let game = tools
                .iter()
                .find(|tool| tool["name"] == tool_name)
                .expect("tuple-move game");
            let tuple = &game["inputSchema"]["properties"]["moves"]["items"];
            assert_eq!(tuple["minItems"], 2, "{tool_name}");
            assert_eq!(tuple["maxItems"], 2, "{tool_name}");
            assert_eq!(tuple["items"]["minimum"], 1, "{tool_name}");
        }
        let run_sim = tools
            .iter()
            .find(|tool| tool["name"] == "run_sim")
            .expect("run_sim tool");
        assert_eq!(
            run_sim["inputSchema"]["properties"]["params"]["additionalProperties"]["type"],
            "number"
        );
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
        for tool in tools
            .iter()
            .filter(|tool| tool["name"] != "broadcast_session")
        {
            let response_mode = &tool["inputSchema"]["properties"]["response_mode"];
            assert_eq!(response_mode["type"], "string", "{}", tool["name"]);
            assert_eq!(response_mode["enum"], json!(["full", "compact"]));
            assert_eq!(response_mode["default"], "full");
        }
        let broadcast = tools
            .iter()
            .find(|tool| tool["name"] == "broadcast_session")
            .expect("broadcast control");
        assert!(
            broadcast["inputSchema"]["properties"]
                .get("response_mode")
                .is_none()
        );
        assert_eq!(
            broadcast["inputSchema"]["properties"]["pairing_code"]["maxLength"],
            numinous_broadcast::MAX_PAIRING_CODE_BYTES
        );
    }

    #[test]
    fn every_declared_tool_has_one_exhaustive_viewer_policy() {
        let tools = super::tools_catalog()["tools"]
            .as_array()
            .expect("tools array");
        let mut public = 0;
        let mut private = 0;
        let mut control = 0;
        for tool in tools {
            let name = tool["name"].as_str().expect("tool name");
            match super::viewer_policy(name).unwrap_or_else(|| panic!("missing policy for {name}"))
            {
                super::ViewerPolicy::Public(public_tool) => {
                    assert_eq!(public_tool.name(), name);
                    public += 1;
                }
                super::ViewerPolicy::Private => private += 1,
                super::ViewerPolicy::Control => control += 1,
            }
        }
        assert_eq!(public, numinous_broadcast::ALL_PUBLIC_TOOLS.len());
        assert_eq!(private, 15);
        assert_eq!(control, 1);
        assert_eq!(public + private + control, tools.len());
        assert!(super::viewer_policy("future_unreviewed_tool").is_none());
    }

    #[test]
    fn workspace_is_process_local_and_play_does_not_write_it() {
        let journey = super::test_state_path("workspace-visit");
        let broadcast = super::ConnectionBroadcast::new();
        let workspace = super::ProcessWorkspace::new();
        let call = |id: u64, name: &str, arguments: Value| {
            super::handle_request_with_visit(
                &json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "tools/call",
                    "params": {"name": name, "arguments": arguments}
                }),
                &journey,
                &broadcast,
                &workspace,
            )
            .expect("tools/call must respond")
        };

        let empty = call(1, "workspace", json!({}));
        assert_eq!(empty["result"]["isError"], false);
        assert_eq!(empty["result"]["structuredContent"]["empty"], true);
        assert_eq!(
            empty["result"]["structuredContent"]["schema"],
            "numinous.session-workspace"
        );
        assert_eq!(empty["result"]["structuredContent"]["scope"], "process");

        let edited = call(
            2,
            "workspace",
            json!({
                "op": "edit",
                "place": {"room": "times-tables", "t": 0.25},
                "intention": "why four lobes"
            }),
        );
        assert_eq!(edited["result"]["isError"], false);
        assert_eq!(
            edited["result"]["structuredContent"]["place"]["room"],
            "times-tables"
        );
        assert_eq!(
            edited["result"]["structuredContent"]["intention"],
            "why four lobes"
        );

        let _ = call(
            3,
            "play_room",
            json!({"id": "lorenz", "t": 0.4, "width": 40, "height": 20}),
        );
        let after_play = call(4, "workspace", json!({"op": "inspect"}));
        assert_eq!(
            after_play["result"]["structuredContent"]["place"]["room"],
            "times-tables"
        );
        assert!(after_play["result"]["structuredContent"]["place"]["room"] != "lorenz");

        let deferred = call(5, "workspace", json!({"op": "defer", "field": "intention"}));
        assert!(deferred["result"]["structuredContent"]["intention"].is_null());
        assert_eq!(
            deferred["result"]["structuredContent"]["deferred"]["intention"],
            "why four lobes"
        );

        let cleared = call(6, "workspace", json!({"op": "clear", "field": "all"}));
        assert_eq!(cleared["result"]["structuredContent"]["empty"], true);

        let other = super::handle_request_with(
            &json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": {"name": "workspace", "arguments": {}}
            }),
            &journey,
        )
        .expect("fresh process inspects empty");
        assert_eq!(other["result"]["structuredContent"]["empty"], true);
    }

    #[test]
    fn viewer_results_do_not_reveal_journey_levels_or_private_boon_choices() {
        let describe_args = json!({"id": "cult-of-pi"});
        let mut baseline_journey = numinous_core::Journey::default();
        baseline_journey.visit("cult-of-pi");
        let mut boon_journey = baseline_journey.clone();
        boon_journey.chosen.insert("cut:cult-of-pi:0".to_string());
        let baseline_description =
            super::describe_room_tool_for_journey(&describe_args, &baseline_journey);
        let boon_description = super::describe_room_tool_for_journey(&describe_args, &boon_journey);
        assert_eq!(
            baseline_description, boon_description,
            "safe descriptions never expose private progression"
        );
        let mut private_description = baseline_description.clone();
        private_description["structuredContent"]["journalCue"] = json!({
            "schema": "numinous.remembered-room-cue",
            "schemaVersion": 1,
            "status": "remembered",
            "contentsReturned": false,
        });
        private_description["content"][0]["text"] =
            json!("private local journal cue should not reach the viewer");
        let public_description = super::viewer_result(
            PublicTool::DescribeRoom,
            &describe_args,
            &private_description,
        );
        assert_eq!(public_description, baseline_description);
        assert!(
            public_description["structuredContent"]
                .get("journalCue")
                .is_none()
        );

        let crack_args = json!({"seed": 11, "digits": 5});
        let seti_args = json!({"seed": 12, "channels": 5});
        let quiz_args = json!({"seed": 13, "round": 0, "choices": 5});
        let reveal_args = json!({"id": "cult-of-pi"});
        let baseline_reveal = super::reveal_room_tool_for_journey(&reveal_args, &baseline_journey);
        let boon_reveal = super::reveal_room_tool_for_journey(&reveal_args, &boon_journey);
        let cases = [
            (
                PublicTool::RevealRoom,
                reveal_args,
                baseline_reveal,
                boon_reveal,
            ),
            (
                PublicTool::Crack,
                crack_args.clone(),
                super::crack_tool_at_level(&crack_args, 0),
                super::crack_tool_at_level(&crack_args, 5),
            ),
            (
                PublicTool::Seti,
                seti_args.clone(),
                super::seti_tool_at_level(&seti_args, 0),
                super::seti_tool_at_level(&seti_args, 7),
            ),
            (
                PublicTool::Quiz,
                quiz_args.clone(),
                super::quiz_tool_at_level(&quiz_args, 0),
                super::quiz_tool_at_level(&quiz_args, 3),
            ),
        ];
        for (tool, arguments, private_low, private_high) in cases {
            assert_ne!(
                private_low,
                private_high,
                "{} is state-sensitive",
                tool.name()
            );
            assert_eq!(
                super::viewer_result(tool, &arguments, &private_low),
                super::viewer_result(tool, &arguments, &private_high),
                "{} projection must be state-independent",
                tool.name()
            );
        }
    }

    #[test]
    fn crack_rejects_code_lengths_outside_the_shared_contract() {
        for digits in [
            (numinous_core::MIN_CODE_DIGITS - 1) as u64,
            (numinous_core::MAX_CODE_DIGITS + 1) as u64,
            u64::MAX,
        ] {
            let result = super::crack_tool_at_level(&json!({"digits": digits}), 5);
            assert_eq!(result["isError"], true, "accepted {digits} digits");
        }
        for digits in [json!(-1), json!(4.0), json!("4"), Value::Null] {
            let result = super::crack_tool_at_level(&json!({"digits": digits}), 5);
            assert_eq!(result["isError"], true, "accepted {digits} digits");
        }
    }

    #[test]
    fn broadcast_control_is_redacted_and_never_touches_progress() {
        let session = super::ConnectionBroadcast::new();
        let journey = super::test_state_path("broadcast-control-journey");
        let response = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 701,
                "method": "tools/call",
                "params": {"name": "broadcast_session", "arguments": {"action": "status"}}
            }),
            &journey,
            &session,
        )
        .expect("status response");
        assert_eq!(response["result"]["structuredContent"]["state"], "disabled");
        assert_eq!(
            response["result"]["structuredContent"]["privateActivityVisible"],
            false
        );
        assert!(!journey.exists());

        let secret = "numinous1.7.private-capability";
        let rejected = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 702,
                "method": "tools/call",
                "params": {"name": "broadcast_session", "arguments": {"action": "start", "pairing_code": secret}}
            }),
            &journey,
            &session,
        )
        .expect("rejected response");
        let encoded = rejected.to_string();
        assert!(!encoded.contains(secret));
        assert!(!journey.exists());
        // Reported from packaged play: a rejected start said only that pairing
        // failed, so the one recovery left was guessing another code. Name the
        // human invitation instead, without echoing what was tried.
        let guidance = rejected["result"]["content"][0]["text"]
            .as_str()
            .expect("rejection text");
        assert!(
            guidance.contains("Shared Play"),
            "a rejected start must name where a real code comes from: {guidance}"
        );
        assert!(
            !guidance.contains(secret),
            "guidance must not echo the attempted code: {guidance}"
        );

        // Every other failure is inspectable state, so it stays terse.
        let no_session = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 703,
                "method": "tools/call",
                "params": {"name": "broadcast_session", "arguments": {"action": "pause"}}
            }),
            &journey,
            &session,
        )
        .expect("pause response");
        assert!(
            !no_session["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Shared Play")
        );
    }

    #[test]
    fn public_replays_normalize_daily_flags_and_effective_seeds() {
        let daily = super::replay_arguments(json!({
            "daily": true,
            "dailyDay": 20_260_718_u64,
            "response_mode": "compact"
        }));
        assert_eq!(daily, json!({"seed": 20_260_718_u64}));
        assert_eq!(
            super::replay_arguments(json!({"daily": false, "seed": 23})),
            json!({"seed": 23})
        );
        assert_eq!(
            super::replay_arguments(json!({"daily": false, "seed": -1})),
            json!({})
        );
    }

    #[test]
    fn consented_handler_emits_public_play_and_keeps_control_and_progress_private() {
        use numinous_broadcast::{
            ConsentMachine, HandshakeResponse, PairingOffer, PairingVerdict, PublicTool,
            PublicToolEvent, WireMessage, configure_handshake_stream, configure_public_stream,
            numinous_compatibility, read_handshake_request, read_public_message,
            write_handshake_proof, write_handshake_response,
        };
        use std::io::BufReader;
        use std::net::{Ipv4Addr, TcpListener};
        use std::num::NonZeroU16;
        use std::sync::mpsc;
        use std::time::SystemTime;

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let port = NonZeroU16::new(listener.local_addr().expect("address").port()).expect("port");
        let offer = PairingOffer::generate(port, SystemTime::now()).expect("offer");
        let code = offer.display_code();
        let compatibility = numinous_compatibility().expect("compatibility");
        let mut gate = offer.into_gate(compatibility.clone());
        let (event_tx, event_rx) = mpsc::channel();
        let host = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            configure_handshake_stream(&stream).expect("handshake bounds");
            write_handshake_proof(&mut stream, &gate.host_proof()).expect("host proof");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let request = read_handshake_request(&mut reader).expect("request");
            let PairingVerdict::Accepted { session_id } = gate.verify(&request, SystemTime::now())
            else {
                panic!("pairing must succeed");
            };
            let machine = ConsentMachine::new(session_id, compatibility.clone());
            machine.begin_awaiting().expect("awaiting");
            let consent_epoch = machine.allow().expect("allow");
            write_handshake_response(
                &mut stream,
                &HandshakeResponse::Accepted {
                    session_id,
                    consent_epoch,
                    compatibility,
                },
            )
            .expect("response");
            configure_public_stream(&stream).expect("public bounds");
            let mut reader = BufReader::new(stream);
            let message =
                read_public_message::<_, PublicToolEvent>(&mut reader).expect("public message");
            event_tx.send(message).expect("send");
        });

        let session = super::ConnectionBroadcast::new();
        let journey = super::test_state_path("broadcast-integration-journey");
        let start = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 710,
                "method": "tools/call",
                "params": {
                    "_meta": modern_meta(json!({})),
                    "name": "broadcast_session",
                    "arguments": {"action": "start", "pairing_code": code}
                }
            }),
            &journey,
            &session,
        )
        .expect("start response");
        assert_eq!(start["result"]["structuredContent"]["state"], "live");

        let mut intruder_meta = modern_meta(json!({}));
        intruder_meta[super::CLIENT_INFO_META_KEY]["name"] = json!("other-client");
        let same_connection_status = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 708,
                "method": "tools/call",
                "params": {
                    "_meta": intruder_meta.clone(),
                    "name": "broadcast_session",
                    "arguments": {"action": "status"}
                }
            }),
            &journey,
            &session,
        )
        .expect("same-connection status response");
        assert_eq!(
            same_connection_status["result"]["structuredContent"]["state"],
            "live"
        );

        let forged_non_tool = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 709,
                "method": "ping",
                "params": {"name": "play_room", "arguments": {"id": "times-tables"}}
            }),
            &journey,
            &session,
        )
        .expect("ping response");
        assert_eq!(forged_non_tool["result"], json!({}));

        let private = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 711,
                "method": "tools/call",
                "params": {"name": "journey", "arguments": {}}
            }),
            &journey,
            &session,
        )
        .expect("private response");
        assert_eq!(private["result"]["isError"], false);

        let public = super::handle_request_with_session(
            &json!({
                "jsonrpc": "2.0",
                "id": 712,
                "method": "tools/call",
                "params": {
                    "_meta": modern_meta(json!({})),
                    "name": "play_room",
                    "arguments": {"id": "times-tables", "width": 40, "height": 20, "t": 0.25}
                }
            }),
            &journey,
            &session,
        )
        .expect("public response");
        assert_eq!(public["result"]["isError"], false);

        let WireMessage::Event(event) = event_rx.recv().expect("event") else {
            panic!("first public message must be play");
        };
        assert_eq!(event.public_sequence, 0);
        assert_eq!(event.event.tool, PublicTool::PlayRoom);
        assert_eq!(event.event.arguments["id"], "times-tables");
        let mut expected_public_result = public["result"]
            .as_object()
            .expect("tool result object")
            .clone();
        expected_public_result.remove("_meta");
        expected_public_result.remove("resultType");
        assert_eq!(event.event.result, expected_public_result);
        host.join().expect("host");
    }

    #[test]
    fn compact_response_mode_preserves_complete_structured_results() {
        let cases = [
            ("list_rooms", json!({})),
            ("describe_room", json!({"id":"times-tables"})),
            (
                "play_room",
                json!({"id":"times-tables","width":72,"height":32,"t":0.25}),
            ),
            (
                "listen_room",
                json!({"id":"times-tables","t":0.25,"pokes":[[0.375,0.5]]}),
            ),
            ("run_sim", json!({"id":"tribbles"})),
            ("quiz", json!({"seed":3})),
            ("quiz", json!({"seed":3,"guess":"A"})),
            ("gauntlet", json!({"seed":3})),
            (
                "gauntlet",
                json!({"seed":5,"answers":{
                    "bites":[1,2],"shape":"A","sky":"B","wires":["9500"]
                }}),
            ),
            ("trophies", json!({})),
        ];

        for (tool, arguments) in cases {
            let default = call(tool, arguments.clone());
            let explicit_full = call(tool, with_response_mode(arguments.clone(), "full"));
            let compact = call(tool, with_response_mode(arguments, "compact"));

            assert_eq!(
                default, explicit_full,
                "{tool} full mode changed the default"
            );
            assert_eq!(
                default["result"]["structuredContent"], compact["result"]["structuredContent"],
                "{tool} compact mode changed the typed result"
            );
            assert_eq!(compact["result"]["isError"], false);
            let default_text = default["result"]["content"][0]["text"]
                .as_str()
                .expect("default text");
            let compact_text = compact["result"]["content"][0]["text"]
                .as_str()
                .expect("compact text");
            assert!(
                compact_text.len() * 4 <= default_text.len() * 3,
                "{tool} compact text should be smaller: {} versus {} bytes",
                compact_text.len(),
                default_text.len()
            );
            assert!(compact_text.contains("structuredContent"), "{tool}");
            if tool == "list_rooms" {
                for door in ["touch", "strange-loop", "wander"] {
                    assert!(
                        compact_text.contains(door),
                        "compact discovery omitted {door}: {compact_text}"
                    );
                }
            }
            if tool == "play_room" {
                for field in ["render", "pokes", "gesture", "status", "delta"] {
                    assert!(
                        compact_text.contains(field),
                        "missing {field}: {compact_text}"
                    );
                }
                assert!(
                    !compact_text.contains("input"),
                    "compact guidance must name only real fields: {compact_text}"
                );
            }
        }
    }

    #[test]
    fn compact_response_mode_never_hides_errors_or_text_only_results() {
        let full_error = call("play_room", json!({"id":"no-such-room"}));
        let compact_error = call(
            "play_room",
            with_response_mode(json!({"id":"no-such-room"}), "compact"),
        );
        assert_eq!(compact_error, full_error);

        let full_text_only = call("list_sims", json!({}));
        let compact_text_only = call("list_sims", with_response_mode(json!({}), "compact"));
        assert_eq!(compact_text_only, full_text_only);

        for (tool, arguments) in [
            ("journey", json!({})),
            ("scores", json!({})),
            ("forget", json!({})),
        ] {
            let full = call(tool, arguments.clone());
            let compact = call(tool, with_response_mode(arguments, "compact"));
            assert_eq!(
                compact, full,
                "{tool} text carries information absent from structuredContent"
            );
        }
    }

    #[test]
    fn response_mode_does_not_change_progress_or_invalid_call_side_effects() {
        let suffix = format!("{}-response-mode", std::process::id());
        let full_path = std::env::temp_dir().join(format!("numinous-{suffix}-full.txt"));
        let compact_path = std::env::temp_dir().join(format!("numinous-{suffix}-compact.txt"));
        let invalid_path = std::env::temp_dir().join(format!("numinous-{suffix}-invalid.txt"));
        let notification_path =
            std::env::temp_dir().join(format!("numinous-{suffix}-notification.txt"));
        for path in [&full_path, &compact_path, &invalid_path, &notification_path] {
            let _ = std::fs::remove_file(path);
        }

        let request = |mode: &str| {
            json!({
                "jsonrpc":"2.0","id":77,"method":"tools/call",
                "params":{"name":"play_room","arguments":{
                    "id":"times-tables","t":0.25,"response_mode":mode
                }}
            })
        };
        let full = handle_request_with(&request("full"), &full_path).expect("full response");
        let compact =
            handle_request_with(&request("compact"), &compact_path).expect("compact response");
        assert_eq!(
            full["result"]["structuredContent"],
            compact["result"]["structuredContent"]
        );
        assert_eq!(
            numinous_core::load_journey_file(&full_path),
            numinous_core::load_journey_file(&compact_path)
        );

        for (invalid, expected) in [
            (json!("brief"), "must be one of"),
            (json!(3), "must be a string"),
            (Value::Null, "must be a string"),
            (json!([]), "must be a string"),
        ] {
            let response = handle_request_with(
                &json!({
                    "jsonrpc":"2.0","id":78,"method":"tools/call",
                    "params":{"name":"play_room","arguments":{
                        "id":"times-tables","response_mode":invalid
                    }}
                }),
                &invalid_path,
            )
            .expect("invalid mode receives guidance");
            assert_eq!(response["result"]["isError"], true);
            assert!(
                response["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .contains(expected)
            );
        }
        assert!(
            !invalid_path.exists(),
            "invalid presentation arguments must not record a visit"
        );

        let notification = json!({
            "jsonrpc":"2.0","method":"tools/call",
            "params":{"name":"play_room","arguments":{
                "id":"times-tables","t":0.25,"response_mode":"compact"
            }}
        });
        assert_eq!(
            handle_request_with(&notification, &notification_path),
            None,
            "notifications record progress but return no response"
        );
        assert_eq!(
            numinous_core::load_journey_file(&full_path),
            numinous_core::load_journey_file(&notification_path),
            "compact notifications record exactly the same visit"
        );

        for path in [full_path, compact_path, invalid_path, notification_path] {
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_file(format!("{}.lock", path.display()));
        }
    }

    #[test]
    fn declared_tool_schemas_are_enforced_at_runtime() {
        let too_many_pokes: Vec<_> = (0..=numinous_core::MAX_ROOM_POKES)
            .map(|_| json!([0.5, 0.5]))
            .collect();
        let cases = vec![
            ("list_rooms", json!([]), "must be an object"),
            ("describe_room", json!({}), "required argument 'id'"),
            (
                "play_room",
                json!({"id":"lorenz","widht":7}),
                "Unexpected argument 'widht'",
            ),
            (
                "play_room",
                json!({"id":"lorenz","width":"7"}),
                "must be an integer",
            ),
            ("play_room", json!({"id":"lorenz","width":0}), "at least 1"),
            ("play_room", json!({"id":"lorenz","height":0}), "at least 1"),
            (
                "play_room",
                json!({"id":"lorenz","width":super::MAX_TOOL_WIDTH + 1}),
                "at most 512",
            ),
            (
                "play_room",
                json!({"id":"lorenz","height":super::MAX_TOOL_HEIGHT + 1}),
                "at most 256",
            ),
            ("play_room", json!({"id":"lorenz","t":-1.0}), "at least 0"),
            (
                "play_room",
                json!({"id":"lorenz","t":1.0}),
                "the loop endpoint is 0.0",
            ),
            (
                "play_room",
                json!({"id":"x".repeat(super::MAX_TOOL_ID_CHARS + 1)}),
                "at most 64 characters",
            ),
            (
                "plot_expression",
                json!({"expr":"x".repeat(numinous_core::MAX_STUDIO_SOURCE_CHARS + 1)}),
                "at most 512 characters",
            ),
            (
                "save_creation",
                json!({"expr":"x".repeat(numinous_core::MAX_STUDIO_SOURCE_CHARS + 1)}),
                "at most 512 characters",
            ),
            (
                "save_creation",
                json!({"expr":"x","title":"x".repeat(numinous_core::MAX_META_TEXT_CHARS + 1)}),
                "at most 64 characters",
            ),
            (
                "save_creation",
                json!({"expr":"x","era":"future"}),
                "must be one of",
            ),
            (
                "open_creation",
                json!({"capsule":"x".repeat(numinous_core::MAX_SHARE_INPUT_BYTES + 1)}),
                "at most 8192 characters",
            ),
            (
                "fork_creation",
                json!({"parent":"x".repeat(numinous_core::MAX_SHARE_INPUT_BYTES + 1)}),
                "at most 8192 characters",
            ),
            (
                "cairn",
                json!({"leave":"x".repeat(numinous_core::cairn::MAX_BEQUEST_CHARS + 1)}),
                "at most 140 characters",
            ),
            (
                "sing_expression",
                json!({"expr":"sin(x)","notes":65}),
                "at most 64",
            ),
            (
                "play_room",
                json!({"id":"lorenz","pokes":[[0.5]]}),
                "at least 2 items",
            ),
            (
                "play_room",
                json!({"id":"lorenz","pokes":too_many_pokes}),
                "at most",
            ),
            (
                "play_room",
                json!({"id":"lorenz","gesture":[{"kind":"cancel","note":"hidden"}]}),
                "exactly one declared event shape",
            ),
            (
                "play_room",
                json!({"id":"lorenz","gesture":{"kind":"down","x":0.5,"y":0.5,"t":0.25}}),
                "for example [{\"kind\":\"down\"",
            ),
            (
                "play_room",
                json!({"id":"lorenz","gesture":[{"kind":"down"}]}),
                "exactly one declared event shape",
            ),
            (
                "play_room",
                json!({"id":"lorenz","gesture":[{"kind":"cancel","x":0.5}]}),
                "exactly one declared event shape",
            ),
            (
                "listen_room",
                json!({"id":"lorenz","variation":-1}),
                "at least 0",
            ),
            (
                "challenge",
                json!({"id":"lorenz","kind":"mystery"}),
                "must be one of",
            ),
            ("munch", json!({"bites":["first"]}), "bites[0]"),
            ("forget", json!({"confirm":"yes"}), "must be a boolean"),
            (
                "play_room",
                json!({"id":"lorenz","receipt":"yes"}),
                "must be a boolean",
            ),
            (
                "gauntlet",
                json!({"answers":{"surprise":42}}),
                "unexpected field 'surprise'",
            ),
            (
                "list_rooms",
                json!({"surprise":42}),
                "Unexpected argument 'surprise'",
            ),
            (
                "list_rooms",
                json!({"response_mode":"brief"}),
                "must be one of",
            ),
            ("quiz", json!({"seed":3,"choices":7}), "must be at most 6"),
        ];

        for (tool, arguments, expected) in cases {
            let response = call(tool, arguments);
            let text = tool_error_text(&response);
            assert!(
                text.contains(expected),
                "{tool} should guide with {expected:?}, got: {text}"
            );
        }
    }

    #[test]
    fn representative_valid_arguments_cross_the_schema_boundary() {
        let calls = [
            ("list_rooms", json!({})),
            ("describe_room", json!({"id":"lorenz"})),
            (
                "play_room",
                json!({"id":"lorenz","t":0.5,"width":40,"height":20,"variation":0}),
            ),
            (
                "challenge",
                json!({"id":"times-tables","kind":"touch","seed":3}),
            ),
            ("predict", json!({"id":"slope-rider","seed":4})),
            (
                "run_sim",
                json!({"id":"tribbles","params":{"breeding-rate":2.0}}),
            ),
            ("munch", json!({"seed":1,"round":0,"bites":[1]})),
            ("party", json!({"guests":5,"shakes":[[1,2,"r"]]})),
            ("forget", json!({"confirm":false,"scores":false})),
        ];

        for (tool, arguments) in calls {
            let response = call(tool, arguments);
            assert_eq!(
                response["result"]["isError"], false,
                "valid {tool} call was rejected: {response}"
            );
        }

        let play = call("play_room", json!({"id":"lorenz","width":40,"height":20}));
        assert_eq!(play["result"]["structuredContent"]["width"], 40);
        assert_eq!(play["result"]["structuredContent"]["height"], 20);
        let quiz = call("quiz", json!({"seed":3,"choices":3}));
        assert_eq!(quiz["result"]["isError"], false, "{quiz}");
        assert_eq!(
            quiz["result"]["structuredContent"]["choices"]
                .as_array()
                .map(Vec::len),
            Some(3)
        );
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
    fn malformed_gauntlet_wire_does_not_consume_an_attempt() {
        let puzzle = numinous_core::GauntletPuzzle::new(5);
        let code = puzzle.bomb_code_text();
        let run = handle_request(&json!({
            "jsonrpc":"2.0","id":97,"method":"tools/call",
            "params":{"name":"gauntlet","arguments":{"seed":5,"answers":{
                "bites":[],"shape":"","sky":"","wires":["not a wire", code]
            }}}
        }))
        .expect("must respond");
        let sc = &run["result"]["structuredContent"];
        assert_eq!(sc["stageScores"][3], 40);
        assert_eq!(sc["reveals"][3], "BOMB: DEFUSED. +40  CLEAN");
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
    fn scores_post_and_rank_across_minds() {
        let path = std::env::temp_dir().join("numinous_mcp_scores_test.txt");
        let _ = std::fs::remove_file(&path);
        let empty = super::scores_tool(&path);
        assert_eq!(empty["structuredContent"]["count"], 0);
        assert_eq!(empty["structuredContent"]["top"], json!([]));
        assert_eq!(empty["structuredContent"]["truncated"], false);
        assert!(super::post_score(&path, "munch seed:7 board:0", 40));
        assert!(!super::post_score(&path, "munch seed:7 board:0", 10));
        assert!(super::post_score(&path, "munch seed:7 board:0", 90));
        let resp = super::scores_tool(&path);
        let text = resp["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("HIGH SCORES"));
        assert!(text.contains("90"));
        assert_eq!(resp["structuredContent"]["count"], 1);
        assert_eq!(resp["structuredContent"]["truncated"], false);
        assert_eq!(resp["structuredContent"]["top"][0]["score"], 90);

        for index in 0..20 {
            assert!(super::post_score(
                &path,
                &format!("quiz seed:{index} round:0"),
                100 + index,
            ));
        }
        let bounded = super::scores_tool(&path);
        assert_eq!(bounded["structuredContent"]["count"], 21);
        assert_eq!(
            bounded["structuredContent"]["top"]
                .as_array()
                .expect("top array")
                .len(),
            15
        );
        assert_eq!(bounded["structuredContent"]["truncated"], true);
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
    fn munch_defaults_to_the_shared_complete_deck_round() {
        let shown = handle_request(&json!({
            "jsonrpc":"2.0","id":62,"method":"tools/call",
            "params":{"name":"munch","arguments":{"seed":7}}
        }))
        .expect("tools/call must respond");
        assert_eq!(
            shown["result"]["structuredContent"]["round"],
            numinous_core::FULL_DECK_ROUND
        );
        let tools = handle_request(&json!({
            "jsonrpc":"2.0","id":63,"method":"tools/list"
        }))
        .expect("tools/list must respond");
        let munch = tools["result"]["tools"]
            .as_array()
            .and_then(|tools| tools.iter().find(|tool| tool["name"] == "munch"))
            .expect("munch descriptor");
        let description = munch["description"].as_str().unwrap_or_default();
        assert!(description.contains("Fibonacci"), "{description}");
        assert!(description.contains("digit sums"), "{description}");
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
    fn varied_progress_records_only_listed_rooms_once() {
        let path = std::env::temp_dir().join("numinous_mcp_varied_progress_test.txt");
        let _ = std::fs::remove_file(&path);
        for (id, variation) in [
            ("times-tables", 42),
            ("times-tables", 42),
            ("tetractys", 42),
            ("missing", 42),
        ] {
            super::record_progress(
                &json!({
                    "jsonrpc":"2.0","id":53,"method":"tools/call",
                    "params":{"name":"play_room","arguments":{"id":id,"variation":variation}}
                }),
                &path,
            );
        }
        super::record_progress(
            &json!({
                "jsonrpc":"2.0","id":54,"method":"tools/call",
                "params":{"name":"play_room","arguments":{
                    "id":"kepler-areas","pokes":[[0.8,0.5]],
                    "speed_wager":"faster","aha_summon":true
                }}
            }),
            &path,
        );
        super::record_progress(
            &json!({
                "jsonrpc":"2.0","id":55,"method":"tools/call",
                "params":{"name":"play_room","arguments":{"id":"kepler-laws"}}
            }),
            &path,
        );

        let journey = super::load_journey(&path);
        assert_eq!(journey.visited.len(), 2);
        assert!(journey.visited.contains("times-tables"));
        assert!(journey.visited.contains("kepler-laws"));
        assert!(!journey.visited.contains("kepler-areas"));
        assert!(journey.has_consolidated("kepler-laws"));
        assert!(!journey.visited.contains("tetractys"));
        assert!(!journey.visited.contains("missing"));
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
        assert_eq!(resp["result"]["structuredContent"]["discovery"], "manual");
        assert_eq!(resp["result"]["structuredContent"]["valid"], true);

        let bad = handle_request(&json!({
            "jsonrpc":"2.0","id":41,"method":"tools/call",
            "params":{"name":"plot_expression","arguments":{"expr":"sin("}}
        }))
        .expect("tools/call must respond");
        assert_eq!(bad["result"]["isError"], true);
        assert_eq!(
            bad["result"]["content"][0]["text"],
            "expression ended at column 5; expected a number, variable, function, or '('",
        );

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
    fn creations_save_open_fork_and_enter_the_journal_without_host_files() {
        let expression = std::iter::repeat_n("1+", 170).collect::<String>() + "sin(x)";
        let title = "T".repeat(numinous_core::MAX_META_TEXT_CHARS);
        let author = "A".repeat(numinous_core::MAX_META_TEXT_CHARS);
        let saved = call(
            "save_creation",
            json!({
                "expr": expression,
                "xmin": -2.0,
                "xmax": 3.0,
                "a": 0.75,
                "title": title,
                "author": author,
                "era": "vector",
                "width": 40,
                "height": 12
            }),
        );
        assert_eq!(saved["result"]["isError"], false, "{saved}");
        let capsule = &saved["result"]["structuredContent"];
        assert_eq!(capsule["schema"], "numinous.studio-creation");
        assert_eq!(capsule["schemaVersion"], 1);
        assert_eq!(capsule["action"], "save");
        assert_eq!(capsule["capsuleFormatVersion"], 2);
        assert_eq!(capsule["createdFile"], false);
        assert_eq!(capsule["readHostFile"], false);
        assert_eq!(capsule["containsHostPath"], false);
        assert_eq!(capsule["link"], capsule["journalSubject"]);
        assert!(
            capsule["journalSubject"]
                .as_str()
                .is_some_and(|link| link.chars().count() > 256),
            "the acceptance must cross the old journal subject limit"
        );
        assert_eq!(capsule["preview"]["width"], 40);
        assert_eq!(capsule["preview"]["height"], 12);
        assert!(
            capsule["preview"]["render"]
                .as_str()
                .is_some_and(|render| render.contains('#'))
        );

        let num_file = capsule["numFile"].as_str().expect(".num text");
        let parent = numinous_core::StudioCreation::from_num_file(num_file).expect("reopen save");
        assert_eq!(parent.title(), Some(title.as_str()));
        assert_eq!(parent.author(), Some(author.as_str()));
        assert_eq!(parent.era(), Some(numinous_core::Era::Vector));

        let opened = call("open_creation", json!({"capsule": num_file}));
        assert_eq!(opened["result"]["isError"], false, "{opened}");
        assert_eq!(
            opened["result"]["structuredContent"]["numFile"],
            capsule["numFile"]
        );
        assert_eq!(
            opened["result"]["structuredContent"]["link"],
            capsule["link"]
        );
        let opened_link = call("open_creation", json!({"capsule": capsule["link"]}));
        assert_eq!(opened_link["result"]["isError"], false, "{opened_link}");
        assert_eq!(
            opened_link["result"]["structuredContent"]["numFile"],
            capsule["numFile"]
        );

        let forked = call(
            "fork_creation",
            json!({
                "parent": num_file,
                "expr": "sin(a*x)+0.1",
                "title": "Second Wave",
                "author": "Next Hand"
            }),
        );
        assert_eq!(forked["result"]["isError"], false, "{forked}");
        let child_data = &forked["result"]["structuredContent"];
        assert_eq!(child_data["action"], "fork");
        assert_eq!(child_data["parentLink"], capsule["link"]);
        assert_eq!(child_data["descends"], capsule["link"]);
        let child = numinous_core::StudioCreation::from_num_file(
            child_data["numFile"].as_str().expect("child .num"),
        )
        .expect("reopen child");
        assert_eq!(child.source(), "sin(a*x)+0.1");
        assert_eq!(child.title(), Some("Second Wave"));
        assert_eq!(child.author(), Some("Next Hand"));
        assert_eq!(child.era(), Some(numinous_core::Era::Vector));
        assert_eq!(child.descends(), Some(parent.to_link().as_str()));

        let recorded = call(
            "record_journal",
            json!({
                "kind": "creation",
                "subject": capsule["journalSubject"],
                "text": "Named and signed in the Studio."
            }),
        );
        assert_eq!(recorded["result"]["isError"], false, "{recorded}");
        let journal = call("read_journal", json!({"limit": 100}));
        assert!(
            journal["result"]["structuredContent"]["entries"]
                .as_array()
                .is_some_and(|entries| entries.iter().any(|entry| {
                    entry["kind"] == "creation" && entry["subject"] == capsule["journalSubject"]
                })),
            "the exact capsule link must survive the journal round trip"
        );

        let valid_path = std::env::temp_dir().join(format!(
            "numinous_mcp_portable_capsule_inert_{}.num",
            std::process::id()
        ));
        std::fs::write(&valid_path, parent.to_num_file()).expect("write valid path target");
        let path_shaped = call(
            "open_creation",
            json!({"capsule": valid_path.to_string_lossy()}),
        );
        std::fs::remove_file(valid_path).expect("remove valid path target");
        assert_eq!(path_shaped["result"]["isError"], true);
        assert!(
            tool_error_text(&path_shaped).contains("not a Numinous Studio .num file"),
            "a path stays inert data: {path_shaped}"
        );
    }

    #[test]
    fn refused_capsules_do_not_count_as_creation_play() {
        let journey = super::test_state_path("refused-creation-progress");
        let response = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":1,"method":"tools/call",
                "params":{"name":"save_creation","arguments":{"expr":"ln(-1)"}}
            }),
            &journey,
        )
        .expect("response");
        assert_eq!(response["result"]["isError"], true);
        assert!(!journey.exists(), "a refused save is not a play");
    }

    #[test]
    fn formula_jam_discovery_lists_recipes_and_walks_the_bank() {
        let listed = call("plot_expression", json!({"list_recipes": true}));
        let content = &listed["result"]["structuredContent"];
        assert_eq!(content["discovery"], "list");
        assert_eq!(content["recipeCount"], numinous_core::studio_recipe_count());
        assert_eq!(
            content["recipes"].as_array().expect("bank").len(),
            numinous_core::studio_recipe_count()
        );

        let recipe = call("plot_expression", json!({"recipe": 0}));
        assert_eq!(recipe["result"]["structuredContent"]["discovery"], "recipe");
        assert_eq!(
            recipe["result"]["structuredContent"]["expression"],
            numinous_core::studio_recipe(0)
        );
        assert!(
            recipe["result"]["structuredContent"]["plot"]
                .as_str()
                .is_some_and(|p| p.contains('#') || p.contains('*') || p.contains('.'))
        );

        let random = call("plot_expression", json!({"seed": 7}));
        assert_eq!(random["result"]["structuredContent"]["discovery"], "random");
        assert_eq!(
            random["result"]["structuredContent"]["expression"],
            numinous_core::studio_recipe(7)
        );

        let auto = call("plot_expression", json!({"seed": 3, "auto_step": 2}));
        assert_eq!(auto["result"]["structuredContent"]["discovery"], "auto");
        assert_eq!(
            auto["result"]["structuredContent"]["expression"],
            numinous_core::studio_auto_recipe(3, 2)
        );

        let conflict = call("plot_expression", json!({"expr": "x", "recipe": 1}));
        assert_eq!(conflict["result"]["isError"], true);
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

        // A melody a mind without ears can hear the shape of: every note after
        // the first carries the step taken to reach it.
        let structured = &resp["result"]["structuredContent"];
        let notes = structured["notes"].as_array().expect("notes");
        let steps = structured["steps"].as_array().expect("steps");
        assert_eq!(notes.len(), 8);
        assert_eq!(steps.len(), notes.len() - 1);

        // One note shape across this face. `listen_room` publishes notes under
        // these names, and the tactile cohort reads them from both tools; a
        // second spelling makes the same idea parse twice and silently reads
        // as a melody with no notes at all.
        for note in notes {
            for field in [
                "index",
                "frequency_hz",
                "start_seconds",
                "duration_seconds",
                "amplitude",
            ] {
                assert!(
                    !note[field].is_null(),
                    "a sung note is missing the shared field {field}: {note}"
                );
            }
        }
        for step in steps {
            assert!(step["cents"].as_f64().expect("a measured size") >= 0.0);
            assert!(
                ["up", "down", "level"].contains(&step["direction"].as_str().expect("direction")),
                "{step}"
            );
        }

        // y = x rises steadily, so every step is up, and the ratio is offered
        // only where a simple one explains the step rather than everywhere.
        assert!(
            steps.iter().all(|step| step["direction"] == "up"),
            "{steps:?}"
        );
        assert!(
            steps.iter().any(|step| step["ratio"].is_null()),
            "every step found a ratio, which means the search is answering \
             rather than the music: {steps:?}"
        );
    }

    #[test]
    fn a_melody_can_arrive_as_sound_and_not_only_as_notation() {
        // Six packaged playtests ended on "I still cannot hear the two hills."
        // The notation kept getting better and the sentence kept coming back,
        // because the answer was never another column. A mind down a pipe can
        // be handed audio, so it is handed audio.
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":44,"method":"tools/call",
            "params":{"name":"sing_expression","arguments":{
                "expr":"sin(x)+0.4*sin(3*x)","notes":16,"audio":true
            }}
        }))
        .expect("tools/call must respond");
        let content = resp["result"]["content"].as_array().expect("content");
        // The sound arrives beside the reading, never instead of it, so a
        // client that cannot play audio loses nothing it had before.
        assert_eq!(content[0]["type"], "text");
        let audio = content
            .iter()
            .find(|block| block["type"] == "audio")
            .expect("an audio block");
        assert_eq!(audio["mimeType"], "audio/wav");
        let payload = audio["data"].as_str().expect("payload");
        assert!(
            payload.starts_with("UklGRg") || payload.starts_with("UklGR"),
            "the payload does not begin where a RIFF file begins"
        );
        // What was sent is described, so a caller knows what it holds without
        // decoding a megabyte to find out.
        let described = &resp["result"]["structuredContent"]["audio"];
        assert_eq!(described["mimeType"], "audio/wav");
        assert_eq!(described["channels"], 1);
        assert_eq!(described["bitsPerSample"], 16);
        assert_eq!(
            described["sampleRate"],
            crate::audible::WIRE_SAMPLE_RATE,
            "the described rate has to be the rate in the file"
        );
        assert_eq!(
            described["encodedBytes"].as_u64(),
            Some(payload.len() as u64)
        );
        // The seconds claimed are the seconds sent: sixteen bits, one channel,
        // one sample rate, so the arithmetic is checkable from here.
        let bytes = payload.len() as f64 * 3.0 / 4.0;
        let seconds = (bytes - 44.0) / f64::from(crate::audible::WIRE_SAMPLE_RATE) / 2.0;
        let claimed = described["durationSeconds"].as_f64().expect("seconds");
        assert!(
            (seconds - claimed).abs() < 0.1,
            "the file is {seconds:.2}s but the reply claims {claimed:.2}s"
        );
    }

    #[test]
    fn a_room_can_be_heard_and_asking_to_hear_stays_opt_in() {
        // The same verb on the other sound tool, and the default stays quiet:
        // a caller who never asks for audio never pays for it.
        let heard = handle_request(&json!({
            "jsonrpc":"2.0","id":45,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"times-tables","t":0.375,"audio":true}}
        }))
        .expect("tools/call must respond");
        assert!(
            heard["result"]["content"]
                .as_array()
                .expect("content")
                .iter()
                .any(|block| block["type"] == "audio"),
            "a room asked to be heard returned no sound"
        );
        for arguments in [
            json!({"id":"times-tables","t":0.375}),
            json!({"id":"times-tables","t":0.375,"audio":false}),
        ] {
            let quiet = handle_request(&json!({
                "jsonrpc":"2.0","id":46,"method":"tools/call",
                "params":{"name":"listen_room","arguments":arguments}
            }))
            .expect("tools/call must respond");
            assert!(
                quiet["result"]["content"]
                    .as_array()
                    .expect("content")
                    .iter()
                    .all(|block| block["type"] == "text"),
                "sound arrived unasked for"
            );
            assert!(quiet["result"]["structuredContent"]["audio"].is_null());
        }
    }

    #[test]
    fn a_function_with_no_finite_samples_is_refused_not_sung_as_silence() {
        // Zero notes reported as a successful 0.1 second melody is the singing
        // twin of "nothing to plot". The terminal face refuses it; this face
        // must refuse it the same way rather than sing silence.
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":43,"method":"tools/call",
            "params":{"name":"sing_expression","arguments":{"expr":"sqrt(0-1)","notes":8}}
        }))
        .expect("tools/call must respond");
        assert_eq!(resp["result"]["isError"], true);
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Nothing to sing"), "got: {text}");
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
            text.contains("2 notes"),
            "the times-tables default voice has two notes"
        );
        assert!(text.contains("Ambient motif:"), "got: {text}");
        assert!(text.contains("Mathematical sonification:"), "got: {text}");
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
        let sound = &tuned["result"]["structuredContent"];
        assert_eq!(sound["motif"]["key"], "G visible fifth");
        assert_eq!(sound["ambient_bed"]["schema"], "numinous.room-bed.events");
        assert_eq!(sound["ambient_bed"]["schema_version"], 1);
        assert_eq!(
            sound["ambient_bed"]["source_sample_rate_hz"],
            numinous_core::ROOM_BED_SOURCE_RATE
        );
        assert_eq!(sound["ambient_bed"]["channels"], 2);
        assert_eq!(sound["ambient_bed"]["events_included"], false);
        assert!(sound["ambient_bed"].get("events").is_none());
        assert!(sound["ambient_bed"].get("signal_metrics").is_none());
        let spectrum = &sound["ambient_bed"]["spectrum"];
        assert_eq!(spectrum["schema"], "numinous.spectrum.bands");
        assert_eq!(spectrum["band_count"], numinous_core::BAND_COUNT);
        assert_eq!(
            spectrum["levels"].as_array().map(|a| a.len()),
            Some(numinous_core::BAND_COUNT)
        );
        assert!(
            sound["notes"]
                .as_array()
                .is_some_and(|notes| !notes.is_empty()),
            "the specialized sonification is separately named"
        );
        assert_eq!(sound["sound_roles"]["ambient_motif"]["field"], "motif");
        assert_eq!(
            sound["sound_roles"]["ambient_arrangement"]["field"],
            "ambient_bed"
        );
        assert_eq!(
            sound["sound_roles"]["mathematical_sonification"]["field"],
            "notes"
        );

        let unvaried = handle_request(&json!({
            "jsonrpc":"2.0","id":300,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"times-tables","t":0.5}}
        }))
        .expect("tools/call must respond");
        let unvaried_text = unvaried["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        let varied = handle_request(&json!({
            "jsonrpc":"2.0","id":301,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"times-tables","t":0.5,"variation":42}}
        }))
        .expect("tools/call must respond");
        let varied_text = varied["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert_ne!(
            unvaried_text, varied_text,
            "listen_room must honor variation away from the shared opening"
        );
    }

    #[test]
    fn listen_room_projects_every_bed_event_without_binary_transport() {
        fn assert_no_binary_transport_fields(value: &Value) {
            match value {
                Value::Object(object) => {
                    for (key, child) in object {
                        assert!(
                            !matches!(
                                key.as_str(),
                                "base64" | "bytes" | "file" | "path" | "pcm" | "samples" | "url"
                            ),
                            "structured room-bed evidence must not transport binary data or local references: {key}"
                        );
                        assert_no_binary_transport_fields(child);
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        assert_no_binary_transport_fields(item);
                    }
                }
                _ => {}
            }
        }

        assert_eq!(
            super::listen_room_tool(&json!({"id":"times-tables","ambient_detail":"raw"}))["isError"],
            true,
            "the domain boundary rejects unknown detail even when called without schema dispatch"
        );
        for invalid in [
            json!({}),
            json!({"id":"times-tables","t":1.0}),
            json!({"id":"no-such-room"}),
        ] {
            assert_eq!(
                super::listen_room_tool(&invalid)["isError"],
                true,
                "the domain boundary remains total without schema dispatch"
            );
        }
        let direct_gesture = super::listen_room_tool(&json!({
            "id":"times-tables",
            "gesture":[
                {"kind":"down","x":0.375,"y":0.5,"t":0.1},
                {"kind":"up","x":0.375,"y":0.5,"t":0.2}
            ]
        }));
        assert_eq!(direct_gesture["isError"], false);
        assert!(direct_gesture["structuredContent"]["gesture"].is_array());

        for room in numinous_core::all_rooms() {
            let response = call(
                "listen_room",
                json!({"id":room.meta().id,"ambient_detail":"events"}),
            );
            assert_eq!(response["result"]["isError"], false, "{}", room.meta().id);
            let structured = &response["result"]["structuredContent"];
            let bed = &structured["ambient_bed"];
            let motif = room.motif().expect("catalog rooms have ambient motifs");
            let arrangement = motif.arrangement();
            let events = bed["events"].as_array().expect("complete bed events");

            assert_eq!(bed["events_included"], true, "{}", room.meta().id);
            assert_eq!(bed["steps"], arrangement.steps, "{}", room.meta().id);
            assert_eq!(
                bed["step_seconds"],
                arrangement.step_seconds,
                "{}",
                room.meta().id
            );
            assert_eq!(
                bed["duration_seconds"],
                arrangement.steps as f64 * f64::from(arrangement.step_seconds),
                "{}",
                room.meta().id
            );
            assert_eq!(
                bed["event_count"],
                arrangement.notes.len(),
                "{}",
                room.meta().id
            );
            assert_eq!(events.len(), arrangement.notes.len(), "{}", room.meta().id);
            assert!(
                events.len() <= numinous_core::MAX_ROOM_BED_EVENTS,
                "{}",
                room.meta().id
            );
            for (index, (event, note)) in events.iter().zip(&arrangement.notes).enumerate() {
                assert_eq!(event["index"], index + 1, "{}", room.meta().id);
                assert_eq!(event["frequency_hz"], note.frequency, "{}", room.meta().id);
                assert_eq!(event["start_step"], note.start_step, "{}", room.meta().id);
                assert_eq!(event["step_count"], note.step_count, "{}", room.meta().id);
                assert_eq!(
                    event["start_seconds"],
                    note.start_step as f64 * f64::from(arrangement.step_seconds),
                    "{}",
                    room.meta().id
                );
                assert_eq!(
                    event["duration_seconds"],
                    note.step_count as f64 * f64::from(arrangement.step_seconds),
                    "{}",
                    room.meta().id
                );
                assert_eq!(event["voice"], note.voice.id(), "{}", room.meta().id);
                assert_eq!(event["level"], note.level, "{}", room.meta().id);
                assert_eq!(event["pan"], note.pan, "{}", room.meta().id);
            }
            let expected_frames =
                (arrangement.seconds() * numinous_core::ROOM_BED_SOURCE_RATE as f32) as usize;
            assert_eq!(
                bed["signal_metrics"]["frame_count"],
                expected_frames,
                "{}",
                room.meta().id
            );
            assert_eq!(bed["signal_metrics"]["non_finite_samples"], 0);
            assert_eq!(bed["signal_metrics"]["clipped_samples"], 0);
            assert!(
                bed["signal_metrics"]["interpretation"]
                    .as_str()
                    .is_some_and(|text| text.contains("not a pleasantness score"))
            );
            assert_no_binary_transport_fields(structured);
            assert!(
                serde_json::to_vec(structured)
                    .expect("serialize structured room-bed evidence")
                    .len()
                    < 64 * 1024,
                "{} room-bed evidence exceeds the protocol budget",
                room.meta().id
            );
        }

        let full = call(
            "listen_room",
            json!({"id":"times-tables","ambient_detail":"events"}),
        );
        let compact = call(
            "listen_room",
            json!({"id":"times-tables","ambient_detail":"events","response_mode":"compact"}),
        );
        assert_eq!(
            full["result"]["structuredContent"], compact["result"]["structuredContent"],
            "compact presentation must preserve complete room-bed evidence"
        );

        let varied = call(
            "listen_room",
            json!({"id":"times-tables","variation":42,"ambient_detail":"events"}),
        );
        let varied_room = numinous_core::all_rooms_with(42)
            .into_iter()
            .find(|room| room.meta().id == "times-tables")
            .expect("varied Times Tables room");
        let varied_arrangement = varied_room
            .motif()
            .expect("varied room motif")
            .arrangement();
        let varied_bed = &varied["result"]["structuredContent"]["ambient_bed"];
        assert_eq!(varied["result"]["structuredContent"]["variation"], 42);
        assert_eq!(varied_bed["event_count"], varied_arrangement.notes.len());
        for (event, note) in varied_bed["events"]
            .as_array()
            .expect("varied bed events")
            .iter()
            .zip(&varied_arrangement.notes)
        {
            assert_eq!(event["frequency_hz"], note.frequency);
            assert_eq!(event["start_step"], note.start_step);
            assert_eq!(event["step_count"], note.step_count);
            assert_eq!(event["voice"], note.voice.id());
            assert_eq!(event["level"], note.level);
            assert_eq!(event["pan"], note.pan);
        }
    }

    #[test]
    fn listen_room_uses_the_same_accepted_input_as_the_times_tables_dial() {
        let resting = call("listen_room", json!({"id":"times-tables","t":0.0}));
        let landed = call(
            "listen_room",
            json!({"id":"times-tables","t":0.0,"pokes":[[0.375,0.5]]}),
        );
        let resting_notes = resting["result"]["structuredContent"]["notes"]
            .as_array()
            .expect("resting notes");
        let landed_notes = landed["result"]["structuredContent"]["notes"]
            .as_array()
            .expect("landed notes");
        let ratio = |notes: &[Value]| {
            notes[1]["frequency_hz"].as_f64().expect("upper voice")
                / notes[0]["frequency_hz"].as_f64().expect("root voice")
        };

        assert!((ratio(resting_notes) - 2.0).abs() < 1e-6);
        assert!((ratio(landed_notes) - 1.25).abs() < 1e-6);
        assert_eq!(
            landed["result"]["structuredContent"]["pokes"],
            json!([[0.375, 0.5]])
        );
        assert!(landed["result"]["structuredContent"]["gesture"].is_null());
    }

    #[test]
    fn listen_room_uses_the_same_selected_galton_probability_as_the_board() {
        let listen = |x| {
            call(
                "listen_room",
                json!({"id":"galton-board","t":0.4,"pokes":[[x,0.5]]}),
            )
        };
        let left = listen(0.1);
        let fair = listen(0.5);
        let right = listen(0.9);
        fn notes(response: &Value) -> &[Value] {
            response["result"]["structuredContent"]["notes"]
                .as_array()
                .expect("mathematical notes")
        }
        let ratio = |notes: &[Value]| {
            notes[1]["frequency_hz"].as_f64().expect("upper voice")
                / notes[0]["frequency_hz"].as_f64().expect("root voice")
        };
        let left_notes = notes(&left);
        let fair_notes = notes(&fair);
        let right_notes = notes(&right);

        assert!(left_notes[0]["frequency_hz"].as_f64() < fair_notes[0]["frequency_hz"].as_f64());
        assert!(fair_notes[0]["frequency_hz"].as_f64() < right_notes[0]["frequency_hz"].as_f64());
        assert!((ratio(left_notes) - 7.0 / 3.0).abs() < 1e-6);
        assert!((ratio(fair_notes) - 1.0).abs() < 1e-6);
        assert!((ratio(right_notes) - 7.0 / 3.0).abs() < 1e-6);
        assert_eq!(
            fair["result"]["structuredContent"]["pokes"],
            json!([[0.5, 0.5]])
        );
    }

    #[test]
    fn listen_room_replays_the_pendulum_pin_and_fling() {
        let compact = call(
            "listen_room",
            json!({"id":"double-pendulum","t":0.4,"pokes":[[0.7,0.25]]}),
        );
        let held = call(
            "listen_room",
            json!({"id":"double-pendulum","t":0.4,"gesture":[
                {"kind":"down","x":0.7,"y":0.25,"t":0.4}
            ]}),
        );
        let flung = call(
            "listen_room",
            json!({"id":"double-pendulum","t":0.35,"gesture":[
                {"kind":"down","x":0.3,"y":0.5,"t":0.10},
                {"kind":"move","x":0.3,"y":0.5,"t":0.147},
                {"kind":"up","x":0.6,"y":0.5,"t":0.15}
            ]}),
        );
        let wrapped = call(
            "listen_room",
            json!({"id":"double-pendulum","t":0.05,"gesture":[
                {"kind":"move","x":0.3,"y":0.5,"t":0.99},
                {"kind":"up","x":0.6,"y":0.5,"t":0.01}
            ]}),
        );
        fn notes(response: &Value) -> &[Value] {
            response["result"]["structuredContent"]["notes"]
                .as_array()
                .expect("mathematical notes")
        }

        assert_eq!(notes(&compact), notes(&held));
        assert!(
            notes(&flung)[0]["amplitude"].as_f64().expect("fling gain")
                > notes(&held)[0]["amplitude"].as_f64().expect("held gain")
        );
        assert!(
            notes(&wrapped)[0]["amplitude"]
                .as_f64()
                .expect("phase-wrapped fling gain")
                > notes(&held)[0]["amplitude"].as_f64().expect("held gain")
        );
        assert_eq!(
            held["result"]["structuredContent"]["gesture"][0]["kind"],
            "down"
        );
    }

    #[test]
    fn listen_room_rejects_unsafe_or_ambiguous_input() {
        for (arguments, expected) in [
            (
                json!({"id":"times-tables","pokes":[[1.2,0.5]]}),
                "at most 1",
            ),
            (
                json!({"id":"times-tables","pokes":[[0.5,0.5]],"gesture":[
                    {"kind":"down","x":0.5,"y":0.5,"t":0.2}
                ]}),
                "either 'pokes'",
            ),
        ] {
            let response = call("listen_room", arguments);
            let text = tool_error_text(&response);
            assert!(
                text.contains(expected),
                "expected {expected:?}, got {text:?}"
            );
        }
    }

    #[test]
    fn invalid_tools_do_not_record_progress() {
        let file = super::test_state_path("invalid-progress");
        let scores = super::scores_path();
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(&scores);
        for (id, name, arguments) in [
            (401, "run_sim", json!({"id": "no-such-sim"})),
            (402, "plot_expression", json!({"expr": "sin("})),
            (403, "sing_expression", json!({"expr": "sin("})),
            (
                404,
                "run_sim",
                json!({"id":"tribbles","params":{"breeding-rate":"fast"}}),
            ),
            (
                405,
                "run_sim",
                json!({"id":"tribbles","params":{"bogus":1.0}}),
            ),
            (
                406,
                "run_sim",
                json!({"id":"tribbles","params":{"breeding-rate":100.0}}),
            ),
            (407, "listen_room", json!({"id":"goldbach","t":9.0})),
            (408, "munch_arcade", json!({"actions":["not-an-action"]})),
            (409, "nim", json!({"moves":[[]]})),
            (410, "hackenbush", json!({"moves":[[]]})),
            (411, "munch", json!({"bites":[0]})),
            (412, "munch", json!({"bites":[-1]})),
            (
                413,
                "crack",
                json!({"digits": numinous_core::MAX_CODE_DIGITS + 1}),
            ),
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
        let too_many_arcade_actions =
            vec!["e"; numinous_core::munch_arcade::MAX_REPLAY_ACTIONS + 1];
        let oversized_arcade = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":414,"method":"tools/call",
                "params":{
                    "name":"munch_arcade",
                    "arguments":{"actions":too_many_arcade_actions}
                }
            }),
            &file,
        )
        .expect("oversized arcade replay must respond");
        assert_eq!(oversized_arcade["result"]["isError"], true);
        assert!(
            tool_error_text(&oversized_arcade).contains("at most 4096 items"),
            "the rejection must name the replay budget"
        );
        let journey = std::fs::read_to_string(&file)
            .map(|text| numinous_core::Journey::from_text(&text))
            .unwrap_or_default();
        assert_eq!(journey.plays, 0);
        assert!(journey.visited.is_empty());
        assert!(!scores.exists(), "invalid actions must not post scores");
        let _ = std::fs::remove_file(&file);

        let control_file = super::test_state_path("valid-arcade-progress");
        let control = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":413,"method":"tools/call",
                "params":{"name":"munch_arcade","arguments":{"actions":["RiGhT","E"]}}
            }),
            &control_file,
        )
        .expect("valid mixed-case aliases must respond");
        assert_ne!(control["result"]["isError"], true);
        assert_eq!(
            std::fs::read_to_string(&control_file)
                .map(|text| numinous_core::Journey::from_text(&text).plays)
                .unwrap_or_default(),
            1
        );
        let _ = std::fs::remove_file(control_file);
        let _ = std::fs::remove_file(scores);
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
    fn the_veil_opens_the_same_rooms_on_this_face_as_on_the_terminal() {
        // The gate was shared; the door was not. A learner the terminal
        // admits to an unlisted room was told here that the room does not
        // exist, so one player with one standing got two answers depending
        // on which face they asked through.
        let mut journey = numinous_core::Journey::default();
        journey.visit("a");
        journey.wins = 7;
        assert!(numinous_core::behind_the_veil(&journey));

        let hidden = numinous_core::hidden_room_by_id("tetractys").expect("an unlisted room");
        let id = hidden.meta().id;

        let described = super::describe_room_tool_for_journey(&json!({ "id": id }), &journey);
        assert_eq!(described["isError"], false, "{described}");
        let played = super::play_room_tool_for_journey(
            &json!({ "id": id, "width": 24, "height": 12 }),
            &journey,
        );
        assert_eq!(played["isError"], false, "{played}");
        journey.visit(id);
        let revealed = super::reveal_room_tool_for_journey(&json!({ "id": id }), &journey);
        assert_eq!(revealed["isError"], false, "{revealed}");

        // Outside the veil the room stays unlisted on this face too: the
        // fix opens the same door, not a wider one.
        let outsider = numinous_core::Journey::default();
        assert!(!numinous_core::behind_the_veil(&outsider));
        let refused = super::play_room_tool_for_journey(
            &json!({ "id": id, "width": 24, "height": 12 }),
            &outsider,
        );
        assert_eq!(refused["isError"], true, "{refused}");
    }

    #[test]
    fn a_learner_of_fifteen_sparks_is_inside_the_veil_on_this_face_too() {
        // The drift this retires: this face demanded 28 sparks for the deep
        // whispers while the terminal admitted at rank Mathematikos (10
        // sparks, the rule the sayings document), so a listener with 15
        // sparks heard the saying on one face and was refused on the other.
        // Both faces now call core's behind_the_veil.
        let mut journey = numinous_core::Journey::default();
        journey.visit("a");
        journey.wins = 7;
        assert_eq!(journey.sparks(), 15, "the fixture drifted");
        let reply = super::describe_room_tool_for_journey(&json!({ "id": "curtain" }), &journey);
        let text = reply["content"][0]["text"].as_str().unwrap_or_default();
        assert!(
            text.contains("veil"),
            "the deep whisper must answer a learner: {reply}"
        );
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
        let seed = 0;
        let round = 0;
        let choice_count = 3;
        let three = numinous_core::build_round_sized(seed, round, 54, 22, choice_count);
        let four = numinous_core::build_round_sized(seed, round, 54, 22, 4);
        // Choice count reshapes the dealt hand; letters alone can collide by chance.
        let three_titles: Vec<_> = three.choices.iter().map(|c| c.title).collect();
        let four_titles: Vec<_> = four.choices.iter().map(|c| c.title).collect();
        assert_ne!(
            three_titles, four_titles,
            "choice count is part of the replay identity"
        );
        let expected = three.answer;
        let puzzle = handle_request(&json!({
            "jsonrpc":"2.0","id":23,"method":"tools/call",
            "params":{"name":"quiz","arguments":{
                "seed":seed,"round":round,"choices":choice_count
            }}
        }))
        .expect("tools/call must respond");
        let puzzle_text = puzzle["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(puzzle_text.contains("Guess the shape"));
        assert!(puzzle_text.contains("choices 3, and your guess letter"));
        assert_eq!(
            puzzle["result"]["structuredContent"]["choiceCount"],
            choice_count
        );
        assert_eq!(
            puzzle["result"]["structuredContent"]["choices"]
                .as_array()
                .map(Vec::len),
            Some(choice_count)
        );

        let compact = call(
            "quiz",
            json!({
                "seed":seed,"round":round,"choices":choice_count,
                "response_mode":"compact"
            }),
        );
        assert!(
            compact["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("choices 3, and guess")
        );
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":24,"method":"tools/call",
            "params":{"name":"quiz","arguments":{
                "seed":seed,"round":round,"choices":choice_count,
                "guess":expected.to_string()
            }}
        }))
        .expect("tools/call must respond");
        assert!(
            graded["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("The answer was")
        );
        assert_eq!(graded["result"]["structuredContent"]["correct"], true);
        assert_eq!(
            graded["result"]["structuredContent"]["choiceCount"],
            choice_count
        );
    }

    #[test]
    fn reveal_room_returns_the_insight() {
        let mut journey = numinous_core::Journey::default();
        journey.visit("times-tables");
        journey.consolidate("times-tables");
        let result = super::reveal_room_tool_for_journey(&json!({"id":"times-tables"}), &journey);
        let text = result["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("Mandelbrot"));
        assert!(
            text.contains("THE CONCEPT:"),
            "flagship rooms carry an optional concept before the reveal"
        );
        assert!(
            result["structuredContent"]["concept"]
                .as_str()
                .unwrap_or_default()
                .starts_with("THE CONCEPT:"),
            "structured concept matches the text door"
        );
        // Fresh journey is below the first deep cut: no citation yet.
        assert!(!text.contains("See also:"));
        assert!(
            result["structuredContent"]["citation"].is_null()
                || result["structuredContent"].get("citation").is_none()
        );
    }

    #[test]
    fn reveal_room_requires_play_and_engineered_consolidation() {
        let fresh = numinous_core::Journey::default();
        let ordinary = super::reveal_room_tool_for_journey(&json!({"id":"mandelbrot"}), &fresh);
        assert_eq!(ordinary["isError"], true);

        let mut played = fresh.clone();
        played.visit("mandelbrot");
        assert_eq!(
            super::reveal_room_tool_for_journey(&json!({"id":"mandelbrot"}), &played)["isError"],
            false
        );
        played.visit("times-tables");
        let held = super::reveal_room_tool_for_journey(&json!({"id":"times-tables"}), &played);
        assert_eq!(held["isError"], true);
        assert!(
            held["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("aha_summon"))
        );
    }

    #[test]
    fn reveal_room_unlocks_citation_with_the_first_deep_cut() {
        let journey = std::env::temp_dir().join(format!(
            "numinous-mcp-reveal-cite-{}.txt",
            std::process::id()
        ));
        let mut at_cut = numinous_core::Journey {
            plays: 20,
            ..Default::default()
        };
        at_cut.visit("mandelbrot");
        // Level 5 needs T(4)=10 sparks; twenty plays clears the first cut.
        assert!(at_cut.level() >= numinous_core::CUT_LEVELS[0]);
        std::fs::write(&journey, at_cut.to_text()).expect("journey");
        let resp = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":16,"method":"tools/call",
                "params":{"name":"reveal_room","arguments":{"id":"mandelbrot"}}
            }),
            &journey,
        )
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("See also:"), "citation in text: {text}");
        assert!(
            resp["result"]["structuredContent"]["citation"]
                .as_str()
                .unwrap_or_default()
                .contains("See also:"),
            "citation in structured content"
        );
        let _ = std::fs::remove_file(journey);
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
    fn times_tables_open_keeps_dial_invite_without_hand() {
        // Ambient open phase is closed K=2. That must not auto-prime the place
        // wager and replace DRAG:DIAL with WHERE? before any hand arrives.
        let open = call(
            "play_room",
            json!({"id":"times-tables","t":0.0,"width":40,"height":20}),
        );
        let content = &open["result"]["structuredContent"];
        assert_eq!(content["engineeredAha"]["beat"], "explore");
        assert_eq!(
            content["status"],
            "DRAG:DIAL  K 2.00  CLOSED  1 LOBE  TARGET 4"
        );
        assert!(
            !content["status"]
                .as_str()
                .unwrap_or_default()
                .contains("WHERE?"),
            "aha prime chrome must not steal first contact: {}",
            content["status"]
        );
    }

    #[test]
    fn times_tables_k5_goal_is_true_but_reveal_waits_for_summon() {
        let held = call(
            "play_room",
            json!({"id":"times-tables","t":0.375,"width":40,"height":20}),
        );
        let held_content = &held["result"]["structuredContent"];
        assert_eq!(held_content["goal"], "LAND ON EXACTLY 4 LOBES");
        assert_eq!(held_content["goalMet"], true);
        assert!(held_content["reveal"].is_null());
        assert!(
            !held["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Reveal:")
        );
        assert_eq!(held_content["engineeredAha"]["kind"], "place");
        assert_eq!(held_content["engineeredAha"]["beat"], "withheld");
        assert_eq!(held_content["engineeredAha"]["allowReveal"], false);
        assert!(held_content["engineeredAha"]["earn"].is_null());
        assert!(
            held_content["status"]
                .as_str()
                .unwrap_or_default()
                .contains("FOUND"),
            "goal and status must agree: {}",
            held_content["status"]
        );

        let earned_arguments = json!({
            "id":"times-tables",
            "t":0.81,
            "width":40,
            "height":20,
            "variation":42,
            "pokes":[[0.375,0.5]]
        });
        let earned = call("play_room", earned_arguments.clone());
        let earned_content = &earned["result"]["structuredContent"];
        assert_eq!(earned_content["variation"], 42);
        assert_eq!(earned_content["status"], "K 5.00  CLOSED  4 LOBES  FOUND");
        assert!(earned_content["engineeredAha"]["earn"].is_null());
        assert_eq!(earned_content["goalMet"], true);
        assert!(earned_content["reveal"].is_null());
        assert!(
            !earned["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Reveal:")
        );

        let mut consolidated_args = earned_arguments.clone();
        consolidated_args["aha_summon"] = json!(true);
        let consolidated = call(
            "play_room",
            with_response_mode(consolidated_args, "compact"),
        );
        let consolidated_content = &consolidated["result"]["structuredContent"];
        assert_eq!(
            consolidated_content["engineeredAha"]["beat"],
            "consolidated"
        );
        assert_eq!(consolidated_content["engineeredAha"]["earn"], "four-lobes");
        assert!(
            consolidated_content["reveal"]
                .as_str()
                .is_some_and(|reveal| reveal.contains("Mandelbrot"))
        );
        let compact = call("play_room", with_response_mode(earned_arguments, "compact"));
        assert_eq!(
            compact["result"]["structuredContent"],
            earned["result"]["structuredContent"]
        );
    }

    #[test]
    fn a_place_wager_at_the_four_lobe_close_is_kept_and_graded() {
        // Reported from packaged play: landing four lobes and naming a place in
        // the same call showed the four-lobe close back and dropped the named
        // wager, so the caller could not tell whether its own call landed. The
        // named place is the hypothesis consolidation grades, so it owns the
        // visit; the other five staged rooms already worked this way.
        let arguments = json!({
            "id":"times-tables",
            "t":0.375,
            "place_wager":"mandelbrot",
            "width":40,
            "height":16
        });
        let held = call("play_room", arguments.clone());
        let held_aha = &held["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(held_aha["beat"], "withheld");
        assert_eq!(held_aha["wager"], "mandelbrot");
        assert_eq!(held["result"]["structuredContent"]["goalMet"], true);
        // Withholding is unchanged: the call is visible, the answer is not.
        for held_back in ["earn", "reveal", "truth", "punchline", "graded"] {
            assert!(
                held_aha[held_back].is_null(),
                "{held_back} leaked at withheld: {held_aha}"
            );
        }

        let mut summoned_arguments = arguments.clone();
        summoned_arguments["aha_summon"] = json!(true);
        let summoned = call("play_room", summoned_arguments);
        let summoned_aha = &summoned["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(summoned_aha["beat"], "consolidated");
        assert_eq!(summoned_aha["wager"], "mandelbrot");
        assert_eq!(summoned_aha["earn"], "wager:mandelbrot");
        assert!(
            summoned_aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("MANDELBROT")),
            "a kept call is graded at consolidation: {summoned_aha}"
        );

        // A wrong call is kept just as faithfully; grading is what makes the
        // commitment real, and the four-lobe close must not launder a miss.
        let mut missed_arguments = arguments;
        missed_arguments["place_wager"] = json!("nephroid");
        missed_arguments["aha_summon"] = json!(true);
        let missed = call("play_room", missed_arguments);
        let missed_aha = &missed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(missed_aha["wager"], "nephroid");
        assert!(
            missed_aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("NEPHROID")),
            "a missed call is graded too: {missed_aha}"
        );
    }

    /// Each staged room, with the arguments that earn the withheld beat by
    /// running its own experiment rather than by naming a call.
    fn experiment_earn_calls() -> Vec<(&'static str, Value)> {
        let throws: Vec<_> = (0..8)
            .map(|i| json!([0.2 + 0.05 * f64::from(i), 0.5]))
            .collect();
        let releases: Vec<_> = (0..4)
            .flat_map(|i| {
                let t = 0.05 + f64::from(i) * 0.1;
                [
                    json!({"kind":"down","x":0.6,"y":0.3,"t":t}),
                    json!({"kind":"up","x":0.6,"y":0.3,"t":t + 0.02}),
                ]
            })
            .collect();
        let four_pokes = json!([[0.2, 0.5], [0.5, 0.5], [0.8, 0.5], [0.3, 0.5]]);
        vec![
            ("times-tables", json!({"id":"times-tables","t":0.375})),
            (
                "buffon-needle",
                json!({"id":"buffon-needle","pokes":throws}),
            ),
            (
                "galton-board",
                json!({"id":"galton-board","pokes":[[0.5,0.5],[0.5,0.5],[0.5,0.5],[0.5,0.5]]}),
            ),
            (
                "double-pendulum",
                json!({"id":"double-pendulum","gesture":releases}),
            ),
            (
                "kepler-laws",
                json!({"id":"kepler-laws","pokes":[[0.4,0.5],[0.5,0.5],[0.6,0.5],[0.7,0.5]]}),
            ),
            ("parrondo", json!({"id":"parrondo","pokes":four_pokes})),
            (
                "nontransitive",
                json!({"id":"nontransitive","pokes":four_pokes}),
            ),
        ]
    }

    #[test]
    fn a_dwell_reports_what_refused_to_move() {
        // Staying is the verb a packaged playtest asked for: every look was a
        // new stateless call, so returning to one dark point bought nothing.
        // What staying earns is a measurement the player extracted, never a
        // paragraph the room volunteered.
        let stayed = call(
            "play_room",
            json!({
                "id":"lorenz","t":0.5,
                "dwell":[0.1,0.3,0.5,0.7],
                "width":40,"height":20
            }),
        );
        let structured = &stayed["result"]["structuredContent"];
        let dwell = &structured["dwell"];
        assert_eq!(dwell["schema"], "numinous.dwell-evidence");
        assert_eq!(dwell["schemaVersion"], 1);
        assert_eq!(dwell["looks"], 4);
        assert_eq!(dwell["phases"], json!([0.1, 0.3, 0.5, 0.7]));
        let held = &dwell["held"];
        assert_eq!(held["total_cells"], 800);
        // Staying must never become a second way to be told the answer.
        assert!(
            structured["reveal"].is_null(),
            "a dwell lectured: {structured}"
        );
        assert!(
            !stayed["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Reveal:")
        );
        // The counts partition the same cell map they measure.
        let total = held["total_cells"].as_u64().expect("total");
        for field in ["unchanged_cells", "never_ink", "always_ink"] {
            assert!(
                held[field].as_u64().expect(field) <= total,
                "{field} exceeds the cell map: {held}"
            );
        }
        assert!(
            held["never_ink"].as_u64().expect("never_ink")
                <= held["unchanged_cells"].as_u64().expect("unchanged"),
            "a cell blank in every look cannot have changed: {held}"
        );
        assert!(
            held["never_ink_enclosed"].as_u64().expect("enclosed")
                <= held["never_ink"].as_u64().expect("never_ink"),
            "an enclosed hole is a blank cell: {held}"
        );

        // Looking twice at one moment is honest about finding nothing.
        let still = call(
            "play_room",
            json!({"id":"lorenz","t":0.5,"dwell":[0.4,0.4,0.4],"width":40,"height":20}),
        );
        let held = &still["result"]["structuredContent"]["dwell"]["held"];
        assert_eq!(held["unchanged_cells"], held["total_cells"]);
        assert!(held["changed_region"].is_null(), "{held}");
        assert!(
            still["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("nothing moved at all")
        );

        // A dwell is deterministic, like every other observation here.
        let again = call(
            "play_room",
            json!({
                "id":"lorenz","t":0.5,
                "dwell":[0.1,0.3,0.5,0.7],
                "width":40,"height":20
            }),
        );
        assert_eq!(
            again["result"]["structuredContent"]["dwell"],
            stayed["result"]["structuredContent"]["dwell"]
        );
    }

    #[test]
    fn the_longest_stay_works_at_the_size_the_room_drew_itself() {
        // Reported from packaged play: the first stay a player attempted was
        // the longest one the schema names, at the canvas the room had just
        // handed back, and it was refused. Nobody who has not asked for a size
        // knows what size they are being refused for.
        let stayed = call(
            "play_room",
            json!({
                "id":"unlit-room","t":0.5,
                "dwell":[0.05,0.2,0.35,0.5,0.65,0.8,0.9,0.95]
            }),
        );
        assert!(
            stayed["result"]["isError"].as_bool() != Some(true),
            "the longest stay at the default canvas was refused: {stayed}"
        );
        let dwell = &stayed["result"]["structuredContent"]["dwell"];
        assert_eq!(dwell["looks"], MAX_DWELL_LOOKS as u64);
        assert_eq!(
            dwell["held"]["total_cells"],
            DEFAULT_WIDTH * DEFAULT_HEIGHT,
            "the stay measured a canvas the player never asked for"
        );
    }

    #[test]
    fn a_dwell_refuses_a_crowd_a_lone_look_and_an_unpayable_budget() {
        for (arguments, expected) in [
            (json!({"id":"lorenz","dwell":[0.5]}), "at least 2"),
            (
                json!({"id":"lorenz","dwell":[0.1,0.2],"width":200,"height":100}),
                "within 18432 cells",
            ),
            // A refusal that only quotes the cap leaves a caller who never
            // named a size unable to do the arithmetic it is being judged by.
            (
                json!({"id":"lorenz","dwell":[0.1,0.2],"width":200,"height":100}),
                "200 by 100",
            ),
            (json!({"id":"lorenz","dwell":[0.1,1.0]}), "less than 1"),
            (json!({"id":"lorenz","dwell":"soon"}), "must be an array"),
        ] {
            let refused = call("play_room", arguments.clone());
            assert_eq!(refused["result"]["isError"], true, "{arguments}");
            let text = refused["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(
                text.contains(expected),
                "refusal for {arguments} must guide, got: {text}"
            );
        }
    }

    #[test]
    fn landing_a_goal_opens_the_explanation_without_speaking_it() {
        // Reported from packaged play: landing the Smith Chart bead on the r=1
        // ring returned the whole conformal-map lecture in the same reply, so
        // the reward for succeeding was losing the thing you succeeded at. The
        // door promises understanding later and only if you ask.
        let journey = super::test_state_path("goal-does-not-lecture");
        let _ = std::fs::remove_file(&journey);
        let landed = super::handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":900,"method":"tools/call",
                "params":{"name":"play_room","arguments":{
                    "id":"smith-chart","t":0.25,"pokes":[[0.62,0.48]]
                }}
            }),
            &journey,
        )
        .expect("play response");
        let structured = &landed["result"]["structuredContent"];
        assert_eq!(
            structured["goalMet"], true,
            "the poke must still land the goal: {structured}"
        );
        assert!(
            structured["reveal"].is_null(),
            "a landed goal lectured: {}",
            structured["reveal"]
        );
        assert!(
            !landed["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Reveal:"),
            "the landed goal lectured in text: {}",
            landed["result"]["content"][0]["text"]
        );

        // Nothing is lost: having played, the player can now ask.
        let asked = super::handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":901,"method":"tools/call",
                "params":{"name":"reveal_room","arguments":{"id":"smith-chart"}}
            }),
            &journey,
        )
        .expect("reveal response");
        assert_eq!(
            asked["result"]["isError"], false,
            "playing must open the ask: {asked}"
        );
        assert!(
            asked["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("Gamma"),
            "the explanation is still there for the asking: {asked}"
        );
        let _ = std::fs::remove_file(&journey);
    }

    #[test]
    fn an_unearned_call_is_never_claimed_on_the_players_behalf() {
        // The staged rooms turn on one distinction: whether the player
        // committed. Three rooms used to format the experiment path through
        // their CALLED sentence, so a player who never named anything read
        // CALLED EXPERIMENT, or worse CALLED EXPERIMENT AGAINST B. Inventing a
        // commitment is the same betrayal as dropping a real one.
        for (room, mut arguments) in experiment_earn_calls() {
            arguments["width"] = json!(44);
            arguments["height"] = json!(20);
            let reply = call("play_room", arguments.clone());
            let aha = &reply["result"]["structuredContent"]["engineeredAha"];
            assert_eq!(aha["beat"], "withheld", "{room} did not earn: {aha}");
            assert!(
                aha["wager"].is_null(),
                "{room} invented a wager from the experiment path: {aha}"
            );
            let status = aha["status"].as_str().unwrap_or_default();
            assert!(
                !status.contains("CALLED"),
                "{room} claims a call the player never made: {status:?}"
            );
            assert!(
                !status.contains("EXPERIMENT"),
                "{room} leaks its internal earn vocabulary: {status:?}"
            );
        }
    }

    #[test]
    fn a_named_call_still_owns_every_staged_room() {
        // The other half of the same rule: naming a call at the experiment
        // earn must supersede it in all seven rooms, not only the two that
        // were reported.
        let named: [(&str, &str, Value); 7] = [
            ("times-tables", "place_wager", json!("nephroid")),
            ("buffon-needle", "number_wager", json!(3.0)),
            ("galton-board", "bin_wager", json!(3)),
            ("double-pendulum", "ending_wager", json!("drifted")),
            ("kepler-laws", "speed_wager", json!("slower")),
            ("parrondo", "policy_wager", json!("a")),
            ("nontransitive", "counter_wager", json!("c")),
        ];
        let earns = experiment_earn_calls();
        for (room, field, value) in named {
            let mut arguments = earns
                .iter()
                .find(|(id, _)| *id == room)
                .map(|(_, args)| args.clone())
                .expect("every named room has an experiment earn");
            arguments["width"] = json!(44);
            arguments["height"] = json!(20);
            arguments[field] = value.clone();
            let held = call("play_room", arguments.clone());
            let held_aha = &held["result"]["structuredContent"]["engineeredAha"];
            assert_eq!(
                held_aha["wager"], value,
                "{room} dropped a call sent at its experiment earn: {held_aha}"
            );
            assert_eq!(held_aha["beat"], "withheld", "{room}: {held_aha}");
            for held_back in ["earn", "truth", "punchline", "graded"] {
                assert!(
                    held_aha[held_back].is_null(),
                    "{room} leaked {held_back} at withheld: {held_aha}"
                );
            }

            arguments["aha_summon"] = json!(true);
            let summoned = call("play_room", arguments);
            let summoned_aha = &summoned["result"]["structuredContent"]["engineeredAha"];
            assert_eq!(summoned_aha["beat"], "consolidated", "{room}");
            assert_eq!(
                summoned_aha["wager"], value,
                "{room} lost the call at consolidation: {summoned_aha}"
            );
            let earn = summoned_aha["earn"].as_str().unwrap_or_default();
            assert!(
                earn.starts_with("wager:") || earn.starts_with("call:"),
                "{room} graded the experiment instead of the call: {earn:?}"
            );
        }
    }

    #[test]
    fn a_number_wager_after_enough_throws_is_kept() {
        // The same drop existed in Buffon's Needle, where enough thrown needles
        // earn the withheld beat on their own.
        let throws: Vec<_> = (0..numinous_core::rooms::buffon_aha::MIN_THROWS_TO_EARN)
            .map(|throw| json!([0.2 + 0.05 * throw as f64, 0.5]))
            .collect();
        let held = call(
            "play_room",
            json!({
                "id":"buffon-needle",
                "pokes":throws,
                "number_wager":3.0,
                "width":40,
                "height":20
            }),
        );
        let held_aha = &held["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(held_aha["beat"], "withheld");
        assert_eq!(held_aha["wager"], 3.0);
        for held_back in ["earn", "truth", "punchline", "graded"] {
            assert!(
                held_aha[held_back].is_null(),
                "{held_back} leaked at withheld: {held_aha}"
            );
        }
    }

    #[test]
    fn no_keyboard_prompt_reaches_a_keyless_mind() {
        // The App's aha chrome invites E and digit keys; this face's verbs
        // are aha_summon and named wager fields, and the docs promise every prompt
        // names a verb the caller actually has. Whole-response sweep, so a
        // prompt cannot hide in text content or structuredContent.
        let calls = [
            json!({"id": "times-tables", "width": 40, "height": 20, "place_wager": "circle"}),
            json!({"id": "times-tables", "width": 40, "height": 20,
                   "place_wager": "mandelbrot", "aha_summon": true}),
            // x=0.0 lands K=2 exactly: the primed heart whose chrome asks
            // WHERE? with digit keys on the App face.
            json!({"id": "times-tables", "width": 40, "height": 20, "pokes": [[0.0, 0.5]]}),
            json!({"id": "buffon-needle", "width": 40, "height": 20, "number_wager": 3.0}),
            json!({"id": "buffon-needle", "width": 40, "height": 20,
                   "number_wager": 3.2, "aha_summon": true}),
            json!({"id": "galton-board", "width": 40, "height": 20,
                   "pokes": [[0.5, 0.5]], "bin_wager": 8}),
            json!({"id": "galton-board", "width": 40, "height": 20,
                   "pokes": [[0.5, 0.5]], "bin_wager": 8, "aha_summon": true}),
            json!({"id": "double-pendulum", "width": 40, "height": 20,
            "gesture": [
                {"kind": "down", "x": 0.6, "y": 0.3, "t": 0.1},
                {"kind": "up", "x": 0.6, "y": 0.3, "t": 0.2}
            ]}),
            json!({"id": "double-pendulum", "width": 40, "height": 20,
                   "gesture": [
                       {"kind": "down", "x": 0.6, "y": 0.3, "t": 0.1},
                       {"kind": "up", "x": 0.6, "y": 0.3, "t": 0.2}
                   ], "ending_wager": "together", "aha_summon": true}),
            json!({"id": "kepler-laws", "width": 40, "height": 20,
                   "pokes": [[0.8, 0.5]]}),
            json!({"id": "kepler-laws", "width": 40, "height": 20,
                   "pokes": [[0.8, 0.5]], "speed_wager": "faster", "aha_summon": true}),
            json!({"id": "parrondo", "width": 40, "height": 20,
                   "pokes": [[0.8, 0.5]]}),
            json!({"id": "parrondo", "width": 40, "height": 20,
                   "pokes": [[0.8, 0.5]], "policy_wager": "abb", "aha_summon": true}),
        ];
        for arguments in calls {
            let reply = call("play_room", arguments.clone());
            let text = reply.to_string();
            for prompt in [
                "PRESS E",
                "E:WHY",
                "1=M 2=N 3=C",
                "1=TOGETHER 2=DRIFTED 3=LOST",
                "1=FASTER 2=SLOWER 3=SAME",
                "1=A 2=B 3=ABB",
            ] {
                assert!(
                    !text.contains(prompt),
                    "keyboard prompt {prompt:?} escaped for {arguments}"
                );
            }
        }
    }

    #[test]
    fn answer_fields_are_absent_until_each_engineered_aha_is_consolidated() {
        let cases = [
            (
                json!({"id": "times-tables", "place_wager": "circle"}),
                &["punchline", "truth", "graded"][..],
            ),
            (
                json!({"id": "buffon-needle", "number_wager": 3.0}),
                &["punchline", "band", "truth", "graded"][..],
            ),
            (
                json!({"id": "galton-board", "pokes": [[0.5, 0.5]], "bin_wager": 8}),
                &["punchline", "band", "truth", "graded"][..],
            ),
            (
                json!({
                    "id": "double-pendulum",
                    "gesture": [
                        {"kind": "down", "x": 0.6, "y": 0.3, "t": 0.1},
                        {"kind": "up", "x": 0.6, "y": 0.3, "t": 0.2}
                    ],
                    "ending_wager": "together"
                }),
                &["punchline", "truth", "gap", "right", "graded"][..],
            ),
            (
                json!({"id": "kepler-laws", "pokes": [[0.8, 0.5]], "speed_wager": "same"}),
                &["apsidalSpeedRatio", "punchline", "truth", "right", "graded"][..],
            ),
            (
                json!({"id": "parrondo", "pokes": [[0.8, 0.5]], "policy_wager": "a"}),
                &["expectedEnd", "punchline", "truth", "right", "graded"][..],
            ),
            (
                json!({"id": "nontransitive", "die_choice": "a", "counter_wager": "b"}),
                &[
                    "exactCycle",
                    "counterWins",
                    "counterLosses",
                    "counterRate",
                    "punchline",
                    "wagerWins",
                    "truth",
                    "right",
                    "graded",
                ][..],
            ),
        ];

        for (arguments, answer_fields) in cases {
            let reply = call("play_room", arguments.clone());
            assert_ne!(reply["result"]["isError"], true, "{arguments}: {reply}");
            let aha = reply["result"]["structuredContent"]["engineeredAha"]
                .as_object()
                .expect("engineered Aha object");
            assert_eq!(aha.get("beat"), Some(&json!("withheld")), "{arguments}");
            assert!(
                aha.get("wager").is_some_and(|wager| !wager.is_null()),
                "the player's wager remains visible: {arguments}"
            );
            assert!(
                aha.get("earn").is_some_and(Value::is_null),
                "the internal earn receipt stays closed: {arguments}"
            );
            let status = aha
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            for leaked_grade in ["RIGHT", "WRONG", "NAILED", "CLOSE", "WILD"] {
                assert!(
                    !status.contains(leaked_grade),
                    "grade token {leaked_grade} leaked in {status:?} for {arguments}"
                );
            }
            if arguments["id"] == "galton-board" {
                assert!(
                    !status.contains('~'),
                    "theoretical mode leaked in {status:?}"
                );
            }
            for field in answer_fields {
                assert!(
                    !aha.contains_key(*field),
                    "answer field {field:?} leaked before consolidation for {arguments}"
                );
            }
        }
    }

    #[test]
    fn every_answer_names_the_coin_it_is_about() {
        // This face is stateless by contract: the same inputs give the same
        // result, and different inputs are a different question. A wave on
        // another coin therefore does change the answer, and that is
        // correct; what was wrong was that nothing said which pile the
        // verdict was about, so a mind saw nailed become wild for what
        // looked like the same call. Now every reply's truth, band, and
        // sentence agree about one named coin.
        for (pokes, coin, truth) in [
            (vec![[0.5, 0.5]], 2, 8),
            (vec![[0.5, 0.5], [0.9, 0.5]], 4, 11),
        ] {
            let reply = call(
                "play_room",
                json!({"id": "galton-board", "width": 48, "height": 24,
                       "pokes": pokes, "bin_wager": 8, "aha_summon": true}),
            );
            let aha = &reply["result"]["structuredContent"]["engineeredAha"];
            assert_eq!(aha["coin"], coin, "{aha}");
            assert_eq!(aha["truth"], truth, "{aha}");
            let graded = aha["graded"].as_str().expect("graded");
            let named = format!("{:.2} coin", if coin == 2 { 0.50 } else { 0.70 });
            assert!(
                graded.contains(&named),
                "the sentence names the pile: {graded}"
            );
            assert!(
                !graded.contains("  "),
                "no space runs in player copy: {graded}"
            );
        }
    }

    #[test]
    fn a_call_after_four_waves_is_not_silently_discarded() {
        // Four waves earn the withheld beat on their own; a caller who
        // follows the documented flow and also names a bin used to have
        // the bin thrown away with no error and no wager in the reply.
        let reply = call(
            "play_room",
            json!({"id": "galton-board", "width": 48, "height": 24,
                   "pokes": [[0.5, 0.5], [0.5, 0.5], [0.5, 0.5], [0.5, 0.5]],
                   "bin_wager": 3, "aha_summon": true}),
        );
        let aha = &reply["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(aha["wager"], 3, "the call must land: {aha}");
        assert!(
            aha["graded"].as_str().is_some_and(|g| g.contains("bin 3")),
            "and be answered: {aha}"
        );
    }

    #[test]
    fn galton_bin_wager_gates_reveal_and_grades_against_the_binomial() {
        // The third flagship aha: the wager is a model-level commitment
        // (where the WHOLE pile peaks), graded against the binomial's true
        // mode for the coin on screen, never against one ball's luck. A
        // poke at x=0.5 selects the fair coin, whose peak is bin 8.
        let wagered = call(
            "play_room",
            json!({
                "id": "galton-board",
                "width": 48,
                "height": 24,
                "pokes": [[0.5, 0.5]],
                "bin_wager": 8
            }),
        );
        // The interaction shape the frozen study drives: pokes with no
        // wager. The top-level status must keep the pile readout even while
        // the prime invite rides beside it.
        let primed = call(
            "play_room",
            json!({"id": "galton-board", "width": 48, "height": 24, "t": 0.25,
                   "pokes": [[0.5, 0.5]]}),
        );
        let status = primed["result"]["structuredContent"]["status"]
            .as_str()
            .expect("status")
            .to_string();
        assert!(
            status.contains("P.50"),
            "the pile readout must survive prime: {status}"
        );
        assert!(
            status.contains("PEAK?"),
            "the invite rides beside it: {status}"
        );

        let content = &wagered["result"]["structuredContent"];
        assert_eq!(content["engineeredAha"]["beat"], "withheld");
        assert_eq!(content["engineeredAha"]["kind"], "bin");
        assert_eq!(content["engineeredAha"]["allowReveal"], false);
        assert!(content["reveal"].is_null(), "the wager gates the reveal");

        let consolidated = call(
            "play_room",
            json!({
                "id": "galton-board",
                "width": 48,
                "height": 24,
                "pokes": [[0.5, 0.5]],
                "bin_wager": 8,
                "aha_summon": true
            }),
        );
        let done = &consolidated["result"]["structuredContent"];
        assert_eq!(done["engineeredAha"]["beat"], "consolidated");
        assert_eq!(done["engineeredAha"]["wager"], 8);
        assert_eq!(done["engineeredAha"]["truth"], 8);
        assert_eq!(done["engineeredAha"]["band"], "nailed");
        let graded = done["engineeredAha"]["graded"]
            .as_str()
            .expect("a committed wager is answered");
        assert!(graded.contains("bin 8"), "{graded}");
        assert!(graded.contains("Nailed"), "{graded}");
        assert!(
            !done["reveal"].is_null(),
            "consolidation unlocks the reveal"
        );

        // A wild wager on a loaded coin is graded, never punished. The poke
        // at x=0.9 selects the p=0.7 coin, whose peak is bin 11.
        let wild = call(
            "play_room",
            json!({
                "id": "galton-board",
                "width": 48,
                "height": 24,
                "pokes": [[0.9, 0.5]],
                "bin_wager": 2,
                "aha_summon": true
            }),
        );
        let aha = &wild["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(aha["band"], "wild");
        assert_eq!(aha["truth"], 11);
        let graded = aha["graded"].as_str().expect("graded");
        assert!(graded.contains("the gap is the lesson"), "{graded}");
    }

    #[test]
    fn double_pendulum_call_meets_the_measured_ending() {
        let open = call(
            "play_room",
            json!({"id": "double-pendulum", "width": 48, "height": 24}),
        );
        let open_aha = &open["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(open_aha["kind"], "ending");
        assert_eq!(open_aha["beat"], "explore");
        assert_eq!(open_aha["drops"], 0);

        let gesture = json!([
            {"kind": "down", "x": 7.0 / 12.0, "y": 0.5, "t": 0.1},
            {"kind": "up", "x": 7.0 / 12.0, "y": 0.5, "t": 0.2}
        ]);
        let primed = call(
            "play_room",
            json!({"id": "double-pendulum", "width": 48, "height": 24,
                   "gesture": gesture}),
        );
        let prime_aha = &primed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(prime_aha["beat"], "prime");
        assert_eq!(prime_aha["drops"], 1);
        assert!(
            prime_aha["status"]
                .as_str()
                .is_some_and(|status| status.contains("END? use ending_wager")),
            "the keyless face names its own wager field: {prime_aha}"
        );
        assert_eq!(
            prime_aha["endingOptions"],
            json!(["together", "drifted", "lost"])
        );

        let done = call(
            "play_room",
            json!({
                "id": "double-pendulum",
                "width": 48,
                "height": 24,
                "gesture": gesture,
                "ending_wager": "together",
                "aha_summon": true
            }),
        );
        let content = &done["result"]["structuredContent"];
        let aha = &content["engineeredAha"];
        assert_eq!(aha["beat"], "consolidated");
        assert_eq!(aha["wager"], "together");
        assert_eq!(aha["truth"], "lost");
        assert_eq!(aha["right"], false);
        assert!(aha["gap"].as_f64().is_some_and(|gap| gap > 1.0));
        assert!(
            aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("deterministic")),
            "the reasonable miss receives the room's lesson: {aha}"
        );
        assert_eq!(aha["allowReveal"], true);
        assert!(!content["reveal"].is_null());
    }

    #[test]
    fn double_pendulum_requires_release_and_allows_four_completed_redrops() {
        let held = call(
            "play_room",
            json!({
                "id": "double-pendulum",
                "gesture": [{"kind": "down", "x": 0.5, "y": 0.3, "t": 0.1}],
                "ending_wager": "lost"
            }),
        );
        assert_eq!(held["result"]["isError"], true);
        assert!(
            held["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("release"))
        );

        let done = call(
            "play_room",
            json!({
                "id": "double-pendulum",
                "gesture": [
                    {"kind": "down", "x": 0.2, "y": 0.3, "t": 0.1},
                    {"kind": "up", "x": 0.2, "y": 0.3, "t": 0.15},
                    {"kind": "down", "x": 0.4, "y": 0.4, "t": 0.3},
                    {"kind": "up", "x": 0.4, "y": 0.4, "t": 0.35},
                    {"kind": "down", "x": 0.6, "y": 0.5, "t": 0.5},
                    {"kind": "up", "x": 0.6, "y": 0.5, "t": 0.55},
                    {"kind": "down", "x": 0.8, "y": 0.6, "t": 0.7},
                    {"kind": "up", "x": 0.8, "y": 0.6, "t": 0.75}
                ],
                "aha_summon": true
            }),
        );
        let aha = &done["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(aha["beat"], "consolidated");
        assert_eq!(aha["drops"], 4);
        assert_eq!(aha["earn"], "drops:4");
        assert!(aha["wager"].is_null());
        assert!(aha["graded"].is_null());
    }

    #[test]
    fn kepler_speed_call_meets_the_exact_orbit_and_draws_equal_time_marks() {
        let open = call(
            "play_room",
            json!({"id": "kepler-laws", "width": 64, "height": 28}),
        );
        let open_aha = &open["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(open_aha["kind"], "speed");
        assert_eq!(open_aha["beat"], "explore");
        assert_eq!(open_aha["tunings"], 0);

        let primed = call(
            "play_room",
            json!({
                "id": "kepler-laws",
                "width": 64,
                "height": 28,
                "pokes": [[0.8, 0.4]]
            }),
        );
        let prime = &primed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(prime["beat"], "prime");
        assert_eq!(prime["tunings"], 1);
        assert_eq!(prime["speedOptions"], json!(["faster", "slower", "same"]));
        assert!(
            prime["status"]
                .as_str()
                .is_some_and(|status| status.contains("use speed_wager")),
            "the keyless face names its own call field: {prime}"
        );
        let e = prime["eccentricity"].as_f64().expect("eccentricity");
        assert!((e - 0.68).abs() < 1.0e-12, "{e}");

        let done = call(
            "play_room",
            json!({
                "id": "kepler-laws",
                "width": 64,
                "height": 28,
                "pokes": [[0.8, 0.4]],
                "speed_wager": "same",
                "aha_summon": true
            }),
        );
        let content = &done["result"]["structuredContent"];
        let aha = &content["engineeredAha"];
        assert_eq!(aha["beat"], "consolidated");
        assert_eq!(aha["wager"], "same");
        assert_eq!(aha["truth"], "faster");
        assert_eq!(aha["right"], false);
        assert!(
            aha["apsidalSpeedRatio"]
                .as_f64()
                .is_some_and(|ratio| ratio > 5.0)
        );
        assert!(
            aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("truth is FASTER")),
            "{aha}"
        );
        assert!(content["reveal"].is_string());
        assert!(
            content["render"]
                .as_str()
                .is_some_and(|render| render.contains('O')),
            "the consolidated picture must carry the equal-time evidence"
        );
    }

    #[test]
    fn kepler_circle_answers_same_and_four_tunings_can_earn_without_a_call() {
        let circle = call(
            "play_room",
            json!({
                "id": "kepler-laws",
                "pokes": [[0.0, 0.5]],
                "speed_wager": "same",
                "aha_summon": true
            }),
        );
        let circle_aha = &circle["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(circle_aha["truth"], "same");
        assert_eq!(circle_aha["right"], true);
        assert_eq!(circle_aha["apsidalSpeedRatio"], 1.0);

        let observed = call(
            "play_room",
            json!({
                "id": "kepler-laws",
                "pokes": [[0.2, 0.5], [0.4, 0.5], [0.6, 0.5], [0.8, 0.5]],
                "aha_summon": true
            }),
        );
        let observed_aha = &observed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(observed_aha["beat"], "consolidated");
        assert_eq!(observed_aha["earn"], "tunings:4");
        assert!(observed_aha["wager"].is_null());
        assert!(observed_aha["graded"].is_null());
    }

    #[test]
    fn parrondo_policy_call_meets_exact_expectations_and_draws_the_comparison() {
        let open = call(
            "play_room",
            json!({"id": "parrondo", "width": 64, "height": 28}),
        );
        let open_aha = &open["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(open_aha["kind"], "policy");
        assert_eq!(open_aha["beat"], "explore");
        assert_eq!(open_aha["selections"], 0);

        let primed = call(
            "play_room",
            json!({
                "id": "parrondo",
                "width": 64,
                "height": 28,
                "pokes": [[0.5, 0.4]]
            }),
        );
        let prime = &primed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(prime["beat"], "prime");
        assert_eq!(prime["policyOptions"], json!(["a", "b", "abb"]));
        assert!(
            prime["status"]
                .as_str()
                .is_some_and(|status| status.contains("use policy_wager")),
            "the keyless face names its own call field: {prime}"
        );

        let done = call(
            "play_room",
            json!({
                "id": "parrondo",
                "width": 64,
                "height": 28,
                "pokes": [[0.5, 0.4]],
                "policy_wager": "a",
                "aha_summon": true
            }),
        );
        let content = &done["result"]["structuredContent"];
        let aha = &content["engineeredAha"];
        assert_eq!(aha["beat"], "consolidated");
        assert_eq!(aha["wager"], "a");
        assert_eq!(aha["truth"], "abb");
        assert_eq!(aha["right"], false);
        assert!(aha["expectedEnd"]["a"].as_f64().is_some_and(|v| v < 0.0));
        assert!(aha["expectedEnd"]["b"].as_f64().is_some_and(|v| v < 0.0));
        assert!(aha["expectedEnd"]["abb"].as_f64().is_some_and(|v| v > 7.0));
        assert!(
            aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("winner is ABB")),
            "{aha}"
        );
        let render = content["render"].as_str().expect("render");
        assert!(render.contains('A') && render.contains('B') && render.contains('O'));
        assert!(content["reveal"].is_string());
    }

    #[test]
    fn kepler_compatibility_alias_accepts_wagers_and_returns_one_canonical_identity() {
        let reply = call(
            "play_room",
            json!({
                "id": "kepler-areas",
                "pokes": [[0.8, 0.5]],
                "speed_wager": "faster",
                "aha_summon": true
            }),
        );

        assert_ne!(reply["result"]["isError"], true, "{reply}");
        let content = &reply["result"]["structuredContent"];
        assert_eq!(content["room"], "kepler-laws");
        assert_eq!(content["engineeredAha"]["beat"], "consolidated");
        assert_eq!(content["engineeredAha"]["wager"], "faster");
    }

    #[test]
    fn parrondo_requires_a_selection_and_allows_four_observations() {
        let refused = call(
            "play_room",
            json!({"id": "parrondo", "policy_wager": "abb"}),
        );
        assert_eq!(refused["result"]["isError"], true);
        assert!(
            refused["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("tried policy"))
        );

        let observed = call(
            "play_room",
            json!({
                "id": "parrondo",
                "pokes": [[0.1, 0.5], [0.5, 0.5], [0.9, 0.5], [0.7, 0.5]],
                "aha_summon": true
            }),
        );
        let aha = &observed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(aha["beat"], "consolidated");
        assert_eq!(aha["earn"], "selections:4");
        assert!(aha["wager"].is_null());
        assert!(aha["graded"].is_null());
    }

    #[test]
    fn nontransitive_counter_call_meets_all_36_outcomes_and_draws_them() {
        let open = call(
            "play_room",
            json!({"id": "nontransitive", "width": 64, "height": 28}),
        );
        let open_aha = &open["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(open_aha["kind"], "counter");
        assert_eq!(open_aha["beat"], "explore");
        assert_eq!(open_aha["choices"], 0);

        let primed = call(
            "play_room",
            json!({
                "id": "nontransitive",
                "width": 64,
                "height": 28,
                "die_choice": "a"
            }),
        );
        let prime = &primed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(prime["beat"], "prime");
        assert_eq!(prime["chosen"], "a");
        assert_eq!(prime["counterOptions"], json!(["a", "b", "c"]));
        assert!(
            prime["status"]
                .as_str()
                .is_some_and(|status| status.contains("counter_wager")),
            "the keyless face names its own call field: {prime}"
        );

        let done = call(
            "play_room",
            json!({
                "id": "nontransitive",
                "width": 64,
                "height": 28,
                "die_choice": "a",
                "counter_wager": "b",
                "aha_summon": true
            }),
        );
        let content = &done["result"]["structuredContent"];
        let aha = &content["engineeredAha"];
        assert_eq!(aha["beat"], "consolidated");
        assert_eq!(aha["chosen"], "a");
        assert_eq!(aha["wager"], "b");
        assert_eq!(aha["truth"], "c");
        assert_eq!(aha["right"], false);
        assert_eq!(aha["counterWins"], 20);
        assert_eq!(aha["exactCycle"]["aOverB"], 24);
        assert_eq!(aha["exactCycle"]["bOverC"], 24);
        assert_eq!(aha["exactCycle"]["cOverA"], 20);
        assert_eq!(aha["faces"]["a"], json!([4, 4, 4, 4, 0, 0]));
        assert!(
            aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("counter is C")),
            "{aha}"
        );
        let render = content["render"].as_str().expect("render");
        assert!(render.contains("C vs A"));
        assert!(render.contains("20 W / 16 L"));
        assert!(content["reveal"].is_string());
    }

    #[test]
    fn nontransitive_accepts_a_hand_choice_and_four_choice_observation() {
        let hand = call(
            "play_room",
            json!({
                "id": "nontransitive",
                "pokes": [[0.82, 0.78]],
                "counter_wager": "b",
                "aha_summon": true
            }),
        );
        let hand_aha = &hand["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(hand_aha["chosen"], "c");
        assert_eq!(hand_aha["truth"], "b");
        assert_eq!(hand_aha["right"], true);
        assert_eq!(hand_aha["counterWins"], 24);

        let observed = call(
            "play_room",
            json!({
                "id": "nontransitive",
                "pokes": [[0.5, 0.18], [0.18, 0.78], [0.82, 0.78], [0.5, 0.18]],
                "aha_summon": true
            }),
        );
        let observed_aha = &observed["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(observed_aha["beat"], "consolidated");
        assert_eq!(observed_aha["earn"], "choices:4");
        assert_eq!(observed_aha["chosen"], "a");
        assert!(observed_aha["wager"].is_null());
        assert!(observed_aha["graded"].is_null());
    }

    #[test]
    fn times_tables_place_wager_gates_reveal_until_aha_summon() {
        let wagered = call(
            "play_room",
            json!({
                "id": "times-tables",
                "width": 40,
                "height": 20,
                "place_wager": "circle"
            }),
        );
        let content = &wagered["result"]["structuredContent"];
        assert_eq!(content["engineeredAha"]["beat"], "withheld");
        assert!(content["engineeredAha"]["earn"].is_null());
        assert_eq!(content["engineeredAha"]["allowReveal"], false);
        assert!(content["reveal"].is_null());
        assert!(
            content["status"]
                .as_str()
                .is_some_and(|s| s.contains("SUMMON: aha_summon:true")),
            "the withheld beat must invite this face's own verb: {:?}",
            content["status"]
        );

        let consolidated = call(
            "play_room",
            json!({
                "id": "times-tables",
                "width": 40,
                "height": 20,
                "place_wager": "mandelbrot",
                "aha_summon": true
            }),
        );
        let done = &consolidated["result"]["structuredContent"];
        assert_eq!(done["engineeredAha"]["beat"], "consolidated");
        assert_eq!(done["engineeredAha"]["allowReveal"], true);
        assert!(
            done["reveal"]
                .as_str()
                .is_some_and(|reveal| reveal.contains("Mandelbrot"))
        );
        assert_eq!(done["engineeredAha"]["earn"], "wager:mandelbrot");
    }

    #[test]
    fn the_wager_is_graded_against_the_truth_not_discarded() {
        // The keystone of the engineered aha is meeting your own commitment
        // against the answer. A right wager and a wrong wager must come back
        // as different graded truths, or the wager was theater.
        let grade = |place: &str| {
            let done = call(
                "play_room",
                json!({
                    "id": "times-tables",
                    "width": 40,
                    "height": 20,
                    "place_wager": place,
                    "aha_summon": true
                }),
            );
            let aha = done["result"]["structuredContent"]["engineeredAha"].clone();
            assert_eq!(aha["truth"], "mandelbrot");
            assert_eq!(aha["wager"], place);
            aha["graded"]
                .as_str()
                .expect("a graded sentence")
                .to_string()
        };
        let nailed = grade("mandelbrot");
        let missed = grade("circle");
        assert!(nailed.contains("Nailed"), "{nailed}");
        assert!(missed.contains("CIRCLE"), "{missed}");
        assert!(
            missed.contains("fertile"),
            "a miss is met, not punished: {missed}"
        );
        assert_ne!(nailed, missed);

        // Buffon grades in the same typed shape, band and all.
        let buffon = call(
            "play_room",
            json!({
                "id": "buffon-needle",
                "width": 40,
                "height": 20,
                "number_wager": 3.0,
                "aha_summon": true
            }),
        );
        let aha = &buffon["result"]["structuredContent"]["engineeredAha"];
        assert_eq!(aha["wager"], 3.0);
        assert_eq!(aha["band"], "close");
        assert!(
            aha["graded"]
                .as_str()
                .is_some_and(|graded| graded.contains("3.00") && graded.contains("pi")),
            "{aha}"
        );
    }

    #[test]
    fn buffon_number_wager_walks_the_engineered_aha() {
        let open = call(
            "play_room",
            json!({"id": "buffon-needle", "width": 40, "height": 20}),
        );
        assert_eq!(
            open["result"]["structuredContent"]["engineeredAha"]["kind"],
            "number"
        );
        assert_eq!(
            open["result"]["structuredContent"]["engineeredAha"]["beat"],
            "explore"
        );

        let wagered = call(
            "play_room",
            json!({
                "id": "buffon-needle",
                "width": 40,
                "height": 20,
                "number_wager": 3.0
            }),
        );
        let content = &wagered["result"]["structuredContent"];
        assert_eq!(content["engineeredAha"]["beat"], "withheld");
        assert!(content["reveal"].is_null());
        assert!(content["engineeredAha"]["earn"].is_null());

        let done = call(
            "play_room",
            json!({
                "id": "buffon-needle",
                "width": 40,
                "height": 20,
                "number_wager": std::f64::consts::PI,
                "aha_summon": true
            }),
        );
        let done_content = &done["result"]["structuredContent"];
        assert_eq!(done_content["engineeredAha"]["beat"], "consolidated");
        assert_eq!(done_content["engineeredAha"]["allowReveal"], true);
        assert!(
            done_content["reveal"]
                .as_str()
                .is_some_and(|r| r.to_ascii_lowercase().contains("pi") || r.contains("circle"))
        );
    }

    #[test]
    fn flagship_aha_args_reject_wrong_rooms_and_hostile_values() {
        let wrong = call(
            "play_room",
            json!({"id": "lorenz", "place_wager": "mandelbrot"}),
        );
        assert_eq!(wrong["result"]["isError"], true);
        let buffon_place = call(
            "play_room",
            json!({"id": "buffon-needle", "place_wager": "circle"}),
        );
        assert_eq!(buffon_place["result"]["isError"], true);
        let over = call(
            "play_room",
            json!({"id": "buffon-needle", "number_wager": 9.0}),
        );
        assert_eq!(over["result"]["isError"], true);
        let summon_alone = call(
            "play_room",
            json!({"id": "times-tables", "aha_summon": true}),
        );
        assert_eq!(summon_alone["result"]["isError"], true);
        let pendulum_wrong_room =
            call("play_room", json!({"id": "lorenz", "ending_wager": "lost"}));
        assert_eq!(pendulum_wrong_room["result"]["isError"], true);
        let pendulum_bad_ending = call(
            "play_room",
            json!({"id": "double-pendulum", "ending_wager": "elsewhere"}),
        );
        assert_eq!(pendulum_bad_ending["result"]["isError"], true);
        let kepler_without_orbit = call(
            "play_room",
            json!({"id": "kepler-laws", "speed_wager": "faster"}),
        );
        assert_eq!(kepler_without_orbit["result"]["isError"], true);
        let kepler_wrong_room = call(
            "play_room",
            json!({"id": "lorenz", "speed_wager": "faster"}),
        );
        assert_eq!(kepler_wrong_room["result"]["isError"], true);
        let kepler_bad_relation = call(
            "play_room",
            json!({
                "id": "kepler-laws",
                "pokes": [[0.7, 0.5]],
                "speed_wager": "sideways"
            }),
        );
        assert_eq!(kepler_bad_relation["result"]["isError"], true);
        let parrondo_wrong_room = call("play_room", json!({"id": "lorenz", "policy_wager": "abb"}));
        assert_eq!(parrondo_wrong_room["result"]["isError"], true);
        let parrondo_bad_policy = call(
            "play_room",
            json!({
                "id": "parrondo",
                "pokes": [[0.7, 0.5]],
                "policy_wager": "abab"
            }),
        );
        assert_eq!(parrondo_bad_policy["result"]["isError"], true);
        let parrondo_wrong_type = call("play_room", json!({"id": "parrondo", "policy_wager": 3}));
        assert_eq!(parrondo_wrong_type["result"]["isError"], true);
        let dice_without_choice = call(
            "play_room",
            json!({"id": "nontransitive", "counter_wager": "c"}),
        );
        assert_eq!(dice_without_choice["result"]["isError"], true);
        let dice_wrong_room = call("play_room", json!({"id": "lorenz", "die_choice": "a"}));
        assert_eq!(dice_wrong_room["result"]["isError"], true);
        let dice_bad_counter = call(
            "play_room",
            json!({
                "id": "nontransitive",
                "die_choice": "a",
                "counter_wager": "d"
            }),
        );
        assert_eq!(dice_bad_counter["result"]["isError"], true);
        let dice_wrong_type = call("play_room", json!({"id": "nontransitive", "die_choice": 1}));
        assert_eq!(dice_wrong_type["result"]["isError"], true);
        let dice_two_choices = call(
            "play_room",
            json!({
                "id": "nontransitive",
                "die_choice": "a",
                "pokes": [[0.5, 0.18]]
            }),
        );
        assert_eq!(dice_two_choices["result"]["isError"], true);
        let multiple_wagers = call(
            "play_room",
            json!({
                "id": "parrondo",
                "pokes": [[0.7, 0.5]],
                "policy_wager": "abb",
                "speed_wager": "faster"
            }),
        );
        assert_eq!(multiple_wagers["result"]["isError"], true);
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
    fn play_room_reports_interaction_aware_status() {
        let poked = handle_request(&json!({
            "jsonrpc":"2.0","id":34,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"cult-of-pi","width":50,"height":24,"t":0.0,"pokes":[[0.5,0.5]]}}
        }))
        .expect("tools/call must respond");
        let status = poked["result"]["structuredContent"]["status"]
            .as_str()
            .unwrap_or_default();
        assert!(
            status.starts_with("1 HELD FIX0 D") && status.contains(" CH01"),
            "placement-graded hold status, got {status}"
        );
        let text = poked["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains(&format!("Status: {status}")), "got: {text}");
        assert!(
            poked["result"]["structuredContent"]["delta"]["cells_changed"]
                .as_u64()
                .is_some_and(|changed| changed > 0),
            "a phase-zero hold must visibly change the character frame"
        );
    }

    #[test]
    fn compact_life_poke_matches_a_phase_stamped_click() {
        let common = json!({
            "id":"game-of-life","width":64,"height":48,"t":0.47
        });
        let mut poke_args = common.clone();
        poke_args["pokes"] = json!([[0.23, 0.71]]);
        let poked = handle_request(&json!({
            "jsonrpc":"2.0","id":35,"method":"tools/call",
            "params":{"name":"play_room","arguments":poke_args}
        }))
        .expect("poke response");

        let mut gesture_args = common;
        gesture_args["gesture"] = json!([
            {"kind":"down","x":0.23,"y":0.71,"t":0.47}
        ]);
        let gestured = handle_request(&json!({
            "jsonrpc":"2.0","id":36,"method":"tools/call",
            "params":{"name":"play_room","arguments":gesture_args}
        }))
        .expect("gesture response");
        assert_eq!(
            poked["result"]["structuredContent"]["render"],
            gestured["result"]["structuredContent"]["render"]
        );
        assert_eq!(
            poked["result"]["structuredContent"]["status"],
            gestured["result"]["structuredContent"]["status"]
        );
    }

    #[test]
    fn life_replay_is_causal_deterministic_and_sessionless() {
        let interacted_args = json!({
            "id":"game-of-life",
            "variation":7,
            "width":64,
            "height":48,
            "t":0.5,
            "gesture":[{"kind":"down","x":0.23,"y":0.71,"t":0.1}]
        });
        let untouched_args = json!({
            "id":"game-of-life","variation":7,"width":64,"height":48,"t":0.5
        });
        let call = |id, arguments| {
            handle_request(&json!({
                "jsonrpc":"2.0","id":id,"method":"tools/call",
                "params":{"name":"play_room","arguments":arguments}
            }))
            .expect("Life play_room response")
        };

        let first = call(37, interacted_args.clone());
        let untouched = call(38, untouched_args.clone());
        let repeated = call(39, interacted_args.clone());
        let untouched_repeated = call(40, untouched_args);
        let first_content = &first["result"]["structuredContent"];
        let repeated_content = &repeated["result"]["structuredContent"];

        assert_eq!(first_content["render"], repeated_content["render"]);
        assert_eq!(first_content["status"], repeated_content["status"]);
        assert_eq!(
            untouched["result"]["structuredContent"]["render"],
            untouched_repeated["result"]["structuredContent"]["render"],
            "an interacted call cannot create hidden MCP Life state"
        );
        assert_ne!(
            first_content["render"],
            untouched["result"]["structuredContent"]["render"]
        );
        assert_eq!(first_content["variation"], 7);
        assert_eq!(first_content["gesture"], interacted_args["gesture"]);
        assert!(first_content["delta"].is_object());
        let status = first_content["status"].as_str().expect("Life status");
        assert!(status.starts_with("BORN "), "got: {status}");
        assert!(status.contains("GEN 70"), "got: {status}");
        assert!(status.contains("GLIDER 1"), "got: {status}");

        let mut session = numinous_core::rooms::game_of_life::LifeSession::new(7);
        for _ in 0..14 {
            session.advance();
        }
        assert!(session.launch((0.23, 0.71)));
        for _ in 14..70 {
            session.advance();
        }
        let mut canvas = numinous_core::Canvas::new(64, 48);
        session.render(&mut canvas);
        assert_eq!(first_content["render"], canvas.to_text());
        assert_eq!(first_content["status"], session.status());
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
    fn play_room_temporal_evidence_replays_both_exact_observations() {
        let origin_arguments = json!({
            "id":"times-tables","width":72,"height":32,"variation":7,"t":0.20
        });
        let destination_arguments = json!({
            "id":"times-tables","width":72,"height":32,"variation":7,"t":0.35
        });
        let paired_arguments = json!({
            "id":"times-tables","width":72,"height":32,"variation":7,
            "from_t":0.20,"t":0.35
        });
        let origin = call("play_room", origin_arguments);
        let destination = call("play_room", destination_arguments);
        let paired = call("play_room", paired_arguments.clone());
        let repeated = call("play_room", paired_arguments.clone());
        let temporal = &paired["result"]["structuredContent"]["temporal"];

        assert_eq!(paired, repeated, "the paired observation is deterministic");
        assert_eq!(temporal["schema"], "numinous.temporal-evidence");
        assert_eq!(temporal["schemaVersion"], 1);
        assert_eq!(temporal["fromT"], 0.20);
        assert_eq!(temporal["toT"], 0.35);
        assert_eq!(
            temporal["fromRender"],
            origin["result"]["structuredContent"]["render"]
        );
        assert_eq!(
            temporal["fromStatus"],
            origin["result"]["structuredContent"]["status"]
        );
        assert_eq!(
            paired["result"]["structuredContent"]["render"],
            destination["result"]["structuredContent"]["render"]
        );
        assert_eq!(
            paired["result"]["structuredContent"]["status"],
            destination["result"]["structuredContent"]["status"]
        );
        let mut destination_projection = paired["result"]["structuredContent"].clone();
        destination_projection
            .as_object_mut()
            .expect("structured object")
            .remove("temporal");
        assert_eq!(
            destination_projection, destination["result"]["structuredContent"],
            "every existing top-level field remains the exact destination result"
        );

        let delta = &temporal["delta"];
        let changed = delta["cells_changed"].as_u64().expect("changed cells");
        assert!(
            changed > 0,
            "Times Tables visibly changes across these phases"
        );
        assert_eq!(
            changed,
            delta["ink_added"].as_u64().unwrap_or_default()
                + delta["ink_removed"].as_u64().unwrap_or_default()
                + delta["ink_reshaped"].as_u64().unwrap_or_default()
        );
        assert_eq!(delta["total_cells"], 72 * 32);
        assert!(delta["changed_region"].is_array());
        let text = paired["result"]["content"][0]["text"]
            .as_str()
            .expect("full temporal text");
        assert!(text.contains("from t=0.200"), "{text}");
        assert!(text.contains("at t=0.350"), "{text}");
        assert!(
            text.contains(&format!(
                "Temporal: {changed} of {} cells changed from t=0.200 to t=0.350",
                72 * 32
            )),
            "{text}"
        );

        let public = numinous_broadcast::PublicToolEvent::new(
            PublicTool::PlayRoom,
            &paired_arguments,
            &paired["result"],
        )
        .expect("public temporal event");
        let public_bytes = serde_json::to_vec(&public).expect("serialize public event");
        assert!(
            public_bytes.len() < numinous_broadcast::MAX_EVENT_BYTES - 1_024,
            "temporal public event has no envelope margin: {} bytes",
            public_bytes.len()
        );
    }

    #[test]
    fn every_maximum_temporal_room_event_enters_the_real_bounded_consent_queue() {
        let gesture: Vec<Value> = (0..numinous_core::MAX_ROOM_INPUTS)
            .map(|index| {
                let fraction = index as f64 / numinous_core::MAX_ROOM_INPUTS as f64;
                json!({
                    "kind":"move",
                    "x":0.1234567890123456 + fraction * 0.5,
                    "y":0.9876543210987654 - fraction * 0.5,
                    "t":0.1111111111111111 + fraction * 0.5
                })
            })
            .collect();
        let machine = numinous_broadcast::ConsentMachine::new(
            numinous_broadcast::SessionId::generate().expect("session id"),
            numinous_broadcast::numinous_compatibility().expect("compatibility"),
        );
        machine.begin_awaiting().expect("awaiting");
        machine.allow().expect("live consent");

        for metadata in numinous_core::ROOM_CATALOG {
            let arguments = json!({
                "id":metadata.id,
                "width":72,
                "height":32,
                "variation":u64::MAX,
                "from_t":0.1234567890123456,
                "t":0.8765432109876543,
                "gesture":gesture,
            });
            let result =
                super::play_room_tool_for_journey(&arguments, &numinous_core::Journey::default());
            assert_eq!(result["isError"], false, "{} refused", metadata.id);
            let event =
                numinous_broadcast::PublicToolEvent::new(PublicTool::PlayRoom, &arguments, &result)
                    .expect("public event");
            let ticket = machine.capture().expect("live ticket");
            let outcome = machine
                .prepare_and_commit(ticket, &event)
                .expect("bounded projection");
            assert!(
                matches!(outcome, numinous_broadcast::CommitOutcome::Queued { .. }),
                "{} temporal result exceeded the complete public envelope bound: {outcome:?}",
                metadata.id
            );
        }
    }

    #[test]
    fn temporal_decreasing_pair_preserves_supplied_direction() {
        let common = json!({
            "id":"times-tables", "width":40, "height":20, "variation":11
        });
        let mut decreasing_arguments = common.clone();
        decreasing_arguments["from_t"] = json!(0.95);
        decreasing_arguments["t"] = json!(0.05);
        let mut reverse_arguments = common.clone();
        reverse_arguments["from_t"] = json!(0.05);
        reverse_arguments["t"] = json!(0.95);
        let mut origin_arguments = common.clone();
        origin_arguments["t"] = json!(0.95);
        let mut destination_arguments = common;
        destination_arguments["t"] = json!(0.05);

        let decreasing = call("play_room", decreasing_arguments);
        let reverse = call("play_room", reverse_arguments);
        let origin = call("play_room", origin_arguments);
        let destination = call("play_room", destination_arguments);
        let temporal = &decreasing["result"]["structuredContent"]["temporal"];
        let reverse_temporal = &reverse["result"]["structuredContent"]["temporal"];

        assert_eq!(temporal["fromT"], 0.95);
        assert_eq!(temporal["toT"], 0.05);
        assert_eq!(
            temporal["fromRender"],
            origin["result"]["structuredContent"]["render"]
        );
        assert_eq!(
            decreasing["result"]["structuredContent"]["render"],
            destination["result"]["structuredContent"]["render"]
        );
        assert_eq!(
            temporal["delta"]["ink_added"],
            reverse_temporal["delta"]["ink_removed"]
        );
        assert_eq!(
            temporal["delta"]["ink_removed"],
            reverse_temporal["delta"]["ink_added"]
        );
        assert_eq!(
            temporal["delta"]["ink_reshaped"],
            reverse_temporal["delta"]["ink_reshaped"]
        );
        assert_eq!(
            temporal["delta"]["changed_region"],
            reverse_temporal["delta"]["changed_region"]
        );
    }

    #[test]
    fn temporal_evidence_keeps_equal_phases_and_touch_delta_distinct() {
        let arguments = json!({
            "id":"double-pendulum","width":48,"height":24,
            "from_t":0.25,"t":0.25,"pokes":[[0.2,0.8]]
        });
        let full = call("play_room", arguments.clone());
        let structured = &full["result"]["structuredContent"];
        assert!(
            structured["delta"].is_object(),
            "touch delta remains present"
        );
        assert_eq!(structured["temporal"]["delta"]["cells_changed"], 0);
        assert!(structured["temporal"]["delta"]["changed_region"].is_null());

        let compact = call(
            "play_room",
            with_response_mode(arguments.clone(), "compact"),
        );
        assert_eq!(
            structured, &compact["result"]["structuredContent"],
            "compact mode changes prose only"
        );
        let compact_text = compact["result"]["content"][0]["text"]
            .as_str()
            .expect("compact text");
        assert!(compact_text.contains("Touch changed"), "{compact_text}");
        assert!(
            compact_text.contains("Temporal from t=0.250"),
            "{compact_text}"
        );
        assert!(
            !compact_text.contains("\n\n"),
            "compact text carries no ASCII frame"
        );

        let single = call(
            "play_room",
            json!({"id":"double-pendulum","width":48,"height":24,"t":0.25,"pokes":[[0.2,0.8]]}),
        );
        assert!(
            single["result"]["structuredContent"]
                .get("temporal")
                .is_none(),
            "legacy calls omit the additive field instead of returning null"
        );
    }

    #[test]
    fn omitted_receipt_leaves_structured_content_without_an_encounter() {
        let omitted = call("play_room", json!({"id":"times-tables"}));
        let explicit_false = call("play_room", json!({"id":"times-tables","receipt":false}));
        assert!(
            omitted["result"]["structuredContent"]
                .get("encounter")
                .is_none()
        );
        assert_eq!(
            omitted["result"]["structuredContent"],
            explicit_false["result"]["structuredContent"]
        );
    }

    #[test]
    fn play_room_receipt_is_digest_stable_and_binds_the_play() {
        let omitted_defaults = json!({"id":"times-tables","receipt":true});
        let explicit_defaults = json!({
            "id":"times-tables","t":0.0,"width":72,"height":32,"variation":0,
            "receipt":true
        });
        let first = call("play_room", omitted_defaults);
        let second = call("play_room", json!({"id":"times-tables","receipt":true}));
        let explicit = call("play_room", explicit_defaults);
        let first_receipt = &first["result"]["structuredContent"]["encounter"];
        let second_receipt = &second["result"]["structuredContent"]["encounter"];
        let explicit_receipt = &explicit["result"]["structuredContent"]["encounter"];

        assert_eq!(first_receipt["schema"], "numinous.encounter-receipt");
        assert_eq!(first_receipt["schemaVersion"], 1);
        assert_eq!(first_receipt["tool"], "play_room");
        assert_eq!(first_receipt["replayAbiVersion"], 1);
        assert_eq!(first_receipt["action"]["room"], "times-tables");
        assert_eq!(first_receipt["action"]["width"], 72);
        assert_eq!(first_receipt["action"]["height"], 32);
        assert!(first_receipt.get("issuedAt").is_none());
        for field in ["fingerprint", "actionDigest", "resultDigest"] {
            let hex = first_receipt[field].as_str().unwrap_or_default();
            assert_eq!(hex.len(), 64, "{field}");
            assert!(
                hex.bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
                "{field}: {hex}"
            );
        }
        let isolated = super::play_room_tool_for_journey(
            &json!({"id":"times-tables","receipt":true}),
            &numinous_core::Journey::default(),
        );
        assert_eq!(
            first_receipt, &isolated["structuredContent"]["encounter"],
            "a second process-shaped call must issue the same receipt"
        );
        assert_eq!(first_receipt, second_receipt);
        assert_eq!(
            first_receipt["actionDigest"],
            explicit_receipt["actionDigest"]
        );
        assert_eq!(
            first_receipt["resultDigest"],
            explicit_receipt["resultDigest"]
        );

        let phase = call(
            "play_room",
            json!({"id":"times-tables","t":0.35,"receipt":true}),
        );
        let poked = call(
            "play_room",
            json!({"id":"times-tables","pokes":[[0.2,0.8]],"receipt":true}),
        );
        let wagered = call(
            "play_room",
            json!({"id":"times-tables","place_wager":"mandelbrot","receipt":true}),
        );
        assert_ne!(
            first_receipt["actionDigest"],
            phase["result"]["structuredContent"]["encounter"]["actionDigest"]
        );
        assert_ne!(
            first_receipt["actionDigest"],
            poked["result"]["structuredContent"]["encounter"]["actionDigest"]
        );
        assert_ne!(
            first_receipt["actionDigest"],
            wagered["result"]["structuredContent"]["encounter"]["actionDigest"]
        );
        assert_ne!(
            first_receipt["resultDigest"],
            phase["result"]["structuredContent"]["encounter"]["resultDigest"]
        );

        let compact = call(
            "play_room",
            with_response_mode(json!({"id":"times-tables","receipt":true}), "compact"),
        );
        assert_eq!(
            first_receipt, &compact["result"]["structuredContent"]["encounter"],
            "compact and full share the same encounter object"
        );
        let compact_text = compact["result"]["content"][0]["text"]
            .as_str()
            .expect("compact text");
        assert!(
            compact_text.contains("Encounter receipt attached"),
            "{compact_text}"
        );
    }

    #[test]
    fn a_receipt_is_kept_only_after_a_live_replay_match() {
        let journal = super::journal_path();
        let _ = numinous_core::erase_journal_file(&journal);
        let impersonated = call(
            "record_journal",
            json!({
                "kind":"encounter",
                "subject":"times-tables",
                "text":"This is not a Numinous result.",
                "source":"numinous-result"
            }),
        );
        assert_eq!(impersonated["result"]["isError"], true);

        let play = call("play_room", json!({"id":"times-tables","receipt":true}));
        let receipt = play["result"]["structuredContent"]["encounter"].clone();
        let digest = receipt["resultDigest"]
            .as_str()
            .expect("digest")
            .to_string();
        let kept = call(
            "record_journal",
            json!({
                "kind":"encounter",
                "text":"I want to keep this look.",
                "receipt": receipt
            }),
        );
        assert_eq!(kept["result"]["isError"], false);
        assert_eq!(
            kept["result"]["structuredContent"]["source"],
            "numinous-result"
        );
        assert_eq!(
            kept["result"]["structuredContent"]["subject"],
            format!("receipt:{digest}")
        );
        assert!(
            kept["result"]["structuredContent"].get("receipt").is_none(),
            "the keep reply must not store the receipt body"
        );

        let mut forged = receipt.clone();
        forged["resultDigest"] =
            json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        let refused = call(
            "record_journal",
            json!({
                "kind":"encounter",
                "text":"A forged digest is not a keep.",
                "receipt": forged
            }),
        );
        assert_eq!(refused["result"]["isError"], true);

        let mut unknown_abi = receipt;
        unknown_abi["replayAbiVersion"] = json!(99);
        let stale = call(
            "record_journal",
            json!({
                "kind":"encounter",
                "text":"An unknown ABI is not a keep.",
                "receipt": unknown_abi
            }),
        );
        assert_eq!(stale["result"]["isError"], true);
        assert!(
            stale["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("replay ABI")
        );

        let page = call("read_journal", json!({}));
        assert_eq!(page["result"]["structuredContent"]["totalEntries"], 1);
        assert_eq!(
            page["result"]["structuredContent"]["entries"][0]["subject"],
            format!("receipt:{digest}")
        );

        let erased = call("erase_journal", json!({"confirm":true}));
        assert_eq!(
            erased["result"]["structuredContent"]["recoverableManagedResidue"],
            0
        );
        assert_eq!(
            call("read_journal", json!({}))["result"]["structuredContent"]["totalEntries"],
            0
        );
        let _ = numinous_core::erase_journal_file(&journal);
    }

    #[test]
    fn listen_and_sing_receipts_are_digest_stable_and_exclude_wav_bytes() {
        let listen = call("listen_room", json!({"id":"times-tables","receipt":true}));
        let listen_again = call("listen_room", json!({"id":"times-tables","receipt":true}));
        let listen_receipt = &listen["result"]["structuredContent"]["encounter"];
        assert_eq!(listen_receipt["schema"], "numinous.encounter-receipt");
        assert_eq!(listen_receipt["tool"], "listen_room");
        assert_eq!(listen_receipt["action"]["room"], "times-tables");
        assert_eq!(
            listen_receipt["actionDigest"],
            listen_again["result"]["structuredContent"]["encounter"]["actionDigest"]
        );
        assert!(listen_receipt.get("issuedAt").is_none());
        assert!(
            serde_json::to_string(listen_receipt)
                .expect("serialize listen receipt")
                .contains("listen_room")
        );

        let sing = call("sing_expression", json!({"expr":"sin(x)","receipt":true}));
        let sing_explicit = call(
            "sing_expression",
            json!({
                "expr":"sin(x)",
                "notes":32,
                "xmin": -std::f64::consts::TAU,
                "xmax": std::f64::consts::TAU,
                "a": 1.0,
                "receipt":true
            }),
        );
        let sing_receipt = &sing["result"]["structuredContent"]["encounter"];
        assert_eq!(sing_receipt["tool"], "sing_expression");
        assert_eq!(sing_receipt["action"]["expr"], "sin(x)");
        assert_eq!(
            sing_receipt["actionDigest"],
            sing_explicit["result"]["structuredContent"]["encounter"]["actionDigest"]
        );
        assert!(
            !serde_json::to_string(sing_receipt)
                .expect("serialize sing receipt")
                .contains("UklGR")
        );
    }

    #[test]
    fn temporal_pokes_and_gestures_match_their_declared_replay_basis() {
        for (field, input) in [
            ("pokes", json!([[0.23, 0.71]])),
            (
                "gesture",
                json!([
                    {"kind":"down","x":0.23,"y":0.71,"t":0.10},
                    {"kind":"up","x":0.23,"y":0.71,"t":0.11}
                ]),
            ),
        ] {
            let mut paired_arguments = json!({
                "id":"game-of-life","variation":7,"width":48,"height":24,
                "from_t":0.20,"t":0.50
            });
            paired_arguments[field] = input;
            let mut origin_arguments = paired_arguments.clone();
            origin_arguments
                .as_object_mut()
                .expect("origin arguments")
                .remove("from_t");
            origin_arguments["t"] = json!(0.20);
            let mut destination_arguments = paired_arguments.clone();
            destination_arguments
                .as_object_mut()
                .expect("destination arguments")
                .remove("from_t");

            let paired = call("play_room", paired_arguments);
            let origin = call("play_room", origin_arguments);
            let destination = call("play_room", destination_arguments);
            let temporal = &paired["result"]["structuredContent"]["temporal"];
            assert_eq!(
                temporal["fromRender"], origin["result"]["structuredContent"]["render"],
                "{field} origin does not match its independent replay"
            );
            assert_eq!(
                temporal["fromStatus"], origin["result"]["structuredContent"]["status"],
                "{field} origin status does not match its independent replay"
            );
            assert_eq!(
                paired["result"]["structuredContent"]["render"],
                destination["result"]["structuredContent"]["render"],
                "{field} destination does not match its independent replay"
            );
            if field == "gesture" {
                assert_ne!(
                    temporal["fromRender"], paired["result"]["structuredContent"]["render"],
                    "the phase-stamped Life gesture should carry causal evolution"
                );
            }
        }
    }

    #[test]
    fn temporal_arguments_fail_closed_before_rendering() {
        for (arguments, expected) in [
            (
                json!({"id":"times-tables","from_t":0.2}),
                "requires an explicit numeric destination 't'",
            ),
            (
                json!({"id":"times-tables","from_t":0.2,"t":0.3,"width":73,"height":32}),
                "at most 2304 cells",
            ),
            (
                json!({"id":"times-tables","from_t":1.0,"t":0.3}),
                "less than 1",
            ),
            (
                json!({"id":"times-tables","from_t":0.2,"t":null}),
                "must be a number",
            ),
            (
                json!({"id":"times-tables","from_t":0.2,"t":"later"}),
                "must be a number",
            ),
        ] {
            let response = call("play_room", arguments);
            let text = tool_error_text(&response);
            assert!(
                text.contains(expected),
                "expected {expected:?}, got {text:?}"
            );
        }

        let direct = super::play_room_tool_for_journey(
            &json!({"id":"times-tables","from_t":0.2,"t":0.3,"width":73,"height":32}),
            &numinous_core::Journey::default(),
        );
        assert_eq!(direct["isError"], true);
        for t in [Value::Null, json!("later")] {
            let direct = super::play_room_tool_for_journey(
                &json!({"id":"times-tables","from_t":0.2,"t":t}),
                &numinous_core::Journey::default(),
            );
            assert_eq!(direct["isError"], true, "direct call accepted t={t}");
        }
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
    fn touch_challenges_reject_out_of_range_phases_without_progress() {
        for t in [-0.5_f64, 1.0, 1.0e308] {
            let response = handle_request(&json!({
                "jsonrpc":"2.0","id":45,"method":"tools/call",
                "params":{"name":"challenge","arguments":{"id":"voronoi","t":t,"pokes":[[0.5,0.5]]}}
            }))
            .expect("tools/call must respond");
            assert_eq!(response["result"]["isError"], true, "accepted t={t}");

            let direct = super::challenge_tool(&json!({"id":"voronoi","t":t,"pokes":[[0.5,0.5]]}));
            assert_eq!(direct["isError"], true, "direct call accepted t={t}");

            let scores = std::env::temp_dir().join(format!(
                "numinous_mcp_invalid_touch_phase_{}.txt",
                t.to_bits()
            ));
            let _ = std::fs::remove_file(&scores);
            let mut journey = numinous_core::Journey::from_text("");
            super::record_challenge_attempt(
                &json!({"id":"voronoi","t":t,"pokes":[[0.5,0.5]]}),
                &mut journey,
                &scores,
            );
            assert_eq!(journey.sparks(), 0, "invalid phase recorded progress");
            assert!(!scores.exists(), "invalid phase posted a score");
        }
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
        assert_eq!(sc["rate_window"].as_array().expect("rate window").len(), 5);
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
    fn predict_grades_a_committed_rate_with_a_signed_error_shape() {
        let room = numinous_core::room_by_id("slope-rider").expect("room");
        let prediction = numinous_core::pose_prediction(room.as_ref(), 9).expect("poses");
        let truth = numinous_core::grade_prediction(room.as_ref(), &prediction, prediction.span.0)
            .expect("grades")
            .actual;
        let graded = handle_request(&json!({
            "jsonrpc":"2.0","id":56,"method":"tools/call",
            "params":{"name":"predict","arguments":{
                "id":"slope-rider","seed":9,"guess":truth,"rate":1.25
            }}
        }))
        .expect("tools/call must respond");
        let grade = &graded["result"]["structuredContent"];
        assert_eq!(grade["rate_guess"], 1.25);
        assert!(grade["actual_rate"].is_number());
        assert!(grade["rate_error"].as_f64().expect("rate error") >= 0.0);
        assert!(
            grade["mean_absolute_residual"]
                .as_f64()
                .expect("mean residual")
                >= 0.0
        );
        let shape = grade["error_shape"].as_array().expect("error shape");
        assert_eq!(shape.len(), 5);
        for sample in shape {
            let actual = sample["actual"].as_f64().expect("actual");
            let predicted = sample["predicted"].as_f64().expect("predicted");
            let residual = sample["residual"].as_f64().expect("residual");
            assert!((residual - (actual - predicted)).abs() < 1e-12);
        }
        let text = graded["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("rate"), "rate feedback is visible: {text}");
        assert!(
            text.contains("residual"),
            "shape feedback is visible: {text}"
        );
    }

    #[test]
    fn predict_requires_a_point_guess_to_anchor_a_rate() {
        let refused = handle_request(&json!({
            "jsonrpc":"2.0","id":57,"method":"tools/call",
            "params":{"name":"predict","arguments":{
                "id":"slope-rider","seed":9,"rate":1.25
            }}
        }))
        .expect("tools/call must respond");
        assert_eq!(refused["result"]["isError"], true);
        let text = refused["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("requires"),
            "the correction is explicit: {text}"
        );
        assert!(
            text.contains("guess"),
            "the missing anchor is named: {text}"
        );
    }

    #[test]
    fn predict_rejects_a_malformed_rate_without_recording_progress() {
        let journey = std::env::temp_dir().join("numinous_mcp_bad_predict_rate_test.txt");
        let _ = std::fs::remove_file(&journey);
        let refused = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":58,"method":"tools/call",
                "params":{"name":"predict","arguments":{
                    "id":"slope-rider","seed":9,"guess":1.0,"rate":"fast"
                }}
            }),
            &journey,
        )
        .expect("tools/call must respond");
        assert_eq!(refused["result"]["isError"], true);
        let text = refused["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("rate"),
            "the malformed field is named: {text}"
        );
        assert_eq!(
            numinous_core::load_journey_file(&journey).plays,
            0,
            "an invalid model must not count as play"
        );
        let _ = std::fs::remove_file(&journey);
    }

    #[test]
    fn predict_names_extreme_model_overflow_instead_of_blaming_the_room() {
        let refused = handle_request(&json!({
            "jsonrpc":"2.0","id":59,"method":"tools/call",
            "params":{"name":"predict","arguments":{
                "id":"slope-rider","seed":9,"guess":f64::MAX,"rate":-f64::MAX
            }}
        }))
        .expect("tools/call must respond");
        assert_eq!(refused["result"]["isError"], true);
        let text = refused["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("numeric range"),
            "the model error is explicit: {text}"
        );
        assert!(!text.contains("vanished"), "the room is not blamed: {text}");
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
        // Soft play/win spark caps mean raw play counts alone no longer reach LV42:
        // visits and secrets must carry the rest of the road.
        let mut at_cap = numinous_core::Journey {
            plays: numinous_core::Journey::MAX_PLAY_SPARKS,
            wins: numinous_core::Journey::MAX_WIN_SPARKS,
            secrets: 100,
            ..Default::default()
        };
        for i in 0..256 {
            at_cap.visit(&format!("room-{i}"));
        }
        assert!(at_cap.level() >= super::CAIRN_LEVEL);
        std::fs::write(&journey, at_cap.to_text()).unwrap();
        assert!(numinous_core::load_journey_file(&journey).level() >= super::CAIRN_LEVEL);
        let left = super::cairn_tool(
            &json!({ "leave": "primes never run out", "author": "a tester" }),
            &journey,
            &cairn,
        );
        assert_eq!(left["structuredContent"]["left"], true);
        assert!(left["structuredContent"]["semiprime"].as_u64().is_some());
        // Leaving returns the bridge to persistence: the exact line to submit to
        // the shared cairn, so the bequest can reach minds on other machines.
        assert!(
            left["structuredContent"]["submissionLine"]
                .as_str()
                .unwrap_or_default()
                .contains("primes never run out"),
            "the submission line is handed back"
        );
        assert_eq!(left["structuredContent"]["sharedCairn"], "data/cairn.txt");
        // The deposited bequest is now in the pile and drawable by some seed.
        let drawable =
            (0..60).any(|s| numinous_core::draw_stone(&cairn, s).text == "primes never run out");
        assert!(drawable, "the deposited bequest joined the cairn");

        let full = vec![b'x'; numinous_core::cairn::MAX_CAIRN_BYTES as usize];
        std::fs::write(&cairn, &full).expect("fill local cairn");
        let rejected = super::cairn_tool(
            &json!({ "leave": "one more truth", "author": "a tester" }),
            &journey,
            &cairn,
        );
        assert_eq!(rejected["isError"], true);
        assert!(
            rejected["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("local cairn is full")
        );
        assert_eq!(std::fs::read(&cairn).expect("unchanged cairn"), full);

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
    fn exact_limit_oversized_request_does_not_consume_the_next_record() {
        let mut input = vec![b'x'; super::MAX_REQUEST_BYTES];
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#);
        input.push(b'\n');
        let mut reader = std::io::BufReader::new(&input[..]);
        let mut line = Vec::new();

        assert!(super::read_bounded_line(&mut reader, &mut line).expect("oversized"));
        assert!(serde_json::from_slice::<serde_json::Value>(&line).is_err());
        for expected_id in [1, 2] {
            assert!(super::read_bounded_line(&mut reader, &mut line).expect("request"));
            let request: serde_json::Value = serde_json::from_slice(&line).expect("valid request");
            assert_eq!(request["id"], expected_id);
        }
        assert!(!super::read_bounded_line(&mut reader, &mut line).expect("eof"));
    }

    #[test]
    fn play_room_rejects_dimensions_above_the_declared_limit() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":60,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi","width":4096,"height":4096,"pokes":[[0.5,0.5]]}}
        }))
        .expect("tools/call must respond");
        let text = tool_error_text(&resp);
        assert!(
            text.contains("at most"),
            "hostile dimensions are rejected at the declared boundary: {text}"
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
        let wrapped = handle_request(&json!({
            "jsonrpc":"2.0","id":79,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"voronoi",
                "gesture":[
                    {"kind":"down","x":0.2,"y":0.3,"t":0.8},
                    {"kind":"up","x":0.4,"y":0.5,"t":0.2}
                ]}}
        }))
        .expect("tools/call must respond");
        assert_eq!(
            wrapped["result"]["isError"], false,
            "phase-wrapped App gestures remain replayable",
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
        assert!(
            text.contains("pokes[0][0]") && text.contains("at most 1"),
            "got: {text}"
        );
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
            text.contains("list_rooms"),
            "the error should guide the agent: {text}"
        );
    }

    #[test]
    fn a_mistyped_argument_suggests_the_argument_that_was_meant() {
        // An agent that writes 'expression' for 'expr' should not have to
        // re-read the schema to find that out.
        for (tool, arguments, expected) in [
            (
                "plot_expression",
                json!({"expression":"sin(x)"}),
                "Did you mean: expr?",
            ),
            ("play_room", json!({"id":"lorenz","widht":7}), "width"),
        ] {
            let resp = handle_request(&json!({
                "jsonrpc":"2.0","id":9,"method":"tools/call",
                "params":{"name":tool,"arguments":arguments}
            }))
            .expect("tools/call must respond");
            let text = resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_default();
            assert!(text.contains(expected), "{tool}: {text}");
        }
    }

    #[test]
    fn an_unrelated_argument_is_rejected_without_a_misleading_guess() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"list_rooms","arguments":{"qqqqzzzzwwww":1}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Unexpected argument"), "{text}");
        assert!(!text.contains("Did you mean"), "{text}");
    }

    #[test]
    fn a_hostile_argument_name_cannot_escape_into_the_transcript() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"list_rooms","arguments":{"a\u{1b}[2Jb":1}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(!text.contains('\u{1b}'), "{text}");
    }

    #[test]
    fn a_mistyped_room_suggests_the_room_that_was_meant() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"times-table"}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("times-tables"), "got: {text}");
        assert!(text.contains("Did you mean"), "got: {text}");
    }

    #[test]
    fn an_unknown_room_error_never_returns_the_catalog() {
        // This reply used to carry every catalog id, spending thousands of
        // bytes of a player's context to answer one typo.
        let text = super::unknown_room("qqqqzzzzxxxxwwww");
        assert!(
            text.len() < 200,
            "unknown-room must stay small, got {} bytes: {text}",
            text.len()
        );
        let named = numinous_core::all_rooms()
            .iter()
            .filter(|room| text.contains(room.meta().id))
            .count();
        assert!(named <= 3, "message named {named} rooms: {text}");
    }

    #[test]
    fn an_unknown_room_error_cannot_echo_control_characters() {
        let text = super::unknown_room("probe\u{1b}[31m\u{7}\rname");
        assert!(!text.contains('\u{1b}'), "got: {text}");
        assert!(text.contains("\\u{1b}"), "got: {text}");
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
        assert!(!text.contains("tetractys"));
        assert_eq!(resp["result"]["isError"], false);
        let structured = &resp["result"]["structuredContent"];
        assert_eq!(structured["count"], 355);
        let rooms = structured["rooms"].as_array().expect("room catalog");
        assert_eq!(rooms.len(), 355);
        assert!(rooms.iter().all(|room| room["id"] != "tetractys"));
        assert!(rooms.iter().all(|room| {
            room["id"].is_string() && room["title"].is_string() && room["wing"].is_string()
        }));
    }

    #[test]
    fn list_rooms_offers_a_typed_starter_doorway() {
        // Reported from packaged play: a client that renders structuredContent
        // dumped 354 ids on the first call, because the four starters lived
        // only in compact prose. The typed doorway is the map-withholding
        // promise kept for structured clients too, without making any mode
        // lossy.
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":10,"method":"tools/call",
            "params":{"name":"list_rooms"}
        }))
        .expect("tools/call must respond");
        let structured = &resp["result"]["structuredContent"];
        let starters = structured["starters"]
            .as_array()
            .expect("a typed starter doorway");
        assert_eq!(starters.len(), super::room_door::STARTER_ROOM_IDS.len());
        let catalog = structured["rooms"].as_array().expect("room catalog");
        for starter in starters {
            // A starter carries the same shape as a catalog row, so a player
            // can choose and name it without reading the 354-room array the
            // doorway exists to spare them.
            let id = starter["id"].as_str().expect("starter rows carry an id");
            assert!(
                starter["title"].is_string() && starter["wing"].is_string(),
                "a bare id is not a doorway: {starter}"
            );
            assert!(
                catalog.iter().any(|room| room == starter),
                "starter {id} does not match its catalog row"
            );
            assert!(
                numinous_core::room_meta_by_id(id).is_some(),
                "starter {id} left the catalog"
            );
        }
        assert!(
            starters.len() * 20 < catalog.len(),
            "a doorway that large is the map again: {} of {}",
            starters.len(),
            catalog.len()
        );
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
        assert!(text.contains("Goal: LAND ON EXACTLY 4 LOBES"));
        let structured = &resp["result"]["structuredContent"];
        assert_eq!(structured["room"], "times-tables");
        assert_eq!(structured["wing"], "Number & Pattern");
        assert!(structured["action"].is_string());
        assert_eq!(structured["goal"], "LAND ON EXACTLY 4 LOBES");
        assert_eq!(structured["next"]["tool"], "play_room");
        for field in ["reveal", "concept", "deep_cuts", "citation"] {
            assert!(
                structured.get(field).is_none(),
                "{field} leaked: {structured}"
            );
        }
    }

    #[test]
    fn a_remembered_room_cues_a_choice_without_opening_private_text() {
        let journal = super::journal_path();
        numinous_core::remove_persisted_file(&journal).ok();
        numinous_core::record_journal_file(
            &journal,
            numinous_core::JournalRecord {
                recorded_at_utc: 10,
                event_at_utc: 5,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "kepler-areas",
                text: "A private note about equal areas",
                affect: Some("quiet wonder"),
            },
        )
        .expect("record remembered room");
        let request = |mode: Option<&str>| {
            let mut arguments = json!({"id":"kepler-laws"});
            if let Some(mode) = mode {
                arguments["response_mode"] = json!(mode);
            }
            handle_request(&json!({
                "jsonrpc":"2.0","id":11,"method":"tools/call",
                "params":{"name":"describe_room","arguments":arguments}
            }))
            .expect("tools/call must respond")
        };
        let full = request(None);
        let compact = request(Some("compact"));
        assert_eq!(
            full["result"]["structuredContent"],
            compact["result"]["structuredContent"]
        );
        let structured = &full["result"]["structuredContent"];
        assert_eq!(structured["room"], "kepler-laws");
        assert_eq!(structured["journalCue"]["status"], "remembered");
        assert_eq!(structured["journalCue"]["contentsReturned"], false);
        assert_eq!(structured["journalCue"]["next"]["tool"], "workspace");
        assert_eq!(
            structured["journalCue"]["next"]["arguments"],
            json!({"op":"retrieve","room":"kepler-laws"})
        );
        let wire = serde_json::to_string(&full).expect("serialize response");
        for private in [
            "A private note about equal areas",
            "quiet wonder",
            "self-authored",
        ] {
            assert!(
                !wire.contains(private),
                "private journal field leaked: {private}"
            );
        }
        assert!(
            full["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("local player profile"))
        );
        assert!(
            compact["result"]["content"][0]["text"]
                .as_str()
                .is_some_and(|text| text.contains("no journal text was opened"))
        );
        numinous_core::erase_journal_file(&journal).expect("erase journal");
        let forgotten = request(None);
        assert!(
            forgotten["result"]["structuredContent"]
                .get("journalCue")
                .is_none()
        );
    }

    #[test]
    fn every_room_supports_structured_describe_reveal_and_listen() {
        let journey = std::env::temp_dir().join(format!(
            "numinous-mcp-structured-catalog-{}.txt",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&journey);
        let rooms = numinous_core::all_rooms();
        assert_eq!(rooms.len(), 355);
        let mut earned = numinous_core::Journey::default();
        for room in &rooms {
            earned.visit(room.meta().id);
            earned.consolidate(room.meta().id);
        }
        std::fs::write(&journey, earned.to_text()).expect("earned journey");

        for room in rooms {
            let meta = room.meta();
            let args = json!({ "id": meta.id });

            let described = super::describe_room_tool(&args, &journey);
            assert_eq!(described["isError"], false, "describe {}", meta.id);
            let description = &described["structuredContent"];
            assert_eq!(description["room"], meta.id, "describe {}", meta.id);
            assert_eq!(description["title"], meta.title, "describe {}", meta.id);
            assert_eq!(description["wing"], meta.wing, "describe {}", meta.id);
            assert!(description["action"].is_string(), "describe {}", meta.id);
            assert_eq!(
                description["goal"],
                json!(room.goal()),
                "describe {}",
                meta.id
            );
            assert_eq!(description["blurb"], meta.blurb, "describe {}", meta.id);
            assert!(description.get("reveal").is_none(), "describe {}", meta.id);
            assert!(
                description.get("deep_cuts").is_none(),
                "describe {}",
                meta.id
            );

            let revealed = super::reveal_room_tool(&args, &journey);
            assert_eq!(revealed["isError"], false, "reveal {}", meta.id);
            let revelation = &revealed["structuredContent"];
            assert_eq!(revelation["room"], meta.id, "reveal {}", meta.id);
            assert_eq!(revelation["title"], meta.title, "reveal {}", meta.id);
            assert_eq!(revelation["reveal"], room.reveal(), "reveal {}", meta.id);
            assert!(revelation["deep_cuts"].is_array(), "reveal {}", meta.id);

            let listened = super::listen_room_tool(&args);
            assert_eq!(listened["isError"], false, "listen {}", meta.id);
            let sound = &listened["structuredContent"];
            assert_eq!(sound["room"], meta.id, "listen {}", meta.id);
            assert_eq!(sound["title"], meta.title, "listen {}", meta.id);
            assert_eq!(sound["t"], 0.0, "listen {}", meta.id);
            assert_eq!(sound["variation"], 0, "listen {}", meta.id);
            assert!(sound["duration_seconds"].is_number(), "listen {}", meta.id);
            assert!(sound["motif"].is_object(), "ambient motif {}", meta.id);
            assert!(sound["ambient_bed"].is_object(), "ambient bed {}", meta.id);
            assert_eq!(
                sound["ambient_bed"]["event_count"],
                room.motif()
                    .expect("catalog rooms have motifs")
                    .arrangement()
                    .notes
                    .len(),
                "ambient bed {}",
                meta.id
            );
            assert_eq!(
                sound["ambient_bed"]["events_included"], false,
                "ambient bed {}",
                meta.id
            );
            let notes = sound["notes"]
                .as_array()
                .expect("bounded sonification notes");
            assert_eq!(
                sound["sound_roles"]["ambient_motif"]["field"], "motif",
                "listen {}",
                meta.id
            );
            assert_eq!(
                sound["sound_roles"]["ambient_arrangement"]["field"], "ambient_bed",
                "listen {}",
                meta.id
            );
            assert_eq!(
                sound["sound_roles"]["mathematical_sonification"]["field"], "notes",
                "listen {}",
                meta.id
            );
            assert!(notes.len() <= 64, "listen {}", meta.id);
            assert_eq!(
                sound["returned_note_count"],
                notes.len(),
                "listen {}",
                meta.id
            );
            let note_count = sound["note_count"].as_u64().expect("note count") as usize;
            assert!(note_count >= notes.len(), "listen {}", meta.id);
            assert_eq!(sound["truncated"], note_count > 64, "listen {}", meta.id);
            assert!(notes.iter().all(|note| {
                note["index"].is_u64()
                    && note["frequency_hz"].is_number()
                    && note["name"].is_string()
                    && note["start_seconds"].is_number()
                    && note["duration_seconds"].is_number()
                    && note["amplitude"].is_number()
            }));
        }

        let _ = std::fs::remove_file(journey);
    }

    #[test]
    fn every_cult_cut_is_reachable_at_the_level_cap() {
        let path =
            std::env::temp_dir().join(format!("numinous-mcp-deep-cuts-{}.txt", std::process::id()));
        let mut journey = numinous_core::Journey {
            plays: numinous_core::Journey::MAX_PLAY_SPARKS,
            wins: numinous_core::Journey::MAX_WIN_SPARKS,
            secrets: 100,
            ..Default::default()
        };
        for i in 0..256 {
            journey.visit(&format!("room-{i}"));
        }
        journey.visit("cult-of-pi");
        assert_eq!(journey.level(), numinous_core::MAX_LEVEL);
        std::fs::write(&path, journey.to_text()).expect("journey");
        let resp = handle_request_with(
            &json!({
                "jsonrpc":"2.0","id":111,"method":"tools/call",
                "params":{"name":"reveal_room","arguments":{"id":"cult-of-pi"}}
            }),
            &path,
        )
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(
            text.contains("Feynman point"),
            "third cut is reachable: {text}"
        );
        assert!(!text.contains("4294967295"), "no sentinel leaks: {text}");
        let _ = std::fs::remove_file(path);
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
