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
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Near-black background (matches the `Raster` stage), packed `0x00RRGGBB`.
const BACKGROUND: u32 = 0x000A_0B0F;
/// How far the phase advances each frame.
const T_STEP: f64 = 0.004;

/// The application state driven by the winit event loop.
struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    player: Option<numinous_audio::LoopPlayer>,
    rooms: Vec<Box<dyn Room>>,
    current: usize,
    t: f64,
    paused: bool,
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
        format!(
            "Numinous  |  {}  (left/right: switch, space: pause, esc: quit)",
            self.rooms[self.current].meta().title
        )
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

    fn draw(&mut self) {
        let (Some(window), Some(surface)) = (self.window.as_ref(), self.surface.as_mut()) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let (width, height) = (w.get() as usize, h.get() as usize);

        let room = &self.rooms[self.current];
        let mut raster = Raster::with_accent(width, height, room.meta().accent);
        room.render(&mut raster, self.t);
        let rgba = raster.to_rgba();
        let (rw, rh) = (raster.width(), raster.height());

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
            } => match logical_key {
                Key::Named(NamedKey::Escape) => event_loop.exit(),
                Key::Named(NamedKey::ArrowRight) => self.switch(1),
                Key::Named(NamedKey::ArrowLeft) => self.switch(-1),
                Key::Named(NamedKey::Space) => self.paused = !self.paused,
                _ => {}
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if !self.paused {
            self.t = if self.t + T_STEP >= 1.0 {
                0.0
            } else {
                self.t + T_STEP
            };
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
