//! Gaussian primes: primes in the Gaussian integers `Z[i]`.
//!
//! DRAG: TUNE R. See `docs/ROOMS.md`.

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

fn radius(t: f64, hand: Option<(f64, f64)>, seed: u64) -> i32 {
    let s = if seed == 0 { 0 } else { (seed % 4) as i32 };
    let base = if let Some((x, _)) = hand {
        4.0 + x * 14.0
    } else {
        5.0 + phase_unit(t) * 12.0
    };
    (base as i32 + s).clamp(3, 20)
}

fn is_prime_u(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n.is_multiple_of(2) {
        return n == 2;
    }
    let mut d = 3u64;
    while d * d <= n {
        if n.is_multiple_of(d) {
            return false;
        }
        d += 2;
    }
    true
}

/// Gaussian prime test for a+bi (up to units): norm p = a^2+b^2.
/// - if a*b != 0: prime iff norm is ordinary prime == 1 mod 4, or 2
/// - if on axis: prime iff |n| is ordinary prime == 3 mod 4
fn is_gaussian_prime(a: i32, b: i32) -> bool {
    let aa = a.unsigned_abs() as u64;
    let bb = b.unsigned_abs() as u64;
    if aa == 0 && bb == 0 {
        return false;
    }
    if aa == 0 || bb == 0 {
        let n = aa.max(bb);
        return is_prime_u(n) && n % 4 == 3;
    }
    let norm = aa * aa + bb * bb;
    if norm == 2 {
        return true;
    }
    is_prime_u(norm) && norm % 4 == 1
}

fn draw(canvas: &mut dyn Surface, r: i32, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as i32;
    let cy = (height.saturating_sub(1) / 2) as i32;
    let scale = ((width.min(height) as f64) * 0.42 / r.max(1) as f64).max(1.0);
    let twist = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    // axes
    canvas.line(0, cy, width as i32 - 1, cy, '-');
    canvas.line(cx, 0, cx, height as i32 - 1, '|');
    for a in -r..=r {
        for b in -r..=r {
            if !is_gaussian_prime(a, b) {
                continue;
            }
            let x = a as f64 * scale * twist.cos() - b as f64 * scale * twist.sin();
            let y = a as f64 * scale * twist.sin() + b as f64 * scale * twist.cos();
            let px = cx + x.round() as i32;
            let py = cy - y.round() as i32;
            if px >= 0 && py >= 0 && (px as usize) < width && (py as usize) < height {
                canvas.line(px, py, px, py, '#');
            }
        }
    }
}

/// Gaussian primes room.
#[derive(Debug, Default)]
pub struct GaussianPrimes {
    seed: u64,
}

impl GaussianPrimes {
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

impl Room for GaussianPrimes {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gaussian-primes",
            title: "Gaussian Primes",
            wing: "Number & Pattern",
            blurb: "Primes on the Z[i] lattice. t and DRAG: TUNE R.",
            accent: [70, 90, 150],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, radius(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gaussian-primes",
            root: 293.66,
            tempo: 76,
            line: &[0, 3, 7, 10, 7, 3, 0, 12],
            encodes: "gaussian primes: norm p=1 mod 4 splits, p=3 mod 4 stays prime",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = radius(t, None, self.seed);
        Some(format!("r={r}  Z[i]  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = radius(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
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
        let r = radius(t, hands.last().copied(), self.seed);
        let mut cnt = 0u32;
        for a in -r..=r {
            for b in -r..=r {
                if is_gaussian_prime(a, b) {
                    cnt += 1;
                }
            }
        }
        Some(format!("R={r}  G-primes={cnt}"))
    }

    fn reveal(&self) -> &'static str {
        "Gaussian integers a+bi factor uniquely up to units. A rational prime p \
         stays prime in Z[i] only if p = 3 mod 4; if p = 1 mod 4 then p = (a+bi)(a-bi). \
         Norm a^2+b^2 being prime (or 2) marks the lattice points you see."
    }
}

#[cfg(test)]
mod tests {
    use super::GaussianPrimes;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GaussianPrimes::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("Z[i]"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = GaussianPrimes::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
        GaussianPrimes::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
