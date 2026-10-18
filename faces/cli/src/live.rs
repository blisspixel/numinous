//! Owning a terminal for `tour`, `watch`, and `play`.
//!
//! A pipe is not a terminal. Those commands then print one still postcard
//! and a title, or a `--frames N` recording, and they exit. That path does
//! not move the cursor and does not light a star.
//!
//! A terminal is entered once: the alternate screen, a hidden cursor, and
//! each frame wrapped in synchronized output. The cursor and the main screen
//! come back on the way out, including after Ctrl+C, SIGTERM, and a panic.
//! Release builds abort on panic, and the panic hook still runs, so the hook
//! is what restores the screen there. A debug build also restores on unwind
//! when [`LiveSession`] drops.

use std::io::{self, BufRead, Write};
#[cfg(unix)]
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once, OnceLock};
use std::time::Duration;

/// Enter the alternate screen and hide the cursor.
pub(crate) const ENTER_LIVE: &str = "\x1b[?1049h\x1b[?25l";
/// Show the cursor and leave the alternate screen.
pub(crate) const LEAVE_LIVE: &str = "\x1b[?25h\x1b[?1049l";

const SYNC_BEGIN: &str = "\x1b[?2026h";
const SYNC_END: &str = "\x1b[?2026l";

/// Splits recorded frames in a pipe. It is not a cursor sequence, so a file
/// can be cut into frames without terminal control bytes.
pub(crate) const FRAME_SEPARATOR: &str = "\n\u{1c}\n";

static LIVE: AtomicBool = AtomicBool::new(false);
static INTERRUPT: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn interrupt_flag() -> Arc<AtomicBool> {
    Arc::clone(INTERRUPT.get_or_init(|| Arc::new(AtomicBool::new(false))))
}

/// Whether Ctrl+C or SIGTERM has asked the live view to stop.
pub(crate) fn interrupted() -> bool {
    INTERRUPT
        .get()
        .is_some_and(|flag| flag.load(Ordering::SeqCst))
}

#[cfg(test)]
thread_local! {
    static RESTORE_SINK: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}

/// Leave the alternate screen and show the cursor, once.
///
/// A second call is a no-op, so the signal path and [`LiveSession`]'s drop
/// can both try without painting the restore sequence twice.
pub(crate) fn restore_live_terminal() {
    let bytes = LEAVE_LIVE.as_bytes();
    #[cfg(test)]
    {
        // Unit tests share this process. Only a thread that armed the capture
        // sink may clear the flag, or a tour test running beside the panic
        // test would steal the restore and write it at the test runner.
        RESTORE_SINK.with(|sink| {
            let mut slot = sink.borrow_mut();
            if let Some(buf) = slot.as_mut()
                && LIVE.swap(false, Ordering::SeqCst)
            {
                buf.extend_from_slice(bytes);
            }
        });
        // No sink on this thread: leave the flag for the thread that armed it.
    }
    #[cfg(not(test))]
    {
        if !LIVE.swap(false, Ordering::SeqCst) {
            return;
        }
        let mut out = io::stdout();
        let _ = out.write_all(bytes);
        let _ = out.flush();
    }
}

fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_live_terminal();
            previous(info);
        }));
    });
}

fn arm_interrupts() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let flag = interrupt_flag();
        #[cfg(unix)]
        {
            let kept = signal_ids();
            let mut stored = kept.lock().unwrap_or_else(|poison| poison.into_inner());
            if let Ok(id) =
                signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&flag))
            {
                stored.push(id);
            }
            if let Ok(id) = signal_hook::flag::register(signal_hook::consts::SIGINT, flag) {
                stored.push(id);
            }
        }
        #[cfg(not(unix))]
        {
            let _ = ctrlc::set_handler(move || {
                flag.store(true, Ordering::SeqCst);
            });
        }
    });
}

