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

const SESSION_BARRIER_TIMEOUT: Duration = Duration::from_secs(15);
static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

/// Run a full session: send each line, return the parsed response lines.
fn run_session(requests: &[Value]) -> Vec<Value> {
    run_session_with_barrier(requests, || true, &[])
}

/// Run requests on both sides of one externally observable session barrier.
fn run_session_with_barrier(
    before_barrier: &[Value],
    barrier: impl FnMut() -> bool,
    after_barrier: &[Value],
) -> Vec<Value> {
    run_session_with_state_barrier(before_barrier, barrier, after_barrier, None, None)
}

fn run_session_with_journal(requests: &[Value], journal: &std::path::Path) -> Vec<Value> {
    run_session_with_state_barrier(requests, || true, &[], Some(journal), None)
}

fn run_session_with_state(
    requests: &[Value],
    journey: &std::path::Path,
    journal: &std::path::Path,
) -> Vec<Value> {
    run_session_with_state_barrier(requests, || true, &[], Some(journal), Some(journey))
}

fn run_session_with_state_barrier(
    before_barrier: &[Value],
    mut barrier: impl FnMut() -> bool,
    after_barrier: &[Value],
    journal: Option<&std::path::Path>,
    journey: Option<&std::path::Path>,
) -> Vec<Value> {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let suffix = format!("{}-{session}", std::process::id());
    let ephemeral_journey = journey.is_none();
    let journey = journey.map_or_else(
        || std::env::temp_dir().join(format!("numinous_mcp_e2e_journey_{suffix}.txt")),
        std::path::Path::to_path_buf,
    );
    let scores = std::env::temp_dir().join(format!("numinous_mcp_e2e_scores_{suffix}.txt"));
    if ephemeral_journey {
        let _ = std::fs::remove_file(&journey);
    }
    let _ = std::fs::remove_file(&scores);

    let mut command = Command::new(env!("CARGO_BIN_EXE_numinous-mcp"));
    command
        .env("NUMINOUS_JOURNEY", &journey)
        .env("NUMINOUS_SCORES", &scores)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(journal) = journal {
        command.env("NUMINOUS_JOURNAL", journal);
    }
    let mut child = command.spawn().expect("spawn the MCP server");

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

    let barrier_deadline = Instant::now() + SESSION_BARRIER_TIMEOUT;
    while !barrier() {
        if Instant::now() >= barrier_deadline {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            let _ = output_reader.join();
            if ephemeral_journey {
                let _ = std::fs::remove_file(&journey);
            }
            let _ = std::fs::remove_file(&scores);
            panic!("MCP session barrier did not resolve within {SESSION_BARRIER_TIMEOUT:?}");
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
            if ephemeral_journey {
                let _ = std::fs::remove_file(&journey);
            }
            let _ = std::fs::remove_file(&scores);
            panic!("MCP server did not exit within 30 seconds");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = output_reader.join().expect("MCP output reader");

    assert!(status.success(), "server exited with an error");
    if ephemeral_journey {
        let _ = std::fs::remove_file(&journey);
    }
    let _ = std::fs::remove_file(&scores);

    String::from_utf8(stdout)
        .expect("utf8 output")
        .lines()
        .map(|line| serde_json::from_str(line).expect("every reply is valid JSON"))
        .collect()
}

fn reply_by_id(replies: &[Value], id: u64) -> &Value {
    replies
        .iter()
        .find(|response| response["id"] == id)
        .unwrap_or_else(|| panic!("no reply with id {id}"))
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
        json!({
            "jsonrpc":"2.0","id":7,"method":"tools/call",
            "params":{"name":"list_rooms","arguments":{"response_mode":"compact"}}
        }),
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
    let compact_catalog = text_of(by_id(7));
    for door in ["touch", "strange-loop", "wander"] {
        assert!(
            compact_catalog.contains(door),
            "compact catalog omitted {door}: {compact_catalog}"
        );
    }
    let structured_catalog = &by_id(7)["result"]["structuredContent"];
    assert_eq!(
        structured_catalog["threshold"]["doors"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    assert_eq!(
        structured_catalog["chain"]["steps"][0]["id"],
        "cellular-automata"
    );
    assert_eq!(
        structured_catalog["chain"]["steps"][5]["id"],
        "strange-loop"
    );
    assert_eq!(
        structured_catalog["count"].as_u64(),
        Some(numinous_core::ROOM_CATALOG.len() as u64)
    );
}

#[test]
fn show_is_sessionless_across_every_supported_protocol_revision() {
    let legacy = |version: &str| {
        run_session(&[
            json!({
                "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                    "protocolVersion":version,
                    "capabilities":{},
                    "clientInfo":{"name":"show-replay-test","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({
                "jsonrpc":"2.0","id":2,"method":"tools/call","params":{
                    "name":"watch_show",
                    "arguments":{"position":3,"seed":41,"motion":"reduced"}
                }
            }),
        ])
    };
    let mut projections = Vec::new();
    for version in ["2025-06-18", "2025-11-25"] {
        let first = legacy(version);
        let second = legacy(version);
        let first_show = &reply_by_id(&first, 2)["result"]["structuredContent"];
        let second_show = &reply_by_id(&second, 2)["result"]["structuredContent"];
        assert_eq!(first_show, second_show, "{version} replay drifted");
        projections.push(first_show.clone());
    }

    let modern_request = json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "_meta":{
                "io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientInfo":{"name":"show-replay-test","version":"1.0"},
                "io.modelcontextprotocol/clientCapabilities":{}
            },
            "name":"watch_show",
            "arguments":{"position":3,"seed":41,"motion":"reduced"}
        }
    });
    let first = run_session(std::slice::from_ref(&modern_request));
    let second = run_session(&[modern_request]);
    let first_show = &reply_by_id(&first, 1)["result"]["structuredContent"];
    let second_show = &reply_by_id(&second, 1)["result"]["structuredContent"];
    assert_eq!(first_show, second_show, "modern replay drifted");
    projections.push(first_show.clone());
    assert!(projections.windows(2).all(|pair| pair[0] == pair[1]));
    assert_eq!(first_show["segment"]["room"], "busy-beaver");
    assert_eq!(first_show["next"]["arguments"]["position"], 4);
    assert_eq!(first_show["effects"]["continuationStored"], false);
}

/// The text content of a tool-call response.
fn text_of(response: &Value) -> &str {
    response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

fn modern_meta(capabilities: Value) -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {
            "name": "numinous-stdio-test",
            "version": "1.0"
        },
        "io.modelcontextprotocol/clientCapabilities": capabilities
    })
}

