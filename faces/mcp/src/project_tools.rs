//! MCP projection for one explicit project chain.
//!
//! Core owns the document, the chain, and the resume preview. This module
//! parses the closed tool arguments, stores the chain at the face path, and
//! returns the preview. Resume does not call the next tool.

use std::path::Path;

use numinous_core::{
    CreationFact, CreationStatus, EncounterTool, EvidenceFact, EvidenceStatus,
    MAX_PROJECT_EVIDENCE, MAX_PROJECT_FILE_BYTES, MAX_PROJECT_ROOMS, MAX_SHARE_INPUT_BYTES,
    MAX_WORKSPACE_TEXT_CHARS, NextPreview, PROJECT_RESUME_PREVIEW_SCHEMA,
    PROJECT_RESUME_PREVIEW_VERSION, ProjectArgument, ProjectArgumentValue, ProjectDraft,
    ProjectEvidence, ProjectNext, ProjectStore, ReceiptCheck, ResumePreview, RoomStatus,
};
use serde_json::{Map, Value, json};

use super::{journal, tool_error, tool_structured};

/// Keep, import, correct, or preview the explicit project chain.
pub(super) fn project_tool(
    args: &Value,
    path: &Path,
    journal_path: &Path,
    replay: impl Fn(EncounterTool, &Value) -> Value,
) -> Value {
    let op = args.get("op").and_then(Value::as_str).unwrap_or("");
    match op {
        "keep" => keep(args, path),
        "import" => import(args, path),
        "correct" => correct(args, path),
        "resume" => resume(args, path, journal_path, &replay),
        _ => tool_error("op must be keep, import, correct, or resume."),
    }
}

pub(super) fn catalog_entry() -> Value {
    json!({
        "name": "project",
        "description": "Keep, import, correct, or preview one explicit project. op keep stores a question as data, one closed next call, one to four catalog rooms, up to four journal or receipt digests, and an optional Studio creation. op import reads one NUMINOUS_PROJECT 1 document and appends only when confirm is true. op correct appends a new revision and leaves the named revision in place. op resume previews what is present, missing, corrected, collided, or incompatible. structuredContent.preview.next is a tool call you may follow; resume does not apply it, does not change the workspace, and does not copy journal text into the chain. The chain file is NUMINOUS_PROJECT, or .numinous-project when that variable is unset. This is not portable-1, and portable-1 does not import a project.",
        "inputSchema": input_schema(),
        "outputSchema": output_schema()
    })
}

fn keep(args: &Value, path: &Path) -> Value {
    if let Err(message) = allow_only(args, &["question", "next", "rooms", "evidence", "creation"]) {
        return tool_error(&message);
    }
    let draft = match draft_from_args(args) {
        Ok(draft) => draft,
        Err(message) => return tool_error(&message),
    };
    match numinous_core::keep_project_file(path, &draft) {
        Ok(outcome) => store_result("keep", path, &outcome),
        Err(error) => tool_error(&format!("Failed to keep project: {error}")),
    }
}

fn import(args: &Value, path: &Path) -> Value {
    if let Err(message) = allow_only(args, &["document", "confirm", "origin"]) {
        return tool_error(&message);
    }
    let Some(document) = args.get("document").and_then(Value::as_str) else {
        return tool_error("Missing required string argument 'document'.");
    };
    let confirm = args
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let origin = match optional_revision(args, "origin") {
        Ok(origin) => origin,
        Err(message) => return tool_error(&message),
    };
    match numinous_core::import_project_file(path, document, now(), origin, confirm) {
        Ok(outcome) => store_result("import", path, &outcome),
        Err(error) => tool_error(&format!("Failed to import project: {error}")),
    }
}

fn correct(args: &Value, path: &Path) -> Value {
    if let Err(message) = allow_only(
        args,
        &[
            "revision", "question", "next", "rooms", "evidence", "creation",
        ],
    ) {
        return tool_error(&message);
    }
    let Some(revision) = args.get("revision").and_then(Value::as_u64) else {
        return tool_error("Missing required integer argument 'revision'.");
    };
    if revision == 0 {
        return tool_error("revision must be at least 1.");
    }
    let draft = match draft_from_args(args) {
        Ok(draft) => draft,
        Err(message) => return tool_error(&message),
    };
    match numinous_core::correct_project_file(path, revision, &draft) {
        Ok(outcome) => store_result("correct", path, &outcome),
        Err(error) => tool_error(&format!("Failed to correct project: {error}")),
    }
}