#[cfg(unix)]
fn signal_ids() -> &'static Mutex<Vec<signal_hook::SigId>> {
    static KEPT: OnceLock<Mutex<Vec<signal_hook::SigId>>> = OnceLock::new();
    KEPT.get_or_init(|| Mutex::new(Vec::new()))
}

/// The alternate screen and the hidden cursor for one live view.
pub(crate) struct LiveSession;

impl LiveSession {
    /// Enter the alternate screen. Dropping the session, or a panic, leaves it.
    pub(crate) fn enter() -> Self {
        install_panic_hook();
        arm_interrupts();
        interrupt_flag().store(false, Ordering::SeqCst);
        LIVE.store(true, Ordering::SeqCst);
        let mut out = io::stdout();
        let _ = write!(out, "{ENTER_LIVE}");
        let _ = out.flush();
        Self
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        restore_live_terminal();
    }
}

/// One synchronized terminal frame: home, the picture, clear to the end.
///
/// A leading cursor-home on `body` is not repeated. The frame never uses a
/// full-screen clear.
pub(crate) fn live_frame(body: &str) -> String {
    let body = body.strip_prefix("\x1b[H").unwrap_or(body);
    debug_assert!(!body.contains("\x1b[2J"), "a live frame repaints in place");
    format!("{SYNC_BEGIN}\x1b[H{body}\x1b[J{SYNC_END}")
}

/// Whether `text` moves the cursor or switches a terminal mode.
///
/// SGR sequences, which end in `m`, are color and weight, not cursor control.
/// The live loops call this only from tests; production frames are built
/// without cursor bytes on a pipe and wrapped on a terminal.
#[cfg(test)]
pub(crate) fn has_cursor_control(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) != Some(&b'[') {
            return true;
        }
        let mut end = index + 2;
        if bytes.get(end) == Some(&b'?') {
            return true;
        }
        while end < bytes.len()
            && (bytes[end].is_ascii_digit() || matches!(bytes[end], b';' | b':'))
        {
            end += 1;
        }
        match bytes.get(end) {
            Some(b'm') => index = end + 1,
            _ => return true,
        }
    }
    false
}

/// `--frames` as a count, or the omission that means "until stopped".
///
/// Zero is a refusal: a recording of nothing would look like a success.
pub(crate) fn frame_count(frames: Option<u64>) -> Result<Option<u64>, &'static str> {
    match frames {
        Some(0) => Err("--frames needs at least one frame.\n"),
        other => Ok(other),
    }
}

/// Join recorded frames with [`FRAME_SEPARATOR`]. One frame stays whole.
pub(crate) fn join_recorded(frames: impl IntoIterator<Item = String>) -> String {
    let mut out = String::new();
    for (index, frame) in frames.into_iter().enumerate() {
        if index > 0 {
            out.push_str(FRAME_SEPARATOR);
        }
        out.push_str(&frame);
    }
    out
}

/// Sleep in short slices so a stop request lands inside one frame interval.
pub(crate) fn sleep_unless_interrupted(total: Duration) {
    let step = Duration::from_millis(30);
    let start = std::time::Instant::now();
    while start.elapsed() < total {
        if interrupted() {
            return;
        }
        let remaining = total.saturating_sub(start.elapsed());
        std::thread::sleep(step.min(remaining));
    }
}

/// Read one stdin line, returning `None` on end of input or a stop request.
///
/// The read itself blocks, so a second thread waits on it and the caller
/// polls the stop flag. A stop leaves that thread blocked until the process
/// exits, which is the end of a live view.
pub(crate) fn read_stdin_line() -> Option<String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut line = String::new();
        let read = input.read_line(&mut line).unwrap_or(0);
        let _ = sender.send(if read == 0 { None } else { Some(line) });
    });
    loop {
        if interrupted() {
            return None;
        }
        match receiver.recv_timeout(Duration::from_millis(30)) {
            Ok(line) => return line,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
        }
    }
}

