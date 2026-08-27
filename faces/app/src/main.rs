#![windows_subsystem = "windows"]
//! Numinous windowed app.
//!
//! Opens a real window and shows a room animating in full color, rendered on the
//! CPU into a pixel buffer (the same `Raster` the CLI writes to PNG). Left/right
//! switch rooms, space pauses, escape quits. This is the start of the GUI
//! Cabinet (see `docs/DESIGN.md`). Its default `winit` and `softbuffer` path runs
//! on macOS, Linux, and Windows. The disabled `gpu-post` feature presents the
//! same room raster through the measured Sensory Lift candidate and falls back
//! visibly to software when that path is unavailable.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use numinous_app::session_viewer::{SessionViewer, ViewerInputMode};
use numinous_core::{Journey, ROOM_BED_SOURCE_RATE, Raster, Room, Surface, all_rooms_with};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Icon, Window, WindowId};

mod audio_state;
mod bindings;
mod console;
mod feedback;
mod gallery;
mod game_runtime;
mod gamepad;
mod hud;
mod input_feedback;
mod input_runtime;
mod live_render;
mod mouse_input;
mod overlays;
mod playtest;
mod postcard;
mod presentation;
mod radio_cache;
mod room_input;
mod save_gate;
mod session_audio;
mod studio_panel;
mod wager;

use crate::audio_state::Program as AudioProgram;
use crate::session_audio::SessionAudio;
use numinous_app::{controls, game_draw, input_legend, menu, play, room_phase};
use play::{ArcadePlay, GauntletPlay, MunchPlay, NimPlay, QuizPlay};
use room_phase::{effective_room_phase, has_finite_parameter_input};

/// Frames of The Show crossfade when the gallery advances rooms.
const SHOW_CROSSFADE_FRAMES: u8 = 14;
/// Wall time for the Times Tables cardioid-to-Mandelbrot morph beat.
const TIMES_TABLES_MORPH_SECONDS: f64 = 1.6;
/// Wall time for the Buffon circle-grows-from-sticks morph beat.
const BUFFON_MORPH_SECONDS: f64 = 1.6;
/// Wall time for the Double Pendulum divergence-curve morph beat.
const PENDULUM_MORPH_SECONDS: f64 = 1.6;
/// Wall time for the Kepler equal-time-mark morph beat.
const KEPLER_MORPH_SECONDS: f64 = 1.6;
/// Wall time for the Parrondo exact-expectation morph beat.
const PARRONDO_MORPH_SECONDS: f64 = 1.6;
const NONTRANSITIVE_MORPH_SECONDS: f64 = 1.6;

/// Blend `prev` into `dest` with `weight` of the previous frame in [0, 1].
fn blend_rgba(dest: &mut [u8], prev: &[u8], weight: f32) {
    let w = weight.clamp(0.0, 1.0);
    let inv = 1.0 - w;
    for (d, p) in dest.iter_mut().zip(prev.iter()) {
        *d = (f32::from(*d) * inv + f32::from(*p) * w) as u8;
    }
}

/// Shift an RGBA buffer a few pixels left or right for a short bad-grade shake.
fn apply_screen_shake(rgba: &mut [u8], width: usize, height: usize, frames_left: u8) {
    if width < 4 || height == 0 || rgba.len() < width * height * 4 {
        return;
    }
    let shift = if frames_left.is_multiple_of(2) {
        3_isize
    } else {
        -3_isize
    };
    let row_bytes = width * 4;
    let mut shifted = vec![10_u8, 11, 15, 255]
        .into_iter()
        .cycle()
        .take(rgba.len())
        .collect::<Vec<_>>();
    for y in 0..height {
        let src = y * row_bytes;
        for x in 0..width {
            let dest_x = x as isize + shift;
            if dest_x < 0 || dest_x >= width as isize {
                continue;
            }
            let from = src + x * 4;
            let to = src + dest_x as usize * 4;
            shifted[to..to + 4].copy_from_slice(&rgba[from..from + 4]);
        }
    }
    rgba.copy_from_slice(&shifted);
}
#[cfg(test)]
fn mandelbrot_gpu_view(
    t: f64,
    variation: u64,
    width: u32,
    height: u32,
    inputs: &[numinous_core::RoomInput],
) -> (f32, f32, f32) {
    let (center_x, center_y, horizontal_half_span) =
        numinous_core::rooms::mandelbrot::selected_view_input(
            inputs,
            width as usize,
            height as usize,
            variation,
            t,
        );
    let vertical_span = if width == 0 {
        0.0
    } else {
        2.0 * horizontal_half_span * f64::from(height) / f64::from(width)
    };
    (center_x as f32, center_y as f32, vertical_span as f32)
}

fn live_mandelbrot_gpu_view(
    camera: numinous_core::rooms::mandelbrot::MandelbrotCamera,
    width: u32,
    height: u32,
) -> Option<(f32, f32, f32)> {
    let (center_x, center_y, horizontal_half_span) = camera.view();
    if width == 0
        || height == 0
        || !center_x.is_finite()
        || !center_y.is_finite()
        || !horizontal_half_span.is_finite()
        || horizontal_half_span <= 0.0
    {
        return None;
    }
    let center_x_f32 = center_x as f32;
    let center_y_f32 = center_y as f32;
    let pixel_step = 2.0 * horizontal_half_span / f64::from(width);
    let spacing = f32_spacing(center_x_f32).max(f32_spacing(center_y_f32));
    if pixel_step < spacing {
        return None;
    }
    let vertical_span = 2.0 * horizontal_half_span * f64::from(height) / f64::from(width);
    Some((center_x_f32, center_y_f32, vertical_span as f32))
}

fn f32_spacing(value: f32) -> f64 {
    if !value.is_finite() {
        return f64::INFINITY;
    }
    let adjacent_bits = if value >= 0.0 {
        value.to_bits().saturating_add(1)
    } else {
        value.to_bits().saturating_sub(1)
    };
    f64::from((f32::from_bits(adjacent_bits) - value).abs())
}

fn julia_gpu_c(t: f64, variation: u64, pokes: &[(f64, f64)]) -> (f32, f32) {
    let (cx, cy) = numinous_core::rooms::julia::selected_c(t, variation, pokes);
    (cx as f32, cy as f32)
}

fn julia_gpu_vertical_span(width: u32, height: u32) -> f32 {
    if width == 0 {
        0.0
    } else {
        3.2 * height as f32 / width as f32
    }
}

/// Normal room phase cycles per elapsed second.
///
/// Slightly above a quarter-cycle per second so quiet rooms still feel alive
/// without rushing the math past readability.
const T_RATE: f64 = 0.30;
/// The Show advances more slowly for a deliberate, hypnotic pace.
const SHOW_T_RATE: f64 = 0.11;
/// A restored or stalled window never consumes a giant simulation step.
const MAX_TICK_SECONDS: f64 = 0.05;
/// Target presentation cadence. The simulation still uses measured time.
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
/// Deliberate Life cadence: fast enough to move, slow enough to read births.
const LIFE_STEP_SECONDS: f64 = 0.12;
/// One presentation tick cannot consume an unbounded simulation backlog.
const MAX_LIFE_STEPS_PER_TICK: usize = 8;

fn bounded_tick_seconds(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64().clamp(0.0, MAX_TICK_SECONDS)
}

fn fullscreen_toggle_target(is_fullscreen: bool) -> Option<winit::window::Fullscreen> {
    if is_fullscreen {
        None
    } else {
        Some(winit::window::Fullscreen::Borderless(None))
    }
}

/// How much of this tick the ambient world may consume.
///
/// Reduced motion hands it zero seconds, so everything that moves whether or
/// not the player engages stops: the room phase, The Show's drift into the next
/// room (which only happens when a phase sweep completes), the Mandelbrot
/// camera, and the Life cadence. The player's own input is untouched, and the
/// tick still runs, so the App keeps drawing and keeps responding.
///
/// # What this deliberately does not stop
///
/// The boundary is ambient motion, not all motion, and saying otherwise would
/// overstate what the setting does. Three things keep their real time:
///
/// - The two engineered aha morphs. Short, bounded, and the direct completion
///   of an act the player just performed. Freezing them strands someone mid-aha
///   with no way to finish, which breaks a feature rather than calming one.
/// - Transient feedback: the arrival card countdown, bite and flash timers,
///   banner lifetimes. Each is a brief acknowledgement of something the player
///   did and ends on its own.
/// - The Munch Arcade beat, which steps the Vexations. That motion is the game.
///   A player has to choose to enter the Arcade, and freezing its hunters would
///   not calm the room, it would remove the thing being played.
///
/// If that boundary is ever judged wrong, the Arcade is the case to revisit
/// first: it is the only one of the three that runs for as long as the player
/// stays, rather than ending by itself.
fn ambient_tick_seconds(elapsed: f64, motion: numinous_core::Motion) -> f64 {
    if motion.animates() { elapsed } else { 0.0 }
}

fn advance_gallery_phase(
    phase: f64,
    elapsed: f64,
    time_scale: f64,
    rate: f64,
    first_contact_obscured: bool,
) -> (f64, bool) {
    if first_contact_obscured {
        return (phase, false);
    }
    let next = phase + rate * elapsed * time_scale;
    if next >= 1.0 {
        (next.rem_euclid(1.0), true)
    } else {
        (next, false)
    }
}

fn effective_room_inputs(
    inputs: &[numinous_core::RoomInput],
    the_show: bool,
) -> &[numinous_core::RoomInput] {
    if the_show { &[] } else { inputs }
}

fn selected_parameter_sound(
    program: AudioProgram,
    modal_active: bool,
    room: &dyn Room,
    phase: f64,
    inputs: &[numinous_core::RoomInput],
    the_show: bool,
) -> Option<numinous_core::ParametricSound> {
    if program != AudioProgram::RoomScore
        || modal_active
        || !the_show && !has_finite_parameter_input(inputs)
    {
        return None;
    }
    let effective_phase = effective_room_phase(room.meta().id, phase, inputs, the_show);
    room.parameter_sound(effective_phase, effective_room_inputs(inputs, the_show))
}

fn life_step_audio_owned(program: AudioProgram, modal_active: bool, room_id: &str) -> bool {
    room_transient_audio_owned(program, modal_active) && room_id == "game-of-life"
}

fn room_transient_audio_owned(program: AudioProgram, modal_active: bool) -> bool {
    program == AudioProgram::RoomScore && !modal_active
}

fn selected_life_step_audio(
    program: AudioProgram,
    modal_active: bool,
    muted: bool,
    completed_steps: usize,
    session: &numinous_core::rooms::game_of_life::LifeSession,
    sample_rate: u32,
) -> Option<Vec<f32>> {
    if !life_step_audio_owned(program, modal_active, "game-of-life")
        || muted
        || completed_steps == 0
    {
        return None;
    }
    let samples = session.step_sound().render_stereo(sample_rate);
    (!samples.is_empty()).then_some(samples)
}

fn selected_room_interaction_audio(
    program: AudioProgram,
    modal_active: bool,
    muted: bool,
    accepted: bool,
    room: &dyn Room,
    inputs: &[numinous_core::RoomInput],
    sample_rate: u32,
) -> Option<Vec<f32>> {
    if program != AudioProgram::RoomScore || modal_active || muted || !accepted {
        return None;
    }
    room.interaction_stereo(inputs, sample_rate)
        .filter(|samples| !samples.is_empty())
}

/// Which local store had save trouble. The journey and the scores fail
/// independently, so each carries its own once-per-spell warning latch.
#[derive(Clone, Copy)]
enum SaveStore {
    Journey,
    Scores,
    Preferences,
}

/// The two save-trouble warning lines, named so the level-up celebration can
/// recognize a warning on screen and decline to paint over it.
const JOURNEY_SAVE_WARNING: &str = "PROGRESS IS NOT SAVING  SEE .NUMINOUS-CRASH.LOG";
const SCORE_SAVE_WARNING: &str = "SCORES ARE NOT SAVING  SEE .NUMINOUS-CRASH.LOG";
const PREFERENCES_SAVE_WARNING: &str = "SETTINGS ARE NOT SAVING  SEE .NUMINOUS-CRASH.LOG";
const PREFERENCES_LOAD_WARNING: &str = "SETTINGS COULD NOT BE LOADED  SEE .NUMINOUS-CRASH.LOG";

/// The application state driven by the winit event loop.
/// Which naming field the keyboard currently feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamingField {
    Title,
    Author,
}

/// The naming step's fields, exactly as the player left them.
///
/// Two levels of optionality, because two different questions are being
/// asked. Whether a share carries this at all answers "was the player
/// asked"; each field answers "what did they leave". Collapsing those into
/// one `Option` per field is what made a player who deleted a reopened
/// creation's name watch the old name ship anyway: the form said unnamed
/// while the capsule, the README, the postcard headline, and the folder
/// slug all said otherwise. An emptied field is a clearing, not an
/// absence.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ShareIdentity {
    title: Option<String>,
    author: Option<String>,
}

/// The F4 naming step's editable state: one text line for the creation's
/// name, one for its signature.
#[derive(Debug, Clone)]
struct ShareNaming {
    title: String,
    author: String,
    field: NamingField,
}

impl ShareNaming {
    /// The identity decision these fields carry, clearings included.
    fn identity(&self) -> ShareIdentity {
        let field = |value: &str| {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_string())
        };
        ShareIdentity {
            title: field(&self.title),
            author: field(&self.author),
        }
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.field {
            NamingField::Title => &mut self.title,
            NamingField::Author => &mut self.author,
        }
    }
}

struct App {
    /// How much the world may move on its own. Read once at construction, so
    /// every tick answers the same way and a test can pin it.
    motion: numinous_core::Motion,
    window: Option<Arc<Window>>,
    presenter: Option<presentation::WindowPresenter>,
    presentation_warned: bool,
    player: Option<numinous_audio::LoopPlayer>,
    #[cfg(test)]
    transient_audio_clears: std::cell::Cell<usize>,
    #[cfg(test)]
    interaction_audio_events: std::cell::Cell<usize>,
    gamepad: gamepad::GamepadInput,
    /// The last input family that performed a meaningful action.
    input_mode: input_legend::InputMode,
    /// Cached room-bed spectrum for the visualizer meter (room index + bands).
    spectrum_cache: Option<(usize, [f32; numinous_core::BAND_COUNT])>,
    /// Preferred visualizer sample source (room bed, output mix, loopback).
    visualizer_source: numinous_audio::VisualizerSource,
    /// Optional system loopback capture when the OS exposes a mix device.
    loopback: Option<numinous_audio::InputCapture>,
    /// Previous frame bands for onset / lever mapping.
    spectrum_prev: [f32; numinous_core::BAND_COUNT],
    /// Multiplier from live spectrum bass (1.0 when drive is off).
    visualizer_scale: f64,
    mandelbrot_camera: numinous_core::rooms::mandelbrot::MandelbrotCamera,
    life_session: numinous_core::rooms::game_of_life::LifeSession,
    life_accumulator: f64,
    /// Times Tables five-beat engineered aha for the ordinary App visit.
    times_tables_aha: numinous_core::rooms::times_tables_aha::TimesTablesAha,
    /// Buffon five-beat engineered aha for the ordinary App visit.
    buffon_aha: numinous_core::rooms::buffon_aha::BuffonAha,
    /// Staged Galton aha (third flagship): wager the pile's peak bin.
    galton_aha: numinous_core::rooms::galton_aha::GaltonAha,
    /// Staged Double Pendulum aha: call where the deterministic twin ends.
    pendulum_aha: numinous_core::rooms::pendulum_aha::PendulumAha,
    /// Staged Kepler aha: call how speed changes near the sun.
    kepler_aha: numinous_core::rooms::kepler_aha::KeplerAha,
    /// Staged Parrondo aha: call which policy wins in expectation.
    parrondo_aha: numinous_core::rooms::parrondo_aha::ParrondoAha,
    /// Staged Nontransitive Dice aha: choose first, then call the counter.
    nontransitive_aha: numinous_core::rooms::nontransitive_aha::NontransitiveAha,
    /// The universal readout wager, while one is posed on this room.
    room_wager: Option<wager::RoomWager>,
    rooms: Vec<Box<dyn Room>>,
    current: usize,
    t: f64,
    paused: bool,
    dragging: bool,
    show_info: bool,
    /// The Show: lean back and let the whole collection play itself.
    the_show: bool,
    /// Last presented Show frame for room-to-room crossfade.
    show_crossfade_prev: Option<Vec<u8>>,
    /// Remaining frames of Show crossfade blend.
    show_crossfade_frames: u8,
    /// The Studio: type an expression and watch it live.
    studio: bool,
    /// The typed Studio expression and its last-good parse state.
    studio_panel: studio_panel::StudioPanel,
    /// The F4 naming step: a share waiting for its title and signature.
    share_naming: Option<ShareNaming>,
    /// The author name from the last named share, offered on the next one.
    /// Names are the social currency of the fork loop; remembering the
    /// signature makes signing the default instead of a chore.
    remembered_author: String,
    /// The Gallery wall over the Studio, while the player is browsing it.
    gallery: Option<gallery::GalleryPanel>,
    /// Human-owned, read-only view of one explicitly paired MCP session.
    session_viewer: SessionViewer,
    /// Publishes each retained public sequence's sound at most once.
    session_audio: SessionAudio,
    /// GPU fractal renderer, when this machine has one (CPU raster otherwise).
    gpu: Option<numinous_gpu::FractalRenderer>,
    /// Adaptive live-render resolution for CPU room frames (see live_render).
    live_scale: live_render::LiveScale,
    /// The visual era ('e' cycles: phosphor, 8-bit, vector, modern).
    era: numinous_core::Era,
    /// Sound off ('m' toggles).
    muted: bool,
    /// Master volume, 0.0 to 1.0 ('[' and ']' step it globally).
    volume: f32,
    /// Window presentation saved for the next launch.
    preferred_window_mode: numinous_core::WindowModePreference,
    /// The program that owns the player source, independent of focus and gain.
    audio_program: AudioProgram,
    /// Whether menu or contextual help chrome is visible.
    show_help: bool,
    /// Typed Cabinet navigation state, shared by drawing and every input path.
    menu: menu::MenuState,
    /// Start in fullscreen from the launch option or environment.
    start_fullscreen: bool,
    /// A menu action requested the same orderly shutdown as the window button.
    quit_requested: bool,
    /// Whether the player has already been told the journey file is failing,
    /// so one trouble spell warns once instead of on every play.
    journey_save_warned: bool,
    /// The same spell for the scores file, held separately: the two stores
    /// fail independently, so one must not speak for the other.
    score_save_warned: bool,
    /// The same once-per-spell warning latch for App preferences.
    preferences_save_warned: bool,
    /// Where this App writes its diagnostics. The real crash log in the
    /// player's home by default; headless tests point it at scratch so a
    /// test failure cannot append to a real player's file.
    crash_log: std::path::PathBuf,
    /// A `.num` path or `numinous://` link from the launch arguments, opened
    /// into the Studio once the window exists. The file is a front door.
    start_open: Option<String>,
    /// Presentation frame counter for animation and game cadence.
    frame: u64,
    /// Elapsed-time anchor, so motion does not depend on event-loop load.
    last_tick: Instant,
    /// Focused windows animate and speak; background windows hold their state.
    window_active: bool,
    /// Wall-clock anchor used to reconcile presentation-only transitions.
    inactive_since: Option<Instant>,
    /// Time speed multiplier (W faster, S slower), like sprint and sneak.
    time_scale: f64,
    /// The player's journey: the same file the CLI levels (visits, plays, wins).
    journey: Journey,
    /// Last Journey state successfully merged into the local file.
    journey_saved: Journey,
    /// The level before the last change, to catch level-ups as they happen.
    level_seen: u32,
    /// Transient on-screen feedback such as LEVEL UP, volume, and save status.
    banner: Option<feedback::Banner>,
    /// Remaining presentation frames for a short camera shake (bad Munch grades).
    screen_shake: u8,
    /// Whether this visit's room goal has already raised its earned Aha.
    goal_announced: bool,
    /// The quiz, when playing: the round, its number, and the answer flash.
    quiz: Option<QuizPlay>,
    /// Rooms recently asked about, excluded from the next deals.
    quiz_recent: Vec<&'static str>,
    /// Munch, when playing in the window.
    munch: Option<MunchPlay>,
    /// The next standalone full-deck board to consider.
    munch_next_round: u64,
    /// The previous standalone rule, so consecutive boards change families.
    munch_last_rule: Option<numinous_core::munchers::Rule>,
    /// Nim, when playing in the window.
    nim: Option<NimPlay>,
    /// The Gauntlet, when running in the window.
    gauntlet: Option<GauntletPlay>,
    /// The arcade, when the Vexations are loose.
    arcade: Option<ArcadePlay>,
    /// Controller-selected digit for the Gauntlet code stage.
    controller_digit: u8,
    /// The chiptune bed for the current room, rendered once per room.
    tune: Arc<Vec<f32>>,
    /// The journey overlay ('j' toggles): level, rank, trophies, resonances.
    show_journey: bool,
    /// Where the mouse last was, for clicking cells and choices.
    mouse: (f64, f64),
    /// The hands in the current room: normalized poke points (R clears).
    pokes: Vec<(f64, f64)>,
    /// The same hands as replayable gesture events (down/move/up/cancel,
    /// phase-stamped), so held rooms can read pulls and releases.
    inputs: Vec<numinous_core::RoomInput>,
    /// A press began on a listening room: drags keep poking.
    poking: bool,
    /// Per-visit variation seed for rooms that support replayable novelty.
    variation: u64,
    /// Bounds file-producing shortcuts independently of event-loop key repeat.
    save_gate: save_gate::SaveGate,
    /// The radio: Some(index into STATIONS) when a cached station plays.
    radio: Option<usize>,
    /// The loaded station track, if any.
    radio_track: Arc<Vec<f32>>,
    /// Native sample rate of `radio_track`; the audio loop converts it live.
    radio_track_rate: u32,
    /// Frames left on the arrival card (the room explaining itself).
    room_card: u64,
    /// The tuned station's playlist on disk, in rotation order.
    radio_paths: Vec<std::path::PathBuf>,
    /// Which playlist entry is on the air.
    radio_index: usize,
    /// When the current track ends and the next takes the air.
    radio_until: Option<std::time::Instant>,
    /// Where the journey persists (the CLI's file; a scratch file in tests).
    journey_file: std::path::PathBuf,
    /// Where scores persist (the shared table; a scratch file in tests).
    scores_file: std::path::PathBuf,
    /// Where versioned App display and audio preferences persist.
    preferences_file: std::path::PathBuf,
    /// Power-user console (` / ~): room load, phase, variation, and friends.
    console: console::Console,
}

