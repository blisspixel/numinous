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
use std::time::SystemTime;

use numinous_core::{Journey, Raster, Room, Surface, all_rooms_with};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Icon, Window, WindowId};

mod controls;
mod feedback;
mod game_draw;
mod hud;
mod live_render;
mod mouse_input;
mod overlays;
mod play;
mod playtest;
mod postcard;
mod radio_cache;
mod room_input;
mod studio_panel;

use play::{ArcadePlay, GauntletPlay, MunchPlay, NimPlay, QuizPlay, gauntlet_total};

/// Near-black background (matches the `Raster` stage), packed `0x00RRGGBB`.
const BACKGROUND: u32 = 0x000A_0B0F;

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

/// How far the phase advances each frame.
const T_STEP: f64 = 0.004;
/// In The Show, how far the phase advances each frame (slower, hypnotic).
const SHOW_T_STEP: f64 = 0.0016;

/// The application state driven by the winit event loop.
struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    player: Option<numinous_audio::LoopPlayer>,
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
    /// GPU fractal renderer, when this machine has one (CPU raster otherwise).
    gpu: Option<numinous_gpu::FractalRenderer>,
    /// Adaptive live-render resolution for CPU room frames (see live_render).
    live_scale: live_render::LiveScale,
    /// The visual era ('e' cycles: phosphor, 8-bit, vector, modern).
    era: numinous_core::Era,
    /// Sound off ('m' toggles).
    muted: bool,
    /// Master volume, 0.0 to 1.0 ('-' and '=' step it).
    volume: f32,
    /// The help overlay ('h' toggles; shown at launch so nobody is lost).
    show_help: bool,
    /// Start in fullscreen (from --fullscreen / -f arg or env). Supports user's request for full screen view.
    start_fullscreen: bool,
    /// Frame counter, used to refresh the audio as the phase sweeps.
    frame: u64,
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
    /// The chiptune bed for the current room, rendered once per room.
    tune: Vec<f32>,
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
    /// The radio: Some(index into STATIONS) when a cached station plays.
    radio: Option<usize>,
    /// The loaded station track, if any.
    radio_track: Vec<f32>,
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
            rooms: all_rooms_with(0),
            current: 0,
            t: 0.0,
            paused: false,
            dragging: false,
            show_info: false,
            the_show: false,
            studio: false,
            studio_panel: studio_panel::StudioPanel::default(),
            gpu: None,
            live_scale: live_render::LiveScale::new(),
            era: numinous_core::Era::default(),
            muted: false,
            volume: 0.45,
            show_help: true,
            start_fullscreen: false,
            frame: 0,
            time_scale: 1.0,
            journey: journey.clone(),
            journey_saved: journey,
            level_seen: 1,
            banner: None,
            quiz: None,
            quiz_recent: Vec::new(),
            munch: None,
            munch_next_round: numinous_core::FULL_DECK_ROUND,
            munch_last_rule: None,
            nim: None,
            gauntlet: None,
            arcade: None,
            tune: Vec::new(),
            show_journey: false,
            mouse: (0.0, 0.0),
            pokes: Vec::new(),
            inputs: Vec::new(),
            poking: false,
            variation: 0,
            room_card: room_input::ROOM_CARD_FRAMES,
            radio: None,
            radio_track: Vec::new(),
            radio_paths: Vec::new(),
            radio_index: 0,
            radio_until: None,
            journey_file,
            scores_file,
        }
    }

    /// Persist the journey and raise the LEVEL UP banner when the level moves.
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
        });
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
                    key if controls::apply_munch_control(
                        &mut play.cursor,
                        &mut play.bites,
                        key,
                    ) => {}
                    _ => {}
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
                self.update_audio();
            }
            Key::Named(NamedKey::Enter) => self.munch_grade(),
            key => {
                if let Some(play) = &mut self.munch {
                    let _ = controls::apply_munch_control(&mut play.cursor, &mut play.bites, key);
                }
            }
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
        postcard::write_room_postcard(
            self.rooms[self.current].as_ref(),
            self.t,
            &self.inputs,
            self.era,
            &postcard::default_postcard_dir(),
        )
        .ok()
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
        if self.studio {
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
        self.studio_reparse();
    }

    fn modal_mode_active(&self) -> bool {
        self.studio
            || self.quiz.is_some()
            || self.munch.is_some()
            || self.nim.is_some()
            || self.gauntlet.is_some()
            || self.arcade.is_some()
    }

    fn show_mode_active(&self) -> bool {
        self.the_show && !self.modal_mode_active()
    }

    fn left_press_context(&self) -> mouse_input::LeftPressContext {
        mouse_input::LeftPressContext {
            game_click_mode: self.munch.is_some() || self.quiz.is_some(),
            studio: self.studio,
            show_help: self.show_help,
            show_journey: self.show_journey,
            arcade: self.arcade.is_some(),
            nim: self.nim.is_some(),
            gauntlet: self.gauntlet.is_some(),
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
            room_input::cancel_open_gesture(&mut self.inputs);
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

    fn handle_playtest_shortcut(&mut self, key: &Key) -> bool {
        if !matches!(key, Key::Named(NamedKey::F9)) {
            return false;
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
        now: SystemTime,
    ) -> bool {
        if !matches!(key, Key::Named(NamedKey::F9)) {
            return false;
        }
        let result = self.save_playtest_note_to(dir, now);
        self.set_playtest_note_banner(result);
        true
    }

    fn set_playtest_note_banner(&mut self, result: std::io::Result<std::path::PathBuf>) {
        self.banner = Some(feedback::playtest_note(result));
    }

    fn change_volume(&mut self, step: f32) {
        self.volume = (self.volume + step).clamp(0.0, 1.0);
        self.banner = Some(feedback::volume(self.volume));
        if !self.refresh_radio_audio() {
            self.update_audio();
        }
    }

    /// A click lands in the games: munch toggles the cell, the quiz answers.
    fn click(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        let (width, height) = (size.width as f64, size.height as f64);
        if width < 1.0 || height < 1.0 {
            return;
        }
        let (mx, my) = self.mouse;
        if let Some(play) = &mut self.munch {
            if play.graded.is_some() {
                return;
            }
            if let Some(cell) =
                game_draw::MunchLayout::new(size.width as usize, size.height as usize).hit(mx, my)
            {
                play.cursor = cell;
                controls::toggle_munch_bite(&mut play.bites, cell);
            }
            return;
        }
        if let Some(quiz) = &self.quiz {
            if quiz.flash.is_some() {
                self.quiz_next();
                return;
            }
            let layout = game_draw::QuizChoiceLayout::new(
                size.width as usize,
                size.height as usize,
                quiz.round.choices.len(),
            );
            if let Some(index) = layout.hit(my, quiz.round.choices.len())
                && let Some(choice) = quiz.round.choices.get(index)
            {
                let letter = choice.letter;
                self.quiz_answer(letter);
            }
        }
    }

    /// Tune in to the current dial position: build the playlist, join the
    /// broadcast mid-stream (the station was always on the air), and play.
    fn tune_in(&mut self) {
        self.radio_track.clear();
        self.radio_paths.clear();
        self.radio_until = None;
        let Some(i) = self.radio else {
            self.update_audio();
            return;
        };
        let st = &numinous_core::STATIONS[i];
        let dir = radio_cache::default_dir();
        self.radio_paths = radio_cache::station_tracks(&dir, st.id);
        // Join the broadcast live: the wall clock decides which track is on.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        if let Some((index, position)) = radio_cache::live_position(&self.radio_paths, now) {
            self.radio_index = index;
            self.radio_play_or_advance(position);
        }
        if let Some(window) = &self.window {
            let st = &numinous_core::STATIONS[i];
            window.set_title(&format!(
                "Numinous  |  radio: {}{}",
                st.name,
                if self.radio_paths.is_empty() {
                    "  (no tracks yet: numinous tune2)"
                } else {
                    ""
                }
            ));
        }
        // The dial speaks on screen, especially when the station is silent.
        let st = &numinous_core::STATIONS[i];
        self.banner = Some(feedback::radio(st.name, st.id, self.radio_paths.len()));
        self.update_audio();
    }

    fn radio_play_or_advance(&mut self, offset: f64) -> bool {
        let track_count = self.radio_paths.len();
        if track_count == 0 {
            self.radio_track.clear();
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
        self.radio_track.clear();
        self.radio_until = None;
        false
    }

    /// Put the current playlist entry on the air, starting `offset` seconds
    /// in: read it (mono or stereo), resample to the device's rate so pitch
    /// and tempo are true, and hand it to the player once.
    fn radio_play(&mut self, offset: f64) -> bool {
        self.radio_track.clear();
        self.radio_until = None;
        let Some(path) = self.radio_paths.get(self.radio_index) else {
            return false;
        };
        let device_rate = self.player.as_ref().map_or(44_100, |p| p.sample_rate());
        let Some(loaded) = radio_cache::load_track(path, offset, device_rate) else {
            return false;
        };
        self.radio_track = loaded.stereo;
        self.radio_until = Some(std::time::Instant::now() + loaded.remaining);
        if !self.muted
            && let Some(player) = &self.player
        {
            let volume = self.volume;
            player.set_stereo(self.radio_track.iter().map(|&s| s * volume).collect());
        }
        true
    }

    fn refresh_radio_audio(&self) -> bool {
        if self.radio.is_none() || self.radio_track.is_empty() {
            return false;
        }
        let Some(player) = &self.player else {
            return true;
        };
        if self.muted {
            player.set_samples(Vec::new());
        } else {
            let volume = self.volume;
            player.set_stereo(self.radio_track.iter().map(|&s| s * volume).collect());
        }
        true
    }

    /// GPU-render the current room if it has a real-time GPU path (the deep
    /// fractal zooms), returning the RGBA frame; `None` means draw on the CPU.
    fn gpu_frame(&mut self, width: usize, height: usize) -> Option<Vec<u8>> {
        let id = self.rooms[self.current].meta().id;
        let (w, h) = (width as u32, height as u32);
        let mandelbrot_view = (id == "mandelbrot")
            .then(|| mandelbrot_gpu_view(self.t, self.variation, w, h, &self.inputs));
        let julia_c = (id == "julia").then(|| julia_gpu_c(self.t, self.variation, &self.pokes));
        let gpu = self.gpu.as_mut()?;
        match id {
            "mandelbrot" => {
                let (center_x, center_y, scale) = mandelbrot_view?;
                Some(gpu.render(
                    w,
                    h,
                    center_x,
                    center_y,
                    scale,
                    numinous_core::rooms::FRACTAL_MAX_ITER,
                    numinous_gpu::Fractal::Mandelbrot,
                ))
            }
            "julia" => {
                let (cx, cy) = julia_c?;
                let c = numinous_gpu::Fractal::Julia { cx, cy };
                Some(gpu.render(
                    w,
                    h,
                    0.0,
                    0.0,
                    julia_gpu_vertical_span(w, h),
                    numinous_core::rooms::FRACTAL_MAX_ITER,
                    c,
                ))
            }
            _ => None,
        }
    }

    fn studio_reparse(&mut self) {
        let spec = self.studio_panel.reparse();
        self.set_studio_sound(spec);
    }

    fn set_studio_sound(&self, spec: Option<numinous_core::SoundSpec>) {
        let Some(player) = &self.player else {
            return;
        };
        if self.muted {
            player.set_samples(Vec::new());
            return;
        }
        if let Some(spec) = spec {
            let volume = self.volume;
            player.set_samples(
                spec.render(player.sample_rate())
                    .into_iter()
                    .map(|sample| sample * volume)
                    .collect(),
            );
        }
    }

    fn exit_studio(&mut self) {
        self.studio = false;
        if self.radio.is_some() && !self.radio_track.is_empty() {
            if !self.muted
                && let Some(player) = &self.player
            {
                let volume = self.volume;
                player.set_stereo(self.radio_track.iter().map(|&s| s * volume).collect());
            }
        } else {
            self.update_audio();
        }
    }

    /// Render the current room's sound at the current phase and send it to the
    /// looping player, so the room you see is the room you hear.
    fn update_audio(&mut self) {
        let Some(player) = &self.player else {
            return;
        };
        if self.muted {
            player.set_samples(Vec::new());
            return;
        }
        let spec = self.rooms[self.current].sound(self.t);
        let tone: Vec<f32> = spec
            .render(player.sample_rate())
            .into_iter()
            .map(|s| s * 0.5)
            .collect();
        // Engine A is the room bed while radio is off. Radio v1 owns the
        // player buffer while it is on so long records do not restart.
        if self.tune.is_empty() {
            // The room's own phrase when it has one; the seeded chip otherwise.
            let pattern = match self.rooms[self.current].motif() {
                Some(motif) => motif.pattern(),
                None => numinous_core::compose(self.current as u64 + 1, 8),
            };
            self.tune = pattern.render(player.sample_rate());
        }
        if self.radio.is_some() && !self.radio_track.is_empty() {
            // The station is the sound, and radio_play already handed the
            // record to the player. Full one-bus room-over-radio mixing needs
            // a non-restarting overlay path in the player.
            return;
        }
        let mut mix = self.tune.clone();
        if !tone.is_empty() {
            for (i, sample) in mix.iter_mut().enumerate() {
                *sample = (*sample * 0.55 + tone[i % tone.len()] * 0.45).clamp(-1.0, 1.0);
            }
        }
        let volume = self.volume;
        player.set_samples(mix.into_iter().map(|s| s * volume).collect());
    }

    fn title(&self) -> String {
        if self.the_show {
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
        room_input::reset_room_view(
            &mut self.t,
            &mut self.room_card,
            &mut self.pokes,
            &mut self.inputs,
        );
        self.tune.clear();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.visit_current();
        self.update_audio();
    }

    fn reset_current_room(&mut self) {
        room_input::reset_room_view(
            &mut self.t,
            &mut self.room_card,
            &mut self.pokes,
            &mut self.inputs,
        );
        self.update_audio();
    }

    fn draw_studio(&self, raster: &mut Raster, width: usize, height: usize) {
        self.studio_panel.draw(raster, width, height, self.t);
    }

    fn modal_frame(&self, width: usize, height: usize) -> Option<Raster> {
        if let Some(play) = &self.arcade {
            Some(game_draw::draw_arcade(play, width, height))
        } else if let Some(run) = &self.gauntlet {
            Some(game_draw::draw_gauntlet(
                &self.rooms,
                run,
                self.frame,
                width,
                height,
            ))
        } else if let Some(play) = &self.munch {
            Some(game_draw::draw_munch(play, self.frame, width, height))
        } else if let Some(play) = &self.nim {
            Some(game_draw::draw_nim(play, width, height))
        } else {
            self.quiz
                .as_ref()
                .map(|quiz| game_draw::draw_quiz(&self.rooms, quiz, width, height))
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
            room.render_input(&mut raster, self.t, &self.inputs);
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
        hud::draw_room_chrome(
            raster,
            room,
            &hud::RoomChrome {
                t: self.t,
                room_card: self.room_card,
                show_info: self.show_info,
                show_help: self.show_help,
                show_journey: self.show_journey,
                banner_active: self.banner.is_some(),
                the_show: self.the_show,
                studio: self.studio,
                muted: self.muted,
                level: self.journey.level(),
            },
            &self.inputs,
            width,
            height,
        );

        if self.show_help && !self.the_show {
            overlays::draw_help_overlay(raster, width, height);
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
            );
        }
    }

    fn present_raster(&mut self, mut raster: Raster, width: usize, height: usize) {
        self.draw_banner_on_raster(&mut raster, width, height);
        let (rw, rh) = (raster.width(), raster.height());
        let mut rgba = raster.to_rgba();
        self.era.apply(&mut rgba, rw, rh);
        self.blit(&rgba, rw, rh, width, height);
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
        if self.window.is_some() {
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
        if self.start_fullscreen {
            if let Some(w) = &self.window {
                w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
        }
        self.player = match numinous_audio::LoopPlayer::new() {
            Ok(player) => Some(player),
            Err(error) => {
                // Silence must never be a mystery: say it on screen and in
                // the crash log, then keep running visual-only.
                self.banner = Some(feedback::sound_device_unavailable(&error));
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                let path = std::path::PathBuf::from(home).join(".numinous-crash.log");
                use std::io::Write as _;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = file.write_all(
                        format!(
                            "audio open failed: {error}
"
                        )
                        .as_bytes(),
                    );
                }
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
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
                        ..
                    },
                ..
            } => {
                self.clear_pointer_state();
                if self.handle_playtest_shortcut(&logical_key) {
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
                        Key::Named(NamedKey::Backspace) => {
                            let spec = self.studio_panel.backspace();
                            self.set_studio_sound(spec);
                        }
                        Key::Named(NamedKey::Space) => {
                            self.studio_panel.push_space();
                        }
                        Key::Character(s) => {
                            let spec = self.studio_panel.push_text(&s);
                            self.set_studio_sound(spec);
                        }
                        _ => {}
                    }
                } else {
                    match logical_key {
                        // Esc is the menu, like every game since Doom. Quit from
                        // the window's close button.
                        Key::Named(NamedKey::Escape) => {
                            if self.the_show {
                                self.the_show = false;
                                self.show_help = false;
                                if let Some(window) = &self.window {
                                    window.set_title(&self.title());
                                }
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
                        Key::Named(NamedKey::Space) => self.paused = !self.paused,
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
                        // Volume steps: '-' softer, '=' louder.
                        Key::Character(c) if c.as_str() == "-" || c.as_str() == "=" => {
                            let step = if c.as_str() == "-" { -0.1 } else { 0.1 };
                            self.change_volume(step);
                        }
                        Key::Character(c) if c.as_str() == "m" => {
                            self.muted = !self.muted;
                            if !self.muted && self.radio.is_some() {
                                self.tune_in();
                            } else {
                                self.update_audio();
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
                            if self.the_show {
                                self.the_show = false;
                                if let Some(window) = &self.window {
                                    window.set_title(&self.title());
                                }
                            }
                            self.show_journey = !self.show_journey;
                        }
                        // Y turns the radio dial: off, then station by station.
                        Key::Character(c) if c.as_str() == "y" => {
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
                            if let Some(path) = self.save_postcard()
                                && let Some(window) = &self.window
                            {
                                window.set_title(&format!(
                                    "Numinous  |  postcard saved: {}",
                                    path.display()
                                ));
                            }
                        }
                        // B for the big show (lean back).
                        Key::Character(c) if c.as_str() == "b" => {
                            self.the_show = !self.the_show;
                            if self.the_show {
                                self.show_help = false;
                                self.show_journey = false;
                            }
                            self.paused = false;
                            if let Some(window) = &self.window {
                                window.set_title(&self.title());
                            }
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
                                room_input::reset_room_view(
                                    &mut self.t,
                                    &mut self.room_card,
                                    &mut self.pokes,
                                    &mut self.inputs,
                                );
                                self.tune.clear();
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
                if state == ElementState::Pressed {
                    let action = mouse_input::left_press_action(self.left_press_context());
                    self.set_pointer_state(mouse_input::pointer_state_after_left_press(action));
                    match action {
                        mouse_input::LeftPressAction::GameClick => self.click(),
                        mouse_input::LeftPressAction::RoomPoke => {
                            // The poke: the room answers the hand, and keeps
                            // answering while the hand drags.
                            self.poking = true;
                            if let Some(window) = &self.window {
                                let size = window.inner_size();
                                if let Some(point) = mouse_input::normalized_window_point(
                                    self.mouse,
                                    (size.width, size.height),
                                ) {
                                    room_input::push_poke(&mut self.pokes, point);
                                    room_input::record_pointer_down(
                                        &mut self.inputs,
                                        point,
                                        self.t,
                                    );
                                } else {
                                    self.poking = false;
                                }
                            } else {
                                self.poking = false;
                            }
                        }
                        mouse_input::LeftPressAction::PhaseDrag => {}
                        mouse_input::LeftPressAction::Ignore => {}
                    }
                } else {
                    // Record the lift before the state change, so the
                    // gesture completes as a release rather than a cancel.
                    if self.poking
                        && let Some(window) = &self.window
                    {
                        let size = window.inner_size();
                        if let Some(point) = mouse_input::normalized_window_point(
                            self.mouse,
                            (size.width, size.height),
                        ) {
                            room_input::record_pointer_up(&mut self.inputs, point, self.t);
                        }
                    }
                    self.set_pointer_state(mouse_input::pointer_state_after_left_release());
                }
            }
            WindowEvent::Focused(false) => self.clear_pointer_state(),
            WindowEvent::MouseWheel { delta, .. } if !self.studio => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
                };
                self.t = (self.t + lines * 0.02).rem_euclid(1.0);
                self.update_audio();
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
                        if room_input::extend_poke_trail(&mut self.pokes, point) {
                            room_input::record_pointer_move(&mut self.inputs, point, self.t);
                        }
                    } else {
                        // The window lost its size mid-drag: the gesture
                        // ends without a lift, so close it gently.
                        room_input::cancel_open_gesture(&mut self.inputs);
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.refresh_pointer_state();
        if !self.paused && !self.dragging {
            let show_active = self.show_mode_active();
            let base = if show_active { SHOW_T_STEP } else { T_STEP };
            let step = base * self.time_scale;
            if self.t + step >= 1.0 {
                self.t = 0.0;
                // In The Show, a finished sweep drifts into the next room.
                if show_active {
                    self.switch(1);
                }
            } else {
                self.t += step;
            }
            // The sound follows the sweep instead of droning on one tone.
            self.frame += 1;
            // The room's voice follows the sweep, but never while the radio
            // is on the air: resetting the loop buffer would restart the
            // record every couple of seconds.
            if self.frame % 120 == 0
                && !self.studio
                && self.radio.is_none()
                && self.quiz.is_none()
                && self.munch.is_none()
                && self.nim.is_none()
                && self.gauntlet.is_none()
                && self.arcade.is_none()
            {
                self.update_audio();
            }
            // The station rotates: when a track ends, the next takes the air.
            if self.radio.is_some()
                && let Some(until) = self.radio_until
                && std::time::Instant::now() >= until
                && !self.radio_paths.is_empty()
            {
                self.radio_index = (self.radio_index + 1) % self.radio_paths.len();
                self.radio_play_or_advance(0.0);
                self.update_audio();
            }
            room_input::tick_room_card(&mut self.room_card);
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
                if !play.over && self.frame % interval == 0 {
                    self.arcade_beat();
                }
            }
            if self.banner.as_mut().is_some_and(|banner| !banner.tick()) {
                self.banner = None;
            }
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
    let mut pixels = vec![0; reader.output_buffer_size()];
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
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let path = std::path::PathBuf::from(home).join(".numinous-crash.log");
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());
        let entry = format!(
            "panic at {location}: {info}
"
        );
        use std::io::Write as _;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = file.write_all(entry.as_bytes());
        }
    }));
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
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
        App, TestStateRoot, app_icon, julia_gpu_c, julia_gpu_vertical_span, mandelbrot_gpu_view,
        radio_cache,
    };
    use std::time::{Duration, UNIX_EPOCH};

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
        // Focus loss and modal opens route through set_pointer_state, which
        // must close the open gesture gently.
        app.clear_pointer_state();
        assert!(!app.poking);
        assert_eq!(
            app.inputs.last(),
            Some(&numinous_core::RoomInput::PointerCancel),
            "an interrupted gesture ends in a cancel, not a phantom hold"
        );
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
            "the accelerated camera remains held after the first click"
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
        let dir = std::env::temp_dir().join("numinous_app_playtest_shortcut");
        let _ = std::fs::remove_dir_all(&dir);

        assert!(app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F9),
            &dir,
            UNIX_EPOCH + Duration::from_secs(88),
        ));
        assert!(
            app.quiz.is_some(),
            "shortcut does not close the active mode"
        );
        let lines = app.banner.as_ref().expect("saved banner").lines();
        assert_eq!(lines[0], "PLAYTEST NOTE SAVED");
        assert!(dir.join("playtest-88.md").exists());

        let blocker = std::env::temp_dir().join("numinous_app_playtest_blocker");
        let _ = std::fs::remove_file(&blocker);
        std::fs::write(&blocker, "not a directory").expect("blocker file");
        assert!(app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F9),
            &blocker,
            UNIX_EPOCH + Duration::from_secs(89),
        ));
        let lines = app.banner.as_ref().expect("failure banner").lines();
        assert_eq!(lines[0], "PLAYTEST NOTE FAILED");
        assert!(lines[1].starts_with("WRITE ERROR:"));
        assert!(!app.handle_playtest_shortcut_to(
            &Key::Named(NamedKey::F8),
            &dir,
            UNIX_EPOCH + Duration::from_secs(90),
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
        app.radio_track = vec![0.25, -0.25, 0.5, -0.5];

        app.change_volume(0.1);

        assert!((app.volume - 0.55).abs() < f32::EPSILON);
        let banner = app.banner.as_ref().expect("volume banner");
        assert_eq!(banner.lines()[0], "VOLUME 55%");
        assert_eq!(app.radio_track, vec![0.25, -0.25, 0.5, -0.5]);
        let _ = std::fs::remove_file(&app.journey_file);
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
        app.radio_track = vec![0.25, -0.25];
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
