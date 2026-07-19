use crate::input_legend::InputMode;
use numinous_broadcast::{
    ConsentMachine, ControlMarker, EventEnvelope, FrameError, HandshakeResponse, PairingGate,
    PairingOffer, PairingVerdict, PublicReceiver, PublicToolEvent, ReceiveOutcome,
    configure_handshake_stream, configure_public_stream, numinous_compatibility,
    read_handshake_request, read_public_message, write_handshake_proof, write_handshake_response,
};
use numinous_core::{Raster, Surface};
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::io::{self, BufReader};
use std::net::{Ipv4Addr, Shutdown, SocketAddrV4, TcpListener, TcpStream};
use std::num::NonZeroU16;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

pub(crate) const MAX_RETAINED_EVENTS: usize = 256;
pub(crate) const MAX_RETAINED_BYTES: usize = 16 * 1_024 * 1_024;
const ACCEPT_POLL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ViewerStatus {
    Closed,
    AwaitingGuest,
    Live,
    GuestPaused,
    GuestStopped,
    PairingExpired,
    PairingRejected,
    Disconnected,
    ProtocolRejected,
}

impl ViewerStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Closed => "CLOSED",
            Self::AwaitingGuest => "WAITING FOR A CONSENTING MCP PLAYER",
            Self::Live => "LIVE",
            Self::GuestPaused => "THE MCP PLAYER PAUSED THE BROADCAST",
            Self::GuestStopped => "THE MCP PLAYER ENDED THE BROADCAST",
            Self::PairingExpired => "THE PAIRING CODE EXPIRED",
            Self::PairingRejected => "PAIRING CLOSED AFTER FAILED HANDSHAKES",
            Self::Disconnected => "THE MCP PLAYER DISCONNECTED",
            Self::ProtocolRejected => "THE PUBLIC STREAM FAILED VALIDATION",
        }
    }
}

#[derive(Debug)]
struct StoredFrame {
    sequence: u64,
    encoded: Box<[u8]>,
}

#[derive(Debug)]
struct SharedState {
    status: ViewerStatus,
    pairing_code: Option<String>,
    frames: VecDeque<StoredFrame>,
    retained_bytes: usize,
    retention_dropped: u64,
}

impl SharedState {
    fn closed() -> Self {
        Self {
            status: ViewerStatus::Closed,
            pairing_code: None,
            frames: VecDeque::new(),
            retained_bytes: 0,
            retention_dropped: 0,
        }
    }

    fn awaiting(pairing_code: String) -> Self {
        Self {
            status: ViewerStatus::AwaitingGuest,
            pairing_code: Some(pairing_code),
            frames: VecDeque::new(),
            retained_bytes: 0,
            retention_dropped: 0,
        }
    }

    fn retain(&mut self, envelope: &EventEnvelope<PublicToolEvent>) -> Result<(), ()> {
        let encoded = serde_json::to_vec(envelope).map_err(|_| ())?;
        self.retain_encoded(envelope.public_sequence, encoded.into_boxed_slice())
    }

    fn retain_encoded(&mut self, sequence: u64, encoded: Box<[u8]>) -> Result<(), ()> {
        self.retain_with_limits(sequence, encoded, MAX_RETAINED_EVENTS, MAX_RETAINED_BYTES)
    }

    fn retain_with_limits(
        &mut self,
        sequence: u64,
        encoded: Box<[u8]>,
        maximum_events: usize,
        maximum_bytes: usize,
    ) -> Result<(), ()> {
        if maximum_events == 0 || encoded.len() > maximum_bytes {
            return Err(());
        }
        while self.frames.len() >= maximum_events
            || self.retained_bytes.saturating_add(encoded.len()) > maximum_bytes
        {
            let Some(displaced) = self.frames.pop_front() else {
                return Err(());
            };
            self.retained_bytes = self.retained_bytes.saturating_sub(displaced.encoded.len());
            self.retention_dropped = self.retention_dropped.saturating_add(1);
        }
        self.retained_bytes = self.retained_bytes.saturating_add(encoded.len());
        self.frames.push_back(StoredFrame { sequence, encoded });
        Ok(())
    }

    fn clear(&mut self) {
        self.status = ViewerStatus::Closed;
        self.pairing_code = None;
        self.frames.clear();
        self.retained_bytes = 0;
        self.retention_dropped = 0;
    }
}

#[derive(Debug)]
struct WorkerControl {
    cancelled: AtomicBool,
    active_stream: Mutex<Option<TcpStream>>,
}

