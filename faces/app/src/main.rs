#![windows_subsystem = "windows"]
//! Numinous windowed app.
//!
//! Opens a real window and shows a room animating in full color, rendered on the
//! CPU into a pixel buffer (the same `Raster` the CLI writes to PNG). Left/right
//! switch rooms, space pauses, escape quits. This is the start of the GUI
//! Cabinet (see `docs/DESIGN.md`); it uses `winit` for the window and
//! `softbuffer` for a windowing-toolkit-free pixel blit, so it runs on macOS,
//! Linux, and Windows.

use std::num::NonZeroU32;
use std::rc::Rc;
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
mod feedback;
mod gamepad;
mod hud;
mod input_feedback;
mod live_render;
mod mouse_input;
mod overlays;
mod playtest;
mod postcard;
mod radio_cache;
mod room_input;
mod save_gate;
mod studio_panel;

use crate::audio_state::Program as AudioProgram;
use numinous_app::{controls, game_draw, input_legend, play, room_phase};
use play::{ArcadePlay, GauntletPlay, MunchPlay, NimPlay, QuizPlay, gauntlet_total};
use room_phase::{effective_room_phase, has_finite_parameter_input};

/// Near-black background (matches the `Raster` stage), packed `0x00RRGGBB`.
const BACKGROUND: u32 = 0x000A_0B0F;
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
const T_RATE: f64 = 0.24;
/// The Show advances more slowly for a deliberate, hypnotic pace.
const SHOW_T_RATE: f64 = 0.096;
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

/// The application state driven by the winit event loop.
struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    player: Option<numinous_audio::LoopPlayer>,
    #[cfg(test)]
    transient_audio_clears: std::cell::Cell<usize>,
    #[cfg(test)]
    interaction_audio_events: std::cell::Cell<usize>,
    gamepad: gamepad::GamepadInput,
    /// The last input family that performed a meaningful action.
    input_mode: input_legend::InputMode,
    mandelbrot_camera: numinous_core::rooms::mandelbrot::MandelbrotCamera,
    life_session: numinous_core::rooms::game_of_life::LifeSession,
    life_accumulator: f64,
    rooms: Vec<Box<dyn Room>>,
    current: usize,
    t: f64,
    paused: bool,
    dragging: bool,
    show_info: bool,
    /// The Show: lean back and let the whole collection play itself.
    the_show: bool,
    /// The Studio: type an expression and watch it live.
    studio: bool,
    /// The typed Studio expression and its last-good parse state.
    studio_panel: studio_panel::StudioPanel,
    /// Human-owned, read-only view of one explicitly paired MCP session.
    session_viewer: SessionViewer,
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
    /// The program that owns the player source, independent of focus and gain.
    audio_program: AudioProgram,
    /// The help overlay ('h' toggles; shown at launch so nobody is lost).
    show_help: bool,
    /// Start in fullscreen (from --fullscreen / -f arg or env). Supports user's request for full screen view.
    start_fullscreen: bool,
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
    /// Controller-selected game in the launch menu.
    controller_menu_selection: usize,
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
}

impl App {
    fn new() -> Self {
        let journey_file = journey_path();
        let scores_file = scores_path();
        let journey = numinous_core::load_journey_file(&journey_file);
        Self {
            window: None,
            surface: None,
            player: None,
            #[cfg(test)]
            transient_audio_clears: std::cell::Cell::new(0),
            #[cfg(test)]
            interaction_audio_events: std::cell::Cell::new(0),
            gamepad: gamepad::GamepadInput::new(),
            input_mode: input_legend::InputMode::default(),
            mandelbrot_camera: numinous_core::rooms::mandelbrot::MandelbrotCamera::new(0),
            life_session: numinous_core::rooms::game_of_life::LifeSession::new(0),
            life_accumulator: 0.0,
            rooms: all_rooms_with(0),
            current: 0,
            t: 0.0,
            paused: false,
            dragging: false,
            show_info: false,
            the_show: false,
            studio: false,
            studio_panel: studio_panel::StudioPanel::default(),
            session_viewer: SessionViewer::default(),
            gpu: None,
            live_scale: live_render::LiveScale::new(),
            era: numinous_core::Era::default(),
            muted: false,
            volume: 0.45,
            audio_program: AudioProgram::RoomScore,
            show_help: true,
            start_fullscreen: false,
            frame: 0,
            last_tick: Instant::now(),
            window_active: true,
            inactive_since: None,
            time_scale: 1.0,
            journey: journey.clone(),
            journey_saved: journey,
            level_seen: 1,
            banner: None,
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
            controller_menu_selection: 0,
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
        }
    }

