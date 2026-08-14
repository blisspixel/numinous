//! CLI presentation for player-owned local-state inventory and erasure.
//!
//! The core owns path resolution, locking, inspection, and mutation. This
//! module owns only terminal prose and the explicit confirmation boundary.

use numinous_core::{
    LocalFileInventory, LocalStateEraseSelection, LocalStateInventory, LocalStatePaths,
    erase_local_state, inspect_local_state,
};

fn managed_file_line(label: &str, file: &LocalFileInventory, detail: &str) -> String {
    let path = super::terminal_safe(&file.path.to_string_lossy());
    let state = if !file.exists && file.sidecar_files == 0 {
        "absent".to_string()
    } else if !file.exists {
        format!(
            "primary file absent, {} adjacent persistence files totaling {} bytes{}",
            file.sidecar_files,
            file.sidecar_bytes,
            if file.sidecar_scan_capped {
                ", sidecar scan capped"
            } else {
                ""
            }
        )
    } else if file.managed_file {
        format!(
            "{} bytes, {} adjacent persistence files totaling {} bytes{}",
            file.bytes,
            file.sidecar_files,
            file.sidecar_bytes,
            if file.sidecar_scan_capped {
                ", sidecar scan capped"
            } else {
                ""
            }
        )
    } else {
        "unexpected non-file object, erasure will fail closed".to_string()
    };
    format!("  {label:<12} {state}; {detail}; path {path}")
}

