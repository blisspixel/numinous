//! Figure-eight knot: the second simplest prime knot.
//!
//! DRAG: TUNE PHASE. See `docs/ROOMS.md`.

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

fn phase(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * 2.0 * std::f64::consts::PI + s
    } else {
        phase_unit(t) * 2.0 * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, ph: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.28;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.06
    };
    // Parametric figure-eight knot (Lissajous-type):
    // x = (2+cos 2t) cos 3t, y = (2+cos 2t) sin 3t, z = sin 4t
    let steps = 260;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let t = ph + 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r = 2.0 + (2.0 * t).cos();
        let x = r * (3.0 * t).cos();
        let y = r * (3.0 * t).sin();
        let z = (4.0 * t).sin();
        let xr = x * rot.cos() - y * rot.sin();
        let yr = x * rot.sin() + y * rot.cos();
        let d = 1.0 / (4.0 + z * 0.4);
        let px = (cx + xr * sc * d).round() as i32;
        let py = (cy - yr * sc * d * 0.65).round() as i32;
        if let Some((ox, oy)) = prev {
            let ch = if z > 0.3 {
                '#'
            } else if z > -0.3 {
                '*'
            } else {
                '.'
            };
            canvas.line(ox, oy, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Figure-eight knot room.
#[derive(Debug, Default)]
pub struct FigureEightKnot {
    seed: u64,
}

impl FigureEightKnot {
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

impl Room for FigureEightKnot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "figure-eight-knot",
            title: "Figure-Eight Knot",
            wing: "Shape & Space",
            blurb: "Second simplest prime knot. t and DRAG: TUNE PHASE.",
            accent: [100, 70, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phase(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "figure-eight-knot",
            root: 82.41,
            tempo: 90,
            line: &[0, 4, 5, 9, 12, 9, 5, 4],
            encodes: "figure-eight knot: four crossings, amphichiral prime",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHASE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = phase(t, None, self.seed);
        Some(format!("p={p:.2}  4xing  DRAG:PH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = phase(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = phase(t, hands.last().copied(), self.seed);
        let deg = (p.rem_euclid(1.0) * 360.0).floor() as i32;
        Some(format!("ph={deg}deg  cr=4  fig8"))
    }

    fn reveal(&self) -> &'static str {
        "The figure-eight knot is the second simplest prime knot after the \
         trefoil: four crossings and amphichiral (equivalent to its mirror). \
         It is the unique hyperbolic knot with minimal volume among simple primes."
    }
}

#[cfg(test)]
mod tests {
    use super::FigureEightKnot;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FigureEightKnot::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("4xing"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phase_changes() {
        let r = FigureEightKnot::new();
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
        FigureEightKnot::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