#[test]
fn packaged_room_findings_stay_fixed_over_real_stdio() {
    let call = |id: u64, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{
                "_meta":modern_meta(json!({})),
                "name":"play_room",
                "arguments":arguments
            }
        })
    };
    let replies = run_session(&[
        call(
            1,
            json!({
                "id":"the-only-move",
                "t":0.40,
                "pokes":[
                    [0.32,0.32],[0.50,0.32],[0.68,0.32],
                    [0.32,0.50],[0.50,0.50],[0.68,0.50],
                    [0.32,0.68],[0.50,0.68],[0.68,0.68]
                ]
            }),
        ),
        call(
            2,
            json!({
                "id":"degree-720",
                "gesture":[
                    {"kind":"down","x":0.20,"y":0.45,"t":0.0},
                    {"kind":"up","x":0.85,"y":0.45,"t":0.0}
                ]
            }),
        ),
        call(3, json!({"id":"degree-720","pokes":[[0.85,0.45]]})),
    ]);

    let only_move = &reply_by_id(&replies, 1)["result"]["structuredContent"];
    assert_eq!(only_move["goalMet"], true);
    let status = only_move["status"].as_str().unwrap_or_default();
    assert!(status.contains("WON 1"), "{status}");
    assert!(status.contains("WASTE #2,#4,#6"), "{status}");

    let across = &reply_by_id(&replies, 2)["result"]["structuredContent"];
    assert_eq!(across["goalMet"], false);
    let status = across["status"].as_str().unwrap_or_default();
    assert!(status.contains("TURNS 2.00"), "{status}");
    assert!(status.contains("TWIST +2.00"), "{status}");
    assert!(!status.contains("OVER"), "{status}");

    let stone = &reply_by_id(&replies, 3)["result"]["structuredContent"];
    assert_eq!(stone["goalMet"], true);
    assert!(
        stone["status"]
            .as_str()
            .unwrap_or_default()
            .contains("OVER")
    );
}

#[test]
fn modern_play_room_returns_exact_temporal_evidence_over_real_stdio() {
    let play = |id: u64, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{
                "_meta":modern_meta(json!({})),
                "name":"play_room",
                "arguments":arguments
            }
        })
    };
    let replies = run_session(&[
        json!({
            "jsonrpc":"2.0", "id":1, "method":"tools/list",
            "params":{"_meta":modern_meta(json!({}))}
        }),
        play(
            2,
            json!({
                "id":"times-tables","width":40,"height":20,
                "from_t":0.2,"t":0.35,"variation":3
            }),
        ),
        play(
            3,
            json!({
                "id":"times-tables","width":40,"height":20,
                "t":0.2,"variation":3
            }),
        ),
        play(
            4,
            json!({
                "id":"times-tables","width":40,"height":20,
                "t":0.35,"variation":3
            }),
        ),
    ]);
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    let play_schema = by_id(1)["result"]["tools"]
        .as_array()
        .and_then(|tools| tools.iter().find(|tool| tool["name"] == "play_room"))
        .expect("play_room schema");
    assert_eq!(
        play_schema["inputSchema"]["dependentRequired"]["from_t"],
        json!(["t"])
    );
    let paired = &by_id(2)["result"]["structuredContent"];
    assert_eq!(paired["temporal"]["schema"], "numinous.temporal-evidence");
    assert_eq!(paired["temporal"]["fromT"], 0.2);
    assert_eq!(paired["temporal"]["toT"], 0.35);
    assert_eq!(
        paired["temporal"]["fromRender"],
        by_id(3)["result"]["structuredContent"]["render"]
    );
    assert_eq!(
        paired["render"],
        by_id(4)["result"]["structuredContent"]["render"]
    );
    assert!(text_of(by_id(2)).contains("Temporal:"));
}