fn resume(
    args: &Value,
    path: &Path,
    journal_path: &Path,
    replay: &impl Fn(EncounterTool, &Value) -> Value,
) -> Value {
    if let Err(message) = allow_only(args, &["revision", "receipt"]) {
        return tool_error(&message);
    }
    let revision = match optional_revision(args, "revision") {
        Ok(revision) => revision,
        Err(message) => return tool_error(&message),
    };
    let chain = match numinous_core::try_load_project_file(path) {
        Ok(chain) => chain,
        Err(error) => return tool_error(&format!("Failed to read project: {error}")),
    };
    let journal = match numinous_core::try_load_journal_file(journal_path) {
        Ok(journal) => journal,
        Err(error) => return tool_error(&format!("Failed to read journal: {error}")),
    };
    let receipt = match receipt_check(args, replay) {
        Ok(receipt) => receipt,
        Err(message) => return tool_error(&message),
    };
    let preview = match chain.preview(revision, &journal, receipt) {
        Ok(preview) => preview,
        Err(error) => return tool_error(&format!("Failed to preview project: {error}")),
    };
    let Some(stored) = chain.revision(preview.revision_id) else {
        return tool_error("Failed to preview project: the revision is not in the chain.");
    };
    let preview_json = match preview_json(&preview) {
        Ok(preview_json) => preview_json,
        Err(message) => return tool_error(&message),
    };
    let text = preview_text(&preview);
    tool_structured(
        &text,
        envelope(
            "resume",
            false,
            "preview",
            Some(preview.revision_id),
            Some(stored.identity_hex()),
            Some(stored.to_document()),
            Some(preview_json),
        ),
    )
}

fn store_result(action: &str, path: &Path, outcome: &ProjectStore) -> Value {
    let written = matches!(outcome, ProjectStore::Appended { .. });
    let revision_id = match outcome {
        ProjectStore::Appended { revision_id, .. }
        | ProjectStore::AlreadyPresent { revision_id, .. }
        | ProjectStore::Collided {
            origin_revision: revision_id,
        } => Some(*revision_id),
        ProjectStore::NeedsConfirm => None,
    };
    let (identity, document) = if let Some(revision_id) = revision_id.filter(|_| {
        matches!(
            outcome,
            ProjectStore::Appended { .. } | ProjectStore::AlreadyPresent { .. }
        )
    }) {
        match stored_document(path, revision_id) {
            Ok(pair) => (Some(pair.0), Some(pair.1)),
            Err(message) => return tool_error(&message),
        }
    } else {
        (None, None)
    };
    let name = match outcome {
        ProjectStore::Appended { .. } => "appended",
        ProjectStore::AlreadyPresent { .. } => "already_present",
        ProjectStore::Collided { .. } => "collided",
        ProjectStore::NeedsConfirm => "needs_confirm",
    };
    tool_structured(
        &store_text(action, outcome),
        envelope(action, written, name, revision_id, identity, document, None),
    )
}

fn stored_document(path: &Path, revision_id: u64) -> Result<(String, String), String> {
    let chain = numinous_core::try_load_project_file(path)
        .map_err(|error| format!("Failed to read project: {error}"))?;
    let revision = chain.revision(revision_id).ok_or_else(|| {
        format!("Failed to read project: revision {revision_id} is not in the chain.")
    })?;
    Ok((revision.identity_hex(), revision.to_document()))
}

fn envelope(
    action: &str,
    written: bool,
    outcome: &str,
    revision_id: Option<u64>,
    identity_hex: Option<String>,
    document: Option<String>,
    preview: Option<Value>,
) -> Value {
    json!({
        "action": action,
        "applied": false,
        "written": written,
        "outcome": outcome,
        "revisionId": revision_id,
        "identityHex": identity_hex,
        "document": document,
        "preview": preview,
    })
}

fn store_text(action: &str, outcome: &ProjectStore) -> String {
    match outcome {
        ProjectStore::Appended { revision_id, .. } if action == "correct" => format!(
            "Correction {revision_id} saved. The corrected revision remains as it was. Resume does not apply the project."
        ),
        ProjectStore::Appended { revision_id, .. } => format!(
            "Project revision {revision_id} saved. The question is stored as data. Resume can preview the next call, and this call did not apply it."
        ),
        ProjectStore::AlreadyPresent { revision_id, .. } => format!(
            "These project bytes are already revision {revision_id}. Nothing was written."
        ),
        ProjectStore::Collided { origin_revision } => format!(
            "Origin revision {origin_revision} already holds different bytes. Nothing was written. Pass confirm true to append a new local revision without replacing that origin."
        ),
        ProjectStore::NeedsConfirm => "Import would append a new revision. Nothing was written. Pass confirm true to keep this import.".to_string(),
    }
}

