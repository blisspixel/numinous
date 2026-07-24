//! End-to-end test of the MCP server as an agent client actually uses it:
//! spawn the real binary, speak newline-delimited JSON-RPC over stdio, and
//! walk every tool. Hermetic: journey and scores go to temp files.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use numinous_app::nim_render::draw_nim_board;
use numinous_app::session_viewer::{SessionViewer, ViewerInputMode, ViewerStatus};
use numinous_app::studio_render::{CurveLayout, draw_curve};
use numinous_broadcast::PublicTool;
use numinous_core::Raster;
use serde_json::{Value, json};

/// Run a full session: send each line, return the parsed response lines.
fn run_session(requests: &[Value]) -> Vec<Value> {
    run_session_with_barrier(requests, || true, &[])
}

/// Run requests on both sides of one externally observable session barrier.
fn run_session_with_barrier(
    before_barrier: &[Value],
    mut barrier: impl FnMut() -> bool,
    after_barrier: &[Value],
) -> Vec<Value> {
    static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let suffix = format!("{}-{session}", std::process::id());
    let journey = std::env::temp_dir().join(format!("numinous_mcp_e2e_journey_{suffix}.txt"));
    let scores = std::env::temp_dir().join(format!("numinous_mcp_e2e_scores_{suffix}.txt"));
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

    let mut stdout = child.stdout.take().expect("stdout");
    let output_reader = thread::spawn(move || {
        let mut output = Vec::new();
        stdout.read_to_end(&mut output).expect("read MCP output");
        output
    });
    let mut stdin = child.stdin.take().expect("stdin");
    for request in before_barrier {
        writeln!(stdin, "{request}").expect("write request before barrier");
    }
    stdin.flush().expect("flush requests before barrier");

    let barrier_deadline = Instant::now() + Duration::from_secs(5);
    while !barrier() {
        if Instant::now() >= barrier_deadline {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            let _ = output_reader.join();
            let _ = std::fs::remove_file(&journey);
            let _ = std::fs::remove_file(&scores);
            panic!("MCP session barrier did not resolve within 5 seconds");
        }
        thread::sleep(Duration::from_millis(5));
    }
    for request in after_barrier {
        writeln!(stdin, "{request}").expect("write request after barrier");
    }
    drop(stdin);

    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().expect("inspect MCP process") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = output_reader.join();
            let _ = std::fs::remove_file(&journey);
            let _ = std::fs::remove_file(&scores);
            panic!("MCP server did not exit within 30 seconds");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = output_reader.join().expect("MCP output reader");

    assert!(status.success(), "server exited with an error");
    let _ = std::fs::remove_file(&journey);
    let _ = std::fs::remove_file(&scores);

    String::from_utf8(stdout)
        .expect("utf8 output")
        .lines()
        .map(|line| serde_json::from_str(line).expect("every reply is valid JSON"))
        .collect()
}

#[test]
fn compact_mode_is_discoverable_and_compatible_over_real_stdio() {
    let call = |id: u64, mode: Option<&str>| {
        let mut arguments = json!({"id":"times-tables","t":0.25});
        if let Some(mode) = mode {
            arguments
                .as_object_mut()
                .expect("arguments object")
                .insert("response_mode".to_string(), json!(mode));
        }
        json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":"play_room","arguments":arguments}
        })
    };
    let replies = run_session(&[
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"numinous-test","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        call(2, None),
        call(3, Some("full")),
        call(4, Some("compact")),
        call(5, Some("brief")),
        json!({"jsonrpc":"2.0","id":6,"method":"ping"}),
    ]);
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    assert_eq!(by_id(0)["result"]["protocolVersion"], "2025-06-18");

    let play_schema = by_id(1)["result"]["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "play_room"))
        .expect("play_room schema");
    assert_eq!(
        play_schema["inputSchema"]["properties"]["response_mode"]["enum"],
        json!(["full", "compact"])
    );
    assert_eq!(by_id(2)["result"], by_id(3)["result"]);
    assert_eq!(
        by_id(2)["result"]["structuredContent"],
        by_id(4)["result"]["structuredContent"]
    );
    assert!(text_of(by_id(4)).len() < text_of(by_id(2)).len());
    assert!(text_of(by_id(4)).contains("structuredContent.render"));
    assert_eq!(by_id(5)["result"]["isError"], true);
    assert!(text_of(by_id(5)).contains("must be one of"));
    assert!(
        by_id(6)["result"].is_object(),
        "server continues after error"
    );
}