#[test]
fn temporal_play_records_the_coarse_journey_visit_without_touching_the_journal() {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "numinous_mcp_temporal_persistence_{}_{}",
        std::process::id(),
        session
    ));
    std::fs::create_dir(&root).expect("fresh temporal persistence root");
    let journey = root.join("journey.txt");
    let journal = root.join("journal.txt");
    numinous_core::record_journal_file(
        &journal,
        numinous_core::JournalRecord {
            recorded_at_utc: 200,
            event_at_utc: 100,
            source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
            kind: "encounter",
            subject: "before-temporal-play",
            text: "Keep this exact player-chosen record.",
            affect: None,
        },
    )
    .expect("seed player journal");
    let journal_before = std::fs::read(&journal).expect("read seeded journal");

    let replies = run_session_with_state(
        &[
            json!({
                "jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
                    "protocolVersion":"2025-11-25", "capabilities":{},
                    "clientInfo":{"name":"temporal-persistence","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({
                "jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
                    "name":"play_room", "arguments":{
                        "id":"times-tables", "from_t":0.2, "t":0.35,
                        "width":40, "height":20
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":3, "method":"tools/call", "params":{
                    "name":"play_room", "arguments":{
                        "id":"times-tables", "receipt":true,
                        "width":40, "height":20
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":4, "method":"tools/call",
                "params":{"name":"read_journal","arguments":{}}
            }),
        ],
        &journey,
        &journal,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(2)["result"]["isError"], false);
    assert_eq!(by_id(3)["result"]["isError"], false);
    assert_eq!(
        by_id(3)["result"]["structuredContent"]["encounter"]["schema"],
        "numinous.encounter-receipt"
    );
    assert_eq!(by_id(4)["result"]["structuredContent"]["totalEntries"], 1);

    let persisted_journey = numinous_core::load_journey_file(&journey);
    assert!(persisted_journey.visited.contains("times-tables"));
    assert_eq!(
        std::fs::read(&journal).expect("read journal after temporal play"),
        journal_before,
        "temporal play and a receipt must not promote any action or result into the journal"
    );

    numinous_core::erase_journal_file(&journal).expect("erase test journal");
    numinous_core::remove_persisted_file(&journey).expect("erase test journey");
    std::fs::remove_dir(&root).expect("empty temporal persistence root");
}

#[test]
fn a_receipt_is_kept_only_when_the_player_promotes_a_live_match() {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "numinous_mcp_receipt_promotion_{}_{}",
        std::process::id(),
        session
    ));
    std::fs::create_dir(&root).expect("fresh receipt promotion root");
    let journey = root.join("journey.txt");
    let journal = root.join("journal.txt");

    let replies = run_session_with_state(
        &[
            json!({
                "jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
                    "protocolVersion":"2025-11-25", "capabilities":{},
                    "clientInfo":{"name":"receipt-promotion","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({
                "jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
                    "name":"play_room", "arguments":{
                        "id":"times-tables", "receipt":true,
                        "width":40, "height":20
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":3, "method":"tools/call",
                "params":{"name":"read_journal","arguments":{}}
            }),
        ],
        &journey,
        &journal,
    );
    assert_eq!(reply_by_id(&replies, 2)["result"]["isError"], false);
    let receipt = reply_by_id(&replies, 2)["result"]["structuredContent"]["encounter"].clone();
    let digest = receipt["resultDigest"].as_str().expect("result digest");
    assert_eq!(
        reply_by_id(&replies, 3)["result"]["structuredContent"]["totalEntries"],
        0
    );

    let forged = {
        let mut bad = receipt.clone();
        if let Some(digest) = bad.get_mut("resultDigest") {
            *digest = json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
        }
        bad
    };
    let creation = numinous_core::StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
        .expect("creation")
        .with_title("A Kept Wave")
        .expect("title")
        .with_author("First Hand")
        .expect("author")
        .with_era(numinous_core::Era::Vector);
    let kept = run_session_with_state(
        &[
            json!({
                "jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
                    "protocolVersion":"2025-11-25", "capabilities":{},
                    "clientInfo":{"name":"receipt-promotion","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({
                "jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
                    "name":"record_journal", "arguments":{
                        "kind":"encounter",
                        "text":"I want to keep this look.",
                        "source":"numinous-result",
                        "receipt": receipt.clone()
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":3, "method":"tools/call", "params":{
                    "name":"record_journal", "arguments":{
                        "kind":"encounter",
                        "subject":"times-tables",
                        "text":"A forged digest is not a keep.",
                        "receipt": forged
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":4, "method":"tools/call", "params":{
                    "name":"record_journal", "arguments":{
                        "kind":"encounter",
                        "subject":"times-tables",
                        "text":"Impersonation without a receipt.",
                        "source":"numinous-result"
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":5, "method":"tools/call",
                "params":{"name":"read_journal","arguments":{}}
            }),
            json!({
                "jsonrpc":"2.0", "id":6, "method":"tools/call", "params":{
                    "name":"export_journal", "arguments":{
                        "format":"portable-1",
                        "receipt":receipt,
                        "creation":creation.to_num_file()
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":7, "method":"tools/call", "params":{
                    "name":"export_journal", "arguments":{
                        "format":"portable-1", "receipt":forged
                    }
                }
            }),
            json!({
                "jsonrpc":"2.0", "id":8, "method":"tools/call",
                "params":{"name":"erase_journal","arguments":{"confirm":true}}
            }),
            json!({
                "jsonrpc":"2.0", "id":9, "method":"tools/call",
                "params":{"name":"read_journal","arguments":{}}
            }),
        ],
        &journey,
        &journal,
    );
    assert_eq!(reply_by_id(&kept, 2)["result"]["isError"], false);
    assert_eq!(
        reply_by_id(&kept, 2)["result"]["structuredContent"]["source"],
        "numinous-result"
    );
    assert_eq!(
        reply_by_id(&kept, 2)["result"]["structuredContent"]["subject"],
        format!("receipt:{digest}")
    );
    assert!(
        reply_by_id(&kept, 2)["result"]["structuredContent"]
            .get("receipt")
            .is_none()
    );
    assert_eq!(reply_by_id(&kept, 3)["result"]["isError"], true);
    assert_eq!(reply_by_id(&kept, 4)["result"]["isError"], true);
    let page = &reply_by_id(&kept, 5)["result"]["structuredContent"];
    assert_eq!(page["totalEntries"], 1);
    let expected_subject = format!("receipt:{digest}");
    assert_eq!(page["entries"][0]["subject"], expected_subject);
    assert_eq!(page["entries"][0]["source"], "numinous-result");
    assert_eq!(page["entries"][0]["text"], "I want to keep this look.");
    let capsule = &reply_by_id(&kept, 6)["result"]["structuredContent"];
    assert_eq!(capsule["schema"], "numinous.portable-evidence-capsule");
    assert_eq!(capsule["manifest"]["closedFileSet"], true);
    assert_eq!(
        capsule["manifest"]["selection"]["receiptResultDigest"],
        digest
    );
    assert_eq!(capsule["manifest"]["selection"]["creationIncluded"], true);
    let paths = capsule["files"]
        .as_array()
        .expect("capsule files")
        .iter()
        .map(|file| file["path"].as_str().expect("capsule path"))
        .collect::<Vec<_>>();
    assert!(paths.contains(&"native/encounter-receipt.json"));
    assert!(paths.contains(&"creations/studio.num"));
    assert_eq!(reply_by_id(&kept, 7)["result"]["isError"], true);
    let erased = &reply_by_id(&kept, 8)["result"]["structuredContent"];
    assert_eq!(erased["recoverableManagedResidue"], 0);
    assert_eq!(erased["managedSidecarFiles"], 0);
    assert_eq!(
        reply_by_id(&kept, 9)["result"]["structuredContent"]["totalEntries"],
        0
    );

    let inventory = numinous_core::inspect_journal_file(&journal).expect("erased inventory");
    assert!(!inventory.exists);
    assert_eq!(inventory.sidecar_files, 0);
    numinous_core::remove_persisted_file(&journey).expect("erase test journey");
    std::fs::remove_dir(&root).expect("empty receipt promotion root");
}

#[test]
fn receipt_digests_match_across_two_mcp_processes() {
    let play = json!({
        "jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
            "name":"play_room", "arguments":{
                "id":"times-tables", "receipt":true,
                "width":40, "height":20
            }
        }
    });
    let encounter_of = |root_label: &str| -> Value {
        let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "numinous_mcp_receipt_process_{root_label}_{}_{}",
            std::process::id(),
            session
        ));
        std::fs::create_dir(&root).expect("fresh receipt process root");
        let journey = root.join("journey.txt");
        let journal = root.join("journal.txt");
        let replies = run_session_with_state(std::slice::from_ref(&play), &journey, &journal);
        let encounter = replies[0]["result"]["structuredContent"]["encounter"].clone();
        numinous_core::erase_journal_file(&journal).ok();
        numinous_core::remove_persisted_file(&journey).ok();
        let _ = std::fs::remove_dir(&root);
        encounter
    };
    let first = encounter_of("a");
    let second = encounter_of("b");
    assert_eq!(first["schema"], "numinous.encounter-receipt");
    assert_eq!(first, second, "two processes must issue the same receipt");
    for field in ["fingerprint", "actionDigest", "resultDigest"] {
        let hex = first[field].as_str().unwrap_or_default();
        assert_eq!(hex.len(), 64, "{field} must be 32 bytes as lowercase hex");
        assert!(
            hex.bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            "{field} must be lowercase hex: {hex}"
        );
    }
    assert!(first.get("issuedAt").is_none());
}

#[test]
fn descriptions_and_withheld_wagers_stay_honest_over_real_stdio() {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "numinous_mcp_reveal_gate_{}_{}",
        std::process::id(),
        session
    ));
    std::fs::create_dir(&root).expect("fresh reveal gate root");
    let journey = root.join("journey.txt");
    let journal = root.join("journal.txt");

    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{"name":name,"arguments":arguments}
        })
    };
    let replies = run_session_with_state(
        &[
            call(1, "describe_room", json!({"id":"kepler-laws"})),
            call(2, "reveal_room", json!({"id":"kepler-laws"})),
            call(
                3,
                "play_room",
                json!({
                    "id":"times-tables", "t":0.375,
                    "place_wager":"mandelbrot", "width":40, "height":18
                }),
            ),
            call(4, "reveal_room", json!({"id":"times-tables"})),
            call(
                5,
                "play_room",
                json!({
                    "id":"times-tables", "t":0.375,
                    "place_wager":"mandelbrot", "aha_summon":true,
                    "width":40, "height":18
                }),
            ),
            call(6, "reveal_room", json!({"id":"times-tables"})),
        ],
        &journey,
        &journal,
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    let description = &by_id(1)["result"];
    assert_eq!(description["isError"], false);
    assert!(description["structuredContent"].get("reveal").is_none());
    assert!(!text_of(by_id(1)).contains("second law"));
    assert_eq!(by_id(2)["result"]["isError"], true);

    let withheld = &by_id(3)["result"]["structuredContent"];
    assert_eq!(withheld["goalMet"], true);
    assert_eq!(withheld["engineeredAha"]["beat"], "withheld");
    assert_eq!(withheld["engineeredAha"]["earn"], Value::Null);
    assert_eq!(withheld["engineeredAha"]["truth"], Value::Null);
    assert!(
        !withheld["status"]
            .as_str()
            .unwrap_or_default()
            .contains("NAILED")
    );
    assert_eq!(by_id(4)["result"]["isError"], true);

    let consolidated = &by_id(5)["result"]["structuredContent"];
    assert_eq!(consolidated["engineeredAha"]["beat"], "consolidated");
    assert_eq!(consolidated["engineeredAha"]["truth"], "mandelbrot");
    assert_eq!(by_id(6)["result"]["isError"], false);
    assert!(by_id(6)["result"]["structuredContent"]["reveal"].is_string());

    let persisted = numinous_core::load_journey_file(&journey);
    assert!(persisted.has_consolidated("times-tables"));
    numinous_core::remove_persisted_file(&journey).expect("erase test journey");
    std::fs::remove_dir(&root).expect("empty reveal gate root");
}

