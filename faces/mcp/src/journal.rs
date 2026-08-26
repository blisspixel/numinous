//! MCP projections for the player-owned experience journal.

use numinous_core::EncounterTool;
use serde_json::{Value, json};

use super::{encounter::parse_submitted_receipt, tool_error, tool_structured, tool_text};

/// Default and maximum entry counts for one journal read or export page.
pub(super) const DEFAULT_PAGE_ENTRIES: usize = 50;
pub(super) const MAX_PAGE_ENTRIES: usize = 100;

/// Inspect a player-owned journal page without hiding persistence errors.
pub(super) fn read_tool(args: &Value, path: &std::path::Path) -> Value {
    let journal = match numinous_core::try_load_journal_file(path) {
        Ok(journal) => journal,
        Err(error) => return tool_error(&format!("Failed to read journal: {error}")),
    };
    let (after_entry_id, limit) = page_args(args);
    let structured = page_json(&journal, after_entry_id, limit);
    let entries = structured["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if entries.is_empty() {
        return tool_structured("Your journal is empty on this page.", structured);
    }
    let mut lines = Vec::with_capacity(entries.len() + 1);
    for entry in &entries {
        let status = if entry["current"] == true {
            "current"
        } else {
            "superseded"
        };
        let supersedes = entry["supersedes"]
            .as_u64()
            .map_or_else(String::new, |entry_id| format!("; supersedes #{entry_id}"));
        let affect = entry["affect"].as_str().map_or_else(String::new, |value| {
            format!("; affect {}", display_field(value))
        });
        lines.push(format!(
            "#{} [{status}] event {}; recorded {}; source {}; {} {}: {}{affect}{supersedes}",
            entry["entryId"].as_u64().unwrap_or_default(),
            entry["eventAtUtc"].as_u64().unwrap_or_default(),
            entry["recordedAtUtc"].as_u64().unwrap_or_default(),
            entry["source"].as_str().unwrap_or_default(),
            display_field(entry["kind"].as_str().unwrap_or_default()),
            display_field(entry["subject"].as_str().unwrap_or_default()),
            display_field(entry["text"].as_str().unwrap_or_default()),
        ));
    }
    if structured["page"]["hasMore"] == true {
        lines.push(format!(
            "More entries follow. Continue after_entry_id {}.",
            structured["page"]["nextAfterEntryId"]
                .as_u64()
                .unwrap_or_default()
        ));
    }
    tool_structured(&lines.join("\n"), structured)
}

