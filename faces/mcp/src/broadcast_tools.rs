//! MCP lifecycle and result projection for explicitly consented App viewing.

use std::sync::{Mutex, MutexGuard};

use numinous_broadcast::PublicTool;
use serde_json::{Value, json};

use crate::broadcast::{self, SessionBroadcast, SessionSnapshot};
use crate::{tool_error, tool_structured};

pub(super) struct ConnectionBroadcast {
    session: Mutex<SessionBroadcast>,
}

impl ConnectionBroadcast {
    pub(super) fn new() -> Self {
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

    pub(super) fn capture(
        &self,
        tool: PublicTool,
        arguments: &Value,
    ) -> Option<broadcast::PublicCall> {
        self.lock().capture(tool, arguments)
    }

    fn lock(&self) -> MutexGuard<'_, SessionBroadcast> {
        self.session
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub(super) fn broadcast_session_tool(args: &Value, broadcast: &ConnectionBroadcast) -> Value {
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
