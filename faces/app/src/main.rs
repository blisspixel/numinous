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
use numinous_core::{Journey, Raster, Room, Surface, all_rooms_with};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Icon, Window, WindowId};

mod audio_runtime;
mod audio_state;
mod bindings;
mod console;
mod creation_runtime;
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
mod room_runtime;
mod save_gate;
mod session_audio;
mod studio_panel;
mod wager;

use crate::audio_state::Program as AudioProgram;
use crate::creation_runtime::{NamingField, ShareNaming};
use crate::session_audio::SessionAudio;
use numinous_app::{controls, game_draw, input_legend, menu, play, room_phase};
use play::{ArcadePlay, GauntletPlay, MunchPlay, NimPlay, QuizPlay};
use room_phase::{effective_room_phase, has_finite_parameter_input};

#[cfg(test)]
use crate::creation_runtime::ShareIdentity;

/// Frames of The Show crossfade when the gallery advances rooms.
const SHOW_CROSSFADE_FRAMES: u8 = 14;
/// Wall time for the Times Tables cardioid-to-Mandelbrot morph beat.
const TIMES_TABLES_MORPH_SECONDS: f64 = 1.6;
/// Wall time for the Buffon circle-grows-from-sticks morph beat.
const BUFFON_MORPH_SECONDS: f64 = 1.6;
/// Wall time for the Galton curve-settles-over-pile morph beat.
const GALTON_MORPH_SECONDS: f64 = 1.6;
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
/// - The seven engineered aha morphs. Short, bounded, and the direct completion
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
    /// The route the player chose through the catalog, if they chose one.
    ///
    /// None means the whole catalog and the plain arrow step. A route is what
    /// the protocol face calls a door: a smaller, ordered place to be, which
    /// the arrows then stay inside until the player leaves it.
    route: Option<Route>,
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

/// A smaller, ordered place inside the catalog that the arrows stay within.
///
/// The protocol face calls these doors, and offers three: one astonishing room,
/// an ordered walk, and a wander by wing. A wing is a set of catalog indices
/// walked in catalog order. A walk is an authored sequence that carries a
/// question into each room, so its order is its own and not the catalog's.
enum Route {
    /// Wander one wing, in catalog order, wrapping inside it.
    Wing(numinous_core::Wing),
    /// Follow an authored walk, in its own order, carrying its questions.
    Walk {
        walk: &'static numinous_core::RoomWalk,
        step: usize,
    },
}

impl Route {
    /// The room this route opens on.
    fn doorway(&self) -> usize {
        match self {
            Self::Wing(wing) => wing.doorway(),
            Self::Walk { walk, .. } => walk
                .steps
                .first()
                .and_then(|first| numinous_core::catalog_index(first.room_id))
                .unwrap_or(0),
        }
    }

    /// Where a step of `delta` lands, advancing the route's own position.
    ///
    /// A wing walks catalog order. A walk walks its authored order, which is
    /// why it tracks a step rather than reading the player's current room: two
    /// steps of a walk could name the same room, and the catalog position
    /// cannot tell those apart.
    fn step(&mut self, current: usize, delta: isize, total: usize) -> usize {
        match self {
            Self::Wing(wing) => room_input::stepped_within(current, delta, &wing.rooms),
            Self::Walk { walk, step } => {
                let count = walk.steps.len();
                if count == 0 {
                    return current;
                }
                *step = room_input::wrapped_room_index(*step, delta, count);
                walk.steps
                    .get(*step)
                    .and_then(|at| numinous_core::catalog_index(at.room_id))
                    .filter(|index| *index < total)
                    .unwrap_or(current)
            }
        }
    }

    /// The question this route carries into the room it is now on, if any.
    fn question(&self) -> Option<&'static str> {
        match self {
            Self::Wing(_) => None,
            Self::Walk { walk, step } => walk.steps.get(*step).map(|at| at.question),
        }
    }
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
            route: None,
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
            menu::MenuIntent::EnterWing(index) => {
                if let Some(wing) = numinous_core::wings().into_iter().nth(index) {
                    let name = wing.name;
                    let count = wing.len();
                    self.choose_route(Some(Route::Wing(wing)));
                    self.banner = Some(feedback::wing_entered(name, count));
                }
                self.close_menu();
            }
            menu::MenuIntent::EnterWalk => {
                let walk = &numinous_core::STRANGE_LOOP_WALK;
                self.choose_route(Some(Route::Walk { walk, step: 0 }));
                let question = self.route.as_ref().and_then(Route::question);
                self.banner = Some(feedback::walk_entered(
                    walk.title,
                    walk.steps.len(),
                    question,
                ));
                self.close_menu();
            }
            menu::MenuIntent::LeaveWing => {
                self.choose_route(None);
                self.banner = Some(feedback::wing_left());
                self.close_menu();
            }
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
        self.current = match &mut self.route {
            Some(route) => route.step(self.current, delta, self.rooms.len()),
            None => room_input::wrapped_room_index(self.current, delta, self.rooms.len()),
        };
        self.rooms = room_input::redeal_rooms(&mut self.variation, &mut self.current);
        self.reset_room_runtime();
        self.tune = Arc::new(Vec::new());
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.visit_current();
        self.update_audio();
    }

    /// Enter a route, or leave the one we are in.
    ///
    /// Entering lands on the route's first room and keeps the arrows inside it,
    /// which is what makes a door a place to be rather than a label. The whole
    /// catalog is always one step away, because leaving is the same call with
    /// nothing chosen.
    fn choose_route(&mut self, route: Option<Route>) {
        let doorway = route.as_ref().map(Route::doorway);
        self.route = route;
        if let Some(index) = doorway {
            self.goto_room_index(index);
        }
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
