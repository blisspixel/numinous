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

use numinous_core::{Raster, Room, Surface, all_rooms};
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
    fn update_audio(&self) {
        if let Some(player) = &self.player {
            let spec = self.rooms[self.current].sound(self.t);
            player.set_samples(spec.render(player.sample_rate()));
        }
    }

    fn title(&self) -> String {
        if self.the_show {
            format!(
                "Numinous  |  The Show  |  {}",
                self.rooms[self.current].meta().title
            )
        } else {
            format!(
                "Numinous  |  {}  (drag: scrub, arrows: switch, i: reveal, s: the show, tab: studio, esc: quit)",
                self.rooms[self.current].meta().title
            )
        }
    }

    fn switch(&mut self, delta: isize) {
        let n = self.rooms.len() as isize;
        self.current = (((self.current as isize + delta) % n + n) % n) as usize;
        self.t = 0.0;
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
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

        // Render the frame fully before borrowing the window surface.
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

        let rgba = raster.to_rgba();
        let (rw, rh) = (raster.width(), raster.height());

        // Now borrow the surface and blit.
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
        self.update_audio();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
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
                if self.studio {
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
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        Key::Named(NamedKey::Tab) => {
                            self.studio = true;
                            self.studio_reparse();
                        }
                        Key::Named(NamedKey::ArrowRight) => self.switch(1),
                        Key::Named(NamedKey::ArrowLeft) => self.switch(-1),
                        Key::Named(NamedKey::Space) => self.paused = !self.paused,
                        Key::Character(s) if s.as_str() == "i" => {
                            self.show_info = !self.show_info;
                        }
                        Key::Character(s) if s.as_str() == "s" => {
                            self.the_show = !self.the_show;
                            self.paused = false;
                            if let Some(window) = &self.window {
                                window.set_title(&self.title());
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
            let step = if self.the_show { SHOW_T_STEP } else { T_STEP };
            if self.t + step >= 1.0 {
                self.t = 0.0;
                // In The Show, a finished sweep drifts into the next room.
                if self.the_show {
                    self.switch(1);
                }
            } else {
                self.t += step;
            }
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run the app");
}
