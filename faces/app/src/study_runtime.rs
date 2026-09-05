//! Read-only room study routing and return-state ownership.

use numinous_app::study_reader::{ReaderCommand, ReaderIntent, StudyReader};
use numinous_core::Raster;
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

use super::{App, controls, gamepad, input_legend, menu};

pub(super) struct ActiveStudy {
    pub(super) reader: StudyReader,
    return_menu: Option<menu::MenuState>,
}

impl App {
    fn study_entry_available(&self) -> bool {
        self.activity_kind().is_none()
            && !self.console.is_open()
            && (!self.show_journey || self.show_help)
    }

    fn study_shortcut_available(&self) -> bool {
        self.study_entry_available()
            && !(self.show_help
                && self.menu.is_open()
                && self.menu.route() != menu::MenuRoute::Home)
    }

    fn capture_room_input_for_study(&mut self) {
        // Accepted history describes the experiment, not opening its notes.
        // Retain its last held pose; a later fresh press resumes room input.
        self.dragging = false;
        self.poking = false;
        // Captured controller releases never reach the gameplay latch.
        self.experiment_primary_consumed = false;
        self.study_keys.extend(self.pressed_keys.iter().cloned());
        self.study_pointer_needs_press = true;
        self.gamepad.capture_for_reader();
        self.menu.clear_pointer();
    }

    pub(super) fn open_room_study(&mut self) -> bool {
        if self.study.is_some() || !self.study_entry_available() {
            return false;
        }
        let reader = match StudyReader::new(self.rooms[self.current].as_ref(), &self.study_locale) {
            Ok(reader) => reader,
            Err(_) => {
                self.banner = Some(super::feedback::Banner::status(
                    "THE READER COULD NOT OPEN",
                    super::feedback::REFUSAL_FRAMES,
                ));
                return false;
            }
        };
        self.capture_room_input_for_study();
        let return_menu = (self.show_help && self.menu.is_open()).then(|| self.menu.clone());
        self.show_help = false;
        self.menu.close();
        self.study = Some(ActiveStudy {
            reader,
            return_menu,
        });
        true
    }

    pub(super) fn close_room_study(&mut self) {
        let Some(study) = self.study.take() else {
            return;
        };
        self.capture_room_input_for_study();
        if let Some(menu) = study.return_menu {
            self.menu = menu;
            self.show_help = true;
        }
    }

    fn apply_study_intent(&mut self, intent: ReaderIntent) {
        match intent {
            ReaderIntent::None => {}
            ReaderIntent::Close => self.close_room_study(),
            ReaderIntent::Language(locale) => {
                let Some(study) = self.study.as_ref() else {
                    return;
                };
                let depth = study.reader.depth();
                if let Ok(mut reader) = StudyReader::new(self.rooms[self.current].as_ref(), &locale)
                {
                    reader.navigate(ReaderCommand::Select(depth));
                    if let Some(study) = self.study.as_mut() {
                        study.reader = reader;
                    }
                    self.study_locale = locale;
                    // This explicit preference action is the only persistent
                    // effect of study navigation. It does not touch Journey.
                    self.persist_preferences();
                }
            }
        }
    }

    fn navigate_study(&mut self, command: ReaderCommand) {
        if let Some(study) = &mut self.study {
            let intent = study.reader.navigate(command);
            self.apply_study_intent(intent);
        }
    }

    /// Runs before any ordinary keyboard cancellation or gameplay dispatch.
    pub(super) fn handle_study_key(&mut self, key: &Key, repeat: bool) -> bool {
        // Track every press before deciding its owner, so even a brief reader
        // visit captures keys whose first repeat has not arrived yet. Use the
        // ordinary command identity so changing letter case cannot evade it.
        let command_key = controls::normalized_command_key(key);
        self.pressed_keys.insert(command_key.clone());
        if self.study.is_none() && self.study_keys.contains(&command_key) {
            if repeat {
                return true;
            }
            // A fresh press also recovers if the OS omitted a release.
            self.study_keys.remove(&command_key);
        }
        let explain = matches!(key, Key::Character(text)
            if text.eq_ignore_ascii_case("e") || text.as_str() == "?");
        if self.study.is_none() {
            if explain && self.study_shortcut_available() {
                if !repeat {
                    self.input_mode = input_legend::InputMode::KeyboardMouse;
                    self.study_keys.insert(command_key);
                    self.open_room_study();
                }
                return true;
            }
            return false;
        }
        self.input_mode = input_legend::InputMode::KeyboardMouse;
        self.study_keys.insert(command_key);
        if self.handle_global_audio_key(key, repeat) {
            return true;
        }
        let command = match key {
            Key::Named(NamedKey::ArrowUp) => Some(ReaderCommand::Lines(-1.0)),
            Key::Named(NamedKey::ArrowDown) => Some(ReaderCommand::Lines(1.0)),
            Key::Named(NamedKey::PageUp) => Some(ReaderCommand::Pages(-1)),
            Key::Named(NamedKey::PageDown) => Some(ReaderCommand::Pages(1)),
            Key::Named(NamedKey::Home) => Some(ReaderCommand::Start),
            Key::Named(NamedKey::End) => Some(ReaderCommand::End),
            Key::Named(NamedKey::ArrowLeft) if !repeat => Some(ReaderCommand::Depth(-1)),
            Key::Named(NamedKey::ArrowRight) if !repeat => Some(ReaderCommand::Depth(1)),
            Key::Named(NamedKey::Enter) if !repeat => Some(ReaderCommand::Mathematics),
            Key::Named(NamedKey::Escape) if !repeat => Some(ReaderCommand::Back),
            _ if explain && !repeat => Some(ReaderCommand::Back),
            Key::Character(text) if text.eq_ignore_ascii_case("l") && !repeat => {
                Some(ReaderCommand::Language)
            }
            _ => None,
        };
        if let Some(command) = command {
            self.navigate_study(command);
        }
        true
    }

