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
mod game_tools;
mod journal;
mod local_state;
mod portable;
mod progress;
mod protocol;
mod response;
mod room_door;
mod room_input;
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
use encounter::{
    action_json as encounter_action_json, delta_counts as encounter_delta_counts,
    dwell_counts as encounter_dwell_counts, issue as issue_encounter, issue_receipt,
    listen_action as encounter_listen_action, listen_action_json,
    listen_result as encounter_listen_result, play_action as encounter_play_action,
    play_result as encounter_play_result, receipt_json, request as encounter_request,
};
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
use numinous_core::{Canvas, room_by_id};
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
use room_input::{gesture_json, parse_room_inputs, render_room_observation, room_status_at};
use schema::{validate_declared_tool_arguments, validate_schema_value};
use serde_json::{Value, json};
use sim_tools::{list_sims_text, run_sim_tool};
use studio_tools::{
    fork_creation_tool, open_creation_tool, plot_expression_tool, save_creation_tool,
    sing_expression_tool,
};
use temporal::{dwell_evidence_json, evidence_json as temporal_evidence_json, render_delta_json};
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

fn play_room_tool(args: &Value, journey_file: &std::path::Path) -> Value {
    play_room_tool_for_journey(args, &load_journey(journey_file))
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
