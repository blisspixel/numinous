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
                         then play_room to render it as ASCII and see what the math does."
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
        None => tool_error(&unknown_room(id)),
    }
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
            room.render_ascii(&mut canvas, t);
            tool_text(&format!(
                "{} at t={t:.3}:\n\n{}",
                room.meta().title,
                canvas.to_text()
            ))
        }
        None => tool_error(&unknown_room(id)),
    }
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
        assert_eq!(tools.len(), 4);
        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"reveal_room"));
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
