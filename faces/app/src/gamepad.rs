//! Controller input translated into device-independent app commands.
//!
//! Hardware events stay here. The rest of the app receives semantic buttons
//! and a normalized virtual hand, the same coordinate space used by mouse and
//! replayable room input.

use std::time::Instant;

use gilrs::{Axis, Button, EventType, Gilrs};
use numinous_core::Surface;

const DEADZONE: f64 = 0.18;
const HAND_SPEED: f64 = 0.62;
const MAX_FRAME_SECONDS: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Command {
    PrimaryDown,
    PrimaryUp,
    Back,
    Menu,
    Inspect,
    Reset,
    PreviousRoom,
    NextRoom,
    Slower,
    Faster,
    Up,
    Down,
    Left,
    Right,
    CycleEra,
    CycleRadio,
    Pause,
    PointerMoved { point: (f64, f64), held: bool },
    PhaseDelta(f64),
    CancelPointer,
}

#[derive(Debug, Clone)]
struct VirtualHand {
    point: (f64, f64),
    stick: (f64, f64),
    phase_axis: f64,
    held: bool,
    visible: bool,
}

impl Default for VirtualHand {
    fn default() -> Self {
        Self {
            point: (0.5, 0.5),
            stick: (0.0, 0.0),
            phase_axis: 0.0,
            held: false,
            visible: false,
        }
    }
}

impl VirtualHand {
    fn set_axis(&mut self, axis: Axis, value: f32) {
        let value = shaped_axis(value);
        match axis {
            Axis::LeftStickX => self.stick.0 = value,
            Axis::LeftStickY => self.stick.1 = -value,
            Axis::RightStickX => self.phase_axis = value,
            _ => return,
        }
        if value != 0.0 {
            self.visible = true;
        }
    }

    fn press(&mut self, button: Button) -> Option<Command> {
        let command = pressed_command(button)?;
        self.visible = true;
        if button == Button::South {
            self.held = true;
        }
        Some(command)
    }

    fn tick(&mut self, seconds: f64) -> Vec<Command> {
        let seconds = seconds.clamp(0.0, MAX_FRAME_SECONDS);
        let mut commands = Vec::with_capacity(2);
        let before = self.point;
        self.point.0 = (self.point.0 + self.stick.0 * HAND_SPEED * seconds).clamp(0.0, 1.0);
        self.point.1 = (self.point.1 + self.stick.1 * HAND_SPEED * seconds).clamp(0.0, 1.0);
        if self.point != before {
            commands.push(Command::PointerMoved {
                point: self.point,
                held: self.held,
            });
        }
        if self.phase_axis != 0.0 {
            commands.push(Command::PhaseDelta(self.phase_axis * seconds * 0.45));
        }
        commands
    }

    fn cancel(&mut self) -> Option<Command> {
        self.stick = (0.0, 0.0);
        self.phase_axis = 0.0;
        self.held.then(|| {
            self.held = false;
            Command::CancelPointer
        })
    }
}

fn shaped_axis(value: f32) -> f64 {
    let value = f64::from(value).clamp(-1.0, 1.0);
    let magnitude = value.abs();
    if magnitude <= DEADZONE {
        return 0.0;
    }
    let normalized = (magnitude - DEADZONE) / (1.0 - DEADZONE);
    value.signum() * normalized * normalized
}

pub(crate) struct GamepadInput {
    gilrs: Option<Gilrs>,
    hand: VirtualHand,
    last_tick: Instant,
    active: bool,
}

impl GamepadInput {
    pub(crate) fn new() -> Self {
        #[cfg(test)]
        let gilrs = None;
        #[cfg(not(test))]
        let gilrs = Gilrs::new().ok();
        Self {
            gilrs,
            hand: VirtualHand::default(),
            last_tick: Instant::now(),
            active: true,
        }
    }

    pub(crate) fn poll(&mut self, now: Instant) -> Vec<Command> {
        let mut commands = Vec::new();
        if !self.active {
            self.drain_events();
            self.last_tick = now;
            return commands;
        }
        if let Some(gilrs) = &mut self.gilrs {
            while let Some(event) = gilrs.next_event() {
                match event.event {
                    EventType::ButtonPressed(button, _) => {
                        if let Some(command) = self.hand.press(button) {
                            commands.push(command);
                        }
                    }
                    EventType::ButtonReleased(Button::South, _) => {
                        self.hand.held = false;
                        commands.push(Command::PrimaryUp);
                    }
                    EventType::AxisChanged(axis, value, _) => self.hand.set_axis(axis, value),
                    EventType::Disconnected => {
                        if let Some(command) = self.hand.cancel() {
                            commands.push(command);
                        }
                    }
                    _ => {}
                }
            }
        }
        let seconds = now.saturating_duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        commands.extend(self.hand.tick(seconds));
        commands
    }

