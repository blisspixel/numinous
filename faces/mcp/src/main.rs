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

use numinous_core::{Canvas, all_rooms, room_by_id};
use serde_json::{Value, json};

/// The MCP protocol revision this server targets.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Default ASCII canvas size for `play_room`.
const DEFAULT_WIDTH: u64 = 72;
const DEFAULT_HEIGHT: u64 = 32;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
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

/// Write a single JSON-RPC message as one newline-terminated line.
fn write_message(out: &mut impl Write, message: &Value) -> io::Result<()> {
    writeln!(out, "{message}")?;
    out.flush()
}

/// Handle one JSON-RPC request. Returns `None` for notifications (no `id`),
/// which receive no response.
fn handle_request(request: &Value) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let result = match method {
        "initialize" => Ok(initialize_result()),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => call_tool(request.get("params")),
        "ping" => Ok(json!({})),
        other => Err((-32601_i64, format!("Method not found: {other}"))),
    };

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
                "description": "Play a room: render it and get back an ASCII picture of the result, so you can see what the math does.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Room id, for example times-tables." },
                        "t": { "type": "number", "description": "Phase in [0,1). For Times Tables this sweeps the multiplier." },
                        "width": { "type": "integer", "description": "ASCII canvas width in columns." },
                        "height": { "type": "integer", "description": "ASCII canvas height in rows." }
                    },
                    "required": ["id"],
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
                        "t": { "type": "number", "description": "Phase in [0,1)." }
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
                "name": "quiz",
                "description": "Play Guess the Shape. Call with seed and round to get a mystery render and lettered choices; call again with your guess (a letter) to learn if you were right and why.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed and round give the same puzzle." },
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
fn call_tool(params: Option<&Value>) -> Result<Value, (i64, String)> {
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
        "describe_room" => Ok(describe_room_tool(&args)),
        "reveal_room" => Ok(reveal_room_tool(&args)),
        "play_room" => Ok(play_room_tool(&args)),
        "listen_room" => Ok(listen_room_tool(&args)),
        "list_sims" => Ok(tool_text(&list_sims_text())),
        "run_sim" => Ok(run_sim_tool(&args)),
        "quiz" => Ok(quiz_tool(&args)),
        other => Err((-32602_i64, format!("Unknown tool: {other}"))),
    }
}

fn describe_room_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    match room_by_id(id) {
        Some(room) => {
            let m = room.meta();
            tool_text(&format!(
                "{} ({})\nWing: {}\n\n{}\n\nReveal: {}",
                m.title,
                m.id,
                m.wing,
                m.blurb,
                room.reveal()
            ))
        }
        // Not every name is a room. A few of them answer anyway.
        None => match numinous_core::akousma(id) {
            Some(whisper) => tool_text(whisper),
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
    let Some(room) = room_by_id(id) else {
        return tool_error(&unknown_room(id));
    };
    let spec = room.sound(t);
    let mut lines = vec![format!(
        "{} at t={t:.3}: {:.1}s of sound, {} notes.",
        room.meta().title,
        spec.duration,
        spec.notes.len()
    )];
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

fn play_room_tool(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'id'.");
    };
    let t = args.get("t").and_then(Value::as_f64).unwrap_or(0.0);
    let width = args
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_WIDTH) as usize;
    let height = args
        .get("height")
        .and_then(Value::as_u64)
        .unwrap_or(DEFAULT_HEIGHT) as usize;

    match room_by_id(id) {
        Some(room) => {
            let mut canvas = Canvas::new(width, height);
            room.render(&mut canvas, t);
            tool_text(&format!(
                "{} at t={t:.3}:\n\n{}",
                room.meta().title,
                canvas.to_text()
            ))
        }
        None => tool_error(&unknown_room(id)),
    }
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
fn quiz_tool(args: &Value) -> Value {
    let seed = args.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let round = args.get("round").and_then(Value::as_u64).unwrap_or(0);
    let quiz = numinous_core::build_round(seed, round, 54, 22);
    match args.get("guess").and_then(Value::as_str) {
        Some(guess) => {
            let letter = guess.trim().chars().next().map(|c| c.to_ascii_uppercase());
            let verdict = if letter == Some(quiz.answer) {
                "Correct!"
            } else {
                "Not quite."
            };
            tool_text(&format!(
                "{verdict} The answer was {} ({}).\n\n{}",
                quiz.answer, quiz.answer_title, quiz.answer_reveal
            ))
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
    use super::handle_request;
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
        assert_eq!(tools.len(), 8);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"reveal_room"));
        assert!(names.contains(&"run_sim"));
        assert!(names.contains(&"quiz"));
        assert!(names.contains(&"listen_room"));
    }

    #[test]
    fn listen_room_returns_readable_notation() {
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":30,"method":"tools/call",
            "params":{"name":"listen_room","arguments":{"id":"lissajous","t":0.0}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        assert!(text.contains("Hz"), "got: {text}");
        assert!(
            text.contains("2 notes"),
            "the lissajous chord has two notes"
        );
        assert_eq!(resp["result"]["isError"], false);
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
        let resp = handle_request(&json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"times-tables","width":40,"height":20}}
        }))
        .expect("tools/call must respond");
        let text = resp["result"]["content"][0]["text"]
            .as_str()
            .expect("text content");
        assert!(text.contains('*'), "the render should contain ink");
        assert_eq!(resp["result"]["isError"], false);
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
