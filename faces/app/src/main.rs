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

use numinous_core::{Journey, Raster, Room, Surface, all_rooms};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Near-black background (matches the `Raster` stage), packed `0x00RRGGBB`.
const BACKGROUND: u32 = 0x000A_0B0F;
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
    /// What the player has typed in the Studio.
    studio_text: String,
    /// The last expression that parsed, kept so the curve stays alive mid-edit.
    studio_expr: Option<numinous_core::Expr>,
    /// The current parse error, shown gently under the input.
    studio_error: Option<String>,
    /// GPU fractal renderer, when this machine has one (CPU raster otherwise).
    gpu: Option<numinous_gpu::FractalRenderer>,
    /// The visual era ('e' cycles: phosphor, 8-bit, vector, modern).
    era: numinous_core::Era,
    /// Sound off ('m' toggles).
    muted: bool,
    /// The help overlay ('h' toggles; shown at launch so nobody is lost).
    show_help: bool,
    /// Frame counter, used to refresh the audio as the phase sweeps.
    frame: u64,
    /// Time speed multiplier (W faster, S slower), like sprint and sneak.
    time_scale: f64,
    /// The player's journey: the same file the CLI levels (visits, plays, wins).
    journey: Journey,
    /// The level before the last change, to catch level-ups as they happen.
    level_seen: u32,
    /// A LEVEL UP banner: the lines shown, and frames left to show them.
    banner: Option<(Vec<String>, u64)>,
    /// The quiz, when playing: the round, its number, and the answer flash.
    quiz: Option<QuizPlay>,
    /// Munch, when playing in the window.
    munch: Option<MunchPlay>,
    /// Nim, when playing in the window.
    nim: Option<NimPlay>,
    /// The Gauntlet, when running in the window.
    gauntlet: Option<GauntletPlay>,
    /// The chiptune bed for the current room, rendered once per room.
    tune: Vec<f32>,
    /// The journey overlay ('j' toggles): level, rank, trophies, resonances.
    show_journey: bool,
    /// Where the mouse last was, for clicking cells and choices.
    mouse: (f64, f64),
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
}

/// The in-window quiz state: what is asked, and how the last answer landed.
struct QuizPlay {
    round: numinous_core::QuizRound,
    number: u64,
    /// After an answer: (was it right, frames left on the flash).
    flash: Option<(bool, u64)>,
}

/// The in-window Munch: a board, a cursor, your bites, and the verdict.
struct MunchPlay {
    board: numinous_core::Board,
    seed: u64,
    /// Cursor cell, 0-based (5 rows of 6).
    cursor: usize,
    /// Cells bitten so far, 0-based.
    bites: std::collections::BTreeSet<usize>,
    /// After Enter: the graded outcome, shown until a key.
    graded: Option<numinous_core::Munched>,
}

/// The in-window Gauntlet: four stages riding the other games' state.
struct GauntletPlay {
    seed: u64,
    /// 0 munch, 1 shape, 2 sky, 3 bomb, 4 done.
    stage: usize,
    munch: MunchPlay,
    quiz: QuizPlay,
    scan: numinous_core::SetiScan,
    secret: Vec<u8>,
    /// The bomb keypad: what is typed, and the feedback so far.
    wire: String,
    wire_lines: Vec<String>,
    /// Stage scores and clean flags, in order.
    scores: Vec<i64>,
    cleared: Vec<bool>,
    /// The running narration line.
    message: String,
}

