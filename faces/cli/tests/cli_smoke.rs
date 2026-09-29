//! Public-process smoke coverage for command parsing and its explicit stack.

use std::process::Command;

fn isolated_command(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(args)
        .env("NUMINOUS_JOURNEY", root.join("journey.txt"))
        .env("NUMINOUS_SCORES", root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", root.join("cairn.txt"))
        .env("NUMINOUS_PREFERENCES", root.join("preferences.txt"))
        .output()
        .expect("launch the public CLI binary")
}

#[test]
fn public_binary_crosses_the_explicit_command_stack() {
    let state_root =
        std::env::temp_dir().join(format!("numinous-cli-smoke-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["sonify", "--help"])
        .env("NUMINOUS_JOURNEY", state_root.join("journey.txt"))
        .env("NUMINOUS_SCORES", state_root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", state_root.join("cairn.txt"))
        .env("NUMINOUS_PREFERENCES", state_root.join("preferences.txt"))
        .output()
        .expect("launch the public CLI binary");

    assert!(
        output.status.success(),
        "public help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("help is UTF-8");
    assert!(stdout.contains("--layer <LAYER>"));
    assert!(stdout.contains("room-bed"));
    assert!(stdout.contains("--variation <VARIATION>"));
    assert!(!state_root.exists(), "help must not create player state");
}

#[test]
fn public_describe_is_safe_and_reveal_is_earned() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-reveal-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);

    let described = isolated_command(&root, &["describe", "kepler-laws"]);
    assert!(described.status.success());
    let described = String::from_utf8(described.stdout).expect("description is UTF-8");
    assert!(described.contains("Kepler Areas"));
    assert!(described.contains("Play: numinous render kepler-laws"));
    assert!(!described.contains("Reveal:"));
    assert!(!described.contains("second law"));

    let early = isolated_command(&root, &["reveal", "lissajous"]);
    assert!(!early.status.success());
    let early = String::from_utf8(early.stderr).expect("early refusal is UTF-8");
    assert!(early.contains("Render or play the room once, then ask again"));

    let played = isolated_command(
        &root,
        &["render", "lissajous", "--width", "24", "--height", "12"],
    );
    assert!(played.status.success());
    let revealed = isolated_command(&root, &["reveal", "lissajous"]);
    assert!(revealed.status.success());
    let revealed = String::from_utf8(revealed.stdout).expect("reveal is UTF-8");
    assert!(revealed.contains("rational frequency ratio"));

    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_journey_override_names_the_file_contract() {
    let root = std::env::temp_dir().join(format!(
        "numinous-cli-directory-contract-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("journey.txt")).expect("Journey directory fixture");

    let output = isolated_command(&root, &["journey"]);
    let stderr = String::from_utf8(output.stderr).expect("diagnostic is UTF-8");
    assert!(stderr.contains("NUMINOUS_JOURNEY must name a file"));
    assert!(stderr.contains("is a directory"));

    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_forget_previews_fails_closed_and_erases_isolated_state() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-forget-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let journey = root.join("journey.txt");
    let scores = root.join("scores.txt");
    let cairn = root.join("cairn.txt");
    let journal = home.join(".numinous-journal");
    let preferences = home.join(".numinous-preferences");
    let radio = home.join(".numinous-radio");
    let crash = home.join(".numinous-crash.log");
    std::fs::create_dir_all(&journey).expect("unexpected Journey directory");
    std::fs::create_dir_all(&radio).expect("radio fixture");
    std::fs::write(&scores, b"50\tmunch seed:1 board:0\n").expect("score fixture");
    std::fs::write(&cairn, b"Ada\ttruth survives inspection\n").expect("Cairn fixture");
    std::fs::create_dir_all(&home).expect("state home fixture");
    std::fs::write(&journal, b"NUMINOUS_JOURNAL\t2\n").expect("journal fixture");
    std::fs::write(
        &preferences,
        numinous_core::AppPreferences::default().to_text(),
    )
    .expect("preferences fixture");
    std::fs::write(radio.join("trance-001.wav"), b"RIFF").expect("radio fixture");
    std::fs::write(&crash, b"isolated diagnostic").expect("crash fixture");

    let command = |args: &[&str], protected_radio: Option<&std::path::Path>| {
        let mut process = Command::new(env!("CARGO_BIN_EXE_numinous"));
        process
            .args(args)
            .env("NUMINOUS_JOURNEY", &journey)
            .env("NUMINOUS_SCORES", &scores)
            .env("NUMINOUS_CAIRN", &cairn)
            .env("NUMINOUS_PREFERENCES", &preferences)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env_remove("NUMINOUS_RADIO");
        if let Some(path) = protected_radio {
            process.env("NUMINOUS_RADIO", path);
        }
        process.output().expect("launch public forget command")
    };

    let preview = command(&["forget"], None);
    assert!(preview.status.success());
    let preview_text = String::from_utf8(preview.stdout).expect("preview is UTF-8");
    assert!(preview_text.contains("unexpected non-file object"));
    assert!(preview_text.contains("journey.txt"));
    assert!(preview_text.contains(".numinous-journal"));
    assert!(preview_text.contains(".numinous-radio"));
    assert!(journey.is_dir(), "preview is non-destructive");

    let blocked = command(&["forget", "--confirm", "--all-local"], None);
    assert!(!blocked.status.success());
    let blocked_text = String::from_utf8(blocked.stderr).expect("failure is UTF-8");
    assert!(blocked_text.contains("Erasure stopped at journey"));
    assert!(scores.is_file(), "global preflight preserves later stores");

    std::fs::remove_dir(&journey).expect("replace invalid Journey object");
    std::fs::write(&journey, b"visited lorenz\nplays 1\n").expect("Journey fixture");

    let protected = command(&["forget", "--confirm", "--all-local"], Some(&radio));
    assert!(!protected.status.success());
    let protected_text = String::from_utf8(protected.stderr).expect("failure is UTF-8");
    assert!(protected_text.contains("selected radio source"));
    for path in [
        &journey,
        &scores,
        &cairn,
        &journal,
        &preferences,
        &radio,
        &crash,
    ] {
        assert!(path.exists(), "{} must be preserved", path.display());
    }

    let journal_erased = command(&["forget", "--confirm", "--journal"], None);
    assert!(journal_erased.status.success());
    assert!(!journey.exists(), "Journey is the baseline selected store");
    assert!(
        !journal.exists(),
        "explicit journal selection must erase it"
    );
    for path in [&scores, &cairn, &preferences, &radio, &crash] {
        assert!(path.exists(), "{} must be preserved", path.display());
    }

    std::fs::write(&journey, b"visited lorenz\nplays 1\n").expect("Journey replacement");
    std::fs::write(&journal, b"NUMINOUS_JOURNAL\t2\n").expect("journal replacement");
    let erased = command(&["forget", "--confirm", "--all-local"], None);
    assert!(
        erased.status.success(),
        "complete erasure failed: {}",
        String::from_utf8_lossy(&erased.stderr)
    );
    let erased_text = String::from_utf8(erased.stdout).expect("receipt is UTF-8");
    assert!(erased_text.contains("0 managed stores and 0 known bytes remain"));
    for path in [
        &journey,
        &scores,
        &cairn,
        &journal,
        &preferences,
        &radio,
        &crash,
    ] {
        assert!(!path.exists(), "{} must be absent", path.display());
    }
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_project_resume_apply_stays_inside_the_process() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-project-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    let project = root.join("project.txt");
    let journal = root.join("journal.txt");
    let journey = root.join("journey.txt");
    numinous_core::keep_project_file(
        &project,
        &numinous_core::ProjectDraft {
            recorded_at_utc: 1_700_000_000,
            question: "Which room is this?".to_string(),
            next: numinous_core::ProjectNext::PlayRoom {
                room: "lissajous".to_string(),
                phase: Some("0.25".to_string()),
            },
            rooms: vec!["lissajous".to_string()],
            evidence: Vec::new(),
            creation: None,
        },
    )
    .expect("keep");
    let project_bytes = std::fs::read(&project).expect("project bytes");
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["project", "resume", "--apply"])
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_PROJECT", &project)
        .env("NUMINOUS_JOURNAL", &journal)
        .env("NUMINOUS_JOURNEY", &journey)
        .env("NUMINOUS_SCORES", root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", root.join("cairn.txt"))
        .env("NUMINOUS_PREFERENCES", root.join("preferences.txt"))
        .env_remove("NUMINOUS_RADIO")
        .output()
        .expect("launch the public CLI binary");
    assert!(
        output.status.success(),
        "resume failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("resume is UTF-8");
    assert!(stdout.contains("Intention: Which room is this?"));
    assert!(stdout.contains("Place: lissajous t=0.25"));
    assert!(stdout.contains("Preview workspace changed: false"));
    assert!(stdout.contains("Preview journal changed: false"));
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(&project).expect("project after"),
        project_bytes
    );
    assert!(!journal.exists(), "resume must not create a journal");
    assert!(!journey.exists(), "resume must not create a journey");
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}

#[test]
fn public_project_resume_of_an_empty_chain_fails_without_creating_files() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-project-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    let project = root.join("project.txt");
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["project", "resume", "--apply"])
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_PROJECT", &project)
        .env("NUMINOUS_JOURNAL", root.join("journal.txt"))
        .env("NUMINOUS_JOURNEY", root.join("journey.txt"))
        .env("NUMINOUS_SCORES", root.join("scores.txt"))
        .env_remove("NUMINOUS_RADIO")
        .output()
        .expect("launch the public CLI binary");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("diagnostic is UTF-8");
    assert!(stderr.contains("project chain is empty"), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !project.exists(),
        "a failed resume must not create the chain"
    );
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}
