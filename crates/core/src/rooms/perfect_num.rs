//! Even perfect numbers via Euclid-Euler: 2^{p-1}(2^p-1) for Mersenne prime p.
//!
//! DRAG: TUNE K. See `docs/ROOMS.md`.

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

fn count(t: f64, hand: Option<(f64, f64)>, seed: u64) -> usize {
    let s = if seed == 0 { 0 } else { (seed % 3) as usize };
    let base = if let Some((x, _)) = hand {
        1.0 + x * 6.0
    } else {
        2.0 + phase_unit(t) * 5.0
    };
    (base as usize + s).clamp(1, 8)
}

fn is_prime(n: u64) -> bool {
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

/// First Mersenne prime exponents (known small ones).
const MERSENNE_P: &[u32] = &[2, 3, 5, 7, 13, 17, 19, 31];

fn perfect_from_p(p: u32) -> Option<u64> {
    if p >= 32 {
        return None;
    }
    let m = (1u64 << p) - 1;
    if !is_prime(m) {
        return None;
    }
    Some((1u64 << (p - 1)) * m)
}

/// Compact label: exact n for small perfects, log10 scale for large ones.
fn perfect_label(p: u32) -> String {
    match perfect_from_p(p) {
        Some(n) if n < 1_000_000 => format!("p={p}  n={n}"),
        Some(n) => {
            let digs = (n as f64).log10().floor() as i32 + 1;
            format!("p={p}  ~{digs} digits")
        }
        None => format!("p={p}  not Mersenne"),
    }
}

fn draw(canvas: &mut dyn Surface, k: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let k = k.min(MERSENNE_P.len());
    let bar_w = (width / k.max(1)).max(2);
    let pad = if seed == 0 { 0i32 } else { (seed % 2) as i32 };
    for (i, &p) in MERSENNE_P.iter().take(k).enumerate() {
        let n = perfect_from_p(p).unwrap_or(0);
        let x0 = (i * bar_w) as i32 + pad;
        // log height of perfect number
        let h = if n > 1 {
            ((n as f64).ln() / 25.0 * height as f64).round() as i32
        } else {
            2
        };
        let h = h.clamp(2, height as i32 - 1);
        canvas.line(x0, height as i32 - 1, x0, height as i32 - 1 - h, '#');
        // Mersenne exponent as short top tick
        let th = (p as i32).min(height as i32 / 3);
        canvas.line(x0 + 1, 1, x0 + 1, 1 + th, '=');
    }
}

/// Perfect numbers room.
#[derive(Debug, Default)]
pub struct PerfectNum {
    seed: u64,
}

impl PerfectNum {
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

impl Room for PerfectNum {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "perfect-num",
            title: "Perfect Numbers",
            wing: "Number & Pattern",
            blurb: "Even perfects from Mersenne primes. t and DRAG: TUNE K.",
            accent: [110, 90, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, count(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "perfect-num",
            root: 466.16,
            tempo: 66,
            line: &[0, 5, 7, 12, 7, 5, 0, 9],
            encodes: "even perfect: 2^{p-1}(2^p-1) iff 2^p-1 prime (Euclid-Euler)",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = count(t, None, self.seed);
        let p = MERSENNE_P.get(k - 1).copied().unwrap_or(2);
        Some(format!("k={k}  {}  DRAG:K", perfect_label(p)))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = count(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
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
        let k = count(t, hands.last().copied(), self.seed);
        let p = MERSENNE_P.get(k - 1).copied().unwrap_or(2);
        Some(perfect_label(p))
    }

    fn reveal(&self) -> &'static str {
        "A perfect number equals the sum of its proper divisors (6, 28, 496...). \
         Euclid and Euler proved every even perfect number is 2^{p-1}(2^p-1) with \
         2^p-1 a Mersenne prime. No odd perfect number is known."
    }
}

#[cfg(test)]
mod tests {
    use super::PerfectNum;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PerfectNum::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains('p'));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn k_changes() {
        let r = PerfectNum::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
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
        PerfectNum::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