fn inventory_report(
    inventory: &LocalStateInventory,
    selection: LocalStateEraseSelection,
) -> String {
    let radio_path = super::terminal_safe(&inventory.radio_cache.path.to_string_lossy());
    let radio_state = format!(
        "{}; {} generated WAV files, {} bytes, {} unexpected entries{}, {} sidecar files, {} sidecar bytes{}",
        if inventory.radio_cache.exists {
            "directory present"
        } else {
            "directory absent"
        },
        inventory.radio_cache.files,
        inventory.radio_cache.bytes,
        inventory.radio_cache.unexpected_entries,
        if inventory.radio_cache.truncated {
            ", cache scan capped"
        } else {
            ""
        },
        inventory.radio_cache.sidecar_files,
        inventory.radio_cache.sidecar_bytes,
        if inventory.radio_cache.sidecar_scan_capped {
            ", sidecar scan capped"
        } else {
            ""
        }
    );
    let selected = [
        selection.journey.then_some("journey"),
        selection.scores.then_some("scores"),
        selection.cairn.then_some("Cairn drafts"),
        selection.journal.then_some("experience journal"),
        selection.radio_cache.then_some("radio cache"),
        selection.crash_log.then_some("crash log"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    format!(
        "Numinous-managed local state:\n\n{}\n{}\n{}\n{}\n  {:<12} {}; path {}\n{}\n\nSelected for confirmed erasure: {}.\nNo state was erased by this preview. Use `numinous forget --confirm` for the Journey, add individual flags for other stores, or use `numinous forget --confirm --all-local` for every store above.\n\nNot inventoried or erased: user-selected exports such as PNG, APNG, WAV, and `.num` files; installed application files; the Rust toolchain; and bundled canonical Cairn stones. Local Cairn drafts store author and message as bounded plaintext until erased or separately submitted.",
        managed_file_line(
            "journey",
            &inventory.journey.file,
            &format!(
                "{} rooms, {} wins, {} plays, {} secrets",
                inventory.journey.rooms_entered,
                inventory.journey.wins,
                inventory.journey.plays,
                inventory.journey.secrets_heard
            )
        ),
        managed_file_line(
            "scores",
            &inventory.scores.file,
            &format!("{} entries", inventory.scores.entries)
        ),
        managed_file_line(
            "Cairn",
            &inventory.cairn.file,
            &format!("{} local plaintext drafts", inventory.cairn.local_drafts)
        ),
        managed_file_line("journal", &inventory.journal, "opt-in experience records"),
        "radio cache",
        radio_state,
        radio_path,
        managed_file_line("crash log", &inventory.crash_log, "App diagnostic"),
        selected
    )
}

pub(super) fn forget_local_state(
    paths: &LocalStatePaths,
    confirm: bool,
    selection: LocalStateEraseSelection,
    all_local: bool,
) -> Result<String, String> {
    let before = inspect_local_state(paths)
        .map_err(|error| format!("Could not inventory local state: {error}."))?;
    if !confirm {
        return Ok(inventory_report(&before, selection));
    }

    let after = erase_local_state(paths, selection).map_err(|error| {
        let residue = inspect_local_state(paths)
            .map(|inventory| {
                format!(
                    " {} managed stores and {} known bytes remain.",
                    inventory.managed_residue_count(),
                    inventory.total_managed_bytes()
                )
            })
            .unwrap_or_else(|_| " Residue could not be inventoried.".to_string());
        format!("Erasure stopped at {}: {}.{residue}", error.target(), error)
    })?;
    if all_local && after.managed_residue_count() != 0 {
        return Err(format!(
            "Complete erasure could not be verified: {} managed stores and {} known bytes remain.",
            after.managed_residue_count(),
            after.total_managed_bytes()
        ));
    }
    Ok(format!(
        "Selected local state erased and verified. {} managed stores and {} known bytes remain. Bundled canonical Cairn stones and user-managed exports were not changed.",
        after.managed_residue_count(),
        after.total_managed_bytes()
    ))
}

#[cfg(test)]
mod tests {
    use super::forget_local_state;
    use numinous_core::{LocalStateEraseSelection, LocalStatePaths, inspect_local_state};

    #[test]
    fn inventories_and_completely_erases_managed_local_state() {
        let root = std::env::temp_dir().join(format!(
            "numinous_cli_forget_complete_{}_{}",
            std::process::id(),
            crate::pick_day()
        ));
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
        std::fs::create_dir_all(&paths.radio_cache).expect("fixture directory");
        std::fs::write(&paths.journey, b"visited lorenz\nwins 1\nplays 2\n")
            .expect("journey fixture");
        std::fs::write(&paths.scores, b"50\tmunch seed:1 board:0\n").expect("score fixture");
        std::fs::write(&paths.cairn, b"Ada\tproof is a program\n").expect("Cairn fixture");
        std::fs::write(&paths.journal, b"an opt-in experience\n").expect("journal fixture");
        std::fs::write(paths.radio_cache.join("trance-001.wav"), b"RIFF").expect("radio fixture");
        let crash_temp = root.join(".crash.log.999.1.tmp");
        std::fs::write(&crash_temp, b"orphan diagnostic temp").expect("crash temp fixture");

        let preview = forget_local_state(&paths, false, LocalStateEraseSelection::complete(), true)
            .expect("preview");
        for expected in [
            "journey",
            "scores",
            "Cairn",
            "journal",
            "radio cache",
            "crash log",
            "local plaintext drafts",
            "user-selected exports",
        ] {
            assert!(preview.contains(expected), "missing {expected}: {preview}");
        }
        assert!(!preview.contains("Nothing else is kept"));
        assert!(
            preview.contains("primary file absent"),
            "sidecar-only state must name the missing primary: {preview}"
        );
        assert!(paths.journey.exists(), "preview is non-destructive");

        let erased = forget_local_state(&paths, true, LocalStateEraseSelection::complete(), true)
            .expect("complete erasure");
        assert!(erased.contains("0 managed stores and 0 known bytes remain"));
        assert_eq!(
            inspect_local_state(&paths)
                .expect("post-erasure inventory")
                .managed_residue_count(),
            0
        );
        assert!(!crash_temp.exists(), "crash temp must be erased");
        std::fs::remove_dir(&root).expect("fixture root cleanup");
    }
}
