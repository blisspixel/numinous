//! `numinous-mcp` answers `--help` and `--version` before it reads stdin.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_numinous-mcp"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    // Hold the write end open. Closing it would deliver EOF, and the server
    // loop treats EOF as a finished session, which would hide a hang.
    let _stdin = child.stdin.take().expect("stdin");
    let mut stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");
    let out_buf = Arc::new(Mutex::new(Vec::new()));
    let err_buf = Arc::new(Mutex::new(Vec::new()));
    let out_bucket = Arc::clone(&out_buf);
    let err_bucket = Arc::clone(&err_buf);
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        *out_bucket.lock().expect("stdout") = buf;
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        *err_bucket.lock().expect("stderr") = buf;
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if started.elapsed() > Duration::from_secs(2) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{args:?} did not exit before reading stdin");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    drop(_stdin);
    out_thread.join().expect("stdout thread");
    err_thread.join().expect("stderr thread");
    let stdout = String::from_utf8(out_buf.lock().expect("stdout").clone()).expect("utf-8");
    let stderr = String::from_utf8(err_buf.lock().expect("stderr").clone()).expect("utf-8");
    (status.code(), stdout, stderr)
}

#[test]
fn help_and_version_exit_without_reading_stdin() {
    let (code, stdout, stderr) = run(&["--help"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert!(stdout.contains("stdio MCP server"), "{stdout}");
    assert!(stdout.contains("newline-delimited JSON-RPC"), "{stdout}");
    assert!(
        stdout.contains(r#"{"mcpServers": {"numinous": {"command": "numinous-mcp"}}}"#),
        "{stdout}"
    );
    assert!(
        stdout.contains("terminal interface is numinous"),
        "{stdout}"
    );

    let (code, stdout, stderr) = run(&["-h"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stdout.contains("stdio MCP server"), "{stdout}");

    let (code, stdout, stderr) = run(&["--version"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(
        stdout.trim(),
        format!("numinous-mcp {}", env!("CARGO_PKG_VERSION"))
    );

    let (code, stdout, stderr) = run(&["-V"]);
    assert_eq!(code, Some(0));
    assert_eq!(
        stdout.trim(),
        format!("numinous-mcp {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn an_unknown_argument_exits_2_with_one_line() {
    let (code, stdout, stderr) = run(&["--nope"]);
    assert_eq!(code, Some(2));
    assert!(stdout.is_empty(), "{stdout}");
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines, ["numinous-mcp: unknown argument '--nope'"]);
}

#[test]
fn closed_stdin_without_arguments_stays_quiet() {
    let output = Command::new(env!("CARGO_BIN_EXE_numinous-mcp"))
        .stdin(Stdio::null())
        .output()
        .expect("spawn");
    assert!(output.status.success(), "{:?}", output.status.code());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