    pub(super) fn release_study_key(&mut self, key: &Key) {
        let command_key = controls::normalized_command_key(key);
        self.pressed_keys.remove(&command_key);
        self.study_keys.remove(&command_key);
    }

    /// Consume reader pointer events before menu, pause, or room release logic.
    pub(super) fn handle_study_window_pointer(&mut self, state: ElementState) -> bool {
        let Some(study) = &mut self.study else {
            return false;
        };
        self.input_mode = input_legend::InputMode::KeyboardMouse;
        let intent = match state {
            ElementState::Pressed => {
                study.reader.pointer_down(self.mouse);
                ReaderIntent::None
            }
            ElementState::Released => study.reader.pointer_up(self.mouse),
        };
        self.apply_study_intent(intent);
        true
    }

    fn study_pixel_point(&self, normalized: (f64, f64)) -> (f64, f64) {
        let (width, height) = self.window.as_ref().map_or((900, 700), |window| {
            let size = window.inner_size();
            (size.width, size.height)
        });
        (
            normalized.0 * f64::from(width),
            normalized.1 * f64::from(height),
        )
    }

    pub(super) fn handle_study_pointer_down(&mut self, point: (f64, f64)) -> bool {
        let point = self.study_pixel_point(point);
        if let Some(study) = &mut self.study {
            study.reader.pointer_down(point);
            return true;
        }
        self.study_pointer_needs_press = false;
        false
    }

    pub(super) fn handle_study_pointer_move(&mut self, _point: (f64, f64)) -> bool {
        self.study.is_some() || self.study_pointer_needs_press
    }

    pub(super) fn handle_study_pointer_up(&mut self, point: (f64, f64)) -> bool {
        let point = self.study_pixel_point(point);
        if let Some(study) = &mut self.study {
            let intent = study.reader.pointer_up(point);
            self.apply_study_intent(intent);
            return true;
        }
        self.study_pointer_needs_press
    }

    pub(super) fn handle_study_wheel(&mut self, lines: f64) -> bool {
        if self.study.is_none() {
            return false;
        }
        if lines.is_finite() {
            self.input_mode = input_legend::InputMode::KeyboardMouse;
            self.navigate_study(ReaderCommand::Lines(-lines.clamp(-100.0, 100.0) as f32));
        }
        true
    }

    pub(super) fn clear_study_pointer(&mut self) -> bool {
        if let Some(study) = &mut self.study {
            study.reader.clear_pointer();
            self.capture_room_input_for_study();
            return true;
        }
        false
    }

    pub(super) fn handle_study_gamepad(&mut self, command: gamepad::Command) -> bool {
        if self.study.is_none() {
            if command == gamepad::Command::Inspect && self.study_shortcut_available() {
                self.input_mode = input_legend::InputMode::Controller;
                self.open_room_study();
                return true;
            }
            return false;
        }
        self.input_mode = input_legend::InputMode::Controller;
        let command = match command {
            gamepad::Command::Up => Some(ReaderCommand::Lines(-1.0)),
            gamepad::Command::Down => Some(ReaderCommand::Lines(1.0)),
            gamepad::Command::PreviousRoom => Some(ReaderCommand::Pages(-1)),
            gamepad::Command::NextRoom => Some(ReaderCommand::Pages(1)),
            gamepad::Command::Left => Some(ReaderCommand::Depth(-1)),
            gamepad::Command::Right => Some(ReaderCommand::Depth(1)),
            gamepad::Command::PrimaryDown => Some(ReaderCommand::Mathematics),
            gamepad::Command::Back | gamepad::Command::Inspect => Some(ReaderCommand::Back),
            gamepad::Command::Menu => {
                self.close_room_study();
                if !self.show_help {
                    self.open_home_menu();
                }
                None
            }
            _ => None,
        };
        if let Some(command) = command {
            self.navigate_study(command);
        }
        true
    }

    /// Keep a complete poll batch in the reader even when one event closes it.
    /// The return value tells the tick owner to hold the room for this interval.
    pub(super) fn handle_gamepad_batch(&mut self, commands: Vec<gamepad::Command>) -> bool {
        let mut captured = self.study.is_some();
        for command in commands {
            if !captured
                || self.study.is_some()
                || matches!(
                    command,
                    gamepad::Command::ToggleMute
                        | gamepad::Command::VolumeUp
                        | gamepad::Command::VolumeDown
                )
            {
                self.handle_gamepad_command(command);
            }
            captured |= self.study.is_some();
        }
        captured
    }

    pub(super) fn study_frame(&mut self, width: usize, height: usize) -> Option<Vec<u8>> {
        let study = self.study.as_mut()?;
        Some(
            match study.reader.render(
                width as u32,
                height as u32,
                self.input_mode,
                self.gamepad.controller_copy(),
            ) {
                Ok(rgba) => rgba,
                Err(_) => {
                    let mut raster = Raster::with_accent(width, height, [78, 255, 255]);
                    numinous_core::draw_text(
                        &mut raster,
                        "THE READER COULD NOT DRAW THIS PAGE",
                        12,
                        20,
                        1,
                        '#',
                    );
                    numinous_core::draw_text(&mut raster, "E OR ESC: RETURN", 12, 44, 1, '*');
                    raster.to_rgba()
                }
            },
        )
    }
}
