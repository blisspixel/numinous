//! End-to-end test of the MCP server as an agent client actually uses it:
//! spawn the real binary, speak newline-delimited JSON-RPC over stdio, and
//! walk every tool. Hermetic: journey and scores go to temp files.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// Run a full session: send each line, return the parsed response lines.
fn run_session(requests: &[Value]) -> Vec<Value> {
    let journey = std::env::temp_dir().join("numinous_mcp_e2e_journey.txt");
    let scores = std::env::temp_dir().join("numinous_mcp_e2e_scores.txt");
    let _ = std::fs::remove_file(&journey);
    let _ = std::fs::remove_file(&scores);

    let mut child = Command::new(env!("CARGO_BIN_EXE_numinous-mcp"))
        .env("NUMINOUS_JOURNEY", &journey)
        .env("NUMINOUS_SCORES", &scores)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the MCP server");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write request");
        }
    } // closing stdin ends the session

    let output = child.wait_with_output().expect("server exits cleanly");
    assert!(output.status.success(), "server exited with an error");
    let _ = std::fs::remove_file(&journey);
    let _ = std::fs::remove_file(&scores);

    String::from_utf8(output.stdout)
        .expect("utf8 output")
        .lines()
        .map(|line| serde_json::from_str(line).expect("every reply is valid JSON"))
        .collect()
}

/// The text content of a tool-call response.
fn text_of(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

#[test]
fn a_full_agent_session_walks_every_tool() {
    let call = |id: u64, name: &str, args: Value| {
        json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
               "params":{"name":name,"arguments":args}})
    };
    let requests = vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}), // no reply
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        call(3, "list_rooms", json!({})),
        call(4, "describe_room", json!({"id":"mandelbrot"})),
        call(5, "reveal_room", json!({"id":"times-tables"})),
        call(
            6,
            "play_room",
            json!({"id":"lorenz","t":0.7,"width":50,"height":24}),
        ),
        call(7, "listen_room", json!({"id":"lissajous","t":0.0})),
        call(8, "list_sims", json!({})),
        call(
            9,
            "run_sim",
            json!({"id":"wing","params":{"angle-of-attack":20}}),
        ),
        call(10, "quiz", json!({"seed":7,"round":0})),
        call(11, "quiz", json!({"seed":7,"round":0,"guess":"A"})),
        call(12, "plot_expression", json!({"expr":"sin(3*x) + x/2"})),
        call(13, "sing_expression", json!({"expr":"x","notes":6})),
        call(14, "explain_joke", json!({})),
        call(15, "munch", json!({"seed":7,"round":0})),
        call(16, "munch", json!({"seed":7,"round":0,"bites":[1,2,3]})),
        call(17, "describe_room", json!({"id":"hippasus"})), // the whisper
        call(18, "journey", json!({})),
        call(19, "scores", json!({})),
        json!({"jsonrpc":"2.0","id":20,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":21,"method":"no-such-method"}),
        call(22, "no_such_tool", json!({})),
    ];
    let replies = run_session(&requests);

    // 22 id-carrying requests, one notification with no reply.
    assert_eq!(replies.len(), 22, "one reply per id-carrying request");
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    assert_eq!(by_id(1)["result"]["serverInfo"]["name"], "numinous");
    assert_eq!(
        by_id(2)["result"]["tools"].as_array().map(Vec::len),
        Some(15)
    );
    assert!(text_of(by_id(3)).contains("times-tables"));
    assert!(text_of(by_id(4)).contains("Fractals"));
    assert!(text_of(by_id(5)).contains("Mandelbrot"));
    assert!(text_of(by_id(6)).contains('#'), "the butterfly has ink");
    assert!(text_of(by_id(7)).contains("Hz"));
    assert!(text_of(by_id(8)).contains("tribbles"));
    assert!(text_of(by_id(9)).contains("STALL"));
    assert!(text_of(by_id(10)).contains("Guess the shape"));
    assert!(by_id(11)["result"]["structuredContent"]["correct"].is_boolean());
    assert!(text_of(by_id(12)).contains('#'));
    assert!(text_of(by_id(13)).contains("6 notes"));
    assert!(text_of(by_id(14)).contains("frog"));
    assert!(text_of(by_id(15)).contains("[ 1]"));
    let munched = &by_id(16)["result"]["structuredContent"];
    assert!(munched["score"].is_i64() || munched["score"].is_u64());
    assert!(munched["missed"].is_array(), "dense feedback rides along");
    assert!(text_of(by_id(17)).contains("sea"), "the whisper answers");
    let journey = &by_id(18)["result"]["structuredContent"];
    assert!(
        journey["xp"].as_u64().unwrap_or(0) > 0,
        "the session itself earned XP: {journey}"
    );
    assert!(text_of(by_id(19)).contains("HIGH SCORES"), "munch posted");
    assert!(by_id(20)["result"].is_object());
    assert_eq!(by_id(21)["error"]["code"], -32601);
    assert_eq!(by_id(22)["error"]["code"], -32602);
}

#[test]
fn malformed_input_gets_a_parse_error_and_the_server_keeps_going() {
    let journey = std::env::temp_dir().join("numinous_mcp_e2e_parse_journey.txt");
    let _ = std::fs::remove_file(&journey);
    let mut child = Command::new(env!("CARGO_BIN_EXE_numinous-mcp"))
        .env("NUMINOUS_JOURNEY", &journey)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "this is not json").expect("write");
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"ping"}}"#).expect("write");
    }
    let output = child.wait_with_output().expect("exit");
    let lines: Vec<Value> = String::from_utf8(output.stdout)
        .expect("utf8")
        .lines()
        .map(|l| serde_json::from_str(l).expect("valid json"))
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["error"]["code"], -32700, "parse error reported");
    assert!(
        lines[1]["result"].is_object(),
        "and the server kept serving"
    );
    let _ = std::fs::remove_file(&journey);
}
