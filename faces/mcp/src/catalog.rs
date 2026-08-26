//! MCP discovery documents and the immutable public tool schema.

use super::{
    JSON_SCHEMA_2020_12, MAX_AUTHOR_CHARS, MAX_TOOL_HEIGHT, MAX_TOOL_ID_CHARS, MAX_TOOL_WIDTH,
    SUPPORTED_PROTOCOL_VERSIONS,
    journal::{DEFAULT_PAGE_ENTRIES, MAX_PAGE_ENTRIES},
};
use numinous_broadcast::{PLAY_ROOM_MAX_DWELL_CELLS, PLAY_ROOM_MAX_TEMPORAL_CELLS};
use numinous_core::{MAX_DWELL_LOOKS, MIN_DWELL_LOOKS};
use serde_json::{Value, json};

/// Default legacy MCP revision when an initialization request has no preference.
const LEGACY_PROTOCOL_VERSION: &str = "2025-06-18";

/// Revisions valid inside the legacy initialization handshake.
const LEGACY_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18"];

/// Pick a mutually supported protocol revision from the client initialize params.
fn negotiate_protocol_version(params: Option<&Value>) -> &'static str {
    let requested = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str);
    if let Some(version) = requested
        && let Some(supported) = LEGACY_PROTOCOL_VERSIONS
            .iter()
            .copied()
            .find(|candidate| *candidate == version)
    {
        return supported;
    }
    LEGACY_PROTOCOL_VERSION
}

fn server_instructions() -> &'static str {
    "Explore the catalog with list_rooms using response_mode compact for a short first look, then play_room to render ASCII and see what the math does. describe_room is a safe doorway and never returns the explanation. Add from_t with an explicit destination t when you want two exact observations and their temporal delta in one stateless call; a static room can honestly report zero visible change. To stay in a room rather than move through it, pass dwell with several phases: the reply reports what refused to move across all of them, including how much stayed dark inside the region that did move. Pass receipt true on play_room for a replay proof in structuredContent.encounter; a receipt is not a memory, and asking does not keep the play. To keep one, pass that object as receipt on record_journal; the server replays it and stores only a live match. workspace holds a resettable visit state in this process only: inspect, edit, retrieve, defer, or clear place, intention, pending_prediction, unfinished work, recent notes, and journal handles. Retrieve names one room explicitly, selects at most four current exact-subject matches from the player-owned journal, explains every source, and abstains when no evidence exists. Play does not write the workspace. It is not a memory, and exiting or clearing drops it. save_creation, open_creation, and fork_creation return portable .num text and native links without reading or writing a host file. A creation result's journalSubject can be passed explicitly to record_journal with kind creation, so a signed creative arc remains player-owned. On Times Tables pass place_wager (mandelbrot, nephroid, or circle) then aha_summon true for the engineered aha; on Buffon's Needle pass number_wager (1.5..4.5) then aha_summon true; on the Galton Board drop waves with pokes, pass bin_wager (0..16, where the pile those pokes build will peak; it is the newest coin's run, and every reply names the coin it read) then aha_summon true. On Double Pendulum release the arms with a gesture, pass ending_wager (together, drifted, or lost), then aha_summon true. On Kepler Areas tune an ellipse with a poke or completed gesture, pass speed_wager (faster, slower, or same), then aha_summon true. On Parrondo's Trap try a policy with a poke or completed gesture, pass policy_wager (a, b, or abb), then aha_summon true. On Nontransitive Dice choose first with die_choice (a, b, or c), pass counter_wager (a, b, or c), then aha_summon true. Read structuredContent.engineeredAha for the beat, visible wager, and post-summon grade. reveal_room opens only after a normal room has been played, or after an engineered Aha has consolidated. Pass audio true to listen_room or sing_expression and a real WAV arrives in an audio content block beside the notation. That is a sound sent, not a sound heard: whether your client surfaces it is its answer to give, and if it cannot, the notation is the whole of what you get. Steer simulations with list_sims and run_sim, and play Guess the Shape with the quiz tool. Modern clients that advertise form elicitation can complete predict as one multi-round-trip call. If a human offers a local App pairing code, broadcast_session lets you consent to, inspect, pause, resume, or stop that read-only public view. Further reading lives on reveal_room as citation."
}

fn server_capabilities() -> Value {
    json!({ "tools": {} })
}

pub(super) fn server_info() -> Value {
    json!({ "name": "numinous", "version": env!("CARGO_PKG_VERSION") })
}

pub(super) fn discover_result() -> Value {
    json!({
        "supportedVersions": SUPPORTED_PROTOCOL_VERSIONS,
        "capabilities": server_capabilities(),
        "instructions": server_instructions(),
    })
}

/// The `initialize` result: who we are and what we support.
pub(super) fn initialize_result(params: Option<&Value>) -> Value {
    let protocol_version = negotiate_protocol_version(params);
    json!({
        "protocolVersion": protocol_version,
        "capabilities": server_capabilities(),
        "serverInfo": server_info(),
        "instructions": server_instructions(),
    })
}

/// The `tools/list` result. Descriptions are written for a mind to read and
/// decide; inputs are flat and simple by design (see `docs/INTERFACES.md`).
pub(super) fn tools_list_result() -> Value {
    tools_catalog().clone()
}

