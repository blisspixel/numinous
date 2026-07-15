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