fn preview_text(preview: &ResumePreview) -> String {
    let next = match &preview.next {
        NextPreview::Ready(call) => format!("Next: {}.", call.tool),
        NextPreview::Incompatible(incompatible) => {
            format!(
                "Next: {} is incompatible: {}.",
                incompatible.tool, incompatible.reason
            )
        }
    };
    let evidence = if preview.evidence.is_empty() {
        "Evidence: none.".to_string()
    } else {
        let parts = preview
            .evidence
            .iter()
            .map(|fact| format!("{} {}", fact.kind, evidence_status_name(fact.status)))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Evidence: {parts}.")
    };
    format!(
        "Project revision {}. Question: {}. {next} {evidence} This preview does not apply the next call, and it does not write the journal or the workspace.",
        preview.revision_id, preview.question
    )
}

fn preview_json(preview: &ResumePreview) -> Result<Value, String> {
    let (status, tool, arguments, reason) = match &preview.next {
        NextPreview::Ready(call) => ("ready", call.tool, arguments_json(&call.arguments)?, None),
        NextPreview::Incompatible(incompatible) => (
            "incompatible",
            incompatible.tool.as_str(),
            json!({}),
            Some(incompatible.reason.as_str()),
        ),
    };
    Ok(json!({
        "schema": preview.schema,
        "schemaVersion": preview.version,
        "revisionId": preview.revision_id,
        "question": preview.question,
        "interpreted": preview.interpreted,
        "next": {
            "status": status,
            "tool": tool,
            "arguments": arguments,
            "reason": reason,
        },
        "rooms": preview.rooms.iter().map(|room| json!({
            "id": room.id,
            "status": room_status_name(room.status),
        })).collect::<Vec<_>>(),
        "evidence": preview.evidence.iter().map(evidence_json).collect::<Vec<_>>(),
        "creation": creation_json(&preview.creation),
        "willReturn": preview.will_return,
        "notApplied": preview.not_applied,
        "workspaceChanged": preview.workspace_changed,
        "journalChanged": preview.journal_changed,
        "supersededBy": preview.superseded_by,
        "parentResolved": preview.parent_resolved,
    }))
}

fn arguments_json(arguments: &[ProjectArgument]) -> Result<Value, String> {
    let mut object = Map::new();
    for argument in arguments {
        object.insert(argument.name.to_string(), argument_value(&argument.value)?);
    }
    Ok(Value::Object(object))
}

fn argument_value(value: &ProjectArgumentValue) -> Result<Value, String> {
    match value {
        ProjectArgumentValue::Text(text) => Ok(Value::String(text.clone())),
        ProjectArgumentValue::Number(token) => {
            let number: f64 = token
                .parse()
                .map_err(|_| "The previewed phase is not a JSON number.".to_string())?;
            let number = serde_json::Number::from_f64(number)
                .ok_or_else(|| "The previewed phase is not a JSON number.".to_string())?;
            Ok(Value::Number(number))
        }
    }
}

fn evidence_json(fact: &EvidenceFact) -> Value {
    json!({
        "kind": fact.kind,
        "digestHex": fact.digest_hex,
        "status": evidence_status_name(fact.status),
        "entryId": fact.entry_id,
        "supersededBy": fact.superseded_by,
        "subject": fact.subject,
        "text": fact.text,
        "tool": fact.tool,
        "sameBytesElsewhere": fact.same_bytes_elsewhere,
    })
}

fn creation_json(creation: &CreationFact) -> Value {
    json!({
        "status": creation_status_name(creation.status),
        "descends": creation.descends,
        "lineageWasNotInTheLink": creation.lineage_was_not_in_the_link,
        "periodText": creation.period_text,
    })
}

fn room_status_name(status: RoomStatus) -> &'static str {
    match status {
        RoomStatus::Present => "present",
        RoomStatus::Missing => "missing",
        RoomStatus::Incompatible => "incompatible",
    }
}

fn evidence_status_name(status: EvidenceStatus) -> &'static str {
    match status {
        EvidenceStatus::Present => "present",
        EvidenceStatus::Missing => "missing",
        EvidenceStatus::Corrected => "corrected",
        EvidenceStatus::Collided => "collided",
        EvidenceStatus::Incompatible => "incompatible",
    }
}

fn creation_status_name(status: CreationStatus) -> &'static str {
    match status {
        CreationStatus::Missing => "missing",
        CreationStatus::Present => "present",
        CreationStatus::Incompatible => "incompatible",
    }
}

fn draft_from_args(args: &Value) -> Result<ProjectDraft, String> {
    let Some(question) = args.get("question").and_then(Value::as_str) else {
        return Err("Missing required string argument 'question'.".to_string());
    };
    let Some(next) = args.get("next") else {
        return Err("Missing required object argument 'next'.".to_string());
    };
    let Some(rooms) = args.get("rooms") else {
        return Err("Missing required array argument 'rooms'.".to_string());
    };
    Ok(ProjectDraft {
        recorded_at_utc: now(),
        question: question.to_string(),
        next: parse_next(next)?,
        rooms: parse_rooms(rooms)?,
        evidence: parse_evidence(args.get("evidence"))?,
        creation: parse_creation(args)?,
    })
}

