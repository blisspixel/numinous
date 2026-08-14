//! MCP presentation for player-owned local-state inventory and erasure.
//!
//! The core owns path resolution, locking, inspection, and mutation. This
//! module owns only the typed MCP projection and explicit consent boundary.

use numinous_core::{
    LocalFileInventory, LocalStateEraseSelection, LocalStateInventory, LocalStatePaths,
    erase_local_state, inspect_local_state,
};
use serde_json::{Value, json};

fn safe_path_text(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .chars()
        .flat_map(char::escape_default)
        .collect()
}

fn file_inventory_json(file: &LocalFileInventory) -> Value {
    json!({
        "path": file.path.to_string_lossy(),
        "exists": file.exists,
        "bytes": file.bytes,
        "managed_regular_file": file.managed_file,
        "sidecar_files": file.sidecar_files,
        "sidecar_bytes": file.sidecar_bytes,
        "sidecar_scan_capped": file.sidecar_scan_capped,
    })
}

fn inventory_json(inventory: &LocalStateInventory) -> Value {
    let mut journey = file_inventory_json(&inventory.journey.file);
    journey["rooms_entered"] = json!(inventory.journey.rooms_entered);
    journey["wins"] = json!(inventory.journey.wins);
    journey["plays"] = json!(inventory.journey.plays);
    journey["secrets_heard"] = json!(inventory.journey.secrets_heard);
    let mut scores = file_inventory_json(&inventory.scores.file);
    scores["entries"] = json!(inventory.scores.entries);
    let mut cairn = file_inventory_json(&inventory.cairn.file);
    cairn["local_plaintext_drafts"] = json!(inventory.cairn.local_drafts);
    cairn["bundled_canonical_stones_preserved"] = json!(true);
    json!({
        "journey": journey,
        "scores": scores,
        "cairn": cairn,
        "journal": file_inventory_json(&inventory.journal),
        "radio_cache": {
            "path": inventory.radio_cache.path.to_string_lossy(),
            "exists": inventory.radio_cache.exists,
            "bytes": inventory.radio_cache.bytes,
            "files": inventory.radio_cache.files,
            "unexpected_entries": inventory.radio_cache.unexpected_entries,
            "scan_capped": inventory.radio_cache.truncated,
            "sidecar_files": inventory.radio_cache.sidecar_files,
            "sidecar_bytes": inventory.radio_cache.sidecar_bytes,
            "sidecar_scan_capped": inventory.radio_cache.sidecar_scan_capped,
        },
        "crash_log": file_inventory_json(&inventory.crash_log),
        "managed_bytes": inventory.total_managed_bytes(),
        "managed_store_residue": inventory.managed_residue_count(),
    })
}