impl WorkerControl {
    fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            active_stream: Mutex::new(None),
        }
    }

    fn install(&self, stream: &TcpStream) -> io::Result<()> {
        let mut active_stream = lock(&self.active_stream);
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "session viewer is closing",
            ));
        }
        *active_stream = Some(stream.try_clone()?);
        Ok(())
    }

    fn clear_stream(&self) {
        *lock(&self.active_stream) = None;
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(stream) = lock(&self.active_stream).take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
struct ViewerSnapshot {
    status: ViewerStatus,
    pairing_code: Option<String>,
    event: Option<EventEnvelope<PublicToolEvent>>,
    selected_index: Option<usize>,
    event_count: usize,
    retention_dropped: u64,
    display_paused: bool,
}

/// Human-owned, read-only local MCP session viewer.
pub(crate) struct SessionViewer {
    shared: Arc<Mutex<SharedState>>,
    control: Arc<WorkerControl>,
    worker: Option<JoinHandle<()>>,
    follow_live: bool,
    selected_sequence: Option<u64>,
    result_scroll: usize,
    result_column: usize,
    cached_event: Option<(u64, EventEnvelope<PublicToolEvent>)>,
}

impl Default for SessionViewer {
    fn default() -> Self {
        Self {
            shared: Arc::new(Mutex::new(SharedState::closed())),
            control: Arc::new(WorkerControl::new()),
            worker: None,
            follow_live: true,
            selected_sequence: None,
            result_scroll: 0,
            result_column: 0,
            cached_event: None,
        }
    }
}

impl SessionViewer {
    pub(crate) fn open(&mut self) -> Result<(), ViewerOpenError> {
        self.close();
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let port = NonZeroU16::new(listener.local_addr()?.port())
            .ok_or(ViewerOpenError::InvalidLocalEndpoint)?;
        let compatibility = numinous_compatibility().map_err(|_| ViewerOpenError::Compatibility)?;
        let offer = PairingOffer::generate(port, SystemTime::now())?;
        let pairing_code = offer.display_code();
        let gate = offer.into_gate(compatibility.clone());
        let deadline = Instant::now()
            .checked_add(numinous_broadcast::PAIRING_TTL)
            .ok_or(ViewerOpenError::Clock)?;
        let shared = Arc::new(Mutex::new(SharedState::awaiting(pairing_code)));
        let control = Arc::new(WorkerControl::new());
        let worker_shared = Arc::clone(&shared);
        let worker_control = Arc::clone(&control);
        let worker = thread::Builder::new()
            .name("numinous-session-viewer".to_string())
            .spawn(move || {
                listener_worker(
                    listener,
                    gate,
                    compatibility,
                    deadline,
                    &worker_shared,
                    &worker_control,
                );
            })?;
        self.shared = shared;
        self.control = control;
        self.worker = Some(worker);
        self.follow_live = true;
        self.selected_sequence = None;
        self.result_scroll = 0;
        self.result_column = 0;
        self.cached_event = None;
        Ok(())
    }

    pub(crate) fn close(&mut self) {
        self.control.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        lock(&self.shared).clear();
        self.follow_live = true;
        self.selected_sequence = None;
        self.result_scroll = 0;
        self.result_column = 0;
        self.cached_event = None;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.worker.is_some()
    }

    #[cfg(test)]
    pub(crate) fn status(&self) -> ViewerStatus {
        lock(&self.shared).status
    }

    pub(crate) fn toggle_display_pause(&mut self) {
        if self.follow_live {
            self.follow_live = false;
            self.selected_sequence = lock(&self.shared).frames.back().map(|frame| frame.sequence);
        } else {
            self.follow_live = true;
            self.selected_sequence = None;
        }
        self.result_scroll = 0;
        self.result_column = 0;
        self.cached_event = None;
    }

    pub(crate) fn scrub(&mut self, delta: isize) {
        let shared = lock(&self.shared);
        if shared.frames.is_empty() {
            return;
        }
        let current = self.selected_sequence.and_then(|sequence| {
            shared
                .frames
                .iter()
                .position(|frame| frame.sequence == sequence)
        });
        let base = current.unwrap_or(shared.frames.len() - 1);
        let next = base
            .saturating_add_signed(delta)
            .min(shared.frames.len() - 1);
        self.selected_sequence = Some(shared.frames[next].sequence);
        self.follow_live = false;
        self.result_scroll = 0;
        self.result_column = 0;
        self.cached_event = None;
    }

    pub(crate) fn scroll_result(&mut self, delta: isize) {
        self.result_scroll = self.result_scroll.saturating_add_signed(delta);
    }

    pub(crate) fn pan_result(&mut self, delta: isize) {
        self.result_column = self.result_column.saturating_add_signed(delta);
    }

    pub(crate) fn draw(&mut self, width: usize, height: usize, input_mode: InputMode) -> Raster {
        let snapshot = self.snapshot();
        let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
        raster.clear_rows(0, height as i32);
        raster.line(0, 0, width.saturating_sub(1) as i32, 0, '-');
        raster.line(
            0,
            height.saturating_sub(1) as i32,
            width.saturating_sub(1) as i32,
            height.saturating_sub(1) as i32,
            '-',
        );
        let (lines, scale, line_step) = layout_lines(
            &snapshot,
            input_mode,
            width,
            height,
            self.result_scroll,
            self.result_column,
        );
        let line_height = line_step * scale;
        let top = ((height as i32 - lines.len() as i32 * line_height) / 2).max(4);
        let block_width = lines
            .iter()
            .map(|line| line.chars().count() as i32 * 6 * scale)
            .max()
            .unwrap_or(0);
        let left = ((width as i32 - block_width) / 2).max(10);
        for (index, line) in lines.iter().enumerate() {
            numinous_core::draw_text(
                &mut raster,
                line,
                left,
                top + index as i32 * line_height,
                scale,
                '#',
            );
        }
        raster
    }

    fn snapshot(&mut self) -> ViewerSnapshot {
        let shared = lock(&self.shared);
        let selected_index = if self.follow_live {
            shared.frames.len().checked_sub(1)
        } else {
            self.selected_sequence.and_then(|sequence| {
                shared
                    .frames
                    .iter()
                    .position(|frame| frame.sequence == sequence)
            })
        };
        let selected_index = selected_index.or_else(|| {
            if self.follow_live || self.selected_sequence.is_some() {
                (!shared.frames.is_empty()).then_some(0)
            } else {
                None
            }
        });
        let selected = selected_index.map(|index| {
            let frame = &shared.frames[index];
            (frame.sequence, frame.encoded.to_vec())
        });
        let metadata = (
            shared.status,
            shared.pairing_code.clone(),
            shared.frames.len(),
            shared.retention_dropped,
        );
        drop(shared);

        let event = selected.and_then(|(sequence, encoded)| {
            if self.cached_event.as_ref().map(|cached| cached.0) != Some(sequence) {
                let decoded = serde_json::from_slice(&encoded).ok()?;
                self.cached_event = Some((sequence, decoded));
            }
            self.cached_event.as_ref().map(|cached| cached.1.clone())
        });
        if !self.follow_live {
            self.selected_sequence = event.as_ref().map(|envelope| envelope.public_sequence);
        }
        ViewerSnapshot {
            status: metadata.0,
            pairing_code: metadata.1,
            event,
            selected_index,
            event_count: metadata.2,
            retention_dropped: metadata.3,
            display_paused: !self.follow_live,
        }
    }
}

impl Drop for SessionViewer {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Debug)]
pub(crate) enum ViewerOpenError {
    Io(io::Error),
    Pairing(numinous_broadcast::PairingError),
    InvalidLocalEndpoint,
    Compatibility,
    Clock,
}