fn parse_next(value: &Value) -> Result<ProjectNext, String> {
    let tool = value.get("tool").and_then(Value::as_str).unwrap_or("");
    let Some(arguments) = value.get("arguments").filter(|value| value.is_object()) else {
        return Err("next.arguments must be an object.".to_string());
    };
    match tool {
        "open_creation" => Ok(ProjectNext::OpenCreation {
            capsule: required_string(arguments, "capsule")?,
        }),
        "fork_creation" => Ok(ProjectNext::ForkCreation),
        "play_room" => Ok(ProjectNext::PlayRoom {
            room: required_string(arguments, "id")?,
            phase: match arguments.get("t") {
                None => None,
                Some(phase) => Some(phase_token(phase)?),
            },
        }),
        "study_room" => Ok(ProjectNext::StudyRoom {
            room: required_string(arguments, "room")?,
        }),
        _ => Err(
            "next.tool must be open_creation, fork_creation, play_room, or study_room.".to_string(),
        ),
    }
}

fn phase_token(value: &Value) -> Result<String, String> {
    let Some(phase) = value.as_f64() else {
        return Err("next.arguments.t must be a finite number in [0, 1).".to_string());
    };
    if !phase.is_finite() || !(0.0..1.0).contains(&phase) {
        return Err("next.arguments.t must be a finite number in [0, 1).".to_string());
    }
    Ok(phase.to_string())
}

fn parse_rooms(value: &Value) -> Result<Vec<String>, String> {
    let Some(rooms) = value.as_array() else {
        return Err("rooms must be an array of catalog room ids.".to_string());
    };
    let mut parsed = Vec::with_capacity(rooms.len());
    for room in rooms {
        let Some(id) = room.as_str() else {
            return Err("rooms must be an array of catalog room ids.".to_string());
        };
        parsed.push(id.to_string());
    }
    Ok(parsed)
}

fn parse_evidence(value: Option<&Value>) -> Result<Vec<ProjectEvidence>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let Some(items) = value.as_array() else {
        return Err("evidence must be an array of typed links.".to_string());
    };
    let mut evidence = Vec::with_capacity(items.len());
    for item in items {
        evidence.push(parse_evidence_item(item)?);
    }
    Ok(evidence)
}

fn parse_evidence_item(value: &Value) -> Result<ProjectEvidence, String> {
    let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
    let Some(digest_text) = value.get("digest").and_then(Value::as_str) else {
        return Err("evidence.digest must be 64 hexadecimal characters.".to_string());
    };
    let digest = decode_digest(digest_text)?;
    match kind {
        "journal" => {
            if value.get("tool").is_some() {
                return Err("journal evidence does not name a tool.".to_string());
            }
            let entry_id = match value.get("entry_id") {
                None => None,
                Some(id) => Some(required_entry_id(id)?),
            };
            Ok(ProjectEvidence::Journal { digest, entry_id })
        }
        "receipt" => {
            if value.get("entry_id").is_some() {
                return Err("receipt evidence does not name an entry_id.".to_string());
            }
            let tool = match value.get("tool").and_then(Value::as_str) {
                None => None,
                Some(name) => Some(EncounterTool::from_name(name).ok_or_else(|| {
                    "evidence.tool must be play_room, listen_room, or sing_expression.".to_string()
                })?),
            };
            Ok(ProjectEvidence::Receipt { digest, tool })
        }
        _ => Err("evidence.kind must be journal or receipt.".to_string()),
    }
}

fn required_entry_id(value: &Value) -> Result<u64, String> {
    match value.as_u64() {
        Some(id) if id >= 1 => Ok(id),
        _ => Err("evidence.entry_id must be an integer of at least 1.".to_string()),
    }
}

fn parse_creation(args: &Value) -> Result<Option<String>, String> {
    match args.get("creation") {
        None => Ok(None),
        Some(Value::String(text)) if !text.is_empty() => Ok(Some(text.clone())),
        Some(_) => Err(
            "creation must be Studio .num text, a native link, or a bundled experiment id."
                .to_string(),
        ),
    }
}

fn required_string(arguments: &Value, name: &str) -> Result<String, String> {
    match arguments.get(name).and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Ok(text.to_string()),
        _ => Err(format!("Missing required string argument '{name}'.")),
    }
}

fn optional_revision(args: &Value, name: &str) -> Result<Option<u64>, String> {
    match args.get(name) {
        None => Ok(None),
        Some(value) => match value.as_u64() {
            Some(id) if id >= 1 => Ok(Some(id)),
            _ => Err(format!("{name} must be an integer of at least 1.")),
        },
    }
}

fn allow_only(args: &Value, allowed: &[&str]) -> Result<(), String> {
    let Some(object) = args.as_object() else {
        return Err("Arguments must be an object.".to_string());
    };
    if let Some(key) = object
        .keys()
        .find(|key| *key != "op" && !allowed.contains(&key.as_str()))
    {
        return Err(format!(
            "Argument '{key}' is not used by this project operation."
        ));
    }
    Ok(())
}

