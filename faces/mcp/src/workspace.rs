//! Process-local MCP session workspace: inspect, edit, defer, or clear.

use std::sync::{Mutex, MutexGuard};

use numinous_core::{
    MAX_WORKSPACE_RECENT, MAX_WORKSPACE_RETRIEVED, SESSION_WORKSPACE_SCHEMA,
    SESSION_WORKSPACE_SCHEMA_VERSION, SessionWorkspace, WorkspaceClear, WorkspaceError,
    WorkspaceField, WorkspaceObservation, WorkspaceObservationDraft, WorkspacePlace,
    WorkspacePlaceDraft, WorkspaceRetrieval, WorkspaceRetrievalDraft, WorkspaceUnfinished,
    WorkspaceUnfinishedDraft, WorkspaceUpdate,
};
use serde_json::{Value, json};

use super::{tool_error, tool_structured};

/// Mutex-owned visit workspace for one MCP process.
pub(super) struct ProcessWorkspace {
    inner: Mutex<SessionWorkspace>,
}

impl ProcessWorkspace {
    pub(super) fn new() -> Self {
        Self {
            inner: Mutex::new(SessionWorkspace::new()),
        }
    }

    fn lock(&self) -> MutexGuard<'_, SessionWorkspace> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub(super) fn workspace_tool(args: &Value, workspace: &ProcessWorkspace) -> Value {
    let op = args.get("op").and_then(Value::as_str).unwrap_or("inspect");
    let mut state = workspace.lock();
    let outcome = match op {
        "inspect" => Ok(()),
        "edit" => parse_update(args).and_then(|update| state.edit(update)),
        "defer" => required_field(args).and_then(|field| state.defer(field)),
        "clear" => required_clear(args).map(|field| state.clear(field)),
        _ => {
            return tool_error(
                "Unknown workspace op. Use inspect, edit, defer, or clear. Inspect is the default.",
            );
        }
    };
    match outcome {
        Ok(()) => {
            let structured = workspace_json(&state);
            tool_structured(&workspace_prose(&state), structured)
        }
        Err(error) => tool_error(&error.to_string()),
    }
}

pub(super) fn compact_workspace_summary(structured: &Value) -> Option<String> {
    let empty = structured.get("empty")?.as_bool()?;
    if empty {
        return Some(
            "Workspace is empty. It lives only in this process. Read structuredContent."
                .to_string(),
        );
    }
    let occupied = occupied_count(structured);
    Some(format!(
        "Workspace holds {occupied} active slot(s). Process-local; not a memory. Read structuredContent."
    ))
}

fn occupied_count(structured: &Value) -> u64 {
    let mut count = 0;
    for key in ["place", "intention", "pending_prediction", "unfinished"] {
        if structured.get(key).is_some_and(|value| !value.is_null()) {
            count += 1;
        }
    }
    if structured
        .get("recent")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        count += 1;
    }
    if structured
        .get("retrieved")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        count += 1;
    }
    count
}

fn required_field(args: &Value) -> Result<WorkspaceField, WorkspaceError> {
    let name = args
        .get("field")
        .and_then(Value::as_str)
        .ok_or(WorkspaceError::UnknownField)?;
    WorkspaceField::parse(name)
}

fn required_clear(args: &Value) -> Result<WorkspaceClear, WorkspaceError> {
    let name = args
        .get("field")
        .and_then(Value::as_str)
        .ok_or(WorkspaceError::UnknownField)?;
    WorkspaceClear::parse(name)
}

fn parse_update(args: &Value) -> Result<WorkspaceUpdate, WorkspaceError> {
    Ok(WorkspaceUpdate {
        place: optional_place(args)?,
        intention: optional_string(args, "intention"),
        pending_prediction: optional_string(args, "pending_prediction"),
        unfinished: optional_unfinished(args)?,
        recent: optional_recent(args)?,
        retrieved: optional_retrieved(args)?,
    })
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn optional_place(args: &Value) -> Result<Option<WorkspacePlaceDraft>, WorkspaceError> {
    let Some(value) = args.get("place") else {
        return Ok(None);
    };
    let object = value.as_object().ok_or(WorkspaceError::UnknownField)?;
    let room = object
        .get("room")
        .and_then(Value::as_str)
        .ok_or(WorkspaceError::UnknownField)?
        .to_string();
    let t = match object.get("t") {
        Some(value) => Some(value.as_f64().ok_or(WorkspaceError::InvalidPhase)?),
        None => None,
    };
    let variation = match object.get("variation") {
        Some(value) => Some(value.as_u64().ok_or(WorkspaceError::UnknownField)?),
        None => None,
    };
    Ok(Some(WorkspacePlaceDraft { room, t, variation }))
}

fn optional_unfinished(args: &Value) -> Result<Option<WorkspaceUnfinishedDraft>, WorkspaceError> {
    let Some(value) = args.get("unfinished") else {
        return Ok(None);
    };
    let object = value.as_object().ok_or(WorkspaceError::UnknownField)?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(WorkspaceError::UnknownField)?;
    let note = object
        .get("note")
        .and_then(Value::as_str)
        .ok_or(WorkspaceError::UnknownField)?
        .to_string();
    match kind {
        "action" => Ok(Some(WorkspaceUnfinishedDraft::Action {
            room: object
                .get("room")
                .and_then(Value::as_str)
                .map(str::to_string),
            note,
        })),
        "creation" => Ok(Some(WorkspaceUnfinishedDraft::Creation {
            title: object
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string),
            note,
        })),
        _ => Err(WorkspaceError::UnknownField),
    }
}

