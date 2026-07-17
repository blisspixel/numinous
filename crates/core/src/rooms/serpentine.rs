//! Serpentine curve: y(x^2 + a^2) = a b x, a cubic rational snake.
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

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

fn param_a(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.4 + s
    } else {
        0.6 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = a.clamp(0.3, 2.0);
    let b = 1.2
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.1
        };
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sx = (width as f64) * 0.4 / (3.0 * a);
    let sy = (height as f64) * 0.35 / b.max(0.5);
    // y = a b x / (x^2 + a^2)
    let mut prev: Option<(i32, i32)> = None;
    let steps = 280;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let x = -3.5 * a + 7.0 * a * u;
        let y = a * b * x / (x * x + a * a);
        let px = (cx + x * sx).round() as i32;
        let py = (cy - y * sy).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // asymptote y=0
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
}

/// Serpentine room.
#[derive(Debug, Default)]
pub struct Serpentine {
    seed: u64,
}

impl Serpentine {
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

impl Room for Serpentine {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "serpentine",
            title: "Serpentine Curve",
            wing: "Shape & Space",
            blurb: "Newton's snake y = a b x/(x^2+a^2). t and DRAG: TUNE A.",
            accent: [50, 120, 70],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, param_a(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "serpentine",
            root: 11.56,
            tempo: 88,
            line: &[0, 3, 5, 8, 12, 8, 5, 3],
            encodes: "serpentine: cubic rational with one max and one min",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = param_a(t, None, self.seed);
        Some(format!("a={a:.2}  snake  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = param_a(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let a = param_a(t, hands.last().copied(), self.seed);
        // Serpentine peak height a/2 at x = a (Newton form).
        let peak = 0.5 * a;
        Some(format!("a={a:.2}  peak~{peak:.2}  serp"))
    }

    fn reveal(&self) -> &'static str {
        "Newton studied the serpentine curve y(x^2 + a^2) = a b x. It has a single \
         maximum and minimum, an S-shaped middle, and the x-axis as asymptote: a \
         cubic rational that snakes once across the plane."
    }
}

#[cfg(test)]
mod tests {
    use super::Serpentine;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Serpentine::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("snake"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Serpentine::new();
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
        Serpentine::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