/// Return a truthful inventory first, then erase only with explicit consent.
pub(super) fn forget_tool(args: &Value, paths: &LocalStatePaths) -> Value {
    let confirm = args
        .get("confirm")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let all_local = args
        .get("all_local")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let selection = if all_local {
        LocalStateEraseSelection::complete()
    } else {
        LocalStateEraseSelection {
            journey: true,
            scores: args.get("scores").and_then(Value::as_bool).unwrap_or(false),
            cairn: args.get("cairn").and_then(Value::as_bool).unwrap_or(false),
            journal: args
                .get("journal")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            radio_cache: args
                .get("radio_cache")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            crash_log: args
                .get("crash_log")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    };
    let before = match inspect_local_state(paths) {
        Ok(inventory) => inventory,
        Err(error) => {
            return super::tool_error(&format!("Could not inventory local state: {error}."));
        }
    };
    if !confirm {
        let text = format!(
            "Numinous-managed local state:\n\
             journey: {} rooms, {} wins, {} plays, {} secrets, {} bytes at {}\n\
             scores: {} entries, {} bytes at {}\n\
             Cairn: {} local plaintext drafts, {} bytes at {}\n\
             journal: {} bytes at {}\n\
             radio cache: {} generated WAV files, {} bytes, {} unexpected entries, {} sidecar files and {} sidecar bytes at {}\n\
             crash log: {} bytes at {}\n\n\
             No state was erased by this preview. Confirm the Journey alone, select other stores, or set all_local true for complete managed erasure. User-selected exports, installed files, the Rust toolchain, and bundled canonical Cairn stones are outside this command.",
            before.journey.rooms_entered,
            before.journey.wins,
            before.journey.plays,
            before.journey.secrets_heard,
            before.journey.file.bytes,
            safe_path_text(&before.journey.file.path),
            before.scores.entries,
            before.scores.file.bytes,
            safe_path_text(&before.scores.file.path),
            before.cairn.local_drafts,
            before.cairn.file.bytes,
            safe_path_text(&before.cairn.file.path),
            before.journal.bytes,
            safe_path_text(&before.journal.path),
            before.radio_cache.files,
            before.radio_cache.bytes,
            before.radio_cache.unexpected_entries,
            before.radio_cache.sidecar_files,
            before.radio_cache.sidecar_bytes,
            safe_path_text(&before.radio_cache.path),
            before.crash_log.bytes,
            safe_path_text(&before.crash_log.path),
        );
        return super::tool_structured(
            &text,
            json!({
                "action": "preview",
                "confirm_required": true,
                "requested_scores_erasure": selection.scores,
                "requested_erasure": {
                    "journey": selection.journey,
                    "scores": selection.scores,
                    "cairn": selection.cairn,
                    "journal": selection.journal,
                    "radio_cache": selection.radio_cache,
                    "crash_log": selection.crash_log,
                    "all_local": all_local,
                },
                "remembered": inventory_json(&before),
                "exclusions": [
                    "user-selected exports",
                    "installed application files",
                    "Rust toolchain",
                    "bundled canonical Cairn stones",
                ],
            }),
        );
    }
    let after = match erase_local_state(paths, selection) {
        Ok(inventory) => inventory,
        Err(error) => {
            let residue = inspect_local_state(paths)
                .map(|inventory| {
                    format!(
                        " {} managed stores and {} known bytes remain.",
                        inventory.managed_residue_count(),
                        inventory.total_managed_bytes()
                    )
                })
                .unwrap_or_else(|_| " Residue could not be inventoried.".to_string());
            return super::tool_error(&format!(
                "Erasure stopped at {}: {}.{residue}",
                error.target(),
                error
            ));
        }
    };
    if all_local && after.managed_residue_count() != 0 {
        return super::tool_error(&format!(
            "Complete erasure could not be verified: {} managed stores and {} known bytes remain.",
            after.managed_residue_count(),
            after.total_managed_bytes()
        ));
    }
    super::tool_structured(
        &format!(
            "Selected local state erased and verified. {} managed stores and {} known bytes remain. Bundled canonical Cairn stones and user-selected exports were not changed.",
            after.managed_residue_count(),
            after.total_managed_bytes()
        ),
        json!({
            "action": "erase",
            "confirmed": true,
            "journey_erased": selection.journey,
            "scores_erased": selection.scores,
            "scores_preserved": !selection.scores,
            "cairn_erased": selection.cairn,
            "journal_erased": selection.journal,
            "radio_cache_erased": selection.radio_cache,
            "crash_log_erased": selection.crash_log,
            "all_local": all_local,
            "before": inventory_json(&before),
            "residue": inventory_json(&after),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::forget_tool;
    use numinous_core::LocalStatePaths;
    use serde_json::json;

    #[test]
    fn shows_first_and_erases_only_on_consent() {
        let root = std::env::temp_dir().join(format!("numinous_mcp_forget_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = LocalStatePaths {
            journey: root.join("journey.txt"),
            scores: root.join("scores.txt"),
            cairn: root.join("cairn.txt"),
            journal: root.join("journal.txt"),
            radio_cache: root.join("radio"),
            protected_radio_source: None,
            crash_log: root.join("crash.log"),
        };
        std::fs::create_dir_all(&paths.radio_cache).unwrap();
        std::fs::write(
            &paths.journey,
            "visited lorenz
wins 1
secrets 0
plays 2
",
        )
        .unwrap();
        std::fs::write(&paths.scores, "50\tmunch seed:1 board:0\n").unwrap();
        std::fs::write(&paths.cairn, "Ada\tproof is a program\n").unwrap();
        std::fs::write(&paths.journal, "one opt-in experience\n").unwrap();
        std::fs::write(paths.radio_cache.join("trance-001.wav"), b"RIFF").unwrap();
        std::fs::write(&paths.crash_log, b"diagnostic").unwrap();

        let shown = forget_tool(&json!({}), &paths);
        let text = shown["content"][0]["text"].as_str().unwrap_or_default();
        assert!(text.contains("1 rooms entered") || text.contains("1 wins"));
        assert!(text.contains("Cairn"));
        assert!(text.contains("journal"));
        assert!(text.contains("radio cache"));
        assert!(text.contains("crash log"));
        assert!(!text.contains("Nothing else is kept"));
        assert_eq!(shown["structuredContent"]["action"], "preview");
        assert_eq!(shown["structuredContent"]["confirm_required"], true);
        assert_eq!(
            shown["structuredContent"]["remembered"]["journey"]["rooms_entered"],
            1
        );
        assert_eq!(
            shown["structuredContent"]["remembered"]["scores"]["entries"],
            1
        );
        assert_eq!(
            shown["structuredContent"]["remembered"]["cairn"]["local_plaintext_drafts"],
            1
        );
        assert_eq!(
            shown["structuredContent"]["remembered"]["journal"]["exists"],
            true
        );
        assert_eq!(
            shown["structuredContent"]["remembered"]["radio_cache"]["files"],
            1
        );
        assert!(paths.journey.exists(), "nothing was erased without consent");

        let scores_requested = forget_tool(&json!({"scores": true}), &paths);
        assert_eq!(
            scores_requested["structuredContent"]["requested_scores_erasure"],
            true
        );
        assert!(paths.journey.exists(), "a preview must retain the journey");
        assert!(paths.scores.exists(), "a preview must retain scores");

        let erased = forget_tool(&json!({"confirm": true}), &paths);
        assert_eq!(erased["structuredContent"]["action"], "erase");
        assert_eq!(erased["structuredContent"]["journey_erased"], true);
        assert_eq!(erased["structuredContent"]["scores_erased"], false);
        assert_eq!(erased["structuredContent"]["scores_preserved"], true);
        assert!(!paths.journey.exists());
        assert!(paths.scores.exists());
        assert!(paths.cairn.exists());
        assert!(paths.journal.exists());

        let journal_erased = forget_tool(&json!({"confirm": true, "journal": true}), &paths);
        assert_eq!(journal_erased["structuredContent"]["journal_erased"], true);
        assert!(!paths.journal.exists());
        assert!(paths.scores.exists());
        assert!(paths.cairn.exists());
        assert!(paths.radio_cache.exists());
        assert!(paths.crash_log.exists());

        std::fs::write(&paths.journal, "replacement opt-in experience\n").unwrap();

        let erased_all = forget_tool(&json!({"confirm": true, "all_local": true}), &paths);
        assert_eq!(erased_all["structuredContent"]["all_local"], true);
        assert_eq!(
            erased_all["structuredContent"]["residue"]["managed_store_residue"],
            0
        );
        for path in [
            &paths.journey,
            &paths.scores,
            &paths.cairn,
            &paths.journal,
            &paths.radio_cache,
            &paths.crash_log,
        ] {
            assert!(!path.exists(), "{} must be absent", path.display());
        }

        let unremovable = root.join("unremovable");
        let lock = std::path::PathBuf::from(format!("{}.lock", unremovable.display()));
        let _ = std::fs::remove_file(&lock);
        let _ = std::fs::remove_dir(&unremovable);
        std::fs::create_dir(&unremovable).unwrap();
        let failed_paths = LocalStatePaths {
            journey: unremovable.clone(),
            scores: paths.scores.clone(),
            cairn: paths.cairn.clone(),
            journal: paths.journal.clone(),
            radio_cache: paths.radio_cache.clone(),
            protected_radio_source: paths.protected_radio_source.clone(),
            crash_log: paths.crash_log.clone(),
        };
        let failed = forget_tool(&json!({"confirm": true}), &failed_paths);
        assert_eq!(failed["isError"], true);
        assert!(
            unremovable.is_dir(),
            "failed erasure must not claim success"
        );
        std::fs::remove_dir(&unremovable).unwrap();
        let _ = std::fs::remove_file(lock);
        std::fs::remove_dir(&root).unwrap();
    }
}