    fn drain_events(&mut self) {
        if let Some(gilrs) = &mut self.gilrs {
            while gilrs.next_event().is_some() {}
        }
    }

    pub(crate) fn deactivate(&mut self) -> Option<Command> {
        self.active = false;
        self.drain_events();
        self.last_tick = Instant::now();
        self.hand.cancel()
    }

    pub(crate) fn activate(&mut self) {
        self.drain_events();
        let _ = self.hand.cancel();
        self.last_tick = Instant::now();
        self.active = true;
    }

    pub(crate) fn cursor(&self) -> Option<(f64, f64)> {
        self.hand.visible.then_some(self.hand.point)
    }

    #[cfg(test)]
    pub(crate) fn set_cursor_for_test(&mut self, point: (f64, f64)) {
        self.hand.point = (point.0.clamp(0.0, 1.0), point.1.clamp(0.0, 1.0));
        self.hand.visible = true;
    }
}

fn pressed_command(button: Button) -> Option<Command> {
    Some(match button {
        Button::South => Command::PrimaryDown,
        Button::East => Command::Back,
        Button::Start => Command::Menu,
        Button::Select => Command::Inspect,
        Button::LeftThumb => Command::Reset,
        Button::LeftTrigger => Command::PreviousRoom,
        Button::RightTrigger => Command::NextRoom,
        Button::LeftTrigger2 => Command::Slower,
        Button::RightTrigger2 => Command::Faster,
        Button::DPadUp => Command::Up,
        Button::DPadDown => Command::Down,
        Button::DPadLeft => Command::Left,
        Button::DPadRight => Command::Right,
        Button::West => Command::CycleEra,
        Button::North => Command::CycleRadio,
        Button::RightThumb => Command::Pause,
        _ => return None,
    })
}

