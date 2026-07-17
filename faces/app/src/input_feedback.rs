//! Pointer feedback for the native App surface.
//!
//! The room renderer owns the mathematical consequence. This layer only makes
//! the latest accepted hand gesture locatable. Gesture boundaries ensure
//! separate clicks never become a false trail or a field of stale markers.

use numinous_core::{Gesture, MAX_ROOM_INPUTS, RoomInput, Surface, latest_gesture};

fn point(input: &RoomInput) -> Option<(f64, f64)> {
    let (x, y) = match *input {
        RoomInput::PointerDown { x, y, .. }
        | RoomInput::PointerMove { x, y, .. }
        | RoomInput::PointerUp { x, y, .. } => (x, y),
        _ => return None,
    };
    (x.is_finite() && y.is_finite()).then(|| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
}

fn pixel(point: (f64, f64), width: usize, height: usize) -> (i32, i32) {
    (
        (point.0 * width.saturating_sub(1) as f64).round() as i32,
        (point.1 * height.saturating_sub(1) as f64).round() as i32,
    )
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
    let end = match gesture {
        Gesture::Held { .. } => bounded
            .iter()
            .rposition(|input| {
                matches!(
                    input,
                    RoomInput::PointerDown { .. } | RoomInput::PointerMove { .. }
                )
            })
            .expect("held gesture has a down or move"),
        Gesture::Released { .. } => bounded
            .iter()
            .rposition(|input| matches!(input, RoomInput::PointerUp { .. }))
            .expect("released gesture has an up"),
        Gesture::Cancelled { .. } => bounded
            .iter()
            .rposition(|input| matches!(input, RoomInput::PointerCancel))
            .expect("cancelled gesture has a cancel"),
    };
    let mut start = end;
    if !matches!(bounded[end], RoomInput::PointerDown { .. }) {
        for index in (0..end).rev() {
            match bounded[index] {
                RoomInput::PointerMove { .. } => start = index,
                RoomInput::PointerDown { .. } => {
                    start = index;
                    break;
                }
                RoomInput::Wheel { .. } | RoomInput::Key { .. } => {}
                RoomInput::PointerUp { .. } | RoomInput::PointerCancel => break,
                _ => {}
            }
        }
    }
    let mut held = None;
    for input in &bounded[start..=end] {
        match input {
            RoomInput::PointerDown { .. } => {
                held = point(input);
            }
            RoomInput::PointerMove { .. } => {
                let to = point(input);
                if let (Some(from), Some(to)) = (held, to) {
                    let from = pixel(from, width, height);
                    let to = pixel(to, width, height);
                    surface.line(from.0, from.1, to.0, to.1, '+');
                }
                held = to;
            }
            RoomInput::PointerUp { .. } => {
                if let (Some(from), Some(to)) = (held, point(input)) {
                    let from_pixel = pixel(from, width, height);
                    let to_pixel = pixel(to, width, height);
                    if from_pixel != to_pixel {
                        surface.line(from_pixel.0, from_pixel.1, to_pixel.0, to_pixel.1, '+');
                    }
                }
                held = None;
            }
            RoomInput::PointerCancel => held = None,
            RoomInput::Wheel { .. } | RoomInput::Key { .. } => {}
            _ => {}
        }
    }
    let latest = match gesture {
        Gesture::Held { at, .. } | Gesture::Released { at, .. } | Gesture::Cancelled { at } => {
            (at.0, at.1)
        }
    };
    if !latest.0.is_finite() || !latest.1.is_finite() {
        return;
    }
    let latest = (latest.0.clamp(0.0, 1.0), latest.1.clamp(0.0, 1.0));
    let (x, y) = pixel(latest, width, height);
    let half_span = ((width.min(height) as f64 * 0.09).round() as i32).max(5);
    surface.line(x - half_span, y, x + half_span, y, '+');
    surface.line(x, y - half_span, x, y + half_span, '+');
}

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
    fn separate_clicks_do_not_gain_a_connecting_stroke() {
        let mut canvas = Canvas::new(100, 60);
        draw(
            &mut canvas,
            &[down(0.1, 0.2), up(0.1, 0.2), down(0.9, 0.8), up(0.9, 0.8)],
        );
        assert_eq!(canvas.cell(10, 12), Some(' '));
        assert_eq!(canvas.cell(50, 30), Some(' '));
        assert_eq!(canvas.cell(89, 47), Some('+'));
    }

    #[test]
    fn an_older_drag_does_not_leave_a_stale_trail() {
        let mut canvas = Canvas::new(100, 60);
        draw(
            &mut canvas,
            &[
                down(0.1, 0.2),
                RoomInput::PointerMove {
                    x: 0.3,
                    y: 0.2,
                    t: 0.05,
                },
                up(0.3, 0.2),
                down(0.9, 0.8),
                up(0.9, 0.8),
            ],
        );
        assert_eq!(canvas.cell(20, 12), Some(' '));
        assert_eq!(canvas.cell(89, 47), Some('+'));
    }

    #[test]
    fn a_truncated_active_drag_still_draws_from_its_oldest_retained_move() {
        let mut canvas = Canvas::new(100, 60);
        let inputs: Vec<_> = (0..100)
            .map(|index| RoomInput::PointerMove {
                x: index as f64 / 100.0,
                y: 0.5,
                t: index as f64 / 100.0,
            })
            .collect();
        draw(&mut canvas, &inputs);
        assert_eq!(canvas.cell(50, 30), Some('+'));
        assert_eq!(canvas.cell(98, 30), Some('+'));
    }

    #[test]
    fn one_drag_draws_its_path_and_latest_reticle() {
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
            ],
        );
        assert_eq!(canvas.cell(50, 30), Some('+'));
        assert_eq!(canvas.cell(89, 47), Some('+'));
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