impl App {
    fn new() -> Self {
        let state_paths = local_state_paths();
        let journey_file = state_paths.journey.clone();
        let scores_file = state_paths.scores.clone();
        let preferences_file = state_paths.preferences.clone();
        let crash_log = state_paths.crash_log.clone();
        let journey = numinous_core::load_journey_file(&journey_file);
        #[cfg(test)]
        let (preferences, preferences_load_error) =
            (numinous_core::AppPreferences::default(), None::<String>);
        #[cfg(not(test))]
        let (preferences, preferences_load_error) =
            match numinous_core::read_app_preferences_file(&preferences_file) {
                Ok(preferences) => (preferences, None),
                Err(error) => (
                    numinous_core::AppPreferences::default(),
                    Some(error.to_string()),
                ),
            };
        if let Some(error) = preferences_load_error.as_deref() {
            let _ = append_crash_log_at(
                &crash_log,
                &format!("App preference load failed: {error}\n"),
            );
        }
        Self {
            window: None,
            presenter: None,
            presentation_warned: false,
            player: None,
            #[cfg(test)]
            transient_audio_clears: std::cell::Cell::new(0),
            #[cfg(test)]
            interaction_audio_events: std::cell::Cell::new(0),
            gamepad: gamepad::GamepadInput::new(),
            input_mode: input_legend::InputMode::default(),
            spectrum_cache: None,
            visualizer_source: numinous_audio::VisualizerSource::RoomBed,
            loopback: None,
            spectrum_prev: [0.0; numinous_core::BAND_COUNT],
            visualizer_scale: 1.0,
            mandelbrot_camera: numinous_core::rooms::mandelbrot::MandelbrotCamera::new(0),
            life_session: numinous_core::rooms::game_of_life::LifeSession::new(0),
            life_accumulator: 0.0,
            times_tables_aha: numinous_core::rooms::times_tables_aha::TimesTablesAha::new(),
            buffon_aha: numinous_core::rooms::buffon_aha::BuffonAha::new(),
            galton_aha: numinous_core::rooms::galton_aha::GaltonAha::new(),
            pendulum_aha: numinous_core::rooms::pendulum_aha::PendulumAha::new(0),
            kepler_aha: numinous_core::rooms::kepler_aha::KeplerAha::new(0.0),
            parrondo_aha: numinous_core::rooms::parrondo_aha::ParrondoAha::new(),
            nontransitive_aha: numinous_core::rooms::nontransitive_aha::NontransitiveAha::new(),
            room_wager: None,
            rooms: all_rooms_with(0),
            current: 0,
            t: 0.0,
            paused: false,
            dragging: false,
            show_info: false,
            the_show: false,
            show_crossfade_prev: None,
            show_crossfade_frames: 0,
            studio: false,
            studio_panel: studio_panel::StudioPanel::default(),
            share_naming: None,
            remembered_author: String::new(),
            gallery: None,
            session_viewer: SessionViewer::default(),
            session_audio: SessionAudio::default(),
            gpu: None,
            live_scale: live_render::LiveScale::new(),
            era: preferences.era,
            muted: preferences.muted,
            volume: f32::from(preferences.volume_percent) / 100.0,
            preferred_window_mode: preferences.window_mode,
            audio_program: AudioProgram::RoomScore,
            show_help: true,
            menu: menu::MenuState::launch(),
            start_fullscreen: false,
            quit_requested: false,
            journey_save_warned: false,
            score_save_warned: false,
            preferences_save_warned: false,
            crash_log,
            start_open: None,
            frame: 0,
            motion: numinous_core::Motion::from_env(),
            last_tick: Instant::now(),
            window_active: true,
            inactive_since: None,
            time_scale: 1.0,
            journey: journey.clone(),
            journey_saved: journey,
            level_seen: 1,
            banner: preferences_load_error
                .map(|_| feedback::Banner::status(PREFERENCES_LOAD_WARNING, 240)),
            screen_shake: 0,
            goal_announced: false,
            quiz: None,
            quiz_recent: Vec::new(),
            munch: None,
            munch_next_round: numinous_core::FULL_DECK_ROUND,
            munch_last_rule: None,
            nim: None,
            gauntlet: None,
            arcade: None,
            controller_digit: 0,
            tune: Arc::new(Vec::new()),
            show_journey: false,
            mouse: (0.0, 0.0),
            pokes: Vec::new(),
            inputs: Vec::new(),
            poking: false,
            variation: 0,
            save_gate: save_gate::SaveGate::default(),
            room_card: room_input::ROOM_CARD_FRAMES,
            radio: None,
            radio_track: Arc::new(Vec::new()),
            radio_track_rate: 44_100,
            radio_paths: Vec::new(),
            radio_index: 0,
            radio_until: None,
            journey_file,
            scores_file,
            preferences_file,
            console: console::Console::default(),
        }
    }

    /// Say once, on screen and in the crash log, that a local save is
    /// failing. This method persists nothing itself; it only reports.
    ///
    /// Progress files are the player's own history: a write that fails
    /// silently lets a whole session evaporate at exit with nothing ever
    /// said. Each store carries its own trouble spell, so one failing file
    /// cannot nag through the other's successes and one healthy file cannot
    /// silence the other's warning. Every failure is logged; the banner
    /// shows once per spell, and returns true when it was raised so the
    /// caller can keep a celebration from painting over it.
    fn report_save_trouble(
        &mut self,
        store: SaveStore,
        what: &str,
        error: &dyn std::fmt::Display,
    ) -> bool {
        let _ = append_crash_log_at(&self.crash_log, &format!("{what} failed: {error}\n"));
        let (warned, line) = match store {
            SaveStore::Journey => (&mut self.journey_save_warned, JOURNEY_SAVE_WARNING),
            SaveStore::Scores => (&mut self.score_save_warned, SCORE_SAVE_WARNING),
            SaveStore::Preferences => (&mut self.preferences_save_warned, PREFERENCES_SAVE_WARNING),
        };
        if *warned {
            return false;
        }
        *warned = true;
        self.banner = Some(feedback::Banner::status(line, 180));
        true
    }

    /// Whether a save-trouble warning is on screen right now. A game flow
    /// can post a failing score and then level the journey in the same
    /// tick, so the celebration check cannot rely only on what this one
    /// call raised.
    fn save_warning_showing(&self) -> bool {
        self.banner.as_ref().is_some_and(|banner| {
            banner.lines().first().is_some_and(|line| {
                line == JOURNEY_SAVE_WARNING
                    || line == SCORE_SAVE_WARNING
                    || line == PREFERENCES_SAVE_WARNING
            })
        })
    }

    /// Persist the journey and raise the Journey banner when the level moves.
    ///
    /// A save-trouble warning outranks the celebration, whether this call
    /// raised it or a failing score post did moments earlier: a level-up
    /// banner painted over the one warning of a trouble spell would restore
    /// the exact silence the warning exists to break.
    fn journey_changed(&mut self) {
        match numinous_core::persist_journey_delta(
            &self.journey_file,
            &self.journey_saved,
            &self.journey,
        ) {
            Ok(saved) => {
                self.journey = saved.clone();
                self.journey_saved = saved;
                self.journey_save_warned = false;
            }
            Err(error) => {
                self.report_save_trouble(SaveStore::Journey, "journey save", &error);
            }
        }
        let level = self.journey.level();
        if level > self.level_seen && !self.save_warning_showing() {
            self.banner = Some(feedback::level_up(level, self.journey.boons_available()));
        }
        self.level_seen = level;
    }

    /// Entering a room counts as a visit, exactly as it does in the CLI.
    fn visit_current(&mut self) {
        let id = self.rooms[self.current].meta().id;
        if !self.journey.visited.contains(id) {
            self.journey.visit(id);
            self.journey_changed();
        }
    }

    /// Start (or advance) the quiz: a fresh seeded round, phase-of-day seeded
    /// so everyone who opens the app today can compare notes.
    fn report_export_outcome(
        &mut self,
        success_label: &str,
        failure_line: &'static str,
        outcome: std::io::Result<std::path::PathBuf>,
    ) {
        match outcome {
            Ok(path) => {
                if let Some(window) = &self.window {
                    window.set_title(&format!("Numinous  |  {success_label}: {}", path.display()));
                }
            }
            Err(error) => {
                let _ = append_crash_log_at(
                    &self.crash_log,
                    &format!("{success_label} failed: {error}\n"),
                );
                self.banner = Some(feedback::Banner::status(
                    failure_line,
                    feedback::REFUSAL_FRAMES,
                ));
            }
        }
    }