#[test]
fn modern_stateless_discovery_tools_and_prediction_work_over_real_stdio() {
    let replies = run_session(&[
        json!({
            "jsonrpc":"2.0", "id":1, "method":"server/discover",
            "params":{"_meta":modern_meta(json!({}))}
        }),
        json!({
            "jsonrpc":"2.0", "id":2, "method":"tools/list",
            "params":{"_meta":{
                "io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}
            }}
        }),
        json!({
            "jsonrpc":"2.0", "id":3, "method":"tools/call",
            "params":{
                "_meta":modern_meta(json!({"elicitation":{"form":{}}})),
                "name":"predict",
                "arguments":{"id":"slope-rider","seed":4}
            }
        }),
        json!({
            "jsonrpc":"2.0", "id":4, "method":"tools/call",
            "params":{
                "_meta":modern_meta(json!({"elicitation":{"form":{}}})),
                "name":"predict",
                "arguments":{"id":"slope-rider","seed":4},
                "inputResponses":{
                    "prediction":{
                        "action":"accept",
                        "content":{"guess":0.0}
                    }
                }
            }
        }),
        json!({
            "jsonrpc":"2.0", "id":5, "method":"ping",
            "params":{"_meta":modern_meta(json!({}))}
        }),
    ]);
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    assert_eq!(by_id(1)["result"]["resultType"], "complete");
    assert_eq!(
        by_id(1)["result"]["supportedVersions"],
        json!(["2026-07-28", "2025-11-25", "2025-06-18"])
    );
    assert_eq!(by_id(1)["result"]["cacheScope"], "public");
    assert_eq!(by_id(2)["result"]["resultType"], "complete");
    assert_eq!(
        by_id(2)["result"]["tools"].as_array().map(Vec::len),
        Some(40)
    );
    assert_eq!(by_id(3)["result"]["resultType"], "input_required");
    assert_eq!(
        by_id(3)["result"]["inputRequests"]["prediction"]["method"],
        "elicitation/create"
    );
    assert_eq!(by_id(4)["result"]["resultType"], "complete");
    assert_eq!(by_id(4)["result"]["structuredContent"]["game"], "predict");
    assert_eq!(by_id(5)["error"]["code"], -32601);
}