fn receipt_check(
    args: &Value,
    replay: &impl Fn(EncounterTool, &Value) -> Value,
) -> Result<ReceiptCheck, String> {
    let Some(receipt) = args.get("receipt") else {
        return Ok(ReceiptCheck::NotSupplied);
    };
    match journal::verify_receipt(receipt, replay) {
        Ok(digest) => Ok(ReceiptCheck::Verified(decode_digest(&digest)?)),
        Err(message) if message.starts_with("This receipt does not match a live replay") => {
            Ok(ReceiptCheck::Disagreed)
        }
        Err(message) => Err(message),
    }
}

fn decode_digest(text: &str) -> Result<[u8; 32], String> {
    if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("digest must be 64 hexadecimal characters.".to_string());
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .map_err(|_| "digest must be 64 hexadecimal characters.".to_string())?;
    }
    Ok(digest)
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn input_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "op": {
                "type": "string",
                "enum": ["keep", "import", "correct", "resume"],
                "description": "keep stores a new project. import reads one portable document. correct appends a revision. resume previews and does not apply."
            },
            "question": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_WORKSPACE_TEXT_CHARS,
                "description": "The question, stored as data. It is never dispatched as a tool call."
            },
            "next": {
                "type": "object",
                "description": "The one next call. Follow it only by a later, separate tool call.",
                "properties": {
                    "tool": {
                        "type": "string",
                        "enum": ["open_creation", "fork_creation", "play_room", "study_room"]
                    },
                    "arguments": {
                        "type": "object",
                        "properties": {
                            "capsule": {
                                "type": "string",
                                "minLength": 1,
                                "maxLength": MAX_SHARE_INPUT_BYTES,
                                "description": "open_creation capsule: .num text, a native link, or a bundled experiment id."
                            },
                            "id": {
                                "type": "string",
                                "minLength": 1,
                                "maxLength": super::MAX_TOOL_ID_CHARS,
                                "description": "play_room room id. The room must also be named in rooms."
                            },
                            "t": {
                                "type": "number",
                                "minimum": 0,
                                "exclusiveMaximum": 1,
                                "description": "Optional play_room phase in [0, 1)."
                            },
                            "room": {
                                "type": "string",
                                "minLength": 1,
                                "maxLength": super::MAX_TOOL_ID_CHARS,
                                "description": "study_room room id. The room must also be named in rooms."
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["tool", "arguments"],
                "additionalProperties": false
            },
            "rooms": {
                "type": "array",
                "minItems": 1,
                "maxItems": MAX_PROJECT_ROOMS,
                "items": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": super::MAX_TOOL_ID_CHARS
                },
                "description": "One to four catalog room ids. Aliases are resolved while keeping."
            },
            "evidence": {
                "type": "array",
                "maxItems": MAX_PROJECT_EVIDENCE,
                "items": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["journal", "receipt"] },
                        "digest": {
                            "type": "string",
                            "minLength": 64,
                            "maxLength": 64,
                            "description": "Lowercase or uppercase SHA-256 hex. Journal links use the entry identity digest. Receipt links use the result digest."
                        },
                        "entry_id": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Optional journal-local id. A receipt link does not take one."
                        },
                        "tool": {
                            "type": "string",
                            "enum": ["play_room", "listen_room", "sing_expression"],
                            "description": "Optional receipt tool name. A journal link does not take one."
                        }
                    },
                    "required": ["kind", "digest"],
                    "additionalProperties": false
                },
                "description": "Up to four typed links. The project stores the digest, not the journal text."
            },
            "creation": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_SHARE_INPUT_BYTES,
                "description": "Optional Studio .num text, native link, or bundled experiment id. A filesystem path is refused."
            },
            "document": {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_PROJECT_FILE_BYTES,
                "description": "import only: one NUMINOUS_PROJECT 1 document."
            },
            "confirm": {
                "type": "boolean",
                "description": "import only. true appends a new revision. The same payload bytes are already present either way."
            },
            "origin": {
                "type": "integer",
                "minimum": 1,
                "description": "import only: caller's previous local revision id. A different payload at that id collides until confirm appends a new id."
            },
            "revision": {
                "type": "integer",
                "minimum": 1,
                "description": "correct: the current revision to supersede. resume: the revision to preview. Omit on resume to preview the latest."
            },
            "receipt": {
                "type": "object",
                "description": "resume only: structuredContent.encounter. A live mismatch still returns the preview and marks the receipt link incompatible. Resume does not apply the next call."
            }
        },
        "required": ["op"],
        "additionalProperties": false
    })
}

fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": { "type": "string", "enum": ["keep", "import", "correct", "resume"] },
            "applied": { "type": "boolean" },
            "written": { "type": "boolean" },
            "outcome": {
                "type": "string",
                "enum": ["appended", "already_present", "collided", "needs_confirm", "preview"]
            },
            "revisionId": nullable(json!({ "type": "integer", "minimum": 1 })),
            "identityHex": nullable(json!({ "type": "string" })),
            "document": nullable(json!({ "type": "string" })),
            "preview": nullable(preview_schema())
        },
        "required": ["action", "applied", "written", "outcome", "revisionId", "identityHex", "document", "preview"],
        "additionalProperties": false
    })
}

fn preview_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "schema": { "type": "string", "enum": [PROJECT_RESUME_PREVIEW_SCHEMA] },
            "schemaVersion": { "type": "integer", "minimum": PROJECT_RESUME_PREVIEW_VERSION },
            "revisionId": { "type": "integer", "minimum": 1 },
            "question": { "type": "string" },
            "interpreted": { "type": "boolean" },
            "next": {
                "type": "object",
                "properties": {
                    "status": { "type": "string", "enum": ["ready", "incompatible"] },
                    "tool": { "type": "string" },
                    "arguments": {
                        "type": "object",
                        "properties": {
                            "capsule": { "type": "string" },
                            "parent": { "type": "string" },
                            "id": { "type": "string" },
                            "room": { "type": "string" },
                            "t": { "type": "number" }
                        },
                        "additionalProperties": false
                    },
                    "reason": nullable(json!({ "type": "string" }))
                },
                "required": ["status", "tool", "arguments", "reason"],
                "additionalProperties": false
            },
            "rooms": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "status": { "type": "string", "enum": ["present", "missing", "incompatible"] }
                    },
                    "required": ["id", "status"],
                    "additionalProperties": false
                }
            },
            "evidence": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["journal", "receipt"] },
                        "digestHex": { "type": "string" },
                        "status": { "type": "string", "enum": ["present", "missing", "corrected", "collided", "incompatible"] },
                        "entryId": nullable(json!({ "type": "integer", "minimum": 1 })),
                        "supersededBy": nullable(json!({ "type": "integer", "minimum": 1 })),
                        "subject": nullable(json!({ "type": "string" })),
                        "text": nullable(json!({ "type": "string" })),
                        "tool": nullable(json!({ "type": "string" })),
                        "sameBytesElsewhere": { "type": "boolean" }
                    },
                    "required": ["kind", "digestHex", "status", "entryId", "supersededBy", "subject", "text", "tool", "sameBytesElsewhere"],
                    "additionalProperties": false
                }
            },
            "creation": {
                "type": "object",
                "properties": {
                    "status": { "type": "string", "enum": ["missing", "present", "incompatible"] },
                    "descends": nullable(json!({ "type": "string" })),
                    "lineageWasNotInTheLink": { "type": "boolean" },
                    "periodText": nullable(json!({ "type": "string" }))
                },
                "required": ["status", "descends", "lineageWasNotInTheLink", "periodText"],
                "additionalProperties": false
            },
            "willReturn": { "type": "boolean" },
            "notApplied": { "type": "boolean" },
            "workspaceChanged": { "type": "boolean" },
            "journalChanged": { "type": "boolean" },
            "supersededBy": nullable(json!({ "type": "integer", "minimum": 1 })),
            "parentResolved": nullable(json!({ "type": "boolean" }))
        },
        "required": [
            "schema", "schemaVersion", "revisionId", "question", "interpreted", "next",
            "rooms", "evidence", "creation", "willReturn", "notApplied",
            "workspaceChanged", "journalChanged", "supersededBy", "parentResolved"
        ],
        "additionalProperties": false
    })
}

fn nullable(schema: Value) -> Value {
    json!({ "oneOf": [schema, { "type": "null" }] })
}

#[cfg(test)]
mod tests {
    use super::project_tool;
    use numinous_core::JournalRecord;
    use serde_json::json;

    struct Isolated {
        root: std::path::PathBuf,
    }

    impl Isolated {
        fn new(label: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "numinous-mcp-project-{label}-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir(&root).expect("private project directory");
            Self { root }
        }

        fn project(&self) -> std::path::PathBuf {
            self.root.join("project.txt")
        }

        fn journal(&self) -> std::path::PathBuf {
            self.root.join("journal.txt")
        }
    }

    impl Drop for Isolated {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn call(root: &Isolated, args: serde_json::Value) -> serde_json::Value {
        project_tool(&args, &root.project(), &root.journal(), |_, _| json!({}))
    }

    fn keep_closing(root: &Isolated) -> serde_json::Value {
        call(
            root,
            json!({
                "op": "keep",
                "question": "What period do these two oscillators share?",
                "next": {"tool": "open_creation", "arguments": {"capsule": "closing-voices"}},
                "rooms": ["lissajous"]
            }),
        )
    }