    /// Persist the journey and raise the Journey banner when the level moves.
    fn journey_changed(&mut self) {
        if let Ok(saved) = numinous_core::persist_journey_delta(
            &self.journey_file,
            &self.journey_saved,
            &self.journey,
        ) {
            self.journey = saved.clone();
            self.journey_saved = saved;
        }
        let level = self.journey.level();
        if level > self.level_seen {
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
    fn quiz_next(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        let room_ids = self.rooms.iter().map(|room| room.meta().id);
        let quiz = play::deal_quiz(seed, self.journey.plays, room_ids, &mut self.quiz_recent);
        self.journey.play();
        self.journey_changed();
        self.quiz = Some(quiz);
        self.sync_room_parameter_voice();
    }

    /// Answer the quiz with a letter; right or wrong, the reveal follows.
    fn quiz_answer(&mut self, letter: char) {
        if self
            .quiz
            .as_mut()
            .and_then(|quiz| play::answer_quiz(quiz, letter))
            == Some(true)
        {
            self.journey.win();
            self.journey_changed();
        }
    }

    /// Post a score to the shared table (the CLI's file and rules).
    fn post_score(&self, key: &str, score: i64) -> bool {
        numinous_core::record_score_file(&self.scores_file, key, score).unwrap_or(false)
    }

    /// Deal a Munch board (today's).
    fn munch_start(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        self.journey.play();
        self.journey_changed();
        let (round, board) = play::deal_munch(seed, self.munch_next_round, self.munch_last_rule);
        self.munch_next_round = round.saturating_add(1);
        self.munch_last_rule = Some(board.rule);
        self.munch = Some(MunchPlay {
            board,
            seed,
            round,
            cursor: 0,
            bites: std::collections::BTreeSet::new(),
            graded: None,
            bite_flash: None,
        });
        self.sync_room_parameter_voice();
    }

    /// Grade the Munch board: the dense feedback, the score, the record.
    fn munch_grade(&mut self) {
        let Some(play) = self.munch.as_mut() else {
            return;
        };
        if play.graded.is_some() {
            return;
        }
        let bites: Vec<usize> = play.bites.iter().copied().collect();
        let outcome = numinous_core::grade_munch(&play.board, &bites);
        let clean = outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
        let (seed, round, score) = (play.seed, play.round, outcome.score);
        play.graded = Some(outcome);
        self.post_score(&format!("munch seed:{seed} board:{round}"), score);
        if clean {
            self.journey.win();
        }
        self.journey_changed();
    }

    /// Deal a Nim game (today's heaps).
    fn nim_start(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        self.journey.play();
        self.journey_changed();
        let heaps = numinous_core::nim_new(seed);
        self.nim = Some(NimPlay {
            selected: heaps.iter().position(|&h| h > 0).unwrap_or(0),
            take: 1,
            heaps,
            seed,
            message: String::from("THE ORDER PLAYS A SECRET. BEAT IT AND IT IS YOURS."),
            over: None,
        });
        self.sync_room_parameter_voice();
    }

    /// Commit the aimed Nim move; the Order answers at once.
    fn nim_move(&mut self) {
        let Some(play) = self.nim.as_mut() else {
            return;
        };
        if play.over.is_some() {
            return;
        }
        if !numinous_core::nim_apply(&mut play.heaps, play.selected, play.take) {
            play.message = String::from("THAT MOVE IS NOT ON THE BOARD.");
            return;
        }
        if numinous_core::nim_finished(&play.heaps) {
            play.over = Some(true);
            let seed = play.seed;
            self.journey.win();
            self.journey_changed();
            self.post_score(&format!("nim seed:{seed}"), 1);
            return;
        }
        let (heap, take) = numinous_core::nim_order(&play.heaps);
        let _ = numinous_core::nim_apply(&mut play.heaps, heap, take);
        if numinous_core::nim_finished(&play.heaps) {
            play.over = Some(false);
            play.message = String::from("THE ORDER TAKES THE LAST STONE. AGAIN. (NOT LUCK.)");
            return;
        }
        play.message = format!("THE ORDER TAKES {take} FROM HEAP {}.", heap + 1);
        if play.heaps.get(play.selected).copied().unwrap_or(0) == 0 {
            play.selected = play.heaps.iter().position(|&h| h > 0).unwrap_or(0);
        }
        play.take = play.take.min(play.heaps[play.selected].max(1));
    }

    /// Start the arcade: today's run, spirits loose, the beat ticking.
    fn arcade_start(&mut self) {
        self.the_show = false;
        // Clear any stale pause from the wander view: the arcade is real-time, and
        // a leaked pause would freeze the Vexations while the player kept eating,
        // then post an unfairly-earned score to the shared table.
        self.paused = false;
        let seed = play::daily_seed();
        self.journey.play();
        self.journey_changed();
        self.arcade = Some(ArcadePlay {
            run: numinous_core::munch_arcade::Arcade::new(seed),
            seed,
            flash: None,
            over: false,
        });
        self.sync_room_parameter_voice();
    }

    /// One player action into the live arcade.
    fn arcade_act(&mut self, action: numinous_core::munch_arcade::Action) {
        use numinous_core::munch_arcade::Turn;
        let Some(play) = self.arcade.as_mut() else {
            return;
        };
        if play.over {
            return;
        }
        match play.run.act(action) {
            Turn::Cleared => {
                play.flash = Some((false, 40));
                self.journey.win();
                self.journey_changed();
            }
            Turn::Over => play.over = true,
            _ => {}
        }
    }

    /// The spirits' beat: called from the frame clock.
    fn arcade_beat(&mut self) {
        use numinous_core::munch_arcade::Turn;
        let Some(play) = self.arcade.as_mut() else {
            return;
        };
        if play.over {
            return;
        }
        match play.run.tick() {
            Turn::Caught => play.flash = Some((true, 40)),
            Turn::Over => {
                play.over = true;
                let (seed, score) = (play.seed, play.run.score);
                self.post_score(&format!("arcade seed:{seed}"), score);
            }
            _ => {}
        }
    }

    /// Start the Gauntlet: today's run, four stages, a combo.
    fn gauntlet_start(&mut self) {
        self.the_show = false;
        self.paused = false;
        let seed = play::daily_seed();
        self.gauntlet = Some(GauntletPlay {
            seed,
            stage: 0,
            munch: MunchPlay {
                board: numinous_core::build_board(seed, 0),
                seed,
                round: 0,
                cursor: 0,
                bites: std::collections::BTreeSet::new(),
                graded: None,
                bite_flash: None,
            },
            quiz: QuizPlay {
                round: numinous_core::build_round(seed, 1, 44, 18),
                flash: None,
            },
            scan: numinous_core::build_scan(seed, 4),
            secret: numinous_core::secret_code(seed ^ 0x0000_6A17_0000_0B0B, 4),
            wire: String::new(),
            wire_lines: Vec::new(),
            scores: Vec::new(),
            cleared: Vec::new(),
            message: String::from("STAGE 1 OF 4  MUNCH. CLEAN STAGES BUILD YOUR COMBO."),
        });
        self.sync_room_parameter_voice();
    }

    /// Bank a gauntlet stage: score, clean flag, journey, and the narration.
    fn gauntlet_bank(&mut self, points: i64, clean: bool, what: &str) {
        self.journey.play();
        if clean {
            self.journey.win();
        }
        self.journey_changed();
        let Some(run) = self.gauntlet.as_mut() else {
            return;
        };
        run.scores.push(points);
        run.cleared.push(clean);
        run.stage += 1;
        let combo = run.cleared.iter().take_while(|&&c| c).count() + 1;
        run.message = if run.stage < 4 {
            format!(
                "{what}  STAGE {} OF 4{}",
                run.stage + 1,
                if clean {
                    format!("  COMBO X{combo}")
                } else {
                    String::new()
                }
            )
        } else {
            what.to_string()
        };
        if run.stage == 4 {
            let total = gauntlet_total(&run.scores, &run.cleared);
            let seed = run.seed;
            self.post_score(&format!("gauntlet seed:{seed}"), total);
        }
    }

    /// One key into the Gauntlet: routed to whichever stage is live.
    fn gauntlet_key(&mut self, key: &Key) {
        if matches!(key, Key::Named(NamedKey::Escape)) {
            self.gauntlet = None;
            self.clear_transient_audio();
            self.update_audio();
            return;
        }
        let Some(run) = self.gauntlet.as_mut() else {
            return;
        };
        match run.stage {
            0 => {
                let play = &mut run.munch;
                match key {
                    Key::Named(NamedKey::Enter) => {
                        let bites: Vec<usize> = play.bites.iter().copied().collect();
                        let outcome = numinous_core::grade_munch(&play.board, &bites);
                        let clean =
                            outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
                        let (points, what) = (outcome.score, format!("MUNCH +{}.", outcome.score));
                        self.gauntlet_bank(points, clean, &what);
                    }
                    key => {
                        if let Some(cell) =
                            controls::apply_munch_control(&mut play.cursor, &mut play.bites, key)
                        {
                            play.flash_bite(cell);
                            self.play_munch_crunch(cell as u64 ^ 0x6A17);
                        }
                    }
                }
            }
            1 => {
                if let Key::Character(c) = key
                    && c.len() == 1
                {
                    let letter = c.chars().next().unwrap_or(' ').to_ascii_uppercase();
                    if let Some(correct) = play::answer_quiz(&mut run.quiz, letter) {
                        let what = format!(
                            "IT WAS {} ({}).",
                            run.quiz.round.answer,
                            run.quiz.round.answer_title.to_uppercase()
                        );
                        self.gauntlet_bank(if correct { 25 } else { 0 }, correct, &what);
                    }
                }
            }
            2 => {
                if let Key::Character(c) = key
                    && c.len() == 1
                {
                    let letter = c.chars().next().unwrap_or(' ').to_ascii_uppercase();
                    if run.scan.channels.iter().any(|ch| ch.letter == letter) {
                        let correct = letter == run.scan.answer;
                        let what = format!("THE SIGNAL WAS {}.", run.scan.answer);
                        self.gauntlet_bank(if correct { 25 } else { 0 }, correct, &what);
                    }
                }
            }
            3 => match key {
                Key::Named(NamedKey::Backspace) => {
                    run.wire.pop();
                }
                Key::Named(NamedKey::Enter) => {
                    let guess: Vec<u8> = run
                        .wire
                        .chars()
                        .filter(char::is_ascii_digit)
                        .map(|c| c as u8 - b'0')
                        .collect();
                    if guess.len() != 4 {
                        return;
                    }
                    let feedback = numinous_core::grade(&run.secret, &guess);
                    if feedback.locked == 4 {
                        let spare = 4 - run.wire_lines.len() as i64;
                        self.gauntlet_bank(10 * spare.max(0), true, "DEFUSED.");
                        return;
                    }
                    run.wire_lines.push(format!(
                        "{}: {} LOCKED, {} LOOSE",
                        run.wire, feedback.locked, feedback.loose
                    ));
                    run.wire.clear();
                    if run.wire_lines.len() >= 5 {
                        let code: String =
                            run.secret.iter().map(|&d| char::from(b'0' + d)).collect();
                        self.gauntlet_bank(0, false, &format!("BOOM. IT WAS {code}."));
                    }
                }
                Key::Character(c) if run.wire.len() < 4 => {
                    for ch in c.chars().filter(char::is_ascii_digit) {
                        if run.wire.len() < 4 {
                            run.wire.push(ch);
                        }
                    }
                }
                _ => {}
            },
            _ => {
                self.gauntlet = None;
                self.clear_transient_audio();
                self.update_audio();
            }
        }
    }

    /// One key into standalone Munch.
    fn munch_key(&mut self, key: &Key) {
        let graded = self
            .munch
            .as_ref()
            .is_some_and(|play| play.graded.is_some());
        if graded {
            match key {
                Key::Named(NamedKey::Escape) => {
                    self.munch = None;
                    self.clear_transient_audio();
                    self.update_audio();
                }
                Key::Named(NamedKey::Enter | NamedKey::Space) => self.munch_start(),
                _ => {}
            }
            return;
        }
        match key {
            Key::Named(NamedKey::Escape) => {
                self.munch = None;
                self.clear_transient_audio();
                self.update_audio();
            }
            Key::Named(NamedKey::Enter) => self.munch_grade(),
            key => {
                if let Some(play) = &mut self.munch
                    && let Some(cell) =
                        controls::apply_munch_control(&mut play.cursor, &mut play.bites, key)
                {
                    play.flash_bite(cell);
                    self.play_munch_crunch(cell as u64);
                }
            }
        }
    }

    /// Soft one-shot noise tick over the room score (Munch bite juice).
    fn play_munch_crunch(&self, seed: u64) {
        let Some(player) = &self.player else {
            return;
        };
        if self.muted {
            return;
        }
        let samples = numinous_core::munch_crunch(player.sample_rate(), seed);
        player.play_oneshot(samples, 0.55 * self.volume);
    }

    fn clear_transient_audio(&self) {
        #[cfg(test)]
        self.transient_audio_clears
            .set(self.transient_audio_clears.get().saturating_add(1));
        if let Some(player) = &self.player {
            player.clear_oneshot();
        }
    }

    /// One key into standalone Nim, including an explicit retry after either
    /// result so a loss can teach without ejecting the player.
    fn nim_key(&mut self, key: &Key) {
        let over = self.nim.as_ref().is_some_and(|play| play.over.is_some());
        if over {
            match key {
                Key::Named(NamedKey::Escape) => {
                    self.nim = None;
                    self.update_audio();
                }
                Key::Named(NamedKey::Enter | NamedKey::Space) => self.nim_start(),
                _ => {}
            }
            return;
        }
        match key {
            Key::Named(NamedKey::Escape) => {
                self.nim = None;
                self.update_audio();
            }
            Key::Named(NamedKey::Enter) => self.nim_move(),
            key => {
                if let Some(play) = &mut self.nim {
                    let _ = controls::apply_nim_control(play, key);
                }
            }
        }
    }

    /// Write the current room's frame to a PNG next to the save files: the
    /// postcard key. Returns the path it wrote.
    fn save_postcard(&self) -> Option<std::path::PathBuf> {
        self.save_postcard_to(&postcard::default_postcard_dir())
            .ok()
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
    fn save_short_loop(&self) -> Option<std::path::PathBuf> {
        self.save_short_loop_to(&postcard::default_postcard_dir())
            .ok()
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
        };
        let report = playtest::build_report(&snapshot, now);
        playtest::write_report(dir, now, &report)
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
        self.the_show = false;
        self.paused = false;
        self.show_help = false;
        self.show_journey = false;
        self.studio = true;
        self.audio_program = AudioProgram::Studio;
        if let Some(player) = &self.player {
            player.clear_oneshot();
        }
        self.studio_reparse();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    fn open_session_viewer(&mut self) {
        self.the_show = false;
        self.paused = false;
        self.show_help = false;
        self.show_journey = false;
        self.banner = None;
        match self.session_viewer.open() {
            Ok(()) => self.sync_room_parameter_voice(),
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
        self.sync_room_parameter_voice();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    fn toggle_show(&mut self) {
        self.the_show = !self.the_show;
        if self.the_show {
            self.show_help = false;
            self.show_journey = false;
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
        self.show_help = false;
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

    fn help_menu_selection(&self) -> Option<usize> {
        (self.input_mode == input_legend::InputMode::Controller && !self.modal_mode_active())
            .then_some(self.controller_menu_selection)
    }

    fn handle_modal_help_key(&mut self, key: &Key) -> bool {
        if !(self.show_help && self.modal_mode_active()) {
            return false;
        }
        if key == &Key::Named(NamedKey::Escape) {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            self.show_help = false;
        }
        true
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

    fn change_volume(&mut self, step: f32) {
        self.volume = (self.volume + step).clamp(0.0, 1.0);
        self.banner = Some(feedback::volume(self.volume, self.muted));
        self.apply_master_gain();
    }

    fn apply_master_gain(&self) {
        if let Some(player) = &self.player {
            player.set_master_gain(if self.muted { 0.0 } else { self.volume });
        }
    }

    fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.apply_master_gain();
    }

    fn handle_global_audio_key(&mut self, key: &Key, repeat: bool) -> bool {
        let Key::Character(text) = key else {
            return false;
        };
        if text.eq_ignore_ascii_case("m") {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            if !repeat {
                self.toggle_mute();
            }
            return true;
        }
        let step = match text.as_str() {
            "[" => Some(-0.1),
            "]" => Some(0.1),
            "-" if !self.studio => Some(-0.1),
            "=" if !self.studio => Some(0.1),
            _ => None,
        };
        if let Some(step) = step {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            self.change_volume(step);
            return true;
        }
        false
    }

    /// One step from the Muncher toward a clicked board cell.
    fn arcade_step_toward(from: usize, to: usize) -> Option<numinous_core::munch_arcade::Action> {
        let cols = numinous_core::munchers::COLS;
        let (fr, fc) = (from / cols, from % cols);
        let (tr, tc) = (to / cols, to % cols);
        if tr < fr {
            Some(numinous_core::munch_arcade::Action::Up)
        } else if tr > fr {
            Some(numinous_core::munch_arcade::Action::Down)
        } else if tc < fc {
            Some(numinous_core::munch_arcade::Action::Left)
        } else if tc > fc {
            Some(numinous_core::munch_arcade::Action::Right)
        } else {
            None
        }
    }

    /// A click lands in the games: cells, heaps, choices, and stages all answer.
    fn click(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        let (width, height) = (size.width as usize, size.height as usize);
        if width == 0 || height == 0 {
            return;
        }
        let (mx, my) = self.mouse;
        if let Some(play) = &mut self.munch {
            if play.graded.is_some() {
                return;
            }
            if let Some(cell) = game_draw::MunchLayout::new(width, height).hit(mx, my) {
                play.cursor = cell;
                controls::toggle_munch_bite(&mut play.bites, cell);
                play.flash_bite(cell);
                self.play_munch_crunch(cell as u64);
            }
            return;
        }
        if let Some(quiz) = &self.quiz {
            if quiz.flash.is_some() {
                self.quiz_next();
                return;
            }
            let layout = game_draw::QuizChoiceLayout::new(width, height, quiz.round.choices.len());
            if let Some(index) = layout.hit(my, quiz.round.choices.len())
                && let Some(choice) = quiz.round.choices.get(index)
            {
                let letter = choice.letter;
                self.quiz_answer(letter);
            }
            return;
        }
        if self.nim.as_ref().is_some_and(|play| play.over.is_none()) {
            let heaps = self
                .nim
                .as_ref()
                .map(|play| play.heaps.clone())
                .unwrap_or_default();
            if let Some((heap, take)) = game_draw::NimLayout::new(width, height).hit(mx, my, &heaps)
            {
                if let Some(play) = self.nim.as_mut() {
                    play.selected = heap;
                    let max_take = play.heaps.get(heap).copied().unwrap_or(1).max(1);
                    play.take = take.max(1).min(max_take);
                }
                // A click that names both heap and stones commits the move.
                self.nim_move();
            }
            return;
        }
        if let Some(play) = &mut self.arcade {
            if play.over {
                return;
            }
            if let Some(cell) = game_draw::MunchLayout::new(width, height).hit(mx, my) {
                let muncher = play.run.muncher;
                if cell == muncher {
                    self.arcade_act(numinous_core::munch_arcade::Action::Eat);
                } else if let Some(action) = Self::arcade_step_toward(muncher, cell) {
                    self.arcade_act(action);
                }
            }
            return;
        }
        if let Some(run) = &self.gauntlet {
            match run.stage {
                0 => {
                    if run.munch.graded.is_some() {
                        return;
                    }
                    if let Some(cell) = game_draw::MunchLayout::new(width, height).hit(mx, my) {
                        if let Some(run) = self.gauntlet.as_mut() {
                            run.munch.cursor = cell;
                            controls::toggle_munch_bite(&mut run.munch.bites, cell);
                            run.munch.flash_bite(cell);
                        }
                        self.play_munch_crunch(cell as u64 ^ 0x6A17);
                    }
                }
                1 => {
                    if run.quiz.flash.is_some() {
                        return;
                    }
                    let choices = run.quiz.round.choices.len();
                    let layout = game_draw::QuizChoiceLayout::new(width, height, choices);
                    if let Some(index) = layout.hit(my, choices)
                        && let Some(letter) = self
                            .gauntlet
                            .as_ref()
                            .and_then(|g| g.quiz.round.choices.get(index).map(|c| c.letter))
                    {
                        self.gauntlet_key(&Key::Character(letter.to_string().into()));
                    }
                }
                _ => {}
            }
        }
    }

    fn set_mouse_from_normalized(&mut self, point: (f64, f64)) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        self.mouse = (
            point.0.clamp(0.0, 1.0) * f64::from(size.width),
            point.1.clamp(0.0, 1.0) * f64::from(size.height),
        );
    }

    fn begin_pointer_at(&mut self, point: (f64, f64)) {
        if self.paused {
            return;
        }
        self.set_mouse_from_normalized(point);
        let action = mouse_input::left_press_action(self.left_press_context());
        self.set_pointer_state(mouse_input::pointer_state_after_left_press(action));
        match action {
            mouse_input::LeftPressAction::GameClick => self.click(),
            mouse_input::LeftPressAction::RoomPoke => {
                self.poking = true;
                self.record_room_touch(point);
                if self.rooms[self.current].meta().id == "mandelbrot"
                    && let Some(window) = &self.window
                {
                    let size = window.inner_size();
                    let _ = self.mandelbrot_camera.dive(
                        point.0,
                        point.1,
                        size.width as usize,
                        size.height as usize,
                    );
                }
            }
            mouse_input::LeftPressAction::PhaseDrag | mouse_input::LeftPressAction::Ignore => {}
        }
    }

    fn move_pointer_to(&mut self, point: (f64, f64), held: bool) {
        if self.paused {
            return;
        }
        self.set_mouse_from_normalized(point);
        if held && self.poking && room_input::extend_poke_trail(&mut self.pokes, point) {
            let accepted = room_input::record_pointer_move(&mut self.inputs, point, self.t);
            self.maybe_announce_room_goal();
            self.sync_room_parameter_voice();
            self.play_room_interaction_audio(accepted);
        }
    }

    fn end_pointer_at(&mut self, point: (f64, f64)) {
        self.set_mouse_from_normalized(point);
        let accepted =
            self.poking && room_input::record_pointer_up(&mut self.inputs, point, self.t);
        self.set_pointer_state(mouse_input::pointer_state_after_left_release());
        self.maybe_announce_room_goal();
        self.sync_room_parameter_voice();
        self.play_room_interaction_audio(accepted);
    }

    fn apply_wheel_delta(&mut self, lines: f64) -> bool {
        if self.studio
            || self.paused
            || self.show_help && self.modal_mode_active()
            || lines == 0.0
            || !lines.is_finite()
        {
            return false;
        }
        self.input_mode = input_legend::InputMode::KeyboardMouse;
        if self.current_room_is_life() {
            self.time_scale = if lines.is_sign_positive() {
                (self.time_scale * 2.0).min(8.0)
            } else {
                (self.time_scale / 2.0).max(0.25)
            };
            return true;
        }
        self.t = (self.t + lines * 0.02).rem_euclid(1.0);
        self.update_audio();
        true
    }

    fn gamepad_direction(&mut self, command: gamepad::Command) {
        if self.show_help && self.modal_mode_active() {
            return;
        }
        if self.show_help {
            let count = input_legend::MenuChoice::ALL.len();
            self.controller_menu_selection = match command {
                gamepad::Command::Up | gamepad::Command::Left => {
                    (self.controller_menu_selection + count - 1) % count
                }
                gamepad::Command::Down | gamepad::Command::Right => {
                    (self.controller_menu_selection + 1) % count
                }
                _ => return,
            };
            return;
        }
        if self.studio || self.show_journey || self.the_show {
            return;
        }
        let key = match command {
            gamepad::Command::Up => Key::Named(NamedKey::ArrowUp),
            gamepad::Command::Down => Key::Named(NamedKey::ArrowDown),
            gamepad::Command::Left => Key::Named(NamedKey::ArrowLeft),
            gamepad::Command::Right => Key::Named(NamedKey::ArrowRight),
            _ => return,
        };
        if let Some(play) = &mut self.arcade {
            if let Some(action) = controls::arcade_action_for_key(&key)
                && !play.over
            {
                self.arcade_act(action);
            }
        } else if let Some(stage) = self.gauntlet.as_ref().map(|run| run.stage) {
            match stage {
                1 | 2 => {
                    let letter = match command {
                        gamepad::Command::Up => 'A',
                        gamepad::Command::Right => 'B',
                        gamepad::Command::Down => 'C',
                        gamepad::Command::Left => 'D',
                        _ => return,
                    };
                    self.gauntlet_key(&Key::Character(letter.to_string().into()));
                }
                3 => match command {
                    gamepad::Command::Up => {
                        self.controller_digit = (self.controller_digit + 1) % 10;
                        if let Some(run) = &mut self.gauntlet {
                            run.message = format!(
                                "SELECTED DIGIT {}. SOUTH ADDS, NORTH SUBMITS.",
                                self.controller_digit
                            );
                        }
                    }
                    gamepad::Command::Down => {
                        self.controller_digit = (self.controller_digit + 9) % 10;
                        if let Some(run) = &mut self.gauntlet {
                            run.message = format!(
                                "SELECTED DIGIT {}. SOUTH ADDS, NORTH SUBMITS.",
                                self.controller_digit
                            );
                        }
                    }
                    gamepad::Command::Left => {
                        self.gauntlet_key(&Key::Named(NamedKey::Backspace));
                    }
                    gamepad::Command::Right => self.gamepad_primary(),
                    _ => {}
                },
                _ => self.gauntlet_key(&key),
            }
        } else if self.munch.is_some() {
            self.munch_key(&key);
        } else if self.nim.is_some() {
            self.nim_key(&key);
        } else if self.quiz.is_some() {
            let letter = match command {
                gamepad::Command::Up => 'A',
                gamepad::Command::Right => 'B',
                gamepad::Command::Down => 'C',
                gamepad::Command::Left => 'D',
                _ => return,
            };
            self.quiz_answer(letter);
        } else {
            match command {
                gamepad::Command::Left => self.switch(-1),
                gamepad::Command::Right => self.switch(1),
                gamepad::Command::Up => self.time_scale = (self.time_scale * 2.0).min(8.0),
                gamepad::Command::Down => self.time_scale = (self.time_scale / 2.0).max(0.25),
                _ => {}
            }
        }
    }

    fn gamepad_primary(&mut self) {
        if self.show_help && self.modal_mode_active() {
            self.show_help = false;
        } else if self.show_help {
            self.show_help = false;
            match input_legend::MenuChoice::at(self.controller_menu_selection) {
                input_legend::MenuChoice::Quiz => self.quiz_next(),
                input_legend::MenuChoice::Munch => self.munch_start(),
                input_legend::MenuChoice::Nim => self.nim_start(),
                input_legend::MenuChoice::Gauntlet => self.gauntlet_start(),
                input_legend::MenuChoice::Arcade => self.arcade_start(),
                input_legend::MenuChoice::Show => self.toggle_show(),
                input_legend::MenuChoice::Studio => self.enter_studio(),
                input_legend::MenuChoice::Journey => self.toggle_journey(),
                input_legend::MenuChoice::WatchAgent => self.open_session_viewer(),
            }
        } else if let Some(over) = self.arcade.as_ref().map(|play| play.over) {
            if over {
                self.arcade = None;
                self.update_audio();
            } else {
                self.arcade_act(numinous_core::munch_arcade::Action::Eat);
            }
        } else if self.gauntlet.as_ref().is_some_and(|run| run.stage == 3) {
            self.gauntlet_key(&Key::Character(
                char::from(b'0' + self.controller_digit).to_string().into(),
            ));
        } else if self.gauntlet.is_some() {
            self.gauntlet_key(&Key::Named(NamedKey::Space));
        } else if self.munch.is_some() {
            self.munch_key(&Key::Named(NamedKey::Space));
        } else if self.nim.is_some() {
            self.nim_key(&Key::Named(NamedKey::Enter));
        } else if self.quiz.as_ref().is_some_and(|quiz| quiz.flash.is_some()) {
            self.quiz_next();
        } else if self.quiz.is_some() {
            self.quiz_answer('A');
        } else if let Some(point) = self.gamepad.cursor() {
            self.begin_pointer_at(point);
        }
    }

    fn gamepad_back(&mut self) {
        if self.show_help {
            self.show_help = false;
        } else if self.the_show {
            self.toggle_show();
        } else if self.show_journey {
            self.show_journey = false;
        } else if self.arcade.is_some() {
            if let Some(play) = self.arcade.take() {
                self.post_score(&format!("arcade seed:{}", play.seed), play.run.score);
            }
            self.update_audio();
        } else if self.gauntlet.is_some() {
            self.gauntlet_key(&Key::Named(NamedKey::Escape));
        } else if self.munch.is_some() {
            self.munch_key(&Key::Named(NamedKey::Escape));
        } else if self.nim.is_some() {
            self.nim_key(&Key::Named(NamedKey::Escape));
        } else if self.quiz.is_some() {
            self.quiz = None;
            self.update_audio();
        } else if self.studio {
            self.exit_studio();
        } else {
            self.show_help = !self.show_help;
        }
    }

    fn gamepad_menu(&mut self) {
        self.clear_pointer_state();
        if self.the_show {
            self.toggle_show();
        }
        self.show_journey = false;
        self.show_help = !self.show_help;
    }

    fn gamepad_confirm_secondary(&mut self) {
        if self.arcade.is_some()
            || self.quiz.is_some()
            || self.studio
            || self.show_help && self.modal_mode_active()
        {
            return;
        }
        if self.gauntlet.is_some() {
            self.gauntlet_key(&Key::Named(NamedKey::Enter));
        } else if self.munch.is_some() {
            self.munch_key(&Key::Named(NamedKey::Enter));
        } else if self.nim.is_some() {
            self.nim_key(&Key::Named(NamedKey::Enter));
        } else {
            let stations = numinous_core::STATIONS.len();
            self.radio = match self.radio {
                None => Some(0),
                Some(i) if i + 1 < stations => Some(i + 1),
                Some(_) => None,
            };
            self.tune_in();
        }
    }

    fn handle_gamepad_command(&mut self, command: gamepad::Command) {
        match command {
            gamepad::Command::ToggleMute => {
                self.input_mode = input_legend::InputMode::Controller;
                self.toggle_mute();
                return;
            }
            gamepad::Command::VolumeDown => {
                self.input_mode = input_legend::InputMode::Controller;
                self.change_volume(-0.1);
                return;
            }
            gamepad::Command::VolumeUp => {
                self.input_mode = input_legend::InputMode::Controller;
                self.change_volume(0.1);
                return;
            }
            _ => {}
        }
        if self.session_viewer.is_open() {
            if command != gamepad::Command::CancelPointer {
                self.input_mode = input_legend::InputMode::Controller;
            }
            match command {
                gamepad::Command::Back => self.close_session_viewer(),
                gamepad::Command::Menu => {
                    self.close_session_viewer();
                    self.show_help = true;
                }
                gamepad::Command::Pause => self.session_viewer.toggle_display_pause(),
                gamepad::Command::Left => self.session_viewer.scrub(-1),
                gamepad::Command::Right => self.session_viewer.scrub(1),
                gamepad::Command::Up => self.session_viewer.scroll_result(-1),
                gamepad::Command::Down => self.session_viewer.scroll_result(1),
                gamepad::Command::PreviousRoom => self.session_viewer.pan_result(-4),
                gamepad::Command::NextRoom => self.session_viewer.pan_result(4),
                _ => {}
            }
            return;
        }
        if self.paused
            && !matches!(
                command,
                gamepad::Command::Pause
                    | gamepad::Command::PrimaryUp
                    | gamepad::Command::CancelPointer
            )
        {
            return;
        }
        if self.show_help
            && self.modal_mode_active()
            && !matches!(
                command,
                gamepad::Command::PrimaryDown
                    | gamepad::Command::PrimaryUp
                    | gamepad::Command::Back
                    | gamepad::Command::Menu
                    | gamepad::Command::CancelPointer
            )
        {
            return;
        }
        if command != gamepad::Command::CancelPointer {
            self.input_mode = input_legend::InputMode::Controller;
        }
        match command {
            gamepad::Command::PrimaryDown => self.gamepad_primary(),
            gamepad::Command::PrimaryUp => {
                if let Some(point) = self.gamepad.cursor() {
                    self.end_pointer_at(point);
                }
            }
            gamepad::Command::Back => self.gamepad_back(),
            gamepad::Command::Menu => self.gamepad_menu(),
            gamepad::Command::Inspect => self.show_info = !self.show_info,
            gamepad::Command::Reset => self.reset_current_room(),
            gamepad::Command::PreviousRoom if !self.modal_mode_active() => self.switch(-1),
            gamepad::Command::NextRoom if !self.modal_mode_active() => self.switch(1),
            gamepad::Command::Slower => {
                self.time_scale = (self.time_scale / 2.0).max(0.25);
            }
            gamepad::Command::Faster => {
                self.time_scale = (self.time_scale * 2.0).min(8.0);
            }
            gamepad::Command::Up
            | gamepad::Command::Down
            | gamepad::Command::Left
            | gamepad::Command::Right => self.gamepad_direction(command),
            gamepad::Command::CycleEra => self.era = self.era.next(),
            gamepad::Command::CycleRadio => self.gamepad_confirm_secondary(),
            gamepad::Command::Pause => self.toggle_pause(),
            gamepad::Command::PointerMoved { point, held } => {
                self.move_pointer_to(point, held);
            }
            gamepad::Command::PhaseDelta(delta)
                if !self.modal_mode_active() && self.current_room_is_life() =>
            {
                self.time_scale = (self.time_scale * 2.0_f64.powf(delta * 4.0)).clamp(0.25, 8.0);
            }
            gamepad::Command::PhaseDelta(delta) if !self.modal_mode_active() => {
                self.t = (self.t + delta).rem_euclid(1.0);
                self.sync_room_parameter_voice();
            }
            gamepad::Command::CancelPointer => self.clear_pointer_state(),
            gamepad::Command::ToggleMute
            | gamepad::Command::VolumeDown
            | gamepad::Command::VolumeUp => {}
            gamepad::Command::PreviousRoom
            | gamepad::Command::NextRoom
            | gamepad::Command::PhaseDelta(_) => {}
        }
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
        if self.studio {
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
        if self.radio.is_none() || !self.sync_radio_to_wall_clock() {
            self.update_audio();
        }
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    /// Render the current room's stable score and crossfade to it.
    fn update_audio(&mut self) {
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
        self.goal_announced = false;
    }

    fn reset_current_room(&mut self) {
        self.reset_room_runtime();
        self.update_audio();
    }

    fn current_room_is_life(&self) -> bool {
        self.rooms[self.current].meta().id == "game-of-life"
    }

    fn current_status_override(&self, width: usize) -> Option<String> {
        self.current_room_is_life().then(|| {
            if width <= 400 {
                self.life_session.compact_status()
            } else {
                self.life_session.status()
            }
        })
    }

    fn reset_life_session(&mut self) {
        self.life_session = numinous_core::rooms::game_of_life::LifeSession::new(self.variation);
        self.life_accumulator = 0.0;
        self.clear_transient_audio();
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
            || self.show_help && self.modal_mode_active()
        {
            return 0;
        }
        self.advance_life(elapsed * self.time_scale)
    }

    fn draw_studio(&self, raster: &mut Raster, width: usize, height: usize) {
        self.studio_panel
            .draw(raster, self.input_mode, width, height, self.t);
    }

    fn modal_frame(&self, width: usize, height: usize) -> Option<Raster> {
        if self.show_help && self.modal_mode_active() {
            return None;
        }
        if let Some(play) = &self.arcade {
            Some(game_draw::draw_arcade(play, self.input_mode, width, height))
        } else if let Some(run) = &self.gauntlet {
            Some(game_draw::draw_gauntlet(
                &self.rooms,
                run,
                self.frame,
                self.input_mode,
                width,
                height,
            ))
        } else if let Some(play) = &self.munch {
            Some(game_draw::draw_munch(
                play,
                self.frame,
                self.input_mode,
                width,
                height,
            ))
        } else if let Some(play) = &self.nim {
            Some(game_draw::draw_nim(play, self.input_mode, width, height))
        } else {
            self.quiz
                .as_ref()
                .map(|quiz| game_draw::draw_quiz(&self.rooms, quiz, self.input_mode, width, height))
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
                input_legend::InputMode::Controller => ViewerInputMode::Controller,
            };
            let raster = self.session_viewer.draw(width, height, viewer_input_mode);
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
            } else {
                let phase =
                    effective_room_phase(room.meta().id, self.t, &self.inputs, self.the_show);
                room.render_input(&mut raster, phase, room_inputs);
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
        hud::draw_room_chrome(
            raster,
            room,
            &hud::RoomChrome {
                t: phase,
                room_card: self.room_card,
                show_info: self.show_info,
                show_help: self.show_help,
                show_journey: self.show_journey,
                banner_active: self.banner.is_some(),
                the_show: self.the_show,
                studio: self.studio,
                muted: self.muted,
                level: self.journey.level(),
                input_mode: self.input_mode,
            },
            inputs,
            status_override.as_deref(),
            width,
            height,
        );

        if self.show_help && !self.the_show {
            overlays::draw_help_overlay(
                raster,
                width,
                height,
                self.help_menu_selection(),
                self.input_mode,
                self.modal_mode_active(),
            );
        }

        if self.show_journey && !self.the_show {
            let board = numinous_core::load_scoreboard_file(&self.scores_file);
            overlays::draw_journey_overlay(
                raster,
                &self.journey,
                &board,
                self.rooms.len(),
                width,
                height,
                self.input_mode,
            );
        }

        if self.input_mode == input_legend::InputMode::Controller
            && let Some(point) = self.gamepad.cursor()
        {
            gamepad::draw_cursor(raster, point, width, height);
        }
    }

    fn present_raster(&mut self, mut raster: Raster, width: usize, height: usize) {
        if self.paused {
            overlays::draw_pause_overlay(&mut raster, width, height, self.input_mode);
        }
        self.draw_banner_on_raster(&mut raster, width, height);
        hud::draw_audio_state(&mut raster, &self.audio_state(), width);
        let (rw, rh) = (raster.width(), raster.height());
        let mut rgba = raster.to_rgba();
        self.era.apply(&mut rgba, rw, rh);
        self.blit(&rgba, rw, rh, width, height);
    }

    fn audio_state(&self) -> hud::AudioState {
        audio_state::describe(
            self.audio_program,
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

    /// Copy an RGBA frame (`rw` x `rh`) onto the window surface (`width` x `height`).
    fn blit(&mut self, rgba: &[u8], rw: usize, rh: usize, width: usize, height: usize) {
        let (Some(w), Some(h)) = (
            NonZeroU32::new(width as u32),
            NonZeroU32::new(height as u32),
        ) else {
            return;
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        for (i, pixel) in buffer.iter_mut().enumerate() {
            let (x, y) = (i % width, i / width);
            *pixel = if x < rw && y < rh {
                let o = (y * rw + x) * 4;
                (u32::from(rgba[o]) << 16) | (u32::from(rgba[o + 1]) << 8) | u32::from(rgba[o + 2])
            } else {
                BACKGROUND
            };
        }
        let _ = buffer.present();
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
        let window = Rc::new(window);
        let Ok(context) = softbuffer::Context::new(window.clone()) else {
            return;
        };
        let Ok(surface) = softbuffer::Surface::new(&context, window.clone()) else {
            return;
        };
        self.window = Some(window);
        self.surface = Some(surface);
        // Apply initial fullscreen if requested (borderless for broad compat; exclusive available via F cycle).
        if self.start_fullscreen
            && let Some(w) = &self.window
        {
            w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }
        self.player = match numinous_audio::LoopPlayer::new() {
            Ok(player) => Some(player),
            Err(error) => {
                // Silence must never be a mystery: say it on screen and in
                // the crash log, then keep running visual-only.
                self.banner = Some(feedback::sound_device_unavailable(&error));
                let path = crash_log_path();
                let _ = append_crash_log_at(&path, &format!("audio open failed: {error}\n"));
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
                self.session_viewer.close();
                let _ = numinous_core::persist_journey_delta(
                    &self.journey_file,
                    &self.journey_saved,
                    &self.journey,
                );
                event_loop.exit();
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
                if self.handle_global_audio_key(&logical_key, repeat) {
                    return;
                }
                if self.session_viewer.is_open() {
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    match logical_key {
                        Key::Named(NamedKey::Escape) => self.close_session_viewer(),
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
                        Key::Character(c) if c.as_str() == "a" => {
                            self.session_viewer.pan_result(-4);
                        }
                        Key::Character(c) if c.as_str() == "d" => {
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
                if self.handle_modal_help_key(&logical_key) {
                    return;
                }
                self.input_mode = input_legend::InputMode::KeyboardMouse;
                if self.handle_playtest_shortcut(&logical_key, repeat) {
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
                } else if self.studio {
                    // Studio mode: the keyboard is a math keyboard.
                    match logical_key {
                        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                            self.exit_studio();
                        }
                        Key::Named(NamedKey::F1) => {
                            self.studio_panel.toggle_help();
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
                    match logical_key {
                        // Esc is the menu, like every game since Doom. Quit from
                        // the window's close button.
                        Key::Named(NamedKey::Escape) => {
                            if self.the_show {
                                self.toggle_show();
                                self.show_help = false;
                            } else {
                                self.show_help = !self.show_help;
                            }
                        }
                        Key::Named(NamedKey::Tab) => {
                            self.enter_studio();
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
                        // E inspects, like use in every shooter.
                        Key::Character(c) if c.as_str() == "e" => {
                            self.show_info = !self.show_info;
                        }
                        // Q swaps the era, like swapping weapons.
                        Key::Character(c) if c.as_str() == "q" => {
                            self.era = self.era.next();
                            if let Some(window) = &self.window {
                                window.set_title(&self.title());
                            }
                        }
                        // R returns this visit to its initial state. Moving to a
                        // different room still deals the next variation.
                        Key::Character(c) if c.as_str() == "r" => {
                            self.reset_current_room();
                        }
                        // F cycles fullscreen modes for full screen view + options (windowed, borderless, exclusive).
                        // Borderless for compat; exclusive uses primary monitor's first video mode for "true" fullscreen.
                        // Shows current in banner (like volume) to surface the video setting.
                        Key::Character(c) if c.as_str() == "f" => {
                            if let Some(window) = &self.window {
                                let next = match window.fullscreen() {
                                    Some(winit::window::Fullscreen::Borderless(_)) => {
                                        // Try exclusive on primary monitor first mode.
                                        if let Some(monitor) = window.primary_monitor() {
                                            monitor
                                                .video_modes()
                                                .next()
                                                .map(winit::window::Fullscreen::Exclusive)
                                        } else {
                                            None
                                        }
                                    }
                                    Some(winit::window::Fullscreen::Exclusive(_)) => None,
                                    None => Some(winit::window::Fullscreen::Borderless(None)),
                                };
                                window.set_fullscreen(next.clone());
                                let label = match &next {
                                    Some(winit::window::Fullscreen::Borderless(_)) => {
                                        "BORDERLESS".to_string()
                                    }
                                    Some(winit::window::Fullscreen::Exclusive(m)) => {
                                        let size = m.size();
                                        format!("EXCLUSIVE {}x{}", size.width, size.height)
                                    }
                                    None => "WINDOWED".to_string(),
                                };
                                self.banner = Some(feedback::fullscreen(&label));
                            }
                        }
                        Key::Character(c) if c.as_str() == "h" => {
                            self.show_help = !self.show_help;
                        }
                        // G deals the quiz: guess the shape, in the window.
                        Key::Character(c) if c.as_str() == "g" && self.show_help => {
                            self.show_help = false;
                            self.quiz_next();
                        }
                        // C chomps: today's Munch board, in the window.
                        Key::Character(c) if c.as_str() == "c" && self.show_help => {
                            self.show_help = false;
                            self.munch_start();
                        }
                        // N is nim: three heaps against the Order.
                        Key::Character(c) if c.as_str() == "n" && self.show_help => {
                            self.show_help = false;
                            self.nim_start();
                        }
                        // T runs the Gauntlet: four stages, one number.
                        Key::Character(c) if c.as_str() == "t" && self.show_help => {
                            self.show_help = false;
                            self.gauntlet_start();
                        }
                        // V looses the Vexations: the arcade.
                        Key::Character(c) if c.as_str() == "v" && self.show_help => {
                            self.show_help = false;
                            self.arcade_start();
                        }
                        // J opens the journey: what the play has made of you.
                        Key::Character(c) if c.as_str() == "j" => {
                            self.toggle_journey();
                        }
                        // X opens the explicitly consented local MCP session viewer.
                        Key::Character(c) if c.as_str() == "x" => {
                            self.open_session_viewer();
                        }
                        // Y turns the radio dial: off, then station by station.
                        Key::Character(c) if c.as_str() == "y" && !repeat => {
                            let stations = numinous_core::STATIONS.len();
                            self.radio = match self.radio {
                                None => Some(0),
                                Some(i) if i + 1 < stations => Some(i + 1),
                                Some(_) => None,
                            };
                            self.tune_in();
                        }
                        // P keeps the picture: the postcard key.
                        Key::Character(c) if c.as_str() == "p" => {
                            if self.save_gate.admit(
                                save_gate::SaveKind::Postcard,
                                Instant::now(),
                                repeat,
                            ) && let Some(path) = self.save_postcard()
                                && let Some(window) = &self.window
                            {
                                window.set_title(&format!(
                                    "Numinous  |  postcard saved: {}",
                                    path.display()
                                ));
                            }
                        }
                        // L keeps the motion: a short looping APNG of this visit.
                        Key::Character(c) if c.as_str() == "l" => {
                            if self.save_gate.admit(
                                save_gate::SaveKind::ShortLoop,
                                Instant::now(),
                                repeat,
                            ) && let Some(path) = self.save_short_loop()
                                && let Some(window) = &self.window
                            {
                                window.set_title(&format!(
                                    "Numinous  |  loop saved: {}",
                                    path.display()
                                ));
                            }
                        }
                        // B for the big show (lean back).
                        Key::Character(c) if c.as_str() == "b" => {
                            self.toggle_show();
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
                if self.paused {
                    self.clear_pointer_state();
                    return;
                }
                if self.show_help && self.modal_mode_active() {
                    self.clear_pointer_state();
                    return;
                }
                self.input_mode = input_legend::InputMode::KeyboardMouse;
                let point = self.window.as_ref().and_then(|window| {
                    let size = window.inner_size();
                    mouse_input::normalized_window_point(self.mouse, (size.width, size.height))
                });
                match (state, point) {
                    (ElementState::Pressed, Some(point)) => self.begin_pointer_at(point),
                    (ElementState::Released, Some(point)) => self.end_pointer_at(point),
                    (ElementState::Pressed, None) => self.clear_pointer_state(),
                    (ElementState::Released, None) => {
                        self.set_pointer_state(mouse_input::pointer_state_after_left_release());
                    }
                }
            }
            WindowEvent::Focused(false) => {
                self.suspend_presentation_clock(Instant::now());
                self.clear_pointer_state();
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
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
        if !first_contact_obscured {
            self.advance_life_if_active(elapsed);
        }
        if !(self.paused || self.dragging || self.show_help && self.modal_mode_active()) {
            if !first_contact_obscured && self.rooms[self.current].meta().id == "mandelbrot" {
                self.mandelbrot_camera.advance(elapsed * self.time_scale);
            }
            let show_active = self.show_mode_active();
            let rate = if show_active { SHOW_T_RATE } else { T_RATE };
            let (next_phase, wrapped) = advance_gallery_phase(
                self.t,
                elapsed,
                self.time_scale,
                rate,
                first_contact_obscured,
            );
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

fn crash_log_path() -> std::path::PathBuf {
    #[cfg(test)]
    {
        test_state_path("crash")
    }
    #[cfg(not(test))]
    {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".numinous-crash.log")
    }
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
fn journey_path() -> std::path::PathBuf {
    #[cfg(test)]
    {
        test_state_path("journey")
    }
    #[cfg(not(test))]
    {
        if let Ok(path) = std::env::var("NUMINOUS_JOURNEY") {
            return std::path::PathBuf::from(path);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".numinous-journey")
    }
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
fn scores_path() -> std::path::PathBuf {
    #[cfg(test)]
    {
        test_state_path("scores")
    }
    #[cfg(not(test))]
    {
        if let Ok(path) = std::env::var("NUMINOUS_SCORES") {
            return std::path::PathBuf::from(path);
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home).join(".numinous-scores")
    }
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
    event_loop.run_app(&mut app).expect("run the app");
}

#[cfg(test)]
mod tests {
    use super::{
        App, AudioProgram, TestStateRoot, advance_gallery_phase, app_icon, append_crash_log_at,
        bounded_tick_seconds, effective_room_phase, julia_gpu_c, julia_gpu_vertical_span,
        life_step_audio_owned, live_mandelbrot_gpu_view, mandelbrot_gpu_view, radio_cache,
        room_transient_audio_owned, selected_life_step_audio, selected_parameter_sound,
        selected_room_interaction_audio,
    };
    use crate::input_legend::{InputMode, MenuChoice};
    use numinous_core::ROOM_BED_SOURCE_RATE;
    use std::sync::Arc;
    use std::time::{Duration, Instant, UNIX_EPOCH};
    use winit::keyboard::{Key, NamedKey};

    /// An app pointed at scratch files, with no window, player, or GPU.
    fn headless(name: &str) -> App {
        let mut app = App::new();
        app.journey = numinous_core::Journey::default();
        app.journey_saved = app.journey.clone();
        app.journey_file = super::test_state_path(name);
        app.scores_file = app.journey_file.with_extension("scores");
        let _ = std::fs::remove_file(&app.journey_file);
        let _ = std::fs::remove_file(&app.scores_file);
        app.level_seen = 1;
        app
    }

    #[test]
    fn crash_writer_waits_for_the_shared_erasure_lock() {
        let path = super::test_state_path("crash-lock");
        let _ = std::fs::remove_file(&path);
        let guard = numinous_core::lock_local_state(&path).expect("hold erasure lock");
        let writer_path = path.clone();
        let (sent, received) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            let result = append_crash_log_at(&writer_path, "diagnostic\n");
            sent.send(result).expect("report writer result");
        });
        assert!(
            received
                .recv_timeout(std::time::Duration::from_millis(25))
                .is_err(),
            "writer must wait while erasure owns the path"
        );
        drop(guard);
        received
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("writer resumes")
            .expect("writer succeeds");
        writer.join().expect("writer joined");
        assert_eq!(
            std::fs::read(&path).expect("crash receipt"),
            b"diagnostic\n"
        );
        numinous_core::remove_persisted_file(&path).expect("fixture cleanup");
    }

    #[test]
    fn room_score_prerender_is_device_independent_and_memory_bounded() {
        let mut largest = 0;
        for room in numinous_core::all_rooms() {
            let motif = room.motif().expect("catalog motif");
            let samples = motif.arrangement().render_stereo(ROOM_BED_SOURCE_RATE);
            assert_eq!(
                samples.len(),
                (motif.arrangement().seconds() * ROOM_BED_SOURCE_RATE as f32) as usize * 2,
                "{} source length",
                room.meta().id
            );
            largest = largest.max(samples.len());
        }
        assert!(
            largest <= 2_000_000,
            "largest room score held {largest} samples"
        );
    }

    fn select_life(app: &mut App) {
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "game-of-life")
            .expect("Life room");
        app.show_help = false;
        app.reset_life_session();
    }

    fn select_times_tables(app: &mut App) {
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "times-tables")
            .expect("Times Tables room");
        app.show_help = false;
    }

    fn select_galton(app: &mut App) {
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "galton-board")
            .expect("Galton Board room");
        app.show_help = false;
    }

    #[test]
    fn times_tables_holds_its_cardioid_until_input_but_the_show_keeps_sweeping() {
        assert_eq!(effective_room_phase("times-tables", 0.73, &[], false), 0.0);
        assert_eq!(effective_room_phase("times-tables", 0.73, &[], true), 0.73);
        assert_eq!(effective_room_phase("lissajous", 0.73, &[], false), 0.73);

        let input = [numinous_core::RoomInput::PointerDown {
            x: 0.4,
            y: 0.5,
            t: 0.2,
        }];
        assert_eq!(
            effective_room_phase("times-tables", 0.73, &input, false),
            0.73
        );

        let released = [numinous_core::RoomInput::PointerUp {
            x: 0.4,
            y: 0.5,
            t: 0.3,
        }];
        assert_eq!(
            effective_room_phase("times-tables", 0.73, &released, false),
            0.73
        );

        let invalid_release = [numinous_core::RoomInput::PointerUp {
            x: f64::NAN,
            y: 0.5,
            t: 0.3,
        }];
        assert_eq!(
            effective_room_phase("times-tables", 0.73, &invalid_release, false),
            0.0
        );
    }

    #[test]
    fn only_room_score_routes_the_hand_controlled_math_voice() {
        let app = headless("numinous_app_test_times_voice.txt");
        let room = app
            .rooms
            .iter()
            .find(|room| room.meta().id == "times-tables")
            .expect("Times Tables room");
        let input = [numinous_core::RoomInput::PointerDown {
            x: 0.375,
            y: 0.5,
            t: 0.2,
        }];

        assert!(
            selected_parameter_sound(
                AudioProgram::RoomScore,
                false,
                room.as_ref(),
                0.7,
                &[],
                false,
            )
            .is_none()
        );
        let voice = selected_parameter_sound(
            AudioProgram::RoomScore,
            false,
            room.as_ref(),
            0.7,
            &input,
            false,
        )
        .expect("accepted dial voice");
        assert_eq!(voice.ratio(), 1.25);
        assert!(
            selected_parameter_sound(
                AudioProgram::Studio,
                false,
                room.as_ref(),
                0.7,
                &input,
                false,
            )
            .is_none()
        );
        assert!(
            selected_parameter_sound(
                AudioProgram::Radio,
                false,
                room.as_ref(),
                0.7,
                &input,
                false,
            )
            .is_none()
        );
        assert!(
            selected_parameter_sound(
                AudioProgram::RoomScore,
                true,
                room.as_ref(),
                0.7,
                &input,
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn galton_coin_selection_reaches_the_room_score_voice() {
        let app = headless("numinous_app_test_galton_voice.txt");
        let room = app
            .rooms
            .iter()
            .find(|room| room.meta().id == "galton-board")
            .expect("Galton Board room");
        let input = |x| [numinous_core::RoomInput::PointerDown { x, y: 0.5, t: 0.4 }];
        let left = input(0.1);
        let fair = input(0.5);
        let right = input(0.9);

        let select = |inputs: &[numinous_core::RoomInput]| {
            selected_parameter_sound(
                AudioProgram::RoomScore,
                false,
                room.as_ref(),
                0.4,
                inputs,
                false,
            )
            .expect("selected coin voice")
        };
        let left = select(&left);
        let fair = select(&fair);
        let right = select(&right);

        assert!(left.root_hz() < fair.root_hz());
        assert!(fair.root_hz() < right.root_hz());
        assert_eq!(left.ratio(), 7.0 / 3.0);
        assert_eq!(fair.ratio(), 1.0);
        assert_eq!(right.ratio(), 7.0 / 3.0);
    }

    #[test]
    fn galton_peg_sequence_obeys_room_score_ownership() {
        let app = headless("numinous_app_test_galton_pegs.txt");
        let room = app
            .rooms
            .iter()
            .find(|room| room.meta().id == "galton-board")
            .expect("Galton Board room");
        let input = [numinous_core::RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.4,
        }];
        let select = |program, modal, muted, accepted| {
            selected_room_interaction_audio(
                program,
                modal,
                muted,
                accepted,
                room.as_ref(),
                &input,
                48_000,
            )
        };

        assert!(select(AudioProgram::RoomScore, false, false, true).is_some());
        assert!(select(AudioProgram::Studio, false, false, true).is_none());
        assert!(select(AudioProgram::Radio, false, false, true).is_none());
        assert!(select(AudioProgram::RoomScore, true, false, true).is_none());
        assert!(select(AudioProgram::RoomScore, false, true, true).is_none());
        assert!(select(AudioProgram::RoomScore, false, false, false).is_none());
    }

    #[test]
    fn double_pendulum_release_sequence_obeys_room_score_ownership() {
        let app = headless("numinous_app_test_pendulum_release.txt");
        let room = app
            .rooms
            .iter()
            .find(|room| room.meta().id == "double-pendulum")
            .expect("Double Pendulum room");
        let input = [
            numinous_core::RoomInput::PointerMove {
                x: 0.3,
                y: 0.6,
                t: 0.147,
            },
            numinous_core::RoomInput::PointerUp {
                x: 0.7,
                y: 0.4,
                t: 0.15,
            },
        ];
        let select = |program, modal, muted, accepted| {
            selected_room_interaction_audio(
                program,
                modal,
                muted,
                accepted,
                room.as_ref(),
                &input,
                48_000,
            )
        };

        assert!(select(AudioProgram::RoomScore, false, false, true).is_some());
        assert!(select(AudioProgram::Studio, false, false, true).is_none());
        assert!(select(AudioProgram::Radio, false, false, true).is_none());
        assert!(select(AudioProgram::RoomScore, true, false, true).is_none());
        assert!(select(AudioProgram::RoomScore, false, true, true).is_none());
        assert!(select(AudioProgram::RoomScore, false, false, false).is_none());
    }

    #[test]
    fn double_pendulum_release_dispatches_once_from_the_pointer_lifecycle() {
        let mut app = headless("numinous_app_test_pendulum_release_route.txt");
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "double-pendulum")
            .expect("Double Pendulum room");
        let events_before = app.interaction_audio_events.get();

        app.t = 0.1;
        assert!(app.record_room_touch((0.3, 0.6)));
        app.poking = true;
        app.t = 0.147;
        app.move_pointer_to((0.35, 0.55), true);
        assert_eq!(app.interaction_audio_events.get(), events_before);

        app.t = 0.15;
        app.end_pointer_at((0.7, 0.4));
        assert_eq!(app.interaction_audio_events.get(), events_before + 1);
        app.end_pointer_at((0.7, 0.4));
        assert_eq!(
            app.interaction_audio_events.get(),
            events_before + 1,
            "a second lift without an open gesture cannot replay the event"
        );
    }

    #[test]
    fn a_radio_transition_cancels_an_open_pendulum_before_room_score_returns() {
        let mut app = headless("numinous_app_test_pendulum_radio_boundary.txt");
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "double-pendulum")
            .expect("Double Pendulum room");
        app.radio = Some(numinous_core::STATIONS.len() - 1);
        app.audio_program = AudioProgram::Radio;
        app.t = 0.1;
        assert!(app.record_room_touch((0.3, 0.6)));
        app.poking = true;
        app.t = 0.147;
        app.move_pointer_to((0.35, 0.55), true);
        let events_before = app.interaction_audio_events.get();

        app.handle_gamepad_command(crate::gamepad::Command::CycleRadio);

        assert!(app.radio.is_none());
        assert_eq!(app.audio_program, AudioProgram::RoomScore);
        assert!(!app.poking);
        assert!(matches!(
            app.inputs.last(),
            Some(numinous_core::RoomInput::PointerCancel)
        ));
        app.t = 0.15;
        app.end_pointer_at((0.7, 0.4));
        assert_eq!(app.interaction_audio_events.get(), events_before);
    }

    #[test]
    fn failed_radio_resync_cancels_an_open_pendulum_before_room_score_returns() {
        let mut app = headless("numinous_app_test_pendulum_radio_resync_boundary.txt");
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "double-pendulum")
            .expect("Double Pendulum room");
        app.radio = Some(0);
        app.audio_program = AudioProgram::Radio;
        app.radio_paths.clear();
        app.t = 0.1;
        assert!(app.record_room_touch((0.3, 0.6)));
        app.poking = true;
        app.t = 0.147;
        app.move_pointer_to((0.35, 0.55), true);
        let events_before = app.interaction_audio_events.get();

        assert!(!app.sync_radio_at(0.0));

        assert_eq!(app.audio_program, AudioProgram::RoomScore);
        assert!(!app.poking);
        assert!(matches!(
            app.inputs.last(),
            Some(numinous_core::RoomInput::PointerCancel)
        ));
        app.t = 0.15;
        app.end_pointer_at((0.7, 0.4));
        assert_eq!(app.interaction_audio_events.get(), events_before);
    }

    #[test]
    fn galton_release_and_bet_motion_preserve_the_active_peg_sequence() {
        let mut app = headless("numinous_app_test_galton_peg_lifecycle.txt");
        select_galton(&mut app);
        let clears_before = app.transient_audio_clears.get();

        assert!(app.record_room_touch((0.5, 0.5)));
        app.poking = true;
        app.move_pointer_to((0.6, 0.5), true);
        app.end_pointer_at((0.6, 0.5));

        assert_eq!(app.transient_audio_clears.get(), clears_before);
        assert!(matches!(
            app.inputs.as_slice(),
            [
                numinous_core::RoomInput::PointerDown { .. },
                numinous_core::RoomInput::PointerMove { .. },
                numinous_core::RoomInput::PointerUp { .. }
            ]
        ));
        assert!(room_transient_audio_owned(AudioProgram::RoomScore, false));
        assert!(!room_transient_audio_owned(AudioProgram::RoomScore, true));
        assert!(!room_transient_audio_owned(AudioProgram::Radio, false));
    }

    #[test]
    fn show_entry_retires_a_room_interaction_sequence() {
        let mut app = headless("numinous_app_test_show_retires_galton_pegs.txt");
        select_galton(&mut app);
        assert!(app.record_room_touch((0.5, 0.5)));
        let clears_before = app.transient_audio_clears.get();

        app.toggle_show();

        assert!(app.the_show);
        assert_eq!(app.transient_audio_clears.get(), clears_before + 1);
    }

    #[test]
    fn double_pendulum_gesture_reaches_the_room_score_voice() {
        let app = headless("numinous_app_test_pendulum_voice.txt");
        let room = app
            .rooms
            .iter()
            .find(|room| room.meta().id == "double-pendulum")
            .expect("Double Pendulum room");
        let select = |inputs: &[numinous_core::RoomInput]| {
            selected_parameter_sound(
                AudioProgram::RoomScore,
                false,
                room.as_ref(),
                0.35,
                inputs,
                false,
            )
            .expect("accepted pendulum voice")
        };
        let left = [numinous_core::RoomInput::PointerDown {
            x: 0.1,
            y: 0.5,
            t: 0.2,
        }];
        let right = [numinous_core::RoomInput::PointerDown {
            x: 0.9,
            y: 0.5,
            t: 0.2,
        }];
        assert!(select(&left).root_hz() < select(&right).root_hz());

        let gentle = [
            numinous_core::RoomInput::PointerDown {
                x: 0.58,
                y: 0.5,
                t: 0.05,
            },
            numinous_core::RoomInput::PointerMove {
                x: 0.58,
                y: 0.5,
                t: 0.10,
            },
            numinous_core::RoomInput::PointerUp {
                x: 0.6,
                y: 0.5,
                t: 0.15,
            },
        ];
        let fast = [
            numinous_core::RoomInput::PointerDown {
                x: 0.3,
                y: 0.5,
                t: 0.10,
            },
            numinous_core::RoomInput::PointerMove {
                x: 0.3,
                y: 0.5,
                t: 0.147,
            },
            numinous_core::RoomInput::PointerUp {
                x: 0.6,
                y: 0.5,
                t: 0.15,
            },
        ];
        assert!(select(&gentle).gain() < select(&fast).gain());
        assert!(
            selected_parameter_sound(
                AudioProgram::Radio,
                false,
                room.as_ref(),
                0.35,
                &fast,
                false,
            )
            .is_none()
        );
    }

    #[test]
    fn the_show_sweeps_the_times_tables_voice_without_retained_hand_input() {
        let app = headless("numinous_app_test_times_show_voice.txt");
        let room = app
            .rooms
            .iter()
            .find(|room| room.meta().id == "times-tables")
            .expect("Times Tables room");
        let retained = [numinous_core::RoomInput::PointerDown {
            x: 0.375,
            y: 0.5,
            t: 0.2,
        }];

        let early = selected_parameter_sound(
            AudioProgram::RoomScore,
            false,
            room.as_ref(),
            0.1,
            &retained,
            true,
        )
        .expect("Show voice");
        let late = selected_parameter_sound(
            AudioProgram::RoomScore,
            false,
            room.as_ref(),
            0.7,
            &retained,
            true,
        )
        .expect("moving Show voice");
        assert_ne!(early.ratio(), late.ratio());
    }

    #[test]
    fn four_lobes_raise_one_earned_banner_and_reset_cleanly() {
        let mut app = headless("numinous_app_test_times_goal.txt");
        select_times_tables(&mut app);
        app.switch(1);
        app.switch(-1);
        assert_ne!(app.variation, 0);
        assert_eq!(app.rooms[app.current].meta().id, "times-tables");
        assert_eq!(
            app.rooms[app.current]
                .status_input(
                    effective_room_phase("times-tables", app.t, &app.inputs, false),
                    &app.inputs,
                )
                .as_deref(),
            Some("K 2.00  CLOSED  1 LOBE  TARGET 4")
        );

        assert!(app.record_room_touch((0.374, 0.5)));
        assert!(app.goal_announced);
        assert_eq!(
            app.banner.as_ref().expect("earned Aha").lines(),
            ["FOUR LOBES FOUND", "INSPECT: WHY THE HEART MATTERS"]
        );

        app.maybe_announce_room_goal();
        assert!(app.goal_announced, "the same discovery does not spam");
        app.reset_room_runtime();
        assert!(!app.goal_announced);
        assert!(app.banner.is_none());
        assert!(app.inputs.is_empty());
        assert_eq!(app.t, 0.0);
        assert_eq!(
            app.rooms[app.current]
                .status_input(
                    effective_room_phase("times-tables", app.t, &app.inputs, false),
                    &app.inputs,
                )
                .as_deref(),
            Some("K 2.00  CLOSED  1 LOBE  TARGET 4")
        );
    }

    fn write_test_wav(path: &std::path::Path, channels: u16, seconds: u32) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("write wav");
        for i in 0..44_100 * seconds {
            let sample = ((i as f32 * 0.05).sin() * 12_000.0) as i16;
            for channel in 0..channels {
                let signed = if channel % 2 == 0 { sample } else { -sample };
                writer.write_sample(signed).expect("sample");
            }
        }
        writer.finalize().expect("finalize");
    }

    #[test]
    fn app_test_profiles_are_stable_isolated_and_owned() {
        let player_journey = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|home| std::path::PathBuf::from(home).join(".numinous-journey"));
        let player_scores = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|home| std::path::PathBuf::from(home).join(".numinous-scores"));

        let first = std::thread::spawn(|| {
            let journey = super::journey_path();
            let scores = super::scores_path();
            assert_eq!(journey, super::journey_path());
            assert_eq!(scores, super::scores_path());
            assert_eq!(journey.parent(), scores.parent());
            assert_ne!(journey, scores);
            std::fs::write(&journey, "isolated").expect("write isolated app Journey");
            (journey, scores)
        })
        .join()
        .expect("first app test profile thread");
        let second = std::thread::spawn(|| (super::journey_path(), super::scores_path()))
            .join()
            .expect("second app test profile thread");

        assert_ne!(first.0.parent(), second.0.parent());
        assert!(
            !first.0.exists(),
            "the first thread owns and clears its files"
        );
        assert!(
            !second.0.exists(),
            "the second thread owns and clears its files"
        );
        if let Ok(path) = player_journey {
            assert_ne!(first.0, path);
            assert_ne!(second.0, path);
        }
        if let Ok(path) = player_scores {
            assert_ne!(first.1, path);
            assert_ne!(second.1, path);
        }

        let collision = std::env::temp_dir().join(format!(
            "numinous-app-test-collision-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&collision);
        let _ = std::fs::remove_file(&collision);
        std::fs::write(&collision, "not a directory").expect("write collision file");
        let result = std::panic::catch_unwind(|| TestStateRoot::at(collision.clone()));
        assert!(result.is_err(), "a file collision must be rejected");
        std::fs::remove_file(collision).expect("remove collision file");
    }

    #[test]
    fn losing_the_pointer_mid_gesture_records_a_cancel() {
        let mut app = headless("numinous_app_test_gesture_cancel.txt");
        app.poking = true;
        crate::room_input::record_pointer_down(&mut app.inputs, (0.4, 0.4), 0.1);
        app.t = 0.8;
        // Focus loss and modal opens route through set_pointer_state, which
        // must close the open gesture gently.
        app.clear_pointer_state();
        assert!(!app.poking);
        assert_eq!(
            app.inputs.last(),
            Some(&numinous_core::RoomInput::PointerCancel),
            "an interrupted gesture ends in a cancel, not a phantom hold"
        );
        assert!(matches!(
            app.inputs.get(app.inputs.len() - 2),
            Some(numinous_core::RoomInput::PointerMove { t, .. }) if *t == 0.8
        ));
        // A release recorded normally is not followed by a stray cancel.
        app.poking = true;
        crate::room_input::record_pointer_down(&mut app.inputs, (0.5, 0.5), 0.2);
        crate::room_input::record_pointer_up(&mut app.inputs, (0.5, 0.5), 0.25);
        app.clear_pointer_state();
        assert!(matches!(
            app.inputs.last(),
            Some(numinous_core::RoomInput::PointerUp { .. })
        ));
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn switching_rooms_records_visits_and_persists() {
        let mut app = headless("numinous_app_test_switch.txt");
        app.switch(1);
        app.switch(1);
        assert_eq!(app.journey.visited.len(), 2, "two rooms entered");
        let disk = numinous_core::Journey::from_text(
            &std::fs::read_to_string(&app.journey_file).expect("persisted"),
        );
        assert_eq!(disk.visited, app.journey.visited);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn the_quiz_deals_records_and_scores_wins() {
        let mut app = headless("numinous_app_test_quiz.txt");
        app.quiz_next();
        assert_eq!(app.journey.plays, 1, "dealing a round is a play");
        let disk = numinous_core::Journey::from_text(
            &std::fs::read_to_string(&app.journey_file).expect("persisted deal"),
        );
        assert_eq!(disk.plays, 1, "dealing a round persists the play");
        let answer = app.quiz.as_ref().expect("a round is live").round.answer;
        app.quiz_answer('!');
        assert!(
            app.quiz.as_ref().unwrap().flash.is_none(),
            "letters off the menu do nothing"
        );
        app.quiz_answer(answer);
        assert_eq!(app.journey.wins, 1, "the right answer is a win");
        let disk = numinous_core::Journey::from_text(
            &std::fs::read_to_string(&app.journey_file).expect("persisted win"),
        );
        assert_eq!(disk.wins, 1, "the right answer persists the win");
        let (correct, _) = app.quiz.as_ref().unwrap().flash.expect("verdict shows");
        assert!(correct);
        app.quiz_next();
        assert_eq!(app.journey.plays, 2, "the next round deals");
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn level_ups_raise_the_banner_with_lore() {
        let mut app = headless("numinous_app_test_banner.txt");
        app.journey.play();
        app.journey_changed(); // one spark crosses the first threshold: level 2
        let banner = app.banner.as_ref().expect("the banner rises");
        let lines = banner.lines();
        assert!(lines[0].contains("LEVEL UP  LV 2"));
        assert!(lines.len() >= 2, "the lore line rides along");
        assert!(banner.frames_left() > 0);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn room_reset_preserves_visit_and_clears_interaction() {
        let mut app = headless("numinous_app_test_room_reset.txt");
        app.variation = 17;
        app.t = 0.8;
        app.pokes.push((0.2, 0.7));
        app.inputs.push(numinous_core::RoomInput::PointerDown {
            x: 0.2,
            y: 0.7,
            t: 0.8,
        });
        let room_id = app.rooms[app.current].meta().id;

        app.reset_current_room();

        assert_eq!(app.rooms[app.current].meta().id, room_id);
        assert_eq!(app.variation, 17);
        assert_eq!(app.t, 0.0);
        assert!(app.pokes.is_empty());
        assert!(app.inputs.is_empty());
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn life_session_keeps_advancing_after_a_gallery_phase_wrap() {
        let mut app = headless("numinous_app_test_life_continuity.txt");
        select_life(&mut app);
        let mut advanced = 0;
        while advanced < 140 {
            let remaining = 140 - advanced;
            let batch = remaining.min(super::MAX_LIFE_STEPS_PER_TICK);
            advanced += app.advance_life(super::LIFE_STEP_SECONDS * batch as f64);
        }
        app.t = 0.999;
        app.t = 0.0;
        assert_eq!(app.advance_life(super::LIFE_STEP_SECONDS), 1);
        assert_eq!(app.life_session.generation(), 141);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn life_step_audio_obeys_program_ownership_and_uses_the_exact_step() {
        let mut session = numinous_core::rooms::game_of_life::LifeSession::new(4);
        assert!(session.launch((0.5, 0.5)));
        session.advance();
        assert_eq!(session.tracked_glider_phase(), Some(1));
        assert_eq!(session.step_sound().glider_phase(), Some(1));

        let audio =
            selected_life_step_audio(AudioProgram::RoomScore, false, false, 1, &session, 48_000)
                .expect("room-score Life step");
        assert_eq!(audio.len() % 2, 0);
        assert!(audio.iter().any(|sample| sample.abs() > 0.0));
        assert!(life_step_audio_owned(
            AudioProgram::RoomScore,
            false,
            "game-of-life"
        ));

        for (program, modal, muted, steps) in [
            (AudioProgram::Studio, false, false, 1),
            (AudioProgram::Radio, false, false, 1),
            (AudioProgram::RoomScore, true, false, 1),
            (AudioProgram::RoomScore, false, true, 1),
            (AudioProgram::RoomScore, false, false, 0),
        ] {
            assert!(
                selected_life_step_audio(program, modal, muted, steps, &session, 48_000,).is_none()
            );
        }
        assert!(!life_step_audio_owned(
            AudioProgram::Studio,
            false,
            "game-of-life"
        ));
        assert!(!life_step_audio_owned(
            AudioProgram::Radio,
            false,
            "game-of-life"
        ));
        assert!(!life_step_audio_owned(
            AudioProgram::RoomScore,
            true,
            "game-of-life"
        ));
        assert!(!life_step_audio_owned(
            AudioProgram::RoomScore,
            false,
            "times-tables"
        ));

        session.advance();
        session.advance();
        let newest =
            selected_life_step_audio(AudioProgram::RoomScore, false, false, 3, &session, 48_000)
                .expect("newest presented generation");
        assert_eq!(newest, session.step_sound().render_stereo(48_000));
    }

    #[test]
    fn life_touch_uses_the_shared_room_input_and_session_route() {
        let mut app = headless("numinous_app_test_life_touch.txt");
        select_life(&mut app);
        let clears_before_launch = app.transient_audio_clears.get();

        assert!(app.record_room_touch((0.3, 0.7)));
        assert_eq!(app.pokes, vec![(0.3, 0.7)]);
        assert!(matches!(
            app.inputs.as_slice(),
            [numinous_core::RoomInput::PointerDown { x: 0.3, y: 0.7, .. }]
        ));
        assert_eq!(app.life_session.launches(), 1);
        assert_eq!(app.life_accumulator, 0.0);
        assert_eq!(
            app.transient_audio_clears.get(),
            clears_before_launch + 1,
            "a successful launch retires the previously presented birth texture"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn room_reset_restores_life_and_closes_a_held_pointer() {
        let mut app = headless("numinous_app_test_life_reset.txt");
        select_life(&mut app);
        app.record_room_touch((0.4, 0.6));
        app.poking = true;
        app.advance_life(super::LIFE_STEP_SECONDS * 9.0);
        let clears_before_reset = app.transient_audio_clears.get();

        app.reset_current_room();

        assert!(!app.poking);
        assert!(app.inputs.is_empty());
        assert!(app.pokes.is_empty());
        assert_eq!(app.life_session.generation(), 0);
        assert_eq!(app.life_session.launches(), 0);
        assert_eq!(app.life_accumulator, 0.0);
        assert_eq!(
            app.transient_audio_clears.get(),
            clears_before_reset + 1,
            "reset retires audio from the discarded Life generation"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn life_status_uses_the_persistent_session_after_history_limits() {
        let mut app = headless("numinous_app_test_life_status.txt");
        select_life(&mut app);
        for i in 0..25 {
            let x = 0.1 + (i % 5) as f64 * 0.18;
            let y = 0.1 + (i / 5) as f64 * 0.18;
            assert!(app.record_room_touch((x, y)));
        }
        for _ in 0..141 {
            app.life_session.advance();
        }
        app.t = 0.0;

        assert_eq!(app.pokes.len(), numinous_core::MAX_ROOM_POKES);
        assert_eq!(app.life_session.launches(), 25);
        let wide = app.current_status_override(900).expect("wide Life status");
        let compact = app
            .current_status_override(360)
            .expect("compact Life status");
        assert!(wide.contains("GEN 141"), "got: {wide}");
        assert!(wide.contains("GLIDERS 25"), "got: {wide}");
        assert!(compact.contains("G141"), "got: {compact}");
        assert!(compact.contains("GL25"), "got: {compact}");
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn life_advancement_obeys_pause_focus_and_speed_controls() {
        let mut app = headless("numinous_app_test_life_pause.txt");
        select_life(&mut app);
        app.paused = true;
        assert_eq!(app.advance_life_if_active(super::LIFE_STEP_SECONDS), 0);
        assert_eq!(app.life_session.generation(), 0);
        app.paused = false;
        app.window_active = false;
        assert_eq!(app.advance_life_if_active(super::LIFE_STEP_SECONDS), 0);
        app.window_active = true;
        assert_eq!(app.advance_life_if_active(super::LIFE_STEP_SECONDS), 1);

        let phase = app.t;
        let speed = app.time_scale;
        assert!(app.apply_wheel_delta(1.0));
        assert_eq!(app.t, phase, "Life wheel changes cadence, not hidden phase");
        assert!(app.time_scale > speed);
        let after_wheel = app.time_scale;
        app.handle_gamepad_command(crate::gamepad::Command::PhaseDelta(-0.1));
        assert_eq!(app.t, phase);
        assert!(app.time_scale < after_wheel);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn presentation_clock_advances_a_studio_morph_through_pause_and_focus_loss() {
        let mut app = headless("numinous_app_test_studio_focus_clock.txt");
        app.studio = true;
        assert!(app.studio_panel.load_random_recipe().is_some());
        let start = Instant::now();

        app.paused = true;
        app.advance_presentation_time(0.3);
        assert!(
            app.studio_panel.load_random_recipe().is_none(),
            "pause must not finish a half-complete morph early"
        );

        app.suspend_presentation_clock(start);
        app.resume_presentation_clock(start + Duration::from_millis(300));
        assert!(
            app.studio_panel.load_random_recipe().is_some(),
            "the remaining focus-loss time must finish the original morph"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn controller_reset_closes_a_held_life_touch() {
        let mut app = headless("numinous_app_test_life_controller_reset.txt");
        select_life(&mut app);
        assert!(app.record_room_touch((0.5, 0.5)));
        app.poking = true;

        app.handle_gamepad_command(crate::gamepad::Command::Reset);

        assert!(!app.poking);
        assert!(app.inputs.is_empty());
        assert!(app.pokes.is_empty());
        assert_eq!(app.life_session.generation(), 0);
        assert_eq!(app.life_session.launches(), 0);
        app.handle_gamepad_command(crate::gamepad::Command::PointerMoved {
            point: (0.7, 0.7),
            held: true,
        });
        app.handle_gamepad_command(crate::gamepad::Command::PrimaryUp);
        assert!(app.inputs.is_empty());
        assert!(app.pokes.is_empty());
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn controller_primary_routes_one_complete_life_launch() {
        let mut app = headless("numinous_app_test_life_controller_touch.txt");
        select_life(&mut app);
        app.gamepad.set_cursor_for_test((0.35, 0.65));

        app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);
        assert!(app.poking);
        assert_eq!(app.life_session.launches(), 1);
        assert!(matches!(
            app.inputs.as_slice(),
            [numinous_core::RoomInput::PointerDown {
                x: 0.35,
                y: 0.65,
                ..
            }]
        ));

        app.handle_gamepad_command(crate::gamepad::Command::PrimaryUp);
        assert!(!app.poking);
        assert!(matches!(
            app.inputs.last(),
            Some(numinous_core::RoomInput::PointerUp {
                x: 0.35,
                y: 0.65,
                ..
            })
        ));
        assert_eq!(app.life_session.launches(), 1);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn life_postcard_matches_the_persistent_session() {
        let mut app = headless("numinous_app_test_life_postcard.txt");
        select_life(&mut app);
        app.record_room_touch((0.35, 0.65));
        for _ in 0..141 {
            app.life_session.advance();
        }
        let dir = std::env::temp_dir().join(format!(
            "numinous-life-postcard-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create postcard directory");

        let path = app.save_postcard_to(&dir).expect("save Life postcard");
        assert!(
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("game-of-life-141")),
            "Life postcard names the persistent generation: {}",
            path.display()
        );
        let file = std::fs::File::open(path).expect("open Life postcard");
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = decoder.read_info().expect("read Life postcard header");
        let mut decoded = vec![
            0;
            reader
                .output_buffer_size()
                .expect("decoded postcard dimensions fit address space")
        ];
        let output = reader
            .next_frame(&mut decoded)
            .expect("decode Life postcard");
        let decoded = &decoded[..output.buffer_size()];

        let size = crate::postcard::POSTCARD_SIZE as usize;
        let room = app.rooms[app.current].as_ref();
        let mut expected = numinous_core::Raster::with_accent(size, size, room.meta().accent);
        app.life_session.render(&mut expected);
        let mut expected = expected.to_rgba();
        app.era.apply(&mut expected, size, size);
        assert_eq!(decoded, expected);

        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn embedded_app_icon_decodes() {
        assert!(app_icon().is_some());
    }

    #[test]
    fn accelerated_mandelbrot_uses_the_core_camera_and_shared_chrome() {
        let phase = 0.63;
        let variation = 17;
        let (center_x, center_y, half_span) =
            numinous_core::rooms::mandelbrot::automatic_view(phase, variation);
        let gpu = mandelbrot_gpu_view(phase, variation, 900, 700, &[]);
        assert!((f64::from(gpu.0) - center_x).abs() < 1e-6);
        assert!((f64::from(gpu.1) - center_y).abs() < 1e-6);
        let expected_vertical_span = 2.0 * half_span * 700.0 / 900.0;
        assert!((f64::from(gpu.2) - expected_vertical_span).abs() < 1e-6);

        let inputs = [
            numinous_core::RoomInput::PointerDown {
                x: 0.5,
                y: 0.5,
                t: phase,
            },
            numinous_core::RoomInput::PointerDown {
                x: 0.75,
                y: 0.25,
                t: phase + 0.1,
            },
        ];
        let (selected_x, selected_y, selected_half_span) =
            numinous_core::rooms::mandelbrot::selected_view_input(
                &inputs, 900, 700, variation, phase,
            );
        let selected_gpu = mandelbrot_gpu_view(phase, variation, 900, 700, &inputs);
        assert!((f64::from(selected_gpu.0) - selected_x).abs() < 1e-6);
        assert!((f64::from(selected_gpu.1) - selected_y).abs() < 1e-6);
        assert!(selected_half_span < half_span);
        assert!(
            (f64::from(selected_gpu.2) - 2.0 * selected_half_span * 700.0 / 900.0).abs() < 1e-6
        );
        assert_eq!(
            selected_gpu,
            mandelbrot_gpu_view(0.99, variation, 900, 700, &inputs),
            "deterministic face rendering preserves the selected camera"
        );

        let mut app = headless("numinous_app_test_gpu_chrome.txt");
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "mandelbrot")
            .expect("Mandelbrot room");
        app.show_help = false;
        app.room_card = 0;
        app.t = phase;
        let source = vec![64u8; 320 * 220 * 4];
        let mut raster = numinous_core::Raster::from_rgba(
            320,
            220,
            app.rooms[app.current].meta().accent,
            &source,
        )
        .expect("GPU frame import");
        let before = raster.to_rgba();
        let room = &app.rooms[app.current];
        app.draw_room_interface(&mut raster, room.as_ref(), 320, 220);
        let after = raster.to_rgba();
        assert_ne!(
            after, before,
            "GPU frames must receive title and footer chrome"
        );
        assert!(
            after[(220 - 8) * 320 * 4..]
                .chunks_exact(4)
                .any(|pixel| pixel[0..3] != [64, 64, 64]),
            "the reset footer reaches the accelerated frame"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn elapsed_simulation_time_is_measured_and_bounded() {
        let ordinary = bounded_tick_seconds(Duration::from_millis(16));
        assert!((ordinary - 0.016).abs() < 1e-9);
        assert_eq!(bounded_tick_seconds(Duration::from_secs(10)), 0.05);
    }

    #[test]
    fn journey_banner_preserves_the_first_contact_clock_and_card() {
        let mut banner = Some(super::feedback::level_up(2, 0));
        let mut phase = 0.0;
        let mut room_card = crate::room_input::ROOM_CARD_FRAMES;

        for _ in 0..300 {
            let obscured = banner.is_some() && room_card > 0;
            let (next, wrapped) = advance_gallery_phase(phase, 1.0 / 60.0, 1.0, 0.24, obscured);
            phase = next;
            assert!(!wrapped);
            crate::room_input::tick_room_card(&mut room_card, banner.is_some());
            if banner.as_mut().is_some_and(|value| !value.tick()) {
                banner = None;
            }
        }

        assert!(banner.is_none());
        assert_eq!(phase, 0.0);
        assert_eq!(room_card, crate::room_input::ROOM_CARD_FRAMES);
        let (next, wrapped) = advance_gallery_phase(phase, 1.0 / 60.0, 1.0, 0.24, false);
        assert!(next > 0.0);
        assert!(!wrapped);
        crate::room_input::tick_room_card(&mut room_card, false);
        assert_eq!(room_card, crate::room_input::ROOM_CARD_FRAMES - 1);
    }

    #[test]
    fn live_mandelbrot_view_tracks_the_persistent_camera() {
        let mut camera = numinous_core::rooms::mandelbrot::MandelbrotCamera::new(17);
        let initial = live_mandelbrot_gpu_view(camera, 900, 700).expect("opening GPU view");
        camera.advance(1.0);
        let advanced = live_mandelbrot_gpu_view(camera, 900, 700).expect("advanced GPU view");
        assert_ne!(advanced, initial, "elapsed time advances the live camera");

        assert!(camera.dive(0.75, 0.25, 900, 700));
        let selected = live_mandelbrot_gpu_view(camera, 900, 700).expect("selected GPU view");
        camera.advance(1.0);
        let deeper = live_mandelbrot_gpu_view(camera, 900, 700).expect("deeper GPU view");
        assert_ne!(deeper, selected, "a selected target keeps zooming");
        assert!(deeper.2 < selected.2, "the vertical span keeps shrinking");
    }

    #[test]
    fn deep_mandelbrot_view_falls_back_before_gpu_coordinates_collapse() {
        let mut camera = numinous_core::rooms::mandelbrot::MandelbrotCamera::new(17);
        camera.advance(200.0);
        assert!(live_mandelbrot_gpu_view(camera, 900, 700).is_none());
    }

    #[test]
    fn controller_dpad_navigates_rooms_without_a_mouse() {
        let mut app = headless("numinous_app_test_controller_room.txt");
        app.show_help = false;
        let original = app.current;

        app.handle_gamepad_command(crate::gamepad::Command::Right);

        assert_ne!(app.current, original);
        assert!(app.inputs.is_empty());
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn controller_can_open_and_leave_every_menu_destination() {
        for selection in 0..MenuChoice::ALL.len() {
            let mut app = headless(&format!(
                "numinous_app_test_controller_destination_{selection}.txt"
            ));
            app.controller_menu_selection = selection;

            app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);

            assert!(!app.show_help);
            match MenuChoice::at(selection) {
                MenuChoice::Quiz => assert!(app.quiz.is_some()),
                MenuChoice::Munch => assert!(app.munch.is_some()),
                MenuChoice::Nim => assert!(app.nim.is_some()),
                MenuChoice::Gauntlet => assert!(app.gauntlet.is_some()),
                MenuChoice::Arcade => assert!(app.arcade.is_some()),
                MenuChoice::Show => assert!(app.the_show),
                MenuChoice::Studio => assert!(app.studio),
                MenuChoice::Journey => assert!(app.show_journey),
                MenuChoice::WatchAgent => assert!(app.session_viewer.is_open()),
            }

            app.handle_gamepad_command(crate::gamepad::Command::Back);
            assert!(!app.the_show);
            assert!(!app.studio);
            assert!(!app.show_journey);
            assert!(!app.session_viewer.is_open());
            assert!(
                app.quiz.is_none()
                    && app.munch.is_none()
                    && app.nim.is_none()
                    && app.gauntlet.is_none()
                    && app.arcade.is_none(),
                "controller Back leaves menu destination {selection}"
            );
            let _ = std::fs::remove_file(&app.journey_file);
            let _ = std::fs::remove_file(&app.scores_file);
        }
    }

    #[test]
    fn controller_pause_is_explicit_and_sets_the_active_input_mode() {
        use crate::gamepad::Command;

        let mut app = headless("numinous_app_test_controller_pause.txt");
        app.show_help = false;
        assert_eq!(app.input_mode, InputMode::KeyboardMouse);

        app.handle_gamepad_command(Command::CancelPointer);
        assert_eq!(app.input_mode, InputMode::KeyboardMouse);
        assert!(!app.paused);

        app.handle_gamepad_command(Command::Pause);
        assert_eq!(app.input_mode, InputMode::Controller);
        assert!(app.paused);

        app.arcade_start();
        app.paused = true;
        app.input_mode = InputMode::KeyboardMouse;
        let before = {
            let play = app.arcade.as_ref().unwrap();
            (play.run.muncher, play.run.score, play.run.eaten.clone())
        };
        app.handle_gamepad_command(Command::Right);
        app.handle_gamepad_command(Command::PrimaryDown);
        let after = {
            let play = app.arcade.as_ref().unwrap();
            (play.run.muncher, play.run.score, play.run.eaten.clone())
        };
        assert_eq!(after, before, "paused Arcade rejects movement and eating");
        assert_eq!(
            app.input_mode,
            InputMode::KeyboardMouse,
            "ignored controller input does not steal the active legend"
        );

        app.handle_gamepad_command(Command::Pause);
        assert!(!app.paused);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn controller_menu_pauses_and_resumes_without_discarding_a_game() {
        let mut app = headless("numinous_app_test_controller_pause_menu.txt");
        app.show_help = false;
        app.quiz_next();

        app.handle_gamepad_command(crate::gamepad::Command::Menu);
        assert!(app.show_help);
        assert!(app.quiz.is_some());
        assert!(app.modal_frame(320, 220).is_none());
        assert_eq!(app.help_menu_selection(), None);
        assert_eq!(
            crate::input_legend::help_lines(app.input_mode, app.help_menu_selection(), true),
            [
                "ACTIVITY PAUSED",
                "SOUTH / START / EAST RETURN",
                "THE CURRENT RUN STAYS INTACT"
            ]
        );

        let selection = app.controller_menu_selection;
        app.handle_gamepad_command(crate::gamepad::Command::Right);
        assert_eq!(app.controller_menu_selection, selection);
        assert!(app.quiz.as_ref().is_some_and(|quiz| quiz.flash.is_none()));

        app.handle_gamepad_command(crate::gamepad::Command::PrimaryDown);
        assert!(!app.show_help);
        assert!(app.quiz.is_some());
        assert!(app.modal_frame(320, 220).is_some());
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn modal_help_blocks_keyboard_gameplay_until_escape_returns() {
        let mut app = headless("numinous_app_test_modal_help_keyboard.txt");
        app.show_help = false;
        app.quiz_next();
        app.show_help = true;

        assert!(app.handle_modal_help_key(&Key::Character("a".into())));
        assert!(app.quiz.as_ref().is_some_and(|quiz| quiz.flash.is_none()));
        assert!(app.show_help);
        assert!(app.handle_modal_help_key(&Key::Named(NamedKey::Escape)));
        assert!(!app.show_help);
        assert!(app.quiz.is_some());
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn modal_help_and_zero_motion_block_wheel_state_changes() {
        let mut app = headless("numinous_app_test_modal_help_wheel.txt");
        app.show_help = false;
        app.quiz_next();
        app.show_help = true;
        app.input_mode = InputMode::Controller;
        app.t = 0.4;

        assert!(!app.apply_wheel_delta(3.0));
        assert_eq!(app.t, 0.4);
        assert_eq!(app.input_mode, InputMode::Controller);

        app.show_help = false;
        assert!(!app.apply_wheel_delta(0.0));
        assert_eq!(app.t, 0.4);
        assert_eq!(app.input_mode, InputMode::Controller);
        assert!(app.apply_wheel_delta(2.0));
        assert!((app.t - 0.44).abs() < f64::EPSILON);
        assert_eq!(app.input_mode, InputMode::KeyboardMouse);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn paused_pointer_input_cannot_touch_a_room() {
        let mut app = headless("numinous_app_test_paused_pointer.txt");
        app.show_help = false;
        app.paused = true;

        app.begin_pointer_at((0.5, 0.5));
        app.move_pointer_to((0.7, 0.5), true);

        assert!(app.inputs.is_empty());
        assert!(app.pokes.is_empty());
        assert!(!app.poking);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn controller_routes_each_game_and_every_gauntlet_stage() {
        use crate::gamepad::Command;

        let command_for = |letter: char| match letter.to_ascii_uppercase() {
            'A' => Command::Up,
            'B' => Command::Right,
            'C' => Command::Down,
            'D' => Command::Left,
            _ => panic!("choice must be A through D"),
        };

        let mut quiz = headless("numinous_app_test_controller_quiz_route.txt");
        quiz.show_help = false;
        quiz.quiz_next();
        quiz.handle_gamepad_command(Command::Right);
        assert!(quiz.quiz.as_ref().is_some_and(|play| play.flash.is_some()));
        quiz.handle_gamepad_command(Command::CycleRadio);
        assert!(quiz.radio.is_none());

        let mut munch = headless("numinous_app_test_controller_munch_route.txt");
        munch.show_help = false;
        munch.munch_start();
        munch.handle_gamepad_command(Command::Right);
        munch.handle_gamepad_command(Command::PrimaryDown);
        assert!(
            munch
                .munch
                .as_ref()
                .is_some_and(|play| play.bites.contains(&1))
        );
        munch.handle_gamepad_command(Command::CycleRadio);
        assert!(munch.munch.is_some());
        assert!(munch.radio.is_none());

        let mut nim = headless("numinous_app_test_controller_nim_route.txt");
        nim.show_help = false;
        nim.nim_start();
        let before: u32 = nim.nim.as_ref().unwrap().heaps.iter().sum();
        nim.handle_gamepad_command(Command::Right);
        nim.handle_gamepad_command(Command::PrimaryDown);
        let after: u32 = nim.nim.as_ref().unwrap().heaps.iter().sum();
        assert!(after < before);

        let mut arcade = headless("numinous_app_test_controller_arcade_route.txt");
        arcade.show_help = false;
        arcade.arcade_start();
        let before = arcade.arcade.as_ref().unwrap().run.muncher;
        arcade.handle_gamepad_command(Command::Right);
        assert_ne!(arcade.arcade.as_ref().unwrap().run.muncher, before);
        let target = {
            let run = &arcade.arcade.as_ref().unwrap().run;
            run.board
                .numbers
                .iter()
                .position(|&number| run.board.rule.fits(number))
                .expect("arcade board has an edible number")
        };
        arcade.arcade.as_mut().unwrap().run.muncher = target;
        let score = arcade.arcade.as_ref().unwrap().run.score;
        arcade.handle_gamepad_command(Command::PrimaryDown);
        assert!(
            arcade.arcade.as_ref().unwrap().run.eaten[target]
                || arcade.arcade.as_ref().unwrap().run.score > score
        );
        arcade.handle_gamepad_command(Command::CycleRadio);
        assert!(arcade.radio.is_none());
        arcade.arcade.as_mut().unwrap().over = true;
        arcade.handle_gamepad_command(Command::PrimaryDown);
        assert!(arcade.arcade.is_none());

        let mut gauntlet = headless("numinous_app_test_controller_gauntlet_route.txt");
        gauntlet.show_help = false;
        gauntlet.gauntlet_start();
        gauntlet.handle_gamepad_command(Command::Right);
        gauntlet.handle_gamepad_command(Command::PrimaryDown);
        gauntlet.handle_gamepad_command(Command::CycleRadio);
        assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 1);

        let shape = gauntlet.gauntlet.as_ref().unwrap().quiz.round.answer;
        gauntlet.handle_gamepad_command(command_for(shape));
        assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 2);
        let sky = gauntlet.gauntlet.as_ref().unwrap().scan.answer;
        gauntlet.handle_gamepad_command(command_for(sky));
        assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 3);

        let secret = gauntlet.gauntlet.as_ref().unwrap().secret.clone();
        for digit in secret {
            gauntlet.controller_digit = digit;
            gauntlet.handle_gamepad_command(Command::PrimaryDown);
        }
        gauntlet.handle_gamepad_command(Command::CycleRadio);
        assert_eq!(gauntlet.gauntlet.as_ref().unwrap().stage, 4);
        gauntlet.handle_gamepad_command(Command::PrimaryDown);
        assert!(gauntlet.gauntlet.is_none());

        let mut studio = headless("numinous_app_test_controller_studio_route.txt");
        studio.show_help = false;
        studio.studio = true;
        studio.handle_gamepad_command(Command::CycleRadio);
        assert!(studio.radio.is_none());
        studio.handle_gamepad_command(Command::Menu);
        assert!(studio.show_help);
        assert!(studio.studio);
        studio.handle_gamepad_command(Command::Right);
        assert!(studio.studio);
        studio.handle_gamepad_command(Command::PrimaryDown);
        assert!(!studio.show_help);
        assert!(studio.studio);

        for app in [&quiz, &munch, &nim, &arcade, &gauntlet, &studio] {
            let _ = std::fs::remove_file(&app.journey_file);
            let _ = std::fs::remove_file(&app.scores_file);
        }
    }

    #[test]
    fn accelerated_julia_uses_the_core_selected_constant() {
        let pokes = [(0.2, 0.8), (0.75, 0.25)];
        let expected = numinous_core::rooms::julia::selected_c(0.4, 13, &pokes);
        let actual = julia_gpu_c(0.4, 13, &pokes);
        assert!((f64::from(actual.0) - expected.0).abs() < 1e-6);
        assert!((f64::from(actual.1) - expected.1).abs() < 1e-6);
        assert_ne!(actual, julia_gpu_c(0.4, 13, &[]));
        assert!((julia_gpu_vertical_span(900, 700) - 3.2 * 700.0 / 900.0).abs() < 1e-6);
        assert_eq!(julia_gpu_vertical_span(0, 700), 0.0);
    }

    #[test]
    fn playtest_note_writes_current_session_context() {
        let mut app = headless("numinous_app_test_playtest_note.txt");
        app.journey.visit(app.rooms[app.current].meta().id);
        app.journey.play();
        app.t = 0.5;
        app.variation = 9;
        app.pokes = vec![(0.2, 0.4), (0.8, 0.1)];
        let dir = std::env::temp_dir().join("numinous_app_playtest_note");
        let _ = std::fs::remove_dir_all(&dir);

        let path = app
            .save_playtest_note_to(&dir, UNIX_EPOCH + Duration::from_secs(77))
            .expect("report saved");
        let report = std::fs::read_to_string(&path).expect("report readable");

        assert!(report.contains("Saved at Unix seconds: 77"));
        assert!(report.contains(app.rooms[app.current].meta().title));
        assert!(report.contains("Variation: 9"));
        assert!(report.contains("Poke trail: 2 point(s)"));
        assert!(report.contains("Poke points newest-last: (0.200,0.400) (0.800,0.100)"));
        assert!(report.contains("Sound: off"));
        assert!(report.contains("First unprompted whoa"));
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn playtest_shortcut_is_global_and_reports_failures() {
        use winit::keyboard::{Key, NamedKey};
        let mut app = headless("numinous_app_test_playtest_shortcut.txt");
        app.quiz_next();
        let dir = super::test_state_path("playtest-shortcut");
        let _ = std::fs::remove_dir_all(&dir);
        let input_start = Instant::now();

        assert!(app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F9),
            &dir,
            UNIX_EPOCH + Duration::from_secs(88),
            input_start,
            false,
        ));
        assert!(
            app.quiz.is_some(),
            "shortcut does not close the active mode"
        );
        let lines = app.banner.as_ref().expect("saved banner").lines();
        assert_eq!(lines[0], "PLAYTEST NOTE SAVED");
        assert!(dir.join("playtest-88.md").exists());
        assert!(app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F9),
            &dir,
            UNIX_EPOCH + Duration::from_secs(89),
            input_start + Duration::from_millis(1),
            true,
        ));
        assert!(
            !dir.join("playtest-89.md").exists(),
            "a repeated key event must not produce another file"
        );

        let blocker = super::test_state_path("playtest-blocker");
        let _ = std::fs::remove_file(&blocker);
        std::fs::write(&blocker, "not a directory").expect("blocker file");
        assert!(app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F9),
            &blocker,
            UNIX_EPOCH + Duration::from_secs(90),
            input_start + Duration::from_secs(1),
            false,
        ));
        let lines = app.banner.as_ref().expect("failure banner").lines();
        assert_eq!(lines[0], "PLAYTEST NOTE FAILED");
        assert!(lines[1].starts_with("WRITE ERROR:"));
        assert!(!app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F8),
            &dir,
            UNIX_EPOCH + Duration::from_secs(91),
            input_start + Duration::from_secs(2),
            false,
        ));

        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(blocker);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn banner_overlay_is_visible_on_the_shared_raster_path() {
        let mut app = headless("numinous_app_test_banner_overlay.txt");
        app.banner = Some(super::feedback::playtest_note(Ok(
            std::path::PathBuf::from("playtest-note.md"),
        )));

        let mut raster = numinous_core::Raster::with_accent(320, 220, [120, 220, 190]);
        let before_raster = raster.to_rgba();
        app.draw_banner_on_raster(&mut raster, 320, 220);
        assert_ne!(raster.to_rgba(), before_raster);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn volume_feedback_survives_while_radio_is_active() {
        let mut app = headless("numinous_app_test_radio_volume_banner.txt");
        app.radio = Some(0);
        app.radio_track = Arc::new(vec![0.25, -0.25, 0.5, -0.5]);

        app.change_volume(0.1);

        assert!((app.volume - 0.55).abs() < f32::EPSILON);
        let banner = app.banner.as_ref().expect("volume banner");
        assert_eq!(banner.lines()[0], "VOLUME 55%");
        assert_eq!(app.radio_track.as_slice(), [0.25, -0.25, 0.5, -0.5]);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn global_audio_keys_work_in_every_mode_without_consuming_studio_text() {
        let mut modes = Vec::new();

        let mut quiz = headless("numinous_app_test_audio_quiz.txt");
        quiz.quiz_next();
        modes.push(quiz);

        let mut munch = headless("numinous_app_test_audio_munch.txt");
        munch.munch_start();
        modes.push(munch);

        let mut nim = headless("numinous_app_test_audio_nim.txt");
        nim.nim_start();
        modes.push(nim);

        let mut gauntlet = headless("numinous_app_test_audio_gauntlet.txt");
        gauntlet.gauntlet_start();
        modes.push(gauntlet);

        let mut arcade = headless("numinous_app_test_audio_arcade.txt");
        arcade.arcade_start();
        modes.push(arcade);

        let mut paused = headless("numinous_app_test_audio_paused.txt");
        paused.paused = true;
        modes.push(paused);

        for app in &mut modes {
            let mode = app.playtest_mode();
            assert!(app.handle_global_audio_key(&Key::Character("m".into()), false));
            assert!(app.muted, "mute works in {mode}");
            assert!(app.handle_global_audio_key(&Key::Character("]".into()), false));
            assert!(
                (app.volume - 0.55).abs() < f32::EPSILON,
                "volume works in {mode}"
            );
            assert_eq!(app.playtest_mode(), mode, "audio keys preserve {mode}");
        }

        let mut studio = headless("numinous_app_test_audio_studio.txt");
        studio.enter_studio();
        let source = studio.studio_panel.source_for_test().to_string();
        assert!(studio.handle_global_audio_key(&Key::Character("m".into()), false));
        assert!(studio.handle_global_audio_key(&Key::Character("[".into()), false));
        assert_eq!(studio.studio_panel.source_for_test(), source);
        assert!(!studio.handle_global_audio_key(&Key::Character("-".into()), false));
        assert_eq!(studio.audio_program, AudioProgram::Studio);
    }

    #[test]
    fn controller_audio_commands_are_global_while_paused_and_in_studio() {
        let mut app = headless("numinous_app_test_controller_audio.txt");
        app.enter_studio();
        app.paused = true;
        let source = app.studio_panel.source_for_test().to_string();

        app.handle_gamepad_command(crate::gamepad::Command::ToggleMute);
        app.handle_gamepad_command(crate::gamepad::Command::VolumeUp);

        assert!(app.muted);
        assert!((app.volume - 0.55).abs() < f32::EPSILON);
        assert_eq!(app.audio_program, AudioProgram::Studio);
        assert_eq!(app.studio_panel.source_for_test(), source);
        assert_eq!(app.input_mode, InputMode::Controller);
    }

    #[test]
    fn controller_radio_action_works_from_the_root_help_menu() {
        let mut app = headless("numinous_app_test_controller_menu_radio.txt");
        app.show_help = true;
        app.radio = Some(numinous_core::STATIONS.len() - 1);

        app.handle_gamepad_command(crate::gamepad::Command::CycleRadio);

        assert!(app.show_help);
        assert!(app.radio.is_none());
        assert_eq!(
            app.banner.as_ref().expect("radio off banner").lines(),
            ["RADIO OFF", "ROOM MUSIC"]
        );
    }

    #[test]
    fn studio_owns_audio_until_exit_then_rejoins_live_radio() {
        let path =
            std::env::temp_dir().join(format!("numinous_studio_radio_{}.wav", std::process::id()));
        write_test_wav(&path, 2, 2);
        let mut app = headless("numinous_app_test_studio_radio.txt");
        app.radio = Some(0);
        app.radio_paths = vec![path.clone()];
        app.radio_index = 0;
        app.radio_track = Arc::new(vec![0.25, -0.25]);
        app.audio_program = AudioProgram::Radio;

        app.enter_studio();
        let selected_radio = app.radio_track.clone();
        assert_eq!(app.audio_program, AudioProgram::Studio);
        assert!(!app.sync_radio_at(1.0));
        assert!(Arc::ptr_eq(&app.radio_track, &selected_radio));
        app.update_audio();
        assert_eq!(app.audio_program, AudioProgram::Studio);

        app.exit_studio();
        assert_eq!(app.audio_program, AudioProgram::Radio);
        assert!(!app.radio_track.is_empty());
        assert!(app.radio_until.is_some());
        assert!(app.title().contains("radio:"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn failed_radio_resync_restores_the_room_score_source() {
        let mut app = headless("numinous_app_test_radio_resync_failure.txt");
        app.radio = Some(0);
        app.radio_paths = vec![std::env::temp_dir().join("numinous_missing_radio_track.wav")];
        app.radio_track = Arc::new(vec![0.25, -0.25]);
        app.radio_until = Some(Instant::now());
        app.audio_program = AudioProgram::Radio;

        assert!(!app.sync_radio_at(1.0));

        assert_eq!(app.audio_program, AudioProgram::RoomScore);
        assert!(app.radio_track.is_empty());
        assert!(app.radio_until.is_none());
        assert!(!app.title().contains("radio:"));
    }

    #[test]
    fn radio_off_restores_room_score_title_and_feedback_together() {
        let mut app = headless("numinous_app_test_radio_off.txt");
        app.radio = Some(numinous_core::STATIONS.len() - 1);
        app.radio_track = Arc::new(vec![0.25, -0.25]);
        app.audio_program = AudioProgram::Radio;
        assert!(app.title().contains("radio:"));

        app.radio = None;
        app.tune_in();

        assert_eq!(app.audio_program, AudioProgram::RoomScore);
        assert!(!app.title().contains("radio:"));
        assert_eq!(
            app.banner.as_ref().expect("radio off banner").lines(),
            ["RADIO OFF", "ROOM MUSIC"]
        );
        assert_eq!(app.audio_state().label(), "NO SOUND DEVICE");
    }

    #[test]
    fn modal_modes_take_control_from_the_show() {
        let mut app = headless("numinous_app_test_show_modes_studio.txt");
        app.the_show = true;
        app.show_help = true;
        app.show_journey = true;
        app.enter_studio();
        assert!(app.studio);
        assert!(!app.the_show);
        assert!(!app.show_help);
        assert!(!app.show_journey);
        let _ = std::fs::remove_file(&app.journey_file);

        let mut app = headless("numinous_app_test_show_modes_quiz.txt");
        app.the_show = true;
        app.quiz_next();
        assert!(app.quiz.is_some());
        assert!(!app.the_show);
        let _ = std::fs::remove_file(&app.journey_file);

        let mut app = headless("numinous_app_test_show_modes_games.txt");
        app.the_show = true;
        app.munch_start();
        assert!(app.munch.is_some());
        assert!(!app.the_show);
        app.the_show = true;
        app.nim_start();
        assert!(app.nim.is_some());
        assert!(!app.the_show);
        app.the_show = true;
        app.gauntlet_start();
        assert!(app.gauntlet.is_some());
        assert!(!app.the_show);
        app.the_show = true;
        app.arcade_start();
        assert!(app.arcade.is_some());
        assert!(!app.the_show);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn every_game_entry_releases_the_room_parameter_voice() {
        for (name, enter) in [
            (
                "numinous_app_test_parameter_voice_quiz.txt",
                App::quiz_next as fn(&mut App),
            ),
            (
                "numinous_app_test_parameter_voice_munch.txt",
                App::munch_start,
            ),
            ("numinous_app_test_parameter_voice_nim.txt", App::nim_start),
            (
                "numinous_app_test_parameter_voice_gauntlet.txt",
                App::gauntlet_start,
            ),
            (
                "numinous_app_test_parameter_voice_arcade.txt",
                App::arcade_start,
            ),
        ] {
            let mut app = headless(name);
            select_times_tables(&mut app);
            assert!(app.record_room_touch((0.375, 0.5)));
            assert!(app.desired_room_parameter_sound().is_some());

            enter(&mut app);

            assert!(app.modal_mode_active());
            assert!(app.desired_room_parameter_sound().is_none());
            let _ = std::fs::remove_file(&app.journey_file);
        }
    }

    #[test]
    fn entering_a_game_or_modal_clears_a_stale_pause() {
        // A pause set in the wander view (Space) must not leak into a game. The
        // real-time arcade is the dangerous one: a leaked pause froze the threat
        // while the player kept eating, then posted an unfair score.
        let mut app = headless("numinous_app_test_pause_clear.txt");
        for enter in [
            App::arcade_start,
            App::munch_start,
            App::nim_start,
            App::quiz_next,
            App::gauntlet_start,
            App::enter_studio,
        ] {
            app.paused = true;
            enter(&mut app);
            assert!(
                !app.paused,
                "entering a game or modal must clear a stale pause"
            );
        }
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn show_auto_advance_ignores_hidden_modal_state() {
        let mut app = headless("numinous_app_test_show_guard.txt");
        app.the_show = true;
        assert!(app.show_mode_active());
        app.studio = true;
        assert!(!app.show_mode_active());
        app.studio = false;
        app.quiz_next();
        app.the_show = true;
        assert!(!app.show_mode_active());
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn modal_frames_take_priority_over_gpu_eligible_rooms() {
        let mut app = headless("numinous_app_test_modal_frame_priority.txt");
        app.show_help = false;
        app.current = app
            .rooms
            .iter()
            .position(|room| room.meta().id == "mandelbrot")
            .expect("mandelbrot room");
        app.quiz_next();

        let raster = app.modal_frame(320, 220).expect("modal frame");

        assert!(app.modal_mode_active());
        assert!(raster.lit_count() > 100);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn munch_in_the_window_grades_and_posts() {
        let mut app = headless("numinous_app_test_munch.txt");
        app.munch_start();
        let first_round = app.munch.as_ref().unwrap().round;
        assert!(
            first_round >= 4,
            "standalone Munch opens the full rule deck"
        );
        assert_eq!(app.journey.plays, 1, "a dealt board is a play");
        {
            let play = app.munch.as_mut().unwrap();
            play.cursor = 3;
            play.bites.insert(3);
            play.bites.insert(7);
        }
        app.munch_grade();
        let outcome = app.munch.as_ref().unwrap().graded.as_ref().unwrap();
        assert_eq!(outcome.hits + outcome.bad_bites, 2, "two bites graded");
        app.munch_grade(); // grading twice changes nothing
        assert_eq!(app.journey.plays, 1);
        let scores = std::fs::read_to_string(&app.scores_file).expect("score persisted");
        assert!(scores.contains(&format!("board:{first_round}")));
        let _ = std::fs::remove_file(&app.journey_file);
        let _ = std::fs::remove_file(&app.scores_file);
    }

    #[test]
    fn munch_key_routes_shared_controls() {
        use winit::keyboard::{Key, NamedKey};
        let mut app = headless("numinous_app_test_munch_keys.txt");
        app.munch_start();
        app.munch_key(&Key::Character("d".into()));
        assert_eq!(app.munch.as_ref().unwrap().cursor, 1);
        app.munch_key(&Key::Character("e".into()));
        assert!(app.munch.as_ref().unwrap().bites.contains(&1));
        app.munch_key(&Key::Named(NamedKey::Space));
        assert!(!app.munch.as_ref().unwrap().bites.contains(&1));
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn leaving_munch_or_gauntlet_retires_queued_transient_audio() {
        let mut app = headless("numinous_app_test_transient_audio_exit.txt");

        app.munch_start();
        let before_ungraded_exit = app.transient_audio_clears.get();
        app.munch_key(&Key::Named(NamedKey::Escape));
        assert!(app.munch.is_none());
        assert_eq!(app.transient_audio_clears.get(), before_ungraded_exit + 1);

        app.munch_start();
        app.munch_grade();
        let before_graded_exit = app.transient_audio_clears.get();
        app.munch_key(&Key::Named(NamedKey::Escape));
        assert!(app.munch.is_none());
        assert_eq!(app.transient_audio_clears.get(), before_graded_exit + 1);

        app.gauntlet_start();
        let before_gauntlet_exit = app.transient_audio_clears.get();
        app.gauntlet_key(&Key::Named(NamedKey::Escape));
        assert!(app.gauntlet.is_none());
        assert_eq!(app.transient_audio_clears.get(), before_gauntlet_exit + 1);

        app.gauntlet_start();
        app.gauntlet.as_mut().expect("active Gauntlet").stage = 4;
        let before_completed_gauntlet_exit = app.transient_audio_clears.get();
        app.gauntlet_key(&Key::Named(NamedKey::Enter));
        assert!(app.gauntlet.is_none());
        assert_eq!(
            app.transient_audio_clears.get(),
            before_completed_gauntlet_exit + 1
        );

        let _ = std::fs::remove_file(&app.journey_file);
        let _ = std::fs::remove_file(&app.scores_file);
    }

    #[test]
    fn graded_munch_advances_only_on_enter_or_space() {
        use winit::keyboard::{Key, NamedKey};
        let mut app = headless("numinous_app_test_munch_next.txt");
        app.munch_start();
        let first_round = app.munch.as_ref().unwrap().round;
        let first_rule = app.munch.as_ref().unwrap().board.rule;
        app.munch_grade();

        app.munch_key(&Key::Character("x".into()));
        assert_eq!(app.munch.as_ref().unwrap().round, first_round);
        app.munch_key(&Key::Named(NamedKey::Enter));

        let next = app.munch.as_ref().expect("next board remains in Munch");
        assert!(next.round > first_round);
        assert!(!super::play::same_rule_family(first_rule, next.board.rule));
        assert_eq!(app.journey.plays, 2);
        let _ = std::fs::remove_file(&app.journey_file);
        let _ = std::fs::remove_file(&app.scores_file);
    }

    #[test]
    fn nim_in_the_window_plays_the_order() {
        let mut app = headless("numinous_app_test_nim.txt");
        app.nim_start();
        let before: u32 = app.nim.as_ref().unwrap().heaps.iter().sum();
        {
            let play = app.nim.as_mut().unwrap();
            play.take = 1;
        }
        app.nim_move();
        let play = app.nim.as_ref().unwrap();
        let after: u32 = play.heaps.iter().sum();
        // Your stone and the Order's reply both left the board (unless over).
        assert!(after < before);
        assert!(play.over.is_none() || play.over == Some(false) || play.over == Some(true));
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn nim_result_requires_an_explicit_retry_or_exit() {
        let mut app = headless("numinous_app_test_nim_retry.txt");
        app.nim_start();
        app.nim.as_mut().unwrap().over = Some(false);
        let plays = app.journey.plays;

        app.nim_key(&winit::keyboard::Key::Character("x".into()));
        assert!(
            app.nim.is_some(),
            "an unrelated key must not eject the result"
        );
        assert_eq!(app.journey.plays, plays);

        app.nim_key(&winit::keyboard::Key::Named(
            winit::keyboard::NamedKey::Enter,
        ));
        assert_eq!(app.nim.as_ref().unwrap().over, None);
        assert_eq!(app.journey.plays, plays + 1);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn the_live_arcade_acts_beats_and_ends() {
        use numinous_core::munch_arcade::Action;
        let mut app = headless("numinous_app_test_arcade.txt");
        app.arcade_start();
        assert_eq!(app.journey.plays, 1);
        app.arcade_act(Action::Right);
        app.arcade_act(Action::Eat);
        let before = app.arcade.as_ref().unwrap().run.vexations.clone();
        app.arcade_beat();
        let after = &app.arcade.as_ref().unwrap().run.vexations;
        assert!(
            before
                .iter()
                .zip(after.iter())
                .any(|(b, a)| b.cell != a.cell),
            "the beat moves spirits"
        );
        // Beat until the spirits finish the job; the run must end and score.
        for _ in 0..500 {
            app.arcade_beat();
            if app.arcade.as_ref().unwrap().over {
                break;
            }
        }
        assert!(
            app.arcade.as_ref().unwrap().over,
            "the spirits always win eventually"
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn the_gauntlet_runs_four_stages_and_totals_with_combo() {
        use winit::keyboard::{Key, NamedKey};
        let mut app = headless("numinous_app_test_gauntlet.txt");
        app.gauntlet_start();
        // Stage 1: submit an empty munch board (0 points, not clean).
        app.gauntlet_key(&Key::Named(NamedKey::Enter));
        assert_eq!(app.gauntlet.as_ref().unwrap().stage, 1);
        // Stage 2: answer the shape correctly.
        let answer = app.gauntlet.as_ref().unwrap().quiz.round.answer;
        app.gauntlet_key(&Key::Character(answer.to_string().to_lowercase().into()));
        let run = app.gauntlet.as_ref().unwrap();
        assert_eq!(run.stage, 2);
        assert_eq!(run.scores[1], 25);
        assert!(run.cleared[1]);
        // Stage 3: answer the sky correctly.
        let sky = app.gauntlet.as_ref().unwrap().scan.answer;
        app.gauntlet_key(&Key::Character(sky.to_string().to_lowercase().into()));
        assert_eq!(app.gauntlet.as_ref().unwrap().stage, 3);
        // Stage 4: cut the right wire first try.
        let code: String = app
            .gauntlet
            .as_ref()
            .unwrap()
            .secret
            .iter()
            .map(|&d| char::from(b'0' + d))
            .collect();
        for ch in code.chars() {
            app.gauntlet_key(&Key::Character(ch.to_string().into()));
        }
        app.gauntlet_key(&Key::Named(NamedKey::Enter));
        let run = app.gauntlet.as_ref().unwrap();
        assert_eq!(run.stage, 4, "the run is complete");
        // Scores: 0 (miss), then 25*1, 25*2, 40*3 = 195.
        assert_eq!(super::gauntlet_total(&run.scores, &run.cleared), 195);
        assert_eq!(app.journey.plays, 4);
        assert_eq!(app.journey.wins, 3);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn gauntlet_munch_stage_routes_shared_controls() {
        use winit::keyboard::{Key, NamedKey};
        let mut app = headless("numinous_app_test_gauntlet_munch_keys.txt");
        app.gauntlet_start();
        app.gauntlet_key(&Key::Character("d".into()));
        assert_eq!(app.gauntlet.as_ref().unwrap().munch.cursor, 1);
        app.gauntlet_key(&Key::Character("e".into()));
        assert!(app.gauntlet.as_ref().unwrap().munch.bites.contains(&1));
        app.gauntlet_key(&Key::Named(NamedKey::Space));
        assert!(!app.gauntlet.as_ref().unwrap().munch.bites.contains(&1));
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn the_radio_loads_cached_tracks_and_joins_live() {
        let dir = std::env::temp_dir().join("numinous_radio_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("trance-001.wav");
        write_test_wav(&path, 1, 3);
        assert!(radio_cache::audio_is_bounded(&path));
        let duration = radio_cache::duration_seconds(&path).expect("duration");
        assert!(
            (2.9..=3.1).contains(&duration),
            "duration should be about three seconds, got {duration}"
        );
        // SAFETY-free env override: the test sets the var via a scratch app
        // field instead. tune_in reads NUMINOUS_RADIO; set through the
        // process env is forbidden, so exercise radio_play directly.
        let mut app = headless("numinous_app_test_radio.txt");
        app.radio = Some(0);
        app.radio_paths = vec![path.clone()];
        app.radio_index = 0;
        assert!(app.radio_play(1.0));
        assert!(
            app.radio_track.len() > 44_100 * 2,
            "the record is loaded ({} samples)",
            app.radio_track.len()
        );
        assert!(app.radio_until.is_some(), "rotation is armed");
        assert!(
            app.radio_track.iter().any(|&s| s.abs() > 0.1),
            "the record has music in it"
        );
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn radio_resync_selects_the_wall_clock_track_after_an_inactive_gap() {
        let dir = std::env::temp_dir().join("numinous_radio_resync_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let first = dir.join("trance-001.wav");
        let second = dir.join("trance-002.wav");
        write_test_wav(&first, 1, 2);
        write_test_wav(&second, 1, 2);

        let mut app = headless("numinous_app_test_radio_resync.txt");
        app.radio = Some(0);
        app.radio_paths = vec![first, second];
        assert!(app.sync_radio_at(2.5));
        assert_eq!(app.radio_index, 1);
        assert!(app.radio_until.is_some());
        assert!(app.sync_radio_at(8.25));
        assert_eq!(app.radio_index, 0);

        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn radio_duration_uses_frames_for_stereo_tracks() {
        let dir = std::env::temp_dir().join("numinous_radio_stereo_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("stereo.wav");
        write_test_wav(&path, 2, 3);

        let duration = radio_cache::duration_seconds(&path).expect("duration");

        assert!(
            (2.9..=3.1).contains(&duration),
            "duration should be about three seconds, got {duration}"
        );
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn oversized_radio_files_are_rejected_before_loading() {
        let path = std::env::temp_dir().join("numinous_radio_oversized.wav");
        let file = std::fs::File::create(&path).expect("oversized placeholder");
        file.set_len(radio_cache::MAX_AUDIO_BYTES + 1)
            .expect("make sparse oversized file");
        assert!(!radio_cache::audio_is_bounded(&path));
        assert!(radio_cache::duration_seconds(&path).is_none());

        let mut app = headless("numinous_app_test_radio_oversized.txt");
        app.radio_paths = vec![path.clone()];
        app.radio_index = 0;
        app.radio_track = Arc::new(vec![0.25, -0.25]);
        app.radio_until = Some(std::time::Instant::now());
        assert!(!app.radio_play(0.0));
        assert!(app.radio_track.is_empty());
        assert!(app.radio_until.is_none());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn radio_rotation_recovers_from_a_bad_cached_file() {
        let dir = std::env::temp_dir().join("numinous_radio_recovery_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create dir");
        let bad = dir.join("trance-bad.wav");
        let good = dir.join("trance-good.wav");
        std::fs::write(&bad, b"not actually a wav").expect("bad wav");
        write_test_wav(&good, 1, 2);

        let mut app = headless("numinous_app_test_radio_recovery.txt");
        app.radio = Some(0);
        app.radio_paths = vec![bad, good.clone()];
        app.radio_index = 0;

        assert!(app.radio_play_or_advance(0.0));
        assert_eq!(app.radio_paths[app.radio_index], good);
        assert!(!app.radio_track.is_empty());
        assert!(app.radio_until.is_some());

        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn modal_contexts_clear_stale_pointer_state() {
        let mut app = headless("numinous_app_test_pointer_state.txt");
        app.poking = true;
        app.show_help = true;
        app.refresh_pointer_state();
        assert!(!app.poking);

        app.show_help = false;
        app.dragging = true;
        app.studio = true;
        app.refresh_pointer_state();
        assert!(!app.dragging);

        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn quiz_answers_letter_matches_a_choice() {
        let mut app = headless("numinous_app_test_letters.txt");
        app.quiz_next();
        let quiz = app.quiz.as_ref().unwrap();
        assert!(
            quiz.round
                .choices
                .iter()
                .any(|c| c.letter == quiz.round.answer)
        );
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn quiz_deal_rules_stay_out_of_the_event_loop_coordinator() {
        let source = include_str!("main.rs");

        assert!(source.contains("play::deal_quiz"));
        assert!(source.contains("play::answer_quiz"));
        assert!(!source.contains(concat!("I", "CONIC")));
        assert!(!source.contains(concat!("build", "_round", "_pool")));
        assert!(!source.contains(concat!("quiz_recent", ".", "push")));
    }
}
