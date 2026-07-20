//! Human-owned local viewing of an explicitly consented MCP public session.
//!
//! The listener accepts one authenticated loopback guest, retains a bounded
//! in-memory public timeline, and reconstructs native room frames without
//! exposing control of the guest or representing private activity.

use numinous_broadcast::{
    ConsentMachine, ControlMarker, EventEnvelope, FrameError, HandshakeResponse,
    PLAY_ROOM_MAX_HEIGHT, PLAY_ROOM_MAX_WIDTH, PairingGate, PairingOffer, PairingVerdict,
    PublicReceiver, PublicTool, PublicToolEvent, ReceiveOutcome, configure_handshake_stream,
    configure_public_stream, numinous_compatibility, read_handshake_request, read_public_message,
    write_handshake_proof, write_handshake_response,
};
use numinous_core::{Expr, Raster, Room, RoomInput, Surface};
use serde_json::{Map, Value};
#[cfg(test)]
use std::cell::Cell;
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

const MAX_RETAINED_EVENTS: usize = 256;
const MAX_RETAINED_BYTES: usize = 16 * 1_024 * 1_024;
const ACCEPT_POLL: Duration = Duration::from_millis(10);

/// The control-label family shown by the local session viewer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ViewerInputMode {
    /// Keyboard and mouse labels.
    #[default]
    KeyboardMouse,
    /// Game-controller labels.
    Controller,
}

/// The current local viewer connection state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerStatus {
    /// No listener or retained stream exists.
    Closed,
    /// A one-use local pairing offer is waiting for a guest.
    AwaitingGuest,
    /// The authenticated guest is broadcasting public actions.
    Live,
    /// The guest paused public event emission.
    GuestPaused,
    /// The guest ended the public broadcast normally.
    GuestStopped,
    /// The one-use pairing offer expired before acceptance.
    PairingExpired,
    /// The listener exhausted bounded invalid handshake attempts.
    PairingRejected,
    /// The authenticated transport ended without a valid stop marker.
    Disconnected,
    /// The authenticated public stream failed strict protocol validation.
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

struct RoomReplay {
    room: Box<dyn Room>,
    phase: f64,
    inputs: Vec<RoomInput>,
}

impl RoomReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        let mut raster = Raster::with_accent(width, height, self.room.meta().accent);
        if self.inputs.is_empty() {
            self.room.render(&mut raster, self.phase);
        } else {
            self.room
                .render_input(&mut raster, self.phase, &self.inputs);
        }
        raster
    }

    fn status(&self) -> Option<String> {
        if self.inputs.is_empty() {
            self.room.status(self.phase)
        } else {
            self.room.status_input(self.phase, &self.inputs)
        }
    }
}

struct StudioReplay {
    source: String,
    expression: Expr,
    xmin: f64,
    xmax: f64,
    parameter: f64,
    ymin: f64,
    ymax: f64,
}

impl StudioReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        let mut raster = Raster::with_accent(width, height, [198, 132, 255]);
        let _ = crate::studio_render::draw_curve(
            &mut raster,
            crate::studio_render::CurveLayout {
                width,
                height,
                top: 35.0,
                bottom_margin: 18.0,
            },
            self.xmin,
            self.xmax,
            |x| Some(numinous_core::eval(&self.expression, x, self.parameter)),
        );
        raster
    }

    fn detail(&self) -> String {
        format!(
            "PLOT EXPRESSION / Y = {} / X [{:.3}, {:.3}] / A {:.3} / Y [{:.3}, {:.3}]",
            single_line(&self.source.to_uppercase(), 80),
            self.xmin,
            self.xmax,
            self.parameter,
            self.ymin,
            self.ymax
        )
    }
}

struct NimGameReplay {
    seed: u64,
    replay: numinous_core::nim::NimReplay,
}

impl NimGameReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        crate::nim_render::draw_nim_board(&self.replay.heaps, None, width, height)
            .unwrap_or_else(|| Raster::with_accent(width, height, [230, 200, 120]))
    }

    fn detail(&self) -> String {
        let state = match self.replay.winner {
            Some(numinous_core::nim::NimWinner::Player) => "PLAYER WON",
            Some(numinous_core::nim::NimWinner::Order) => "ORDER WON",
            None => "IN PROGRESS",
        };
        let heaps = self
            .replay
            .heaps
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        format!("NIM / SEED {} / {state} / HEAPS {heaps}", self.seed)
    }
}

struct MunchGameReplay {
    seed: u64,
    round: u64,
    play: crate::play::MunchPlay,
}

impl MunchGameReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        crate::game_draw::draw_munch(
            &self.play,
            0,
            crate::input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    }

    fn detail(&self) -> String {
        format!("MUNCH / SEED {} / ROUND {}", self.seed, self.round)
    }
}

struct ArcadeGameReplay {
    seed: u64,
    play: crate::play::ArcadePlay,
}

impl ArcadeGameReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        crate::game_draw::draw_arcade(
            &self.play,
            crate::input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    }

    fn detail(&self) -> String {
        format!(
            "ARCADE / SEED {} / LEVEL {}",
            self.seed, self.play.run.level
        )
    }
}

struct QuizGameReplay {
    seed: u64,
    plays: u32,
    play: crate::play::QuizPlay,
    rooms: Vec<Box<dyn numinous_core::Room>>,
}

impl QuizGameReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        crate::game_draw::draw_quiz(
            &self.rooms,
            &self.play,
            crate::input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    }

    fn detail(&self) -> String {
        format!("QUIZ / SEED {} / PLAYS {}", self.seed, self.plays)
    }
}

struct GauntletGameReplay {
    seed: u64,
    play: crate::play::GauntletPlay,
    rooms: Vec<Box<dyn numinous_core::Room>>,
}

impl GauntletGameReplay {
    fn render(&self, width: usize, height: usize) -> Raster {
        crate::game_draw::draw_gauntlet(
            &self.rooms,
            &self.play,
            0,
            crate::input_legend::InputMode::KeyboardMouse,
            width,
            height,
        )
    }

    fn detail(&self) -> String {
        format!("GAUNTLET / SEED {}", self.seed)
    }
}

enum NativeReplayKind {
    Room(RoomReplay),
    Studio(StudioReplay),
    Nim(NimGameReplay),
    Munch(MunchGameReplay),
    Arcade(ArcadeGameReplay),
    Quiz(QuizGameReplay),
    Gauntlet(Box<GauntletGameReplay>),
}

struct NativeReplay {
    kind: NativeReplayKind,
    #[cfg(test)]
    render_count: Cell<usize>,
}

