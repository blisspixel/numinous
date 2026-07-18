//! Public-process smoke coverage for command parsing and its explicit stack.

use std::process::Command;

#[test]
fn public_binary_crosses_the_explicit_command_stack() {
    let state_root =
        std::env::temp_dir().join(format!("numinous-cli-smoke-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_numinous"))
        .args(["sonify", "--help"])
        .env("NUMINOUS_JOURNEY", state_root.join("journey.txt"))
        .env("NUMINOUS_SCORES", state_root.join("scores.txt"))
        .env("NUMINOUS_CAIRN", state_root.join("cairn.txt"))
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
fn public_forget_previews_fails_closed_and_erases_isolated_state() {
    let root =
        std::env::temp_dir().join(format!("numinous-cli-forget-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let journey = root.join("journey.txt");
    let scores = root.join("scores.txt");
    let cairn = root.join("cairn.txt");
    let radio = home.join(".numinous-radio");
    let crash = home.join(".numinous-crash.log");
    std::fs::create_dir_all(&journey).expect("unexpected Journey directory");
    std::fs::create_dir_all(&radio).expect("radio fixture");
    std::fs::write(&scores, b"50\tmunch seed:1 board:0\n").expect("score fixture");
    std::fs::write(&cairn, b"Ada\ttruth survives inspection\n").expect("Cairn fixture");
    std::fs::write(radio.join("trance-001.wav"), b"RIFF").expect("radio fixture");
    std::fs::write(&crash, b"isolated diagnostic").expect("crash fixture");

    let command = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_numinous"))
            .args(args)
            .env("NUMINOUS_JOURNEY", &journey)
            .env("NUMINOUS_SCORES", &scores)
            .env("NUMINOUS_CAIRN", &cairn)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .output()
            .expect("launch public forget command")
    };

    let preview = command(&["forget"]);
    assert!(preview.status.success());
    let preview_text = String::from_utf8(preview.stdout).expect("preview is UTF-8");
    assert!(preview_text.contains("unexpected non-file object"));
    assert!(preview_text.contains("journey.txt"));
    assert!(preview_text.contains(".numinous-radio"));
    assert!(journey.is_dir(), "preview is non-destructive");

    let blocked = command(&["forget", "--confirm", "--all-local"]);
    assert!(!blocked.status.success());
    let blocked_text = String::from_utf8(blocked.stderr).expect("failure is UTF-8");
    assert!(blocked_text.contains("Erasure stopped at journey"));
    assert!(scores.is_file(), "global preflight preserves later stores");

    std::fs::remove_dir(&journey).expect("replace invalid Journey object");
    std::fs::write(&journey, b"visited lorenz\nplays 1\n").expect("Journey fixture");
    let erased = command(&["forget", "--confirm", "--all-local"]);
    assert!(
        erased.status.success(),
        "complete erasure failed: {}",
        String::from_utf8_lossy(&erased.stderr)
    );
    let erased_text = String::from_utf8(erased.stdout).expect("receipt is UTF-8");
    assert!(erased_text.contains("0 managed stores and 0 known bytes remain"));
    for path in [&journey, &scores, &cairn, &radio, &crash] {
        assert!(!path.exists(), "{} must be absent", path.display());
    }
    std::fs::remove_dir_all(root).expect("fixture cleanup");
}