/// The in-window Nim: the heaps, your aim, and the Order's last word.
struct NimPlay {
    heaps: Vec<u32>,
    seed: u64,
    /// Which heap you are aiming at.
    selected: usize,
    /// How many stones you mean to take.
    take: u32,
    /// The Order's last move, narrated.
    message: String,
    /// The end: Some(true) is your win (the secret shows), Some(false) is not.
    over: Option<bool>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            player: None,
            rooms: all_rooms(),
            current: 0,
            t: 0.0,
            paused: false,
            dragging: false,
            show_info: false,
            the_show: false,
            studio: false,
            studio_text: String::from("sin(a*x) + x/3"),
            studio_expr: numinous_core::parse("sin(a*x) + x/3").ok(),
            studio_error: None,
            gpu: None,
            era: numinous_core::Era::default(),
            muted: false,
            show_help: true,
            frame: 0,
            time_scale: 1.0,
            journey: Journey::from_text(
                &std::fs::read_to_string(journey_path()).unwrap_or_default(),
            ),
            level_seen: 1,
            banner: None,
            quiz: None,
            munch: None,
            nim: None,
            gauntlet: None,
            tune: Vec::new(),
            show_journey: false,
            mouse: (0.0, 0.0),
            room_card: 360,
            radio: None,
            radio_track: Vec::new(),
            radio_paths: Vec::new(),
            radio_index: 0,
            radio_until: None,
            journey_file: journey_path(),
        }
    }

    /// Persist the journey and raise the LEVEL UP banner when the level moves.
    fn journey_changed(&mut self) {
        let _ = std::fs::write(&self.journey_file, self.journey.to_text());
        let level = self.journey.level();
        if level > self.level_seen {
            let mut lines = vec![
                format!("LEVEL UP  LV {level}"),
                numinous_core::level_lore(level).to_uppercase(),
            ];
            if self.journey.boons_available() > 0 {
                lines.push("BOON BANKED: NUMINOUS CHOOSE".to_string());
            }
            self.banner = Some((lines, 300));
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
        let number = self.quiz.as_ref().map_or(0, |q| q.number + 1);
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(1);
        // The ramp: a brand new player's first rounds are three-way picks
        // among the most recognizable rooms; the catalog opens up from there.
        let round = if number < 3 {
            numinous_core::build_round_pool(seed, number, 10, 10, 3, &numinous_core::ICONIC)
        } else {
            numinous_core::build_round(seed, number, 10, 10)
        };
        self.journey.play();
        self.journey_changed();
        self.quiz = Some(QuizPlay {
            round,
            number,
            flash: None,
        });
    }

    /// Answer the quiz with a letter; right or wrong, the reveal follows.
    fn quiz_answer(&mut self, letter: char) {
        let Some(quiz) = self.quiz.as_mut() else {
            return;
        };
        if quiz.flash.is_some() || !quiz.round.choices.iter().any(|c| c.letter == letter) {
            return;
        }
        let correct = letter == quiz.round.answer;
        quiz.flash = Some((correct, 300));
        if correct {
            self.journey.win();
            self.journey_changed();
        }
    }

    /// Today's seed: everyone who plays today plays the same boards.
    fn daily_seed() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 86_400)
            .unwrap_or(1)
    }

    /// Post a score to the shared table (the CLI's file and rules).
    fn post_score(&self, key: &str, score: i64) -> bool {
        let path = scores_path();
        let mut board = numinous_core::Scoreboard::from_text(
            &std::fs::read_to_string(&path).unwrap_or_default(),
        );
        let best = board.record(key, score);
        let _ = std::fs::write(&path, board.to_text());
        best
    }

    /// Deal a Munch board (today's).
    fn munch_start(&mut self) {
        let seed = Self::daily_seed();
        self.journey.play();
        self.journey_changed();
        self.munch = Some(MunchPlay {
            board: numinous_core::build_board(seed, 0),
            seed,
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
        let (seed, score) = (play.seed, outcome.score);
        play.graded = Some(outcome);
        self.post_score(&format!("munch seed:{seed} board:0"), score);
        if clean {
            self.journey.win();
        }
        self.journey_changed();
    }

    /// Deal a Nim game (today's heaps).
    fn nim_start(&mut self) {
        let seed = Self::daily_seed();
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

    /// Start the Gauntlet: today's run, four stages, a combo.
    fn gauntlet_start(&mut self) {
        let seed = Self::daily_seed();
        self.gauntlet = Some(GauntletPlay {
            seed,
            stage: 0,
            munch: MunchPlay {
                board: numinous_core::build_board(seed, 0),
                seed,
                cursor: 0,
                bites: std::collections::BTreeSet::new(),
                graded: None,
            },
            quiz: QuizPlay {
                round: numinous_core::build_round(seed, 1, 10, 10),
                number: 1,
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
                    Key::Named(NamedKey::Space) => {
                        let cell = play.cursor;
                        let _ = play.bites.remove(&cell) || play.bites.insert(cell);
                    }
                    Key::Named(NamedKey::ArrowRight) => play.cursor = (play.cursor + 1) % 30,
                    Key::Named(NamedKey::ArrowLeft) => play.cursor = (play.cursor + 29) % 30,
                    Key::Named(NamedKey::ArrowDown) => play.cursor = (play.cursor + 6) % 30,
                    Key::Named(NamedKey::ArrowUp) => play.cursor = (play.cursor + 24) % 30,
                    Key::Character(c) => match c.as_str() {
                        "d" => play.cursor = (play.cursor + 1) % 30,
                        "a" => play.cursor = (play.cursor + 29) % 30,
                        "s" => play.cursor = (play.cursor + 6) % 30,
                        "w" => play.cursor = (play.cursor + 24) % 30,
                        "e" => {
                            let cell = play.cursor;
                            let _ = play.bites.remove(&cell) || play.bites.insert(cell);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            1 => {
                if let Key::Character(c) = key
                    && c.len() == 1
                {
                    let letter = c.chars().next().unwrap_or(' ').to_ascii_uppercase();
                    if run.quiz.round.choices.iter().any(|ch| ch.letter == letter) {
                        let correct = letter == run.quiz.round.answer;
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

    /// Write the current room's frame to a PNG next to the save files: the
    /// postcard key. Returns the path it wrote.
    fn save_postcard(&self) -> Option<std::path::PathBuf> {
        let room = &self.rooms[self.current];
        let mut raster = Raster::with_accent(900, 900, room.meta().accent);
        room.render(&mut raster, self.t);
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let path = std::path::PathBuf::from(home).join(format!(
            "numinous-{}-{:03}.png",
            room.meta().id,
            (self.t * 100.0) as u32
        ));
        let file = std::fs::File::create(&path).ok()?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), 900, 900);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&raster.to_rgba()).ok()?;
        Some(path)
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
            // The same geometry the board is drawn with.
            let scale = f64::from((width as i32 / 400).clamp(1, 3));
            let top = 14.0 * scale + 10.0;
            let cell_w = (width - 20.0) / 6.0;
            let cell_h = (height - top - 14.0 * scale) / 5.0;
            if mx >= 10.0 && my >= top && cell_w > 1.0 && cell_h > 1.0 {
                let col = ((mx - 10.0) / cell_w) as usize;
                let row = ((my - top) / cell_h) as usize;
                if col < 6 && row < 5 {
                    let cell = row * 6 + col;
                    play.cursor = cell;
                    let _ = play.bites.remove(&cell) || play.bites.insert(cell);
                }
            }
            return;
        }
        if let Some(quiz) = &self.quiz {
            if quiz.flash.is_some() {
                self.quiz_next();
                return;
            }
            // The choice rows, same geometry they are drawn with.
            let scale = f64::from((width as i32 / 400).clamp(1, 3));
            let line_height = 10.0 * scale;
            let count = quiz.round.choices.len() as f64;
            let base = height - (count + 1.0) * line_height - 8.0;
            if my >= base {
                let index = ((my - base) / line_height) as usize;
                if let Some(choice) = quiz.round.choices.get(index) {
                    let letter = choice.letter;
                    self.quiz_answer(letter);
                }
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
        let dir = if let Ok(dir) = std::env::var("NUMINOUS_RADIO") {
            std::path::PathBuf::from(dir)
        } else {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home).join(".numinous-radio")
        };
        if let Ok(entries) = std::fs::read_dir(&dir) {
            let prefix = format!("{}-", st.id);
            let legacy = format!("{}.wav", st.id);
            self.radio_paths = entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    name.starts_with(&prefix) || name == legacy
                })
                .collect();
            self.radio_paths.sort();
        }
        if !self.radio_paths.is_empty() {
            // Join the broadcast live: the wall clock decides which track is
            // on the air and how far into it we are.
            let durations: Vec<f64> = self
                .radio_paths
                .iter()
                .map(|p| {
                    hound::WavReader::open(p)
                        .map(|r| f64::from(r.duration()) / f64::from(r.spec().sample_rate))
                        .unwrap_or(0.0)
                })
                .collect();
            let total: f64 = durations.iter().sum();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            let mut pos = if total > 1.0 { now % total } else { 0.0 };
            self.radio_index = 0;
            for (idx, &secs) in durations.iter().enumerate() {
                if pos < secs || idx == durations.len() - 1 {
                    self.radio_index = idx;
                    break;
                }
                pos -= secs;
            }
            self.radio_play(pos);
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
        self.update_audio();
    }

    /// Put the current playlist entry on the air, starting `offset` seconds in.
    fn radio_play(&mut self, offset: f64) {
        let Some(path) = self.radio_paths.get(self.radio_index) else {
            return;
        };
        let Ok(mut reader) = hound::WavReader::open(path) else {
            return;
        };
        let rate = f64::from(reader.spec().sample_rate);
        let mut samples: Vec<f32> = reader
            .samples::<i16>()
            .filter_map(Result::ok)
            .map(|s| f32::from(s) / 32_768.0)
            .collect();
        let skip = ((offset * rate) as usize).min(samples.len());
        samples.rotate_left(skip);
        let remaining = (samples.len() - skip) as f64 / rate.max(1.0);
        self.radio_track = samples;
        self.radio_until = Some(
            std::time::Instant::now() + std::time::Duration::from_secs_f64(remaining.max(1.0)),
        );
    }

    /// GPU-render the current room if it has a real-time GPU path (the deep
    /// fractal zooms), returning the RGBA frame; `None` means draw on the CPU.
    fn gpu_frame(&mut self, width: usize, height: usize) -> Option<Vec<u8>> {
        use std::f64::consts::TAU;
        let id = self.rooms[self.current].meta().id;
        let gpu = self.gpu.as_mut()?;
        let (w, h) = (width as u32, height as u32);
        match id {
            "mandelbrot" => {
                // Zoom from the whole set deep into the seahorse valley.
                let zoom = 3.0 * 0.001_f64.powf(self.t) as f32;
                Some(gpu.render(
                    w,
                    h,
                    -0.745,
                    0.113,
                    zoom,
                    400,
                    numinous_gpu::Fractal::Mandelbrot,
                ))
            }
            "julia" => {
                // c walks a circle, morphing the set in real time.
                let theta = TAU * self.t;
                let c = numinous_gpu::Fractal::Julia {
                    cx: (0.7885 * theta.cos()) as f32,
                    cy: (0.7885 * theta.sin()) as f32,
                };
                Some(gpu.render(w, h, 0.0, 0.0, 3.2, 300, c))
            }
            _ => None,
        }
    }

    /// Re-parse the Studio text, keeping the last good curve alive on errors,
    /// and give the new expression a voice when it parses.
    fn studio_reparse(&mut self) {
        match numinous_core::parse(&self.studio_text) {
            Ok(expr) => {
                if let Some(player) = &self.player {
                    let spec = numinous_core::to_melody(
                        &expr,
                        -std::f64::consts::TAU,
                        std::f64::consts::TAU,
                        32,
                        1.0,
                    );
                    player.set_samples(spec.render(player.sample_rate()));
                }
                self.studio_expr = Some(expr);
                self.studio_error = None;
            }
            Err(message) => self.studio_error = Some(message),
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
        // The bed: a tuned radio station (Engine B) when one is cached,
        // otherwise the chiptune (Engine A). The room's voice rides on top,
        // ducked: one bus, as docs/MUSIC.md prescribes.
        if self.tune.is_empty() {
            let pattern = numinous_core::compose(self.current as u64 + 1, 8);
            self.tune = pattern.render(player.sample_rate());
        }
        if self.radio.is_some() && !self.radio_track.is_empty() {
            // The station is the sound: hand the record over untouched, so
            // nothing restarts it mid-play.
            player.set_samples(self.radio_track.clone());
            return;
        }
        let mut mix = self.tune.clone();
        if !tone.is_empty() {
            for (i, sample) in mix.iter_mut().enumerate() {
                *sample = (*sample * 0.55 + tone[i % tone.len()] * 0.45).clamp(-1.0, 1.0);
            }
        }
        player.set_samples(mix);
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
        let n = self.rooms.len() as isize;
        self.current = (((self.current as isize + delta) % n + n) % n) as usize;
        self.t = 0.0;
        self.room_card = 360;
        self.tune.clear();
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
        self.visit_current();
        self.update_audio();
    }

    /// Draw the Studio: the typed expression, its live curve (the parameter `a`
    /// swept by the clock), and any parse error, gently.
    fn draw_studio(&self, raster: &mut Raster, width: usize, height: usize) {
        use std::f64::consts::TAU;
        let scale = (width as i32 / 500).clamp(1, 3);
        numinous_core::draw_text(raster, "THE STUDIO", 10, 10, scale, '-');
        let typed = format!("Y = {}_", self.studio_text.to_uppercase());
        numinous_core::draw_text(raster, &typed, 10, 10 + 12 * scale, scale + 1, '#');
        if let Some(error) = &self.studio_error {
            numinous_core::draw_text(
                raster,
                &error.to_uppercase(),
                10,
                10 + 34 * scale,
                scale,
                '-',
            );
        }

        let Some(expr) = &self.studio_expr else {
            return;
        };
        // The knob turns itself: a sweeps 0..tau with the clock.
        let a = self.t * TAU;
        let (xmin, xmax) = (-TAU, TAU);
        let samples: Vec<(usize, f64)> = (0..width)
            .map(|i| {
                let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
                (i, numinous_core::eval(expr, x, a))
            })
            .filter(|(_, y)| y.is_finite())
            .collect();
        if samples.is_empty() {
            return;
        }
        let ymin = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let ymax = samples
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let yspan = (ymax - ymin).max(1e-9);
        let top = (60 * scale) as f64;
        let plot_h = height as f64 - top - 12.0;
        if plot_h < 8.0 {
            return;
        }
        let mut previous: Option<(i32, i32)> = None;
        for &(i, y) in &samples {
            let sx = i as i32;
            let sy = (top + (1.0 - (y - ymin) / yspan) * plot_h) as i32;
            if let Some((px, py)) = previous {
                raster.line(px, py, sx, sy, '#');
            }
            previous = Some((sx, sy));
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
        // rooms take the GPU path when one exists (full-bleed, no HUD); all else
        // draws on the CPU raster.
        if !self.studio
            && let Some(mut rgba) = self.gpu_frame(width, height)
        {
            self.era.apply(&mut rgba, width, height);
            self.blit(&rgba, width, height, width, height);
            return;
        }
        if let Some(run) = &self.gauntlet {
            let raster = self.draw_gauntlet(run, width, height);
            let (rw, rh) = (raster.width(), raster.height());
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, rw, rh);
            self.blit(&rgba, rw, rh, width, height);
            return;
        }
        if let Some(play) = &self.munch {
            let raster = self.draw_munch(play, width, height);
            let (rw, rh) = (raster.width(), raster.height());
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, rw, rh);
            self.blit(&rgba, rw, rh, width, height);
            return;
        }
        if let Some(play) = &self.nim {
            let raster = self.draw_nim(play, width, height);
            let (rw, rh) = (raster.width(), raster.height());
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, rw, rh);
            self.blit(&rgba, rw, rh, width, height);
            return;
        }
        if let Some(quiz) = &self.quiz {
            let raster = self.draw_quiz(quiz, width, height);
            let (rw, rh) = (raster.width(), raster.height());
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, rw, rh);
            self.blit(&rgba, rw, rh, width, height);
            return;
        }
        let room = &self.rooms[self.current];
        let mut raster = if self.studio {
            let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
            self.draw_studio(&mut raster, width, height);
            raster
        } else {
            let mut raster = Raster::with_accent(width, height, room.meta().accent);
            room.render(&mut raster, self.t);
            raster
        };

        // HUD: the room title, and the reveal when toggled with the 'i' key. The
        // Show stays clean, except each room announces itself as it arrives
        // and leaves its one line as it goes: a let's-play that narrates.
        let scale = (width as i32 / 400).clamp(1, 4);
        if self.the_show {
            if self.t < 0.12 {
                numinous_core::draw_text(
                    &mut raster,
                    &room.meta().title.to_uppercase(),
                    width as i32 / 10,
                    height as i32 - 24 * scale,
                    scale + 1,
                    '#',
                );
            } else if self.t > 0.9 {
                let columns = ((width as i32 / (6 * scale)) - 8).max(12) as usize;
                for (i, line) in numinous_core::wrap_text(&room.reveal().to_uppercase(), columns)
                    .iter()
                    .take(3)
                    .enumerate()
                {
                    numinous_core::draw_text(
                        &mut raster,
                        line,
                        width as i32 / 10,
                        height as i32 - (30 - i as i32 * 9) * scale,
                        scale,
                        '#',
                    );
                }
            }
        }
        if !self.the_show && !self.studio {
            numinous_core::draw_text(
                &mut raster,
                &room.meta().title.to_uppercase(),
                10,
                10,
                scale + 1,
                '#',
            );
            // The arrival card: the room explains itself for a few seconds,
            // then gets out of the way. E brings the full story anytime.
            if self.room_card > 0 && !self.show_info && !self.show_help {
                let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
                for (i, line) in
                    numinous_core::wrap_text(&room.meta().blurb.to_uppercase(), columns)
                        .iter()
                        .take(3)
                        .enumerate()
                {
                    numinous_core::draw_text(
                        &mut raster,
                        line,
                        10,
                        10 + (2 + i as i32) * 9 * scale,
                        scale,
                        '#',
                    );
                }
                numinous_core::draw_text(
                    &mut raster,
                    "(E FOR THE WHOLE STORY)",
                    10,
                    10 + 5 * 9 * scale,
                    scale,
                    '-',
                );
            }
            // The level, top right: the game in one glance.
            let level = format!("LV {}", self.journey.level());
            let lx = width as i32 - (level.len() as i32 * 6 * scale) - 10;
            numinous_core::draw_text(&mut raster, &level, lx, 10, scale, '#');
        }
        if self.show_info && !self.the_show && !self.studio {
            let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
            let line_height = 9 * scale;
            for (i, line) in numinous_core::wrap_text(&room.reveal().to_uppercase(), columns)
                .iter()
                .enumerate()
            {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    10,
                    10 + (2 + i as i32) * line_height,
                    scale,
                    '#',
                );
            }
        }

        // The help overlay (launch state, 'h' to bring back), and a hint bar so
        // nobody has to guess the controls.
        if self.show_help && !self.the_show {
            // The menu owns the screen: dim the room hard so the text reads.
            raster.dim(22);
            let menu_scale = (width as i32 / 300).clamp(2, 4);
            let lines = [
                "PLAY",
                "G          THE QUIZ: NAME THE MATH",
                "C          MUNCH: EAT WHAT FITS",
                "N          NIM: BEAT THE ORDER",
                "T          THE GAUNTLET: ONE RUN",
                "",
                "WANDER",
                "A / D      PREV / NEXT ROOM    1-9 JUMP",
                "W / S      TIME SPEED   MOUSE  SCRUB",
                "E          INSPECT    Q  ERA    R  RESTART",
                "B          THE SHOW   TAB  THE STUDIO",
                "J          JOURNEY    F  FULLSCREEN",
                "M          SOUND      SPACE  PAUSE",
                "",
                "ESC        CLOSE MENU AND WANDER",
            ];
            let line_height = 11 * menu_scale;
            let top = (height as i32 / 2) - (lines.len() as i32 * line_height) / 2;
            for (i, line) in lines.iter().enumerate() {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    width as i32 / 8,
                    top + i as i32 * line_height,
                    menu_scale,
                    '#',
                );
            }
        } else if !self.the_show {
            let mut hint = String::from("G QUIZ   C MUNCH   N NIM   T RUN   E INSPECT   ESC MENU");
            if self.muted {
                hint.push_str("   (MUTED)");
            }
            numinous_core::draw_text(
                &mut raster,
                &hint,
                10,
                height as i32 - 10 * scale,
                scale,
                '-',
            );
        }

        // The journey overlay: the constellation of what you have become.
        if self.show_journey && !self.the_show {
            raster.dim(22);
            let js = (width as i32 / 300).clamp(2, 4);
            let board = numinous_core::Scoreboard::from_text(
                &std::fs::read_to_string(scores_path()).unwrap_or_default(),
            );
            let mut lines = vec![
                format!(
                    "LV {}  [{}]",
                    self.journey.level(),
                    self.journey.level_bar(12)
                ),
                format!(
                    "{} XP  {}",
                    self.journey.sparks(),
                    self.journey.rank().name().to_uppercase()
                ),
                format!(
                    "{} OF {} ROOMS   {} WINS",
                    self.journey.visited.len(),
                    self.rooms.len(),
                    self.journey.wins
                ),
            ];
            if self.journey.streak > 1 {
                lines.push(format!("DAILY STREAK {}", self.journey.streak));
            }
            let earned: Vec<&str> = numinous_core::trophies(&self.journey, &board)
                .into_iter()
                .filter(|t| t.earned)
                .map(|t| t.name)
                .collect();
            lines.push(format!("TROPHIES {}", earned.len()));
            for name in earned.iter().take(6) {
                lines.push(format!("  {}", name.to_uppercase()));
            }
            let lit = numinous_core::resonances(&self.journey, &board)
                .into_iter()
                .filter(|r| r.active)
                .count();
            if lit > 0 {
                lines.push(format!("RESONANCES {lit}"));
            }
            lines.push("J CLOSES".to_string());
            let line_height = 11 * js;
            let top = (height as i32 / 2) - (lines.len() as i32 * line_height) / 2;
            for (i, line) in lines.iter().enumerate() {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    width as i32 / 8,
                    top + i as i32 * line_height,
                    js,
                    '#',
                );
            }
        }
        // The LEVEL UP banner rides over everything for a few seconds.
        if let Some((lines, _)) = &self.banner {
            let bs = (width as i32 / 300).clamp(2, 4);
            let line_height = 12 * bs;
            let top = height as i32 / 6;
            for (i, line) in lines.iter().enumerate() {
                numinous_core::draw_text(
                    &mut raster,
                    line,
                    width as i32 / 8,
                    top + i as i32 * line_height,
                    bs,
                    '#',
                );
            }
        }

        let mut rgba = raster.to_rgba();
        let (rw, rh) = (raster.width(), raster.height());
        self.era.apply(&mut rgba, rw, rh);
        self.blit(&rgba, rw, rh, width, height);
    }

    /// Draw the quiz: the mystery room fullscreen, the choices at the bottom,
    /// and after an answer, the verdict and the reveal.
    fn draw_quiz(&self, quiz: &QuizPlay, width: usize, height: usize) -> Raster {
        let answer_id = quiz
            .round
            .choices
            .iter()
            .find(|c| c.letter == quiz.round.answer)
            .map_or("", |c| c.id);
        let mystery = self.rooms.iter().find(|r| r.meta().id == answer_id);
        let mut raster = match mystery {
            Some(room) => {
                let mut raster = Raster::with_accent(width, height, room.meta().accent);
                room.render(&mut raster, room.postcard_t().max(0.4));
                raster
            }
            None => Raster::new(width, height),
        };
        let scale = (width as i32 / 400).clamp(1, 3);
        let line_height = 10 * scale;
        match &quiz.flash {
            None => {
                numinous_core::draw_text(
                    &mut raster,
                    "WHICH MATH MADE THIS?",
                    10,
                    10,
                    scale + 1,
                    '#',
                );
                let base = height as i32 - (quiz.round.choices.len() as i32 + 1) * line_height - 8;
                for (i, choice) in quiz.round.choices.iter().enumerate() {
                    let line = format!("{}  {}", choice.letter, choice.title.to_uppercase());
                    numinous_core::draw_text(
                        &mut raster,
                        &line,
                        10,
                        base + i as i32 * line_height,
                        scale,
                        '#',
                    );
                }
            }
            Some((correct, _)) => {
                raster.dim(35);
                let verdict = if *correct {
                    "CORRECT".to_string()
                } else {
                    format!(
                        "IT WAS {}: {}",
                        quiz.round.answer,
                        quiz.round.answer_title.to_uppercase()
                    )
                };
                numinous_core::draw_text(&mut raster, &verdict, 10, 10, scale + 1, '#');
                let columns = ((width as i32 / (6 * scale)) - 4).max(12) as usize;
                for (i, line) in
                    numinous_core::wrap_text(&quiz.round.answer_reveal.to_uppercase(), columns)
                        .iter()
                        .enumerate()
                {
                    numinous_core::draw_text(
                        &mut raster,
                        line,
                        10,
                        10 + (3 + i as i32) * line_height,
                        scale,
                        '#',
                    );
                }
                numinous_core::draw_text(
                    &mut raster,
                    "ANY KEY  NEXT     ESC  LEAVE",
                    10,
                    height as i32 - line_height - 4,
                    scale,
                    '-',
                );
            }
        }
        raster
    }

    /// Draw Munch: the 5x6 board as a grid, the cursor, your bites, the rule.
    fn draw_munch(&self, play: &MunchPlay, width: usize, height: usize) -> Raster {
        let mut raster = Raster::with_accent(width, height, [140, 230, 120]);
        let scale = (width as i32 / 400).clamp(1, 3);
        numinous_core::draw_text(
            &mut raster,
            &format!("MUNCH: {}", play.board.rule.describe().to_uppercase()),
            10,
            10,
            scale,
            '#',
        );
        let top = 14 * scale + 10;
        let cell_w = (width as i32 - 20) / 6;
        let cell_h = (height as i32 - top - 14 * scale) / 5;
        for (i, &value) in play.board.numbers.iter().enumerate() {
            let (col, row) = (i as i32 % 6, i as i32 / 6);
            let (x0, y0) = (10 + col * cell_w, top + row * cell_h);
            let (x1, y1) = (x0 + cell_w - 3, y0 + cell_h - 3);
            let bitten = play.bites.contains(&i);
            let mark = if bitten { '#' } else { '-' };
            raster.line(x0, y0, x1, y0, mark);
            raster.line(x0, y1, x1, y1, mark);
            raster.line(x0, y0, x0, y1, mark);
            raster.line(x1, y0, x1, y1, mark);
            if i == play.cursor && play.graded.is_none() {
                // The cursor breathes: a two-frame pulse, cheap and alive.
                let inset = if (self.frame / 20) % 2 == 0 { 1 } else { 2 };
                raster.line(x0 + inset, y0 + inset, x1 - inset, y0 + inset, '#');
                raster.line(x0 + inset, y1 - inset, x1 - inset, y1 - inset, '#');
                raster.line(x0 + inset, y0 + inset, x0 + inset, y1 - inset, '#');
                raster.line(x1 - inset, y0 + inset, x1 - inset, y1 - inset, '#');
            }
            let label = value.to_string();
            let tx = x0 + cell_w / 2 - (label.len() as i32 * 3 * scale);
            let ty = y0 + cell_h / 2 - 4 * scale;
            numinous_core::draw_text(
                &mut raster,
                &label,
                tx,
                ty,
                scale,
                if bitten { '#' } else { '*' },
            );
        }
        match &play.graded {
            None => {
                numinous_core::draw_text(
                    &mut raster,
                    "MOVE  WASD/ARROWS   EAT  SPACE   DONE  ENTER   ESC  LEAVE",
                    10,
                    height as i32 - 10 * scale,
                    scale,
                    '-',
                );
            }
            Some(outcome) => {
                raster.dim(30);
                let clean = outcome.bad_bites == 0 && outcome.left_behind == 0 && outcome.hits > 0;
                let mut lines = vec![format!(
                    "{} +{}",
                    if clean { "PERFECT." } else { "DONE." },
                    outcome.score
                )];
                lines.push(format!(
                    "{} EATEN  {} BAD  {} LEFT",
                    outcome.hits, outcome.bad_bites, outcome.left_behind
                ));
                if !outcome.missed.is_empty() {
                    let listed: Vec<String> = outcome.missed.iter().map(u64::to_string).collect();
                    lines.push(format!("WALKED PAST: {}", listed.join(", ")));
                    if outcome.bad_bites == 0 && outcome.missed.len() == 1 {
                        lines.push("ONE AWAY. THE BOARD REMEMBERS.".to_string());
                    }
                }
                lines.push("ANY KEY LEAVES".to_string());
                let ls = (width as i32 / 300).clamp(2, 4);
                let lh = 12 * ls;
                let ttop = (height as i32 / 2) - (lines.len() as i32 * lh) / 2;
                for (i, line) in lines.iter().enumerate() {
                    numinous_core::draw_text(
                        &mut raster,
                        line,
                        width as i32 / 8,
                        ttop + i as i32 * lh,
                        ls,
                        '#',
                    );
                }
            }
        }
        raster
    }

    /// Draw Nim: heaps as stones, your aim highlighted, the Order's last word.
    fn draw_nim(&self, play: &NimPlay, width: usize, height: usize) -> Raster {
        let mut raster = Raster::with_accent(width, height, [230, 200, 120]);
        let scale = (width as i32 / 400).clamp(1, 3);
        numinous_core::draw_text(&mut raster, "NIM: LAST STONE WINS", 10, 10, scale, '#');
        let top = 20 * scale + 10;
        let row_h = (height as i32 - top - 30 * scale) / 3;
        let stone = (row_h / 2).clamp(4, 10 * scale);
        for (heap, &count) in play.heaps.iter().enumerate() {
            let y = top + heap as i32 * row_h + row_h / 2;
            let selected = heap == play.selected && play.over.is_none();
            numinous_core::draw_text(
                &mut raster,
                &format!("{}{}", if selected { ">" } else { " " }, heap + 1),
                10,
                y - 4 * scale,
                scale,
                if selected { '#' } else { '-' },
            );
            for i in 0..count {
                let x0 = 40 + i as i32 * (stone + 6);
                // Stones you are aiming to take glow; the rest sit quiet.
                let aimed = selected && i >= count.saturating_sub(play.take);
                let mark = if aimed { '#' } else { '*' };
                for dy in 0..stone {
                    raster.line(x0, y + dy, x0 + stone, y + dy, mark);
                }
            }
        }
        let hint = if play.over.is_none() {
            format!(
                "AIM  W/S HEAP   A/D TAKE {}   ENTER TAKE   ESC LEAVE",
                play.take
            )
        } else {
            "ANY KEY LEAVES".to_string()
        };
        numinous_core::draw_text(
            &mut raster,
            &play.message,
            10,
            height as i32 - 22 * scale,
            scale,
            '#',
        );
        numinous_core::draw_text(
            &mut raster,
            &hint,
            10,
            height as i32 - 10 * scale,
            scale,
            '-',
        );
        if play.over == Some(true) {
            raster.dim(25);
            let ls = (width as i32 / 340).clamp(1, 3);
            let columns = ((width as i32 / (6 * ls)) - 6).max(12) as usize;
            let mut lines = vec!["YOU TOOK THE LAST STONE. THE SECRET IS YOURS:".to_string()];
            lines.extend(numinous_core::wrap_text(
                &numinous_core::nim_secret().to_uppercase(),
                columns,
            ));
            let lh = 10 * ls;
            let ttop = (height as i32 / 2) - (lines.len() as i32 * lh) / 2;
            for (i, line) in lines.iter().enumerate() {
                numinous_core::draw_text(&mut raster, line, 20, ttop + i as i32 * lh, ls, '#');
            }
        }
        raster
    }

    /// Draw the Gauntlet: whichever stage is live, with the run's narration.
    fn draw_gauntlet(&self, run: &GauntletPlay, width: usize, height: usize) -> Raster {
        let scale = (width as i32 / 400).clamp(1, 3);
        let mut raster = match run.stage {
            0 => {
                let mut raster = self.draw_munch(&run.munch, width, height);
                raster.dim(100); // no-op, keeps the arm shape uniform
                raster
            }
            1 => self.draw_quiz(&run.quiz, width, height),
            2 => {
                let mut raster = Raster::with_accent(width, height, [150, 210, 255]);
                numinous_core::draw_text(
                    &mut raster,
                    "THE SKY: WHICH CHANNEL IS A MIND?",
                    10,
                    10,
                    scale,
                    '#',
                );
                let lh = 14 * scale;
                for (i, channel) in run.scan.channels.iter().enumerate() {
                    let line = format!(
                        "{}  {:>10}  {}",
                        channel.letter, channel.frequency, channel.trace
                    );
                    numinous_core::draw_text(
                        &mut raster,
                        &line,
                        10,
                        30 * scale + i as i32 * lh,
                        scale,
                        '*',
                    );
                }
                numinous_core::draw_text(
                    &mut raster,
                    "PRESS THE LETTER",
                    10,
                    height as i32 - 22 * scale,
                    scale,
                    '-',
                );
                raster
            }
            3 => {
                let mut raster = Raster::with_accent(width, height, [255, 140, 120]);
                numinous_core::draw_text(
                    &mut raster,
                    "THE BOMB: FOUR DIGITS, FIVE WIRES",
                    10,
                    10,
                    scale,
                    '#',
                );
                numinous_core::draw_text(
                    &mut raster,
                    &format!("CLUE: {}", numinous_core::hint(&run.secret).to_uppercase()),
                    10,
                    26 * scale,
                    scale,
                    '*',
                );
                let lh = 12 * scale;
                for (i, line) in run.wire_lines.iter().enumerate() {
                    numinous_core::draw_text(
                        &mut raster,
                        line,
                        10,
                        44 * scale + i as i32 * lh,
                        scale,
                        '*',
                    );
                }
                numinous_core::draw_text(
                    &mut raster,
                    &format!("> {}_", run.wire),
                    10,
                    44 * scale + run.wire_lines.len() as i32 * lh + lh,
                    scale + 1,
                    '#',
                );
                numinous_core::draw_text(
                    &mut raster,
                    "TYPE DIGITS   ENTER CUTS   BACKSPACE FIXES",
                    10,
                    height as i32 - 22 * scale,
                    scale,
                    '-',
                );
                raster
            }
            _ => {
                let mut raster = Raster::with_accent(width, height, [230, 210, 120]);
                let total = gauntlet_total(&run.scores, &run.cleared);
                let clears = run.cleared.iter().filter(|&&c| c).count();
                let names = ["MUNCH", "SHAPE", "SKY", "BOMB"];
                let mut lines = vec![format!("RUN COMPLETE  {clears}/4 CLEAN")];
                for ((name, score), &clean) in names.iter().zip(&run.scores).zip(&run.cleared) {
                    lines.push(format!(
                        "{name}  +{score}{}",
                        if clean { "  CLEAN" } else { "" }
                    ));
                }
                lines.push(format!("TOTAL {total}  (GAUNTLET SEED:{})", run.seed));
                lines.push("ANY KEY LEAVES".to_string());
                let ls = (width as i32 / 300).clamp(2, 4);
                let lh = 12 * ls;
                let top = (height as i32 / 2) - (lines.len() as i32 * lh) / 2;
                for (i, line) in lines.iter().enumerate() {
                    numinous_core::draw_text(
                        &mut raster,
                        line,
                        width as i32 / 8,
                        top + i as i32 * lh,
                        ls,
                        '#',
                    );
                }
                raster
            }
        };
        // The run's narration rides the top edge on every stage but the last.
        if run.stage < 4 {
            numinous_core::draw_text(
                &mut raster,
                &run.message,
                10,
                height as i32 - 10 * scale,
                scale,
                '#',
            );
        }
        raster
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
        self.player = numinous_audio::LoopPlayer::new().ok();
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
                let _ = std::fs::write(&self.journey_file, self.journey.to_text());
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
                if self.gauntlet.is_some() {
                    self.gauntlet_key(&logical_key);
                } else if let Some(play) = &mut self.munch {
                    if play.graded.is_some() {
                        self.munch = None;
                        self.update_audio();
                    } else {
                        match logical_key {
                            Key::Named(NamedKey::Escape) => {
                                self.munch = None;
                                self.update_audio();
                            }
                            Key::Named(NamedKey::Enter) => self.munch_grade(),
                            Key::Named(NamedKey::Space) => {
                                let cell = play.cursor;
                                let _ = play.bites.remove(&cell) || play.bites.insert(cell);
                            }
                            Key::Named(NamedKey::ArrowRight) => {
                                play.cursor = (play.cursor + 1) % 30;
                            }
                            Key::Named(NamedKey::ArrowLeft) => {
                                play.cursor = (play.cursor + 29) % 30;
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                play.cursor = (play.cursor + 6) % 30;
                            }
                            Key::Named(NamedKey::ArrowUp) => {
                                play.cursor = (play.cursor + 24) % 30;
                            }
                            Key::Character(c) => match c.as_str() {
                                "d" => play.cursor = (play.cursor + 1) % 30,
                                "a" => play.cursor = (play.cursor + 29) % 30,
                                "s" => play.cursor = (play.cursor + 6) % 30,
                                "w" => play.cursor = (play.cursor + 24) % 30,
                                "e" => {
                                    let cell = play.cursor;
                                    let _ = play.bites.remove(&cell) || play.bites.insert(cell);
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                } else if let Some(play) = &mut self.nim {
                    if play.over.is_some() {
                        self.nim = None;
                        self.update_audio();
                    } else {
                        match logical_key {
                            Key::Named(NamedKey::Escape) => {
                                self.nim = None;
                                self.update_audio();
                            }
                            Key::Named(NamedKey::Enter) => self.nim_move(),
                            Key::Named(NamedKey::ArrowUp) => {
                                let n = play.heaps.len();
                                for step in 1..=n {
                                    let heap = (play.selected + n - step % n) % n;
                                    if play.heaps[heap] > 0 {
                                        play.selected = heap;
                                        break;
                                    }
                                }
                                play.take = play.take.min(play.heaps[play.selected].max(1));
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                let n = play.heaps.len();
                                for step in 1..=n {
                                    let heap = (play.selected + step) % n;
                                    if play.heaps[heap] > 0 {
                                        play.selected = heap;
                                        break;
                                    }
                                }
                                play.take = play.take.min(play.heaps[play.selected].max(1));
                            }
                            Key::Character(c) => match c.as_str() {
                                "w" => {
                                    let n = play.heaps.len();
                                    for step in 1..=n {
                                        let heap = (play.selected + n - step % n) % n;
                                        if play.heaps[heap] > 0 {
                                            play.selected = heap;
                                            break;
                                        }
                                    }
                                    play.take = play.take.min(play.heaps[play.selected].max(1));
                                }
                                "s" => {
                                    let n = play.heaps.len();
                                    for step in 1..=n {
                                        let heap = (play.selected + step) % n;
                                        if play.heaps[heap] > 0 {
                                            play.selected = heap;
                                            break;
                                        }
                                    }
                                    play.take = play.take.min(play.heaps[play.selected].max(1));
                                }
                                "d" => {
                                    play.take =
                                        (play.take + 1).min(play.heaps[play.selected].max(1));
                                }
                                "a" => play.take = play.take.saturating_sub(1).max(1),
                                _ => {}
                            },
                            _ => {}
                        }
                    }
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
                            if quiz.round.choices.iter().any(|ch| ch.letter == letter) {
                                self.quiz_answer(letter);
                            }
                        }
                        _ => {}
                    }
                } else if self.studio {
                    // Studio mode: the keyboard is a math keyboard.
                    match logical_key {
                        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                            self.studio = false;
                            self.update_audio();
                        }
                        Key::Named(NamedKey::Backspace) => {
                            self.studio_text.pop();
                            self.studio_reparse();
                        }
                        Key::Named(NamedKey::Space) => {
                            self.studio_text.push(' ');
                        }
                        Key::Character(s) => {
                            self.studio_text.push_str(&s);
                            self.studio_reparse();
                        }
                        _ => {}
                    }
                } else {
                    match logical_key {
                        // Esc is the menu, like every game since Doom. Quit from
                        // the window's close button.
                        Key::Named(NamedKey::Escape) => {
                            self.show_help = !self.show_help;
                        }
                        Key::Named(NamedKey::Tab) => {
                            self.studio = true;
                            self.studio_reparse();
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
                        // R reloads the sweep.
                        Key::Character(c) if c.as_str() == "r" => {
                            self.t = 0.0;
                            self.update_audio();
                        }
                        // F goes fullscreen.
                        Key::Character(c) if c.as_str() == "f" => {
                            if let Some(window) = &self.window {
                                if window.fullscreen().is_some() {
                                    window.set_fullscreen(None);
                                } else {
                                    window.set_fullscreen(Some(
                                        winit::window::Fullscreen::Borderless(None),
                                    ));
                                }
                            }
                        }
                        Key::Character(c) if c.as_str() == "m" => {
                            self.muted = !self.muted;
                            self.update_audio();
                        }
                        Key::Character(c) if c.as_str() == "h" => {
                            self.show_help = !self.show_help;
                        }
                        // G deals the quiz: guess the shape, in the window.
                        Key::Character(c) if c.as_str() == "g" => {
                            self.show_help = false;
                            self.quiz_next();
                        }
                        // C chomps: today's Munch board, in the window.
                        Key::Character(c) if c.as_str() == "c" => {
                            self.show_help = false;
                            self.munch_start();
                        }
                        // N is nim: three heaps against the Order.
                        Key::Character(c) if c.as_str() == "n" => {
                            self.show_help = false;
                            self.nim_start();
                        }
                        // T runs the Gauntlet: four stages, one number.
                        Key::Character(c) if c.as_str() == "t" => {
                            self.show_help = false;
                            self.gauntlet_start();
                        }
                        // J opens the journey: what the play has made of you.
                        Key::Character(c) if c.as_str() == "j" => {
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
                                self.t = 0.0;
                                if let Some(window) = &self.window {
                                    window.set_title(&self.title());
                                }
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
                if state == ElementState::Pressed && (self.munch.is_some() || self.quiz.is_some()) {
                    self.click();
                } else {
                    // Drag horizontally to scrub the room's phase directly.
                    self.dragging = state == ElementState::Pressed;
                }
            }
            WindowEvent::MouseWheel { delta, .. } if !self.studio => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
                };
                self.t = (self.t + lines * 0.02).rem_euclid(1.0);
                self.update_audio();
            }
            WindowEvent::CursorMoved { position, .. } if !self.dragging => {
                self.mouse = (position.x, position.y);
            }
            WindowEvent::CursorMoved { position, .. } if self.dragging => {
                if let Some(window) = &self.window {
                    let w = f64::from(window.inner_size().width.max(1));
                    self.t = (position.x / w).clamp(0.0, 0.999);
                    self.update_audio();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.paused && !self.dragging {
            let base = if self.the_show { SHOW_T_STEP } else { T_STEP };
            let step = base * self.time_scale;
            if self.t + step >= 1.0 {
                self.t = 0.0;
                // In The Show, a finished sweep drifts into the next room.
                if self.the_show {
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
                self.radio_play(0.0);
                self.update_audio();
            }
            if self.room_card > 0 {
                self.room_card -= 1;
            }
            if let Some((_, frames)) = &mut self.banner {
                *frames -= 1;
                if *frames == 0 {
                    self.banner = None;
                }
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Combo math: cleared stages multiply what follows (the shared rule).
fn gauntlet_total(scores: &[i64], cleared: &[bool]) -> i64 {
    let mut total = 0;
    let mut combo = 1;
    for (score, &clear) in scores.iter().zip(cleared) {
        total += score * combo;
        combo = if clear { combo + 1 } else { 1 };
    }
    total
}

/// The journey file: the same one the CLI and MCP level (env-overridable).
fn journey_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("NUMINOUS_JOURNEY") {
        return std::path::PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".numinous-journey")
}

/// The score table, read for the journey overlay's trophy evidence.
fn scores_path() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("NUMINOUS_SCORES") {
        return std::path::PathBuf::from(path);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".numinous-scores")
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run the app");
}

#[cfg(test)]
mod tests {
    use super::App;

    /// An app pointed at scratch files, with no window, player, or GPU.
    fn headless(name: &str) -> App {
        let mut app = App::new();
        app.journey = numinous_core::Journey::default();
        app.journey_file = std::env::temp_dir().join(name);
        app.level_seen = 1;
        app
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
        let answer = app.quiz.as_ref().expect("a round is live").round.answer;
        app.quiz_answer('!');
        assert!(
            app.quiz.as_ref().unwrap().flash.is_none(),
            "letters off the menu do nothing"
        );
        app.quiz_answer(answer);
        assert_eq!(app.journey.wins, 1, "the right answer is a win");
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
        let (lines, frames) = app.banner.as_ref().expect("the banner rises");
        assert!(lines[0].contains("LEVEL UP  LV 2"));
        assert!(lines.len() >= 2, "the lore line rides along");
        assert!(*frames > 0);
        let _ = std::fs::remove_file(&app.journey_file);
    }

    #[test]
    fn munch_in_the_window_grades_and_posts() {
        let mut app = headless("numinous_app_test_munch.txt");
        app.munch_start();
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
        let _ = std::fs::remove_file(&app.journey_file);
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
    fn the_radio_loads_cached_tracks_and_joins_live() {
        let dir = std::env::temp_dir().join("numinous_radio_test");
        let _ = std::fs::create_dir_all(&dir);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let path = dir.join("trance-001.wav");
        let mut writer = hound::WavWriter::create(&path, spec).expect("write wav");
        for i in 0..44_100 * 3 {
            let sample = ((i as f32 * 0.05).sin() * 12_000.0) as i16;
            writer.write_sample(sample).expect("sample");
        }
        writer.finalize().expect("finalize");
        // SAFETY-free env override: the test sets the var via a scratch app
        // field instead. tune_in reads NUMINOUS_RADIO; set through the
        // process env is forbidden, so exercise radio_play directly.
        let mut app = headless("numinous_app_test_radio.txt");
        app.radio = Some(0);
        app.radio_paths = vec![path.clone()];
        app.radio_index = 0;
        app.radio_play(1.0);
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
}
