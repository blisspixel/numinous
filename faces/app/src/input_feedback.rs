//! Pointer feedback for the native App surface.
//!
//! The room renderer owns the mathematical consequence. This layer only makes
//! an *active* hand locatable: a small reticle while held. Completed gestures
//! leave no chrome so the math can breathe. Separate clicks never become a
//! false trail or a field of stale markers.

use numinous_core::{Gesture, MAX_ROOM_INPUTS, RoomInput, Surface, latest_gesture};

fn pixel(point: (f64, f64), width: usize, height: usize) -> (i32, i32) {
    (
        (point.0 * width.saturating_sub(1) as f64).round() as i32,
        (point.1 * height.saturating_sub(1) as f64).round() as i32,
    )
}

/// Half-arm of the live reticle in pixels. Kept small so it never competes
/// with the room: roughly 1.2% of the short edge, clamped to a few pixels.
fn reticle_half_span(width: usize, height: usize) -> i32 {
    let short = width.min(height) as f64;
    ((short * 0.012).round() as i32).clamp(2, 6)
}

pub(crate) fn draw(surface: &mut dyn Surface, inputs: &[RoomInput]) {
    let (width, height) = surface.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let bounded_start = inputs.len().saturating_sub(MAX_ROOM_INPUTS);
    let bounded = &inputs[bounded_start..];
    let Some(gesture) = latest_gesture(bounded) else {
        return;
    };
    // Only while the hand is still down. A released or cancelled gesture
    // already wrote its math into the room; leaving a crosshair or stroke
    // trail on top of most rooms is noise, not guidance.
    let Gesture::Held { at, .. } = gesture else {
        return;
    };
    if !at.0.is_finite() || !at.1.is_finite() {
        return;
    }
    let latest = (at.0.clamp(0.0, 1.0), at.1.clamp(0.0, 1.0));
    let (x, y) = pixel(latest, width, height);
    let half = reticle_half_span(width, height);
    // Gap at the center so the reticle reads as a cross, not a filled blot.
    let gap = 1;
    if half > gap {
        surface.line(x - half, y, x - gap, y, '+');
        surface.line(x + gap, y, x + half, y, '+');
        surface.line(x, y - half, x, y - gap, '+');
        surface.line(x, y + gap, x, y + half, '+');
    }
    surface.plot(x, y, '+');
}

#[cfg(test)]
mod tests {
    use super::{draw, reticle_half_span};
    use numinous_core::{Canvas, RoomInput};

    fn down(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerDown { x, y, t: 0.0 }
    }

    fn up(x: f64, y: f64) -> RoomInput {
        RoomInput::PointerUp { x, y, t: 0.1 }
    }

    fn move_to(x: f64, y: f64, t: f64) -> RoomInput {
        RoomInput::PointerMove { x, y, t }
    }

    #[test]
    fn reticle_stays_compact_on_large_surfaces() {
        // 9% of 900 would be ~81; we want a handful of pixels.
        assert!(reticle_half_span(900, 900) <= 6);
        assert!(reticle_half_span(100, 60) <= 4);
        assert!(reticle_half_span(40, 40) >= 2);
    }

    #[test]
    fn completed_clicks_leave_no_chrome() {
        let mut canvas = Canvas::new(100, 60);
        draw(
            &mut canvas,
            &[down(0.1, 0.2), up(0.1, 0.2), down(0.9, 0.8), up(0.9, 0.8)],
        );
        assert_eq!(
            canvas.ink_count(),
            0,
            "released gestures must not leave a reticle or trail"
        );
    }

    #[test]
    fn an_older_drag_does_not_leave_a_stale_trail() {
        let mut canvas = Canvas::new(100, 60);
        draw(
            &mut canvas,
            &[
                down(0.1, 0.2),
                move_to(0.3, 0.2, 0.05),
                up(0.3, 0.2),
                down(0.9, 0.8),
                up(0.9, 0.8),
            ],
        );
        assert_eq!(canvas.ink_count(), 0);
    }

    #[test]
    fn a_held_hand_draws_only_a_compact_reticle_not_a_path() {
        let mut canvas = Canvas::new(100, 60);
        draw(
            &mut canvas,
            &[
                down(0.1, 0.2),
                move_to(0.5, 0.5, 0.05),
                move_to(0.9, 0.8, 0.1),
            ],
        );
        // Path strokes would light the mid-line; only the latest reticle should ink.
        assert_eq!(
            canvas.cell(50, 30),
            Some(' '),
            "path strokes must not draw across the room"
        );
        assert_eq!(canvas.cell(89, 47), Some('+'));
        let ink = canvas.ink_count();
        // Compact cross: center plus a few arm pixels, far below a long trail.
        assert!(
            (3..=25).contains(&ink),
            "held reticle should stay small, ink={ink}"
        );
    }

    #[test]
    fn hostile_or_non_pointer_input_is_ignored_safely() {
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