    #[test]
    fn resume_next_opens_closing_voices_without_touching_the_journal() {
        let root = Isolated::new("follow");
        let journal = root.journal();
        numinous_core::record_journal_file(
            &journal,
            JournalRecord {
                recorded_at_utc: 10,
                event_at_utc: 10,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "lissajous",
                text: "quartz-journal-token",
                affect: None,
            },
        )
        .expect("journal fixture");
        let before = std::fs::read(&journal).expect("journal bytes");
        let kept = keep_closing(&root);
        assert_eq!(kept["isError"], false, "{kept}");
        assert_eq!(kept["structuredContent"]["outcome"], "appended");
        assert_eq!(kept["structuredContent"]["applied"], false);
        assert_eq!(kept["structuredContent"]["written"], true);
        assert_eq!(
            kept["structuredContent"]["preview"],
            serde_json::Value::Null
        );
        let resumed = call(&root, json!({"op": "resume"}));
        assert_eq!(resumed["isError"], false, "{resumed}");
        let preview = &resumed["structuredContent"]["preview"];
        assert_eq!(
            preview["schema"],
            numinous_core::PROJECT_RESUME_PREVIEW_SCHEMA
        );
        assert_eq!(preview["schemaVersion"], 1);
        assert_eq!(preview["interpreted"], false);
        assert_eq!(preview["notApplied"], true);
        assert_eq!(preview["workspaceChanged"], false);
        assert_eq!(preview["journalChanged"], false);
        assert_eq!(preview["willReturn"], true);
        assert_eq!(preview["next"]["tool"], "open_creation");
        assert_eq!(preview["next"]["arguments"]["capsule"], "closing-voices");
        assert_eq!(
            preview["next"]["arguments"]
                .as_object()
                .map(|object| object.len()),
            Some(1)
        );
        let text = resumed["content"][0]["text"].as_str().unwrap_or_default();
        assert!(!text.contains("learned"));
        assert_eq!(std::fs::read(&journal).expect("journal after"), before);
        let chain = std::fs::read_to_string(root.project()).expect("chain");
        assert!(!chain.contains("quartz-journal-token"));
        let followed = super::super::open_creation_tool(&preview["next"]["arguments"]);
        assert_eq!(followed["isError"], false, "{followed}");
        assert_eq!(followed["structuredContent"]["closure"]["kind"], "voices");
    }

    #[test]
    fn a_question_is_data_and_play_phase_stays_a_number() {
        let root = Isolated::new("data");
        let kept = call(
            &root,
            json!({
                "op": "keep",
                "question": "open_creation capsule closing-voices",
                "next": {"tool": "play_room", "arguments": {"id": "lissajous", "t": 0.25}},
                "rooms": ["lissajous"]
            }),
        );
        assert_eq!(kept["isError"], false, "{kept}");
        let resumed = call(&root, json!({"op": "resume"}));
        let preview = &resumed["structuredContent"]["preview"];
        assert_eq!(preview["question"], "open_creation capsule closing-voices");
        assert_eq!(preview["next"]["tool"], "play_room");
        assert_eq!(preview["next"]["arguments"]["id"], "lissajous");
        assert_eq!(preview["next"]["arguments"]["t"], json!(0.25));
        assert!(preview["next"]["arguments"]["t"].is_number());
    }