fn optional_recent(args: &Value) -> Result<Option<Vec<WorkspaceObservationDraft>>, WorkspaceError> {
    let Some(value) = args.get("recent") else {
        return Ok(None);
    };
    let items = value.as_array().ok_or(WorkspaceError::UnknownField)?;
    items
        .iter()
        .map(|item| {
            let object = item.as_object().ok_or(WorkspaceError::UnknownField)?;
            Ok(WorkspaceObservationDraft {
                room: object
                    .get("room")
                    .and_then(Value::as_str)
                    .ok_or(WorkspaceError::UnknownField)?
                    .to_string(),
                note: object
                    .get("note")
                    .and_then(Value::as_str)
                    .ok_or(WorkspaceError::UnknownField)?
                    .to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn optional_retrieved(
    args: &Value,
) -> Result<Option<Vec<WorkspaceRetrievalDraft>>, WorkspaceError> {
    let Some(value) = args.get("retrieved") else {
        return Ok(None);
    };
    let items = value.as_array().ok_or(WorkspaceError::UnknownField)?;
    items
        .iter()
        .map(|item| {
            let object = item.as_object().ok_or(WorkspaceError::UnknownField)?;
            Ok(WorkspaceRetrievalDraft {
                entry_id: object
                    .get("entry_id")
                    .and_then(Value::as_u64)
                    .ok_or(WorkspaceError::InvalidEntryId)?,
                reason: object
                    .get("reason")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn workspace_json(workspace: &SessionWorkspace) -> Value {
    json!({
        "schema": SESSION_WORKSPACE_SCHEMA,
        "schemaVersion": SESSION_WORKSPACE_SCHEMA_VERSION,
        "scope": "process",
        "empty": workspace.is_empty(),
        "place": place_json(workspace.place()),
        "intention": workspace.intention(),
        "pending_prediction": workspace.pending_prediction(),
        "unfinished": unfinished_json(workspace.unfinished()),
        "recent": workspace.recent().iter().map(observation_json).collect::<Vec<_>>(),
        "retrieved": workspace.retrieved().iter().map(retrieval_json).collect::<Vec<_>>(),
        "deferred": {
            "place": place_json(workspace.deferred().place()),
            "intention": workspace.deferred().intention(),
            "pending_prediction": workspace.deferred().pending_prediction(),
            "unfinished": unfinished_json(workspace.deferred().unfinished()),
            "recent": workspace.deferred().recent().iter().map(observation_json).collect::<Vec<_>>(),
            "retrieved": workspace.deferred().retrieved().iter().map(retrieval_json).collect::<Vec<_>>(),
        }
    })
}

fn place_json(place: Option<&WorkspacePlace>) -> Value {
    match place {
        Some(place) => json!({
            "room": place.room(),
            "t": place.t(),
            "variation": place.variation(),
        }),
        None => Value::Null,
    }
}

fn unfinished_json(unfinished: Option<&WorkspaceUnfinished>) -> Value {
    match unfinished {
        Some(WorkspaceUnfinished::Action { room, note }) => json!({
            "kind": "action",
            "room": room,
            "note": note,
        }),
        Some(WorkspaceUnfinished::Creation { title, note }) => json!({
            "kind": "creation",
            "title": title,
            "note": note,
        }),
        None => Value::Null,
    }
}

fn observation_json(observation: &WorkspaceObservation) -> Value {
    json!({
        "room": observation.room(),
        "note": observation.note(),
    })
}

fn retrieval_json(retrieval: &WorkspaceRetrieval) -> Value {
    json!({
        "entry_id": retrieval.entry_id(),
        "reason": retrieval.reason(),
    })
}

fn workspace_prose(workspace: &SessionWorkspace) -> String {
    if workspace.is_empty() {
        return "Workspace is empty. It lives only in this MCP process: inspect, edit, defer, or clear it here. It is not a memory, and it does not record plays unless you put them here. Exit or clear all to drop it. The journal is how a visit is kept.".to_string();
    }
    let mut lines = vec!["Workspace (this process only, not a memory):".to_string()];
    push_place(&mut lines, "place", workspace.place());
    push_text(&mut lines, "intention", workspace.intention());
    push_text(
        &mut lines,
        "pending_prediction",
        workspace.pending_prediction(),
    );
    push_unfinished(&mut lines, "unfinished", workspace.unfinished());
    push_list(
        &mut lines,
        "recent",
        workspace.recent().len(),
        MAX_WORKSPACE_RECENT,
    );
    push_list(
        &mut lines,
        "retrieved",
        workspace.retrieved().len(),
        MAX_WORKSPACE_RETRIEVED,
    );
    if !workspace.deferred().is_empty() {
        lines.push("deferred: parked fields remain inspectable.".to_string());
    }
    lines.join("\n")
}

fn push_place(lines: &mut Vec<String>, label: &str, place: Option<&WorkspacePlace>) {
    if let Some(place) = place {
        let mut detail = place.room().to_string();
        if let Some(t) = place.t() {
            detail.push_str(&format!(" t={t}"));
        }
        if let Some(variation) = place.variation() {
            detail.push_str(&format!(" variation={variation}"));
        }
        lines.push(format!("{label}: {detail}"));
    }
}

fn push_text(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_unfinished(lines: &mut Vec<String>, label: &str, unfinished: Option<&WorkspaceUnfinished>) {
    match unfinished {
        Some(WorkspaceUnfinished::Action { room, note }) => {
            lines.push(format!("{label}: action in {room}: {note}"));
        }
        Some(WorkspaceUnfinished::Creation { title, note }) => {
            let title = title.as_deref().unwrap_or("untitled");
            lines.push(format!("{label}: creation {title}: {note}"));
        }
        None => {}
    }
}

fn push_list(lines: &mut Vec<String>, label: &str, count: usize, max: usize) {
    if count > 0 {
        lines.push(format!("{label}: {count}/{max}"));
    }
}
