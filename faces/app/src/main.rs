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
    /// The chiptune bed for the current room, rendered once per room.
    tune: Vec<f32>,
    /// The journey overlay ('j' toggles): level, rank, trophies, resonances.
    show_journey: bool,
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
            tune: Vec::new(),
            show_journey: false,
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
        let round = numinous_core::build_round(seed, number, 10, 10);
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
        // The chiptune bed carries the room's voice on top: Music Engine A as
        // the score, the sonification as the accent (docs/MUSIC.md, one bus).
        if self.tune.is_empty() {
            let pattern = numinous_core::compose(self.current as u64 + 1, 8);
            self.tune = pattern.render(player.sample_rate());
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
        if let Some(quiz) = &self.quiz {
            let raster = self.draw_quiz(quiz, width, height);
            let mut rgba = raster.to_rgba();
            self.era.apply(&mut rgba, width, height);
            self.blit(&rgba, width, height, width, height);
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
        // Show draws no HUD at all: nothing between you and the math.
        let scale = (width as i32 / 400).clamp(1, 4);
        if !self.the_show && !self.studio {
            numinous_core::draw_text(
                &mut raster,
                &room.meta().title.to_uppercase(),
                10,
                10,
                scale + 1,
                '#',
            );
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
                "A / D      PREV / NEXT ROOM",
                "1 - 9      JUMP TO ROOM",
                "W / S      TIME FASTER / SLOWER",
                "MOUSE      DRAG OR WHEEL TO SCRUB",
                "E          INSPECT THE MATH",
                "Q          SWAP ERA",
                "R          RESTART SWEEP",
                "F          FULLSCREEN",
                "M          SOUND      SPACE  PAUSE",
                "TAB        THE STUDIO",
                "B          THE SHOW: SIT BACK",
                "G          THE QUIZ: NAME THE MATH",
                "J          YOUR JOURNEY AND TROPHIES",
                "ESC        CLOSE MENU (X QUITS)",
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
            let mut hint = String::from("ESC MENU   M SOUND   Q ERA   E INSPECT");
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
            .with_inner_size(winit::dpi::LogicalSize::new(900.0, 900.0));
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
                if let Some(quiz) = &mut self.quiz {
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
                        // J opens the journey: what the play has made of you.
                        Key::Character(c) if c.as_str() == "j" => {
                            self.show_journey = !self.show_journey;
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
                // Drag horizontally to scrub the room's phase directly.
                self.dragging = state == ElementState::Pressed;
            }
            WindowEvent::MouseWheel { delta, .. } if !self.studio => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
                };
                self.t = (self.t + lines * 0.02).rem_euclid(1.0);
                self.update_audio();
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
            if self.frame % 120 == 0 && !self.studio && self.quiz.is_none() {
                self.update_audio();
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
