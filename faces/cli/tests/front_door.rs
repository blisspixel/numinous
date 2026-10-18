//! The front door, as a player runs it: wrapped help and a completion script.

use std::process::Command;

fn numinous(args: &[&str]) -> Command {
    let root = std::env::temp_dir().join(format!("numinous-front-door-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let mut command = Command::new(env!("CARGO_BIN_EXE_numinous"));
    command
        .args(args)
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_JOURNEY", root.join("journey"))
        .env("NUMINOUS_SCORES", root.join("scores"))
        .env("NO_COLOR", "1")
        .env_remove("TERM");
    command
}

#[test]
fn columns_60_help_wraps_and_names_the_groups_in_order() {
    let output = numinous(&["--help"])
        .env("COLUMNS", "60")
        .output()
        .expect("help");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8(output.stdout).expect("utf-8");
    let mut last = 0;
    for heading in [
        "Start here:",
        "Play:",
        "Games:",
        "Create:",
        "Study:",
        "Your journey:",
        "Maintain:",
    ] {
        let at = help
            .lines()
            .position(|line| line == heading)
            .unwrap_or_else(|| panic!("missing {heading}\n{help}"));
        assert!(at > last, "{heading} is out of order");
        last = at;
    }
    let wide: Vec<&str> = help
        .lines()
        .filter(|line| line.chars().count() > 60)
        .collect();
    assert!(wide.is_empty(), "wider than 60: {wide:?}");
}

#[test]
fn dumb_watch_is_ascii_and_a_normal_terminal_keeps_truecolor() {
    let root = std::env::temp_dir().join(format!(
        "numinous-front-door-picture-{}",
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&root);
    let mut dumb = Command::new(env!("CARGO_BIN_EXE_numinous"));
    let dumb = dumb
        .args([
            "watch",
            "times-tables",
            "--mute",
            "--width",
            "16",
            "--height",
            "8",
        ])
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_JOURNEY", root.join("journey"))
        .env("NUMINOUS_SCORES", root.join("scores"))
        .env("TERM", "dumb")
        .env_remove("NO_COLOR")
        .output()
        .expect("dumb watch");
    assert!(
        dumb.status.success(),
        "{}",
        String::from_utf8_lossy(&dumb.stderr)
    );
    let picture = String::from_utf8(dumb.stdout).expect("utf-8");
    assert!(picture.contains("Times Tables"), "{picture}");
    assert!(!picture.contains("\u{1b}["), "dumb watch emitted an escape");
    assert!(
        picture
            .chars()
            .all(|glyph| !('\u{2580}'..='\u{259F}').contains(&glyph)),
        "dumb watch drew block art"
    );

    let mut color = Command::new(env!("CARGO_BIN_EXE_numinous"));
    let color = color
        .args([
            "render",
            "times-tables",
            "--color",
            "--width",
            "8",
            "--height",
            "4",
        ])
        .env("HOME", &root)
        .env("USERPROFILE", &root)
        .env("NUMINOUS_JOURNEY", root.join("journey-color"))
        .env("NUMINOUS_SCORES", root.join("scores-color"))
        .env("TERM", "xterm-256color")
        .env_remove("NO_COLOR")
        .output()
        .expect("color render");
    assert!(
        color.status.success(),
        "{}",
        String::from_utf8_lossy(&color.stderr)
    );
    let colored = String::from_utf8(color.stdout).expect("utf-8");
    assert!(
        colored.contains("\u{1b}[38;2;"),
        "a normal terminal lost truecolor"
    );
}

#[cfg(unix)]
#[test]
fn completion_bash_passes_bash_n() {
    use std::io::Write;
    use std::process::Stdio;
    let output = numinous(&["completion", "bash"])
        .output()
        .expect("completion");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut child = Command::new("bash")
        .arg("-n")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&output.stdout)
        .expect("write script");
    let checked = child.wait_with_output().expect("bash -n");
    assert!(
        checked.status.success(),
        "bash -n rejected the completion script:\n{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