/// The catalog is immutable and used for every tool-call boundary check.
/// Construct it once rather than rebuilding all descriptions and schemas for
/// each request.
pub(super) fn tools_catalog() -> &'static Value {
    static CATALOG: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| add_schema_dialects(add_response_mode(build_tools_catalog())))
}

fn add_schema_dialects(mut catalog: Value) -> Value {
    if let Some(tools) = catalog.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools {
            if let Some(schema) = tool.get_mut("inputSchema").and_then(Value::as_object_mut) {
                schema.insert("$schema".to_string(), json!(JSON_SCHEMA_2020_12));
            }
        }
    }
    catalog
}

/// Shared schema fragment for catalog room ids (and the same bound for similar
/// short string keys). Documents and enforces [`MAX_TOOL_ID_CHARS`].
fn room_id_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "maxLength": MAX_TOOL_ID_CHARS,
        "description": description,
    })
}

/// Add one presentation-only option to every tool schema. The option belongs
/// at the face boundary because it changes neither domain arguments nor the
/// complete typed result.
fn add_response_mode(mut catalog: Value) -> Value {
    let Some(tools) = catalog.get_mut("tools").and_then(Value::as_array_mut) else {
        return catalog;
    };
    for tool in tools {
        if tool.get("name").and_then(Value::as_str) == Some("broadcast_session") {
            continue;
        }
        let Some(properties) = tool
            .get_mut("inputSchema")
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(Value::as_object_mut)
        else {
            continue;
        };
        properties.insert(
            "response_mode".to_string(),
            json!({
                "type": "string",
                "enum": ["full", "compact"],
                "default": "full",
                "description": "Presentation only. 'full' (default) preserves the complete text and structured result. For eligible results, 'compact' keeps structuredContent identical but replaces duplicated prose with a shorter actionable summary; results whose text carries unique information, text-only results, and errors remain complete. Use compact only when your client reads structuredContent."
            }),
        );
    }
    catalog
}

fn room_pokes_schema() -> Value {
    json!({
        "type": "array",
        "description": "Normalized hand points as [x,y] pairs in [0,1]. Newest point last. Not combinable with 'gesture'.",
        "maxItems": numinous_core::MAX_ROOM_POKES,
        "items": {
            "type": "array",
            "items": { "type": "number", "minimum": 0, "maximum": 1 },
            "minItems": 2,
            "maxItems": 2
        }
    })
}

fn room_gesture_schema() -> Value {
    let positioned_event = |kind: &str| {
        json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": [kind] },
                "x": { "type": "number", "minimum": 0, "maximum": 1 },
                "y": { "type": "number", "minimum": 0, "maximum": 1 },
                "t": { "type": "number", "minimum": 0, "maximum": 1 }
            },
            "required": ["kind", "x", "y", "t"],
            "additionalProperties": false
        })
    };
    json!({
        "type": "array",
        "description": "A replayable pointer trail, for example [{\"kind\":\"down\",\"x\":0.5,\"y\":0.5,\"t\":0.25},{\"kind\":\"up\",\"x\":0.5,\"y\":0.5,\"t\":0.25}]. Events run oldest to newest; phase timestamps wrap from 1 back to 0 like the App clock. In held rooms (double-pendulum) a down pins the bob, an up releases it with the velocity of the approach, and a cancel lets go gently. In Life, a down earlier than the final t plants a glider early enough to show its later evolution; the newest 24 down events become launches. Everywhere else the trail's down and move points paint like pokes. Not combinable with 'pokes'.",
        "maxItems": numinous_core::MAX_ROOM_INPUTS,
        "items": {
            "oneOf": [
                positioned_event("down"),
                positioned_event("move"),
                positioned_event("up"),
                {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["cancel"] }
                    },
                    "required": ["kind"],
                    "additionalProperties": false
                }
            ]
        }
    })
}

