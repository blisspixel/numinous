//! Egyptian fractions: greedy unit-fraction expansion.
//!
//! DRAG: TUNE Q. See `docs/ROOMS.md`.

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

fn fraction(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (u64, u64) {
    let s = if seed == 0 { 0 } else { seed % 5 };
    let num = 2 + s % 4;
    let den_base = if let Some((x, _)) = hand {
        5.0 + x * 40.0
    } else {
        6.0 + phase_unit(t) * 35.0
    };
    let den = (den_base as u64 + s).clamp(num + 1, 60);
    (num, den)
}

/// Greedy Egyptian: repeatedly peel largest unit fraction <= remainder.
fn egyptian(num: u64, den: u64) -> Vec<u64> {
    let mut n = num;
    let mut d = den;
    let mut out = Vec::new();
    for _ in 0..24 {
        if n == 0 || d == 0 {
            break;
        }
        if n == 1 {
            out.push(d);
            break;
        }
        // ceil(d/n)
        let unit = d.div_ceil(n);
        out.push(unit);
        // n/d - 1/unit = (n*unit - d)/(d*unit)
        let new_n = n * unit - d;
        let new_d = d.saturating_mul(unit);
        n = new_n;
        d = new_d;
        // reduce a little to keep bounds
        if d > 1_000_000 {
            break;
        }
    }
    out
}

fn draw(canvas: &mut dyn Surface, num: u64, den: u64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || den == 0 {
        return;
    }
    let parts = egyptian(num, den);
    if parts.is_empty() {
        return;
    }
    let bar_h = (height / parts.len().max(1)).max(1);
    let max_u = *parts.iter().max().unwrap_or(&1);
    let pad = if seed == 0 { 0i32 } else { (seed % 3) as i32 };
    for (i, &u) in parts.iter().enumerate() {
        let y = (i * bar_h) as i32 + pad;
        let frac = (u as f64).ln() / (max_u as f64).ln().max(1e-6);
        let w = ((1.0 - frac * 0.85) * width as f64)
            .round()
            .clamp(2.0, width as f64) as i32;
        // shorter bar for larger denominator (smaller unit)
        canvas.line(0, y, w, y, '#');
        if bar_h > 1 {
            canvas.line(0, y + 1, w, y + 1, '#');
        }
    }
    // baseline for the original fraction height
    let hy = (height as f64 * (num as f64 / den as f64).clamp(0.05, 0.95)).round() as i32;
    canvas.line(0, hy, width as i32 - 1, hy, '-');
}

/// Egyptian fractions room.
#[derive(Debug, Default)]
pub struct EgyptianFrac {
    seed: u64,
}

impl EgyptianFrac {
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

impl Room for EgyptianFrac {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "egyptian-frac",
            title: "Egyptian Fractions",
            wing: "Number & Pattern",
            blurb: "Greedy unit fractions for p/q. t and DRAG: TUNE Q.",
            accent: [150, 110, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (n, d) = fraction(t, None, self.seed);
        draw(canvas, n, d, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "egyptian-frac",
            root: 311.13,
            tempo: 66,
            line: &[0, 4, 5, 9, 5, 4, 0, 7],
            encodes: "Egyptian greedy: peel largest 1/k <= remainder each step",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE Q")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (n, d) = fraction(t, None, self.seed);
        let k = egyptian(n, d).len();
        Some(format!("{n}/{d}  {k} units  DRAG:Q"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (n, d) = fraction(t, hands.last().copied(), self.seed);
        draw(canvas, n, d, self.seed ^ hands.len() as u64);
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
        let (n, d) = fraction(t, hands.last().copied(), self.seed);
        let k = egyptian(n, d).len();
        Some(format!("{n}/{d}  {k} egypt"))
    }

    fn reveal(&self) -> &'static str {
        "Egyptians wrote proper fractions as sums of distinct unit fractions 1/k. \
         The greedy algorithm always takes the largest possible unit fraction at \
         each step. It terminates, but not always with the shortest expansion: \
         4/17 is a classic where greed is longer than optimal."
    }
}

#[cfg(test)]
mod tests {
    use super::EgyptianFrac;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = EgyptianFrac::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("units"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn q_changes() {
        let r = EgyptianFrac::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        EgyptianFrac::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
