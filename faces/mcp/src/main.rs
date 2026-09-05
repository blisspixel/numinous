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
mod broadcast_tools;
mod catalog;
mod challenge_tools;
mod encounter;
mod flagship_aha;
mod game_tools;
mod journal;
mod journey_tools;
mod local_state;
mod portable;
mod progress;
mod protocol;
mod puzzle_tools;
mod response;
mod room_door;
mod room_input;
mod room_tools;
mod schema;
mod show;
mod sim_tools;
mod studio_tools;
mod study;
#[path = "../../shared/study_json.rs"]
mod study_json;
mod temporal;
mod transport;
mod viewer_projection;
mod workspace;

use std::io;

use broadcast_tools::{ConnectionBroadcast, broadcast_session_tool};
use catalog::{discover_result, initialize_result, server_info, tools_catalog, tools_list_result};
#[cfg(test)]
use challenge_tools::record_challenge_attempt;
use challenge_tools::{challenge_tool, predict_tool};
#[cfg(test)]
use game_tools::post_munch_arcade_score;
#[cfg(test)]
use game_tools::quiz_tool_at_level;
use game_tools::{
    arcade_action, fifteen_tool, hackenbush_tool, munch_arcade_tool, munch_tool, nim_tool,
    party_tool, quiz_tool, scores_tool,
};
use journey_tools::{cairn_tool, choose_tool, journey_tool, trophies_tool};
use local_state::forget_tool;
#[cfg(test)]
use numinous_broadcast::PublicTool;
use numinous_broadcast::{
    PLAY_ROOM_DEFAULT_HEIGHT as DEFAULT_HEIGHT, PLAY_ROOM_DEFAULT_WIDTH as DEFAULT_WIDTH,
    PLAY_ROOM_MAX_HEIGHT as MAX_TOOL_HEIGHT, PLAY_ROOM_MAX_WIDTH as MAX_TOOL_WIDTH,
};
#[cfg(test)]
use progress::{CAIRN_LEVEL, DAILY_DAY_KEY, TestStateRoot, test_state_path};
use progress::{
    cairn_path, effective_seed, freeze_daily_day, journal_path, journey_path, load_journey,
    local_state_paths_at, note_save_trouble, post_score, record_progress, scores_path,
};
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
use puzzle_tools::{aliens_tool, crack_tool, gauntlet_answers_from_json, gauntlet_tool, seti_tool};
#[cfg(test)]
use puzzle_tools::{crack_tool_at_level, seti_tool_at_level};
use response::apply_response_mode;
#[cfg(test)]
use room_tools::play_room_tool_for_journey;
use room_tools::{
    describe_room_tool, listen_room_tool, note_name, play_room_tool, reveal_room_tool,
};
#[cfg(test)]
use room_tools::{describe_room_tool_for_journey, reveal_room_tool_for_journey};
use schema::{validate_declared_tool_arguments, validate_schema_value};
use serde_json::{Value, json};
use sim_tools::{list_sims_text, run_sim_tool};
use studio_tools::{
    fork_creation_tool, open_creation_tool, plot_expression_tool, save_creation_tool,
    sing_expression_tool,
};
use temporal::render_delta_json;
use transport::{read_bounded_line, write_message};
use viewer_projection::capture_public_call;
#[cfg(test)]
use viewer_projection::{
    ViewerPolicy, level_the_arguments_require, replay_arguments, viewer_policy, viewer_result,
};
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

fn validate_tools_cursor(params: Option<&Value>) -> Result<(), String> {
    if params.and_then(|params| params.get("cursor")).is_some() {
        return Err(
            "Numinous returns its complete tool catalog in one page; cursor is invalid."
                .to_string(),
        );
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
        "watch_show" => show::tool(&domain_args),
        "describe_room" => describe_room_tool(&domain_args, journey_file),
        "reveal_room" => reveal_room_tool(&domain_args, journey_file),
        "study_room" => study::tool(&domain_args),
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