#[test]
fn returning_journal_survives_two_processes_then_leaves_zero_managed_residue() {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "numinous_mcp_returning_journal_{}_{}",
        std::process::id(),
        session
    ));
    std::fs::create_dir(&root).expect("fresh journal acceptance root");
    let journal = root.join("journal.txt");
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{"name":name,"arguments":arguments}
        })
    };
    let initialize = || {
        json!({
            "jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
                "protocolVersion":"2025-11-25",
                "capabilities":{},
                "clientInfo":{"name":"returning-journal-acceptance","version":"1.0"}
            }
        })
    };

    let first = run_session_with_journal(
        &[
            initialize(),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            call(2, "read_journal", json!({})),
            call(
                3,
                "record_journal",
                json!({
                    "kind":"encounter",
                    "subject":"times-tables",
                    "text":"The rendered multiplier closed nine loops.",
                    "event_time_utc":100,
                    "source":"self-authored"
                }),
            ),
            call(
                4,
                "record_journal",
                json!({
                    "kind":"connection",
                    "subject":"times-tables",
                    "text":"Nine means nine visible lobes.",
                    "event_time_utc":101,
                    "source":"self-authored"
                }),
            ),
            call(5, "read_journal", json!({})),
        ],
        &journal,
    );
    let first_by_id = |id: u64| -> &Value {
        first
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("missing first-process response {id}"))
    };
    assert_eq!(
        first_by_id(2)["result"]["structuredContent"]["totalEntries"],
        0
    );
    assert_eq!(first_by_id(3)["result"]["structuredContent"]["entryId"], 1);
    assert_eq!(first_by_id(4)["result"]["structuredContent"]["entryId"], 2);
    assert_eq!(
        first_by_id(5)["result"]["structuredContent"]["totalEntries"],
        2
    );
    assert!(text_of(first_by_id(5)).contains("Nine means nine visible lobes."));
    assert!(
        journal.exists(),
        "first process persists the opted-in journal"
    );

    let second = run_session_with_journal(
        &[
            initialize(),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            call(2, "read_journal", json!({})),
            call(
                3,
                "correct_journal",
                json!({
                    "entry_id":2,
                    "text":"The visible lobe count follows multiplier minus one.",
                    "source":"self-authored"
                }),
            ),
            call(
                4,
                "play_room",
                json!({"id":"times-tables","t":0.25,"width":40,"height":20}),
            ),
            call(
                5,
                "record_journal",
                json!({
                    "kind":"encounter",
                    "subject":"times-tables",
                    "text":"Used the corrected connection in a new flagship encounter.",
                    "event_time_utc":102,
                    "source":"self-authored"
                }),
            ),
            call(6, "export_journal", json!({"limit":100})),
            call(7, "export_journal", json!({"limit":2,"format":"okf-0.2"})),
            call(8, "erase_journal", json!({"confirm":true})),
            call(9, "read_journal", json!({})),
        ],
        &journal,
    );
    let second_by_id = |id: u64| -> &Value {
        second
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("missing second-process response {id}"))
    };
    assert_eq!(
        second_by_id(2)["result"]["structuredContent"]["totalEntries"],
        2
    );
    assert_eq!(
        second_by_id(3)["result"]["structuredContent"]["supersedes"],
        2
    );
    assert_eq!(second_by_id(5)["result"]["structuredContent"]["entryId"], 4);
    let exported = &second_by_id(6)["result"]["structuredContent"];
    assert_eq!(exported["schema"], "numinous.experience-journal");
    assert_eq!(exported["schemaVersion"], 3);
    assert_eq!(exported["entries"].as_array().map(Vec::len), Some(4));
    assert_eq!(exported["entries"][0]["source"], "self-authored");
    assert_eq!(exported["entries"][0]["eventAtUtc"], 100);
    assert!(
        exported["entries"][0]["recordedAtUtc"]
            .as_u64()
            .is_some_and(|recorded| recorded > 100)
    );
    assert_eq!(exported["entries"][1]["current"], false);
    assert_eq!(exported["entries"][2]["current"], true);
    assert_eq!(exported["entries"][2]["supersedes"], 2);
    assert_eq!(exported["entries"][1]["source"], "self-authored");
    assert_eq!(exported["entries"][2]["source"], "self-authored");
    assert_eq!(exported["createdFile"], false);
    assert_eq!(exported["containsHostPath"], false);
    assert!(
        !serde_json::to_string(exported)
            .expect("export JSON")
            .contains(&root.display().to_string()),
        "portable export must not expose its host path"
    );
    let okf = &second_by_id(7)["result"]["structuredContent"];
    assert_eq!(okf["schema"], "open-knowledge-format");
    assert_eq!(okf["schemaVersion"], "0.2");
    assert_eq!(okf["sourceSchema"], "numinous.experience-journal");
    assert_eq!(okf["page"]["returned"], 2);
    assert_eq!(okf["page"]["hasMore"], true);
    assert_eq!(okf["page"]["nextAfterEntryId"], 2);
    assert_eq!(okf["files"][0]["path"], "index.md");
    assert_eq!(okf["files"][1]["path"], "entries/00000000000000000001.md");
    assert_eq!(okf["createdFile"], false);
    assert_eq!(okf["containsHostPath"], false);
    assert!(
        okf["files"][0]["content"]
            .as_str()
            .is_some_and(|content| content.starts_with("---\nokf_version: \"0.2\"\n---"))
    );
    assert!(
        !serde_json::to_string(okf)
            .expect("OKF export JSON")
            .contains(&root.display().to_string()),
        "OKF export must not expose its host path"
    );
    let erased = &second_by_id(8)["result"]["structuredContent"];
    assert_eq!(erased["recoverableManagedResidue"], 0);
    assert_eq!(erased["managedSidecarFiles"], 0);
    assert_eq!(erased["projectControlledExportFiles"], 0);
    assert_eq!(
        second_by_id(9)["result"]["structuredContent"]["totalEntries"],
        0
    );

    let inventory = numinous_core::inspect_journal_file(&journal).expect("final inventory");
    assert!(!inventory.exists);
    assert_eq!(inventory.sidecar_files, 0);
    assert!(!inventory.sidecar_scan_capped);
    std::fs::remove_dir(&root).expect("empty acceptance root");
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
                json!({
                    "id":"times-tables","from_t":0.1,"t":0.2,
                    "width":40,"height":20,"variation":42,
                    "response_mode":"compact"
                }),
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
            call(
                7,
                "play_room",
                json!({
                    "id":"times-tables","from_t":0.2,"t":0.81,
                    "width":40,"height":20,"variation":42,
                    "pokes":[[0.375,0.5]],"place_wager":"mandelbrot",
                    "aha_summon":true
                }),
            ),
            call(8, "reveal_room", json!({"id":"times-tables"})),
            call(9, "journey", json!({})),
        ],
        || {
            viewer.retained_events().len() >= 6
                && viewer
                    .retained_events()
                    .last()
                    .is_some_and(|event| event.event.tool == PublicTool::RevealRoom)
        },
        &[call(10, "broadcast_session", json!({"action":"stop"}))],
    );
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|response| response["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };
    assert_eq!(by_id(1)["result"]["structuredContent"]["state"], "live");
    assert_eq!(
        by_id(3)["result"]["structuredContent"]["temporal"]["fromT"],
        0.1
    );
    assert!(text_of(by_id(3)).contains("Temporal from t=0.100 to t=0.200"));
    assert_eq!(
        by_id(6)["result"]["structuredContent"]["status"],
        "K 5.00  CLOSED  4 LOBES  FOUND"
    );
    assert_eq!(by_id(6)["result"]["structuredContent"]["goalMet"], true);
    assert_eq!(
        by_id(6)["result"]["structuredContent"]["engineeredAha"]["earn"],
        Value::Null
    );
    assert_eq!(by_id(7)["result"]["isError"], false);
    assert!(by_id(7)["result"]["structuredContent"]["temporal"].is_object());
    assert!(by_id(7)["result"]["structuredContent"]["engineeredAha"].is_object());
    assert!(text_of(by_id(8)).contains("Mandelbrot"));
    assert_eq!(by_id(10)["result"]["structuredContent"]["state"], "stopped");

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
            PublicTool::PlayRoom,
            PublicTool::RevealRoom,
        ]
    );
    assert_eq!(
        events
            .iter()
            .map(|event| event.public_sequence)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5]
    );
    assert!(events.iter().all(|event| event.skipped.is_none()));
    assert_eq!(events[0].event.arguments["from_t"], 0.1);
    assert_eq!(events[0].event.arguments["t"], 0.2);
    assert!(events[0].event.arguments.get("response_mode").is_none());
    assert_eq!(
        events[0].event.result["structuredContent"]["temporal"]["fromT"],
        0.1
    );
    viewer.scrub(-1);
    let aha_fallback = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
    assert!(
        aha_fallback.lit_count() > 100,
        "the exact public temporal Aha result remains visible as text"
    );
    assert!(
        viewer.audio_selection().is_none(),
        "an engineered Aha overlay must not claim incomplete native replay"
    );
    assert_eq!(events[4].event.arguments["aha_summon"], true);
    assert_eq!(
        events[4].event.result["structuredContent"]["temporal"]["fromT"],
        0.2
    );
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
    viewer.scrub(-3);
    let temporal_destination = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
    assert!(
        temporal_destination.lit_count() > 1_000,
        "the compact temporal action reconstructs its native destination"
    );
    let temporal_audio = viewer
        .audio_selection()
        .expect("temporal destination selects local sound");
    assert_eq!(temporal_audio.public_sequence(), 0);
    assert_eq!(
        temporal_audio.render(8_000),
        Some(room.sound_input(0.2, &[]).render(8_000)),
        "Watch Agent replays the destination t, not the origin from_t"
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
    let puzzle = numinous_core::GauntletPuzzle::new(seed);
    let secret = puzzle.bomb_code().to_vec();
    let expected_play = numinous_app::play::GauntletPlay {
        seed,
        stage: 0,
        munch: numinous_app::play::MunchPlay {
            board: puzzle.munch,
            seed,
            round: 0,
            cursor: 30,
            bites: std::collections::BTreeSet::new(),
            graded: None,
            bite_flash: None,
        },
        quiz: numinous_app::play::QuizPlay {
            round: puzzle.shape,
            flash: None,
        },
        scan: puzzle.sky,
        secret,
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
        call(
            5,
            "play_room",
            json!({"id":"lorenz","t":0.7,"width":50,"height":24}),
        ),
        call(6, "reveal_room", json!({"id":"lorenz"})),
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
        call(26, "workspace", json!({})),
        call(27, "watch_show", json!({"motion":"reduced"})),
    ];
    let replies = run_session(&requests);

    // 27 id-carrying requests, one notification with no reply.
    assert_eq!(replies.len(), 27, "one reply per id-carrying request");
    let by_id = |id: u64| -> &Value {
        replies
            .iter()
            .find(|r| r["id"] == id)
            .unwrap_or_else(|| panic!("no reply with id {id}"))
    };

    assert_eq!(by_id(1)["result"]["serverInfo"]["name"], "numinous");
    assert_eq!(
        by_id(2)["result"]["tools"].as_array().map(Vec::len),
        Some(40)
    );
    assert!(text_of(by_id(3)).contains("times-tables"));
    assert!(text_of(by_id(4)).contains("Fractals"));
    assert!(text_of(by_id(5)).contains('#'), "the butterfly has ink");
    assert!(text_of(by_id(6)).contains("Lorenz"));
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
        by_id(27)["result"]["structuredContent"]["segment"]["room"],
        "cellular-automata"
    );
    assert_eq!(
        by_id(25)["result"]["structuredContent"]["state"],
        "disabled"
    );
    assert_eq!(by_id(26)["result"]["structuredContent"]["empty"], true);
    assert_eq!(by_id(26)["result"]["structuredContent"]["scope"], "process");
}