fn promote_receipt(
    receipt: &Value,
    replay: &impl Fn(EncounterTool, &Value) -> Value,
) -> Result<String, String> {
    let submitted = parse_submitted_receipt(receipt)?;
    let replayed = replay(submitted.tool, &submitted.replay_args);
    if replayed.get("isError").and_then(Value::as_bool) == Some(true) {
        let message = replayed
            .get("content")
            .and_then(Value::as_array)
            .and_then(|content| content.first())
            .and_then(|block| block.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("the room could not be replayed");
        return Err(format!("This receipt cannot be replayed: {message}"));
    }
    let live = replayed
        .get("structuredContent")
        .and_then(|content| content.get("encounter"))
        .ok_or_else(|| "Replaying this receipt did not produce an encounter.".to_string())?;
    let live_action = live
        .get("actionDigest")
        .and_then(Value::as_str)
        .ok_or_else(|| "Replaying this receipt did not produce an actionDigest.".to_string())?;
    let live_result = live
        .get("resultDigest")
        .and_then(Value::as_str)
        .ok_or_else(|| "Replaying this receipt did not produce a resultDigest.".to_string())?;
    if live_action != submitted.action_digest || live_result != submitted.result_digest {
        return Err(
            "This receipt does not match a live replay of its action. A keep is refused when the proof and the room disagree."
                .to_string(),
        );
    }
    Ok(submitted.result_digest)
}

/// Append one explicit journal entry, replaying a submitted receipt when present.
pub(super) fn record_tool(
    args: &Value,
    path: &std::path::Path,
    replay: impl Fn(EncounterTool, &Value) -> Value,
) -> Value {
    let kind = args.get("kind").and_then(Value::as_str).unwrap_or("");
    let text = args.get("text").and_then(Value::as_str).unwrap_or("");
    let affect = args.get("affect").and_then(Value::as_str);
    let recorded_at_utc = now();
    let event_at_utc = args
        .get("event_time_utc")
        .and_then(Value::as_u64)
        .unwrap_or(recorded_at_utc);
    if event_at_utc > recorded_at_utc {
        return tool_error("event_time_utc cannot be later than the server record time.");
    }

    let (source, subject) = if let Some(receipt) = args.get("receipt") {
        match promote_receipt(receipt, &replay) {
            Ok(digest) => {
                let expected = format!("{}{digest}", numinous_core::JOURNAL_SUBJECT_RECEIPT_PREFIX);
                if let Some(subject) = args.get("subject").and_then(Value::as_str)
                    && subject != expected
                    && subject != digest
                {
                    return tool_error(&format!(
                        "A promoted receipt is stored as subject '{expected}'. Pass that, the bare digest, or omit subject."
                    ));
                }
                if let Some(source) = args.get("source").and_then(Value::as_str)
                    && source != numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT
                {
                    return tool_error(
                        "A promoted receipt is recorded as source numinous-result. Omit source, or pass that token.",
                    );
                }
                (numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT, expected)
            }
            Err(message) => return tool_error(&message),
        }
    } else {
        let source = args
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or(numinous_core::JOURNAL_SOURCE_SELF_AUTHORED);
        if source == numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT {
            return tool_error(
                "Source numinous-result requires the structuredContent.encounter object as receipt. Asking does not keep a play.",
            );
        }
        let Some(subject) = args.get("subject").and_then(Value::as_str) else {
            return tool_error("Missing required string argument 'subject'.");
        };
        (source, subject.to_string())
    };

    match numinous_core::record_journal_file(
        path,
        numinous_core::JournalRecord {
            recorded_at_utc,
            event_at_utc,
            source,
            kind,
            subject: &subject,
            text,
            affect,
        },
    ) {
        Ok(entry) => tool_structured(
            &format!("Record #{} saved.", entry.entry_id),
            json!({
                "action": "record",
                "entryId": entry.entry_id,
                "recordedAtUtc": entry.recorded_at_utc,
                "eventAtUtc": entry.event_at_utc,
                "source": entry.source,
                "kind": entry.kind,
                "subject": entry.subject,
            }),
        ),
        Err(error) => tool_error(&format!("Failed to record: {error}")),
    }
}

/// Append an immutable correction to an existing journal entry.
pub(super) fn correct_tool(args: &Value, path: &std::path::Path) -> Value {
    let supersedes = args
        .get("entry_id")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let text = args.get("text").and_then(Value::as_str).unwrap_or("");
    let affect = args.get("affect").and_then(Value::as_str);
    let source = args
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or(numinous_core::JOURNAL_SOURCE_SELF_AUTHORED);
    if source == numinous_core::JOURNAL_SOURCE_NUMINOUS_RESULT {
        return tool_error(
            "Correction source numinous-result is unavailable because correct_journal has no replay-verified receipt. Use self-authored or player-provided.",
        );
    }
    let recorded_at_utc = now();
    let event_at_utc = args.get("event_time_utc").and_then(Value::as_u64);
    if event_at_utc.is_some_and(|event_time| event_time > recorded_at_utc) {
        return tool_error("event_time_utc cannot be later than the server record time.");
    }
    match numinous_core::correct_journal_file(
        path,
        recorded_at_utc,
        event_at_utc,
        source,
        supersedes,
        text,
        affect,
    ) {
        Ok(entry) => tool_structured(
            &format!(
                "Correction #{} saved; original #{supersedes} remains inspectable.",
                entry.entry_id
            ),
            json!({
                "action": "correct",
                "entryId": entry.entry_id,
                "supersedes": entry.supersedes,
                "recordedAtUtc": entry.recorded_at_utc,
                "eventAtUtc": entry.event_at_utc,
                "source": entry.source,
                "appendOnly": true,
            }),
        ),
        Err(error) => tool_error(&format!("Failed to correct: {error}")),
    }
}

/// Return a bounded native or OKF journal page without creating a host file.
pub(super) fn export_tool(args: &Value, path: &std::path::Path) -> Value {
    let journal = match numinous_core::try_load_journal_file(path) {
        Ok(journal) => journal,
        Err(error) => return tool_error(&format!("Failed to export journal: {error}")),
    };
    let (after_entry_id, limit) = page_args(args);
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("native");
    if format == "okf-0.2" {
        let page = numinous_core::export_journal_okf(&journal, after_entry_id, limit);
        let files = page
            .files
            .into_iter()
            .map(|file| {
                json!({
                    "path": file.path,
                    "content": file.content,
                })
            })
            .collect::<Vec<_>>();
        return tool_structured(
            &format!(
                "Exported {} journal entries as an in-memory OKF {} bundle page. No file was created.",
                page.returned,
                numinous_core::OKF_VERSION
            ),
            json!({
                "schema": numinous_core::OKF_BUNDLE_SCHEMA,
                "schemaVersion": numinous_core::OKF_VERSION,
                "sourceSchema": "numinous.experience-journal",
                "sourceSchemaVersion": numinous_core::JOURNAL_SCHEMA_VERSION,
                "files": files,
                "page": {
                    "afterEntryId": page.after_entry_id,
                    "limit": limit,
                    "returned": page.returned,
                    "hasMore": page.has_more,
                    "nextAfterEntryId": page.next_after_entry_id,
                },
                "totalEntries": page.total_entries,
                "createdFile": false,
                "containsHostPath": false,
            }),
        );
    }
    if format != "native" {
        return tool_error("format must be native or okf-0.2.");
    }
    let mut structured = page_json(&journal, after_entry_id, limit);
    if let Some(object) = structured.as_object_mut() {
        object.insert("schema".to_string(), json!("numinous.experience-journal"));
        object.insert(
            "schemaVersion".to_string(),
            json!(numinous_core::JOURNAL_SCHEMA_VERSION),
        );
        object.insert("createdFile".to_string(), json!(false));
        object.insert("containsHostPath".to_string(), json!(false));
    }
    let returned = structured["page"]["returned"].as_u64().unwrap_or_default();
    tool_structured(
        &format!(
            "Exported {returned} journal entries as schema version {}. No file was created.",
            numinous_core::JOURNAL_SCHEMA_VERSION
        ),
        structured,
    )
}

/// Preview or perform verified journal erasure.
pub(super) fn erase_tool(args: &Value, path: &std::path::Path) -> Value {
    let confirm = args
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !confirm {
        tool_text("Pass confirm: true to permanently erase your journal.")
    } else {
        let existed = match numinous_core::inspect_journal_file(path) {
            Ok(inventory) => inventory.exists || inventory.sidecar_files != 0,
            Err(error) => return tool_error(&format!("Failed to inspect journal: {error}")),
        };
        match numinous_core::erase_journal_file(path) {
            Ok(inventory) => tool_structured(
                if existed {
                    "Journal erased; zero recoverable managed residue remains."
                } else {
                    "Journal was already empty; zero recoverable managed residue remains."
                },
                json!({
                    "action": "erase",
                    "confirmRequired": true,
                    "confirmed": true,
                    "previouslyPresent": existed,
                    "managedFileResidue": inventory.exists,
                    "managedSidecarFiles": inventory.sidecar_files,
                    "sidecarScanCapped": inventory.sidecar_scan_capped,
                    "recoverableManagedResidue": 0,
                    "projectControlledExportFiles": 0,
                    "externalBackupsCovered": false,
                }),
            ),
            Err(error) => tool_error(&format!("Failed to erase: {error}")),
        }
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn page_args(args: &Value) -> (u64, usize) {
    let after_entry_id = args
        .get("after_entry_id")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_PAGE_ENTRIES)
        .clamp(1, MAX_PAGE_ENTRIES);
    (after_entry_id, limit)
}

fn page_json(journal: &numinous_core::Journal, after_entry_id: u64, limit: usize) -> Value {
    let available = journal
        .entries
        .iter()
        .filter(|entry| entry.entry_id > after_entry_id)
        .collect::<Vec<_>>();
    let entries = available
        .iter()
        .take(limit)
        .map(|entry| entry_json(journal, entry))
        .collect::<Vec<_>>();
    let next_after_entry_id = entries
        .last()
        .and_then(|entry| entry["entryId"].as_u64())
        .unwrap_or(after_entry_id);
    json!({
        "totalEntries": journal.entries.len(),
        "entries": entries,
        "page": {
            "afterEntryId": after_entry_id,
            "limit": limit,
            "returned": available.len().min(limit),
            "hasMore": available.len() > limit,
            "nextAfterEntryId": next_after_entry_id,
        }
    })
}

pub(super) fn entry_json(
    journal: &numinous_core::Journal,
    entry: &numinous_core::JournalEntry,
) -> Value {
    json!({
        "entryId": entry.entry_id,
        "recordedAtUtc": entry.recorded_at_utc,
        "eventAtUtc": entry.event_at_utc,
        "source": entry.source,
        "kind": entry.kind,
        "subject": entry.subject,
        "text": entry.text,
        "affect": entry.affect,
        "supersedes": entry.supersedes,
        "current": journal.is_current(entry.entry_id),
    })
}

fn display_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
#[cfg(test)]
mod tests {
    use numinous_core::EncounterTool;
    use serde_json::{Value, json};

    fn record(args: &Value, path: &std::path::Path, _journey: &std::path::Path) -> Value {
        super::record_tool(args, path, |_tool: EncounterTool, _args: &Value| {
            panic!("a receipt-free journal test must not request replay")
        })
    }

    #[test]
    fn journal_correction_export_and_erasure_preserve_sovereignty() {
        let path = crate::test_state_path("journal-sovereignty");
        let empty = super::read_tool(&json!({}), &path);
        assert_eq!(empty["structuredContent"]["totalEntries"], 0);

        let journey = crate::test_state_path("journal-sovereignty-journey");
        let future = record(
            &json!({
                "kind": "encounter",
                "subject": "times-tables",
                "text": "Future claim",
                "event_time_utc": u64::MAX
            }),
            &path,
            &journey,
        );
        assert_eq!(future["isError"], true);
        assert_eq!(
            super::read_tool(&json!({}), &path)["structuredContent"]["totalEntries"],
            0
        );

        let encounter = record(
            &json!({
                "kind": "encounter",
                "subject": "times-tables",
                "text": "The multiplier closed nine loops.",
                "event_time_utc": 10,
                "source": "self-authored"
            }),
            &path,
            &journey,
        );
        assert_eq!(encounter["isError"], false);
        assert_eq!(encounter["structuredContent"]["entryId"], 1);
        assert_eq!(encounter["structuredContent"]["eventAtUtc"], 10);
        assert!(
            encounter["structuredContent"]["recordedAtUtc"]
                .as_u64()
                .is_some_and(|recorded| recorded >= 10)
        );

        let connection = record(
            &json!({
                "kind": "connection",
                "subject": "times-tables",
                "text": "Nine means nine lobes.",
                "event_time_utc": 11
            }),
            &path,
            &journey,
        );
        assert_eq!(connection["structuredContent"]["entryId"], 2);

        let impersonated_correction = super::correct_tool(
            &json!({
                "entry_id": 2,
                "text": "This correction has no replay proof.",
                "source": "numinous-result"
            }),
            &path,
        );
        assert_eq!(impersonated_correction["isError"], true);
        assert_eq!(
            super::read_tool(&json!({}), &path)["structuredContent"]["totalEntries"],
            2,
            "an unverified source claim must not mutate the journal"
        );

        let correction = super::correct_tool(
            &json!({
                "entry_id": 2,
                "text": "The visible lobe count follows multiplier minus one.",
                "source": "self-authored"
            }),
            &path,
        );
        assert_eq!(correction["structuredContent"]["entryId"], 3);
        assert_eq!(correction["structuredContent"]["supersedes"], 2);
        assert_eq!(correction["structuredContent"]["eventAtUtc"], 11);
        assert_eq!(correction["structuredContent"]["appendOnly"], true);

        let repeated =
            super::correct_tool(&json!({"entry_id": 2, "text": "Silent rewrite"}), &path);
        assert_eq!(repeated["isError"], true);
        assert!(
            repeated["content"][0]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("already superseded")
        );

        let first_page = super::read_tool(&json!({"limit": 2}), &path);
        assert_eq!(first_page["structuredContent"]["page"]["returned"], 2);
        assert_eq!(first_page["structuredContent"]["page"]["hasMore"], true);
        assert_eq!(
            first_page["structuredContent"]["page"]["nextAfterEntryId"],
            2
        );
        let exported = super::export_tool(&json!({"after_entry_id": 0, "limit": 100}), &path);
        let data = &exported["structuredContent"];
        assert_eq!(data["schema"], "numinous.experience-journal");
        assert_eq!(data["schemaVersion"], numinous_core::JOURNAL_SCHEMA_VERSION);
        assert_eq!(data["createdFile"], false);
        assert_eq!(data["containsHostPath"], false);
        assert_eq!(data["entries"].as_array().map(Vec::len), Some(3));
        assert_eq!(data["entries"][1]["current"], false);
        assert_eq!(data["entries"][2]["current"], true);
        assert_eq!(data["entries"][2]["supersedes"], 2);
        assert_eq!(data["entries"][1]["source"], "self-authored");
        assert_eq!(data["entries"][2]["source"], "self-authored");
        assert_eq!(data["entries"][1]["text"], "Nine means nine lobes.");

        let okf = super::export_tool(
            &json!({"after_entry_id": 0, "limit": 2, "format": "okf-0.2"}),
            &path,
        );
        let okf_data = &okf["structuredContent"];
        assert_eq!(okf_data["schema"], numinous_core::OKF_BUNDLE_SCHEMA);
        assert_eq!(okf_data["schemaVersion"], numinous_core::OKF_VERSION);
        assert_eq!(okf_data["sourceSchema"], "numinous.experience-journal");
        assert_eq!(okf_data["page"]["returned"], 2);
        assert_eq!(okf_data["page"]["hasMore"], true);
        assert_eq!(okf_data["files"].as_array().map(Vec::len), Some(3));
        assert_eq!(okf_data["files"][0]["path"], "index.md");
        assert!(
            okf_data["files"][0]["content"]
                .as_str()
                .unwrap_or_default()
                .contains("okf_version: \"0.2\"")
        );
        assert_eq!(
            okf_data["files"][1]["path"],
            "entries/00000000000000000001.md"
        );
        assert_eq!(okf_data["createdFile"], false);
        assert_eq!(okf_data["containsHostPath"], false);
        assert!(
            !serde_json::to_string(okf_data)
                .expect("serialize OKF export")
                .contains(path.to_str().expect("journal path is UTF-8"))
        );

        let invalid = super::export_tool(&json!({"format": "future"}), &path);
        assert_eq!(invalid["isError"], true);

        let preview = super::erase_tool(&json!({"confirm": false}), &path);
        assert_eq!(preview["isError"], false);
        assert!(path.exists());
        let erased = super::erase_tool(&json!({"confirm": true}), &path);
        assert_eq!(erased["structuredContent"]["recoverableManagedResidue"], 0);
        assert_eq!(erased["structuredContent"]["managedSidecarFiles"], 0);
        assert_eq!(erased["structuredContent"]["sidecarScanCapped"], false);
        assert_eq!(
            erased["structuredContent"]["projectControlledExportFiles"],
            0
        );
        assert_eq!(
            super::read_tool(&json!({}), &path)["structuredContent"]["totalEntries"],
            0
        );
    }
}