impl NativeReplay {
    fn room(replay: RoomReplay) -> Self {
        Self {
            kind: NativeReplayKind::Room(replay),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn studio(replay: StudioReplay) -> Self {
        Self {
            kind: NativeReplayKind::Studio(replay),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn nim(replay: NimGameReplay) -> Self {
        Self {
            kind: NativeReplayKind::Nim(replay),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn munch(replay: MunchGameReplay) -> Self {
        Self {
            kind: NativeReplayKind::Munch(replay),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn arcade(replay: ArcadeGameReplay) -> Self {
        Self {
            kind: NativeReplayKind::Arcade(replay),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn quiz(replay: QuizGameReplay) -> Self {
        Self {
            kind: NativeReplayKind::Quiz(replay),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn gauntlet(replay: GauntletGameReplay) -> Self {
        Self {
            kind: NativeReplayKind::Gauntlet(Box::new(replay)),
            #[cfg(test)]
            render_count: Cell::new(0),
        }
    }

    fn render(&self, width: usize, height: usize) -> Raster {
        #[cfg(test)]
        self.render_count.set(self.render_count.get() + 1);
        match &self.kind {
            NativeReplayKind::Room(replay) => replay.render(width, height),
            NativeReplayKind::Studio(replay) => replay.render(width, height),
            NativeReplayKind::Nim(replay) => replay.render(width, height),
            NativeReplayKind::Munch(replay) => replay.render(width, height),
            NativeReplayKind::Arcade(replay) => replay.render(width, height),
            NativeReplayKind::Quiz(replay) => replay.render(width, height),
            NativeReplayKind::Gauntlet(replay) => replay.render(width, height),
        }
    }

    fn detail(&self) -> String {
        match &self.kind {
            NativeReplayKind::Room(replay) => {
                let meta = replay.room.meta();
                let mut detail = format!("PLAY ROOM / {} / T {:.3}", meta.title, replay.phase);
                if let Some(status) = replay.status() {
                    detail.push_str(" / ");
                    detail.push_str(&single_line(&status, 160));
                }
                detail
            }
            NativeReplayKind::Studio(replay) => replay.detail(),
            NativeReplayKind::Nim(replay) => replay.detail(),
            NativeReplayKind::Munch(replay) => replay.detail(),
            NativeReplayKind::Arcade(replay) => replay.detail(),
            NativeReplayKind::Quiz(replay) => replay.detail(),
            NativeReplayKind::Gauntlet(replay) => replay.detail(),
        }
    }

    #[allow(dead_code)]
    fn sound(&self) -> Option<numinous_core::SoundSpec> {
        match &self.kind {
            NativeReplayKind::Room(replay) => Some(
                replay
                    .room
                    .sound_input(replay.phase, replay.inputs.as_slice()),
            ),
            NativeReplayKind::Studio(replay) => Some(numinous_core::to_melody(
                &replay.expression,
                replay.xmin,
                replay.xmax,
                32,
                replay.parameter,
            )),
            NativeReplayKind::Nim(_) => None,
            NativeReplayKind::Munch(_) => None,
            NativeReplayKind::Arcade(_) => None,
            NativeReplayKind::Quiz(_) => None,
            NativeReplayKind::Gauntlet(_) => None,
        }
    }
}

/// Human-owned, read-only local MCP session viewer.
pub struct SessionViewer {
    shared: Arc<Mutex<SharedState>>,
    control: Arc<WorkerControl>,
    worker: Option<JoinHandle<()>>,
    follow_live: bool,
    selected_sequence: Option<u64>,
    result_scroll: usize,
    result_column: usize,
    cached_event: Option<(u64, EventEnvelope<PublicToolEvent>)>,
    cached_replay: Option<(u64, Option<NativeReplay>)>,
    cached_replay_frame: Option<(u64, usize, usize, Raster)>,
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
            cached_replay: None,
            cached_replay_frame: None,
        }
    }
}

/// A snapshot of the audio state for a given event, used for native playback reconstruction.
pub struct AudioSelection {
    public_sequence: u64,
    spec: numinous_core::SoundSpec,
}

impl AudioSelection {
    /// Returns the public sequence number of the event that produced this audio.
    pub fn public_sequence(&self) -> u64 {
        self.public_sequence
    }

    /// Renders the sound specification into a PCM float vector at the given sample rate.
    pub fn render(&self, sample_rate: u32) -> Option<Vec<f32>> {
        Some(self.spec.render(sample_rate))
    }
}

impl SessionViewer {
    /// Retrieve the current cached event's audio, if available.
    pub fn audio_selection(&mut self) -> Option<AudioSelection> {
        let envelope = self.snapshot().event?;
        let spec = self
            .cached_replay
            .as_ref()
            .and_then(|(_, r)| r.as_ref().and_then(|r| r.sound()))?;
        Some(AudioSelection {
            public_sequence: envelope.public_sequence,
            spec,
        })
    }
    /// Opens a fresh loopback-only pairing offer and clears prior events.
    pub fn open(&mut self) -> Result<(), ViewerOpenError> {
        self.close();
        let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let port = NonZeroU16::new(listener.local_addr()?.port())
            .ok_or(ViewerOpenError(ViewerOpenErrorKind::InvalidLocalEndpoint))?;
        let compatibility = numinous_compatibility()
            .map_err(|_| ViewerOpenError(ViewerOpenErrorKind::Compatibility))?;
        let offer = PairingOffer::generate(port, SystemTime::now())?;
        let pairing_code = offer.display_code();
        let gate = offer.into_gate(compatibility.clone());
        let deadline = Instant::now()
            .checked_add(numinous_broadcast::PAIRING_TTL)
            .ok_or(ViewerOpenError(ViewerOpenErrorKind::Clock))?;
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
        self.cached_replay = None;
        self.cached_replay_frame = None;
        Ok(())
    }

    /// Closes the listener, joins its worker, and clears all retained events.
    pub fn close(&mut self) {
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
        self.cached_replay = None;
        self.cached_replay_frame = None;
    }

    /// Reports whether this viewer owns an active listener worker.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.worker.is_some()
    }

    /// Returns the current connection state without exposing a capability.
    #[must_use]
    pub fn status(&self) -> ViewerStatus {
        lock(&self.shared).status
    }

    /// Copies the transient one-use pairing code for a consenting local player.
    ///
    /// Callers must present this value only to the local human and must never
    /// log, persist, or transmit it anywhere except the selected MCP process.
    #[must_use]
    pub fn pairing_code(&self) -> Option<String> {
        lock(&self.shared).pairing_code.clone()
    }

    /// Copies the currently retained, already validated public event stream.
    ///
    /// The copy is bounded by the same event and serialized-byte limits as the
    /// viewer ring. Invalid retained bytes are omitted, which can occur only if
    /// in-process memory has been corrupted after receive validation.
    #[must_use]
    pub fn retained_events(&self) -> Vec<EventEnvelope<PublicToolEvent>> {
        lock(&self.shared)
            .frames
            .iter()
            .filter_map(|frame| serde_json::from_slice(&frame.encoded).ok())
            .collect()
    }

    /// Toggles local display following without sending control to the guest.
    pub fn toggle_display_pause(&mut self) {
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

    /// Moves the local selection through retained public events.
    pub fn scrub(&mut self, delta: isize) {
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
        self.cached_replay = None;
        self.cached_replay_frame = None;
    }

    /// Scrolls the local text result by a signed line delta.
    pub fn scroll_result(&mut self, delta: isize) {
        self.result_scroll = self.result_scroll.saturating_add_signed(delta);
    }

    /// Pans the local text result by a signed column delta.
    pub fn pan_result(&mut self, delta: isize) {
        self.result_column = self.result_column.saturating_add_signed(delta);
    }

    /// Draws the selected public action as native replay or bounded text.
    #[must_use]
    pub fn draw(&mut self, width: usize, height: usize, input_mode: ViewerInputMode) -> Raster {
        let snapshot = self.snapshot();
        if let Some(event) = snapshot.event.as_ref() {
            if self.cached_replay.as_ref().map(|cached| cached.0) != Some(event.public_sequence) {
                self.cached_replay_frame = None;
                self.cached_replay =
                    Some((event.public_sequence, parse_native_replay(&event.event)));
            }
            if self
                .cached_replay
                .as_ref()
                .and_then(|cached| cached.1.as_ref())
                .is_some()
            {
                let frame_key = (event.public_sequence, width, height);
                if self
                    .cached_replay_frame
                    .as_ref()
                    .map(|cached| (cached.0, cached.1, cached.2))
                    != Some(frame_key)
                {
                    let rendered = self
                        .cached_replay
                        .as_ref()
                        .and_then(|cached| cached.1.as_ref())
                        .map(|replay| replay.render(width, height));
                    self.cached_replay_frame =
                        rendered.map(|frame| (event.public_sequence, width, height, frame));
                }
                if let (Some(cached_frame), Some(replay)) = (
                    self.cached_replay_frame.as_ref(),
                    self.cached_replay
                        .as_ref()
                        .and_then(|cached| cached.1.as_ref()),
                ) {
                    let mut raster = cached_frame.3.clone();
                    draw_native_replay_chrome(
                        &mut raster,
                        &snapshot,
                        replay,
                        input_mode,
                        width,
                        height,
                    );
                    return raster;
                }
            }
        } else {
            self.cached_replay = None;
            self.cached_replay_frame = None;
        }
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

fn parse_room_replay(arguments: &Map<String, Value>) -> Option<RoomReplay> {
    if arguments.keys().any(|key| {
        !matches!(
            key.as_str(),
            "id" | "t" | "width" | "height" | "variation" | "pokes" | "gesture"
        )
    }) {
        return None;
    }
    if !valid_optional_dimension(arguments.get("width"), PLAY_ROOM_MAX_WIDTH)
        || !valid_optional_dimension(arguments.get("height"), PLAY_ROOM_MAX_HEIGHT)
    {
        return None;
    }
    let id = arguments.get("id")?.as_str()?;
    let phase = optional_unit(arguments.get("t"), 0.0, false)?;
    let variation = match arguments.get("variation") {
        Some(value) => value.as_u64()?,
        None => 0,
    };
    let pokes = parse_room_pokes(arguments.get("pokes"))?;
    let gesture = parse_room_gesture(arguments.get("gesture"))?;
    if !pokes.is_empty() && !gesture.is_empty() {
        return None;
    }
    let inputs = if gesture.is_empty() {
        numinous_core::inputs_from_pokes(&pokes, phase)
    } else {
        gesture
    };
    let room = if variation == 0 {
        numinous_core::room_by_id(id)
    } else {
        numinous_core::all_rooms_with(variation)
            .into_iter()
            .find(|room| room.meta().id == id)
    }?;
    Some(RoomReplay {
        room,
        phase,
        inputs,
    })
}

fn parse_native_replay(event: &PublicToolEvent) -> Option<NativeReplay> {
    match event.tool {
        PublicTool::PlayRoom => parse_room_replay(&event.arguments).map(NativeReplay::room),
        PublicTool::PlotExpression => {
            parse_studio_replay(&event.arguments, &event.result).map(NativeReplay::studio)
        }
        PublicTool::Nim => parse_nim_replay(&event.arguments, &event.result).map(NativeReplay::nim),
        PublicTool::Munch => parse_munch_replay(&event.arguments).map(NativeReplay::munch),
        PublicTool::MunchArcade => parse_arcade_replay(&event.arguments).map(NativeReplay::arcade),
        PublicTool::Quiz => parse_quiz_replay(&event.arguments).map(NativeReplay::quiz),
        PublicTool::Gauntlet => parse_gauntlet_replay(&event.arguments).map(NativeReplay::gauntlet),
        _ => None,
    }
}

fn parse_nim_replay(
    arguments: &Map<String, Value>,
    result: &Map<String, Value>,
) -> Option<NimGameReplay> {
    if arguments
        .keys()
        .any(|key| !matches!(key.as_str(), "seed" | "moves"))
    {
        return None;
    }
    let seed = match arguments.get("seed") {
        Some(seed) => seed.as_u64()?,
        None => 1,
    };
    let turns = match arguments.get("moves") {
        Some(moves) => {
            let moves = moves.as_array()?;
            if moves.len() > numinous_core::nim::MAX_REPLAY_TURNS {
                return None;
            }
            moves
                .iter()
                .map(|value| {
                    let pair = value.as_array()?;
                    if pair.len() != 2 {
                        return None;
                    }
                    let heap = pair.first()?.as_u64()?.checked_sub(1)?;
                    let heap = usize::try_from(heap).ok()?;
                    if heap >= 3 {
                        return None;
                    }
                    let take = u32::try_from(pair.get(1)?.as_u64()?).ok()?;
                    (take > 0).then_some(numinous_core::nim::NimTurn { heap, take })
                })
                .collect::<Option<Vec<_>>>()?
        }
        None => Vec::new(),
    };
    let replay = numinous_core::nim::replay(seed, &turns).ok()?;
    if Value::Object(result.clone()) != canonical_nim_result(seed, &replay) {
        return None;
    }
    Some(NimGameReplay { seed, replay })
}

fn canonical_nim_result(seed: u64, replay: &numinous_core::nim::NimReplay) -> Value {
    match replay.winner {
        Some(numinous_core::nim::NimWinner::Player) => {
            let secret = numinous_core::nim_secret();
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "You took the last stone. The Order concedes, and keeps its word:\n\n{secret}"
                    )
                }],
                "structuredContent": {
                    "game": "nim", "seed": seed, "won": true, "secret": secret
                },
                "isError": false
            })
        }
        Some(numinous_core::nim::NimWinner::Order) => serde_json::json!({
            "content": [{
                "type": "text",
                "text": "The Order takes the last stone. Again. (It is not luck.)"
            }],
            "structuredContent": {"game": "nim", "seed": seed, "won": false},
            "isError": false
        }),
        None => {
            let narration = replay
                .order
                .iter()
                .map(|turn| format!("The Order takes {} from heap {}.", turn.take, turn.heap + 1))
                .collect::<Vec<_>>();
            let board = replay
                .heaps
                .iter()
                .enumerate()
                .map(|(index, heap)| format!("  {}) {}", index + 1, "O ".repeat(*heap as usize)))
                .collect::<Vec<_>>();
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "NIM seed {seed}. Last stone wins.\n{}\n{}\nMove by calling again with your full move list.",
                        narration.join("\n"),
                        board.join("\n")
                    )
                }],
                "structuredContent": {
                    "game": "nim", "seed": seed, "heaps": replay.heaps, "order": narration
                },
                "isError": false
            })
        }
    }
}

fn parse_munch_replay(arguments: &Map<String, Value>) -> Option<MunchGameReplay> {
    let seed = arguments.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let round = arguments.get("round").and_then(Value::as_u64).unwrap_or(0);
    let board = numinous_core::build_board(seed, round);

    let bites: std::collections::BTreeSet<usize> = arguments
        .get("bites")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_u64)
                .filter(|&n| n >= 1)
                .map(|n| (n - 1) as usize)
                .collect()
        })
        .unwrap_or_default();

    let graded = if arguments.contains_key("bites") {
        let bites_vec: Vec<usize> = bites.iter().copied().collect();
        Some(numinous_core::grade_munch(&board, &bites_vec))
    } else {
        None
    };

    let play = crate::play::MunchPlay {
        board,
        seed,
        round,
        cursor: 30,
        bites,
        graded,
        bite_flash: None,
    };
    Some(MunchGameReplay { seed, round, play })
}