/// The text content of a tool-call response.
fn text_of(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

#[test]
fn app_viewer_follows_a_real_times_tables_agent_session() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    // Wait until the viewer has absorbed every public projection before stop.
    // Stop clears unwritten queue frames, so racing stop after reveal drops
    // RevealRoom under CI load.
    let replies = run_session_with_barrier(
        &[
            json!({
                "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                    "protocolVersion":"2025-06-18",
                    "capabilities":{},
                    "clientInfo":{"name":"viewer-acceptance","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            call(
                1,
                "broadcast_session",
                json!({"action":"start", "pairing_code": pairing_code}),
            ),
            call(2, "journey", json!({})),
            call(
                3,
                "play_room",
                json!({"id":"times-tables","t":0.2,"width":40,"height":20,"variation":42}),
            ),
            call(4, "challenge", json!({"id":"times-tables","seed":7})),
            call(
                5,
                "challenge",
                json!({"id":"times-tables","seed":7,"t":0.81,"pokes":[[0.375,0.5]]}),
            ),
            call(
                6,
                "play_room",
                json!({
                    "id":"times-tables","t":0.81,"width":40,"height":20,
                    "variation":42,"pokes":[[0.375,0.5]]
                }),
            ),
            call(7, "reveal_room", json!({"id":"times-tables"})),
            call(8, "journey", json!({})),
        ],
        || {
            viewer.retained_events().len() >= 5
                && viewer
                    .retained_events()
                    .last()
                    .is_some_and(|event| event.event.tool == PublicTool::RevealRoom)
        },
        &[call(9, "broadcast_session", json!({"action":"stop"}))],
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(
        by_id(6)["result"]["structuredContent"]["status"],
        "K 5.00  CLOSED  4 LOBES  FOUND"
    );
    assert_eq!(by_id(6)["result"]["structuredContent"]["goalMet"], true);
    assert_eq!(
        by_id(6)["result"]["structuredContent"]["engineeredAha"]["earn"],
        "four-lobes"
    );
    assert!(text_of(by_id(7)).contains("Mandelbrot"));
    assert_eq!(by_id(9)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(3);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(
        events
            .iter()
            .map(|event| event.event.tool)
            .collect::<Vec<_>>(),
        [
            PublicTool::PlayRoom,
            PublicTool::Challenge,
            PublicTool::Challenge,
            PublicTool::PlayRoom,
            PublicTool::RevealRoom,
        ]
    );
    assert_eq!(
        events
            .iter()
            .map(|event| event.public_sequence)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert!(events.iter().all(|event| event.skipped.is_none()));
    viewer.scrub(-1);
    let k5_frame = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
    assert!(
        k5_frame.lit_count() > 1_000,
        "the retained K5 action reconstructs a native room frame"
    );
    let room_audio = viewer
        .audio_selection()
        .expect("the retained K5 action selects local sound");
    assert_eq!(room_audio.public_sequence(), 3);
    let room = numinous_core::all_rooms_with(42)
        .into_iter()
        .find(|room| room.meta().id == "times-tables")
        .expect("Times Tables variation");
    let inputs = numinous_core::inputs_from_pokes(&[(0.375, 0.5)], 0.81);
    assert_eq!(
        room_audio.render(8_000),
        Some(room.sound_input(0.81, &inputs).render(8_000)),
        "the real selected room replays exact shared core sound"
    );
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }

    viewer.close();
    assert_eq!(viewer.status(), ViewerStatus::Closed);
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn app_viewer_reconstructs_a_real_studio_agent_creation() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    let replies = run_session(&[
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"studio-viewer-acceptance","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(
            1,
            "broadcast_session",
            json!({"action":"start", "pairing_code": pairing_code}),
        ),
        call(
            2,
            "plot_expression",
            json!({
                "expr":"sin(a*x) + x/3", "xmin":-4.0, "xmax":5.0, "a":2.0
            }),
        ),
        call(3, "broadcast_session", json!({"action":"stop"})),
    ]);
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert!(text_of(by_id(2)).contains("sin(a*x) + x/3"));
    assert_eq!(by_id(3)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(2);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.tool, PublicTool::PlotExpression);
    assert_eq!(events[0].public_sequence, 0);
    assert!(events[0].skipped.is_none());
    let frame = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
    let expression = numinous_core::parse("sin(a*x) + x/3").expect("accepted expression");
    let mut expected = Raster::with_accent(360, 220, [198, 132, 255]);
    draw_curve(
        &mut expected,
        CurveLayout {
            width: 360,
            height: 220,
            top: 35.0,
            bottom_margin: 18.0,
        },
        -4.0,
        5.0,
        |x| Some(numinous_core::eval(&expression, x, 2.0)),
    )
    .expect("expected native Studio curve");
    let actual_rgba = frame.to_rgba();
    let expected_rgba = expected.to_rgba();
    let body_start = 31 * 360 * 4;
    let body_end = (220 - 13) * 360 * 4;
    assert_eq!(
        &actual_rgba[body_start..body_end],
        &expected_rgba[body_start..body_end],
        "the retained expression reconstructs the exact native Studio body outside viewer chrome"
    );
    let body_lit = expected_rgba[body_start..body_end]
        .chunks_exact(4)
        .filter(|pixel| *pixel != [10, 11, 15, 255])
        .count();
    assert!(body_lit > 100, "the native Studio body contains a curve");
    let studio_audio = viewer
        .audio_selection()
        .expect("the retained expression selects local sound");
    assert_eq!(studio_audio.public_sequence(), 0);
    assert_eq!(
        studio_audio.render(8_000),
        Some(numinous_core::to_melody(&expression, -4.0, 5.0, 32, 2.0).render(8_000)),
        "the real selected expression replays exact shared Studio sound"
    );
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "studio-viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }

    viewer.close();
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn app_viewer_reconstructs_a_real_normalized_nim_agent_opening() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    let before_stop = [
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"nim-viewer-acceptance","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(
            1,
            "broadcast_session",
            json!({"action":"start", "pairing_code": pairing_code}),
        ),
        call(2, "nim", json!({"seed": 23, "daily": false})),
        call(
            3,
            "nim",
            json!({
                "seed": 23,
                "moves": vec![json!([1, 1]); numinous_core::nim::MAX_REPLAY_TURNS + 1]
            }),
        ),
        call(4, "nim", json!({"seed": -1})),
    ];
    let after_stop = [call(5, "broadcast_session", json!({"action":"stop"}))];
    let replies = run_session_with_barrier(
        &before_stop,
        || viewer.retained_events().len() == 1,
        &after_stop,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(by_id(2)["result"]["structuredContent"]["game"], "nim");
    assert_eq!(by_id(2)["result"]["structuredContent"]["seed"], 23);
    assert_eq!(by_id(3)["result"]["isError"], true);
    assert!(text_of(by_id(3)).contains("at most 64"));
    assert_eq!(by_id(4)["result"]["isError"], true);
    assert!(text_of(by_id(4)).contains("at least 0"));
    assert_eq!(by_id(5)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(2);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.tool, PublicTool::Nim);
    assert_eq!(events[0].public_sequence, 0);
    assert_eq!(
        Value::Object(events[0].event.arguments.clone()),
        json!({"seed": 23})
    );
    assert!(events[0].skipped.is_none());

    let frame = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
    let replay = numinous_core::nim::replay(23, &[]).expect("opening Nim replay");
    let expected = draw_nim_board(&replay.heaps, None, 360, 220).expect("native Nim board");
    let actual_rgba = frame.to_rgba();
    let expected_rgba = expected.to_rgba();
    let body_start = 31 * 360 * 4;
    let body_end = (220 - 13) * 360 * 4;
    assert_eq!(
        &actual_rgba[body_start..body_end],
        &expected_rgba[body_start..body_end],
        "the retained Nim action reconstructs the exact native game body outside viewer chrome"
    );
    let body_lit = expected_rgba[body_start..body_end]
        .chunks_exact(4)
        .filter(|pixel| *pixel != [10, 11, 15, 255])
        .count();
    assert!(body_lit > 100, "the native Nim body contains heap geometry");
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "nim-viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }

    viewer.close();
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn app_viewer_reconstructs_a_real_munch_agent_opening() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    let seed = 7_u64;
    let before_stop = [
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"munch-viewer-acceptance","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(
            1,
            "broadcast_session",
            json!({"action":"start", "pairing_code": pairing_code}),
        ),
        call(2, "munch", json!({"seed": seed})),
        call(3, "munch", json!({"seed": seed, "bites": [0]})),
        call(4, "journey", json!({})),
    ];
    let after_stop = [call(5, "broadcast_session", json!({"action":"stop"}))];
    let replies = run_session_with_barrier(
        &before_stop,
        || viewer.retained_events().len() == 1,
        &after_stop,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(by_id(2)["result"]["structuredContent"]["game"], "munch");
    assert_eq!(
        by_id(2)["result"]["structuredContent"]["round"],
        numinous_core::FULL_DECK_ROUND
    );
    assert_eq!(by_id(3)["result"]["isError"], true);
    assert_eq!(by_id(5)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(2);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.tool, PublicTool::Munch);
    assert_eq!(events[0].public_sequence, 0);
    assert_eq!(
        Value::Object(events[0].event.arguments.clone()),
        json!({"seed": seed})
    );

    let frame = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
    let board = numinous_core::build_board(seed, numinous_core::FULL_DECK_ROUND);
    let expected_play = numinous_app::play::MunchPlay {
        board,
        seed,
        round: numinous_core::FULL_DECK_ROUND,
        cursor: 30,
        bites: std::collections::BTreeSet::new(),
        graded: None,
        bite_flash: None,
    };
    let expected = numinous_app::game_draw::draw_munch(
        &expected_play,
        0,
        numinous_app::input_legend::InputMode::KeyboardMouse,
        numinous_app::input_legend::ControllerFace::Generic,
        360,
        220,
    );
    let actual_rgba = frame.to_rgba();
    let expected_rgba = expected.to_rgba();
    let body_start = 31 * 360 * 4;
    let body_end = (220 - 13) * 360 * 4;
    assert_eq!(
        &actual_rgba[body_start..body_end],
        &expected_rgba[body_start..body_end],
        "the retained Munch action reconstructs the exact native game body outside viewer chrome"
    );
    let body_lit = expected_rgba[body_start..body_end]
        .chunks_exact(4)
        .filter(|pixel| *pixel != [10, 11, 15, 255])
        .count();
    assert!(
        body_lit > 100,
        "the native Munch body contains board geometry"
    );
    let munch_audio = viewer
        .audio_selection()
        .expect("the retained Munch opening selects local sound");
    assert_eq!(munch_audio.public_sequence(), 0);
    let expected_sound =
        numinous_core::SoundSpec::tone(196.0 + (seed % 5) as f32 * 16.0, 2.0, 0.04);
    assert_eq!(
        munch_audio.render(8_000),
        Some(expected_sound.render(8_000)),
        "the real selected Munch opening replays exact shared game sound"
    );
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "munch-viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }

    viewer.close();
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn app_viewer_reconstructs_a_real_arcade_agent_opening() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    let seed = 11_u64;
    let before_stop = [
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"arcade-viewer-acceptance","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(
            1,
            "broadcast_session",
            json!({"action":"start", "pairing_code": pairing_code}),
        ),
        call(2, "munch_arcade", json!({"seed": seed})),
        call(3, "munch_arcade", json!({"seed": seed, "actions": ["fly"]})),
        call(4, "scores", json!({})),
    ];
    let after_stop = [call(5, "broadcast_session", json!({"action":"stop"}))];
    let replies = run_session_with_barrier(
        &before_stop,
        || viewer.retained_events().len() == 1,
        &after_stop,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(by_id(2)["result"]["structuredContent"]["game"], "arcade");
    assert_eq!(by_id(2)["result"]["structuredContent"]["seed"], seed);
    assert_eq!(by_id(3)["result"]["isError"], true);
    assert_eq!(by_id(5)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(2);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.tool, PublicTool::MunchArcade);
    assert_eq!(events[0].public_sequence, 0);

    let frame = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
    let run = numinous_core::munch_arcade::Arcade::new(seed);
    let expected_play = numinous_app::play::ArcadePlay {
        run,
        seed,
        flash: None,
        over: false,
    };
    let expected = numinous_app::game_draw::draw_arcade(
        &expected_play,
        numinous_app::input_legend::InputMode::KeyboardMouse,
        numinous_app::input_legend::ControllerFace::Generic,
        360,
        220,
    );
    let actual_rgba = frame.to_rgba();
    let expected_rgba = expected.to_rgba();
    let body_start = 31 * 360 * 4;
    let body_end = (220 - 13) * 360 * 4;
    assert_eq!(
        &actual_rgba[body_start..body_end],
        &expected_rgba[body_start..body_end],
        "the retained Arcade action reconstructs the exact native game body outside viewer chrome"
    );
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "arcade-viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }
    viewer.close();
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn app_viewer_reconstructs_a_real_quiz_agent_opening() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    let seed = 13_u64;
    let before_stop = [
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"quiz-viewer-acceptance","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(
            1,
            "broadcast_session",
            json!({"action":"start", "pairing_code": pairing_code}),
        ),
        call(2, "quiz", json!({"seed": seed})),
        // Schema rejects one choice before capture, so no public event is emitted.
        call(3, "quiz", json!({"seed": seed, "choices": 1})),
        call(4, "trophies", json!({})),
    ];
    let after_stop = [call(5, "broadcast_session", json!({"action":"stop"}))];
    let replies = run_session_with_barrier(
        &before_stop,
        || viewer.retained_events().len() == 1,
        &after_stop,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(by_id(2)["result"]["structuredContent"]["game"], "quiz");
    assert_eq!(by_id(2)["result"]["structuredContent"]["seed"], seed);
    assert_eq!(by_id(2)["result"]["structuredContent"]["choiceCount"], 4);
    assert_eq!(by_id(3)["result"]["isError"], true);
    assert_eq!(by_id(5)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(2);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.tool, PublicTool::Quiz);
    assert_eq!(events[0].public_sequence, 0);

    let frame = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
    let rooms = numinous_core::all_rooms();
    let round = numinous_core::build_round_sized(seed, 0, 54, 22, 4);
    let expected_play = numinous_app::play::QuizPlay { round, flash: None };
    let expected = numinous_app::game_draw::draw_quiz(
        &rooms,
        &expected_play,
        numinous_app::input_legend::InputMode::KeyboardMouse,
        numinous_app::input_legend::ControllerFace::Generic,
        360,
        220,
    );
    let actual_rgba = frame.to_rgba();
    let expected_rgba = expected.to_rgba();
    let body_start = 31 * 360 * 4;
    let body_end = (220 - 13) * 360 * 4;
    assert_eq!(
        &actual_rgba[body_start..body_end],
        &expected_rgba[body_start..body_end],
        "the retained Quiz action reconstructs the exact native game body outside viewer chrome"
    );
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "quiz-viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }
    viewer.close();
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn app_viewer_reconstructs_a_real_gauntlet_agent_opening() {
    let mut viewer = SessionViewer::default();
    viewer.open().expect("open the App session viewer");
    let pairing_code = viewer.pairing_code().expect("fresh pairing code");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        })
    };
    let seed = 17_u64;
    let before_stop = [
        json!({
            "jsonrpc":"2.0","id":0,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"gauntlet-viewer-acceptance","version":"1.0"}
            }
        }),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(
            1,
            "broadcast_session",
            json!({"action":"start", "pairing_code": pairing_code}),
        ),
        call(2, "gauntlet", json!({"seed": seed})),
        // Unexpected answers fields fail schema before public capture.
        call(
            3,
            "gauntlet",
            json!({"seed": seed, "answers": {"private": true}}),
        ),
        call(4, "choose", json!({})),
    ];
    let after_stop = [call(5, "broadcast_session", json!({"action":"stop"}))];
    let replies = run_session_with_barrier(
        &before_stop,
        || viewer.retained_events().len() == 1,
        &after_stop,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(by_id(2)["result"]["structuredContent"]["game"], "gauntlet");
    assert_eq!(by_id(2)["result"]["structuredContent"]["seed"], seed);
    assert_eq!(by_id(2)["result"]["structuredContent"]["stages"], 4);
    assert_eq!(by_id(3)["result"]["isError"], true);
    assert_eq!(by_id(5)["result"]["structuredContent"]["state"], "stopped");

    let deadline = Instant::now() + Duration::from_secs(2);
    while viewer.status() != ViewerStatus::GuestStopped {
        assert!(Instant::now() < deadline, "viewer stop marker timed out");
        thread::sleep(Duration::from_millis(5));
    }
    let events = viewer.retained_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.tool, PublicTool::Gauntlet);
    assert_eq!(events[0].public_sequence, 0);

    let frame = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
    let rooms = numinous_core::all_rooms();
    let board = numinous_core::build_board(seed, 0);
    let expected_play = numinous_app::play::GauntletPlay {
        seed,
        stage: 0,
        munch: numinous_app::play::MunchPlay {
            board,
            seed,
            round: 0,
            cursor: 30,
            bites: std::collections::BTreeSet::new(),
            graded: None,
            bite_flash: None,
        },
        quiz: numinous_app::play::QuizPlay {
            round: numinous_core::build_round(seed, 1, 44, 18),
            flash: None,
        },
        scan: numinous_core::build_scan(seed, 4),
        secret: numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4),
        wire: String::new(),
        wire_lines: Vec::new(),
        scores: Vec::new(),
        cleared: Vec::new(),
        message: String::new(),
    };
    let expected = numinous_app::game_draw::draw_gauntlet(
        &rooms,
        &expected_play,
        0,
        numinous_app::input_legend::InputMode::KeyboardMouse,
        numinous_app::input_legend::ControllerFace::Generic,
        360,
        220,
    );
    let actual_rgba = frame.to_rgba();
    let expected_rgba = expected.to_rgba();
    let body_start = 31 * 360 * 4;
    let body_end = (220 - 13) * 360 * 4;
    assert_eq!(
        &actual_rgba[body_start..body_end],
        &expected_rgba[body_start..body_end],
        "the retained Gauntlet action reconstructs the exact native game body outside viewer chrome"
    );
    let public_bytes = serde_json::to_string(&events).expect("serialize public evidence");
    for forbidden in [
        "gauntlet-viewer-acceptance",
        "clientInfo",
        "jsonrpc",
        "pairing_code",
        "NUMINOUS_JOURNEY",
        "NUMINOUS_SCORES",
    ] {
        assert!(
            !public_bytes.contains(forbidden),
            "public evidence contained private field {forbidden}"
        );
    }
    viewer.close();
    assert!(viewer.retained_events().is_empty());
}