#[test]
fn workspace_survives_calls_in_one_process_and_dies_with_it() {
    let call = |id: u64, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{"name":"workspace","arguments":arguments}
        })
    };
    let initialize = json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-06-18",
            "capabilities":{},
            "clientInfo":{"name":"workspace-visit","version":"1.0"}
        }
    });
    let first = run_session(&[
        initialize.clone(),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(2, json!({})),
        call(
            3,
            json!({
                "op":"edit",
                "place":{"room":"lorenz","t":0.5},
                "intention":"watch the storm"
            }),
        ),
        call(4, json!({"op":"inspect"})),
        json!({
            "jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{"name":"play_room","arguments":{"id":"mandelbrot","t":0.1,"width":40,"height":20}}
        }),
        call(6, json!({"op":"inspect"})),
    ]);
    assert_eq!(
        reply_by_id(&first, 2)["result"]["structuredContent"]["empty"],
        true
    );
    assert_eq!(
        reply_by_id(&first, 4)["result"]["structuredContent"]["place"]["room"],
        "lorenz"
    );
    assert_eq!(
        reply_by_id(&first, 6)["result"]["structuredContent"]["place"]["room"],
        "lorenz"
    );
    assert_eq!(
        reply_by_id(&first, 6)["result"]["structuredContent"]["intention"],
        "watch the storm"
    );

    let second = run_session(&[
        initialize,
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        call(2, json!({})),
    ]);
    assert_eq!(
        reply_by_id(&second, 2)["result"]["structuredContent"]["empty"],
        true,
        "a new process must not inherit the previous visit workspace"
    );
}