fn parse_arcade_replay(arguments: &Map<String, Value>) -> Option<ArcadeGameReplay> {
    let seed = arguments.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let mut run = numinous_core::munch_arcade::Arcade::new(seed);

    if let Some(actions) = arguments.get("actions").and_then(Value::as_array) {
        for action_val in actions {
            if let Some(action_str) = action_val.as_str() {
                let action = match action_str.to_ascii_lowercase().as_str() {
                    "up" | "w" => Some(numinous_core::munch_arcade::Action::Up),
                    "down" | "s" => Some(numinous_core::munch_arcade::Action::Down),
                    "left" | "a" => Some(numinous_core::munch_arcade::Action::Left),
                    "right" | "d" => Some(numinous_core::munch_arcade::Action::Right),
                    "eat" | "e" => Some(numinous_core::munch_arcade::Action::Eat),
                    _ => None,
                };
                if let Some(a) = action {
                    run.turn(a);
                }
            }
        }
    }

    let over = run.lives == 0;
    let play = crate::play::ArcadePlay {
        run,
        seed,
        flash: None,
        over,
    };
    Some(ArcadeGameReplay { seed, play })
}

fn parse_quiz_replay(arguments: &Map<String, Value>) -> Option<QuizGameReplay> {
    let seed = arguments.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let round_idx = arguments.get("round").and_then(Value::as_u64).unwrap_or(0);
    let choice_count = arguments
        .get("choices")
        .and_then(Value::as_u64)
        .unwrap_or(4) as usize;
    let round = numinous_core::build_round_sized(seed, round_idx, 54, 22, choice_count);

    let flash = arguments.get("guess").and_then(Value::as_str).map(|g| {
        let letter = g.trim().chars().next().map(|c| c.to_ascii_uppercase());
        let correct = letter == Some(round.answer);
        (correct, 60)
    });

    let play = crate::play::QuizPlay { round, flash };
    Some(QuizGameReplay {
        seed,
        plays: round_idx as u32,
        play,
        rooms: numinous_core::all_rooms(),
    })
}