impl From<io::Error> for ViewerOpenError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<numinous_broadcast::PairingError> for ViewerOpenError {
    fn from(error: numinous_broadcast::PairingError) -> Self {
        Self::Pairing(error)
    }
}

impl fmt::Display for ViewerOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "local viewer I/O failed: {:?}", error.kind()),
            Self::Pairing(error) => write!(formatter, "local viewer pairing failed: {error}"),
            Self::InvalidLocalEndpoint => formatter.write_str("invalid local viewer endpoint"),
            Self::Compatibility => formatter.write_str("viewer compatibility is unavailable"),
            Self::Clock => formatter.write_str("viewer pairing deadline is unavailable"),
        }
    }
}

impl Error for ViewerOpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Pairing(error) => Some(error),
            _ => None,
        }
    }
}

fn listener_worker(
    listener: TcpListener,
    mut gate: PairingGate,
    compatibility: numinous_broadcast::Compatibility,
    deadline: Instant,
    shared: &Arc<Mutex<SharedState>>,
    control: &Arc<WorkerControl>,
) {
    let mut attempts = 0_u8;
    loop {
        if control.is_cancelled() {
            return;
        }
        if Instant::now() >= deadline {
            set_terminal_status(shared, ViewerStatus::PairingExpired);
            return;
        }
        match listener.accept() {
            Ok((stream, peer)) => {
                attempts = attempts.saturating_add(1);
                if !peer.ip().is_loopback()
                    || stream.set_nonblocking(false).is_err()
                    || control.install(&stream).is_err()
                {
                    if attempts >= numinous_broadcast::MAX_HANDSHAKE_ATTEMPTS {
                        gate.revoke();
                    }
                    control.clear_stream();
                    continue;
                }
                match handshake(stream, &mut gate, &compatibility, shared, control) {
                    HandshakeOutcome::Accepted(stream, session_id, epoch) => {
                        receive_stream(stream, session_id, epoch, compatibility, shared, control);
                        control.clear_stream();
                        return;
                    }
                    HandshakeOutcome::Expired => {
                        set_terminal_status(shared, ViewerStatus::PairingExpired);
                        control.clear_stream();
                        return;
                    }
                    HandshakeOutcome::Rejected => {
                        control.clear_stream();
                        if attempts >= numinous_broadcast::MAX_HANDSHAKE_ATTEMPTS
                            || gate.is_revoked()
                        {
                            gate.revoke();
                            set_terminal_status(shared, ViewerStatus::PairingRejected);
                            return;
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_POLL);
            }
            Err(_) => {
                set_terminal_status(shared, ViewerStatus::Disconnected);
                return;
            }
        }
    }
}

enum HandshakeOutcome {
    Accepted(TcpStream, numinous_broadcast::SessionId, u64),
    Rejected,
    Expired,
}

fn handshake(
    mut stream: TcpStream,
    gate: &mut PairingGate,
    compatibility: &numinous_broadcast::Compatibility,
    shared: &Arc<Mutex<SharedState>>,
    control: &Arc<WorkerControl>,
) -> HandshakeOutcome {
    if configure_handshake_stream(&stream).is_err()
        || write_handshake_proof(&mut stream, &gate.host_proof()).is_err()
    {
        return HandshakeOutcome::Rejected;
    }
    let reader_stream = match stream.try_clone() {
        Ok(reader_stream) => reader_stream,
        Err(_) => return HandshakeOutcome::Rejected,
    };
    let request = match read_handshake_request(&mut BufReader::new(reader_stream)) {
        Ok(request) => request,
        Err(_) => {
            let _ = write_handshake_response(&mut stream, &HandshakeResponse::Rejected);
            return HandshakeOutcome::Rejected;
        }
    };
    match gate.verify(&request, SystemTime::now()) {
        PairingVerdict::Accepted { session_id } => {
            let consent = ConsentMachine::new(session_id, compatibility.clone());
            if consent.begin_awaiting().is_err() {
                return HandshakeOutcome::Rejected;
            }
            let epoch = match consent.allow() {
                Ok(epoch) => epoch,
                Err(_) => return HandshakeOutcome::Rejected,
            };
            let response = HandshakeResponse::Accepted {
                session_id,
                consent_epoch: epoch,
                compatibility: compatibility.clone(),
            };
            if write_handshake_response(&mut stream, &response).is_err()
                || configure_public_stream(&stream).is_err()
                || control.is_cancelled()
            {
                return HandshakeOutcome::Rejected;
            }
            let mut state = lock(shared);
            state.status = ViewerStatus::Live;
            state.pairing_code = None;
            HandshakeOutcome::Accepted(stream, session_id, epoch)
        }
        PairingVerdict::Rejected { .. } => {
            let _ = write_handshake_response(&mut stream, &HandshakeResponse::Rejected);
            HandshakeOutcome::Rejected
        }
        PairingVerdict::Expired => {
            let _ = write_handshake_response(&mut stream, &HandshakeResponse::Rejected);
            HandshakeOutcome::Expired
        }
        PairingVerdict::Revoked => {
            let _ = write_handshake_response(&mut stream, &HandshakeResponse::Rejected);
            HandshakeOutcome::Rejected
        }
    }
}

fn receive_stream(
    stream: TcpStream,
    session_id: numinous_broadcast::SessionId,
    epoch: u64,
    compatibility: numinous_broadcast::Compatibility,
    shared: &Arc<Mutex<SharedState>>,
    control: &Arc<WorkerControl>,
) {
    let mut receiver = PublicReceiver::new(session_id, compatibility, epoch);
    let mut reader = BufReader::new(stream);
    loop {
        let message = match read_public_message::<_, PublicToolEvent>(&mut reader) {
            Ok(message) => message,
            Err(_) if control.is_cancelled() => return,
            Err(error) => {
                lock(shared).status = status_for_frame_error(&error);
                return;
            }
        };
        match receiver.receive(message) {
            Ok(ReceiveOutcome::Event(envelope)) => {
                if lock(shared).retain(&envelope).is_err() {
                    lock(shared).status = ViewerStatus::ProtocolRejected;
                    return;
                }
            }
            Ok(ReceiveOutcome::Control(ControlMarker::Paused)) => {
                lock(shared).status = ViewerStatus::GuestPaused;
            }
            Ok(ReceiveOutcome::Control(ControlMarker::Resumed)) => {
                lock(shared).status = ViewerStatus::Live;
            }
            Ok(ReceiveOutcome::Control(ControlMarker::Stopped)) => {
                lock(shared).status = ViewerStatus::GuestStopped;
                return;
            }
            Err(_) => {
                lock(shared).status = ViewerStatus::ProtocolRejected;
                return;
            }
        }
    }
}

fn status_for_frame_error(error: &FrameError) -> ViewerStatus {
    match error {
        FrameError::Io(_) | FrameError::Truncated => ViewerStatus::Disconnected,
        FrameError::Empty
        | FrameError::TooLarge { .. }
        | FrameError::TooDeep { .. }
        | FrameError::InvalidJson => ViewerStatus::ProtocolRejected,
    }
}

fn layout_lines(
    snapshot: &ViewerSnapshot,
    input_mode: InputMode,
    width: usize,
    height: usize,
    scroll: usize,
    column: usize,
) -> (Vec<String>, i32, i32) {
    for scale in (1..=2).rev() {
        let columns = ((width as i32 - 20) / (6 * scale)).max(12) as usize;
        let rows = (height.saturating_sub(12) as i32 / (9 * scale)).max(4) as usize;
        let lines = semantic_lines(snapshot, input_mode, columns, rows, scroll, column);
        if lines.len() <= rows {
            return (lines, scale, 9);
        }
    }
    let columns = ((width as i32 - 20) / 6).max(12) as usize;
    let rows = (height.saturating_sub(12) / 9).max(4);
    let mut lines = semantic_lines(snapshot, input_mode, columns, rows, scroll, column);
    lines.truncate(rows);
    (lines, 1, 9)
}

fn semantic_lines(
    snapshot: &ViewerSnapshot,
    input_mode: InputMode,
    columns: usize,
    rows: usize,
    scroll: usize,
    column: usize,
) -> Vec<String> {
    let mut fixed = vec![
        "WATCH AGENT".to_string(),
        snapshot.status.label().to_string(),
        "PRIVATE ACTIVITY IS NEVER REPRESENTED".to_string(),
    ];
    let mut body = Vec::new();
    if let Some(code) = &snapshot.pairing_code {
        fixed.push(String::new());
        fixed.push("SHARE THIS ONE-USE CODE ONLY WITH THE MCP PLAYER YOU CHOOSE".to_string());
        body.push(code.clone());
        body.push(String::new());
        body.push("THE PLAYER MUST CALL BROADCAST_SESSION START".to_string());
    } else if let Some(event) = &snapshot.event {
        fixed.push(format!(
            "EVENT {} OF {}   PUBLIC SEQUENCE {}{}",
            snapshot.selected_index.map_or(0, |index| index + 1),
            snapshot.event_count,
            event.public_sequence,
            if snapshot.display_paused {
                "   DISPLAY PAUSED"
            } else {
                ""
            }
        ));
        fixed.push(format!(
            "ACTION {}",
            event
                .event
                .tool
                .name()
                .replace('_', " ")
                .to_ascii_uppercase()
        ));
        if let Some(skipped) = event.skipped {
            fixed.push(format!(
                "PRODUCER GAP {} THROUGH {}",
                skipped.first, skipped.last
            ));
        }
        if snapshot.retention_dropped > 0 {
            fixed.push(format!(
                "LOCAL RING RETIRED {} OLDER EVENTS",
                snapshot.retention_dropped
            ));
        }
        body.push(format!(
            "INPUT {}",
            compact_json_object(&event.event.arguments)
        ));
        body.push(String::new());
        body.push("PUBLIC RESULT TEXT".to_string());
        body.extend(public_result_lines(&event.event.result));
    } else {
        body.push(String::new());
        body.push(match snapshot.status {
            ViewerStatus::Live | ViewerStatus::GuestPaused => {
                "CONNECTED. WAITING FOR THE FIRST PUBLIC ACTION".to_string()
            }
            _ => "NO PUBLIC ACTIONS RETAINED".to_string(),
        });
    }

    let controls = match input_mode {
        InputMode::KeyboardMouse => {
            "LEFT/RIGHT EVENT   UP/DOWN RESULT   A/D PAN   SPACE PAUSE   ESC CLOSE"
        }
        InputMode::Controller => "D-PAD EVENT/RESULT   LB/RB PAN   R3 PAUSE DISPLAY   EAST CLOSE",
    };
    let footer = vec![String::new(), controls.to_string()];
    let body_width = body
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let body_column = column.min(body_width.saturating_sub(columns));
    if body_width > columns {
        fixed.push(format!(
            "RESULT COLUMNS {} THROUGH {} OF {}",
            body_column + 1,
            (body_column + columns).min(body_width),
            body_width
        ));
    }
    let mut wrapped_fixed = wrap_fixed_lines(&fixed, columns);
    let visible_body = viewport_lines(&body, columns, body_column);
    let wrapped_footer = wrap_fixed_lines(&footer, columns);
    let reserved = wrapped_fixed.len().saturating_add(wrapped_footer.len());
    let body_room = rows.saturating_sub(reserved);
    let max_start = visible_body.len().saturating_sub(body_room);
    let start = scroll.min(max_start);
    let end = start.saturating_add(body_room).min(visible_body.len());
    wrapped_fixed.extend_from_slice(&visible_body[start..end]);
    wrapped_fixed.extend(wrapped_footer);
    wrapped_fixed
}

fn wrap_fixed_lines(lines: &[String], columns: usize) -> Vec<String> {
    lines
        .iter()
        .flat_map(|line| {
            if line.is_empty() {
                vec![String::new()]
            } else {
                numinous_core::wrap_text(line, columns)
            }
        })
        .collect()
}

fn viewport_lines(lines: &[String], columns: usize, column: usize) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.chars().skip(column).take(columns.max(1)).collect())
        .collect()
}

