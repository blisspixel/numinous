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
mod challenge_tools;
mod encounter;
mod flagship_aha;
mod game_tools;
mod journal;
mod local_state;
mod portable;
mod progress;
mod protocol;
mod response;
mod room_door;
mod room_input;
mod room_tools;
mod schema;
mod show;
mod sim_tools;
mod studio_tools;
mod temporal;
mod transport;
mod workspace;

use std::io;
use std::sync::{Mutex, MutexGuard};

use broadcast::{SessionBroadcast, SessionSnapshot};
use catalog::{discover_result, initialize_result, server_info, tools_catalog, tools_list_result};
#[cfg(test)]
use challenge_tools::record_challenge_attempt;
use challenge_tools::{challenge_tool, predict_tool};
#[cfg(test)]
use game_tools::post_munch_arcade_score;
use game_tools::{
    arcade_action, fifteen_tool, hackenbush_tool, munch_arcade_tool, munch_tool, nim_tool,
    party_tool, quiz_tool, quiz_tool_at_level, scores_tool,
};
use local_state::forget_tool;
use numinous_broadcast::{
    PLAY_ROOM_DEFAULT_HEIGHT as DEFAULT_HEIGHT, PLAY_ROOM_DEFAULT_WIDTH as DEFAULT_WIDTH,
    PLAY_ROOM_MAX_HEIGHT as MAX_TOOL_HEIGHT, PLAY_ROOM_MAX_WIDTH as MAX_TOOL_WIDTH, PublicTool,
};
use progress::{
    CAIRN_LEVEL, DAILY_DAY_KEY, cairn_path, effective_seed, freeze_daily_day, journal_path,
    journey_path, load_journey, local_state_paths_at, note_save_trouble, persist_progress,
    post_score, record_progress, scores_path,
};
#[cfg(test)]
use progress::{TestStateRoot, test_state_path};
#[cfg(test)]
use protocol::{
    CLIENT_CAPABILITIES_META_KEY, CLIENT_INFO_META_KEY, DISCOVERY_CACHE_TTL_MS,
    MODERN_PROTOCOL_VERSION, PROTOCOL_VERSION_META_KEY, SERVER_INFO_META_KEY, TOOLS_CACHE_TTL_MS,
};
use protocol::{
    JSON_SCHEMA_2020_12, RequestEra, SUPPORTED_PROTOCOL_VERSIONS, error_response,
    prepare_prediction_mrtr, protocol_error_response, request_era, result_for_era,
    success_response, valid_request_id, validate_jsonrpc_envelope,
};
use response::apply_response_mode;
#[cfg(test)]
use room_tools::play_room_tool_for_journey;
use room_tools::{
    describe_room_tool, describe_room_tool_for_journey, listen_room_tool, note_name,
    play_room_tool, reveal_room_tool, reveal_room_tool_for_journey,
};
use schema::{validate_declared_tool_arguments, validate_schema_value};
use serde_json::{Value, json};
use sim_tools::{list_sims_text, run_sim_tool};
use studio_tools::{
    fork_creation_tool, open_creation_tool, plot_expression_tool, save_creation_tool,
    sing_expression_tool,
};
use temporal::render_delta_json;
use transport::{read_bounded_line, write_message};
use workspace::{ProcessWorkspace, workspace_tool};

/// Longest catalog id a tool argument may carry (room, sim, or similar).
/// Catalog keys today are far shorter; the bound rejects hostile multi-kilobyte
/// id strings before domain dispatch.
const MAX_TOOL_ID_CHARS: usize = 64;

/// Longest author credit accepted with a Cairn bequest. Matches the sanitize
/// bound in `numinous_core::Bequest::new`.
const MAX_AUTHOR_CHARS: usize = 48;

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
        PublicTool::WatchShow => show::viewer_result(result),
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
        "watch_show" => show::tool(&domain_args),
        "describe_room" => describe_room_tool(&domain_args, journey_file),
        "reveal_room" => reveal_room_tool(&domain_args, journey_file),
        "play_room" => play_room_tool(&domain_args, journey_file),
        "challenge" => challenge_tool(&domain_args),
        "predict" => predict_tool(&domain_args),
        "cairn" => cairn_tool(&domain_args, journey_file, &cairn_path()),
        "read_journal" => journal::read_tool(&domain_args, &journal_path()),
        "record_journal" => {
            journal::record_tool(&domain_args, &journal_path(), |tool, replay_args| {
                replay_encounter(tool, replay_args, journey_file)
            })
        }
        "correct_journal" => journal::correct_tool(&domain_args, &journal_path()),
        "export_journal" => {
            journal::export_tool(&domain_args, &journal_path(), |tool, replay_args| {
                replay_encounter(tool, replay_args, journey_file)
            })
        }
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
    let result = apply_response_mode(name, response_mode, result);
    if let Err(message) = validate_declared_tool_output(name, &result) {
        return Ok(tool_error(&message));
    }
    Ok(result)
}

fn validate_declared_tool_output(name: &str, result: &Value) -> Result<(), String> {
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    let Some(schema) = tools_catalog()
        .get("tools")
        .and_then(Value::as_array)
        .and_then(|tools| {
            tools
                .iter()
                .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
        })
        .and_then(|tool| tool.get("outputSchema"))
    else {
        return Ok(());
    };
    let Some(structured) = result.get("structuredContent") else {
        return Err("The tool could not return its declared structured output.".to_string());
    };
    validate_schema_value(structured, schema, "structuredContent", 0)
        .map_err(|_| "The tool could not return its declared structured output.".to_string())
}

fn replay_encounter(
    tool: numinous_core::EncounterTool,
    replay_args: &Value,
    journey_file: &std::path::Path,
) -> Value {
    match tool {
        numinous_core::EncounterTool::PlayRoom => play_room_tool(replay_args, journey_file),
        numinous_core::EncounterTool::ListenRoom => listen_room_tool(replay_args),
        numinous_core::EncounterTool::SingExpression => sing_expression_tool(replay_args),
    }
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

/// Answer an unknown room id with the rooms it was probably meant to be, then
/// one pointer to the listing tool. Returning the whole catalog spent thousands
/// of bytes of a player's context on a typo and handed over the map this
/// project deliberately withholds (`PLAY.md`).
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

#[cfg(test)]
mod tests;
