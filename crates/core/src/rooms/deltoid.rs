//! Deltoid: three-cusped hypocycloid (also Steiner's hypocycloid).
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn scale(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.25 + x * 0.7 + s
    } else {
        0.35 + phase_unit(t) * 0.45 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.35 * a.clamp(0.2, 1.0);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.03
    };
    // x = 2a cos t + a cos(2t), y = 2a sin t - a sin(2t)
    let steps = 360;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = rad * (2.0 * th.cos() + (2.0 * th).cos()) / 3.0;
        let y = rad * (2.0 * th.sin() - (2.0 * th).sin()) / 3.0;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Deltoid room.
#[derive(Debug, Default)]
pub struct Deltoid {
    seed: u64,
}

impl Deltoid {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }
    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for Deltoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "deltoid",
            title: "Deltoid",
            wing: "Shape & Space",
            blurb: "Three-cusped hypocycloid, Steiner's curve. t and DRAG: TUNE SCALE.",
            accent: [80, 160, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "deltoid",
            root: 493.88,
            tempo: 84,
            line: &[0, 4, 7, 12, 7, 4, 0, 9],
            encodes: "circle in a thrice-larger circle draws three cusps",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        Some(format!("a={a:.2}  deltoid  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'o');
                canvas.line(px, py - 2, px, py + 2, 'o');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = scale(t, hands.last().copied(), self.seed);
        Some(format!("SCALE a={a:.3}  3 cusps"))
    }

    fn reveal(&self) -> &'static str {
        "A deltoid is a hypocycloid with three cusps: a circle of radius a/3 \
         rolls inside a circle of radius a. Also called Steiner's hypocycloid, \
         it is the envelope of lines joining points thrice apart on a circle."
    }
}

#[cfg(test)]
mod tests {
    use super::Deltoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Deltoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("deltoid"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Deltoid::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Deltoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