fn compact_json_object(object: &Map<String, Value>) -> String {
    serde_json::to_string(object).unwrap_or_else(|_| "{}".to_string())
}

fn public_result_lines(result: &Map<String, Value>) -> Vec<String> {
    let text: Vec<String> = result
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .flat_map(|text| text.lines().map(str::to_string))
        .collect();
    if text.is_empty() {
        vec![compact_json_object(result)]
    } else {
        text
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn set_terminal_status(shared: &Mutex<SharedState>, status: ViewerStatus) {
    let mut state = lock(shared);
    state.status = status;
    state.pairing_code = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use numinous_broadcast::{
        PairingCode, PairingError, PublicTool, SequenceRange, WireMessage, read_handshake_proof,
        read_handshake_response, write_handshake_request, write_public_message,
    };
    use std::io::Write;

    fn fixture(sequence: u64, text: &str) -> EventEnvelope<PublicToolEvent> {
        let compatibility = numinous_compatibility().expect("compatibility");
        let event = PublicToolEvent::new(
            PublicTool::PlayRoom,
            &serde_json::json!({"room": "times-tables", "phase": 0.0}),
            &serde_json::json!({"content": [{"type": "text", "text": text}]}),
        )
        .expect("public event");
        EventEnvelope {
            session_id: numinous_broadcast::SessionId::generate().expect("session"),
            consent_epoch: 2,
            public_sequence: sequence,
            skipped: None,
            compatibility,
            event,
        }
    }

    fn wait_until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !condition() {
            assert!(Instant::now() < deadline, "condition timed out");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn connected_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind loopback");
        let endpoint = listener.local_addr().expect("loopback address");
        let connector = thread::spawn(move || TcpStream::connect(endpoint).expect("connect"));
        let (host, _) = listener.accept().expect("accept loopback");
        let guest = connector.join().expect("connector thread");
        (host, guest)
    }

    #[test]
    fn retained_timeline_enforces_event_and_serialized_byte_caps() {
        let mut shared = SharedState::awaiting("code".to_string());
        for sequence in 0..300 {
            shared
                .retain(&fixture(sequence, &format!("frame {sequence}")))
                .expect("retain bounded frame");
        }
        assert_eq!(shared.frames.len(), MAX_RETAINED_EVENTS);
        assert_eq!(shared.frames.front().map(|frame| frame.sequence), Some(44));
        assert_eq!(shared.retention_dropped, 44);
        assert!(shared.retained_bytes <= MAX_RETAINED_BYTES);
        assert_eq!(
            shared.retained_bytes,
            shared
                .frames
                .iter()
                .map(|frame| frame.encoded.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn retained_timeline_retires_oldest_frames_under_byte_pressure() {
        let mut shared = SharedState::awaiting("code".to_string());
        shared
            .retain_with_limits(0, vec![1; 6].into_boxed_slice(), 8, 10)
            .expect("retain first frame");
        shared
            .retain_with_limits(1, vec![2; 6].into_boxed_slice(), 8, 10)
            .expect("retain second frame");

        assert_eq!(shared.frames.len(), 1);
        assert_eq!(shared.frames.front().map(|frame| frame.sequence), Some(1));
        assert_eq!(shared.retained_bytes, 6);
        assert_eq!(shared.retention_dropped, 1);
        assert!(
            shared
                .retain_with_limits(2, vec![3; 11].into_boxed_slice(), 8, 10)
                .is_err()
        );
    }

    #[test]
    fn cancellation_and_stream_install_leave_no_active_stream() {
        for _ in 0..64 {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind loopback");
            let endpoint = listener.local_addr().expect("loopback address");
            let connector = thread::spawn(move || TcpStream::connect(endpoint).expect("connect"));
            let (stream, _) = listener.accept().expect("accept loopback");
            let peer = connector.join().expect("connector thread");
            let control = Arc::new(WorkerControl::new());
            let barrier = Arc::new(std::sync::Barrier::new(3));

            let install_control = Arc::clone(&control);
            let install_barrier = Arc::clone(&barrier);
            let installer = thread::spawn(move || {
                install_barrier.wait();
                install_control.install(&stream)
            });
            let cancel_control = Arc::clone(&control);
            let cancel_barrier = Arc::clone(&barrier);
            let canceller = thread::spawn(move || {
                cancel_barrier.wait();
                cancel_control.cancel();
            });

            barrier.wait();
            let _ = installer.join().expect("installer thread");
            canceller.join().expect("canceller thread");
            assert!(control.is_cancelled());
            assert!(lock(&control.active_stream).is_none());
            drop(peer);
        }
    }

    #[test]
    fn malformed_frames_are_rejected_but_transport_failures_disconnect() {
        for error in [
            FrameError::Empty,
            FrameError::TooLarge { maximum: 1 },
            FrameError::TooDeep { maximum: 1 },
            FrameError::InvalidJson,
        ] {
            assert_eq!(
                status_for_frame_error(&error),
                ViewerStatus::ProtocolRejected
            );
        }
        assert_eq!(
            status_for_frame_error(&FrameError::Truncated),
            ViewerStatus::Disconnected
        );
        assert_eq!(
            status_for_frame_error(&FrameError::Io(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "peer reset",
            ))),
            ViewerStatus::Disconnected
        );
    }

    #[test]
    fn result_viewport_preserves_rows_indentation_and_horizontal_geometry() {
        let lines = vec![
            "    COLUMN A   COLUMN B   COLUMN C".to_string(),
            "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
        ];

        assert_eq!(
            viewport_lines(&lines, 12, 0),
            vec!["    COLUMN A".to_string(), "0123456789AB".to_string()]
        );
        assert_eq!(
            viewport_lines(&lines, 12, 5),
            vec!["OLUMN A   CO".to_string(), "56789ABCDEFG".to_string()]
        );
        assert_eq!(viewport_lines(&lines, 12, 5).len(), lines.len());
    }

    #[test]
    fn labels_and_open_errors_explain_every_stable_state_without_secrets() {
        for status in [
            ViewerStatus::Closed,
            ViewerStatus::AwaitingGuest,
            ViewerStatus::Live,
            ViewerStatus::GuestPaused,
            ViewerStatus::GuestStopped,
            ViewerStatus::PairingExpired,
            ViewerStatus::PairingRejected,
            ViewerStatus::Disconnected,
            ViewerStatus::ProtocolRejected,
        ] {
            assert!(!status.label().is_empty());
        }

        let io_error = ViewerOpenError::from(io::Error::new(
            io::ErrorKind::AddrInUse,
            "private operating-system detail",
        ));
        assert_eq!(io_error.to_string(), "local viewer I/O failed: AddrInUse");
        assert!(Error::source(&io_error).is_some());

        let pairing_error = ViewerOpenError::from(PairingError::InvalidCode);
        assert_eq!(
            pairing_error.to_string(),
            "local viewer pairing failed: invalid pairing code"
        );
        assert!(Error::source(&pairing_error).is_some());

        for (error, expected) in [
            (
                ViewerOpenError::InvalidLocalEndpoint,
                "invalid local viewer endpoint",
            ),
            (
                ViewerOpenError::Compatibility,
                "viewer compatibility is unavailable",
            ),
            (
                ViewerOpenError::Clock,
                "viewer pairing deadline is unavailable",
            ),
        ] {
            assert_eq!(error.to_string(), expected);
            assert!(Error::source(&error).is_none());
        }
    }

    #[test]
    fn drawing_covers_pairing_live_retained_and_compact_viewer_states() {
        let mut viewer = SessionViewer::default();
        let closed = viewer.draw(900, 700, InputMode::KeyboardMouse);
        assert_eq!((closed.width(), closed.height()), (900, 700));
        assert!(closed.lit_count() > 0);

        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::AwaitingGuest;
            shared.pairing_code = Some("PAIRING-CODE-WITH-A-LONG-LOCAL-CAPABILITY".to_string());
        }
        let pairing = viewer.draw(180, 90, InputMode::Controller);
        assert_eq!((pairing.width(), pairing.height()), (180, 90));
        assert!(pairing.lit_count() > 0);

        let mut event = fixture(
            7,
            &format!(
                "    FIXED COLUMN A      FIXED COLUMN B      FIXED COLUMN C\n{}",
                "0123456789".repeat(12)
            ),
        );
        event.skipped = Some(SequenceRange { first: 0, last: 6 });
        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::GuestPaused;
            shared.pairing_code = None;
            shared.retention_dropped = 3;
            shared.retain(&event).expect("retain display event");
        }
        viewer.follow_live = false;
        viewer.selected_sequence = Some(999);
        viewer.scroll_result(4);
        viewer.pan_result(20);
        assert_eq!((viewer.result_scroll, viewer.result_column), (4, 20));
        let retained = viewer.draw(240, 140, InputMode::Controller);
        assert_eq!((retained.width(), retained.height()), (240, 140));
        assert!(retained.lit_count() > 0);
        let cached = viewer.draw(240, 140, InputMode::KeyboardMouse);
        assert!(cached.lit_count() > 0);

        let live_waiting = ViewerSnapshot {
            status: ViewerStatus::Live,
            pairing_code: None,
            event: None,
            selected_index: None,
            event_count: 0,
            retention_dropped: 0,
            display_paused: false,
        };
        let (large, scale, _) =
            layout_lines(&live_waiting, InputMode::KeyboardMouse, 900, 700, 0, 0);
        assert_eq!(scale, 2);
        assert!(
            large
                .iter()
                .any(|line| line.contains("WAITING FOR THE FIRST"))
        );
        let (compact, scale, _) = layout_lines(&live_waiting, InputMode::Controller, 80, 30, 0, 0);
        assert_eq!(scale, 1);
        assert!(!compact.is_empty());
        assert_eq!(public_result_lines(&Map::new()), ["{}".to_string()]);
    }

    #[test]
    fn eight_authenticated_handshake_rejections_close_the_pairing_offer() {
        let mut viewer = SessionViewer::default();
        viewer.open().expect("open viewer");
        let code = lock(&viewer.shared)
            .pairing_code
            .clone()
            .expect("pairing code");
        let pairing = PairingCode::parse(&code, SystemTime::now()).expect("parse code");
        let compatibility = numinous_compatibility().expect("compatibility");

        for _ in 0..numinous_broadcast::MAX_HANDSHAKE_ATTEMPTS {
            let mut stream = TcpStream::connect(pairing.endpoint()).expect("connect");
            configure_handshake_stream(&stream).expect("handshake deadlines");
            let reader_stream = stream.try_clone().expect("reader clone");
            let mut reader = BufReader::new(reader_stream);
            let proof = read_handshake_proof(&mut reader).expect("host proof");
            assert!(pairing.verifies_host_proof(&proof));
            let mut request = pairing.handshake_request(compatibility.clone());
            let replacement = if request.capability.starts_with('0') {
                "1"
            } else {
                "0"
            };
            request.capability.replace_range(0..1, replacement);
            write_handshake_request(&mut stream, &request).expect("invalid request");
            assert_eq!(
                read_handshake_response(&mut reader).expect("rejection"),
                HandshakeResponse::Rejected
            );
        }

        wait_until(|| viewer.status() == ViewerStatus::PairingRejected);
        assert!(lock(&viewer.shared).pairing_code.is_none());
        viewer.close();
    }

    #[test]
    fn expired_listener_and_control_transitions_have_terminal_statuses() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind loopback");
        listener.set_nonblocking(true).expect("nonblocking");
        let port =
            NonZeroU16::new(listener.local_addr().expect("address").port()).expect("nonzero port");
        let compatibility = numinous_compatibility().expect("compatibility");
        let gate = PairingOffer::generate(port, SystemTime::now())
            .expect("pairing offer")
            .into_gate(compatibility.clone());
        let expired = Arc::new(Mutex::new(SharedState::awaiting("code".to_string())));
        let control = Arc::new(WorkerControl::new());
        listener_worker(
            listener,
            gate,
            compatibility.clone(),
            Instant::now(),
            &expired,
            &control,
        );
        assert_eq!(lock(&expired).status, ViewerStatus::PairingExpired);
        assert!(lock(&expired).pairing_code.is_none());

        let (host, mut guest) = connected_pair();
        let session_id = numinous_broadcast::SessionId::generate().expect("session");
        let shared = Arc::new(Mutex::new(SharedState::awaiting("code".to_string())));
        lock(&shared).status = ViewerStatus::Live;
        let receiver_shared = Arc::clone(&shared);
        let receiver_control = Arc::new(WorkerControl::new());
        let worker_control = Arc::clone(&receiver_control);
        let worker = thread::spawn(move || {
            receive_stream(
                host,
                session_id,
                2,
                compatibility,
                &receiver_shared,
                &worker_control,
            );
        });
        for (epoch, marker) in [
            (3, ControlMarker::Paused),
            (4, ControlMarker::Resumed),
            (5, ControlMarker::Stopped),
        ] {
            write_public_message::<_, PublicToolEvent>(
                &mut guest,
                &WireMessage::Control {
                    session_id,
                    consent_epoch: epoch,
                    marker,
                },
            )
            .expect("control marker");
        }
        worker.join().expect("receiver thread");
        assert_eq!(lock(&shared).status, ViewerStatus::GuestStopped);
    }

    #[test]
    fn malformed_and_wrong_session_streams_fail_closed() {
        let compatibility = numinous_compatibility().expect("compatibility");
        let session_id = numinous_broadcast::SessionId::generate().expect("session");

        let (host, mut guest) = connected_pair();
        guest.write_all(b"{}\n").expect("malformed frame");
        drop(guest);
        let malformed = Arc::new(Mutex::new(SharedState::awaiting("code".to_string())));
        lock(&malformed).status = ViewerStatus::Live;
        receive_stream(
            host,
            session_id,
            2,
            compatibility.clone(),
            &malformed,
            &Arc::new(WorkerControl::new()),
        );
        assert_eq!(lock(&malformed).status, ViewerStatus::ProtocolRejected);

        let (host, mut guest) = connected_pair();
        let wrong_session = numinous_broadcast::SessionId::generate().expect("wrong session");
        write_public_message::<_, PublicToolEvent>(
            &mut guest,
            &WireMessage::Control {
                session_id: wrong_session,
                consent_epoch: 3,
                marker: ControlMarker::Paused,
            },
        )
        .expect("wrong-session control");
        drop(guest);
        let rejected = Arc::new(Mutex::new(SharedState::awaiting("code".to_string())));
        lock(&rejected).status = ViewerStatus::Live;
        receive_stream(
            host,
            session_id,
            2,
            compatibility,
            &rejected,
            &Arc::new(WorkerControl::new()),
        );
        assert_eq!(lock(&rejected).status, ViewerStatus::ProtocolRejected);

        let (host, guest) = connected_pair();
        drop(guest);
        let cancelled = Arc::new(Mutex::new(SharedState::awaiting("code".to_string())));
        lock(&cancelled).status = ViewerStatus::Live;
        let control = Arc::new(WorkerControl::new());
        control.cancel();
        receive_stream(
            host,
            session_id,
            2,
            numinous_compatibility().expect("compatibility"),
            &cancelled,
            &control,
        );
        assert_eq!(lock(&cancelled).status, ViewerStatus::Live);
    }

    #[test]
    fn local_pause_and_scrub_never_change_network_or_event_content() {
        let mut viewer = SessionViewer::default();
        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::Live;
            for sequence in 0..3 {
                shared
                    .retain(&fixture(sequence, &format!("frame {sequence}")))
                    .expect("retain");
            }
        }
        viewer.worker = Some(thread::spawn(|| {}));
        assert_eq!(
            viewer.snapshot().event.map(|event| event.public_sequence),
            Some(2)
        );
        viewer.toggle_display_pause();
        viewer.scrub(-1);
        assert_eq!(
            viewer.snapshot().event.map(|event| event.public_sequence),
            Some(1)
        );
        viewer.scrub(-20);
        assert_eq!(
            viewer.snapshot().event.map(|event| event.public_sequence),
            Some(0)
        );
        viewer.toggle_display_pause();
        assert_eq!(
            viewer.snapshot().event.map(|event| event.public_sequence),
            Some(2)
        );
        viewer.close();
        assert_eq!(viewer.status(), ViewerStatus::Closed);
        assert!(lock(&viewer.shared).frames.is_empty());
    }

    #[test]
    fn loopback_pairing_receives_one_public_event_and_clears_on_close() {
        let mut viewer = SessionViewer::default();
        viewer.open().expect("open viewer");
        let code = lock(&viewer.shared)
            .pairing_code
            .clone()
            .expect("pairing code");
        let guest = thread::spawn(move || {
            let pairing = PairingCode::parse(&code, SystemTime::now()).expect("parse code");
            let compatibility = numinous_compatibility().expect("compatibility");
            let mut stream = TcpStream::connect(pairing.endpoint()).expect("connect");
            configure_handshake_stream(&stream).expect("handshake deadlines");
            let reader_stream = stream.try_clone().expect("reader clone");
            let mut reader = BufReader::new(reader_stream);
            let proof = read_handshake_proof(&mut reader).expect("host proof");
            assert!(pairing.verifies_host_proof(&proof));
            write_handshake_request(
                &mut stream,
                &pairing.handshake_request(compatibility.clone()),
            )
            .expect("handshake request");
            let response = read_handshake_response(&mut reader).expect("handshake response");
            let (session_id, epoch) = match response {
                HandshakeResponse::Accepted {
                    session_id,
                    consent_epoch,
                    compatibility: accepted,
                } => {
                    assert_eq!(accepted, compatibility);
                    (session_id, consent_epoch)
                }
                HandshakeResponse::Rejected => panic!("valid pairing rejected"),
            };
            configure_public_stream(&stream).expect("public stream");
            let mut event = fixture(0, "TIMES TABLES K=5 FOUR LOBES");
            event.session_id = session_id;
            event.consent_epoch = epoch;
            write_public_message(&mut stream, &WireMessage::Event(event)).expect("public event");
            write_public_message::<_, PublicToolEvent>(
                &mut stream,
                &WireMessage::Control {
                    session_id,
                    consent_epoch: epoch + 1,
                    marker: ControlMarker::Stopped,
                },
            )
            .expect("stop marker");
        });
        wait_until(|| {
            let shared = lock(&viewer.shared);
            shared.status == ViewerStatus::GuestStopped && shared.frames.len() == 1
        });
        guest.join().expect("guest thread");
        let snapshot = viewer.snapshot();
        assert_eq!(snapshot.status, ViewerStatus::GuestStopped);
        assert_eq!(
            snapshot.event.map(|event| event.event.tool),
            Some(PublicTool::PlayRoom)
        );
        assert!(lock(&viewer.shared).pairing_code.is_none());
        viewer.close();
        let shared = lock(&viewer.shared);
        assert_eq!(shared.status, ViewerStatus::Closed);
        assert!(shared.frames.is_empty());
        assert_eq!(shared.retained_bytes, 0);
    }

    #[test]
    fn viewer_copy_names_consent_privacy_action_and_read_only_controls() {
        let snapshot = ViewerSnapshot {
            status: ViewerStatus::Live,
            pairing_code: None,
            event: Some(fixture(7, "FOUR LOBES FOUND")),
            selected_index: Some(0),
            event_count: 1,
            retention_dropped: 0,
            display_paused: false,
        };
        let copy = semantic_lines(&snapshot, InputMode::KeyboardMouse, 80, 40, 0, 0).join("\n");
        assert!(copy.contains("PRIVATE ACTIVITY IS NEVER REPRESENTED"));
        assert!(copy.contains("ACTION PLAY ROOM"));
        assert!(copy.contains("PUBLIC RESULT TEXT"));
        assert!(copy.contains("FOUR LOBES FOUND"));
        assert!(copy.contains("SPACE PAUSE"));
        for forbidden in ["CALL TOOL", "SEND", "SUBMIT", "CONTROL MCP"] {
            assert!(!copy.contains(forbidden), "viewer offered {forbidden}");
        }
    }
}