    fn save_postcard_to(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        if self.current_room_is_life() {
            let room = self.rooms[self.current].as_ref();
            let size = postcard::POSTCARD_SIZE as usize;
            let mut raster = Raster::with_accent(size, size, room.meta().accent);
            self.life_session.render(&mut raster);
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, size, size);
            return postcard::write_rendered_postcard(
                room.meta().id,
                self.life_session.generation(),
                &rgba,
                dir,
            );
        }
        postcard::write_room_postcard(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            dir,
        )
    }

    /// Write a short looping APNG of the current visit: one phase cycle, or
    /// advancing Life generations for the persistent Game of Life session.
    fn save_short_loop(&mut self) {
        let outcome = self.save_short_loop_to(&postcard::default_postcard_dir());
        self.report_export_outcome(
            "loop saved",
            "LOOP SAVE FAILED  SEE .NUMINOUS-CRASH.LOG",
            outcome,
        );
    }

    fn save_short_loop_to(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        if self.current_room_is_life() {
            let room = self.rooms[self.current].as_ref();
            return postcard::write_life_loop(
                room.meta().id,
                room.meta().accent,
                &self.life_session,
                self.era,
                dir,
            );
        }
        postcard::write_room_loop(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            dir,
        )
    }

    /// Package postcard + loop + README into one share folder (CLI parity).
    fn save_share_bundle(&mut self) {
        let outcome = self.save_share_bundle_to(&postcard::default_postcard_dir());
        self.report_export_outcome(
            "share pack",
            "SHARE PACK FAILED  SEE .NUMINOUS-CRASH.LOG",
            outcome,
        );
    }

    /// Write the current room's postcard PNG: the P key.
    fn save_postcard(&mut self) {
        let outcome = self.save_postcard_to(&postcard::default_postcard_dir());
        self.report_export_outcome(
            "postcard saved",
            "POSTCARD FAILED  SEE .NUMINOUS-CRASH.LOG",
            outcome,
        );
    }

    fn save_share_bundle_to(&self, dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        if self.current_room_is_life() {
            let room = self.rooms[self.current].as_ref();
            return postcard::write_life_share_bundle(
                room.meta().id,
                room.meta().accent,
                &self.life_session,
                self.era,
                self.variation,
                dir,
            );
        }
        postcard::write_room_share_bundle(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            self.variation,
            dir,
        )
    }

    /// The Studio share trio on one key: `creation.num`, the link in the
    /// README, and the postcard, into one fresh share folder.
    ///
    /// Success and failure both speak through the shared export reporter;
    /// the writer discards its own partial folder on failure, so the failure
    /// line stays short rather than promising a cleanup state it cannot
    /// fully guarantee.
    /// Open the F4 naming step. The title prefills from the creation being
    /// shared (so an untouched re-share keeps its identity by default) and
    /// the author from the last signature, because naming happens in the
    /// instrument, not only in CLI flags.
    fn begin_share_naming(&mut self) {
        if self.share_naming.is_some() {
            return;
        }
        let identity = self.studio_panel.current_creation(self.t).ok();
        let title = identity
            .as_ref()
            .and_then(|creation| creation.title())
            .unwrap_or_default()
            .to_string();
        let author = identity
            .as_ref()
            .and_then(|creation| creation.author())
            .unwrap_or(&self.remembered_author)
            .to_string();
        self.share_naming = Some(ShareNaming {
            title,
            author,
            field: NamingField::Title,
        });
    }

    /// Append text to the active naming field, under the same printable
    /// ASCII bound the capsule format enforces, so a name the editor
    /// accepts is a name the share cannot refuse.
    fn naming_push_text(&mut self, text: &str) {
        let Some(naming) = self.share_naming.as_mut() else {
            return;
        };
        let field = naming.active_field_mut();
        let mut remaining =
            numinous_core::MAX_META_TEXT_CHARS.saturating_sub(field.chars().count());
        for c in text.chars() {
            if remaining > 0 && (' '..='~').contains(&c) {
                field.push(c);
                remaining -= 1;
            }
        }
    }

    fn naming_backspace(&mut self) {
        if let Some(naming) = self.share_naming.as_mut() {
            naming.active_field_mut().pop();
        }
    }

    fn naming_toggle_field(&mut self) {
        if let Some(naming) = self.share_naming.as_mut() {
            naming.field = match naming.field {
                NamingField::Title => NamingField::Author,
                NamingField::Author => NamingField::Title,
            };
        }
    }

    /// Cancel the naming step without sharing anything, and say so: a
    /// closed prompt with no banner would leave whether anything was
    /// written a mystery.
    fn cancel_share_naming(&mut self) {
        if self.share_naming.take().is_some() {
            self.banner = Some(feedback::Banner::status("SHARE CANCELLED", 90));
        }
    }

    /// Confirm the naming step: remember the signature and share the trio.
    fn confirm_share_naming(&mut self) {
        let Some(naming) = self.share_naming.take() else {
            return;
        };
        self.remembered_author = naming.author.trim().to_string();
        // An emptied field is the player clearing the name, which the share
        // must honor; it is not the same as never having been asked.
        self.share_studio_creation(Some(naming.identity()));
    }

    fn share_studio_creation(&mut self, identity: Option<ShareIdentity>) {
        match self.share_studio_creation_to(&postcard::default_postcard_dir(), identity) {
            Ok(Ok(dir)) => {
                self.report_export_outcome(
                    "studio share",
                    "SHARE FAILED  SEE .NUMINOUS-CRASH.LOG",
                    Ok(dir),
                );
                self.banner = Some(feedback::Banner::status("SHARED  .NUM + LINK + PNG", 90));
            }
            Ok(Err(studio_panel::ShareRefusal::UnparsedFormula)) => {
                // An unparsed edit has no curve to promise; the refusal names
                // the way forward instead of silently sharing the last-good.
                self.banner = Some(feedback::Banner::status(
                    "FIX THE FORMULA TO SHARE",
                    feedback::REFUSAL_FRAMES,
                ));
            }
            Ok(Err(studio_panel::ShareRefusal::LineageTooLarge)) => {
                // A different refusal deserves a different sentence: telling
                // the player to fix a formula that parses fine points them
                // at the wrong cause.
                self.banner = Some(feedback::Banner::status(
                    "FORK LINEAGE TOO LARGE TO SHARE",
                    feedback::REFUSAL_FRAMES,
                ));
            }
            Err(error) => {
                self.report_export_outcome(
                    "studio share",
                    "SHARE FAILED  SEE .NUMINOUS-CRASH.LOG",
                    Err(error),
                );
            }
        }
    }

    /// Testable body: the outer result is the write, the inner one the
    /// panel's refusal to produce a creation at all.
    fn share_studio_creation_to(
        &self,
        parent: &std::path::Path,
        identity: Option<ShareIdentity>,
    ) -> std::io::Result<Result<std::path::PathBuf, studio_panel::ShareRefusal>> {
        let mut creation = match self.studio_panel.current_creation(self.t) {
            Ok(creation) => creation,
            Err(refusal) => return Ok(Err(refusal)),
        };
        // The naming step's fields ride the capsule, clearings included: an
        // untouched reopen carries the opened capsule's own title and author,
        // so a share that ignored an emptied field would ship a name the
        // player had just deleted. The editor enforces the same printable
        // ASCII bound the format validates, so a name that reaches here
        // cannot be refused; if the two rules ever drift, the share fails
        // loudly through the io path rather than silently shipping wrong
        // identity.
        if let Some(ShareIdentity { title, author }) = identity {
            creation = match title {
                Some(title) => creation.with_title(&title).map_err(std::io::Error::other)?,
                None => creation.without_title(),
            };
            creation = match author {
                Some(author) => creation
                    .with_author(&author)
                    .map_err(std::io::Error::other)?,
                None => creation.without_author(),
            };
        }
        // Record the era only when it says something: Modern is the default
        // look, and omitting it keeps a plain share a version 1 capsule that
        // older builds still open.
        let creation = if self.era == numinous_core::Era::Modern {
            creation
        } else {
            creation.with_era(self.era)
        };
        let rgba = self.studio_panel.postcard_rgba(
            self.t,
            postcard::POSTCARD_SIZE as usize,
            self.era,
            creation.title(),
            creation.author(),
        );
        postcard::write_studio_share_bundle(&creation, &rgba, parent).map(Ok)
    }

    /// Move the Gallery cursor by whole tiles.
    fn gallery_move(&mut self, dx: i32, dy: i32) {
        if let Some(gallery) = &mut self.gallery {
            gallery.move_selection(dx, dy);
        }
    }

    /// Walk one step up the remix tree, or say exactly why the cursor
    /// stayed: no lineage and an absent parent are different answers, and
    /// a key that silently does nothing teaches players it is broken.
    fn gallery_select_parent(&mut self) {
        const NO_LINEAGE: &str = "THIS ONE DESCENDS FROM NOTHING";
        const PARENT_ABSENT: &str = "ITS PARENT IS NOT ON THIS WALL";
        let Some(gallery) = &mut self.gallery else {
            return;
        };
        match gallery.parent_status() {
            crate::gallery::ParentStatus::Local(_) => {
                let _ = gallery.select_parent();
                // A successful walk retires an earlier refusal, so a stale
                // DESCENDS FROM NOTHING cannot linger over a cursor that
                // just moved; unrelated banners are left alone.
                if self.banner.as_ref().is_some_and(|banner| {
                    banner
                        .lines()
                        .first()
                        .is_some_and(|line| line == NO_LINEAGE || line == PARENT_ABSENT)
                }) {
                    self.banner = None;
                }
            }
            crate::gallery::ParentStatus::NoLineage => {
                self.banner = Some(feedback::Banner::status(
                    NO_LINEAGE,
                    feedback::REFUSAL_FRAMES,
                ));
            }
            crate::gallery::ParentStatus::Absent => {
                self.banner = Some(feedback::Banner::status(
                    PARENT_ABSENT,
                    feedback::REFUSAL_FRAMES,
                ));
            }
        }
    }

    /// Fork the creation under the Gallery cursor: the wall closes and the
    /// Studio holds an editable, singing copy that remembers its parent, so
    /// the next share records the descent.
    ///
    /// No paused preview: the player browsed the wall and chose the fork
    /// gesture themselves, and fork must be as cheap as play.
    fn gallery_fork_selected(&mut self) {
        let Some(creation) = self
            .gallery
            .as_ref()
            .and_then(|gallery| gallery.selected_creation())
            .cloned()
        else {
            return;
        };
        self.gallery = None;
        self.quiz = None;
        if let Some(era) = creation.era() {
            self.era = era;
        }
        let spec = self.studio_panel.fork_creation(&creation);
        self.enter_studio_shell();
        self.set_studio_sound(spec);
        self.banner = Some(feedback::Banner::status("FORKED  IT IS YOURS NOW", 90));
    }

    /// Open the creation under the Gallery cursor: the wall closes and the
    /// Studio holds the exact reopened state, paused like any other open.
    fn gallery_open_selected(&mut self) {
        let Some(creation) = self
            .gallery
            .as_ref()
            .and_then(|gallery| gallery.selected_creation())
            .cloned()
        else {
            return;
        };
        self.gallery = None;
        self.open_studio_creation(&creation);
        self.banner = Some(feedback::Banner::status("REOPENED  ENTER: PLAY", 90));
    }

    /// Soft juice when the player bites a number that does not fit the rule.
    fn save_playtest_note(&self) -> std::io::Result<std::path::PathBuf> {
        self.save_playtest_note_to(&playtest::default_log_dir(), SystemTime::now())
    }

    fn save_playtest_note_to(
        &self,
        dir: &std::path::Path,
        now: SystemTime,
    ) -> std::io::Result<std::path::PathBuf> {
        let room = self.rooms.get(self.current).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "current room is missing")
        })?;
        let flagship_aha = self.flagship_aha_playtest_note();
        let snapshot = playtest::PlaytestSnapshot {
            room: room.as_ref(),
            journey: &self.journey,
            room_count: self.rooms.len(),
            phase: self.t,
            variation: self.variation,
            visual_era: self.era.name(),
            sound_on: !self.muted && self.player.is_some(),
            time_scale: self.time_scale,
            poke_points: &self.pokes,
            active_mode: self.playtest_mode(),
            flagship_aha,
        };
        let report = playtest::build_report(&snapshot, now);
        playtest::write_report(dir, now, &report)
    }

    /// Capture room-owned aha state for hallway F9 notes.
    fn flagship_aha_playtest_note(&self) -> Option<playtest::FlagshipAhaNote> {
        if self.the_show {
            return None;
        }
        if self.current_room_is_times_tables() {
            let aha = &self.times_tables_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or("none").to_string(),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_aha_plate(),
            });
        }
        if self.current_room_is_buffon() {
            let aha = &self.buffon_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or_else(|| "none".to_string()),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_circle_overlay(),
            });
        }
        if self.current_room_is_galton() {
            let aha = &self.galton_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or_else(|| "none".to_string()),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_outline_overlay(),
            });
        }
        if self.current_room_is_pendulum() {
            let aha = &self.pendulum_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or_else(|| "none".to_string()),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_curve_overlay(),
            });
        }
        if self.current_room_is_kepler() {
            let aha = &self.kepler_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or_else(|| "none".to_string()),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_time_overlay(),
            });
        }
        if self.current_room_is_parrondo() {
            let aha = &self.parrondo_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or_else(|| "none".to_string()),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_expectation_overlay(),
            });
        }
        if self.current_room_is_nontransitive() {
            let aha = &self.nontransitive_aha;
            return Some(playtest::FlagshipAhaNote {
                beat: aha.beat_label().to_string(),
                status: aha.status(None),
                earn: aha.earn_label().unwrap_or_else(|| "none".to_string()),
                allow_reveal: aha.allow_reveal_text(),
                can_summon: aha.can_summon(),
                aha_plate: aha.uses_outcome_grid(),
            });
        }
        None
    }

    fn playtest_mode(&self) -> &'static str {
        if self.session_viewer.is_open() {
            "watch agent"
        } else if self.studio {
            "studio"
        } else if self.arcade.is_some() {
            "munch arcade"
        } else if self.gauntlet.is_some() {
            "gauntlet"
        } else if self.nim.is_some() {
            "nim"
        } else if self.munch.is_some() {
            "munch"
        } else if self.quiz.is_some() {
            "quiz"
        } else if self.the_show {
            "the show"
        } else {
            "wander"
        }
    }

    fn enter_studio(&mut self) {
        self.enter_studio_shell();
        self.studio_reparse();
    }

    /// Enter Studio mode without touching the panel's formula or voice, so a
    /// reopened creation is not resung by the entry itself.
    fn enter_studio_shell(&mut self) {
        self.the_show = false;
        self.paused = false;
        self.close_menu();
        self.show_journey = false;
        self.studio = true;
        self.audio_program = AudioProgram::Studio;
        if let Some(player) = &self.player {
            player.clear_oneshot();
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// Reopen a saved creation in the Studio, exactly and paused.
    ///
    /// The panel pins the saved window and knob; the entry submits silence so
    /// whatever program was playing does not keep sounding under a preview
    /// that has deliberately not started singing yet.
    fn open_studio_creation(&mut self, creation: &numinous_core::StudioCreation) {
        // A quiz is stateless and would otherwise keep owning the keyboard
        // over the newly opened Studio; scored runs are guarded at the door
        // in open_dropped_file instead of being silently abandoned here.
        self.quiz = None;
        // The wall and the naming step were both about the creation that
        // was here a moment ago. A new one arriving ends them, or Enter
        // would share a stranger's capsule under the name still on screen,
        // and the REOPENED banner would promise a key the wall had taken.
        self.gallery = None;
        self.share_naming = None;
        // A capsule that recorded its Visual Era reopens in that era: the
        // look is part of what was saved.
        if let Some(era) = creation.era() {
            self.era = era;
        }
        self.studio_panel.open_creation(creation);
        self.enter_studio_shell();
        self.set_studio_sound(Some(numinous_core::SoundSpec {
            duration: 0.12,
            notes: Vec::new(),
        }));
    }

    /// Enter confirms a paused reopened preview: the creation starts singing.
    fn studio_confirm_opened(&mut self) {
        if let Some(spec) = self.studio_panel.confirm_opened() {
            self.set_studio_sound(Some(spec));
        }
    }

    /// Open a `.num` file from disk into the Studio, or say briefly why not.
    fn open_num_file(&mut self, path: &std::path::Path) {
        match numinous_core::StudioCreation::from_num_path(path) {
            Ok(creation) => {
                self.open_studio_creation(&creation);
                self.banner = Some(feedback::Banner::status("REOPENED  ENTER: PLAY", 90));
            }
            Err(error) => {
                let line = match error {
                    numinous_core::NumFileError::Io(_) => "COULD NOT READ THE .NUM FILE",
                    numinous_core::NumFileError::TooLarge => "THE .NUM FILE IS TOO LARGE",
                    numinous_core::NumFileError::Invalid(_) => "NOT A VALID .NUM CREATION",
                };
                self.banner = Some(feedback::Banner::status(line, feedback::REFUSAL_FRAMES));
            }
        }
    }

    /// A file dropped on the window: only a `.num` creation opens here.
    fn open_dropped_file(&mut self, path: &std::path::Path) {
        // A scored run in progress is not abandoned by a stray drop; the
        // player finishes or leaves it themselves, then drops again.
        if self.gauntlet.is_some()
            || self.munch.is_some()
            || self.nim.is_some()
            || self.arcade.is_some()
            || self.session_viewer.is_open()
        {
            self.banner = Some(feedback::Banner::status(
                "FINISH THE GAME FIRST",
                feedback::REFUSAL_FRAMES,
            ));
            return;
        }
        let is_num = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("num"));
        if !is_num {
            self.banner = Some(feedback::Banner::status(
                "ONLY .NUM CREATIONS OPEN HERE",
                feedback::REFUSAL_FRAMES,
            ));
            return;
        }
        self.open_num_file(path);
    }

    /// The launch-argument front door: a `.num` path or a `numinous://` link.
    fn open_start_input(&mut self, input: &str) {
        if input.starts_with("numinous://") {
            match numinous_core::StudioCreation::from_link(input) {
                Ok(creation) => {
                    self.open_studio_creation(&creation);
                    self.banner = Some(feedback::Banner::status("REOPENED  ENTER: PLAY", 90));
                }
                Err(_) => {
                    self.banner = Some(feedback::Banner::status(
                        "NOT A VALID NUMINOUS LINK",
                        feedback::REFUSAL_FRAMES,
                    ));
                }
            }
            return;
        }
        self.open_num_file(std::path::Path::new(input));
    }

    fn open_session_viewer(&mut self) {
        self.the_show = false;
        self.paused = false;
        self.close_menu();
        self.show_journey = false;
        self.banner = None;
        match self.session_viewer.open() {
            Ok(()) => {
                self.session_audio.begin();
                self.audio_program = AudioProgram::WatchAgent;
                self.publish_viewer_audio(None);
            }
            Err(_) => {
                self.banner = Some(feedback::session_viewer_unavailable());
            }
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    fn close_session_viewer(&mut self) {
        self.session_viewer.close();
        self.session_audio.end();
        // Restore the pre-viewer program: live radio if still selected, else room.
        if self.radio.is_some() && self.sync_radio_to_wall_clock() {
            // radio_play already owns the source.
        } else {
            self.update_audio();
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// Publish Watch Agent sound once per selected public sequence.
    fn sync_viewer_audio(&mut self) {
        if !self.session_viewer.is_open() {
            return;
        }
        self.audio_program = AudioProgram::WatchAgent;
        let selection = self.session_viewer.audio_selection();
        let sequence = selection.as_ref().map(|sel| sel.public_sequence());
        if !self.session_audio.select(sequence) {
            self.apply_master_gain();
            return;
        }
        self.publish_viewer_audio(selection.as_ref());
    }

    fn publish_viewer_audio(
        &mut self,
        selection: Option<&numinous_app::session_viewer::AudioSelection>,
    ) {
        self.audio_program = AudioProgram::WatchAgent;
        let Some(player) = &self.player else {
            return;
        };
        player.clear_parameter_voice();
        player.clear_oneshot();
        player.set_master_gain(if self.muted { 0.0 } else { self.volume });
        let stereo = match selection.and_then(|sel| sel.render(ROOM_BED_SOURCE_RATE)) {
            Some(mono) if !mono.is_empty() => mono
                .into_iter()
                .flat_map(|sample| [sample, sample])
                .collect::<Vec<_>>(),
            _ => vec![0.0, 0.0],
        };
        player.set_shared_stereo_at_rate(Arc::new(stereo), ROOM_BED_SOURCE_RATE);
    }

    fn toggle_show(&mut self) {
        self.the_show = !self.the_show;
        if self.the_show {
            self.close_menu();
            self.show_journey = false;
            // The Show is watching, not playing: it strips inputs, so a
            // call left posed would sit there with a live band and no
            // hand able to aim it.
            self.room_wager = None;
            self.clear_transient_audio();
        }
        self.paused = false;
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.sync_room_parameter_voice();
    }

    fn toggle_journey(&mut self) {
        if self.the_show {
            self.the_show = false;
            if let Some(window) = &self.window {
                window.set_title(&self.title());
            }
        }
        self.close_menu();
        self.show_journey = !self.show_journey;
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if self.paused {
            self.clear_pointer_state();
        }
    }

    fn modal_mode_active(&self) -> bool {
        self.session_viewer.is_open()
            || self.studio
            || self.quiz.is_some()
            || self.munch.is_some()
            || self.nim.is_some()
            || self.gauntlet.is_some()
            || self.arcade.is_some()
    }

    fn menu_layout(&self) -> menu::MenuLayout {
        let (width, height) = self.window.as_ref().map_or((900, 700), |window| {
            let size = window.inner_size();
            (size.width as usize, size.height as usize)
        });
        menu::MenuLayout::new(&self.menu, width, height)
    }

    fn activity_kind(&self) -> Option<menu::ActivityKind> {
        if self.session_viewer.is_open() {
            Some(menu::ActivityKind::SharedPlay)
        } else if self.studio {
            Some(menu::ActivityKind::Studio)
        } else if self.arcade.is_some() {
            Some(menu::ActivityKind::Arcade)
        } else if self.gauntlet.is_some() {
            Some(menu::ActivityKind::Gauntlet)
        } else if self.nim.is_some() {
            Some(menu::ActivityKind::Nim)
        } else if self.munch.is_some() {
            Some(menu::ActivityKind::Munch)
        } else if self.quiz.is_some() {
            Some(menu::ActivityKind::Quiz)
        } else {
            None
        }
    }

    fn open_home_menu(&mut self) {
        self.show_help = true;
        self.menu.open_home(menu::MenuOrigin::Room);
    }

    fn open_activity_menu(&mut self, kind: menu::ActivityKind) {
        self.show_help = true;
        self.menu.open_pause(kind);
        self.clear_pointer_state();
    }

    fn close_menu(&mut self) {
        self.show_help = false;
        self.menu.close();
    }

    fn menu_back(&mut self) {
        let intent = self.menu.back();
        self.apply_menu_intent(intent);
    }

    fn cycle_visual_era(&mut self) {
        self.era = self.era.next();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.persist_preferences();
    }

    fn cycle_window_mode(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let next = match window.fullscreen() {
            Some(winit::window::Fullscreen::Borderless(_)) => window
                .primary_monitor()
                .and_then(|monitor| monitor.video_modes().next())
                .map(winit::window::Fullscreen::Exclusive),
            Some(winit::window::Fullscreen::Exclusive(_)) => None,
            None => Some(winit::window::Fullscreen::Borderless(None)),
        };
        self.set_window_mode(next);
    }

    fn toggle_fullscreen(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let next = fullscreen_toggle_target(window.fullscreen().is_some());
        self.set_window_mode(next);
    }

    fn set_window_mode(&mut self, next: Option<winit::window::Fullscreen>) {
        let Some(window) = &self.window else {
            return;
        };
        let label = match &next {
            Some(winit::window::Fullscreen::Borderless(_)) => "BORDERLESS".to_string(),
            Some(winit::window::Fullscreen::Exclusive(mode)) => {
                let size = mode.size();
                format!("EXCLUSIVE {}x{}", size.width, size.height)
            }
            None => "WINDOWED".to_string(),
        };
        self.preferred_window_mode = match &next {
            Some(winit::window::Fullscreen::Borderless(_)) => {
                numinous_core::WindowModePreference::Borderless
            }
            Some(winit::window::Fullscreen::Exclusive(_)) => {
                numinous_core::WindowModePreference::Exclusive
            }
            None => numinous_core::WindowModePreference::Windowed,
        };
        window.set_fullscreen(next);
        self.banner = Some(feedback::fullscreen(&label));
        self.persist_preferences();
    }

    fn leave_activity(&mut self, kind: menu::ActivityKind) {
        match kind {
            menu::ActivityKind::Quiz => self.quiz = None,
            menu::ActivityKind::Munch => self.munch = None,
            menu::ActivityKind::Nim => self.nim = None,
            menu::ActivityKind::Gauntlet => self.gauntlet = None,
            menu::ActivityKind::Arcade => {
                if let Some(play) = self.arcade.take() {
                    self.post_score(&format!("arcade seed:{}", play.seed), play.run.score);
                }
            }
            menu::ActivityKind::Studio => self.exit_studio(),
            menu::ActivityKind::SharedPlay => self.close_session_viewer(),
        }
        self.update_audio();
    }

    fn restart_activity(&mut self, kind: menu::ActivityKind) {
        match kind {
            menu::ActivityKind::Quiz => self.quiz_next(),
            menu::ActivityKind::Munch => self.munch_start(),
            menu::ActivityKind::Nim => self.nim_start(),
            menu::ActivityKind::Gauntlet => self.gauntlet_start(),
            menu::ActivityKind::Arcade => self.arcade_start(),
            menu::ActivityKind::Studio | menu::ActivityKind::SharedPlay => {}
        }
    }

    fn apply_menu_intent(&mut self, intent: menu::MenuIntent) {
        match intent {
            menu::MenuIntent::None => {}
            menu::MenuIntent::Close | menu::MenuIntent::ResumeActivity => self.close_menu(),
            menu::MenuIntent::Choose(choice) => self.activate_menu_choice(choice),
            menu::MenuIntent::VolumeDelta(percent) => {
                self.change_volume(f32::from(percent) / 100.0);
            }
            menu::MenuIntent::ToggleMute => {
                self.toggle_mute();
                self.banner = Some(feedback::volume(self.volume, self.muted));
            }
            menu::MenuIntent::CycleEra => self.cycle_visual_era(),
            menu::MenuIntent::CycleWindowMode => self.cycle_window_mode(),
            menu::MenuIntent::SkipRadioTrack => self.skip_radio_track(),
            menu::MenuIntent::ToggleFullscreen => self.toggle_fullscreen(),
            menu::MenuIntent::Quit => self.quit_requested = true,
            menu::MenuIntent::RestartActivity(kind) => {
                self.close_menu();
                self.restart_activity(kind);
            }
            menu::MenuIntent::LeaveActivity(kind) => {
                self.close_menu();
                self.leave_activity(kind);
            }
        }
    }

    fn activate_selected_menu_action(&mut self) {
        self.menu.clear_pointer();
        let intent = self.menu.activate_focused();
        self.apply_menu_intent(intent);
    }

    fn handle_menu_key(&mut self, key: &Key, repeat: bool) -> bool {
        if !self.show_help || !self.menu.is_open() {
            return false;
        }
        if repeat {
            return true;
        }
        if let Key::Character(text) = key
            && console::is_toggle_key(text.as_str())
            && !matches!(self.menu.origin(), menu::MenuOrigin::Activity(_))
        {
            self.close_menu();
            self.console.open();
            return true;
        }
        self.menu.clear_pointer();
        let layout = self.menu_layout();
        let intent = match key {
            Key::Named(NamedKey::Escape) => self.menu.back(),
            Key::Named(NamedKey::ArrowUp) => {
                if layout.is_compact() {
                    self.menu.focus_next(-1);
                } else {
                    self.menu.move_spatial(&layout, menu::Direction::Up);
                }
                menu::MenuIntent::None
            }
            Key::Named(NamedKey::ArrowDown) => {
                if layout.is_compact() {
                    self.menu.focus_next(1);
                } else {
                    self.menu.move_spatial(&layout, menu::Direction::Down);
                }
                menu::MenuIntent::None
            }
            Key::Named(NamedKey::ArrowLeft) => {
                if let Some(intent) = self.menu.adjust_focused(-10) {
                    intent
                } else {
                    self.menu.move_spatial(&layout, menu::Direction::Left);
                    menu::MenuIntent::None
                }
            }
            Key::Named(NamedKey::ArrowRight) => {
                if let Some(intent) = self.menu.adjust_focused(10) {
                    intent
                } else {
                    self.menu.move_spatial(&layout, menu::Direction::Right);
                    menu::MenuIntent::None
                }
            }
            Key::Named(NamedKey::Enter) => self.menu.activate_focused(),
            Key::Character(text) if text.as_str().eq_ignore_ascii_case("f") => {
                menu::MenuIntent::ToggleFullscreen
            }
            Key::Character(text) if text.chars().count() == 1 => self
                .menu
                .activate_shortcut(text.chars().next().expect("one character"))
                .unwrap_or(menu::MenuIntent::None),
            _ => menu::MenuIntent::None,
        };
        self.apply_menu_intent(intent);
        true
    }

    fn handle_quit_key(&mut self, key: &Key, repeat: bool) -> bool {
        if repeat || self.studio || self.console.is_open() {
            return false;
        }
        if matches!(
            key,
            Key::Character(text) if text.as_str().eq_ignore_ascii_case("q")
        ) {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            self.quit_requested = true;
            return true;
        }
        false
    }

    fn show_mode_active(&self) -> bool {
        self.the_show && !self.modal_mode_active()
    }

    fn left_press_context(&self) -> mouse_input::LeftPressContext {
        mouse_input::LeftPressContext {
            game_click_mode: self.munch.is_some()
                || self.quiz.is_some()
                || self.nim.is_some()
                || self.arcade.is_some()
                || self.gauntlet.is_some(),
            studio: self.studio,
            show_help: self.show_help,
            show_journey: self.show_journey,
            room_has_verb: self.rooms[self.current].verb().is_some(),
        }
    }

    fn pointer_state(&self) -> mouse_input::PointerState {
        mouse_input::PointerState {
            dragging: self.dragging,
            poking: self.poking,
        }
    }

    fn set_pointer_state(&mut self, state: mouse_input::PointerState) {
        // A poke that ends without a recorded lift (focus loss, a modal
        // opening) closes its gesture gently; releases record their lift
        // first, which makes this cancel a no-op.
        if self.poking && !state.poking {
            room_input::cancel_open_gesture(&mut self.inputs, self.t);
        }
        self.dragging = state.dragging;
        self.poking = state.poking;
    }

    fn clear_pointer_state(&mut self) {
        self.set_pointer_state(mouse_input::PointerState::default());
    }

    fn refresh_pointer_state(&mut self) {
        let state =
            mouse_input::retain_pointer_state(self.pointer_state(), self.left_press_context());
        self.set_pointer_state(state);
    }

    fn handle_playtest_shortcut(&mut self, key: &Key, repeated: bool) -> bool {
        if !matches!(key, Key::Named(NamedKey::F9)) {
            return false;
        }
        if !self
            .save_gate
            .admit(save_gate::SaveKind::PlaytestNote, Instant::now(), repeated)
        {
            return true;
        }
        let result = self.save_playtest_note();
        self.set_playtest_note_banner(result);
        true
    }

    #[cfg(test)]
    fn handle_playtest_shortcut_to(
        &mut self,
        key: &Key,
        dir: &std::path::Path,
        file_time: SystemTime,
        input_time: Instant,
        repeated: bool,
    ) -> bool {
        if !matches!(key, Key::Named(NamedKey::F9)) {
            return false;
        }
        if !self
            .save_gate
            .admit(save_gate::SaveKind::PlaytestNote, input_time, repeated)
        {
            return true;
        }
        let result = self.save_playtest_note_to(dir, file_time);
        self.set_playtest_note_banner(result);
        true
    }

    fn set_playtest_note_banner(&mut self, result: std::io::Result<std::path::PathBuf>) {
        self.banner = Some(feedback::playtest_note(result));
    }

    fn preferences(&self) -> numinous_core::AppPreferences {
        numinous_core::AppPreferences {
            volume_percent: (self.volume * 100.0).round().clamp(0.0, 100.0) as u8,
            muted: self.muted,
            era: self.era,
            window_mode: self.preferred_window_mode,
        }
    }

    fn persist_preferences(&mut self) {
        match numinous_core::persist_app_preferences_file(
            &self.preferences_file,
            self.preferences(),
        ) {
            Ok(()) => self.preferences_save_warned = false,
            Err(error) => {
                self.report_save_trouble(SaveStore::Preferences, "App preference save", &error);
            }
        }
    }

    fn change_volume(&mut self, step: f32) {
        self.volume = (self.volume + step).clamp(0.0, 1.0);
        self.banner = Some(feedback::volume(self.volume, self.muted));
        self.apply_master_gain();
        self.persist_preferences();
    }

    fn apply_master_gain(&self) {
        if let Some(player) = &self.player {
            player.set_master_gain(if self.muted { 0.0 } else { self.volume });
        }
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.apply_master_gain();
        self.persist_preferences();
    }

    /// Tune in to the current dial position: build the playlist, join the
    /// broadcast mid-stream (the station was always on the air), and play.
    fn tune_in(&mut self) {
        self.clear_pointer_state();
        self.radio_track = Arc::new(Vec::new());
        self.radio_track_rate = 44_100;
        self.radio_paths.clear();
        self.radio_until = None;
        let Some(i) = self.radio else {
            self.update_audio();
            if let Some(window) = &self.window {
                window.set_title(&self.title());
            }
            self.banner = Some(feedback::radio_off());
            return;
        };
        let st = &numinous_core::STATIONS[i];
        let dir = radio_cache::default_dir();
        self.radio_paths = radio_cache::station_tracks(&dir, st.id);
        // Join the broadcast live: the wall clock decides which track is on.
        let _ = self.sync_radio_to_wall_clock();
        // The dial speaks on screen, especially when the station is silent.
        let st = &numinous_core::STATIONS[i];
        self.banner = Some(feedback::radio(st.name, st.id, self.radio_paths.len()));
        self.update_audio();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    fn sync_radio_at(&mut self, now_secs: f64) -> bool {
        // Watch Agent owns the source for the whole paired session.
        if self.studio || self.session_viewer.is_open() {
            return false;
        }
        if self.radio.is_none() {
            self.radio_track = Arc::new(Vec::new());
            self.radio_until = None;
            self.update_audio();
            return false;
        }
        let Some((index, position)) = radio_cache::live_position(&self.radio_paths, now_secs)
        else {
            self.radio_track = Arc::new(Vec::new());
            self.radio_until = None;
            self.update_audio();
            return false;
        };
        self.radio_index = index;
        let playing = self.radio_play_or_advance(position);
        if !playing {
            self.update_audio();
        }
        playing
    }

    fn sync_radio_to_wall_clock(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .unwrap_or(0.0);
        self.sync_radio_at(now)
    }

    fn radio_play_or_advance(&mut self, offset: f64) -> bool {
        let track_count = self.radio_paths.len();
        if track_count == 0 {
            self.radio_track = Arc::new(Vec::new());
            self.radio_track_rate = 44_100;
            self.radio_until = None;
            return false;
        }
        self.radio_index %= track_count;
        let mut next_offset = offset;
        for _ in 0..track_count {
            if self.radio_play(next_offset) {
                return true;
            }
            self.radio_index = (self.radio_index + 1) % track_count;
            next_offset = 0.0;
        }
        self.radio_track = Arc::new(Vec::new());
        self.radio_track_rate = 44_100;
        self.radio_until = None;
        false
    }

    /// Put the current playlist entry on the air, starting `offset` seconds
    /// in: read it (mono or stereo), retain one source-rate stereo buffer, and
    /// hand it to the player for live rate conversion.
    fn radio_play(&mut self, offset: f64) -> bool {
        self.radio_track = Arc::new(Vec::new());
        self.radio_track_rate = 44_100;
        self.radio_until = None;
        let Some(path) = self.radio_paths.get(self.radio_index) else {
            return false;
        };
        let device_rate = self.player.as_ref().map_or(44_100, |p| p.sample_rate());
        let Some(loaded) = radio_cache::load_track(path, offset, device_rate) else {
            return false;
        };
        self.radio_track = loaded.stereo;
        self.radio_track_rate = loaded.sample_rate;
        self.radio_until = Some(std::time::Instant::now() + loaded.remaining);
        self.audio_program = AudioProgram::Radio;
        if let Some(player) = &self.player {
            player.clear_parameter_voice();
            player.clear_oneshot();
            player.set_shared_stereo_at_rate(self.radio_track.clone(), self.radio_track_rate);
            player.set_master_gain(if self.muted { 0.0 } else { self.volume });
        }
        true
    }

    /// GPU-render the current room if it has a real-time GPU path (the deep
    /// fractal zooms), returning the RGBA frame; `None` means draw on the CPU.
    fn gpu_frame(&mut self, width: usize, height: usize) -> Option<Vec<u8>> {
        if !numinous_gpu::frame_size_supported(width, height) {
            return None;
        }
        let id = self.rooms[self.current].meta().id;
        let (w, h) = (width as u32, height as u32);
        let mandelbrot_view = (id == "mandelbrot")
            .then(|| live_mandelbrot_gpu_view(self.mandelbrot_camera, w, h))
            .flatten();
        let julia_c = (id == "julia").then(|| julia_gpu_c(self.t, self.variation, &self.pokes));
        let gpu = self.gpu.as_mut()?;
        let frame = match id {
            "mandelbrot" => {
                let (center_x, center_y, scale) = mandelbrot_view?;
                gpu.render(
                    w,
                    h,
                    center_x,
                    center_y,
                    scale,
                    numinous_core::rooms::FRACTAL_MAX_ITER,
                    numinous_gpu::Fractal::Mandelbrot,
                )
            }
            "julia" => {
                let (cx, cy) = julia_c?;
                let c = numinous_gpu::Fractal::Julia { cx, cy };
                gpu.render(
                    w,
                    h,
                    0.0,
                    0.0,
                    julia_gpu_vertical_span(w, h),
                    numinous_core::rooms::FRACTAL_MAX_ITER,
                    c,
                )
            }
            _ => return None,
        };
        match frame {
            Ok(rgba) => Some(rgba),
            Err(_) => {
                self.gpu = None;
                None
            }
        }
    }

    fn studio_reparse(&mut self) {
        let spec = self.studio_panel.reparse();
        self.set_studio_edit_sound(spec);
    }

    fn set_studio_edit_sound(&mut self, parsed: Option<numinous_core::SoundSpec>) {
        let spec = parsed.or_else(|| self.studio_panel.current_sound());
        self.set_studio_sound(spec);
    }

    fn set_studio_sound(&mut self, spec: Option<numinous_core::SoundSpec>) {
        self.set_studio_sound_with_crossfade(spec, None);
    }

    fn suspend_presentation_clock(&mut self, now: Instant) {
        self.window_active = false;
        self.inactive_since.get_or_insert(now);
    }

    fn advance_presentation_time(&mut self, seconds: f64) {
        if self.studio {
            self.studio_panel.advance_morph(seconds);
        }
    }

    fn resume_presentation_clock(&mut self, now: Instant) {
        if let Some(inactive_since) = self.inactive_since.take() {
            self.advance_presentation_time(
                now.saturating_duration_since(inactive_since).as_secs_f64(),
            );
        }
        self.window_active = true;
        self.last_tick = now;
    }

    fn set_studio_recipe_sound(&mut self, spec: Option<numinous_core::SoundSpec>) {
        self.set_studio_sound_with_crossfade(spec, Some(studio_panel::RECIPE_MORPH_SECONDS as f32));
    }

    fn set_studio_sound_with_crossfade(
        &mut self,
        spec: Option<numinous_core::SoundSpec>,
        crossfade_seconds: Option<f32>,
    ) {
        self.audio_program = AudioProgram::Studio;
        let Some(player) = &self.player else {
            return;
        };
        player.clear_parameter_voice();
        player.clear_oneshot();
        player.set_master_gain(if self.muted { 0.0 } else { self.volume });
        if let Some(spec) = spec {
            let samples = spec.render(player.sample_rate());
            if let Some(seconds) = crossfade_seconds {
                let _ = player.set_samples_with_crossfade(samples, seconds);
            } else {
                player.set_samples(samples);
            }
        }
    }

    fn exit_studio(&mut self) {
        self.studio = false;
        // Any route out of the Studio also leaves the Gallery and any open
        // naming step, so a menu or controller exit cannot strand either
        // one: naming state that survives invisibly comes back holding the
        // keyboard over a creation it was never about.
        self.gallery = None;
        self.share_naming = None;
        if self.radio.is_none() || !self.sync_radio_to_wall_clock() {
            self.update_audio();
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// Render the current room's stable score and crossfade to it.
    fn update_audio(&mut self) {
        if self.session_viewer.is_open() {
            self.sync_viewer_audio();
            return;
        }
        if self.studio {
            self.audio_program = AudioProgram::Studio;
            if let Some(player) = &self.player {
                player.clear_parameter_voice();
                player.clear_oneshot();
            }
            self.apply_master_gain();
            return;
        }
        if self.radio.is_some() && !self.radio_track.is_empty() {
            self.audio_program = AudioProgram::Radio;
            if let Some(player) = &self.player {
                player.clear_parameter_voice();
                player.clear_oneshot();
            }
            self.apply_master_gain();
            return;
        }
        let switching_to_room_score = self.audio_program != AudioProgram::RoomScore;
        if switching_to_room_score {
            self.clear_pointer_state();
        }
        self.audio_program = AudioProgram::RoomScore;
        let Some(player) = &self.player else {
            return;
        };
        player.set_master_gain(if self.muted { 0.0 } else { self.volume });
        let rendered_room_score = self.tune.is_empty();
        if rendered_room_score {
            self.tune = Arc::new(match self.rooms[self.current].motif() {
                Some(motif) => motif.arrangement().render_stereo(ROOM_BED_SOURCE_RATE),
                None => numinous_core::compose(self.current as u64 + 1, 8)
                    .render(ROOM_BED_SOURCE_RATE)
                    .into_iter()
                    .flat_map(|sample| [sample, sample])
                    .collect(),
            });
        }
        if rendered_room_score || switching_to_room_score {
            player.set_shared_stereo_at_rate(self.tune.clone(), ROOM_BED_SOURCE_RATE);
        }
        self.sync_room_parameter_voice();
    }

    fn desired_room_parameter_sound(&self) -> Option<numinous_core::ParametricSound> {
        selected_parameter_sound(
            self.audio_program,
            self.modal_mode_active(),
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.the_show,
        )
    }

    fn sync_room_parameter_voice(&self) {
        if !room_transient_audio_owned(self.audio_program, self.modal_mode_active()) {
            self.clear_transient_audio();
        }
        let Some(player) = &self.player else {
            return;
        };
        let voice = self.desired_room_parameter_sound();
        if let Some(voice) = voice {
            let _ = player.set_parameter_voice(voice.root_hz(), voice.ratio(), voice.gain());
        } else {
            player.clear_parameter_voice();
        }
    }

    fn title(&self) -> String {
        if self.session_viewer.is_open() {
            "Numinous  |  Watch Agent".to_string()
        } else if self.audio_program == AudioProgram::Radio
            && let Some(station) = self
                .radio
                .and_then(|index| numinous_core::STATIONS.get(index))
        {
            format!("Numinous  |  radio: {}", station.name)
        } else if self.the_show {
            format!(
                "Numinous  |  The Show  |  {}",
                self.rooms[self.current].meta().title
            )
        } else {
            let era = if self.era == numinous_core::Era::Modern {
                String::new()
            } else {
                format!("  |  {}", self.era.name())
            };
            format!(
                "Numinous  |  {}{era}  (esc: menu)",
                self.rooms[self.current].meta().title
            )
        }
    }

    fn switch(&mut self, delta: isize) {
        if self.the_show && self.show_crossfade_prev.is_some() {
            self.show_crossfade_frames = SHOW_CROSSFADE_FRAMES;
        }
        self.current = room_input::wrapped_room_index(self.current, delta, self.rooms.len());
        self.rooms = room_input::redeal_rooms(&mut self.variation, &mut self.current);
        self.reset_room_runtime();
        self.tune = Arc::new(Vec::new());
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.visit_current();
        self.update_audio();
    }

    /// Jump to a catalog index without bumping variation (console / power users).
    fn goto_room_index(&mut self, index: usize) {
        if index >= self.rooms.len() {
            return;
        }
        if self.the_show && self.show_crossfade_prev.is_some() {
            self.show_crossfade_frames = SHOW_CROSSFADE_FRAMES;
        }
        self.current = index;
        self.reset_room_runtime();
        self.tune = Arc::new(Vec::new());
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.visit_current();
        self.update_audio();
    }

    /// Rebind rooms at a variation seed while keeping the same room id when possible.
    fn set_variation_seed(&mut self, variation: u64) {
        let id = self
            .rooms
            .get(self.current)
            .map(|r| r.meta().id)
            .unwrap_or("");
        self.variation = variation;
        self.rooms = all_rooms_with(variation);
        if let Some(i) = self.rooms.iter().position(|r| r.meta().id == id) {
            self.current = i;
        } else if !self.rooms.is_empty() {
            self.current = self.current.min(self.rooms.len() - 1);
        }
        self.reset_room_runtime();
        self.tune = Arc::new(Vec::new());
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.visit_current();
        self.update_audio();
    }

    /// Apply one parsed console command; returns log lines to print.
    fn run_console_command(&mut self, command: console::Command) -> Vec<String> {
        use console::Command;
        match command {
            Command::Empty => Vec::new(),
            Command::Close => {
                self.console.close();
                Vec::new()
            }
            Command::Clear => {
                self.console.clear_log();
                Vec::new()
            }
            Command::Help => console::help_lines(),
            Command::Where => {
                let room = &self.rooms[self.current];
                let meta = room.meta();
                vec![format!(
                    "[{}] {}  t={:.3}  vary={}  speed={:.2}",
                    meta.id,
                    meta.title,
                    self.t.clamp(0.0, 1.0),
                    self.variation,
                    self.time_scale
                )]
            }
            Command::Goto(target) => match console::resolve_room(&target, &self.rooms) {
                Ok(index) => {
                    self.goto_room_index(index);
                    let meta = self.rooms[index].meta();
                    vec![format!("loaded [{}] {}", meta.id, meta.title)]
                }
                Err(err) => vec![err],
            },
            Command::List { query } => console::list_rooms(&self.rooms, query.as_deref(), 12),
            Command::Reset => {
                self.reset_current_room();
                vec!["reset visit".into()]
            }
            Command::Era(name) => match console::parse_era(&name) {
                Some(era) => {
                    self.era = era;
                    if let Some(window) = &self.window {
                        window.set_title(&self.title());
                    }
                    self.persist_preferences();
                    vec![format!("era {}", era.name())]
                }
                None => vec![format!("unknown era '{name}'")],
            },
            Command::Mute => {
                self.muted = true;
                self.apply_master_gain();
                self.banner = Some(feedback::volume(self.volume, self.muted));
                self.persist_preferences();
                vec!["muted".into()]
            }
            Command::Unmute => {
                self.muted = false;
                self.apply_master_gain();
                self.banner = Some(feedback::volume(self.volume, self.muted));
                self.persist_preferences();
                vec!["unmuted".into()]
            }
            Command::Volume(v) => {
                self.volume = v.clamp(0.0, 1.0);
                self.apply_master_gain();
                self.banner = Some(feedback::volume(self.volume, self.muted));
                self.persist_preferences();
                vec![format!("volume {:.0}%", self.volume * 100.0)]
            }
            Command::Speed(s) => {
                self.time_scale = s.clamp(0.25, 8.0);
                vec![format!("speed {:.2}", self.time_scale)]
            }
            Command::Phase(t) => {
                self.t = t.clamp(0.0, 1.0);
                vec![format!("t={:.3}", self.t)]
            }
            Command::Vary(v) => {
                self.set_variation_seed(v);
                vec![format!("variation {v}")]
            }
            Command::Studio => {
                self.console.close();
                self.enter_studio();
                vec!["studio".into()]
            }
            Command::Show => {
                self.console.close();
                self.toggle_show();
                vec!["the show".into()]
            }
            Command::Unknown(msg) => vec![msg],
        }
    }

    /// Keyboard path while the console is open. Returns true when handled.
    fn handle_console_key(&mut self, key: &Key) -> bool {
        if !self.console.is_open() {
            return false;
        }
        match key {
            Key::Named(NamedKey::Escape) => {
                self.console.close();
                true
            }
            Key::Named(NamedKey::Enter) => {
                let line = self.console.take_line();
                if !line.is_empty() {
                    self.console.push_log(format!("> {line}"));
                }
                let command = console::parse_line(&line);
                for out in self.run_console_command(command) {
                    self.console.push_log(out);
                }
                true
            }
            Key::Named(NamedKey::Backspace) => {
                self.console.backspace();
                true
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.console.history_older();
                true
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.console.history_newer();
                true
            }
            Key::Named(NamedKey::Space) => {
                self.console.push_char(' ');
                true
            }
            Key::Character(text) => {
                if console::is_toggle_key(text.as_str()) {
                    self.console.close();
                    return true;
                }
                self.console.push_text(text.as_str());
                true
            }
            _ => true, // swallow other keys while open
        }
    }

    fn reset_room_runtime(&mut self) {
        self.clear_pointer_state();
        if self.goal_announced {
            self.banner = None;
        }
        room_input::reset_room_view(
            &mut self.t,
            &mut self.room_card,
            &mut self.pokes,
            &mut self.inputs,
        );
        self.mandelbrot_camera.reset(self.variation);
        self.reset_life_session();
        self.reset_times_tables_aha();
        self.reset_buffon_aha();
        self.reset_galton_aha();
        self.reset_pendulum_aha();
        self.reset_kepler_aha();
        self.reset_parrondo_aha();
        self.reset_nontransitive_aha();
        // A call is about one room's readout; carrying it across the
        // doorway would grade the wrong number.
        self.room_wager = None;
        self.goal_announced = false;
    }

    fn reset_current_room(&mut self) {
        self.reset_room_runtime();
        self.spectrum_cache = None;
        self.update_audio();
    }

    /// Normalized room-bed spectrum for the visualizer meter (cached per room).
    fn room_spectrum_bands(&mut self) -> Option<[f32; numinous_core::BAND_COUNT]> {
        if let Some((idx, bands)) = self.spectrum_cache
            && idx == self.current
        {
            return Some(bands);
        }
        let motif = self.rooms.get(self.current)?.motif()?;
        let samples = motif
            .arrangement()
            .render_stereo(numinous_core::ROOM_BED_SOURCE_RATE);
        let bands =
            numinous_core::arrangement_spectrum(&samples, numinous_core::ROOM_BED_SOURCE_RATE);
        self.spectrum_cache = Some((self.current, bands));
        Some(bands)
    }

    /// Live visualizer bands from the preferred source, with graceful fallback.
    fn visualizer_bands(
        &mut self,
    ) -> Option<(
        [f32; numinous_core::BAND_COUNT],
        numinous_audio::VisualizerSource,
    )> {
        match self.visualizer_source {
            numinous_audio::VisualizerSource::Loopback => {
                if let Some(capture) = self.loopback.as_ref() {
                    let samples = capture.snapshot_frames(2_048);
                    if samples.len() >= 64 {
                        let bands =
                            numinous_core::arrangement_spectrum(&samples, capture.sample_rate());
                        return Some((bands, numinous_audio::VisualizerSource::Loopback));
                    }
                }
                // Fall through to output mix, then room bed.
                if let Some(bands) = self.output_mix_bands() {
                    return Some((bands, numinous_audio::VisualizerSource::OutputMix));
                }
                self.room_spectrum_bands()
                    .map(|b| (b, numinous_audio::VisualizerSource::RoomBed))
            }
            numinous_audio::VisualizerSource::OutputMix => {
                if let Some(bands) = self.output_mix_bands() {
                    return Some((bands, numinous_audio::VisualizerSource::OutputMix));
                }
                self.room_spectrum_bands()
                    .map(|b| (b, numinous_audio::VisualizerSource::RoomBed))
            }
            numinous_audio::VisualizerSource::RoomBed
            | numinous_audio::VisualizerSource::Silent => self
                .room_spectrum_bands()
                .map(|b| (b, numinous_audio::VisualizerSource::RoomBed)),
        }
    }

    fn output_mix_bands(&self) -> Option<[f32; numinous_core::BAND_COUNT]> {
        let player = self.player.as_ref()?;
        let samples = player.snapshot_output_tap(2_048);
        if samples.len() < 64 {
            return None;
        }
        Some(numinous_core::arrangement_spectrum(
            &samples,
            player.sample_rate(),
        ))
    }

    /// Cycle visualizer source: room bed, output mix, loopback (when present).
    fn cycle_visualizer_source(&mut self) {
        self.visualizer_source = match self.visualizer_source {
            numinous_audio::VisualizerSource::RoomBed
            | numinous_audio::VisualizerSource::Silent => {
                numinous_audio::VisualizerSource::OutputMix
            }
            numinous_audio::VisualizerSource::OutputMix => {
                if self.loopback.is_none() {
                    self.loopback = numinous_audio::InputCapture::try_open_loopback().ok();
                }
                if self.loopback.is_some() {
                    numinous_audio::VisualizerSource::Loopback
                } else {
                    numinous_audio::VisualizerSource::RoomBed
                }
            }
            numinous_audio::VisualizerSource::Loopback => {
                self.loopback = None;
                numinous_audio::VisualizerSource::RoomBed
            }
        };
        let label = match self.visualizer_source {
            numinous_audio::VisualizerSource::Loopback => self
                .loopback
                .as_ref()
                .map(|c| format!("VIZ {}", c.device_name()))
                .unwrap_or_else(|| "VIZ LOOPBACK".into()),
            other => format!("VIZ {}", other.label()),
        };
        self.banner = Some(feedback::Banner::status(label, 90));
    }

    fn current_room_is_life(&self) -> bool {
        self.rooms[self.current].meta().id == "game-of-life"
    }

    fn current_room_is_times_tables(&self) -> bool {
        self.rooms[self.current].meta().id == "times-tables"
    }

    fn current_room_is_galton(&self) -> bool {
        self.rooms[self.current].meta().id == "galton-board"
    }

    fn current_room_is_buffon(&self) -> bool {
        self.rooms[self.current].meta().id == "buffon-needle"
    }

    fn current_room_is_pendulum(&self) -> bool {
        self.rooms[self.current].meta().id == "double-pendulum"
    }

    fn current_room_is_kepler(&self) -> bool {
        self.rooms[self.current].meta().id == "kepler-laws"
    }

    fn current_room_is_parrondo(&self) -> bool {
        self.rooms[self.current].meta().id == "parrondo"
    }

    fn current_room_is_nontransitive(&self) -> bool {
        self.rooms[self.current].meta().id == "nontransitive"
    }

    fn current_status_override(&self, width: usize) -> Option<String> {
        if self.current_room_is_life() {
            return Some(if width <= 400 {
                self.life_session.compact_status()
            } else {
                self.life_session.status()
            });
        }
        if self.current_room_is_times_tables() && !self.the_show {
            let phase = effective_room_phase("times-tables", self.t, &self.inputs, self.the_show);
            let dial = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.times_tables_aha.status(dial.as_deref()));
        }
        if self.current_room_is_buffon() && !self.the_show {
            let phase = effective_room_phase("buffon-needle", self.t, &self.inputs, self.the_show);
            let throws = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.buffon_aha.status(throws.as_deref()));
        }
        if self.current_room_is_pendulum() && !self.the_show {
            let phase =
                effective_room_phase("double-pendulum", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.pendulum_aha.status(readout.as_deref()));
        }
        if self.current_room_is_kepler() && !self.the_show {
            let phase = effective_room_phase("kepler-laws", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.kepler_aha.status(readout.as_deref()));
        }
        if self.current_room_is_parrondo() && !self.the_show {
            let phase = effective_room_phase("parrondo", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.parrondo_aha.status(readout.as_deref()));
        }
        if self.current_room_is_nontransitive() && !self.the_show {
            let phase = effective_room_phase("nontransitive", self.t, &self.inputs, self.the_show);
            let readout = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.nontransitive_aha.status(readout.as_deref()));
        }
        if let Some(posed) = &self.room_wager {
            return Some(posed.status());
        }
        if self.current_room_is_galton() && !self.the_show {
            let phase = effective_room_phase("galton-board", self.t, &self.inputs, self.the_show);
            let pile = self.rooms[self.current]
                .status_input(phase, &self.inputs)
                .or_else(|| self.rooms[self.current].status(phase));
            return Some(self.galton_aha.status(pile.as_deref()));
        }
        None
    }

    fn reset_life_session(&mut self) {
        self.life_session = numinous_core::rooms::game_of_life::LifeSession::new(self.variation);
        self.life_accumulator = 0.0;
        self.clear_transient_audio();
    }

    fn reset_times_tables_aha(&mut self) {
        self.times_tables_aha = numinous_core::rooms::times_tables_aha::TimesTablesAha::new();
        if self.current_room_is_times_tables() {
            self.show_info = false;
        }
    }

    fn reset_buffon_aha(&mut self) {
        self.buffon_aha = numinous_core::rooms::buffon_aha::BuffonAha::new();
        if self.current_room_is_buffon() {
            self.show_info = false;
        }
    }

    fn reset_pendulum_aha(&mut self) {
        self.pendulum_aha = numinous_core::rooms::pendulum_aha::PendulumAha::new(self.variation);
        if self.current_room_is_pendulum() {
            self.show_info = false;
        }
    }

    fn reset_kepler_aha(&mut self) {
        let eccentricity = numinous_core::rooms::kepler_laws::eccentricity_for_inputs(
            self.t,
            &self.inputs,
            self.variation,
        );
        self.kepler_aha = numinous_core::rooms::kepler_aha::KeplerAha::new(eccentricity);
        if self.current_room_is_kepler() {
            self.show_info = false;
        }
    }

    fn reset_parrondo_aha(&mut self) {
        self.parrondo_aha = numinous_core::rooms::parrondo_aha::ParrondoAha::new();
        if self.current_room_is_parrondo() {
            self.show_info = false;
        }
    }

    fn reset_nontransitive_aha(&mut self) {
        self.nontransitive_aha = numinous_core::rooms::nontransitive_aha::NontransitiveAha::new();
        if self.current_room_is_nontransitive() {
            self.show_info = false;
        }
    }

    /// U poses the room's own prediction, or closes an open one.
    ///
    /// Every room with a moving numeric readout can be called, which is
    /// most of the catalog; the flagship rooms keep their hand-staged
    /// ahas instead, because a bespoke five-beat arc outranks the generic
    /// one where it exists.
    fn toggle_room_wager(&mut self) {
        if self.the_show || self.studio || self.arcade.is_some() {
            return;
        }
        if self.room_wager.take().is_some() {
            return;
        }
        if self.current_room_is_times_tables()
            || self.current_room_is_buffon()
            || self.current_room_is_galton()
            || self.current_room_is_pendulum()
            || self.current_room_is_kepler()
            || self.current_room_is_parrondo()
            || self.current_room_is_nontransitive()
        {
            self.banner = Some(feedback::Banner::status(
                "THIS ROOM STAGES ITS OWN WAGER",
                feedback::REFUSAL_FRAMES,
            ));
            return;
        }
        let room = self.rooms[self.current].as_ref();
        match wager::RoomWager::pose(room, self.variation) {
            Some(posed) => {
                self.show_info = false;
                self.room_wager = Some(posed);
            }
            None => {
                self.banner = Some(feedback::Banner::status(
                    "THIS ROOM READS NO NUMBER TO CALL",
                    feedback::REFUSAL_FRAMES,
                ));
            }
        }
    }

    /// Commit the posed call and meet the truth.
    fn commit_room_wager(&mut self) {
        let Some(mut posed) = self.room_wager.take() else {
            return;
        };
        let room = self.rooms[self.current].as_ref();
        if posed.commit(room).is_some()
            && let Some(verdict) = posed.verdict()
        {
            self.banner = Some(feedback::Banner::status(
                verdict.to_uppercase(),
                feedback::REFUSAL_FRAMES,
            ));
        }
        self.room_wager = Some(posed);
    }

    fn reset_galton_aha(&mut self) {
        self.galton_aha = numinous_core::rooms::galton_aha::GaltonAha::new();
        if self.current_room_is_galton() {
            self.show_info = false;
        }
    }

    /// Keep the Times Tables aha in step with hand dial and the four-lobe goal.
    fn sync_times_tables_aha(&mut self) {
        if !self.current_room_is_times_tables() || self.the_show {
            return;
        }
        let phase = effective_room_phase("times-tables", self.t, &self.inputs, false);
        let room = numinous_core::rooms::times_tables::TimesTables::new_with(self.variation);
        if has_finite_parameter_input(&self.inputs) {
            let k = room.live_multiplier(phase, &self.inputs);
            self.times_tables_aha.note_hand_multiplier(k);
        }
        if room.goal_met(phase, &self.inputs) {
            let _ = self.times_tables_aha.note_four_lobes();
        }
    }

    /// Keep the Buffon aha in step with player throws.
    fn sync_buffon_aha(&mut self) {
        if !self.current_room_is_buffon() || self.the_show {
            return;
        }
        let throws = numinous_core::rooms::buffon_needle::BuffonNeedle::throw_count(&self.inputs);
        self.buffon_aha.note_throws(throws);
    }

    /// Keep the Galton aha in step with the waves the pile is built from.
    fn sync_galton_aha(&mut self) {
        if !self.current_room_is_galton() || self.the_show {
            return;
        }
        let waves = numinous_core::rooms::galton_board::wave_count_from_inputs(&self.inputs);
        let coin = numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
            .unwrap_or(2);
        self.galton_aha.note_waves(waves, coin);
    }

    /// Keep the Double Pendulum aha in step with completed releases.
    fn sync_pendulum_aha(&mut self) {
        if !self.current_room_is_pendulum() || self.the_show {
            return;
        }
        let room = numinous_core::rooms::double_pendulum::DoublePendulum::new_with(self.variation);
        if let Some(gap) = room.divergence_at_full_sweep_for_inputs(&self.inputs) {
            let _ = self.pendulum_aha.bind_truth_gap(gap);
        }
        let drops = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        self.pendulum_aha.note_drops(drops);
    }

    /// Keep the Kepler aha bound to the ellipse chosen by completed drags.
    fn sync_kepler_aha(&mut self) {
        if !self.current_room_is_kepler() || self.the_show {
            return;
        }
        let eccentricity = numinous_core::rooms::kepler_laws::eccentricity_for_inputs(
            self.t,
            &self.inputs,
            self.variation,
        );
        let _ = self.kepler_aha.bind_eccentricity(eccentricity);
        let tunings = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        self.kepler_aha.note_tunings(tunings);
    }

    /// Keep the Parrondo aha in step with completed policy selections.
    fn sync_parrondo_aha(&mut self) {
        if !self.current_room_is_parrondo() || self.the_show {
            return;
        }
        let selections = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        self.parrondo_aha.note_selections(selections);
    }

    /// Keep the dice aha bound to the newest completed die choice.
    fn sync_nontransitive_aha(&mut self) {
        if !self.current_room_is_nontransitive() || self.the_show {
            return;
        }
        let choices = self
            .inputs
            .iter()
            .filter(|input| matches!(input, numinous_core::RoomInput::PointerUp { .. }))
            .count();
        let chosen = numinous_core::rooms::nontransitive::selected_die_from_inputs(&self.inputs);
        self.nontransitive_aha.note_choices(chosen, choices);
    }

    fn record_current_aha_consolidation(&mut self) {
        let room_id = self.rooms[self.current].meta().id;
        let consolidated = match room_id {
            "times-tables" => self.times_tables_aha.allow_reveal_text(),
            "buffon-needle" => self.buffon_aha.allow_reveal_text(),
            "galton-board" => self.galton_aha.allow_reveal_text(),
            "double-pendulum" => self.pendulum_aha.allow_reveal_text(),
            "kepler-laws" => self.kepler_aha.allow_reveal_text(),
            "parrondo" => self.parrondo_aha.allow_reveal_text(),
            "nontransitive" => self.nontransitive_aha.allow_reveal_text(),
            _ => false,
        };
        if consolidated && self.journey.consolidate(room_id) {
            self.journey_changed();
        }
    }

    /// E / Inspect: summon staged aha on flagship rooms; elsewhere toggle reveal.
    fn toggle_inspect(&mut self) {
        if self.the_show || self.studio {
            return;
        }
        if self.current_room_is_times_tables() {
            use numinous_core::rooms::times_tables_aha::AhaBeat;
            if self.times_tables_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.times_tables_aha.can_summon()
                || matches!(self.times_tables_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.times_tables_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            // Generation first: do not open the punchline card early.
            self.show_info = false;
            return;
        }
        if self.current_room_is_buffon() {
            use numinous_core::rooms::buffon_aha::AhaBeat;
            if self.buffon_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.buffon_aha.can_summon()
                || matches!(self.buffon_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.buffon_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_galton() {
            use numinous_core::rooms::galton_aha::AhaBeat;
            if self.galton_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.galton_aha.can_summon()
                || matches!(self.galton_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.galton_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_pendulum() {
            use numinous_core::rooms::pendulum_aha::AhaBeat;
            if self.pendulum_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.pendulum_aha.can_summon()
                || matches!(self.pendulum_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.pendulum_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_kepler() {
            use numinous_core::rooms::kepler_aha::AhaBeat;
            if self.kepler_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.kepler_aha.can_summon()
                || matches!(self.kepler_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.kepler_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_parrondo() {
            use numinous_core::rooms::parrondo_aha::AhaBeat;
            if self.parrondo_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.parrondo_aha.can_summon()
                || matches!(self.parrondo_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.parrondo_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        if self.current_room_is_nontransitive() {
            use numinous_core::rooms::nontransitive_aha::AhaBeat;
            if self.nontransitive_aha.allow_reveal_text() {
                self.show_info = !self.show_info;
                return;
            }
            if self.nontransitive_aha.can_summon()
                || matches!(self.nontransitive_aha.beat(), AhaBeat::Morph { .. })
            {
                if self.nontransitive_aha.summon() {
                    self.show_info = false;
                    self.record_current_aha_consolidation();
                }
                return;
            }
            self.show_info = false;
            return;
        }
        self.show_info = !self.show_info;
    }

    fn commit_times_tables_wager(
        &mut self,
        place: numinous_core::rooms::times_tables_aha::CardioidHome,
    ) -> bool {
        if !self.current_room_is_times_tables() || self.the_show {
            return false;
        }
        if self.times_tables_aha.commit_wager(place) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("GUESSED {}", place.label()),
                90,
            ));
            true
        } else {
            false
        }
    }

    fn advance_times_tables_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_times_tables() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.times_tables_aha.beat(),
            numinous_core::rooms::times_tables_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        let delta = elapsed / TIMES_TABLES_MORPH_SECONDS;
        self.times_tables_aha.advance_morph(delta);
    }

    fn commit_buffon_wager(&mut self, guess: f64) -> bool {
        if !self.current_room_is_buffon() || self.the_show {
            return false;
        }
        if self.buffon_aha.commit_wager(guess) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(format!("GUESSED {guess:.2}"), 90));
            true
        } else {
            false
        }
    }

    fn advance_buffon_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_buffon() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.buffon_aha.beat(),
            numinous_core::rooms::buffon_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        let delta = elapsed / BUFFON_MORPH_SECONDS;
        self.buffon_aha.advance_morph(delta);
    }

    fn commit_galton_wager(&mut self, bin: usize) -> bool {
        if !self.current_room_is_galton() || self.the_show {
            return false;
        }
        let coin = numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
            .unwrap_or(2);
        if self.galton_aha.commit_wager(bin, coin) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(format!("GUESSED BIN {bin}"), 90));
            true
        } else {
            false
        }
    }

    fn advance_galton_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_galton() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.galton_aha.beat(),
            numinous_core::rooms::galton_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        let delta = elapsed / BUFFON_MORPH_SECONDS;
        self.galton_aha.advance_morph(delta);
    }

    fn commit_pendulum_call(&mut self, ending: numinous_core::rooms::pendulum_aha::Ending) -> bool {
        if !self.current_room_is_pendulum() || self.the_show {
            return false;
        }
        if self.pendulum_aha.commit_call(ending) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", ending.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    fn advance_pendulum_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_pendulum() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.pendulum_aha.beat(),
            numinous_core::rooms::pendulum_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.pendulum_aha
            .advance_morph(elapsed / PENDULUM_MORPH_SECONDS);
    }

    fn commit_kepler_call(
        &mut self,
        relation: numinous_core::rooms::kepler_aha::SpeedRelation,
    ) -> bool {
        if !self.current_room_is_kepler() || self.the_show {
            return false;
        }
        if self.kepler_aha.commit_call(relation) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", relation.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    fn advance_kepler_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_kepler() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.kepler_aha.beat(),
            numinous_core::rooms::kepler_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.kepler_aha
            .advance_morph(elapsed / KEPLER_MORPH_SECONDS);
    }

    fn commit_parrondo_call(&mut self, policy: numinous_core::rooms::parrondo::Policy) -> bool {
        if !self.current_room_is_parrondo() || self.the_show {
            return false;
        }
        if self.parrondo_aha.commit_call(policy) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", policy.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    fn advance_parrondo_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_parrondo() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.parrondo_aha.beat(),
            numinous_core::rooms::parrondo_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.parrondo_aha
            .advance_morph(elapsed / PARRONDO_MORPH_SECONDS);
    }

    fn commit_nontransitive_call(&mut self, die: numinous_core::rooms::nontransitive::Die) -> bool {
        if !self.current_room_is_nontransitive() || self.the_show {
            return false;
        }
        if self.nontransitive_aha.commit_call(die) {
            self.show_info = false;
            self.banner = Some(feedback::Banner::status(
                format!("CALLED {}", die.name()),
                90,
            ));
            true
        } else {
            false
        }
    }

    fn advance_nontransitive_morph(&mut self, elapsed: f64) {
        if !self.current_room_is_nontransitive() || self.the_show || self.paused {
            return;
        }
        if !matches!(
            self.nontransitive_aha.beat(),
            numinous_core::rooms::nontransitive_aha::AhaBeat::Morph { .. }
        ) {
            return;
        }
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return;
        }
        self.nontransitive_aha
            .advance_morph(elapsed / NONTRANSITIVE_MORPH_SECONDS);
    }

    fn record_room_touch(&mut self, point: (f64, f64)) -> bool {
        let poke_added = room_input::push_poke(&mut self.pokes, point);
        let input_added = room_input::record_pointer_down(&mut self.inputs, point, self.t);
        if poke_added && input_added && self.current_room_is_life() {
            let launched = self.life_session.launch(point);
            if launched {
                self.life_accumulator = 0.0;
                self.clear_transient_audio();
            }
            return launched;
        }
        let accepted = poke_added && input_added;
        if accepted {
            self.maybe_announce_room_goal();
            self.sync_room_parameter_voice();
            self.play_room_interaction_audio(true);
        }
        accepted
    }

    fn play_room_interaction_audio(&self, accepted: bool) {
        #[cfg(test)]
        if selected_room_interaction_audio(
            self.audio_program,
            self.modal_mode_active(),
            self.muted,
            accepted,
            self.rooms[self.current].as_ref(),
            &self.inputs,
            48_000,
        )
        .is_some()
        {
            self.interaction_audio_events
                .set(self.interaction_audio_events.get().saturating_add(1));
        }
        let Some(player) = &self.player else {
            return;
        };
        let Some(samples) = selected_room_interaction_audio(
            self.audio_program,
            self.modal_mode_active(),
            self.muted,
            accepted,
            self.rooms[self.current].as_ref(),
            &self.inputs,
            player.sample_rate(),
        ) else {
            return;
        };
        player.play_stereo_oneshot(samples, 0.65);
    }

    fn maybe_announce_room_goal(&mut self) {
        if self.goal_announced || !self.rooms[self.current].goal_met(self.t, &self.inputs) {
            return;
        }
        self.goal_announced = true;
        self.banner = Some(feedback::room_goal(
            self.rooms[self.current]
                .goal()
                .unwrap_or("DISCOVERY COMPLETE"),
        ));
    }

    fn advance_life(&mut self, elapsed: f64) -> usize {
        if !self.current_room_is_life() || !elapsed.is_finite() || elapsed <= 0.0 {
            return 0;
        }
        let max_backlog = LIFE_STEP_SECONDS * MAX_LIFE_STEPS_PER_TICK as f64;
        self.life_accumulator = (self.life_accumulator + elapsed).min(max_backlog);
        let steps = ((self.life_accumulator + 1e-9) / LIFE_STEP_SECONDS).floor() as usize;
        let steps = steps.min(MAX_LIFE_STEPS_PER_TICK);
        for _ in 0..steps {
            self.life_session.advance();
        }
        self.life_accumulator -= steps as f64 * LIFE_STEP_SECONDS;
        // A catch-up tick presents only the newest generation. Voice that same
        // state once instead of replaying a stale burst after the picture.
        self.play_life_step_audio(steps);
        steps
    }

    fn play_life_step_audio(&self, completed_steps: usize) {
        let Some(player) = &self.player else {
            return;
        };
        let Some(samples) = selected_life_step_audio(
            self.audio_program,
            self.modal_mode_active(),
            self.muted,
            completed_steps,
            &self.life_session,
            player.sample_rate(),
        ) else {
            return;
        };
        player.play_stereo_oneshot(samples, 0.65);
    }

    fn advance_life_if_active(&mut self, elapsed: f64) -> usize {
        if !self.window_active
            || self.paused
            || self.dragging
            || self.show_help && self.menu.is_open()
        {
            return 0;
        }
        self.advance_life(elapsed * self.time_scale * self.visualizer_scale)
    }

    fn draw_studio(&self, raster: &mut Raster, width: usize, height: usize) {
        if let Some(gallery) = &self.gallery {
            gallery.draw(raster, width, height);
            return;
        }
        self.studio_panel.draw_with_controller(
            raster,
            self.input_mode,
            self.gamepad.controller_copy(),
            width,
            height,
            self.t,
        );
        if let Some(naming) = &self.share_naming {
            let scale = ((width as i32) / 450).clamp(1, 3);
            let cursor = |field: NamingField| if naming.field == field { "_" } else { "" };
            let lines = [
                "NAME YOUR SHARE".to_string(),
                format!(
                    "TITLE:  {}{}",
                    naming.title.to_uppercase(),
                    cursor(NamingField::Title)
                ),
                format!(
                    "AUTHOR: {}{}",
                    naming.author.to_uppercase(),
                    cursor(NamingField::Author)
                ),
                "TAB: SWITCH  ENTER: SHARE  ESC: CANCEL".to_string(),
            ];
            let top = (height as i32 / 2 - 24 * scale).max(0);
            for (row, line) in lines.iter().enumerate() {
                numinous_core::draw_text(
                    raster,
                    line,
                    10,
                    top + 12 * scale * row as i32,
                    scale,
                    '#',
                );
            }
        }
    }

    fn modal_frame(&self, width: usize, height: usize) -> Option<Raster> {
        let copy = self.gamepad.controller_copy();
        if let Some(play) = &self.arcade {
            Some(game_draw::draw_arcade(
                play,
                self.input_mode,
                copy,
                width,
                height,
            ))
        } else if let Some(run) = &self.gauntlet {
            Some(game_draw::draw_gauntlet(
                &self.rooms,
                run,
                self.frame,
                self.input_mode,
                copy,
                width,
                height,
            ))
        } else if let Some(play) = &self.munch {
            Some(game_draw::draw_munch(
                play,
                self.frame,
                self.input_mode,
                copy,
                width,
                height,
            ))
        } else if let Some(play) = &self.nim {
            Some(game_draw::draw_nim(
                play,
                self.input_mode,
                copy,
                width,
                height,
            ))
        } else {
            self.quiz.as_ref().map(|quiz| {
                game_draw::draw_quiz(&self.rooms, quiz, self.input_mode, copy, width, height)
            })
        }
    }

    fn draw(&mut self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let (width, height) = (w.get() as usize, h.get() as usize);

        // Render the frame fully before borrowing the window surface. Fractal
        // rooms take the GPU path when one exists; their frames rejoin the same
        // interface path as CPU rooms before presentation.
        if self.session_viewer.is_open() {
            let viewer_input_mode = match self.input_mode {
                input_legend::InputMode::KeyboardMouse => ViewerInputMode::KeyboardMouse,
                input_legend::InputMode::Controller => {
                    ViewerInputMode::MappedController(self.gamepad.controller_copy())
                }
            };
            let raster = self.session_viewer.draw(width, height, viewer_input_mode);
            // Cache is warm after draw; publish sequence-owned sound once.
            self.sync_viewer_audio();
            self.present_raster(raster, width, height);
            return;
        }
        if let Some(raster) = self.modal_frame(width, height) {
            self.present_raster(raster, width, height);
            return;
        }
        if !self.studio
            && let Some(rgba) = self.gpu_frame(width, height)
            && let Some(mut raster) =
                Raster::from_rgba(width, height, self.rooms[self.current].meta().accent, &rgba)
        {
            let room = &self.rooms[self.current];
            input_feedback::draw(
                &mut raster,
                effective_room_inputs(&self.inputs, self.the_show),
            );
            self.draw_room_interface(&mut raster, room.as_ref(), width, height);
            self.present_raster(raster, width, height);
            return;
        }
        let room = &self.rooms[self.current];
        let mut raster = if self.studio {
            let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
            self.draw_studio(&mut raster, width, height);
            raster
        } else {
            // Heavy CPU rooms render below window resolution and expand by an
            // integer factor chosen from measured frame time (see live_render);
            // the HUD below draws after the upscale, so its text stays crisp.
            let factor = self.live_scale.factor();
            let (rw, rh) = self.live_scale.render_size(width, height);
            let started = std::time::Instant::now();
            let mut raster = Raster::with_accent(rw, rh, room.meta().accent);
            let room_inputs = effective_room_inputs(&self.inputs, self.the_show);
            if room.meta().id == "mandelbrot" {
                self.mandelbrot_camera.render(&mut raster);
            } else if room.meta().id == "game-of-life" {
                self.life_session.render(&mut raster);
            } else if room.meta().id == "times-tables"
                && !self.the_show
                && self.times_tables_aha.uses_aha_plate()
            {
                let phase =
                    effective_room_phase(room.meta().id, self.t, &self.inputs, self.the_show);
                let k = numinous_core::rooms::times_tables::TimesTables::new_with(self.variation)
                    .live_multiplier(phase, room_inputs);
                numinous_core::rooms::times_tables_aha::render_aha_plate(
                    &mut raster,
                    self.times_tables_aha.beat(),
                    k,
                );
            } else {
                let phase =
                    effective_room_phase(room.meta().id, self.t, &self.inputs, self.the_show);
                room.render_input(&mut raster, phase, room_inputs);
                if room.meta().id == "times-tables"
                    && !self.the_show
                    && matches!(
                        self.times_tables_aha.beat(),
                        numinous_core::rooms::times_tables_aha::AhaBeat::Prime
                    )
                {
                    numinous_core::rooms::times_tables_aha::render_wager_options(
                        &mut raster,
                        self.times_tables_aha.hover(),
                    );
                }
                if room.meta().id == "buffon-needle" && !self.the_show {
                    if matches!(
                        self.buffon_aha.beat(),
                        numinous_core::rooms::buffon_aha::AhaBeat::Prime
                    ) {
                        numinous_core::rooms::buffon_aha::render_guess_band(
                            &mut raster,
                            self.buffon_aha.hover(),
                        );
                    }
                    if self.buffon_aha.uses_circle_overlay() {
                        let progress = match self.buffon_aha.beat() {
                            numinous_core::rooms::buffon_aha::AhaBeat::Morph { progress } => {
                                progress
                            }
                            _ => 1.0,
                        };
                        numinous_core::rooms::buffon_aha::render_circle_overlay(
                            &mut raster,
                            progress,
                        );
                    }
                }
                if room.meta().id == "double-pendulum" && !self.the_show {
                    if matches!(
                        self.pendulum_aha.beat(),
                        numinous_core::rooms::pendulum_aha::AhaBeat::Prime
                    ) {
                        numinous_core::rooms::pendulum_aha::render_ending_band(
                            &mut raster,
                            self.pendulum_aha.hover(),
                        );
                    }
                    if self.pendulum_aha.uses_curve_overlay() {
                        let progress = match self.pendulum_aha.beat() {
                            numinous_core::rooms::pendulum_aha::AhaBeat::Morph { progress } => {
                                progress
                            }
                            _ => 1.0,
                        };
                        numinous_core::rooms::pendulum_aha::render_gap_curve_for_inputs(
                            &mut raster,
                            progress,
                            self.variation,
                            &self.inputs,
                        );
                    }
                }
                if room.meta().id == "kepler-laws" && !self.the_show {
                    if matches!(
                        self.kepler_aha.beat(),
                        numinous_core::rooms::kepler_aha::AhaBeat::Prime
                    ) {
                        numinous_core::rooms::kepler_aha::render_speed_band(
                            &mut raster,
                            self.kepler_aha.hover(),
                        );
                    }
                    if self.kepler_aha.uses_time_overlay() {
                        let progress = match self.kepler_aha.beat() {
                            numinous_core::rooms::kepler_aha::AhaBeat::Morph { progress } => {
                                progress
                            }
                            _ => 1.0,
                        };
                        numinous_core::rooms::kepler_aha::render_equal_time_overlay(
                            &mut raster,
                            progress,
                            self.kepler_aha.eccentricity(),
                        );
                    }
                }
                if room.meta().id == "parrondo" && !self.the_show {
                    if matches!(
                        self.parrondo_aha.beat(),
                        numinous_core::rooms::parrondo_aha::AhaBeat::Prime
                    ) {
                        numinous_core::rooms::parrondo_aha::render_policy_band(
                            &mut raster,
                            self.parrondo_aha.hover(),
                        );
                    }
                    if self.parrondo_aha.uses_expectation_overlay() {
                        let progress = match self.parrondo_aha.beat() {
                            numinous_core::rooms::parrondo_aha::AhaBeat::Morph { progress } => {
                                progress
                            }
                            _ => 1.0,
                        };
                        numinous_core::rooms::parrondo_aha::render_expectation_overlay(
                            &mut raster,
                            progress,
                        );
                    }
                }
                if room.meta().id == "nontransitive" && !self.the_show {
                    if matches!(
                        self.nontransitive_aha.beat(),
                        numinous_core::rooms::nontransitive_aha::AhaBeat::Prime
                    ) {
                        numinous_core::rooms::nontransitive_aha::render_counter_band(
                            &mut raster,
                            self.nontransitive_aha.hover(),
                        );
                    }
                    if self.nontransitive_aha.uses_outcome_grid()
                        && let Some(chosen) = self.nontransitive_aha.chosen()
                    {
                        let progress = match self.nontransitive_aha.beat() {
                            numinous_core::rooms::nontransitive_aha::AhaBeat::Morph {
                                progress,
                            } => progress,
                            _ => 1.0,
                        };
                        numinous_core::rooms::nontransitive_aha::render_outcome_grid(
                            &mut raster,
                            progress,
                            chosen,
                        );
                    }
                }
                if !self.the_show
                    && let Some(posed) = &self.room_wager
                {
                    posed.draw(&mut raster);
                }
                if room.meta().id == "galton-board" && !self.the_show {
                    if matches!(
                        self.galton_aha.beat(),
                        numinous_core::rooms::galton_aha::AhaBeat::Prime
                    ) {
                        numinous_core::rooms::galton_aha::render_bin_band(
                            &mut raster,
                            self.galton_aha.hover(),
                        );
                    }
                    // The curve answers the call, so it is the called
                    // coin's curve, and it is drawn only while the pile
                    // underneath is that same experiment. A player who
                    // wanders to another coin gets no curve over the wrong
                    // pile; the footer says which pile the call was about,
                    // and the curve returns when they do.
                    let live_coin =
                        numinous_core::rooms::galton_board::selected_coin_from_inputs(&self.inputs)
                            .unwrap_or(2);
                    if self.galton_aha.uses_outline_overlay()
                        && self.galton_aha.answers_pile(live_coin)
                    {
                        let progress = match self.galton_aha.beat() {
                            numinous_core::rooms::galton_aha::AhaBeat::Morph { progress } => {
                                progress
                            }
                            _ => 1.0,
                        };
                        let coin = self.galton_aha.coin().unwrap_or(live_coin);
                        numinous_core::rooms::galton_aha::render_outline_overlay(
                            &mut raster,
                            progress,
                            coin,
                        );
                    }
                }
            }
            input_feedback::draw(&mut raster, room_inputs);
            self.live_scale
                .observe(started.elapsed().as_secs_f64() * 1000.0);
            if factor > 1 {
                raster.upscaled(factor, width, height)
            } else {
                raster
            }
        };

        self.draw_room_interface(&mut raster, room.as_ref(), width, height);
        self.present_raster(raster, width, height);
    }

    fn draw_room_interface(
        &self,
        raster: &mut Raster,
        room: &dyn Room,
        width: usize,
        height: usize,
    ) {
        let status_override = self.current_status_override(width);
        let phase = effective_room_phase(room.meta().id, self.t, &self.inputs, self.the_show);
        let inputs = effective_room_inputs(&self.inputs, self.the_show);
        // Flagship ahas gate the text reveal until the morph consolidates.
        let show_info = self.show_info
            && (self.the_show
                || (!self.current_room_is_times_tables()
                    && !self.current_room_is_buffon()
                    && !self.current_room_is_galton()
                    && !self.current_room_is_pendulum()
                    && !self.current_room_is_kepler()
                    && !self.current_room_is_parrondo()
                    && !self.current_room_is_nontransitive())
                || (self.current_room_is_times_tables()
                    && self.times_tables_aha.allow_reveal_text())
                || (self.current_room_is_buffon() && self.buffon_aha.allow_reveal_text())
                || (self.current_room_is_galton() && self.galton_aha.allow_reveal_text())
                || (self.current_room_is_pendulum() && self.pendulum_aha.allow_reveal_text())
                || (self.current_room_is_kepler() && self.kepler_aha.allow_reveal_text())
                || (self.current_room_is_parrondo() && self.parrondo_aha.allow_reveal_text())
                || (self.current_room_is_nontransitive()
                    && self.nontransitive_aha.allow_reveal_text()));
        hud::draw_room_chrome(
            raster,
            room,
            &hud::RoomChrome {
                t: phase,
                room_card: self.room_card,
                show_info,
                show_help: self.show_help,
                show_journey: self.show_journey,
                banner_active: self.banner.is_some(),
                the_show: self.the_show,
                studio: self.studio,
                muted: self.muted,
                level: self.journey.level(),
                input_mode: self.input_mode,
                controller_face: self.gamepad.controller_copy(),
            },
            inputs,
            status_override.as_deref(),
            width,
            height,
        );

        if self.show_journey && !self.the_show {
            let board = numinous_core::load_scoreboard_file(&self.scores_file);
            overlays::draw_journey_overlay_with_controller(
                raster,
                &self.journey,
                &board,
                self.rooms.len(),
                (width, height),
                self.input_mode,
                self.gamepad.controller_copy(),
            );
        }

        if self.console.is_open() {
            console::draw(raster, &self.console, width, height);
        }

        if self.input_mode == input_legend::InputMode::Controller
            && let Some(point) = self.gamepad.cursor()
        {
            gamepad::draw_cursor(raster, point, width, height);
        }
    }

    fn draw_menu_overlay(&self, raster: &mut Raster) {
        if !self.show_help || !self.menu.is_open() {
            return;
        }
        let (window_mode, fullscreen) =
            match self.window.as_ref().and_then(|window| window.fullscreen()) {
                Some(winit::window::Fullscreen::Borderless(_)) => ("borderless", true),
                Some(winit::window::Fullscreen::Exclusive(_)) => ("exclusive", true),
                None => ("windowed", false),
            };
        let _ = menu::draw_menu(
            raster,
            &self.menu,
            self.input_mode,
            self.gamepad.controller_copy(),
            menu::MenuReadout {
                volume_percent: (self.volume * 100.0).round().clamp(0.0, 100.0) as u8,
                muted: self.muted,
                era: self.era.name(),
                window_mode,
                fullscreen,
            },
        );
    }

    fn present_raster(&mut self, mut raster: Raster, width: usize, height: usize) {
        if self.paused {
            overlays::draw_pause_overlay_with_controller(
                &mut raster,
                width,
                height,
                self.input_mode,
                self.gamepad.controller_copy(),
            );
        }
        self.draw_banner_on_raster(&mut raster, width, height);
        hud::draw_audio_state(&mut raster, &self.audio_state(), width);
        // Visualizer path: room bed, mixed output tap, or OS loopback capture.
        // Output mix and loopback drive a scale multiplier and soft beat pokes.
        self.visualizer_scale = 1.0;
        if !self.muted
            && let Some((bands, source)) = self.visualizer_bands()
        {
            hud::draw_spectrum_meter(&mut raster, &bands, width, height);
            let levers = numinous_core::levers_from_bands(&self.spectrum_prev, &bands);
            let drive = matches!(
                source,
                numinous_audio::VisualizerSource::OutputMix
                    | numinous_audio::VisualizerSource::Loopback
            );
            if drive && !self.modal_mode_active() && !self.paused {
                // Bass pumps motion without rewriting the player's time_scale.
                self.visualizer_scale = numinous_core::spectrum_time_scale(1.0, &levers);
                let nudge = numinous_core::spectrum_phase_nudge(&levers);
                if nudge > 0.0 {
                    self.t = (self.t + nudge).rem_euclid(1.0);
                }
                if numinous_core::spectrum_should_poke(&levers)
                    && !self.studio
                    && !self.the_show
                    && self.pokes.len() < 24
                {
                    let (x, y) = numinous_core::spectrum_hand_point(&levers);
                    // Avoid stacking identical soft beats in one breath.
                    let last = self.pokes.last().copied();
                    if last != Some((x, y)) {
                        self.pokes.push((x, y));
                    }
                }
            }
            self.spectrum_prev = bands;
        }
        self.draw_menu_overlay(&mut raster);
        let (rw, rh) = (raster.width(), raster.height());
        let mut rgba = raster.to_rgba();
        self.era.apply(&mut rgba, rw, rh);
        if self.the_show
            && self.show_crossfade_frames > 0
            && let Some(prev) = self.show_crossfade_prev.as_ref()
            && prev.len() == rgba.len()
        {
            let weight = f32::from(self.show_crossfade_frames) / f32::from(SHOW_CROSSFADE_FRAMES);
            blend_rgba(&mut rgba, prev, weight);
            self.show_crossfade_frames = self.show_crossfade_frames.saturating_sub(1);
        }
        if self.the_show {
            self.show_crossfade_prev = Some(rgba.clone());
        } else {
            self.show_crossfade_prev = None;
            self.show_crossfade_frames = 0;
        }
        if self.screen_shake > 0 {
            apply_screen_shake(&mut rgba, rw, rh, self.screen_shake);
            self.screen_shake = self.screen_shake.saturating_sub(1);
        }
        self.blit(&rgba, rw, rh, width, height);
    }

    fn audio_state(&self) -> hud::AudioState {
        let program = if self.session_viewer.is_open() {
            AudioProgram::WatchAgent
        } else {
            self.audio_program
        };
        audio_state::describe(
            program,
            self.radio
                .and_then(|index| numinous_core::STATIONS.get(index))
                .map(|station| station.name),
            self.volume,
            self.muted,
            self.window_active,
            self.player.is_some(),
        )
    }

    fn draw_banner_on_raster(&self, raster: &mut Raster, width: usize, height: usize) {
        if let Some(banner) = &self.banner {
            overlays::draw_banner(raster, banner.lines(), width, height);
        }
    }

    /// Present an RGBA frame (`rw` x `rh`) in the window (`width` x `height`).
    fn blit(&mut self, rgba: &[u8], rw: usize, rh: usize, width: usize, height: usize) {
        let Some(presenter) = self.presenter.as_mut() else {
            return;
        };
        match presenter.present(rgba, rw, rh, width, height) {
            Ok(presentation::PresentOutcome::Presented {
                #[cfg(feature = "gpu-post")]
                    gpu_frame: _gpu_frame,
            }) => {
                self.presentation_warned = false;
            }
            Ok(presentation::PresentOutcome::Skipped) => {}
            #[cfg(feature = "gpu-post")]
            Ok(presentation::PresentOutcome::FellBack(reason)) => {
                if !self.presentation_warned {
                    self.banner = Some(feedback::gpu_post_unavailable(&reason));
                    let _ = append_crash_log_at(
                        &self.crash_log,
                        &format!("GPU presentation fell back to software: {reason}\n"),
                    );
                }
                self.presentation_warned = true;
            }
            Err(error) => {
                if !self.presentation_warned {
                    self.banner = Some(feedback::presentation_unavailable(&error));
                    let _ = append_crash_log_at(
                        &self.crash_log,
                        &format!("window presentation failed: {error}\n"),
                    );
                }
                self.presentation_warned = true;
            }
        }
    }

    fn exit_app(&mut self, event_loop: &ActiveEventLoop) {
        self.quit_requested = false;
        self.session_viewer.close();
        if let Err(error) = numinous_core::persist_journey_delta(
            &self.journey_file,
            &self.journey_saved,
            &self.journey,
        ) {
            let _ = append_crash_log_at(
                &self.crash_log,
                &format!("journey save at exit failed: {error}\n"),
            );
        }
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.resume_presentation_clock(now);
        if self.window.is_some() {
            self.gamepad.activate();
            if self.radio.is_some() && !self.studio {
                let _ = self.sync_radio_to_wall_clock();
                if let Some(window) = &self.window {
                    window.set_title(&self.title());
                }
            }
            if let Some(player) = &self.player {
                player.set_active(true);
            }
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(self.title())
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 900.0))
            .with_window_icon(app_icon())
            .with_maximized(true);
        let Ok(window) = event_loop.create_window(attributes) else {
            return;
        };
        let window = Arc::new(window);
        let initial_size = window.inner_size();
        match presentation::WindowPresenter::new(
            window.clone(),
            initial_size.width,
            initial_size.height,
        ) {
            Ok(presenter) => self.presenter = Some(presenter),
            Err(error) => {
                self.banner = Some(feedback::presentation_unavailable(&error));
                let _ = append_crash_log_at(
                    &self.crash_log,
                    &format!("window presentation initialization failed: {error}\n"),
                );
            }
        }
        self.window = Some(window);
        if let Some(window) = &self.window {
            let mode = if self.start_fullscreen {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                match self.preferred_window_mode {
                    numinous_core::WindowModePreference::Windowed => None,
                    numinous_core::WindowModePreference::Borderless => {
                        Some(winit::window::Fullscreen::Borderless(None))
                    }
                    numinous_core::WindowModePreference::Exclusive => window
                        .primary_monitor()
                        .and_then(|monitor| monitor.video_modes().next())
                        .map(winit::window::Fullscreen::Exclusive)
                        .or(Some(winit::window::Fullscreen::Borderless(None))),
                }
            };
            window.set_fullscreen(mode);
        }
        self.player = match numinous_audio::LoopPlayer::new() {
            Ok(player) => Some(player),
            Err(error) => {
                // Silence must never be a mystery: say it on screen and in
                // the crash log, then keep running visual-only.
                self.banner = Some(feedback::sound_device_unavailable(&error));
                let _ =
                    append_crash_log_at(&self.crash_log, &format!("audio open failed: {error}\n"));
                None
            }
        };
        self.gpu = numinous_gpu::FractalRenderer::new().ok();
        if std::env::var("NUMINOUS_MUTE").is_ok() {
            self.muted = true;
        }
        self.level_seen = self.journey.level();
        self.visit_current();
        self.update_audio();
        if let Some(input) = self.start_open.take() {
            self.open_start_input(&input);
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.suspend_presentation_clock(Instant::now());
        self.clear_pointer_state();
        if let Some(command) = self.gamepad.deactivate() {
            self.handle_gamepad_command(command);
        }
        if let Some(player) = &self.player {
            player.set_active(false);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.exit_app(event_loop);
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        repeat,
                        ..
                    },
                ..
            } => {
                self.clear_pointer_state();
                if self.handle_quit_key(&logical_key, repeat) {
                    return;
                }
                if self.handle_menu_key(&logical_key, repeat) {
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    return;
                }
                if self.handle_global_audio_key(&logical_key, repeat) {
                    return;
                }
                // Power-user console: when open, it owns the keyboard; ` / ~
                // toggles it from ordinary room play (not Studio text entry).
                if self.console.is_open() {
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    let _ = self.handle_console_key(&logical_key);
                    return;
                }
                if let Key::Character(text) = &logical_key
                    && console::is_toggle_key(text.as_str())
                    && !self.studio
                    && !self.session_viewer.is_open()
                {
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    // Leave games and overlays so the console has a clean room.
                    self.quiz = None;
                    self.munch = None;
                    self.nim = None;
                    self.gauntlet = None;
                    self.arcade = None;
                    self.close_menu();
                    self.show_journey = false;
                    self.console.open();
                    return;
                }
                if self.session_viewer.is_open() {
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    match logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.open_activity_menu(menu::ActivityKind::SharedPlay);
                        }
                        Key::Named(NamedKey::Space) => {
                            self.session_viewer.toggle_display_pause();
                        }
                        Key::Named(NamedKey::ArrowLeft) => self.session_viewer.scrub(-1),
                        Key::Named(NamedKey::ArrowRight) => self.session_viewer.scrub(1),
                        Key::Named(NamedKey::ArrowUp) => {
                            self.session_viewer.scroll_result(-1);
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            self.session_viewer.scroll_result(1);
                        }
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("a") => {
                            self.session_viewer.pan_result(-4);
                        }
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("d") => {
                            self.session_viewer.pan_result(4);
                        }
                        _ => {}
                    }
                    return;
                }
                if self.paused {
                    if logical_key == Key::Named(NamedKey::Space) {
                        self.input_mode = input_legend::InputMode::KeyboardMouse;
                        self.toggle_pause();
                    }
                    return;
                }
                self.input_mode = input_legend::InputMode::KeyboardMouse;
                if self.handle_playtest_shortcut(&logical_key, repeat) {
                    return;
                }
                if logical_key == Key::Named(NamedKey::Escape)
                    && let Some(kind) = self.activity_kind()
                {
                    self.open_activity_menu(kind);
                    return;
                }
                if let Some(play) = &mut self.arcade {
                    if play.over {
                        self.arcade = None;
                        self.update_audio();
                    } else {
                        match logical_key {
                            Key::Named(NamedKey::Escape) => {
                                let (seed, score) = (play.seed, play.run.score);
                                self.post_score(&format!("arcade seed:{seed}"), score);
                                self.arcade = None;
                                self.update_audio();
                            }
                            _ => {
                                if let Some(action) = controls::arcade_action_for_key(&logical_key)
                                {
                                    self.arcade_act(action);
                                }
                            }
                        }
                    }
                } else if self.gauntlet.is_some() {
                    self.gauntlet_key(&logical_key);
                } else if self.munch.is_some() {
                    self.munch_key(&logical_key);
                } else if self.nim.is_some() {
                    self.nim_key(&logical_key);
                } else if let Some(quiz) = &mut self.quiz {
                    // Quiz mode: letters answer; after the reveal, any key deals
                    // the next round; Esc leaves.
                    match logical_key {
                        Key::Named(NamedKey::Escape) => {
                            self.quiz = None;
                            self.update_audio();
                        }
                        _ if quiz.flash.is_some() => self.quiz_next(),
                        Key::Character(c) if c.len() == 1 => {
                            let letter = c.chars().next().unwrap_or(' ').to_ascii_uppercase();
                            self.quiz_answer(letter);
                        }
                        _ => {}
                    }
                } else if self.studio && self.gallery.is_some() {
                    // The Gallery wall owns the keys while it is up: browse,
                    // open, or step back to the Studio underneath.
                    match logical_key {
                        Key::Named(NamedKey::Escape)
                        | Key::Named(NamedKey::Tab)
                        | Key::Named(NamedKey::F5) => {
                            self.gallery = None;
                        }
                        Key::Named(NamedKey::Enter) => self.gallery_open_selected(),
                        // Case-insensitive: the footer advertises F, and a
                        // held Shift or Caps Lock must not unplug it.
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("f") => {
                            self.gallery_fork_selected();
                        }
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("d") => {
                            self.gallery_select_parent();
                        }
                        Key::Named(NamedKey::ArrowLeft) => self.gallery_move(-1, 0),
                        Key::Named(NamedKey::ArrowRight) => self.gallery_move(1, 0),
                        Key::Named(NamedKey::ArrowUp) => self.gallery_move(0, -1),
                        Key::Named(NamedKey::ArrowDown) => self.gallery_move(0, 1),
                        _ => {}
                    }
                } else if self.studio && self.share_naming.is_some() {
                    // The naming step owns the keyboard until the share is
                    // named or abandoned; formula editing waits underneath.
                    match logical_key {
                        Key::Named(NamedKey::Enter) => self.confirm_share_naming(),
                        Key::Named(NamedKey::Escape) => self.cancel_share_naming(),
                        Key::Named(NamedKey::Tab) => self.naming_toggle_field(),
                        Key::Named(NamedKey::Backspace) => self.naming_backspace(),
                        Key::Named(NamedKey::Space) => self.naming_push_text(" "),
                        Key::Character(s) => self.naming_push_text(&s),
                        _ => {}
                    }
                } else if self.studio {
                    // Studio mode: the keyboard is a math keyboard.
                    match logical_key {
                        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                            self.exit_studio();
                        }
                        Key::Named(NamedKey::Enter) => {
                            // A reopened creation waits in a paused preview;
                            // Enter is the consent that starts it singing.
                            self.studio_confirm_opened();
                        }
                        Key::Named(NamedKey::F1) => {
                            self.studio_panel.toggle_help();
                        }
                        Key::Named(NamedKey::F5) => {
                            // The wall of saved creations, discovered fresh on
                            // every open so a new share appears without a
                            // restart.
                            self.gallery = Some(gallery::GalleryPanel::open(
                                &postcard::default_postcard_dir(),
                            ));
                        }
                        Key::Named(NamedKey::F2) => {
                            // Formula Jam Random: draw a curated, tested recipe.
                            let spec = self.studio_panel.load_random_recipe();
                            self.set_studio_recipe_sound(spec);
                        }
                        Key::Named(NamedKey::F3) => {
                            // Formula Jam Auto: calm recipe set; F3 resumes after edit.
                            self.studio_panel.toggle_auto();
                        }
                        Key::Named(NamedKey::F4) => {
                            // The share trio starts with its name: F4 opens
                            // the naming step, Enter there writes the bundle.
                            if self.save_gate.admit(
                                save_gate::SaveKind::StudioShare,
                                Instant::now(),
                                repeat,
                            ) {
                                self.begin_share_naming();
                            }
                        }
                        Key::Named(NamedKey::Backspace) => {
                            let spec = self.studio_panel.backspace();
                            self.set_studio_edit_sound(spec);
                        }
                        Key::Named(NamedKey::Space) => {
                            if self.studio_panel.push_space() {
                                self.set_studio_edit_sound(None);
                            }
                        }
                        Key::Character(s) => {
                            let before = self.studio_panel.source_len();
                            let spec = self.studio_panel.push_text(&s);
                            if self.studio_panel.source_len() != before {
                                self.set_studio_edit_sound(spec);
                            }
                        }
                        _ => {}
                    }
                } else {
                    let logical_key = controls::normalized_command_key(&logical_key);
                    match logical_key {
                        // A posed call takes Esc first: dismissing the band
                        // is a smaller step back than opening the menu.
                        Key::Named(NamedKey::Escape) if self.room_wager.is_some() => {
                            self.room_wager = None;
                        }
                        // Esc is the menu, like every game since Doom. Quit from
                        // the window's close button.
                        Key::Named(NamedKey::Escape) => {
                            if self.the_show {
                                self.toggle_show();
                                self.close_menu();
                            } else {
                                self.open_home_menu();
                            }
                        }
                        // A posed call owns the aiming keys until it is
                        // committed or dismissed: the arrows are the
                        // keyboard's only route to a hand verb inside a
                        // room, so they aim here instead of changing rooms.
                        Key::Named(NamedKey::ArrowRight)
                            if self.room_wager.as_ref().is_some_and(wager::RoomWager::open) =>
                        {
                            if let Some(posed) = self.room_wager.as_mut() {
                                posed.nudge(1);
                            }
                        }
                        Key::Named(NamedKey::ArrowLeft)
                            if self.room_wager.as_ref().is_some_and(wager::RoomWager::open) =>
                        {
                            if let Some(posed) = self.room_wager.as_mut() {
                                posed.nudge(-1);
                            }
                        }
                        Key::Named(NamedKey::Enter)
                            if self.room_wager.as_ref().is_some_and(wager::RoomWager::open) =>
                        {
                            self.commit_room_wager();
                        }
                        // U calls the readout: the universal wager.
                        Key::Character(c) if c.as_str() == "u" => {
                            self.toggle_room_wager();
                        }
                        // Enter is the front-door start: into The Show (the room
                        // tour). Same toggle as B, including from the open menu.
                        Key::Named(NamedKey::Enter) => {
                            if self.show_help {
                                self.activate_menu_choice(input_legend::MenuChoice::Show);
                            } else {
                                self.toggle_show();
                            }
                        }
                        Key::Named(NamedKey::Tab) => {
                            if self.show_help {
                                self.activate_menu_choice(input_legend::MenuChoice::Studio);
                            } else {
                                self.enter_studio();
                            }
                        }
                        // A/D strafe between rooms; arrows still work.
                        Key::Named(NamedKey::ArrowRight) => self.switch(1),
                        Key::Named(NamedKey::ArrowLeft) => self.switch(-1),
                        Key::Character(c) if c.as_str() == "d" => self.switch(1),
                        Key::Character(c) if c.as_str() == "a" => self.switch(-1),
                        // W/S run time faster or slower.
                        Key::Named(NamedKey::ArrowUp) => {
                            self.time_scale = (self.time_scale * 2.0).min(8.0);
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            self.time_scale = (self.time_scale / 2.0).max(0.25);
                        }
                        Key::Character(c) if c.as_str() == "w" => {
                            self.time_scale = (self.time_scale * 2.0).min(8.0);
                        }
                        Key::Character(c) if c.as_str() == "s" => {
                            self.time_scale = (self.time_scale / 2.0).max(0.25);
                        }
                        Key::Named(NamedKey::Space) => self.toggle_pause(),
                        // E / ? opens the optional concept + reveal door.
                        // On Times Tables it summons the staged aha first.
                        Key::Character(c) if c.as_str() == "e" || c.as_str() == "?" => {
                            self.toggle_inspect();
                        }
                        // Times Tables place wager: 1 Mandelbrot, 2 Nephroid, 3 Circle.
                        Key::Character(c)
                            if matches!(c.as_str(), "1" | "2" | "3")
                                && self.current_room_is_times_tables()
                                && !self.the_show =>
                        {
                            let digit = c.as_str().as_bytes()[0].saturating_sub(b'0');
                            if let Some(place) =
                                numinous_core::rooms::times_tables_aha::CardioidHome::from_key_digit(
                                    digit,
                                )
                            {
                                let _ = self.commit_times_tables_wager(place);
                            }
                        }
                        // Buffon number wager: 1=2, 2=e, 3=3, 4=pi.
                        Key::Character(c)
                            if matches!(c.as_str(), "1" | "2" | "3" | "4")
                                && self.current_room_is_buffon()
                                && !self.the_show =>
                        {
                            let digit = c.as_str().as_bytes()[0].saturating_sub(b'0');
                            if let Some(guess) =
                                numinous_core::rooms::buffon_aha::guess_from_key_digit(digit)
                            {
                                let _ = self.commit_buffon_wager(guess);
                            }
                        }
                        // Double Pendulum ending call: 1 together, 2 drifted, 3 lost.
                        Key::Character(c)
                            if matches!(c.as_str(), "1" | "2" | "3")
                                && self.current_room_is_pendulum()
                                && !self.the_show =>
                        {
                            let digit = c.as_str().as_bytes()[0].saturating_sub(b'0');
                            if let Some(ending) =
                                numinous_core::rooms::pendulum_aha::Ending::from_key_digit(digit)
                            {
                                let _ = self.commit_pendulum_call(ending);
                            }
                        }
                        // Kepler speed call: 1 faster, 2 slower, 3 same.
                        Key::Character(c)
                            if matches!(c.as_str(), "1" | "2" | "3")
                                && self.current_room_is_kepler()
                                && !self.the_show =>
                        {
                            let digit = c.as_str().as_bytes()[0].saturating_sub(b'0');
                            if let Some(relation) =
                                numinous_core::rooms::kepler_aha::SpeedRelation::from_key_digit(
                                    digit,
                                )
                            {
                                let _ = self.commit_kepler_call(relation);
                            }
                        }
                        // Parrondo policy call: 1 A, 2 B, 3 ABB.
                        Key::Character(c)
                            if matches!(c.as_str(), "1" | "2" | "3")
                                && self.current_room_is_parrondo()
                                && !self.the_show =>
                        {
                            let digit = c.as_str().as_bytes()[0].saturating_sub(b'0');
                            if let Some(policy) =
                                numinous_core::rooms::parrondo::Policy::from_key_digit(digit)
                            {
                                let _ = self.commit_parrondo_call(policy);
                            }
                        }
                        // Nontransitive counter call: 1 A, 2 B, 3 C.
                        Key::Character(c)
                            if matches!(c.as_str(), "1" | "2" | "3")
                                && self.current_room_is_nontransitive()
                                && !self.the_show =>
                        {
                            let digit = c.as_str().as_bytes()[0].saturating_sub(b'0');
                            if let Some(die) =
                                numinous_core::rooms::nontransitive::Die::from_key_digit(digit)
                            {
                                let _ = self.commit_nontransitive_call(die);
                            }
                        }
                        // R returns this visit to its initial state. Moving to a
                        // different room still deals the next variation.
                        Key::Character(c) if c.as_str() == "r" => {
                            self.reset_current_room();
                        }
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("f") => {
                            self.toggle_fullscreen();
                        }
                        Key::Character(c) if c.as_str() == "h" => {
                            self.open_home_menu();
                        }
                        // G deals the quiz: guess the shape, in the window.
                        Key::Character(c) if c.as_str() == "g" && self.show_help => {
                            self.activate_menu_choice(input_legend::MenuChoice::Quiz);
                        }
                        // C chomps: today's Munch board, in the window.
                        Key::Character(c) if c.as_str() == "c" && self.show_help => {
                            self.activate_menu_choice(input_legend::MenuChoice::Munch);
                        }
                        // N is nim: three heaps against the Order.
                        Key::Character(c) if c.as_str() == "n" && self.show_help => {
                            self.activate_menu_choice(input_legend::MenuChoice::Nim);
                        }
                        // T runs the Gauntlet: four stages, one number.
                        Key::Character(c) if c.as_str() == "t" && self.show_help => {
                            self.activate_menu_choice(input_legend::MenuChoice::Gauntlet);
                        }
                        // V looses the Vexations: the arcade.
                        Key::Character(c) if c.as_str() == "v" && self.show_help => {
                            self.activate_menu_choice(input_legend::MenuChoice::Arcade);
                        }
                        // J opens the journey: what the play has made of you.
                        Key::Character(c) if c.as_str() == "j" => {
                            if self.show_help {
                                self.activate_menu_choice(input_legend::MenuChoice::Journey);
                            } else {
                                self.toggle_journey();
                            }
                        }
                        // X opens the explicitly consented local MCP session viewer.
                        Key::Character(c) if c.as_str() == "x" => {
                            if self.show_help {
                                self.activate_menu_choice(input_legend::MenuChoice::WatchAgent);
                            } else {
                                self.open_session_viewer();
                            }
                        }
                        // Y turns the radio dial: off, then station by station.
                        Key::Character(c) if c.as_str() == "y" && !repeat => {
                            self.cycle_radio();
                        }
                        // N advances the current radio station by one track.
                        Key::Character(c) if c.as_str().eq_ignore_ascii_case("n") => {
                            self.skip_radio_track();
                        }
                        // P keeps the picture: the postcard key.
                        Key::Character(c) if c.as_str() == "p" => {
                            if self.save_gate.admit(
                                save_gate::SaveKind::Postcard,
                                Instant::now(),
                                repeat,
                            ) {
                                self.save_postcard();
                            }
                        }
                        // L keeps the motion: a short looping APNG of this visit.
                        Key::Character(c) if c.as_str() == "l" => {
                            if self.save_gate.admit(
                                save_gate::SaveKind::ShortLoop,
                                Instant::now(),
                                repeat,
                            ) {
                                self.save_short_loop();
                            }
                        }
                        // K packs still + loop + README into one share folder.
                        Key::Character(c) if c.as_str() == "k" => {
                            if self.save_gate.admit(
                                save_gate::SaveKind::ShareBundle,
                                Instant::now(),
                                repeat,
                            ) {
                                self.save_share_bundle();
                            }
                        }
                        // B for the big show (lean back).
                        Key::Character(c) if c.as_str() == "b" => {
                            if self.show_help {
                                self.activate_menu_choice(input_legend::MenuChoice::Show);
                            } else {
                                self.toggle_show();
                            }
                        }
                        // O cycles the visualizer source: room bed, output mix, loopback.
                        Key::Character(c) if c.as_str() == "o" && !repeat => {
                            self.cycle_visualizer_source();
                        }
                        // Number keys are room slots, like weapon slots.
                        Key::Character(c)
                            if c.len() == 1 && c.chars().all(|ch| ch.is_ascii_digit()) =>
                        {
                            let digit = c.chars().next().unwrap_or('1');
                            let slot = if digit == '0' {
                                9
                            } else {
                                (digit as usize - '1' as usize) % 10
                            };
                            if slot < self.rooms.len() {
                                self.current = slot;
                                self.rooms = room_input::redeal_rooms(
                                    &mut self.variation,
                                    &mut self.current,
                                );
                                self.reset_room_runtime();
                                self.tune = Arc::new(Vec::new());
                                if let Some(window) = &self.window {
                                    window.set_title(&self.title());
                                }
                                self.visit_current();
                                self.update_audio();
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let point = self.normalized_mouse_point();
                if self.show_help && self.menu.is_open() {
                    self.clear_pointer_state();
                    let layout = self.menu_layout();
                    let target = point.and_then(|point| layout.item_at(point));
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    match state {
                        ElementState::Pressed => self.menu.pointer_down(target),
                        ElementState::Released => {
                            let intent = self.menu.pointer_up(target);
                            self.apply_menu_intent(intent);
                        }
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                if self.paused {
                    self.clear_pointer_state();
                    return;
                }
                if self.console.is_open() {
                    self.clear_pointer_state();
                    return;
                }
                self.input_mode = input_legend::InputMode::KeyboardMouse;
                match (state, point) {
                    (ElementState::Pressed, Some(point)) => self.begin_pointer_at(point),
                    (ElementState::Released, Some(point)) => self.end_pointer_at(point),
                    (ElementState::Pressed, None) => self.clear_pointer_state(),
                    (ElementState::Released, None) => {
                        self.set_pointer_state(mouse_input::pointer_state_after_left_release());
                    }
                }
            }
            WindowEvent::DroppedFile(path) => {
                self.open_dropped_file(&path);
            }
            WindowEvent::Focused(false) => {
                self.suspend_presentation_clock(Instant::now());
                self.clear_pointer_state();
                self.menu.clear_pointer();
                if let Some(command) = self.gamepad.deactivate() {
                    self.handle_gamepad_command(command);
                }
                if let Some(player) = &self.player {
                    player.set_active(false);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(true) => {
                self.resume_presentation_clock(Instant::now());
                self.gamepad.activate();
                if self.radio.is_some() && !self.studio {
                    let _ = self.sync_radio_to_wall_clock();
                    if let Some(window) = &self.window {
                        window.set_title(&self.title());
                    }
                }
                if let Some(player) = &self.player {
                    player.set_active(true);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
                };
                let _ = self.apply_wheel_delta(lines);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x, position.y);
                if self.show_help && self.menu.is_open() {
                    let layout = self.menu_layout();
                    let hovered = self
                        .normalized_mouse_point()
                        .and_then(|point| layout.item_at(point));
                    let changed = self.menu.pointer_move(hovered);
                    if changed && let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    return;
                }
                self.refresh_pointer_state();
                if self.poking
                    && let Some(window) = &self.window
                {
                    let size = window.inner_size();
                    if let Some(point) = mouse_input::normalized_window_point(
                        (position.x, position.y),
                        (size.width, size.height),
                    ) {
                        // Gestures share the poke trail's decimation, so
                        // legacy rooms see identical hands either way.
                        self.move_pointer_to(point, true);
                    } else {
                        // The window lost its size mid-drag: the gesture
                        // ends without a lift, so close it gently.
                        room_input::cancel_open_gesture(&mut self.inputs, self.t);
                        self.poking = false;
                    }
                } else if self.dragging
                    && let Some(window) = &self.window
                {
                    let w = f64::from(window.inner_size().width.max(1));
                    self.t = (position.x / w).clamp(0.0, 0.999);
                    self.update_audio();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.menu.clear_pointer();
                self.clear_pointer_state();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.quit_requested {
            self.exit_app(event_loop);
            return;
        }
        let now = Instant::now();
        event_loop.set_control_flow(ControlFlow::WaitUntil(now + FRAME_INTERVAL));
        let since_last_tick = now.saturating_duration_since(self.last_tick);
        let elapsed = bounded_tick_seconds(since_last_tick);
        self.last_tick = now;
        if let Some(player) = &self.player {
            player.service();
        }
        if !self.window_active {
            return;
        }
        self.advance_presentation_time(since_last_tick.as_secs_f64());
        let commands = self.gamepad.poll(now);
        for command in commands {
            self.handle_gamepad_command(command);
        }
        self.refresh_pointer_state();
        let first_contact_obscured = self.banner.is_some() && self.room_card > 0;
        let ambient = ambient_tick_seconds(elapsed, self.motion);
        let menu_open = self.show_help && self.menu.is_open();
        if !first_contact_obscured && !menu_open {
            self.advance_life_if_active(ambient);
            self.advance_times_tables_morph(elapsed);
            self.advance_buffon_morph(elapsed);
            self.advance_galton_morph(elapsed);
            self.advance_pendulum_morph(elapsed);
            self.advance_kepler_morph(elapsed);
            self.advance_parrondo_morph(elapsed);
            self.advance_nontransitive_morph(elapsed);
        }
        if !(self.paused || self.dragging || menu_open) {
            let motion = self.time_scale * self.visualizer_scale;
            if !first_contact_obscured && self.rooms[self.current].meta().id == "mandelbrot" {
                self.mandelbrot_camera.advance(ambient * motion);
            }
            let show_active = self.show_mode_active();
            // When the visualizer is driving, mid energy quickens The Show's
            // phase rate so denser mixes move the gallery a little faster.
            let rate = if show_active {
                SHOW_T_RATE * (0.72 + self.visualizer_scale.clamp(0.55, 1.55) * 0.28)
            } else {
                T_RATE
            };
            let (next_phase, wrapped) =
                advance_gallery_phase(self.t, ambient, motion, rate, first_contact_obscured);
            self.t = next_phase;
            if wrapped {
                // In The Show, a finished sweep drifts into the next room.
                if show_active {
                    self.switch(1);
                }
            }
            if show_active {
                // The picture and its mathematical voice share this phase.
                // Updating the smoothed target does not restart the room bed.
                self.sync_room_parameter_voice();
            }
            self.frame += 1;
            room_input::tick_room_card(&mut self.room_card, self.banner.is_some());
            // The arcade's heartbeat: the spirits step on the beat, faster
            // each level; the flash counts itself down.
            if let Some(play) = &mut self.arcade {
                if let Some((_, frames)) = &mut play.flash {
                    *frames -= 1;
                    if *frames == 0 {
                        play.flash = None;
                    }
                }
                let interval = 48u64.saturating_sub(play.run.level * 4).max(16);
                if !play.over && self.frame.is_multiple_of(interval) {
                    self.arcade_beat();
                }
            }
            if let Some(play) = &mut self.munch {
                let _ = play.tick_bite_flash();
            }
            if let Some(run) = &mut self.gauntlet {
                let _ = run.munch.tick_bite_flash();
            }
            if self.studio {
                // Auto advances only after dwell and a phrase-edge of gallery phase.
                if let Some(spec) = self.studio_panel.tick_auto(elapsed, self.t) {
                    self.set_studio_recipe_sound(Some(spec));
                }
            }
            if self.banner.as_mut().is_some_and(|banner| !banner.tick()) {
                self.banner = None;
            }
        }
        // A station is a wall-clock broadcast, independent of room pause or a
        // modal menu. Rejoin the exact live position at every track boundary.
        if self.radio.is_some()
            && !self.studio
            && let Some(until) = self.radio_until
            && Instant::now() >= until
            && !self.radio_paths.is_empty()
            && !self.sync_radio_to_wall_clock()
            && let Some(window) = &self.window
        {
            window.set_title(&self.title());
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[cfg(test)]
struct TestStateRoot {
    path: std::path::PathBuf,
}

#[cfg(test)]
impl TestStateRoot {
    fn new() -> Self {
        use std::hash::{Hash, Hasher};

        let thread = std::thread::current();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        thread.id().hash(&mut hasher);
        thread.name().hash(&mut hasher);
        let path = std::env::temp_dir().join(format!(
            "numinous-app-test-{}-{:016x}",
            std::process::id(),
            hasher.finish()
        ));
        Self::at(path)
    }

    fn at(path: std::path::PathBuf) -> Self {
        match std::fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("cannot clear app test state directory: {error}"),
        }
        std::fs::create_dir_all(&path).expect("app test state directory should be writable");
        Self { path }
    }
}

#[cfg(test)]
impl Drop for TestStateRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
std::thread_local! {
    static TEST_STATE_ROOT: TestStateRoot = TestStateRoot::new();
}

#[cfg(test)]
fn test_state_path(kind: &str) -> std::path::PathBuf {
    TEST_STATE_ROOT.with(|root| root.path.join(format!("{kind}.txt")))
}

fn local_state_paths() -> numinous_core::LocalStatePaths {
    #[cfg(test)]
    {
        numinous_core::LocalStatePaths {
            journey: test_state_path("journey"),
            scores: test_state_path("scores"),
            cairn: test_state_path("cairn"),
            journal: test_state_path("journal"),
            preferences: test_state_path("preferences"),
            radio_cache: test_state_path("radio"),
            protected_radio_source: None,
            crash_log: test_state_path("crash"),
        }
    }
    #[cfg(not(test))]
    {
        numinous_core::resolve_local_state_paths()
    }
}

fn crash_log_path() -> std::path::PathBuf {
    local_state_paths().crash_log
}

fn append_crash_log_at(path: &std::path::Path, entry: &str) -> std::io::Result<()> {
    use std::io::Write as _;

    let _lock = numinous_core::lock_local_state(path)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(entry.as_bytes())
}

/// The journey file: the same one the CLI and MCP level (env-overridable).
#[cfg(test)]
fn journey_path() -> std::path::PathBuf {
    local_state_paths().journey
}

fn app_icon() -> Option<Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "../../../assets/logo.png"
    )));
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut pixels).ok()?;
    let bytes = &pixels[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect(),
        _ => return None,
    };
    Icon::from_rgba(rgba, info.width, info.height).ok()
}

/// The score table, read for the journey overlay's trophy evidence.
#[cfg(test)]
fn scores_path() -> std::path::PathBuf {
    local_state_paths().scores
}

fn main() {
    // The GUI subsystem has no console: a panic would vanish. Every panic
    // writes its message and location to a crash log next to the save files,
    // so any crash report can be triaged from one file.
    std::panic::set_hook(Box::new(|info| {
        let path = crash_log_path();
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let entry = format!(
            "panic at {location}: {info}
"
        );
        let _ = append_crash_log_at(&path, &entry);
    }));
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    // Support --fullscreen / -f / -F and NUMINOUS_FULLSCREEN=1 for launch full screen view.
    // Gives user-requested video options at entry without adding deps.
    let args: Vec<String> = std::env::args().collect();
    let env_full = std::env::var("NUMINOUS_FULLSCREEN")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);
    app.start_fullscreen = args
        .iter()
        .any(|a| a == "--fullscreen" || a == "-f" || a == "-F")
        || env_full;
    // A `.num` path or `numinous://` link opens straight into the Studio,
    // reopened exactly and paused. The capsule file is a front door: opening
    // it should feel like inserting a cart, not importing a document.
    app.start_open = args
        .iter()
        .skip(1)
        .find(|argument| {
            argument.starts_with("numinous://") || argument.to_ascii_lowercase().ends_with(".num")
        })
        .cloned();
    event_loop.run_app(&mut app).expect("run the app");
}

#[cfg(test)]
mod tests;