fn parse_gauntlet_replay(arguments: &Map<String, Value>) -> Option<GauntletGameReplay> {
    let seed = arguments.get("seed").and_then(Value::as_u64).unwrap_or(1);
    let mut scores = Vec::new();
    let mut cleared = Vec::new();

    let stage = if let Some(answers) = arguments.get("answers") {
        let board = numinous_core::build_board(seed, 0);
        let bites: Vec<usize> = answers
            .get("bites")
            .and_then(Value::as_array)
            .map(|l| {
                l.iter()
                    .filter_map(Value::as_u64)
                    .filter(|&n| n >= 1)
                    .map(|n| (n - 1) as usize)
                    .collect()
            })
            .unwrap_or_default();
        let outcome = numinous_core::grade_munch(&board, &bites);
        let clean = outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
        scores.push(outcome.score);
        cleared.push(clean);

        let round = numinous_core::build_round(seed, 1, 44, 18);
        let guess = answers
            .get("shape")
            .and_then(Value::as_str)
            .and_then(|g| g.trim().chars().next())
            .map(|c| c.to_ascii_uppercase());
        let clean = guess == Some(round.answer);
        scores.push(if clean { 25 } else { 0 });
        cleared.push(clean);

        let scan = numinous_core::build_scan(seed, 4);
        let guess = answers
            .get("sky")
            .and_then(Value::as_str)
            .and_then(|g| g.trim().chars().next())
            .map(|c| c.to_ascii_uppercase());
        let clean = guess == Some(scan.answer);
        scores.push(if clean { 25 } else { 0 });
        cleared.push(clean);

        let secret = numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4);
        let wires: Vec<&str> = answers
            .get("wires")
            .and_then(Value::as_array)
            .map(|l| l.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let mut clean = false;
        let mut bomb_points = 0i64;
        for (i, raw) in wires.iter().take(5).enumerate() {
            let guess: Vec<u8> = raw
                .chars()
                .filter(|c| c.is_ascii_digit())
                .map(|c| c as u8 - b'0')
                .collect();
            if guess.len() == 4 && numinous_core::grade(&secret, &guess).locked == 4 {
                clean = true;
                bomb_points = 10 * (5 - i as i64 - 1).max(0);
                break;
            }
        }
        scores.push(bomb_points);
        cleared.push(clean);
        4
    } else {
        0
    };

    let play = crate::play::GauntletPlay {
        seed,
        stage,
        munch: crate::play::MunchPlay {
            board: numinous_core::build_board(seed, 0),
            seed,
            round: 0,
            cursor: 30,
            bites: std::collections::BTreeSet::new(),
            graded: None,
            bite_flash: None,
        },
        quiz: crate::play::QuizPlay {
            round: numinous_core::build_round(seed, 1, 44, 18),
            flash: None,
        },
        scan: numinous_core::build_scan(seed, 4),
        secret: numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4),
        wire: String::new(),
        wire_lines: Vec::new(),
        scores,
        cleared,
        message: String::new(),
    };
    Some(GauntletGameReplay {
        seed,
        play,
        rooms: numinous_core::all_rooms(),
    })
}

fn parse_studio_replay(
    arguments: &Map<String, Value>,
    result: &Map<String, Value>,
) -> Option<StudioReplay> {
    if arguments
        .keys()
        .any(|key| !matches!(key.as_str(), "expr" | "xmin" | "xmax" | "a"))
    {
        return None;
    }
    if result.len() != 2 || result.get("isError").and_then(Value::as_bool) != Some(false) {
        return None;
    }
    let blocks = result.get("content")?.as_array()?;
    if blocks.len() != 1 {
        return None;
    }
    let block = blocks.first()?.as_object()?;
    if block.len() != 2 || block.get("type").and_then(Value::as_str) != Some("text") {
        return None;
    }
    let result_text = block.get("text")?.as_str()?;
    let source = arguments.get("expr")?.as_str()?;
    if source.chars().count() > numinous_core::MAX_STUDIO_SOURCE_CHARS {
        return None;
    }
    let expression = numinous_core::parse(source).ok()?;
    let xmin = optional_finite(arguments.get("xmin"), -std::f64::consts::TAU)?;
    let xmax = optional_finite(arguments.get("xmax"), std::f64::consts::TAU)?;
    let parameter = optional_finite(arguments.get("a"), 1.0)?;
    let span = xmax - xmin;
    if xmax <= xmin || !span.is_finite() {
        return None;
    }
    let (plot, expected_ymin, expected_ymax) =
        numinous_core::plot_text(source, xmin, xmax, parameter, 72, 26).ok()?;
    let expected_result = format!(
        "y = {source}    x in [{xmin:.3}, {xmax:.3}]    y in [{expected_ymin:.3}, {expected_ymax:.3}]\n\n{plot}"
    );
    if result_text != expected_result {
        return None;
    }
    let (ymin, ymax) = crate::studio_render::curve_range(72, xmin, xmax, |x| {
        Some(numinous_core::eval(&expression, x, parameter))
    })?;
    if (ymin, ymax) != (expected_ymin, expected_ymax) {
        return None;
    }
    Some(StudioReplay {
        source: source.to_string(),
        expression,
        xmin,
        xmax,
        parameter,
        ymin,
        ymax,
    })
}