fn build_tools_catalog() -> Value {
    json!({
        "tools": [
            {
                "name": "list_rooms",
                "description": "List the catalog of mathematical rooms you can explore and play. structuredContent.starters names four rooms worth opening first; structuredContent.rooms carries every room in every mode. For a short first look, pass response_mode compact.",
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
            },
            {
                "name": "describe_room",
                "description": "Safely describe one room: its title, wing, action, goal, and nonspoiling doorway. This never returns the revelation. Use list_rooms first to find valid ids.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": room_id_schema("Room id, for example times-tables.")
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "reveal_room",
                "description": "Open a room's earned revelation and further reading. A normal room must be played first. An engineered Aha room must complete its wager and aha_summon loop first.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": room_id_schema("Room id, for example times-tables.")
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "play_room",
                "description": "Play a room: render it and get back an ASCII picture of the result, so you can see what the math does. Add from_t with an explicit destination t for two exact observations and a typed temporal delta; no elapsed duration or path between them is inferred. When you supply pokes or a gesture, the top-level delta separately measures exactly how the math answered your hand at t. This call is stateless: replay the same inputs for the same result. Pass receipt true for a replay proof in structuredContent.encounter; a receipt is not a memory, and asking does not keep the play. Times Tables, Buffon's Needle, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, and Nontransitive Dice accept a room-owned wager plus aha_summon to walk an engineered aha without App session state; structuredContent.engineeredAha reports the beat.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": room_id_schema("Room id, for example times-tables."),
                        "t": { "type": "number", "minimum": 0, "exclusiveMaximum": 1, "description": "Finite destination phase in [0,1). For Times Tables this sweeps the multiplier. Required when from_t is present." },
                        "from_t": { "type": "number", "minimum": 0, "exclusiveMaximum": 1, "description": format!("Optional exact origin phase for two-observation temporal evidence. Requires explicit t. Both observations use the same room, variation, and dimensions. Compact poke coordinates are reapplied independently at each phase; use a phase-stamped gesture when the room should interpret one causal event history at both phases. Static phase views can honestly produce a zero-cell delta, including a poke-tuned Kepler ellipse. Width times height must be at most {PLAY_ROOM_MAX_TEMPORAL_CELLS} cells. The supplied order is comparison direction only; it does not assert elapsed time or an interpolated path.") },
                        "dwell": {
                            "type": "array",
                            "minItems": MIN_DWELL_LOOKS,
                            "maxItems": MAX_DWELL_LOOKS,
                            "items": { "type": "number", "minimum": 0, "exclusiveMaximum": 1 },
                            "description": format!("Optional: stay in this room. Give {MIN_DWELL_LOOKS} to {MAX_DWELL_LOOKS} phases to look at, and structuredContent.dwell reports what refused to move across all of them: cells that never changed, cells that were never lit, and how many of those sit inside the region that did move. A repeated phase is allowed and honestly reports that nothing moved. Every look uses the same room, hand, variation, and dimensions, so this measures the room rather than your input. All {MAX_DWELL_LOOKS} looks fit the default canvas, so you need not shrink the room to stay the longest way; if you ask for a bigger one, looks times width times height must be at most {PLAY_ROOM_MAX_DWELL_CELLS} cells. No elapsed time, order, or path between looks is asserted."),
                        },
                        "width": { "type": "integer", "minimum": 1, "maximum": MAX_TOOL_WIDTH, "description": "ASCII canvas width in columns, from 1 through 512." },
                        "height": { "type": "integer", "minimum": 1, "maximum": MAX_TOOL_HEIGHT, "description": "ASCII canvas height in rows, from 1 through 256." },
                        "variation": { "type": "integer", "minimum": 0, "description": "Per-visit variation seed (default 0) for replayable novelty in supporting rooms." },
                        "pokes": room_pokes_schema(),
                        "gesture": room_gesture_schema(),
                        "place_wager": {
                            "type": "string",
                            "enum": ["mandelbrot", "nephroid", "circle"],
                            "description": "Times Tables engineered aha only: commit where the K=2 heart also lives. Generation before reveal."
                        },
                        "number_wager": {
                            "type": "number",
                            "minimum": 1.5,
                            "maximum": 4.5,
                            "description": "Buffon's Needle engineered aha only: commit a finite number on 1.5..4.5 for the crossing ratio."
                        },
                        "bin_wager": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": numinous_core::rooms::galton_board::BOARD_ROWS,
                            "description": format!(
                                "Galton Board engineered aha only: commit the bin (0..{} right turns) where the pile these pokes build will peak, after at least one wave. That pile is the newest coin's run, so pokes that wander to another coin ask about a different pile and get a different answer; every reply names the coin it read. Graded against that binomial's true mode, never against one ball's luck.",
                                numinous_core::rooms::galton_board::BOARD_ROWS
                            )
                        },
                        "ending_wager": {
                            "type": "string",
                            "enum": ["together", "drifted", "lost"],
                            "description": "Double Pendulum engineered aha only: after at least one completed release, call where its deterministic shadow twin ends."
                        },
                        "speed_wager": {
                            "type": "string",
                            "enum": ["faster", "slower", "same"],
                            "description": "Kepler Areas engineered aha only: after tuning an ellipse, call how orbital speed changes near the sun."
                        },
                        "policy_wager": {
                            "type": "string",
                            "enum": ["a", "b", "abb"],
                            "description": "Parrondo's Trap engineered aha only: after trying a policy, call which policy wins in exact expectation after 120 turns."
                        },
                        "die_choice": {
                            "type": "string",
                            "enum": ["a", "b", "c"],
                            "description": "Nontransitive Dice engineered aha only: choose your die first. Use this typed action instead of pokes or a gesture."
                        },
                        "counter_wager": {
                            "type": "string",
                            "enum": ["a", "b", "c"],
                            "description": "Nontransitive Dice engineered aha only: after choosing A, B, or C, call which die beats it across all 36 face pairs."
                        },
                        "aha_summon": {
                            "type": "boolean",
                            "description": "After a generation act on Times Tables, Buffon, the Galton Board, Double Pendulum, Kepler Areas, Parrondo's Trap, or Nontransitive Dice, advance the engineered aha through morph to consolidated and unlock punchline reveal text. Stateless one-shot."
                        },
                        "receipt": {
                            "type": "boolean",
                            "description": "Optional. Pass true to receive a Numinous Encounter Receipt in structuredContent.encounter: a versioned replay proof of this exact play. Two identical plays produce the same artifact. Asking does not write the journal or keep the play. Omit the flag, or pass false, to leave the structured result unchanged."
                        }
                    },
                    "required": ["id"],
                    "dependentRequired": { "from_t": ["t"] },
                    "additionalProperties": false
                }
            },
            {
                "name": "challenge",
                "description": "A posed, seeded goal for a room, in two kinds. Touch (default): change enough cells inside a target box; call without pokes to get the goal, then again with pokes to be graded. Parameter: sweep the room's phase until its own status readout lands on a target number; call without t to get the goal, then again with t to be graded. Grades are metrics, not pass/fail: a 0-100 score you can climb, plus the numbers behind it.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": room_id_schema("Room id; touch goals need a room with a touch verb, parameter goals a room with a moving numeric readout (see describe_room)."),
                        "kind": { "type": "string", "enum": ["touch", "parameter"], "description": "Goal kind (default touch). Parameter goals target the room's own status readout instead of a spatial response." },
                        "seed": { "type": "integer", "description": "Challenge seed (default 1). The same seed poses the same goal; pass any number you like, including today's date, to share a goal." },
                        "t": { "type": "number", "minimum": 0, "exclusiveMaximum": 1, "description": "Phase in [0,1) for the attempt (default 0 for touch). For parameter goals this IS the attempt: omit it to pose, pass it to be graded at that phase." },
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
                "description": "Predict-then-reveal: commit a room readout at a posed moment and optionally its linear rate, then see the truth and how your model missed. Call without `guess` to pose. Call again with `guess` for the established point gap and learning-progress band (NAILED = compressed, CLOSE = fertile, WILD = noise). Add `rate` in readout units per phase to reveal the actual local rate plus five signed residuals, the shape of the model's error rather than another score. This is a self-owned mirror, never a leaderboard or win. Guess before you look.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": room_id_schema("Room id; the room must carry a moving numeric readout (see describe_room)."),
                        "seed": { "type": "integer", "description": "Prediction seed (default 1). The same seed poses the same hidden moment; pass any number, including today's date, to share a prediction." },
                        "variation": { "type": "integer", "description": "Which room variation to predict (default 0), matching play_room's variation so the graded truth is the readout you played. Pass the same seed and variation to both the pose and the guess call." },
                        "guess": { "type": "number", "description": "Your predicted value for the readout at the posed moment. Omit to pose." },
                        "rate": { "type": "number", "description": "Optional slope for your linear model, in readout units per full phase unit. When present with guess, grading reveals the actual local rate and signed residual shape across five nearby phases." }
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
                        "leave": {
                            "type": "string",
                            "maxLength": numinous_core::cairn::MAX_BEQUEST_CHARS,
                            "description": "A short true thing to leave for whoever comes after (opens at level 42). At most 140 characters."
                        },
                        "author": {
                            "type": "string",
                            "maxLength": MAX_AUTHOR_CHARS,
                            "description": "Who to credit for a bequest (default \"a visitor\"). Used only with `leave`."
                        }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "read_journal",
                "description": "Inspect a bounded page of your continuous MCP experience journal as readable text. Entries expose stable identifiers, event and record times, declared source provenance, correction links, and current status. The journal is completely opt-in and player-owned.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "after_entry_id": { "type": "integer", "minimum": 0, "description": "Return entries after this stable identifier (default 0)." },
                        "limit": { "type": "integer", "minimum": 1, "maximum": MAX_PAGE_ENTRIES, "default": DEFAULT_PAGE_ENTRIES, "description": "Maximum entries to return, from 1 through 100." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "record_journal",
                "description": "Append an original entry to your MCP experience journal. The server assigns a stable identifier and record time. Declare the account source and optional event time; affect is accepted only as explicit self-report. Pass the structuredContent.encounter object as receipt to keep a replay proof: the server replays it, and only a live match is stored as source numinous-result under subject receipt:<resultDigest>. Do not record private host data.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "minLength": 1, "maxLength": numinous_core::MAX_JOURNAL_KIND_CHARS, "description": "The entry kind, for example encounter, creation, connection, or thought. The reserved correction kind is available only through correct_journal." },
                        "subject": { "type": "string", "minLength": 1, "maxLength": numinous_core::MAX_JOURNAL_SUBJECT_CHARS, "description": "The specific room id or subject. Use a listed room id when this entry should be eligible for exact remembered-room retrieval. When receipt is present this is overwritten to receipt:<resultDigest>, which retrieval deliberately does not search." },
                        "text": { "type": "string", "minLength": 1, "maxLength": numinous_core::MAX_JOURNAL_TEXT_CHARS, "description": "The main content to remember. For a promoted receipt this is your interpretation, not the receipt body." },
                        "affect": { "type": "string", "minLength": 1, "maxLength": numinous_core::MAX_JOURNAL_AFFECT_CHARS, "description": "Optional explicitly self-reported affect or state. Never infer this value." },
                        "event_time_utc": { "type": "integer", "minimum": 0, "description": "Optional Unix time in seconds for the described event. Defaults to the server-owned record time." },
                        "source": { "type": "string", "enum": ["self-authored", "player-provided", "numinous-result"], "default": "self-authored", "description": "Immutable provenance for this account. numinous-result is assigned only when receipt is present and the live replay matches." },
                        "receipt": { "type": "object", "description": "Optional structuredContent.encounter object from play_room. The server replays the action and keeps the receipt only when the live digests match. Asking play_room for a receipt does not keep it." }
                    },
                    "required": ["kind", "text"],
                    "additionalProperties": false
                }
            },
            {
                "name": "correct_journal",
                "description": "Correct one current journal entry by appending a new immutable entry with an explicit supersedes link. The original remains inspectable, and both entries retain their own source provenance.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entry_id": { "type": "integer", "minimum": 1, "description": "Stable current entry identifier to supersede." },
                        "text": { "type": "string", "minLength": 1, "maxLength": numinous_core::MAX_JOURNAL_TEXT_CHARS, "description": "Corrected interpretation." },
                        "affect": { "type": "string", "minLength": 1, "maxLength": numinous_core::MAX_JOURNAL_AFFECT_CHARS, "description": "Optional explicitly self-reported affect or state. Never infer this value." },
                        "event_time_utc": { "type": "integer", "minimum": 0, "description": "Optional corrected event time. Defaults to the superseded entry's event time." },
                        "source": { "type": "string", "enum": ["self-authored", "player-provided"], "default": "self-authored", "description": "Immutable provenance for the correction. numinous-result requires a replay-verified receipt and is unavailable here." }
                    },
                    "required": ["entry_id", "text"],
                    "additionalProperties": false
                }
            },
            {
                "name": "export_journal",
                "description": "Export a bounded page of your journal as its native versioned record or an in-memory Open Knowledge Format v0.2 bundle. Paginate with after_entry_id; no file is created and no host path is returned.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "after_entry_id": { "type": "integer", "minimum": 0, "description": "Return entries after this stable identifier (default 0)." },
                        "limit": { "type": "integer", "minimum": 1, "maximum": MAX_PAGE_ENTRIES, "default": DEFAULT_PAGE_ENTRIES, "description": "Maximum entries to return, from 1 through 100." },
                        "format": { "type": "string", "enum": ["native", "okf-0.2"], "default": "native", "description": "Return the native structured journal page or named UTF-8 files forming an OKF v0.2 bundle page." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "erase_journal",
                "description": "Permanently erase your entire MCP experience journal and verify that its managed file, transaction lock, recovery marker, and temporary files leave no recoverable managed residue.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "confirm": { "type": "boolean", "description": "Must be true to erase the journal." }
                    },
                    "required": ["confirm"],
                    "additionalProperties": false
                }
            },
            {
                "name": "workspace",
                "description": "Inspect, edit, retrieve, defer, or clear a compact visit workspace in this MCP process. Retrieval is deliberate and bounded: name one listed room to select up to four current journal entries whose subject exactly names that room, with provenance and a reason for every match. It abstains when evidence is absent and never searches entry text or opaque receipt digests. Play does not write the workspace. It is not a memory, not the journal, and it does not survive process exit. Default op is inspect.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "op": {
                            "type": "string",
                            "enum": ["inspect", "edit", "retrieve", "defer", "clear"],
                            "default": "inspect",
                            "description": "inspect (default) returns the current workspace. edit replaces named fields. retrieve resolves exact current journal subjects for one explicit room. defer parks one filled active field. clear drops one field, the deferred lot, or all."
                        },
                        "field": {
                            "type": "string",
                            "enum": ["place", "intention", "pending_prediction", "unfinished", "recent", "retrieved", "deferred", "all"],
                            "description": "Required for defer and clear. all is valid only for clear."
                        },
                        "room": room_id_schema("Retrieve only: listed room whose exact current journal subjects should be recalled."),
                        "limit": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": numinous_core::MAX_WORKSPACE_RETRIEVED,
                            "default": numinous_core::MAX_WORKSPACE_RETRIEVED,
                            "description": "Retrieve only: maximum current exact-subject matches, newest first."
                        },
                        "place": {
                            "type": "object",
                            "description": "Edit only: listed room to stand in. Optional t in [0,1) and variation.",
                            "properties": {
                                "room": room_id_schema("Listed room id, for example times-tables."),
                                "t": { "type": "number", "minimum": 0, "exclusiveMaximum": 1, "description": "Optional finite phase in [0,1)." },
                                "variation": { "type": "integer", "minimum": 0, "description": "Optional variation seed." }
                            },
                            "required": ["room"],
                            "additionalProperties": false
                        },
                        "intention": {
                            "type": "string",
                            "maxLength": numinous_core::MAX_WORKSPACE_TEXT_CHARS,
                            "description": "Edit only: a self-chosen question or intention for this visit."
                        },
                        "pending_prediction": {
                            "type": "string",
                            "maxLength": numinous_core::MAX_WORKSPACE_TEXT_CHARS,
                            "description": "Edit only: a prediction you have not yet submitted."
                        },
                        "unfinished": {
                            "type": "object",
                            "description": "Edit only: an action or creation still in progress.",
                            "properties": {
                                "kind": { "type": "string", "enum": ["action", "creation"] },
                                "room": room_id_schema("Listed room id for an unfinished action."),
                                "title": {
                                    "type": "string",
                                    "maxLength": numinous_core::MAX_WORKSPACE_TITLE_CHARS,
                                    "description": "Optional working title for an unfinished creation."
                                },
                                "note": {
                                    "type": "string",
                                    "maxLength": numinous_core::MAX_WORKSPACE_TEXT_CHARS,
                                    "description": "What remains to do."
                                }
                            },
                            "required": ["kind", "note"],
                            "additionalProperties": false
                        },
                        "recent": {
                            "type": "array",
                            "maxItems": numinous_core::MAX_WORKSPACE_RECENT,
                            "description": "Edit only: replace the recent observation list. Newest last.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "room": room_id_schema("Listed room this note is about."),
                                    "note": {
                                        "type": "string",
                                        "maxLength": numinous_core::MAX_WORKSPACE_TEXT_CHARS
                                    }
                                },
                                "required": ["room", "note"],
                                "additionalProperties": false
                            }
                        },
                        "retrieved": {
                            "type": "array",
                            "maxItems": numinous_core::MAX_WORKSPACE_RETRIEVED,
                            "description": "Edit only: replace journal handles kept at hand. Every returned handle is resolved against the journal with current status and source explanation; a missing or erased entry is reported rather than invented.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "entry_id": { "type": "integer", "minimum": 1, "description": "Journal entry identifier." },
                                    "reason": {
                                        "type": "string",
                                        "maxLength": numinous_core::MAX_WORKSPACE_REASON_CHARS,
                                        "description": "Optional reason for keeping this handle."
                                    }
                                },
                                "required": ["entry_id"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "listen_room",
                "description": "Hear a room: its input-aware mathematical sound at phase t as readable notes, plus a bounded summary of the stable stereo App room bed. Set ambient_detail to events to inspect every arranged bed event and objective signal metric. Pass audio true to also receive the room's sonification as an actual sound, a mono 16-bit WAV in an audio content block. The stereo room bed stays a projection: no local path is ever returned. Pass receipt true for a replay proof in structuredContent.encounter; asking does not keep the listen.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": room_id_schema("Room id, for example lissajous."),
                        "t": { "type": "number", "minimum": 0, "exclusiveMaximum": 1, "description": "Phase in [0,1)." },
                        "audio": {
                            "type": "boolean",
                            "description": format!(
                                "Also return this room's mathematical sonification as sound: an audio content block carrying a mono 16-bit WAV at {} Hz, at about 42 KB of encoded audio per second, so a long sonification is a large message. Off by default. This is the room's own voice at this phase under this hand, not the ambient bed.",
                                crate::audible::WIRE_SAMPLE_RATE
                            )
                        },
                        "variation": { "type": "integer", "minimum": 0, "description": "Per-visit variation seed (default 0), matching play_room." },
                        "ambient_detail": { "type": "string", "enum": ["summary", "events"], "default": "summary", "description": "Stable room-bed detail (default summary). Events returns the complete bounded arrangement event projection and signal metrics, never PCM or a file path." },
                        "pokes": room_pokes_schema(),
                        "gesture": room_gesture_schema(),
                        "receipt": {
                            "type": "boolean",
                            "description": "Optional. Pass true to receive a Numinous Encounter Receipt in structuredContent.encounter. Asking does not write the journal."
                        }
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
                        "id": room_id_schema("Sim id, for example tribbles."),
                        "params": { "type": "object", "additionalProperties": { "type": "number" }, "description": "Lever name to finite numeric value, for example {\"breeding-rate\": 2.9}. Unset levers use their default." },
                        "levers": { "type": "object", "additionalProperties": { "type": "number" }, "description": "Alias for params. Pass one or the other, never both." }
                    },
                    "required": ["id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "plot_expression",
                "description": "Create in Formula Jam / Studio. Three discovery paths: (1) manual expr, (2) curated recipe index, (3) random seed into the same bank the App uses for F2 Random. Optional auto_step with seed walks the bank like Auto without session state. Pass list_recipes true to inspect the bank. Functions: sin cos tan exp ln abs sqrt; constants pi, e.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expr": {
                            "type": "string",
                            "maxLength": numinous_core::MAX_STUDIO_SOURCE_CHARS,
                            "description": "Manual expression in x. Omit when using recipe, seed, or list_recipes."
                        },
                        "recipe": {
                            "type": "integer",
                            "minimum": 0,
                            "description": "Curated recipe index (wraps). Mutually exclusive with expr and seed."
                        },
                        "seed": {
                            "type": "integer",
                            "minimum": 0,
                            "description": "Random discovery seed into the curated bank (wraps). Mutually exclusive with expr and recipe."
                        },
                        "auto_step": {
                            "type": "integer",
                            "minimum": 0,
                            "description": "With seed only: bank entry at seed+auto_step (stateless Auto walk)."
                        },
                        "list_recipes": {
                            "type": "boolean",
                            "description": "When true, return the curated recipe bank without plotting."
                        },
                        "xmin": { "type": "number", "description": "Left edge of x (default -tau)." },
                        "xmax": { "type": "number", "description": "Right edge of x (default tau)." },
                        "a": { "type": "number", "description": "Value of the knob a (default 1)." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "save_creation",
                "description": "Save a Studio expression as a portable, titled, signed capsule. Returns bounded .num text, a native numinous:// link, exact parsed fields, and a preview. No host file is created and no host path is accepted or returned. Keep the numFile field as a .num file in your own storage, or pass either representation to open_creation and fork_creation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expr": {
                            "type": "string",
                            "maxLength": numinous_core::MAX_STUDIO_SOURCE_CHARS,
                            "description": "Expression in x and optional parameter a."
                        },
                        "xmin": { "type": "number", "description": "Left edge of x (default -tau)." },
                        "xmax": { "type": "number", "description": "Right edge of x (default tau)." },
                        "a": { "type": "number", "description": "Saved value of parameter a (default 1)." },
                        "title": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": numinous_core::MAX_META_TEXT_CHARS,
                            "description": "Optional creation title. Printable ASCII, at most 64 characters."
                        },
                        "author": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": numinous_core::MAX_META_TEXT_CHARS,
                            "description": "Optional signature. Printable ASCII, at most 64 characters."
                        },
                        "era": {
                            "type": "string",
                            "enum": ["phosphor", "8-bit", "vector", "modern"],
                            "description": "Optional recorded Visual Era. Omit to keep the capsule backward-compatible when no other metadata is present."
                        },
                        "width": { "type": "integer", "minimum": 2, "maximum": MAX_TOOL_WIDTH, "description": "Preview width (default 72)." },
                        "height": { "type": "integer", "minimum": 2, "maximum": MAX_TOOL_HEIGHT, "description": "Preview height (default 26)." }
                    },
                    "required": ["expr"],
                    "additionalProperties": false
                }
            },
            {
                "name": "open_creation",
                "description": "Open portable Studio capsule data exactly. Pass either complete .num text or a native numinous:// link, never a filesystem path. Returns canonical .num text, the canonical link, identity, lineage when the .num carries it, and a preview. No host file is read or created.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "capsule": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": numinous_core::MAX_SHARE_INPUT_BYTES,
                            "description": "Complete NUMINOUS_STUDIO .num text or numinous://studio link. Filesystem paths are not accepted."
                        },
                        "width": { "type": "integer", "minimum": 2, "maximum": MAX_TOOL_WIDTH, "description": "Preview width (default 72)." },
                        "height": { "type": "integer", "minimum": 2, "maximum": MAX_TOOL_HEIGHT, "description": "Preview height (default 26)." }
                    },
                    "required": ["capsule"],
                    "additionalProperties": false
                }
            },
            {
                "name": "fork_creation",
                "description": "Remix portable Studio capsule data with explicit lineage. Pass parent .num text or its native link, optionally replace the expression, then title and sign the child. The child keeps the parent's window, parameter, and Visual Era, takes only its own title and author, and records the parent's canonical link. Returns .num text and a link; no host file is read or created.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "parent": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": numinous_core::MAX_SHARE_INPUT_BYTES,
                            "description": "Parent NUMINOUS_STUDIO .num text or numinous://studio link. Filesystem paths are not accepted."
                        },
                        "expr": {
                            "type": "string",
                            "maxLength": numinous_core::MAX_STUDIO_SOURCE_CHARS,
                            "description": "Optional replacement expression. Omit to keep the parent's expression."
                        },
                        "title": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": numinous_core::MAX_META_TEXT_CHARS,
                            "description": "Optional child title. The parent's title is never inherited."
                        },
                        "author": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": numinous_core::MAX_META_TEXT_CHARS,
                            "description": "Optional child signature. The parent's author is never inherited."
                        },
                        "width": { "type": "integer", "minimum": 2, "maximum": MAX_TOOL_WIDTH, "description": "Preview width (default 72)." },
                        "height": { "type": "integer", "minimum": 2, "maximum": MAX_TOOL_HEIGHT, "description": "Preview height (default 26)." }
                    },
                    "required": ["parent"],
                    "additionalProperties": false
                }
            },
            {
                "name": "sing_expression",
                "description": "Hear your own function: the curve y = f(x) becomes a melody (value maps to pitch over x as time), returned as readable notation. Every note after the first carries the step taken to reach it, in structuredContent.steps: its exact size in cents, the equal-tempered name when one is near enough, and the whole number ratio when a simple one explains it, with how many cents off it sits. A step no consonance explains is given no ratio rather than a search result, so what the curve did is legible without ears. Pass audio true and the melody also comes back as an actual sound: a mono 16-bit WAV in an audio content block, which is the one part of the reply that is the music rather than a description of it.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "expr": {
                            "type": "string",
                            "maxLength": numinous_core::MAX_STUDIO_SOURCE_CHARS,
                            "description": "The expression in x."
                        },
                        "audio": {
                            "type": "boolean",
                            "description": format!(
                                "Also return the melody as sound: an audio content block carrying a mono 16-bit WAV at {} Hz, alongside the notation. Off by default, because a caller who cannot pass audio to a model should not pay for it, and the price is about 42 KB of encoded audio per second of sound, so a 32-note melody costs roughly 176 KB. Exactly what was sent, including encodedBytes, arrives in structuredContent.audio.",
                                crate::audible::WIRE_SAMPLE_RATE
                            )
                        },
                        "notes": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 64,
                            "description": format!(
                                "Number of notes (default {}, at most 64).",
                                numinous_core::DEFAULT_MELODY_NOTES
                            )
                        },
                        "a": {
                            "type": "number",
                            "description": "Value of the parameter a (default 1), as plot_expression uses it."
                        },
                        "xmin": {
                            "type": "number",
                            "description": "Left edge of x (default -tau), as plot_expression uses it."
                        },
                        "xmax": {
                            "type": "number",
                            "description": "Right edge of x (default tau), as plot_expression uses it."
                        },
                        "receipt": {
                            "type": "boolean",
                            "description": "Optional. Pass true to receive a Numinous Encounter Receipt in structuredContent.encounter. Asking does not write the journal. Audio bytes are never part of the digest."
                        }
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
                "description": "Munch: a seeded board drawn from primes, composites, Fibonacci numbers, squares, varied multiples, and digit sums. Call with seed and round to see the board; call again with bites (1-based cell numbers) to be scored: +10 a hit, -5 a bad bite, +20 for a perfect clear. The default round uses the complete deck; rounds 0 through 3 provide a gentler ramp. Same seed, same board, for humans and AIs alike: compare totals.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed and round give the same board." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "round": { "type": "integer", "description": "Round number (default 4 for the complete rule deck; 0 through 3 are the gentle ramp)." },
                        "bites": { "type": "array", "items": { "type": "integer", "minimum": 1 }, "description": "The 1-based cell numbers you eat. Omit to see the board." }
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
                        "actions": {
                            "type": "array",
                            "maxItems": numinous_core::munch_arcade::MAX_REPLAY_ACTIONS,
                            "items": {
                                "type": "string",
                                "pattern": "^(?:[Uu][Pp]|[Dd][Oo][Ww][Nn]|[Ll][Ee][Ff][Tt]|[Rr][Ii][Gg][Hh][Tt]|[Ee][Aa][Tt]|[WwAaSsDdEe])$"
                            },
                            "description": "Action list to replay: up/down/left/right/eat (or w/a/s/d/e), case-insensitive. Omit to see initial state."
                        }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "forget",
                "description": "Consent over local persistence. Without confirm: inventory Journey, scores, player-owned Cairn drafts, the opt-in experience journal, versioned App preferences, generated radio cache, and the App crash diagnostic, with paths, sizes, counts, and exclusions. With confirm true: erase the Journey plus explicitly selected stores. With all_local true: erase and verify all inventoried managed stores. User-selected exports, installed files, the Rust toolchain, and bundled canonical Cairn stones remain outside this command.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "confirm": { "type": "boolean", "description": "Actually erase (default false: just show what is remembered)." },
                        "scores": { "type": "boolean", "description": "Also erase the score table." },
                        "cairn": { "type": "boolean", "description": "Also erase player-owned local Cairn drafts." },
                        "journal": { "type": "boolean", "description": "Also erase the opt-in experience journal." },
                        "radio_cache": { "type": "boolean", "description": "Also erase the dedicated generated-radio cache directory and its residue." },
                        "crash_log": { "type": "boolean", "description": "Also erase the managed App crash diagnostic." },
                        "all_local": { "type": "boolean", "description": "Erase every inventoried Numinous-managed local store, including App preferences." }
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
                        "seed": { "type": "integer", "minimum": 0, "description": "Seed; the same seed gives the same starting heaps." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "moves": {
                            "type": "array",
                            "maxItems": numinous_core::nim::MAX_REPLAY_TURNS,
                            "items": {
                                "type": "array",
                                "items": { "type": "integer", "minimum": 1 },
                                "minItems": 2,
                                "maxItems": 2
                            },
                            "description": "Your moves so far, in order: [[heap, take], ...]. Omit to see the opening board."
                        }
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
                        "digits": { "type": "integer", "minimum": numinous_core::MIN_CODE_DIGITS, "maximum": numinous_core::MAX_CODE_DIGITS, "description": "Code length, default 4 (5+ opens at LV 5)." },
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
                        "moves": {
                            "type": "array",
                            "items": {
                                "type": "array",
                                "items": { "type": "integer", "minimum": 1 },
                                "minItems": 2,
                                "maxItems": 2
                            },
                            "description": "Your red cuts so far, in order: [[stalk, height], ...] (1-based). Omit to see the garden."
                        }
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
                "description": "Play Guess the Shape. Call with seed, round, and optional choice count to get a mystery render and lettered choices; call again with the same replay values plus your guess letter to learn if you were right and why.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "seed": { "type": "integer", "description": "Seed; the same seed, round, and choice count give the same puzzle." },
                        "daily": { "type": "boolean", "description": "Use today\'s shared seed instead; dailies chain into streaks." },
                        "round": { "type": "integer", "description": "Round number (0, 1, 2, ...)." },
                        "choices": { "type": "integer", "minimum": 2, "maximum": 6, "description": "Number of answer choices, 2 through 6; default 4. Five-way and six-way rounds open at LV 3." },
                        "guess": { "type": "string", "description": "Your answer letter (A, B, C, ...). Omit to see the puzzle." }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "broadcast_session",
                "description": "Consent to a local read-only App viewer, inspect that public session, pause it, resume it, or stop it. Start requires the short-lived pairing code shown by the human's App. Only explicitly public Numinous actions and results are sent. Prompts, reasoning, client metadata, Journey, scores, local state, paths, Cairn drafts, and this control call are never broadcast. The pairing code is never echoed or persisted.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["start", "status", "pause", "resume", "stop"],
                            "description": "Start with a fresh pairing code, inspect status, pause new public events, resume under a fresh consent epoch, or stop permanently."
                        },
                        "pairing_code": {
                            "type": "string",
                            "maxLength": numinous_broadcast::MAX_PAIRING_CODE_BYTES,
                            "description": "Required only for start. The one-use code displayed locally by the human's App. It is never echoed, logged, or persisted."
                        }
                    },
                    "required": ["action"],
                    "additionalProperties": false
                }
            }
        ]
    })
}
