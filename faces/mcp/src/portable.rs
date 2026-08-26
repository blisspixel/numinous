//! Portable, typed evidence capsules for explicit player handoff.

use std::collections::BTreeMap;

use numinous_core::{EncounterTool, Journal};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::journal::{MAX_PAGE_ENTRIES, page_json, verify_receipt};

/// Stable schema emitted by the portable evidence export.
pub(super) const CAPSULE_SCHEMA: &str = "numinous.portable-evidence-capsule";
/// First portable evidence capsule schema version.
pub(super) const CAPSULE_SCHEMA_VERSION: u64 = 1;

struct PayloadFile {
    path: String,
    media_type: &'static str,
    content: String,
}

/// Build one bounded evidence capsule entirely in memory.
pub(super) fn export(
    args: &Value,
    journal: &Journal,
    after_entry_id: u64,
    limit: usize,
    replay: &impl Fn(EncounterTool, &Value) -> Value,
) -> Result<Value, String> {
    let limit = limit.clamp(1, MAX_PAGE_ENTRIES);
    let mut native_page = page_json(journal, after_entry_id, limit);
    let Some(native_object) = native_page.as_object_mut() else {
        return Err("The native journal projection was not an object.".to_string());
    };
    native_object.insert("schema".to_string(), json!("numinous.experience-journal"));
    native_object.insert(
        "schemaVersion".to_string(),
        json!(numinous_core::JOURNAL_SCHEMA_VERSION),
    );

    let okf_page = numinous_core::export_journal_okf(journal, after_entry_id, limit);
    let mut files = vec![PayloadFile {
        path: "native/journal-page.json".to_string(),
        media_type: "application/json",
        content: json_text(&canonical_json(&native_page))?,
    }];
    files.extend(okf_page.files.into_iter().map(|file| PayloadFile {
        path: file.path,
        media_type: "text/markdown; charset=utf-8",
        content: file.content,
    }));

    let receipt_result_digest = if let Some(receipt) = args.get("receipt") {
        let digest = verify_receipt(receipt, replay)?;
        files.push(PayloadFile {
            path: "native/encounter-receipt.json".to_string(),
            media_type: "application/json",
            content: json_text(&canonical_json(receipt))?,
        });
        Some(digest)
    } else {
        None
    };

    let creation_included = if let Some(value) = args.get("creation") {
        let input = value.as_str().ok_or_else(|| {
            "Argument 'creation' must be complete Studio .num text or a native link.".to_string()
        })?;
        let creation = numinous_core::StudioCreation::from_capsule(input)
            .map_err(|error| format!("Could not include Studio creation: {error}"))?;
        files.push(PayloadFile {
            path: "creations/studio.num".to_string(),
            media_type: "application/vnd.numinous.studio+text; charset=utf-8",
            content: creation.to_num_file(),
        });
        true
    } else {
        false
    };

    let contains_self_reported_affect = native_page["entries"]
        .as_array()
        .is_some_and(|entries| entries.iter().any(|entry| !entry["affect"].is_null()));
    let privacy = json!({
        "schema": "numinous.portable-evidence-privacy",
        "schemaVersion": 1,
        "dataClasses": {
            "journalEntries": true,
            "selfReportedAffect": contains_self_reported_affect,
            "encounterReceipt": receipt_result_digest.is_some(),
            "studioCreation": creation_included,
        },
        "selection": "explicit bounded export",
        "affectPolicy": "Only affect explicitly recorded by the player is included.",
        "journalTextPolicy": "Selected journal fields are included exactly and are not scanned for secrets.",
        "notAutomaticallyCollected": [
            "filesystem paths",
            "host private prompts",
            "host hidden reasoning",
            "raw frames",
            "audio buffers",
            "arbitrary host logs",
            "mutable session state"
        ],
        "sharing": "Inspect journal text before sharing. The caller controls whether and where this response is shared."
    });
    files.push(PayloadFile {
        path: "privacy.json".to_string(),
        media_type: "application/json",
        content: json_text(&canonical_json(&privacy))?,
    });

    let retention = json!({
        "schema": "numinous.portable-evidence-retention",
        "schemaVersion": 1,
        "storedByExport": false,
        "createdFile": false,
        "responseOnly": true,
        "importSupported": false,
        "externalCopies": "Caller managed. Journal erasure cannot erase copies outside Numinous.",
        "nativeSource": "The player-owned journal remains unchanged by export."
    });
    files.push(PayloadFile {
        path: "retention.json".to_string(),
        media_type: "application/json",
        content: json_text(&canonical_json(&retention))?,
    });

    files.sort_by(|left, right| left.path.cmp(&right.path));
    let manifest_files = files
        .iter()
        .map(|file| {
            json!({
                "path": file.path,
                "mediaType": file.media_type,
                "bytes": file.content.len(),
                "sha256": sha256(file.content.as_bytes()),
            })
        })
        .collect::<Vec<_>>();
    let manifest = canonical_json(&json!({
        "schema": "numinous.portable-evidence-manifest",
        "schemaVersion": 1,
        "capsuleSchema": CAPSULE_SCHEMA,
        "capsuleSchemaVersion": CAPSULE_SCHEMA_VERSION,
        "closedFileSet": true,
        "digestAlgorithm": "sha256",
        "digestEncoding": "lowercase hexadecimal",
        "jsonCanonicalization": "UTF-8 compact JSON with object keys in lexicographic order",
        "files": manifest_files,
        "selection": {
            "journal": {
                "afterEntryId": after_entry_id,
                "limit": limit,
                "returned": okf_page.returned,
                "hasMore": okf_page.has_more,
                "nextAfterEntryId": okf_page.next_after_entry_id,
                "totalEntries": okf_page.total_entries,
            },
            "receiptResultDigest": receipt_result_digest,
            "creationIncluded": creation_included,
        },
        "importSupported": false,
    }));
    let manifest_sha256 = sha256(json_text(&manifest)?.as_bytes());
    let files = files
        .into_iter()
        .map(|file| {
            json!({
                "path": file.path,
                "mediaType": file.media_type,
                "bytes": file.content.len(),
                "sha256": sha256(file.content.as_bytes()),
                "content": file.content,
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "schema": CAPSULE_SCHEMA,
        "schemaVersion": CAPSULE_SCHEMA_VERSION,
        "manifest": manifest,
        "manifestSha256": manifest_sha256,
        "files": files,
        "createdFile": false,
        "readCallerSuppliedPath": false,
        "sourceJournalRead": true,
        "containsHostPath": false,
        "importSupported": false,
    }))
}

fn json_text(value: &Value) -> Result<String, String> {
    serde_json::to_string(value)
        .map_err(|error| format!("Could not serialize portable evidence: {error}"))
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonical_json).collect()),
        Value::Object(object) => {
            let sorted = object
                .iter()
                .map(|(key, value)| (key.clone(), canonical_json(value)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(sorted.into_iter().collect::<Map<_, _>>())
        }
        value => value.clone(),
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use numinous_core::{EncounterTool, Journal, JournalRecord};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    fn replay(_tool: EncounterTool, _args: &Value) -> Value {
        panic!("a receipt-free capsule test must not replay")
    }

    fn journal() -> Journal {
        let mut journal = Journal::new();
        journal
            .record(JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 10,
                source: numinous_core::JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "creation",
                subject: "times-tables",
                text: "Nine visible lobes.",
                affect: Some("curious"),
            })
            .expect("record");
        journal
    }

    #[test]
    fn capsule_hashes_a_closed_sorted_payload_without_writing() {
        let creation = numinous_core::StudioCreation::new("sin(a*x)", -2.0, 3.0, 0.75)
            .expect("creation")
            .with_title("First Wave")
            .expect("title")
            .with_author("First Hand")
            .expect("author")
            .with_era(numinous_core::Era::Vector);
        let capsule = super::export(
            &json!({"creation":creation.to_num_file()}),
            &journal(),
            0,
            100,
            &replay,
        )
        .expect("capsule");

        assert_eq!(capsule["schema"], super::CAPSULE_SCHEMA);
        assert_eq!(capsule["schemaVersion"], 1);
        assert_eq!(capsule["createdFile"], false);
        assert_eq!(capsule["readCallerSuppliedPath"], false);
        assert_eq!(capsule["sourceJournalRead"], true);
        assert_eq!(capsule["containsHostPath"], false);
        assert_eq!(capsule["importSupported"], false);
        assert_eq!(capsule["manifest"]["closedFileSet"], true);
        assert_eq!(capsule["manifest"]["selection"]["journal"]["returned"], 1);

        let files = capsule["files"].as_array().expect("files");
        let paths = files
            .iter()
            .map(|file| file["path"].as_str().expect("path"))
            .collect::<Vec<_>>();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted);
        assert!(
            paths.iter().all(|path| {
                !path.starts_with('/')
                    && !path.contains('\\')
                    && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
            }),
            "every payload path is a safe bundle-relative path"
        );
        assert_eq!(
            paths,
            vec![
                "creations/studio.num",
                "entries/00000000000000000001.md",
                "index.md",
                "native/journal-page.json",
                "privacy.json",
                "retention.json",
            ]
        );
        assert_eq!(
            capsule["manifest"]["files"]
                .as_array()
                .expect("manifest files")
                .len(),
            files.len()
        );
        assert_eq!(capsule["manifest"]["digestAlgorithm"], "sha256");
        for (file, manifest_file) in files.iter().zip(
            capsule["manifest"]["files"]
                .as_array()
                .expect("manifest files"),
        ) {
            assert_eq!(file["path"], manifest_file["path"]);
            assert_eq!(file["mediaType"], manifest_file["mediaType"]);
            assert_eq!(file["bytes"], manifest_file["bytes"]);
            assert_eq!(file["sha256"], manifest_file["sha256"]);
            let content = file["content"].as_str().expect("content");
            assert_eq!(file["bytes"], content.len());
            assert_eq!(
                file["sha256"],
                Sha256::digest(content.as_bytes())
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
        }
        let manifest_text = serde_json::to_string(&capsule["manifest"]).expect("manifest text");
        assert_eq!(
            capsule["manifestSha256"],
            Sha256::digest(manifest_text.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );

        let creation_file = files
            .iter()
            .find(|file| file["path"] == "creations/studio.num")
            .expect("creation file");
        let reopened = numinous_core::StudioCreation::from_num_file(
            creation_file["content"].as_str().expect("creation content"),
        )
        .expect("canonical creation");
        assert_eq!(reopened, creation);
        let privacy = files
            .iter()
            .find(|file| file["path"] == "privacy.json")
            .expect("privacy file");
        assert!(
            privacy["content"]
                .as_str()
                .is_some_and(|content| content.contains("selfReportedAffect")
                    && content.contains("not scanned for secrets"))
        );
    }

    #[test]
    fn empty_capsule_is_bounded_and_invalid_creation_is_refused() {
        let capsule = super::export(&json!({}), &Journal::new(), 0, usize::MAX, &replay)
            .expect("empty capsule");
        assert_eq!(capsule["manifest"]["selection"]["journal"]["returned"], 0);
        assert_eq!(capsule["manifest"]["selection"]["journal"]["limit"], 100);
        assert_eq!(capsule["files"].as_array().map(Vec::len), Some(4));

        let error = super::export(
            &json!({"creation":"C:\\private\\creation.num"}),
            &Journal::new(),
            0,
            10,
            &replay,
        )
        .expect_err("paths are data, not ambient reads");
        assert!(error.contains("Could not include Studio creation"));
        assert!(!error.contains("private"));
    }
}