fn optional_finite(value: Option<&Value>, default: f64) -> Option<f64> {
    let value = match value {
        Some(value) => value.as_f64()?,
        None => default,
    };
    value.is_finite().then_some(value)
}

fn valid_optional_dimension(value: Option<&Value>, maximum: u64) -> bool {
    value.is_none_or(|value| {
        value
            .as_u64()
            .is_some_and(|size| (1..=maximum).contains(&size))
    })
}

fn optional_unit(value: Option<&Value>, default: f64, inclusive_one: bool) -> Option<f64> {
    let value = match value {
        Some(value) => value.as_f64()?,
        None => default,
    };
    let in_range = if inclusive_one {
        (0.0..=1.0).contains(&value)
    } else {
        (0.0..1.0).contains(&value)
    };
    (value.is_finite() && in_range).then_some(value)
}

fn parse_room_pokes(value: Option<&Value>) -> Option<Vec<(f64, f64)>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    let points = value.as_array()?;
    if points.len() > numinous_core::MAX_ROOM_POKES {
        return None;
    }
    points
        .iter()
        .map(|point| {
            let pair = point.as_array()?;
            if pair.len() != 2 {
                return None;
            }
            let x = optional_unit(pair.first(), 0.0, true)?;
            let y = optional_unit(pair.get(1), 0.0, true)?;
            Some((x, y))
        })
        .collect()
}

fn parse_room_gesture(value: Option<&Value>) -> Option<Vec<RoomInput>> {
    let Some(value) = value else {
        return Some(Vec::new());
    };
    let events = value.as_array()?;
    if events.len() > numinous_core::MAX_ROOM_INPUTS {
        return None;
    }
    events
        .iter()
        .map(|event| {
            let fields = event.as_object()?;
            let kind = fields.get("kind")?.as_str()?;
            if kind == "cancel" {
                return (fields.len() == 1).then_some(RoomInput::PointerCancel);
            }
            if !matches!(kind, "down" | "move" | "up")
                || fields.len() != 4
                || fields
                    .keys()
                    .any(|key| !matches!(key.as_str(), "kind" | "x" | "y" | "t"))
            {
                return None;
            }
            let x = optional_unit(fields.get("x"), 0.0, true)?;
            let y = optional_unit(fields.get("y"), 0.0, true)?;
            let t = optional_unit(fields.get("t"), 0.0, true)?;
            match kind {
                "down" => Some(RoomInput::PointerDown { x, y, t }),
                "move" => Some(RoomInput::PointerMove { x, y, t }),
                "up" => Some(RoomInput::PointerUp { x, y, t }),
                _ => None,
            }
        })
        .collect()
}

fn draw_native_replay_chrome(
    raster: &mut Raster,
    snapshot: &ViewerSnapshot,
    replay: &NativeReplay,
    input_mode: ViewerInputMode,
    width: usize,
    height: usize,
) {
    let top_height = height.min(31);
    let footer_top = height.saturating_sub(13);
    raster.dim_rows(0, top_height as i32, 12);
    raster.dim_rows(footer_top as i32, height as i32, 12);
    let event_number = snapshot.selected_index.map_or(0, |index| index + 1);
    let sequence = snapshot
        .event
        .as_ref()
        .map_or(0, |event| event.public_sequence);
    let display_state = if snapshot.display_paused {
        "DISPLAY PAUSED / "
    } else {
        ""
    };
    let heading = format!(
        "WATCH AGENT / {}{} / EVENT {} OF {} / PUBLIC SEQUENCE {}",
        display_state,
        snapshot.status.label(),
        event_number,
        snapshot.event_count,
        sequence
    );
    let detail = replay.detail();
    numinous_core::draw_text(raster, &heading, 6, 3, 1, '#');
    numinous_core::draw_text(raster, &detail, 6, 12, 1, '#');
    numinous_core::draw_text(
        raster,
        "PRIVATE ACTIVITY IS NEVER REPRESENTED",
        6,
        21,
        1,
        '#',
    );
    let controls = match input_mode {
        ViewerInputMode::KeyboardMouse => {
            "LEFT/RIGHT EVENT   SPACE PAUSE   ESC CLOSE   TEXT EVENTS USE UP/DOWN AND A/D"
        }
        ViewerInputMode::Controller => {
            "D-PAD EVENT   R3 PAUSE DISPLAY   EAST CLOSE   TEXT EVENTS USE D-PAD AND LB/RB"
        }
    };
    numinous_core::draw_text(
        raster,
        controls,
        6,
        footer_top.saturating_add(3) as i32,
        1,
        '#',
    );
    if width > 0 && height > 0 {
        raster.line(0, 0, width.saturating_sub(1) as i32, 0, '-');
        raster.line(
            0,
            height.saturating_sub(1) as i32,
            width.saturating_sub(1) as i32,
            height.saturating_sub(1) as i32,
            '-',
        );
    }
}

fn single_line(text: &str, maximum_chars: usize) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(maximum_chars)
        .collect()
}

impl Drop for SessionViewer {
    fn drop(&mut self) {
        self.close();
    }
}

/// A sanitized failure to create the local session viewer listener.
#[derive(Debug)]
pub struct ViewerOpenError(ViewerOpenErrorKind);

#[derive(Debug)]
enum ViewerOpenErrorKind {
    Io(io::ErrorKind),
    Pairing(numinous_broadcast::PairingError),
    InvalidLocalEndpoint,
    Compatibility,
    Clock,
}

impl From<io::Error> for ViewerOpenError {
    fn from(error: io::Error) -> Self {
        Self(ViewerOpenErrorKind::Io(error.kind()))
    }
}

impl From<numinous_broadcast::PairingError> for ViewerOpenError {
    fn from(error: numinous_broadcast::PairingError) -> Self {
        Self(ViewerOpenErrorKind::Pairing(error))
    }
}

impl fmt::Display for ViewerOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ViewerOpenErrorKind::Io(kind) => {
                write!(formatter, "local viewer I/O failed: {kind:?}")
            }
            ViewerOpenErrorKind::Pairing(error) => {
                write!(formatter, "local viewer pairing failed: {error}")
            }
            ViewerOpenErrorKind::InvalidLocalEndpoint => {
                formatter.write_str("invalid local viewer endpoint")
            }
            ViewerOpenErrorKind::Compatibility => {
                formatter.write_str("viewer compatibility is unavailable")
            }
            ViewerOpenErrorKind::Clock => {
                formatter.write_str("viewer pairing deadline is unavailable")
            }
        }
    }
}