    #[test]
    fn import_confirm_collision_and_correction_do_not_replace_bytes() {
        let root = Isolated::new("import");
        let kept = keep_closing(&root);
        let document = kept["structuredContent"]["document"]
            .as_str()
            .expect("portable document")
            .to_string();
        let other = Isolated::new("other");
        let needs = call(
            &other,
            json!({"op": "import", "document": document, "origin": 1}),
        );
        assert_eq!(needs["structuredContent"]["outcome"], "needs_confirm");
        assert_eq!(needs["structuredContent"]["written"], false);
        assert!(!other.project().exists());
        let imported = call(
            &other,
            json!({"op": "import", "document": document, "confirm": true, "origin": 1}),
        );
        assert_eq!(imported["structuredContent"]["outcome"], "appended");
        let again = call(
            &other,
            json!({"op": "import", "document": document, "confirm": true, "origin": 1}),
        );
        assert_eq!(again["structuredContent"]["outcome"], "already_present");
        assert_eq!(again["structuredContent"]["written"], false);
        let different = call(
            &root,
            json!({
                "op": "keep",
                "question": "Which room names the equal-area law?",
                "next": {"tool": "study_room", "arguments": {"room": "kepler-laws"}},
                "rooms": ["kepler-laws"]
            }),
        );
        let different_document = different["structuredContent"]["document"]
            .as_str()
            .expect("second document")
            .to_string();
        let before = std::fs::read(other.project()).expect("chain before collision");
        let collided = call(
            &other,
            json!({
                "op": "import",
                "document": different_document,
                "origin": 1
            }),
        );
        assert_eq!(collided["structuredContent"]["outcome"], "collided");
        assert_eq!(collided["structuredContent"]["revisionId"], 1);
        assert_eq!(collided["structuredContent"]["written"], false);
        assert_eq!(std::fs::read(other.project()).expect("chain after"), before);

        let corrected = call(
            &other,
            json!({
                "op": "correct",
                "revision": 1,
                "question": "What period remains after the correction?",
                "next": {"tool": "open_creation", "arguments": {"capsule": "closing-voices"}},
                "rooms": ["lissajous"]
            }),
        );
        assert_eq!(corrected["isError"], false, "{corrected}");
        assert_eq!(corrected["structuredContent"]["outcome"], "appended");
        let latest = call(&other, json!({"op": "resume"}));
        assert_eq!(
            latest["structuredContent"]["preview"]["question"],
            "What period remains after the correction?"
        );
        let original = call(&other, json!({"op": "resume", "revision": 1}));
        assert_eq!(
            original["structuredContent"]["preview"]["question"],
            "What period do these two oscillators share?"
        );
        let refused = call(
            &other,
            json!({
                "op": "correct",
                "revision": 1,
                "question": "A second correction of the same target.",
                "next": {"tool": "study_room", "arguments": {"room": "lissajous"}},
                "rooms": ["lissajous"]
            }),
        );
        assert_eq!(refused["isError"], true);
    }

    #[test]
    fn a_disagreeing_receipt_still_previews_and_withholds_text() {
        let root = Isolated::new("receipt");
        let played = super::super::play_room_tool(
            &json!({"id": "lissajous", "t": 0.25, "receipt": true}),
            &root.root.join("journey.txt"),
        );
        assert_eq!(played["isError"], false, "{played}");
        let encounter = played["structuredContent"]["encounter"].clone();
        let digest = encounter["resultDigest"].as_str().expect("result digest");
        let kept = call(
            &root,
            json!({
                "op": "keep",
                "question": "Does the receipt still match?",
                "next": {"tool": "study_room", "arguments": {"room": "lissajous"}},
                "rooms": ["lissajous"],
                "evidence": [{"kind": "receipt", "digest": digest, "tool": "play_room"}]
            }),
        );
        assert_eq!(kept["isError"], false, "{kept}");
        let quiet = call(&root, json!({"op": "resume"}));
        assert_eq!(
            quiet["structuredContent"]["preview"]["evidence"][0]["status"],
            "missing"
        );
        assert_eq!(quiet["structuredContent"]["preview"]["willReturn"], true);
        let mut bad = encounter;
        let forged = "ab".repeat(32);
        bad["resultDigest"] = json!(forged);
        let disagreed = project_tool(
            &json!({"op": "resume", "receipt": bad}),
            &root.project(),
            &root.journal(),
            |tool, replay_args| {
                super::super::replay_encounter(tool, replay_args, &root.root.join("journey.txt"))
            },
        );
        assert_eq!(disagreed["isError"], false, "{disagreed}");
        let evidence = &disagreed["structuredContent"]["preview"]["evidence"][0];
        assert_eq!(evidence["status"], "incompatible");
        assert!(evidence["text"].is_null());
        assert!(evidence["subject"].is_null());
        assert_eq!(evidence["tool"], "play_room");
        assert_eq!(
            disagreed["structuredContent"]["preview"]["willReturn"],
            false
        );
        assert_eq!(disagreed["structuredContent"]["applied"], false);
    }

    #[test]
    fn declared_project_result_matches_its_output_schema() {
        let path = super::super::project_path();
        let _ = numinous_core::erase_project_file(&path);
        let kept = super::super::handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "project",
                "arguments": {
                    "op": "keep",
                    "question": "What period do these two oscillators share?",
                    "next": {"tool": "open_creation", "arguments": {"capsule": "closing-voices"}},
                    "rooms": ["lissajous"]
                }
            }
        }))
        .expect("keep response");
        assert_eq!(kept["result"]["isError"], false, "{kept}");
        let resumed = super::super::handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": "project", "arguments": {"op": "resume"}}
        }))
        .expect("resume response");
        assert_eq!(resumed["result"]["isError"], false, "{resumed}");
        let next = &resumed["result"]["structuredContent"]["preview"]["next"];
        let followed = super::super::handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": next["tool"].as_str().expect("tool"),
                "arguments": next["arguments"]
            }
        }))
        .expect("follow response");
        assert_eq!(followed["result"]["isError"], false, "{followed}");
        assert_eq!(
            followed["result"]["structuredContent"]["closure"]["kind"],
            "voices"
        );
        let _ = numinous_core::erase_project_file(&path);
    }
}