/// Print the epilogue on the main screen, after the alternate screen is gone.
pub(crate) fn end_live_view(session: LiveSession, epilogue: &str) -> std::process::ExitCode {
    drop(session);
    println!("{epilogue}");
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        ENTER_LIVE, FRAME_SEPARATOR, LEAVE_LIVE, LIVE, frame_count, has_cursor_control,
        install_panic_hook, live_frame, restore_live_terminal,
    };
    use std::sync::atomic::Ordering;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn capture_restore() -> String {
        super::RESTORE_SINK.with(|sink| {
            *sink.borrow_mut() = Some(Vec::new());
        });
        LIVE.store(true, Ordering::SeqCst);
        restore_live_terminal();
        let bytes = super::RESTORE_SINK.with(|sink| sink.borrow_mut().take().unwrap_or_default());
        String::from_utf8(bytes).expect("restore bytes are ascii")
    }

    #[test]
    fn a_live_frame_is_synchronized_and_never_clears_the_whole_screen() {
        let frame = live_frame("Julia Set\n");
        assert!(frame.starts_with("\x1b[?2026h\x1b[H"), "{frame:?}");
        assert!(frame.ends_with("\x1b[J\x1b[?2026l"), "{frame:?}");
        assert!(!frame.contains("\x1b[2J"));
        assert!(frame.contains("Julia Set"));
        // A body that already homed is not homed twice.
        let once = live_frame("\x1b[Halready home\n");
        assert_eq!(once.matches("\x1b[H").count(), 1, "{once:?}");
    }

    #[test]
    fn cursor_control_is_not_color() {
        assert!(has_cursor_control(ENTER_LIVE));
        assert!(has_cursor_control(LEAVE_LIVE));
        assert!(has_cursor_control("\x1b[H"));
        assert!(has_cursor_control("\x1b[2J"));
        assert!(has_cursor_control("\x1b[J"));
        assert!(has_cursor_control("\x1b[K"));
        assert!(has_cursor_control(&live_frame("picture\n")));
        assert!(!has_cursor_control("Julia Set\n"));
        assert!(!has_cursor_control("\x1b[38;2;1;2;3m#\x1b[0m\n"));
        assert!(!has_cursor_control("\x1b[1mBold\x1b[0m\n"));
        assert!(!has_cursor_control(FRAME_SEPARATOR));
    }

    #[test]
    fn zero_frames_is_a_refusal_and_a_second_restore_is_empty() {
        assert!(frame_count(Some(0)).is_err());
        assert_eq!(frame_count(None).expect("omitted"), None);
        assert_eq!(frame_count(Some(4)).expect("count"), Some(4));
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let restored = capture_restore();
        assert!(restored.contains("\x1b[?25h"), "{restored:?}");
        assert!(restored.contains("\x1b[?1049l"), "{restored:?}");
        assert!(!LIVE.load(Ordering::SeqCst));
        super::RESTORE_SINK.with(|sink| {
            *sink.borrow_mut() = Some(Vec::new());
        });
        restore_live_terminal();
        let second = super::RESTORE_SINK.with(|sink| sink.borrow_mut().take().unwrap_or_default());
        assert!(second.is_empty(), "a second restore must not repeat");
    }

    #[test]
    fn a_panic_restores_the_terminal_before_unwinding() {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        super::RESTORE_SINK.with(|sink| {
            *sink.borrow_mut() = Some(Vec::new());
        });
        LIVE.store(true, Ordering::SeqCst);
        install_panic_hook();
        let panicked = std::panic::catch_unwind(|| panic!("live view"));
        let bytes = super::RESTORE_SINK.with(|sink| sink.borrow_mut().take().unwrap_or_default());
        let restored = String::from_utf8(bytes).expect("restore bytes");
        assert!(panicked.is_err());
        assert!(
            restored.contains(LEAVE_LIVE),
            "panic left the alternate screen up: {restored:?}"
        );
        assert!(!LIVE.load(Ordering::SeqCst));
    }
}
