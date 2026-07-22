//! Tractrix: path of a pulled object; companion of the catenary.
//!
//! DRAG: TUNE LENGTH. See `docs/ROOMS.md`.

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

fn length(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.2 + s
    } else {
        0.6 + phase_unit(t) * 0.8 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Fixed mapping: a is tow-rope length, so the pull spreads farther on the plate.
    let a = a.clamp(0.4, 1.9);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let scale = (width.min(height) as f64) * 0.38;
    let cx = (width.saturating_sub(1) / 2) as f64;
    let top = height as f64 * 0.08;
    let mut prev: Option<(i32, i32)> = None;
    let steps = 320;
    for i in 1..steps {
        let t = 0.05 + 3.5 * (i as f64 / steps as f64);
        let x = a / t.cosh() + j * 0.05;
        let y = a * (t - t.tanh());
        let px = (cx + x * scale).round() as i32;
        let py = (top + y * scale * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
            // mirror left
            let mx = width.saturating_sub(1) as i32 - px;
            let mox = width.saturating_sub(1) as i32 - ox;
            canvas.line(mox, oy, mx, py, '*');
        }
        prev = Some((px, py));
    }
    // asymptote (pull line)
    let ay = top.round() as i32;
    canvas.line(0, ay, width.saturating_sub(1) as i32, ay, '.');
}

/// Tractrix room.
#[derive(Debug, Default)]
pub struct Tractrix {
    seed: u64,
}

impl Tractrix {
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

impl Room for Tractrix {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tractrix",
            title: "Tractrix",
            wing: "Shape & Space",
            blurb: "The path of a pulled dog: constant tangent length. t and DRAG: TUNE LENGTH.",
            accent: [100, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, length(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tractrix",
            root: 739.99,
            tempo: 76,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "constant pull length as a pursuit curve",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LENGTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = length(t, None, self.seed);
        Some(format!("a={a:.2}  tractrix  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = length(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = length(t, hands.last().copied(), self.seed);
        // Tractrix tangent length is constant a; area under one branch is a^2/2.
        let area = 0.5 * a * a;
        Some(format!("a={a:.2}  A~{area:.2}  pull"))
    }

    fn reveal(&self) -> &'static str {
        "A tractrix is the path of an object dragged by a string of fixed length \
         along a straight line (the dog curve). Its surface of revolution is the \
         pseudosphere of constant negative curvature."
    }
}

#[cfg(test)]
mod tests {
    use super::Tractrix;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Tractrix::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("tractrix"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn length_changes() {
        let r = Tractrix::new();
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
        Tractrix::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
