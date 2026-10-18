//! Pipe and terminal contracts for `tour`, `watch`, and `play`.
//!
//! A pipe must exit with one still, or a `--frames` recording, and must not
//! light a star. A Unix pty checks that Ctrl+C and SIGTERM leave the main
//! screen and show the cursor. Windows CI runs the pipe half; the pty half
//! runs where those signals exist.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "numinous-live-{name}-{}-{}",
        std::process::id(),
        name.len()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn numinous(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_numinous"));
    cmd.env("HOME", dir)
        .env("USERPROFILE", dir)
        .env("NUMINOUS_JOURNEY", dir.join("journey"))
        .env("NUMINOUS_SCORES", dir.join("scores"))
        .env("NUMINOUS_CAIRN", dir.join("cairn"))
        .env("NUMINOUS_JOURNAL", dir.join("journal"))
        .env("NUMINOUS_PROJECT", dir.join("project"))
        .env("NUMINOUS_PREFERENCES", dir.join("preferences"))
        .env_remove("NUMINOUS_REDUCED_MOTION")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null());
    cmd
}

fn run_piped(mut cmd: Command) -> (ExitStatus, String, Duration) {
    let start = Instant::now();
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdout = child.stdout.take().expect("stdout");
    let collected = Arc::new(Mutex::new(Vec::new()));
    let bucket = Arc::clone(&collected);
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        *bucket.lock().expect("stdout bytes") = buf;
    });
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if start.elapsed() > Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("command did not exit within 3s");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    reader.join().expect("reader");
    let bytes = collected.lock().expect("stdout").clone();
    (
        status,
        String::from_utf8_lossy(&bytes).into_owned(),
        start.elapsed(),
    )
}

fn assert_no_star(dir: &Path) {
    let path = dir.join("journey");
    if path.exists() {
        let text = std::fs::read_to_string(&path).expect("journey");
        assert!(
            !text
                .lines()
                .any(|line| line.starts_with("visited ") && line != "visited "),
            "a pipe lit a star: {text}"
        );
    }
}

#[test]
fn pipes_finish_with_one_still_and_light_no_stars() {
    let first = numinous_core::all_rooms();
    let first_title = first[0].meta().title;
    let cases = [
        (
            "tour",
            vec!["tour", "--mute", "--width", "16", "--height", "8"],
            first_title,
        ),
        (
            "watch",
            vec!["watch", "julia", "--mute", "--width", "16", "--height", "8"],
            "Julia Set",
        ),
        (
            "play",
            vec!["play", "julia", "--width", "16", "--height", "8"],
            "Julia Set",
        ),
    ];
    for (name, args, title) in cases {
        let dir = scratch(name);
        let mut cmd = numinous(&dir);
        cmd.args(&args);
        let (status, output, elapsed) = run_piped(cmd);
        assert!(status.success(), "{name} exited {status}");
        assert!(elapsed < Duration::from_secs(3), "{name} took {elapsed:?}");
        assert!(output.contains(title), "{name} missed its title:\n{output}");
        assert!(
            !output.contains('\u{1b}'),
            "{name} sent cursor control to a pipe:\n{output}"
        );
        assert!(!output.contains('\u{1c}'), "{name} split a single postcard");
        assert_no_star(&dir);
    }
}

#[test]
fn frames_record_a_finite_run_without_lighting_a_star() {
    let dir = scratch("frames");
    let mut cmd = numinous(&dir);
    cmd.args([
        "watch", "julia", "--mute", "--width", "16", "--height", "8", "--frames", "4",
    ]);
    let (status, output, _) = run_piped(cmd);
    assert!(status.success(), "{status}");
    assert_eq!(output.matches('\u{1c}').count(), 3, "{output}");
    assert!(!output.contains('\u{1b}'));
    assert!(output.contains("Julia Set"));
    assert_no_star(&dir);
}

#[test]
fn reduced_motion_tour_on_a_pipe_still_waits_and_lights_no_star() {
    let dir = scratch("held");
    let mut cmd = numinous(&dir);
    cmd.env("NUMINOUS_REDUCED_MOTION", "1");
    cmd.args(["tour", "--mute", "--width", "16", "--height", "8"]);
    let (status, output, _) = run_piped(cmd);
    assert!(status.success(), "{status}");
    assert!(
        output.contains("Enter for the next room, q to leave."),
        "reduced motion stopped waiting for the player:\n{output}"
    );
    assert_no_star(&dir);
}

#[cfg(unix)]
#[test]
fn ctrl_c_and_sigterm_restore_the_main_screen() {
    use std::io::Write;
    for signal in ["int", "term"] {
        let dir = scratch(signal);
        let mut cmd = portable_pty::CommandBuilder::new(env!("CARGO_BIN_EXE_numinous"));
        cmd.arg("watch");
        cmd.arg("julia");
        cmd.arg("--mute");
        cmd.arg("--width");
        cmd.arg("16");
        cmd.arg("--height");
        cmd.arg("8");
        cmd.arg("--fps");
        cmd.arg("10");
        cmd.env("HOME", dir.as_os_str());
        cmd.env("USERPROFILE", dir.as_os_str());
        cmd.env("NUMINOUS_JOURNEY", dir.join("journey").as_os_str());
        cmd.env("NUMINOUS_SCORES", dir.join("scores").as_os_str());
        cmd.env("NO_COLOR", "1");
        cmd.env_remove("NUMINOUS_REDUCED_MOTION");
        let system = portable_pty::native_pty_system();
        let pair = system
            .openpty(portable_pty::PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("pty");
        let mut child = pair.slave.spawn_command(cmd).expect("spawn on pty");
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().expect("reader");
        let mut writer = pair.master.take_writer().expect("writer");
        let collected = Arc::new(Mutex::new(Vec::new()));
        let bucket = Arc::clone(&collected);
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => bucket
                        .lock()
                        .expect("pty bytes")
                        .extend_from_slice(&buf[..n]),
                }
            }
        });
        let started = Instant::now();
        loop {
            let text =
                String::from_utf8_lossy(&collected.lock().expect("pty").clone()).into_owned();
            if text.contains("\x1b[?1049h") && text.contains("\x1b[?2026h") {
                break;
            }
            if started.elapsed() > Duration::from_secs(5) {
                let _ = child.kill();
                panic!("{signal}: live session did not start:\n{text}");
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        if signal == "int" {
            writer.write_all(&[0x03]).expect("Ctrl+C");
            writer.flush().expect("flush");
        } else {
            let pid = child.process_id().expect("pid");
            let kill = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status()
                .expect("kill");
            assert!(kill.success(), "SIGTERM was not delivered");
        }
        let stopped = Instant::now();
        loop {
            if child.try_wait().expect("wait").is_some() {
                break;
            }
            if stopped.elapsed() > Duration::from_secs(5) {
                let _ = child.kill();
                panic!("{signal}: the live view did not exit");
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        std::thread::sleep(Duration::from_millis(100));
        let text = String::from_utf8_lossy(&collected.lock().expect("pty").clone()).into_owned();
        assert!(
            text.contains("\x1b[?25h"),
            "{signal}: cursor stayed hidden:\n{text}"
        );
        assert!(
            text.contains("\x1b[?1049l"),
            "{signal}: alternate screen stayed up:\n{text}"
        );
        assert!(
            !text.contains("\x1b[2J"),
            "{signal}: a frame cleared the whole screen"
        );
        let journey = std::fs::read_to_string(dir.join("journey")).unwrap_or_default();
        assert!(
            journey.contains("julia"),
            "{signal}: a terminal watch lit no star: {journey}"
        );
    }
}
