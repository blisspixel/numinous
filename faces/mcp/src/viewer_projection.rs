//! Consent policy and replay-safe projection for the read-only App viewer.

use numinous_broadcast::PublicTool;
use serde_json::{Value, json};

use crate::broadcast;
use crate::broadcast_tools::ConnectionBroadcast;
use crate::game_tools::quiz_tool_at_level;
use crate::progress::DAILY_DAY_KEY;
use crate::puzzle_tools::{crack_tool_at_level, seti_tool_at_level};
use crate::room_tools::{describe_room_tool_for_journey, reveal_room_tool_for_journey};
use crate::show;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ViewerPolicy {
    Public(PublicTool),
    Private,
    Control,
}

pub(super) fn viewer_policy(name: &str) -> Option<ViewerPolicy> {
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

pub(super) fn capture_public_call(
    request: &Value,
    broadcast: &ConnectionBroadcast,
) -> Option<ViewerCall> {
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

pub(super) struct ViewerCall {
    call: broadcast::PublicCall,
    tool: PublicTool,
    arguments: Value,
}

impl ViewerCall {
    pub(super) fn commit(self, result: &Value) {
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
pub(super) fn level_the_arguments_require(tool: PublicTool, arguments: &Value) -> u32 {
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

pub(super) fn viewer_result(tool: PublicTool, arguments: &Value, result: &Value) -> Value {
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

pub(super) fn replay_arguments(mut arguments: Value) -> Value {
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