#[test]
fn remembered_room_retrieval_is_bounded_explained_and_honestly_empty() {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "numinous_mcp_remembered_room_{}_{}",
        std::process::id(),
        session
    ));
    std::fs::create_dir(&root).expect("fresh remembered room root");
    let journal = root.join("journal.txt");
    let original = numinous_core::record_journal_file(
        &journal,
        numinous_core::JournalRecord {
            recorded_at_utc: 10,
            event_at_utc: 5,
            source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
            kind: "encounter",
            subject: "kepler-areas",
            text: "The swept areas looked equal.",
            affect: None,
        },
    )
    .expect("record remembered room");
    let correction = numinous_core::correct_journal_file(
        &journal,
        20,
        None,
        numinous_core::JOURNAL_SOURCE_PLAYER_PROVIDED,
        original.entry_id,
        "The swept areas stayed equal while the speed changed.",
        None,
    )
    .expect("correct remembered room");
    numinous_core::record_journal_file(
        &journal,
        numinous_core::JournalRecord {
            recorded_at_utc: 30,
            event_at_utc: 30,
            source: numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT,
            kind: "encounter",
            subject: "receipt:opaque-proof",
            text: "Mentions kepler-laws, but its subject does not.",
            affect: None,
        },
    )
    .expect("record opaque receipt subject");

    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{"name":name,"arguments":arguments}
        })
    };
    let replies = run_session_with_journal(
        &[
            json!({
                "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                    "protocolVersion":"2025-06-18",
                    "capabilities":{},
                    "clientInfo":{"name":"remembered-room","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            call(2, "describe_room", json!({"id":"kepler-laws"})),
            call(3, "workspace", json!({"op":"inspect"})),
            call(
                4,
                "workspace",
                json!({"op":"retrieve","room":"kepler-laws","limit":1}),
            ),
            call(5, "workspace", json!({"op":"inspect"})),
            call(6, "workspace", json!({"op":"retrieve","room":"mandelbrot"})),
            call(
                7,
                "workspace",
                json!({"op":"edit","retrieved":[{"entry_id":original.entry_id}]}),
            ),
            call(8, "erase_journal", json!({"confirm":true})),
            call(9, "describe_room", json!({"id":"kepler-laws"})),
            call(10, "workspace", json!({"op":"inspect"})),
        ],
        &journal,
    );

    let doorway = &reply_by_id(&replies, 2)["result"];
    assert_eq!(
        doorway["structuredContent"]["journalCue"]["status"],
        "remembered"
    );
    assert_eq!(
        doorway["structuredContent"]["journalCue"]["next"]["arguments"],
        json!({"op":"retrieve","room":"kepler-laws"})
    );
    let doorway_wire = serde_json::to_string(doorway).expect("serialize doorway");
    for private in [
        "The swept areas stayed equal while the speed changed.",
        "player-provided",
        "opaque-proof",
    ] {
        assert!(!doorway_wire.contains(private));
    }
    assert_eq!(
        reply_by_id(&replies, 3)["result"]["structuredContent"]["empty"],
        true,
        "surfacing a cue must not mutate the visit workspace"
    );

    let found = &reply_by_id(&replies, 4)["result"]["structuredContent"];
    assert_eq!(found["schemaVersion"], 2);
    assert_eq!(found["retrieval"]["room"], "kepler-laws");
    assert_eq!(found["retrieval"]["returned"], 1);
    assert_eq!(found["retrieval"]["abstained"], false);
    assert_eq!(found["retrieved"][0]["entry_id"], correction.entry_id);
    assert_eq!(found["retrieved"][0]["status"], "current");
    assert_eq!(
        found["retrieved"][0]["entry"]["source"],
        numinous_core::JOURNAL_SOURCE_PLAYER_PROVIDED
    );
    assert!(
        found["retrieved"][0]["source_explanation"]
            .as_str()
            .is_some_and(|text| text.contains("player supplied"))
    );
    assert_eq!(
        reply_by_id(&replies, 5)["result"]["structuredContent"]["retrieved"][0]["entry_id"],
        correction.entry_id,
        "the resolved handle remains available in this process"
    );

    let absent = &reply_by_id(&replies, 6)["result"]["structuredContent"];
    assert_eq!(absent["retrieval"]["abstained"], true);
    assert_eq!(absent["retrieved"], json!([]));
    assert!(
        absent["retrieval"]["abstentionReason"]
            .as_str()
            .is_some_and(|text| text.contains("receipt digests were not searched"))
    );

    let superseded = &reply_by_id(&replies, 7)["result"]["structuredContent"]["retrieved"][0];
    assert_eq!(superseded["status"], "superseded");
    assert_eq!(superseded["superseded_by"], correction.entry_id);
    assert!(
        reply_by_id(&replies, 9)["result"]["structuredContent"]
            .get("journalCue")
            .is_none()
    );
    let erased = &reply_by_id(&replies, 10)["result"]["structuredContent"]["retrieved"][0];
    assert_eq!(erased["status"], "missing");
    assert!(erased["entry"].is_null());

    let inventory = numinous_core::inspect_journal_file(&journal).expect("inspect erased journal");
    assert!(!inventory.exists);
    std::fs::remove_dir(root).expect("remove remembered room root");
}

#[test]
fn creation_capsules_cross_real_stdio_with_lineage_and_v2_journal_migration() {
    let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "numinous_mcp_creation_lineage_{}_{}",
        std::process::id(),
        session
    ));
    std::fs::create_dir(&root).expect("fresh creation root");
    let journey = root.join("journey.txt");
    let journal = root.join("journal.txt");
    std::fs::write(&journal, "numinous-journal-v2\n").expect("seed v2 journal");

    let parent = numinous_core::StudioCreation::new("sin(a*x)", -2.0, 3.0, 0.75)
        .expect("parent")
        .with_title("First Wave")
        .expect("title")
        .with_author("First Hand")
        .expect("author")
        .with_era(numinous_core::Era::Vector);
    let parent_num = parent.to_num_file();
    let parent_link = parent.to_link();
    let call = |id: u64, name: &str, arguments: Value| {
        json!({
            "jsonrpc":"2.0", "id":id, "method":"tools/call",
            "params":{"name":name,"arguments":arguments}
        })
    };
    let replies = run_session_with_state(
        &[
            json!({
                "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                    "protocolVersion":"2025-06-18",
                    "capabilities":{},
                    "clientInfo":{"name":"creation-lineage","version":"1.0"}
                }
            }),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            call(
                3,
                "save_creation",
                json!({
                    "expr":"sin(a*x)","xmin":-2.0,"xmax":3.0,"a":0.75,
                    "title":"First Wave","author":"First Hand","era":"vector"
                }),
            ),
            call(4, "open_creation", json!({"capsule":parent_link})),
            call(
                5,
                "fork_creation",
                json!({
                    "parent":parent_num,"expr":"sin(a*x)+0.1",
                    "title":"Second Wave","author":"Next Hand"
                }),
            ),
            call(
                6,
                "record_journal",
                json!({
                    "kind":"creation","subject":parent_link,
                    "text":"Named and signed over the portable creation surface."
                }),
            ),
            call(7, "read_journal", json!({"limit":10})),
        ],
        &journey,
        &journal,
    );

    assert_eq!(
        reply_by_id(&replies, 2)["result"]["tools"]
            .as_array()
            .map(Vec::len),
        Some(40)
    );
    let saved = &reply_by_id(&replies, 3)["result"]["structuredContent"];
    assert_eq!(saved["numFile"], parent.to_num_file());
    assert_eq!(saved["link"], parent.to_link());
    assert_eq!(saved["journalSubject"], parent.to_link());
    assert_eq!(saved["createdFile"], false);
    assert_eq!(saved["containsHostPath"], false);
    assert_eq!(
        reply_by_id(&replies, 4)["result"]["structuredContent"]["link"],
        parent.to_link()
    );

    let forked = &reply_by_id(&replies, 5)["result"]["structuredContent"];
    assert_eq!(forked["parentLink"], parent.to_link());
    let child = numinous_core::StudioCreation::from_num_file(
        forked["numFile"].as_str().expect("child .num"),
    )
    .expect("reopen child");
    assert_eq!(child.title(), Some("Second Wave"));
    assert_eq!(child.author(), Some("Next Hand"));
    assert_eq!(child.era(), Some(numinous_core::Era::Vector));
    assert_eq!(child.descends(), Some(parent.to_link().as_str()));

    let read = &reply_by_id(&replies, 7)["result"]["structuredContent"];
    assert_eq!(read["entries"][0]["kind"], "creation");
    assert_eq!(read["entries"][0]["subject"], parent.to_link());
    assert!(
        std::fs::read_to_string(&journal)
            .expect("journal")
            .starts_with("numinous-journal-v3\n"),
        "the first mutation migrates the v2 journal"
    );
    assert_eq!(numinous_core::load_journey_file(&journey).plays, 3);
    assert!(
        std::fs::read_dir(&root)
            .expect("root listing")
            .flatten()
            .all(|entry| entry
                .path()
                .extension()
                .is_none_or(|extension| extension != "num")),
        "portable creation tools must not write a hidden .num file"
    );

    numinous_core::remove_persisted_file(&journal).expect("remove journal");
    numinous_core::remove_persisted_file(&journey).expect("remove journey");
    std::fs::remove_dir(root).expect("remove creation root");
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