impl Error for ViewerOpenError {}

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
    input_mode: ViewerInputMode,
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
    input_mode: ViewerInputMode,
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
        ViewerInputMode::KeyboardMouse => {
            "LEFT/RIGHT EVENT   UP/DOWN RESULT   A/D PAN   SPACE PAUSE   ESC CLOSE"
        }
        ViewerInputMode::Controller => {
            "D-PAD EVENT/RESULT   LB/RB PAN   R3 PAUSE DISPLAY   EAST CLOSE"
        }
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

    fn replay_fixture(sequence: u64, arguments: Value) -> EventEnvelope<PublicToolEvent> {
        let event = PublicToolEvent::new(
            PublicTool::PlayRoom,
            &arguments,
            &serde_json::json!({"content": [{"type": "text", "text": "PUBLIC RESULT"}]}),
        )
        .expect("public replay event");
        EventEnvelope {
            session_id: numinous_broadcast::SessionId::generate().expect("session"),
            consent_epoch: 2,
            public_sequence: sequence,
            skipped: None,
            compatibility: numinous_compatibility().expect("compatibility"),
            event,
        }
    }

    fn studio_fixture(
        sequence: u64,
        arguments: Value,
        result: Value,
    ) -> EventEnvelope<PublicToolEvent> {
        let event = PublicToolEvent::new(PublicTool::PlotExpression, &arguments, &result)
            .expect("public Studio event");
        EventEnvelope {
            session_id: numinous_broadcast::SessionId::generate().expect("session"),
            consent_epoch: 2,
            public_sequence: sequence,
            skipped: None,
            compatibility: numinous_compatibility().expect("compatibility"),
            event,
        }
    }

    fn nim_fixture(
        sequence: u64,
        arguments: Value,
        result: Value,
    ) -> EventEnvelope<PublicToolEvent> {
        let event =
            PublicToolEvent::new(PublicTool::Nim, &arguments, &result).expect("public Nim event");
        EventEnvelope {
            session_id: numinous_broadcast::SessionId::generate().expect("session"),
            consent_epoch: 2,
            public_sequence: sequence,
            skipped: None,
            compatibility: numinous_compatibility().expect("compatibility"),
            event,
        }
    }

    fn studio_result(source: &str, xmin: f64, xmax: f64, parameter: f64) -> Value {
        let (plot, ymin, ymax) = numinous_core::plot_text(source, xmin, xmax, parameter, 72, 26)
            .expect("valid Studio fixture");
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "y = {source}    x in [{xmin:.3}, {xmax:.3}]    y in [{ymin:.3}, {ymax:.3}]\n\n{plot}"
                )
            }],
            "isError": false
        })
    }

    fn cached_render_count(viewer: &SessionViewer) -> usize {
        viewer
            .cached_replay
            .as_ref()
            .and_then(|cached| cached.1.as_ref())
            .map_or(0, |replay| replay.render_count.get())
    }

    fn wait_until(mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !condition() {
            assert!(Instant::now() < deadline, "condition timed out");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn nim_history_for(
        seed: u64,
        winner: numinous_core::nim::NimWinner,
    ) -> Vec<numinous_core::nim::NimTurn> {
        fn search(
            seed: u64,
            turns: Vec<numinous_core::nim::NimTurn>,
            winner: numinous_core::nim::NimWinner,
        ) -> Option<Vec<numinous_core::nim::NimTurn>> {
            let replay = numinous_core::nim::replay(seed, &turns).ok()?;
            if replay.winner == Some(winner) {
                return Some(turns);
            }
            if replay.winner.is_some() {
                return None;
            }
            for (heap, count) in replay.heaps.iter().copied().enumerate() {
                for take in 1..=count {
                    let mut next = turns.clone();
                    next.push(numinous_core::nim::NimTurn { heap, take });
                    if let Some(found) = search(seed, next, winner) {
                        return Some(found);
                    }
                }
            }
            None
        }

        search(seed, Vec::new(), winner).expect("requested Nim outcome is reachable")
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
        assert!(Error::source(&io_error).is_none());

        let pairing_error = ViewerOpenError::from(PairingError::InvalidCode);
        assert_eq!(
            pairing_error.to_string(),
            "local viewer pairing failed: invalid pairing code"
        );
        assert!(Error::source(&pairing_error).is_none());

        for (error, expected) in [
            (
                ViewerOpenError(ViewerOpenErrorKind::InvalidLocalEndpoint),
                "invalid local viewer endpoint",
            ),
            (
                ViewerOpenError(ViewerOpenErrorKind::Compatibility),
                "viewer compatibility is unavailable",
            ),
            (
                ViewerOpenError(ViewerOpenErrorKind::Clock),
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
        let closed = viewer.draw(900, 700, ViewerInputMode::KeyboardMouse);
        assert_eq!((closed.width(), closed.height()), (900, 700));
        assert!(closed.lit_count() > 0);

        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::AwaitingGuest;
            shared.pairing_code = Some("PAIRING-CODE-WITH-A-LONG-LOCAL-CAPABILITY".to_string());
        }
        let pairing = viewer.draw(180, 90, ViewerInputMode::Controller);
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
        let retained = viewer.draw(240, 140, ViewerInputMode::Controller);
        assert_eq!((retained.width(), retained.height()), (240, 140));
        assert!(retained.lit_count() > 0);
        let cached = viewer.draw(240, 140, ViewerInputMode::KeyboardMouse);
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
        let (large, scale, _) = layout_lines(
            &live_waiting,
            ViewerInputMode::KeyboardMouse,
            900,
            700,
            0,
            0,
        );
        assert_eq!(scale, 2);
        assert!(
            large
                .iter()
                .any(|line| line.contains("WAITING FOR THE FIRST"))
        );
        let (compact, scale, _) =
            layout_lines(&live_waiting, ViewerInputMode::Controller, 80, 30, 0, 0);
        assert_eq!(scale, 1);
        assert!(!compact.is_empty());
        assert_eq!(public_result_lines(&Map::new()), ["{}".to_string()]);
    }

    #[test]
    fn native_times_tables_replay_matches_the_core_room_frame() {
        let arguments = serde_json::json!({
            "id": "times-tables",
            "t": 0.81,
            "width": 40,
            "height": 20,
            "variation": 42,
            "pokes": [[0.375, 0.5]]
        });
        let replay = parse_room_replay(arguments.as_object().expect("arguments"))
            .expect("valid native replay");
        let actual = replay.render(320, 180);

        let room = numinous_core::all_rooms_with(42)
            .into_iter()
            .find(|room| room.meta().id == "times-tables")
            .expect("times tables");
        let inputs = numinous_core::inputs_from_pokes(&[(0.375, 0.5)], 0.81);
        let mut expected = Raster::with_accent(320, 180, room.meta().accent);
        room.render_input(&mut expected, 0.81, &inputs);

        assert_eq!(actual.to_rgba(), expected.to_rgba());
        assert_eq!(
            replay.status().as_deref(),
            Some("K 5.00  CLOSED  4 LOBES  FOUND")
        );
    }

    #[test]
    fn native_room_replay_revalidates_every_bounded_input() {
        let too_many_pokes: Vec<_> = (0..=numinous_core::MAX_ROOM_POKES)
            .map(|_| serde_json::json!([0.5, 0.5]))
            .collect();
        for invalid in [
            serde_json::json!({"t": 0.25}),
            serde_json::json!({"id": "times-tables", "t": 1.0}),
            serde_json::json!({"id": "times-tables", "width": 0}),
            serde_json::json!({"id": "times-tables", "height": 257}),
            serde_json::json!({"id": "not-a-room"}),
            serde_json::json!({"id": "times-tables", "private": "not declared"}),
            serde_json::json!({"id": "times-tables", "pokes": too_many_pokes}),
            serde_json::json!({
                "id": "times-tables",
                "pokes": [[0.5, 0.5]],
                "gesture": [{"kind": "cancel"}]
            }),
            serde_json::json!({
                "id": "times-tables",
                "gesture": [{"kind": "move", "x": 0.5, "y": 0.5, "t": 0.2, "extra": 1}]
            }),
        ] {
            assert!(
                parse_room_replay(invalid.as_object().expect("arguments")).is_none(),
                "accepted malformed replay: {invalid}"
            );
        }

        let gesture = serde_json::json!({
            "id": "times-tables",
            "t": 0.5,
            "gesture": [
                {"kind": "down", "x": 0.25, "y": 0.5, "t": 0.4},
                {"kind": "move", "x": 0.375, "y": 0.5, "t": 0.5},
                {"kind": "up", "x": 0.375, "y": 0.5, "t": 0.5},
                {"kind": "cancel"}
            ]
        });
        assert!(parse_room_replay(gesture.as_object().expect("arguments")).is_some());
    }

    #[test]
    fn native_studio_replay_requires_an_exact_successful_finite_plot() {
        let source = "sin(3*x) + x/2";
        let success = studio_result(source, -2.0, 3.0, 0.5);
        let valid = PublicToolEvent::new(
            PublicTool::PlotExpression,
            &serde_json::json!({
                "expr": source, "xmin": -2.0, "xmax": 3.0, "a": 0.5
            }),
            &success,
        )
        .expect("valid public event");
        let replay = parse_native_replay(&valid).expect("valid native Studio replay");
        assert!(matches!(replay.kind, NativeReplayKind::Studio(_)));
        let frame = replay.render(320, 180);
        assert!(frame.lit_count() > 100);
        assert!(replay.detail().contains("SIN(3*X) + X/2"));

        let too_long = "x".repeat(numinous_core::MAX_STUDIO_SOURCE_CHARS + 1);
        for arguments in [
            serde_json::json!({}),
            serde_json::json!({"expr": 7}),
            serde_json::json!({"expr": "x", "private": true}),
            serde_json::json!({"expr": "x", "a": "2"}),
            serde_json::json!({"expr": "x", "xmin": null}),
            serde_json::json!({"expr": "sin("}),
            serde_json::json!({"expr": "x", "xmin": 1.0, "xmax": 1.0}),
            serde_json::json!({"expr": "x", "xmin": -1.0e308, "xmax": 1.0e308}),
            serde_json::json!({"expr": "sqrt(-1)"}),
            serde_json::json!({"expr": too_long}),
        ] {
            let event = PublicToolEvent::new(PublicTool::PlotExpression, &arguments, &success)
                .expect("public event envelope");
            assert!(
                parse_native_replay(&event).is_none(),
                "accepted malformed Studio replay: {arguments}"
            );
        }

        for result in [
            serde_json::json!({}),
            serde_json::json!({"content": [{"text": "missing type"}]}),
            serde_json::json!({"content": [{"type": "image", "text": "wrong type"}]}),
            serde_json::json!({"isError": true, "content": [{"text": "bad"}]}),
            serde_json::json!({"isError": "false", "content": [{"text": "bad"}]}),
            serde_json::json!({
                "isError": false,
                "content": [{"type": "text", "text": "forged result"}]
            }),
        ] {
            let event = PublicToolEvent::new(
                PublicTool::PlotExpression,
                &serde_json::json!({"expr": "x"}),
                &result,
            )
            .expect("public event envelope");
            assert!(parse_native_replay(&event).is_none());
        }
    }

    #[test]
    fn studio_native_body_uses_the_shared_cache_and_invalid_actions_fall_back() {
        let mut viewer = SessionViewer::default();
        let source = "sin(a*x) + x/3";
        let success = studio_result(source, -std::f64::consts::TAU, std::f64::consts::TAU, 2.0);
        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::Live;
            shared
                .retain(&studio_fixture(
                    0,
                    serde_json::json!({"expr": source, "a": 2.0}),
                    success.clone(),
                ))
                .expect("retain valid Studio event");
        }
        let first = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert!(first.lit_count() > 100);
        assert_eq!(cached_render_count(&viewer), 1);
        let repeated = viewer.draw(320, 180, ViewerInputMode::Controller);
        assert_eq!(cached_render_count(&viewer), 1);
        assert_ne!(
            first.to_rgba(),
            repeated.to_rgba(),
            "dynamic chrome updates"
        );
        let resized = viewer.draw(400, 200, ViewerInputMode::KeyboardMouse);
        assert_eq!((resized.width(), resized.height()), (400, 200));
        assert_eq!(cached_render_count(&viewer), 2);

        lock(&viewer.shared)
            .retain(&studio_fixture(
                1,
                serde_json::json!({"expr": "sqrt(-1)"}),
                success,
            ))
            .expect("retain invalid Studio event");
        let fallback = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert!(fallback.lit_count() > 0);
        assert!(matches!(viewer.cached_replay, Some((1, None))));
    }

    #[test]
    fn native_nim_replay_requires_exact_rules_arguments_and_result() {
        let seed = 23;
        let replay = numinous_core::nim::replay(seed, &[]).expect("opening Nim replay");
        let result = canonical_nim_result(seed, &replay);
        let event =
            PublicToolEvent::new(PublicTool::Nim, &serde_json::json!({"seed": seed}), &result)
                .expect("valid public Nim event");
        let native = parse_native_replay(&event).expect("valid native Nim replay");
        assert!(matches!(native.kind, NativeReplayKind::Nim(_)));
        assert_eq!(
            native.render(360, 220).to_rgba(),
            crate::nim_render::draw_nim_board(&replay.heaps, None, 360, 220)
                .expect("shared Nim board")
                .to_rgba()
        );
        assert!(native.detail().contains("NIM / SEED 23 / IN PROGRESS"));

        let too_many_moves =
            vec![serde_json::json!([1, 1]); numinous_core::nim::MAX_REPLAY_TURNS + 1];
        for arguments in [
            serde_json::json!({"seed": "23"}),
            serde_json::json!({"seed": seed, "private": true}),
            serde_json::json!({"seed": seed, "moves": "none"}),
            serde_json::json!({"seed": seed, "moves": [[1]]}),
            serde_json::json!({"seed": seed, "moves": [[0, 1]]}),
            serde_json::json!({"seed": seed, "moves": [[4, 1]]}),
            serde_json::json!({"seed": seed, "moves": [[1, 0]]}),
            serde_json::json!({"seed": seed, "moves": [[1, 8]]}),
            serde_json::json!({"seed": seed, "moves": too_many_moves}),
        ] {
            let event = PublicToolEvent::new(PublicTool::Nim, &arguments, &result)
                .expect("public event envelope");
            assert!(
                parse_native_replay(&event).is_none(),
                "accepted malformed Nim replay: {arguments}"
            );
        }

        for forged in [
            serde_json::json!({}),
            serde_json::json!({
                "content": [{"type": "text", "text": "forged"}],
                "structuredContent": {
                    "game": "nim", "seed": seed, "heaps": replay.heaps, "order": []
                },
                "isError": false
            }),
            serde_json::json!({
                "content": result["content"],
                "structuredContent": {
                    "game": "nim", "seed": seed, "heaps": [7, 7, 7], "order": []
                },
                "isError": false
            }),
        ] {
            let event =
                PublicToolEvent::new(PublicTool::Nim, &serde_json::json!({"seed": seed}), &forged)
                    .expect("public event envelope");
            assert!(parse_native_replay(&event).is_none());
        }
    }

    #[test]
    fn native_nim_replay_attests_both_terminal_result_shapes() {
        let seed = 23;
        for winner in [
            numinous_core::nim::NimWinner::Player,
            numinous_core::nim::NimWinner::Order,
        ] {
            let turns = nim_history_for(seed, winner);
            let replay = numinous_core::nim::replay(seed, &turns).expect("terminal replay");
            assert_eq!(replay.winner, Some(winner));
            let moves = turns
                .iter()
                .map(|turn| serde_json::json!([turn.heap + 1, turn.take]))
                .collect::<Vec<_>>();
            let arguments = serde_json::json!({"seed": seed, "moves": moves});
            let result = canonical_nim_result(seed, &replay);
            let event = PublicToolEvent::new(PublicTool::Nim, &arguments, &result)
                .expect("terminal public event");
            let native = parse_native_replay(&event).expect("attested terminal replay");
            let expected_detail = match winner {
                numinous_core::nim::NimWinner::Player => "PLAYER WON",
                numinous_core::nim::NimWinner::Order => "ORDER WON",
            };
            assert!(native.detail().contains(expected_detail));

            let mut forged = result;
            forged["content"][0]["text"] = serde_json::json!("forged");
            let event = PublicToolEvent::new(PublicTool::Nim, &arguments, &forged)
                .expect("forged terminal event");
            assert!(parse_native_replay(&event).is_none());
        }
    }

    #[test]
    fn nim_native_body_uses_the_shared_cache_and_invalid_actions_fall_back() {
        let mut viewer = SessionViewer::default();
        let seed = 23;
        let replay = numinous_core::nim::replay(seed, &[]).expect("opening Nim replay");
        let result = canonical_nim_result(seed, &replay);
        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::Live;
            shared
                .retain(&nim_fixture(
                    0,
                    serde_json::json!({"seed": seed}),
                    result.clone(),
                ))
                .expect("retain valid Nim event");
        }
        let first = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
        assert!(first.lit_count() > 100);
        assert_eq!(cached_render_count(&viewer), 1);
        let repeated = viewer.draw(360, 220, ViewerInputMode::Controller);
        assert_eq!(cached_render_count(&viewer), 1);
        assert_ne!(first.to_rgba(), repeated.to_rgba());

        lock(&viewer.shared)
            .retain(&nim_fixture(
                1,
                serde_json::json!({"seed": seed, "moves": [[1, 8]]}),
                result,
            ))
            .expect("retain invalid Nim event");
        let fallback = viewer.draw(360, 220, ViewerInputMode::KeyboardMouse);
        assert!(fallback.lit_count() > 0);
        assert!(matches!(viewer.cached_replay, Some((1, None))));
    }

    #[test]
    fn draw_uses_native_replay_and_falls_back_to_text_on_invalid_actions() {
        let mut viewer = SessionViewer::default();
        let envelope = replay_fixture(
            0,
            serde_json::json!({"id": "times-tables", "t": 0.81, "pokes": [[0.375, 0.5]]}),
        );
        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::Live;
            shared.retain(&envelope).expect("retain native event");
        }
        let frame = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert!(frame.lit_count() > 1_000);
        assert!(matches!(viewer.cached_replay, Some((0, Some(_)))));

        let envelope = replay_fixture(1, serde_json::json!({"id": "times-tables", "t": 2.0}));
        lock(&viewer.shared)
            .retain(&envelope)
            .expect("retain invalid event");
        let fallback = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert!(fallback.lit_count() > 0);
        assert!(matches!(viewer.cached_replay, Some((1, None))));
    }

    #[test]
    fn native_body_cache_tracks_sequence_dimensions_and_dynamic_chrome() {
        let mut viewer = SessionViewer::default();
        {
            let mut shared = lock(&viewer.shared);
            shared.status = ViewerStatus::Live;
            shared
                .retain(&replay_fixture(
                    0,
                    serde_json::json!({"id": "times-tables", "t": 0.2}),
                ))
                .expect("retain first replay");
            shared
                .retain(&replay_fixture(
                    1,
                    serde_json::json!({
                        "id": "times-tables", "t": 0.81, "pokes": [[0.375, 0.5]]
                    }),
                ))
                .expect("retain second replay");
        }

        let live = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert_eq!(cached_render_count(&viewer), 1);
        let repeated = viewer.draw(320, 180, ViewerInputMode::Controller);
        assert_eq!(cached_render_count(&viewer), 1);
        assert_ne!(live.to_rgba(), repeated.to_rgba(), "control chrome updates");

        viewer.toggle_display_pause();
        let paused = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert_eq!(cached_render_count(&viewer), 1);
        assert_ne!(live.to_rgba(), paused.to_rgba(), "pause chrome updates");

        let resized = viewer.draw(400, 200, ViewerInputMode::KeyboardMouse);
        assert_eq!((resized.width(), resized.height()), (400, 200));
        assert_eq!(cached_render_count(&viewer), 2);

        let empty = viewer.draw(0, 0, ViewerInputMode::Controller);
        assert_eq!((empty.width(), empty.height()), (0, 0));
        assert_eq!(cached_render_count(&viewer), 3);

        viewer.scrub(-1);
        let earlier = viewer.draw(320, 180, ViewerInputMode::KeyboardMouse);
        assert_eq!((earlier.width(), earlier.height()), (320, 180));
        assert_eq!(
            viewer.cached_replay.as_ref().map(|cached| cached.0),
            Some(0)
        );
        assert_eq!(cached_render_count(&viewer), 1);
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
        let code = viewer.pairing_code().expect("pairing code");
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
        assert_eq!(viewer.retained_events().len(), 1);
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
        let copy =
            semantic_lines(&snapshot, ViewerInputMode::KeyboardMouse, 80, 40, 0, 0).join("\n");
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