pub(crate) fn draw_cursor(
    surface: &mut dyn Surface,
    point: (f64, f64),
    width: usize,
    height: usize,
) {
    if width == 0 || height == 0 {
        return;
    }
    let x = (point.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    let y = (point.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32;
    let radius = (width.min(height) / 90).clamp(5, 12) as i32;
    surface.line(x - radius, y, x - 2, y, '#');
    surface.line(x + 2, y, x + radius, y, '#');
    surface.line(x, y - radius, x, y - 2, '#');
    surface.line(x, y + 2, x, y + radius, '#');
    surface.plot(x, y, '#');
}

#[cfg(test)]
mod tests {
    use super::{Command, GamepadInput, VirtualHand, draw_cursor, pressed_command, shaped_axis};
    use gilrs::{Axis, Button};
    use numinous_core::Canvas;
    use std::time::Instant;

    #[test]
    fn deadzone_prevents_center_drift_and_curve_reaches_full_scale() {
        assert_eq!(shaped_axis(0.0), 0.0);
        assert_eq!(shaped_axis(0.17), 0.0);
        assert_eq!(shaped_axis(-0.17), 0.0);
        assert!((shaped_axis(1.0) - 1.0).abs() < 1e-9);
        assert!((shaped_axis(-1.0) + 1.0).abs() < 1e-9);
        assert!(shaped_axis(0.5) > 0.0 && shaped_axis(0.5) < 0.5);
    }

    #[test]
    fn virtual_hand_moves_with_elapsed_time_and_clamps_to_the_stage() {
        let mut hand = VirtualHand::default();
        hand.set_axis(Axis::LeftStickX, 1.0);
        let first = hand.tick(0.05);
        assert!(matches!(first.as_slice(), [Command::PointerMoved { .. }]));
        assert!(hand.point.0 > 0.5);
        for _ in 0..100 {
            let _ = hand.tick(0.05);
        }
        assert_eq!(hand.point.0, 1.0);
    }

    #[test]
    fn held_motion_is_a_drag_and_cancel_closes_it_once() {
        let mut hand = VirtualHand {
            held: true,
            ..VirtualHand::default()
        };
        hand.set_axis(Axis::LeftStickY, 1.0);
        let commands = hand.tick(0.05);
        assert!(matches!(
            commands.as_slice(),
            [Command::PointerMoved { held: true, .. }]
        ));
        assert_eq!(hand.cancel(), Some(Command::CancelPointer));
        assert_eq!(hand.cancel(), None);
    }

    #[test]
    fn right_stick_scrubs_phase_without_moving_the_hand() {
        let mut hand = VirtualHand::default();
        hand.set_axis(Axis::RightStickX, 0.8);
        let point = hand.point;
        let commands = hand.tick(0.04);
        assert_eq!(hand.point, point);
        assert!(matches!(commands.as_slice(), [Command::PhaseDelta(delta)] if *delta > 0.0));
    }

    #[test]
    fn unsupported_axis_does_not_reveal_or_move_the_hand() {
        let mut hand = VirtualHand::default();
        hand.set_axis(Axis::RightStickY, 1.0);
        assert!(!hand.visible);
        assert!(hand.tick(-1.0).is_empty());
    }

    #[test]
    fn every_supported_button_has_one_semantic_command() {
        let cases = [
            (Button::South, Command::PrimaryDown),
            (Button::East, Command::Back),
            (Button::Start, Command::Menu),
            (Button::Select, Command::Inspect),
            (Button::LeftThumb, Command::Reset),
            (Button::LeftTrigger, Command::PreviousRoom),
            (Button::RightTrigger, Command::NextRoom),
            (Button::LeftTrigger2, Command::Slower),
            (Button::RightTrigger2, Command::Faster),
            (Button::DPadUp, Command::Up),
            (Button::DPadDown, Command::Down),
            (Button::DPadLeft, Command::Left),
            (Button::DPadRight, Command::Right),
            (Button::West, Command::CycleEra),
            (Button::North, Command::CycleRadio),
            (Button::RightThumb, Command::Pause),
        ];
        for (button, expected) in cases {
            assert_eq!(pressed_command(button), Some(expected));
        }
        assert_eq!(pressed_command(Button::Mode), None);
    }

    #[test]
    fn only_supported_button_presses_reveal_the_virtual_hand() {
        let mut hand = VirtualHand::default();
        assert_eq!(hand.press(Button::Mode), None);
        assert!(!hand.visible);

        assert_eq!(hand.press(Button::RightThumb), Some(Command::Pause));
        assert!(hand.visible);
        assert!(!hand.held);
    }

    #[test]
    fn deadzone_axis_activity_stays_hidden_and_still_clears_motion() {
        let mut hand = VirtualHand::default();
        hand.set_axis(Axis::LeftStickX, 0.17);
        assert!(!hand.visible);
        assert_eq!(hand.stick.0, 0.0);

        hand.set_axis(Axis::LeftStickX, 1.0);
        assert!(hand.visible);
        assert_eq!(hand.stick.0, 1.0);
        hand.set_axis(Axis::LeftStickX, 0.0);
        assert_eq!(hand.stick.0, 0.0);
        assert!(hand.tick(0.05).is_empty());
    }

    #[test]
    fn cursor_visibility_tracks_controller_activity() {
        let mut input = GamepadInput::new();
        assert_eq!(input.cursor(), None);
        input.hand.set_axis(Axis::LeftStickX, 0.5);
        assert_eq!(input.cursor(), Some((0.5, 0.5)));
    }

    #[test]
    fn cursor_draws_a_clamped_cross_and_ignores_empty_surfaces() {
        let mut empty = Canvas::new(0, 0);
        draw_cursor(&mut empty, (0.5, 0.5), 0, 0);
        assert_eq!(empty.ink_count(), 0);

        let mut canvas = Canvas::new(30, 20);
        draw_cursor(&mut canvas, (2.0, -1.0), 30, 20);
        assert_eq!(canvas.cell(29, 0), Some('#'));
        assert!(canvas.ink_count() >= 5);
    }

    #[test]
    fn inactive_input_cancels_motion_and_never_replays_it_on_activation() {
        let mut input = GamepadInput::new();
        input.hand.set_axis(Axis::LeftStickX, 1.0);
        input.hand.held = true;
        assert_eq!(input.deactivate(), Some(Command::CancelPointer));
        assert!(input.poll(Instant::now()).is_empty());
        let point = input.hand.point;
        input.activate();
        assert!(input.poll(Instant::now()).is_empty());
        assert_eq!(input.hand.point, point);
    }
}
