//! Process-local MCP session workspace and deliberate journal retrieval.

use std::sync::{Mutex, MutexGuard};

use numinous_core::{
    MAX_WORKSPACE_RECENT, MAX_WORKSPACE_RETRIEVED, SESSION_WORKSPACE_SCHEMA,
    SESSION_WORKSPACE_SCHEMA_VERSION, SessionWorkspace, WorkspaceClear, WorkspaceError,
    WorkspaceField, WorkspaceObservation, WorkspaceObservationDraft, WorkspacePlace,
    WorkspacePlaceDraft, WorkspaceRetrieval, WorkspaceRetrievalDraft, WorkspaceUnfinished,
    WorkspaceUnfinishedDraft, WorkspaceUpdate,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{journal::entry_json, tool_error, tool_structured};

const DEFAULT_RETRIEVAL_LIMIT: usize = MAX_WORKSPACE_RETRIEVED;

struct RetrievalOutcome {
    room: String,
    limit: usize,
    returned: usize,
}

struct RetrievedRoom {
    journal: numinous_core::Journal,
    outcome: RetrievalOutcome,
    entry_ids: Vec<u64>,
}

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

pub(super) fn workspace_tool(
    args: &Value,
    workspace: &ProcessWorkspace,
    journal_path: &std::path::Path,
) -> Value {
    let op = args.get("op").and_then(Value::as_str).unwrap_or("inspect");
    let mut state = workspace.lock();
    let mut journal = None;
    let mut retrieval = None;
    let outcome = match op {
        "inspect" => Ok(()),
        "edit" => {
            let mut update = match parse_update(args) {
                Ok(update) => update,
                Err(error) => return tool_error(&error.to_string()),
            };
            if let Some(handles) = update.retrieved.as_mut()
                && !handles.is_empty()
            {
                let snapshot = match numinous_core::try_load_journal_file(journal_path) {
                    Ok(journal) => journal,
                    Err(error) => {
                        return tool_error(&format!(
                            "Failed to select workspace journal handles: {error}"
                        ));
                    }
                };
                bind_retrieved(handles, &snapshot);
                journal = Some(snapshot);
            }
            state.edit(update)
        }
        "defer" => required_field(args).and_then(|field| state.defer(field)),
        "clear" => required_clear(args).map(|field| state.clear(field)),
        "retrieve" => match retrieve_room(args, journal_path) {
            Ok(found) => {
                let reason = retrieval_reason(&found.outcome.room);
                let mut retrieved = found
                    .entry_ids
                    .iter()
                    .map(|entry_id| WorkspaceRetrievalDraft {
                        entry_id: *entry_id,
                        reason: Some(reason.clone()),
                        record_digest: None,
                    })
                    .collect::<Vec<_>>();
                bind_retrieved(&mut retrieved, &found.journal);
                let result = state.edit(WorkspaceUpdate {
                    retrieved: Some(retrieved),
                    ..WorkspaceUpdate::default()
                });
                if result.is_ok() {
                    retrieval = Some(found.outcome);
                    journal = Some(found.journal);
                }
                result
            }
            Err(message) => return tool_error(&message),
        },
        _ => {
            return tool_error(
                "Unknown workspace op. Use inspect, edit, retrieve, defer, or clear. Inspect is the default.",
            );
        }
    };
    match outcome {
        Ok(()) => {
            if journal.is_none() && has_retrieved_handles(&state) {
                journal = match numinous_core::try_load_journal_file(journal_path) {
                    Ok(journal) => Some(journal),
                    Err(error) => {
                        return tool_error(&format!(
                            "Failed to resolve workspace journal handles: {error}"
                        ));
                    }
                };
            }
            let structured = workspace_json(&state, journal.as_ref(), retrieval.as_ref());
            tool_structured(&workspace_prose(&state), structured)
        }
        Err(error) => tool_error(&error.to_string()),
    }
}

pub(super) fn compact_workspace_summary(structured: &Value) -> Option<String> {
    if let Some(retrieval) = structured.get("retrieval") {
        let room = retrieval.get("room")?.as_str()?;
        let returned = retrieval.get("returned")?.as_u64()?;
        if retrieval.get("abstained")?.as_bool()? {
            return Some(format!(
                "No current exact-subject journal evidence for {room}; retrieval abstained. Read structuredContent.retrieval."
            ));
        }
        return Some(format!(
            "Retrieved {returned} current journal entr{} for {room}, newest first. Read structuredContent.retrieved for provenance.",
            if returned == 1 { "y" } else { "ies" }
        ));
    }
    let empty = structured.get("empty")?.as_bool()?;
    if empty {
        return Some(
            "Workspace is empty. It lives only in this process. Read structuredContent."
                .to_string(),
        );
    }
    let occupied = occupied_count(structured);
    Some(format!(
        "Workspace holds {occupied} active slot(s). Process-local; journal handles include resolution and provenance. Read structuredContent."
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

fn retrieve_room(args: &Value, journal_path: &std::path::Path) -> Result<RetrievedRoom, String> {
    let room = args
        .get("room")
        .and_then(Value::as_str)
        .ok_or_else(|| "Retrieve needs a listed room id in 'room'.".to_string())?;
    let Some(metadata) = numinous_core::room_meta_by_id(room) else {
        return Err(WorkspaceError::UnknownRoom(room.to_string()).to_string());
    };
    let limit = retrieval_limit(args)?;
    let journal = numinous_core::try_load_journal_file(journal_path)
        .map_err(|error| format!("Failed to retrieve from journal: {error}"))?;
    let found = journal
        .current_room_entries(metadata.id, limit)
        .iter()
        .map(|entry| entry.entry_id)
        .collect::<Vec<_>>();
    Ok(RetrievedRoom {
        journal,
        outcome: RetrievalOutcome {
            room: metadata.id.to_string(),
            limit,
            returned: found.len(),
        },
        entry_ids: found,
    })
}

fn retrieval_limit(args: &Value) -> Result<usize, String> {
    let Some(value) = args.get("limit") else {
        return Ok(DEFAULT_RETRIEVAL_LIMIT);
    };
    let limit = value
        .as_u64()
        .and_then(|limit| usize::try_from(limit).ok())
        .ok_or_else(|| "Retrieval limit must be a positive integer.".to_string())?;
    if !(1..=MAX_WORKSPACE_RETRIEVED).contains(&limit) {
        return Err(format!(
            "Retrieval limit must be between 1 and {MAX_WORKSPACE_RETRIEVED}."
        ));
    }
    Ok(limit)
}

fn retrieval_reason(room: &str) -> String {
    format!("Exact current journal subject match for room '{room}'.")
}

fn has_retrieved_handles(workspace: &SessionWorkspace) -> bool {
    !workspace.retrieved().is_empty() || !workspace.deferred().retrieved().is_empty()
}

fn bind_retrieved(handles: &mut [WorkspaceRetrievalDraft], journal: &numinous_core::Journal) {
    for handle in handles {
        handle.record_digest = journal.entry(handle.entry_id).map(record_digest);
    }
}

fn record_digest(entry: &numinous_core::JournalEntry) -> [u8; 32] {
    Sha256::digest(entry.identity_bytes()).into()
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
                record_digest: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn workspace_json(
    workspace: &SessionWorkspace,
    journal: Option<&numinous_core::Journal>,
    retrieval: Option<&RetrievalOutcome>,
) -> Value {
    let mut structured = json!({
        "schema": SESSION_WORKSPACE_SCHEMA,
        "schemaVersion": SESSION_WORKSPACE_SCHEMA_VERSION,
        "scope": "process",
        "empty": workspace.is_empty(),
        "place": place_json(workspace.place()),
        "intention": workspace.intention(),
        "pending_prediction": workspace.pending_prediction(),
        "unfinished": unfinished_json(workspace.unfinished()),
        "recent": workspace.recent().iter().map(observation_json).collect::<Vec<_>>(),
        "retrieved": workspace.retrieved().iter().map(|handle| retrieval_json(handle, journal)).collect::<Vec<_>>(),
        "deferred": {
            "place": place_json(workspace.deferred().place()),
            "intention": workspace.deferred().intention(),
            "pending_prediction": workspace.deferred().pending_prediction(),
            "unfinished": unfinished_json(workspace.deferred().unfinished()),
            "recent": workspace.deferred().recent().iter().map(observation_json).collect::<Vec<_>>(),
            "retrieved": workspace.deferred().retrieved().iter().map(|handle| retrieval_json(handle, journal)).collect::<Vec<_>>(),
        }
    });
    if let Some(retrieval) = retrieval {
        structured["retrieval"] = json!({
            "schema": "numinous.remembered-room-retrieval",
            "schemaVersion": 1,
            "room": retrieval.room,
            "selection": "exact-current-room-subject-newest-first",
            "limit": retrieval.limit,
            "returned": retrieval.returned,
            "abstained": retrieval.returned == 0,
            "abstentionReason": (retrieval.returned == 0).then_some(
                "No current journal entry has this exact canonical room subject. Entry text and receipt digests were not searched."
            ),
        });
    }
    structured
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

fn retrieval_json(
    retrieval: &WorkspaceRetrieval,
    journal: Option<&numinous_core::Journal>,
) -> Value {
    let mut resolved = json!({
        "entry_id": retrieval.entry_id(),
        "reason": retrieval.reason(),
        "why_retrieved": retrieval.reason().unwrap_or(
            "Explicit journal entry handle selected by the player."
        ),
    });
    let Some(journal) = journal else {
        return resolved;
    };
    let Some(entry) = journal
        .entry(retrieval.entry_id())
        .filter(|entry| Some(record_digest(entry)) == retrieval.record_digest())
    else {
        resolved["status"] = json!("missing");
        resolved["entry"] = Value::Null;
        resolved["source_explanation"] = json!(
            "The record selected by this handle is unavailable or no longer matches. It may have been erased or replaced. Select a record explicitly to open it."
        );
        return resolved;
    };
    let superseding_entry_id = journal
        .superseding_entry(entry.entry_id)
        .map(|replacement| replacement.entry_id);
    resolved["status"] = json!(if superseding_entry_id.is_some() {
        "superseded"
    } else {
        "current"
    });
    resolved["superseded_by"] = json!(superseding_entry_id);
    resolved["entry"] = entry_json(journal, entry);
    resolved["source_explanation"] = json!(source_explanation(entry));
    resolved
}

fn source_explanation(entry: &numinous_core::JournalEntry) -> &'static str {
    match entry.source.as_str() {
        numinous_core::JOURNAL_SOURCE_SELF_AUTHORED => {
            "The journal caller declared this as its own account. Numinous did not verify the text."
        }
        numinous_core::JOURNAL_SOURCE_PLAYER_PROVIDED => {
            "The journal caller declared that a player supplied this account. Numinous did not verify the text."
        }
        numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT => {
            "This entry declares a Numinous result. The MCP record path assigns that source only after a live receipt replay, but the journal keeps the declaration and player interpretation, not the receipt body."
        }
        numinous_core::JOURNAL_SOURCE_LEGACY_IMPORT => {
            "This entry was migrated from the prototype journal. Its earlier provenance was not recorded."
        }
        _ => "The journal source token is not recognized by this binary.",
    }
}

fn workspace_prose(workspace: &SessionWorkspace) -> String {
    if workspace.is_empty() {
        return "Workspace is empty. It lives only in this MCP process: inspect, edit, retrieve, defer, or clear it here. It is not a memory, and it does not record plays unless you put them here. Exit or clear all to drop it. The journal is how a visit is kept.".to_string();
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use super::{ProcessWorkspace, compact_workspace_summary, workspace_tool};

    static NEXT_JOURNAL: AtomicU64 = AtomicU64::new(0);

    fn journal_path(label: &str) -> std::path::PathBuf {
        let serial = NEXT_JOURNAL.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "numinous_workspace_{label}_{}_{}.txt",
            std::process::id(),
            serial
        ))
    }

    struct IsolatedJournal {
        directory: std::path::PathBuf,
        path: std::path::PathBuf,
    }

    impl IsolatedJournal {
        fn new(label: &str) -> Self {
            let directory = journal_path(label).with_extension("d");
            std::fs::create_dir(&directory).expect("create isolated journal directory");
            Self {
                path: directory.join("journal.txt"),
                directory,
            }
        }
    }

    impl Drop for IsolatedJournal {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_dir(&self.directory);
        }
    }

    #[test]
    fn retrieval_resolves_current_sources_and_abstains_without_evidence() {
        let path = journal_path("retrieval");
        let first = numinous_core::record_journal_file(
            &path,
            numinous_core::JournalRecord {
                recorded_at_utc: 10,
                event_at_utc: 5,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "kepler-areas",
                text: "The areas looked equal.",
                affect: None,
            },
        )
        .expect("record room memory");
        let corrected = numinous_core::correct_journal_file(
            &path,
            20,
            None,
            numinous_core::JOURNAL_SOURCE_PLAYER_PROVIDED,
            first.entry_id,
            "The areas were equal while the speed changed.",
            None,
        )
        .expect("correct room memory");
        numinous_core::record_journal_file(
            &path,
            numinous_core::JournalRecord {
                recorded_at_utc: 30,
                event_at_utc: 30,
                source: numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT,
                kind: "encounter",
                subject: "receipt:opaque",
                text: "This text says kepler-laws but is not searched.",
                affect: None,
            },
        )
        .expect("record opaque receipt subject");

        let workspace = ProcessWorkspace::new();
        let found = workspace_tool(
            &json!({"op": "retrieve", "room": "kepler-laws", "limit": 1}),
            &workspace,
            &path,
        );
        assert_eq!(found["isError"], false);
        let structured = &found["structuredContent"];
        assert_eq!(structured["schemaVersion"], 2);
        assert_eq!(structured["retrieval"]["room"], "kepler-laws");
        assert_eq!(structured["retrieval"]["returned"], 1);
        assert_eq!(structured["retrieval"]["abstained"], false);
        assert_eq!(structured["retrieved"][0]["entry_id"], corrected.entry_id);
        assert_eq!(structured["retrieved"][0]["status"], "current");
        assert_eq!(
            structured["retrieved"][0]["entry"]["source"],
            numinous_core::JOURNAL_SOURCE_PLAYER_PROVIDED
        );
        assert!(
            structured["retrieved"][0]["source_explanation"]
                .as_str()
                .is_some_and(|text| text.contains("player supplied"))
        );
        assert!(
            compact_workspace_summary(structured)
                .is_some_and(|text| text.contains("Retrieved 1 current journal entry"))
        );

        let absent = workspace_tool(
            &json!({"op": "retrieve", "room": "mandelbrot"}),
            &workspace,
            &path,
        );
        let structured = &absent["structuredContent"];
        assert_eq!(structured["retrieval"]["abstained"], true);
        assert_eq!(structured["retrieval"]["returned"], 0);
        assert_eq!(structured["retrieved"], json!([]));
        assert!(
            structured["retrieval"]["abstentionReason"]
                .as_str()
                .is_some_and(
                    |text| text.contains("Entry text and receipt digests were not searched")
                )
        );

        std::fs::remove_file(path).expect("remove test journal");
    }

    #[test]
    fn explicit_handles_report_supersession_and_erasure() {
        let path = journal_path("handle-resolution");
        let original = numinous_core::record_journal_file(
            &path,
            numinous_core::JournalRecord {
                recorded_at_utc: 10,
                event_at_utc: 10,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "times-tables",
                text: "Nine loops.",
                affect: None,
            },
        )
        .expect("record original");
        let correction = numinous_core::correct_journal_file(
            &path,
            20,
            None,
            numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
            original.entry_id,
            "Ten loops.",
            None,
        )
        .expect("record correction");
        let workspace = ProcessWorkspace::new();
        let selected = workspace_tool(
            &json!({
                "op": "edit",
                "retrieved": [{"entry_id": original.entry_id}]
            }),
            &workspace,
            &path,
        );
        assert_eq!(
            selected["structuredContent"]["retrieved"][0]["status"],
            "superseded"
        );
        assert_eq!(
            selected["structuredContent"]["retrieved"][0]["superseded_by"],
            correction.entry_id
        );
        assert_eq!(
            selected["structuredContent"]["retrieved"][0]["why_retrieved"],
            "Explicit journal entry handle selected by the player."
        );

        std::fs::remove_file(&path).expect("erase test journal");
        let missing = workspace_tool(&json!({}), &workspace, &path);
        assert_eq!(
            missing["structuredContent"]["retrieved"][0]["status"],
            "missing"
        );
        assert!(missing["structuredContent"]["retrieved"][0]["entry"].is_null());
    }

    #[test]
    fn room_handles_do_not_follow_reused_journal_identifiers() {
        let isolated = IsolatedJournal::new("recreated-room-handle");
        let path = isolated.path.clone();
        let original = numinous_core::JournalRecord {
            recorded_at_utc: 10,
            event_at_utc: 10,
            source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
            kind: "encounter",
            subject: "times-tables",
            text: "The original chosen encounter.",
            affect: None,
        };
        let first =
            numinous_core::record_journal_file(&path, original).expect("record original encounter");
        let workspace = ProcessWorkspace::new();
        let selected = workspace_tool(
            &json!({"op": "retrieve", "room": "times-tables"}),
            &workspace,
            &path,
        );
        assert_eq!(
            selected["structuredContent"]["retrieved"][0]["status"],
            "current"
        );
        numinous_core::erase_journal_file(&path).expect("erase journal");
        let replacement = numinous_core::record_journal_file(
            &path,
            numinous_core::JournalRecord {
                text: "A different encounter in the same room and second.",
                ..original
            },
        )
        .expect("recreate journal");
        assert_eq!(first.entry_id, replacement.entry_id);

        let inspected = workspace_tool(&json!({}), &workspace, &path);
        let handle = &inspected["structuredContent"]["retrieved"][0];
        assert_eq!(handle["status"], "missing");
        assert!(handle["entry"].is_null());
        assert!(!inspected.to_string().contains("different encounter"));
    }

    #[test]
    fn deferred_handle_keeps_its_selection_when_active_id_is_reselected() {
        let isolated = IsolatedJournal::new("recreated-deferred-handle");
        let path = isolated.path.clone();
        let original = numinous_core::JournalRecord {
            recorded_at_utc: 10,
            event_at_utc: 10,
            source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
            kind: "encounter",
            subject: "times-tables",
            text: "An encounter selected before erasure.",
            affect: None,
        };
        let first =
            numinous_core::record_journal_file(&path, original).expect("record original encounter");
        let workspace = ProcessWorkspace::new();
        let selection = json!({
            "op": "edit", "retrieved": [{"entry_id": first.entry_id}]
        });
        workspace_tool(&selection, &workspace, &path);
        workspace_tool(
            &json!({"op": "defer", "field": "retrieved"}),
            &workspace,
            &path,
        );
        numinous_core::erase_journal_file(&path).expect("erase journal");
        numinous_core::record_journal_file(
            &path,
            numinous_core::JournalRecord {
                subject: "mandelbrot",
                text: "An encounter deliberately selected after erasure.",
                ..original
            },
        )
        .expect("recreate journal");

        let reselected = workspace_tool(&selection, &workspace, &path);
        let structured = &reselected["structuredContent"];
        assert_eq!(structured["retrieved"][0]["status"], "current");
        assert_eq!(structured["retrieved"][0]["entry"]["subject"], "mandelbrot");
        assert_eq!(structured["deferred"]["retrieved"][0]["status"], "missing");
        assert!(structured["deferred"]["retrieved"][0]["entry"].is_null());
    }

    #[test]
    fn unresolved_handle_needs_a_new_selection_before_opening_a_later_record() {
        let isolated = IsolatedJournal::new("unresolved-handle");
        let path = isolated.path.clone();
        let workspace = ProcessWorkspace::new();
        let selection = json!({"op": "edit", "retrieved": [{"entry_id": 1}]});
        let selected = workspace_tool(&selection, &workspace, &path);
        assert_eq!(
            selected["structuredContent"]["retrieved"][0]["status"],
            "missing"
        );
        numinous_core::record_journal_file(
            &path,
            numinous_core::JournalRecord {
                recorded_at_utc: 10,
                event_at_utc: 10,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "times-tables",
                text: "An encounter that did not exist when the id was selected.",
                affect: None,
            },
        )
        .expect("record later encounter");

        let inspected = workspace_tool(&json!({}), &workspace, &path);
        assert_eq!(
            inspected["structuredContent"]["retrieved"][0]["status"],
            "missing"
        );
        assert!(inspected["structuredContent"]["retrieved"][0]["entry"].is_null());
        let reselected = workspace_tool(&selection, &workspace, &path);
        assert_eq!(
            reselected["structuredContent"]["retrieved"][0]["status"],
            "current"
        );
    }

    #[test]
    fn empty_retrieved_edit_clears_handles_with_malformed_journal() {
        let isolated = IsolatedJournal::new("clear-malformed-journal");
        let path = &isolated.path;
        let original = numinous_core::record_journal_file(
            path,
            numinous_core::JournalRecord {
                recorded_at_utc: 10,
                event_at_utc: 10,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "times-tables",
                text: "An encounter selected while the journal was readable.",
                affect: None,
            },
        )
        .expect("record encounter");
        let workspace = ProcessWorkspace::new();
        let selected = workspace_tool(
            &json!({"op": "edit", "retrieved": [{"entry_id": original.entry_id}]}),
            &workspace,
            path,
        );
        assert_eq!(
            selected["structuredContent"]["retrieved"][0]["status"],
            "current"
        );
        let malformed = "numinous-journal-v3\nnot a valid journal record\n";
        std::fs::write(path, malformed).expect("write malformed journal");

        let cleared = workspace_tool(&json!({"op": "edit", "retrieved": []}), &workspace, path);
        assert_eq!(cleared["isError"], false);
        assert_eq!(cleared["structuredContent"]["retrieved"], json!([]));
        assert_eq!(cleared["structuredContent"]["empty"], true);
        assert_eq!(
            std::fs::read_to_string(path).expect("read unchanged journal"),
            malformed
        );
    }
}
