//! Pointer feedback for the native App surface.
//!
//! Intentionally empty: the rooms are the art. Interaction changes the math,
//! not an overlay. Mouse users already have a system cursor; controller users
//! get a minimal gamepad hand elsewhere. Completed gestures never leave chrome.

use numinous_core::{RoomInput, Surface};

/// No-op: never draw reticle, trail, or hand marks on top of a room.
pub(crate) fn draw(_surface: &mut dyn Surface, _inputs: &[RoomInput]) {}

#[cfg(test)]
mod tests {
    use super::draw;
    use numinous_core::{Canvas, RoomInput};

    fn down(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerDown { x, y, t: 0.0 }
    }

    fn up(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerUp { x, y, t: 0.1 }
    }

    #[test]
    fn feedback_never_inks_the_art() {
        let mut canvas = Canvas::new(100, 60);
        draw(
            &mut canvas,
            &[
                down(0.1, 0.2),
                RoomInput::PointerMove {
                    x: 0.5,
                    y: 0.5,
                    t: 0.05,
                },
                up(0.9, 0.8),
                down(0.2, 0.3),
                up(0.2, 0.3),
            ],
        );
        assert_eq!(canvas.ink_count(), 0);
    }

    #[test]
    fn hostile_surfaces_are_safe() {
        let mut empty = Canvas::new(0, 0);
        draw(&mut empty, &[down(0.5, 0.5)]);
        let mut canvas = Canvas::new(20, 10);
        draw(
            &mut canvas,
            &[
                down(f64::NAN, f64::INFINITY),
                RoomInput::PointerCancel,
                RoomInput::Wheel { delta: 1.0 },
            ],
        );
        assert_eq!(canvas.ink_count(), 0);
    }
}