#[test]
fn a_full_agent_session_walks_every_tool() {
    let call = |id: u64, name: &str, args: Value| {
        json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
               "params":{"name":name,"arguments":args}})
    };
    let requests = vec![
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"numinous-test","version":"1.0"}
            }
        }),
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
        call(23, "challenge", json!({"id":"voronoi","seed":7})),
        call(
            24,
            "challenge",
            json!({"id":"voronoi","seed":7,"pokes":[[0.5,0.5]]}),
        ),
        call(25, "broadcast_session", json!({"action":"status"})),
    ];
    let replies = run_session(&requests);

    // 25 id-carrying requests, one notification with no reply.
    assert_eq!(replies.len(), 25, "one reply per id-carrying request");
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    assert_eq!(by_id(1)["result"]["serverInfo"]["name"], "numinous");
    assert_eq!(
        by_id(2)["result"]["tools"].as_array().map(Vec::len),
        Some(33)
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
    assert!(
        text_of(by_id(23)).contains("CELLS CHANGE INSIDE"),
        "the challenge poses its goal"
    );
    let graded = &by_id(24)["result"]["structuredContent"];
    assert!(
        graded["score"].as_u64().is_some(),
        "the attempt is graded with metrics: {graded}"
    );
    assert_eq!(
        by_id(25)["result"]["structuredContent"]["state"],
        "disabled"
    );
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
