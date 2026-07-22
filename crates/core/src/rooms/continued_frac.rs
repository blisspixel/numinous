//! Continued Fractions: best approximations fall out of a ladder.
//!
//! Walk the continued fraction of a real (ambient golden, or under the hand).
//! Convergents are the best rational hits. DRAG: SET THE REAL. See `docs/ROOMS.md`.

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

fn target(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    if let Some((x, y)) = hand {
        // Map plate to (1, 3) range of interesting irrationals.
        1.0 + x * 2.0 + y * 0.1
    } else {
        let base = 0.5 * (1.0 + 5.0_f64.sqrt()); // golden
        if seed == 0 {
            base + phase_unit(t) * 0.3
        } else {
            base + (seed % 7) as f64 * 0.05 + phase_unit(t) * 0.1
        }
    }
}

fn continued_fraction(x: f64, max_terms: usize) -> Vec<u32> {
    let mut terms = Vec::new();
    let mut v = x;
    for _ in 0..max_terms {
        if !v.is_finite() || v.abs() > 1e12 {
            break;
        }
        let a = v.floor();
        if a < 0.0 || a > u32::MAX as f64 {
            break;
        }
        terms.push(a as u32);
        let f = v - a;
        if f.abs() < 1e-12 {
            break;
        }
        v = 1.0 / f;
    }
    terms
}

fn convergents(terms: &[u32]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut p0 = 0u64;
    let mut q0 = 1u64;
    let mut p1 = 1u64;
    let mut q1 = 0u64;
    for &a in terms {
        let p = a as u64 * p1 + p0;
        let q = a as u64 * q1 + q0;
        out.push((p, q));
        p0 = p1;
        q0 = q1;
        p1 = p;
        q1 = q;
        if out.len() > 12 {
            break;
        }
    }
    out
}

fn draw(canvas: &mut dyn Surface, terms: &[u32], conv: &[(u64, u64)], x: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Ladder of a_i as fat bars.
    for (i, &a) in terms.iter().enumerate().take(10) {
        let y = ((0.12 + i as f64 * 0.075) * height as f64).round() as i32;
        let len = ((a.min(12) as f64 / 12.0) * 0.55 * width as f64).max(3.0);
        let x0 = (0.08 * width as f64).round() as i32;
        let x1 = x0 + len as i32;
        canvas.line(x0, y, x1, y, '#');
        canvas.line(x0, y + 1, x1, y + 1, '*');
        canvas.line(x0, y + 2, x1, y + 2, '.');
    }
    // Convergents as points approaching a vertical line at x position.
    let tx = ((0.15 + (x - 1.0) / 2.0 * 0.7).clamp(0.15, 0.85) * width as f64).round() as i32;
    canvas.line(tx, 0, tx, height.saturating_sub(1) as i32, '.');
    canvas.line(tx + 1, 0, tx + 1, height.saturating_sub(1) as i32, '.');
    for (i, &(p, q)) in conv.iter().enumerate() {
        if q == 0 {
            continue;
        }
        let approx = p as f64 / q as f64;
        let ax =
            ((0.15 + (approx - 1.0) / 2.0 * 0.7).clamp(0.05, 0.95) * width as f64).round() as i32;
        let ay = ((0.2 + i as f64 * 0.06) * height as f64).round() as i32;
        let ch = if i + 1 == conv.len() { 'R' } else { '*' };
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx * dx + dy * dy <= 2 {
                    canvas.plot(ax + dx, ay + dy, ch);
                }
            }
        }
    }
}

/// Continued Fractions room.
#[derive(Debug, Default)]
pub struct ContinuedFrac {
    seed: u64,
}

impl ContinuedFrac {
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

impl Room for ContinuedFrac {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "continued-frac",
            title: "The Ladder of Approximations",
            wing: "Number & Pattern",
            blurb: "Continued fractions peel best rationals from a real. Golden is the hardest. t \
                    and DRAG: SET THE REAL.",
            accent: [160, 200, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let x = target(t, None, self.seed);
        let terms = continued_fraction(x, 10);
        let conv = convergents(&terms);
        draw(canvas, &terms, &conv, x);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "convergents",
            root: 196.0,
            tempo: 96,
            line: &[0, 2, 5, 7, 12, 7, 5, 2],
            encodes: "best rationals falling from a continued ladder",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE REAL")
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = target(t, None, self.seed);
        let terms = continued_fraction(x, 8);
        let last = terms.last().copied().unwrap_or(0);
        Some(format!(
            "x={x:.3}  a0={} a_n={last}  DRAG",
            terms.first().unwrap_or(&0)
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let x = target(t, hands.last().copied(), self.seed);
        let terms = continued_fraction(x, 10);
        let conv = convergents(&terms);
        draw(canvas, &terms, &conv, x);
        if let Some(&(px, py)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let ix = (px * width.saturating_sub(1) as f64).round() as i32;
                let iy = (py * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(ix - 2, iy, ix + 2, iy, '+');
                canvas.line(ix, iy - 2, ix, iy + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let x = target(t, hands.last().copied(), self.seed);
        let terms = continued_fraction(x, 8);
        let conv = convergents(&terms);
        if let Some(&(p, q)) = conv.last() {
            let err = (x - p as f64 / q as f64).abs();
            Some(format!("x={x:.3}  {p}/{q}  err={err:.1e}"))
        } else {
            Some(format!("x={x:.3}  terms={}", terms.len()))
        }
    }

    fn reveal(&self) -> &'static str {
        "Continued fractions produce the best rational approximations to a \
         real: each convergent beats every fraction with a smaller denominator. \
         The golden ratio is the worst approximated, all ones on the ladder."
    }
}

#[cfg(test)]
mod tests {
    use super::{ContinuedFrac, continued_fraction, convergents};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ContinuedFrac::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("x="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn set_changes() {
        let r = ContinuedFrac::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn golden_starts_with_one() {
        let phi = 0.5 * (1.0 + 5.0_f64.sqrt());
        let t = continued_fraction(phi, 6);
        assert_eq!(t[0], 1);
        assert!(t.iter().skip(1).all(|&a| a == 1));
        let c = convergents(&t);
        assert!(!c.is_empty());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        ContinuedFrac::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(ContinuedFrac::new().motif().unwrap().line.len() >= 6);
    }
}
