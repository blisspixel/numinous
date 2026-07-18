//! Mertens function: partial sums of the Mobius function.
//!
//! DRAG: TUNE N. See `docs/ROOMS.md`.

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

fn n_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 30) as f64 };
    if let Some((x, _)) = hand {
        20.0 + x * 180.0 + s
    } else {
        40.0 + phase_unit(t) * 140.0 + s
    }
}

fn mu(n: u32) -> i8 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut primes = 0u32;
    let mut p = 2u32;
    while p * p <= x {
        if x.is_multiple_of(p) {
            x /= p;
            if x.is_multiple_of(p) {
                return 0; // square factor
            }
            primes += 1;
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if x > 1 {
        primes += 1;
    }
    if primes.is_multiple_of(2) { 1 } else { -1 }
}

fn mertens_prefix(n: usize) -> Vec<i32> {
    let mut m = Vec::with_capacity(n.max(1));
    let mut s = 0i32;
    for k in 1..=n {
        s += i32::from(mu(k as u32));
        m.push(s);
    }
    m
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(10.0, 220.0) as usize;
    let vals = mertens_prefix(n);
    let max_abs = vals
        .iter()
        .map(|v| v.unsigned_abs())
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let cy = height as f64 * 0.5;
    let y_scale = height as f64 * 0.42 / max_abs;
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
    let mut prev: Option<(i32, i32)> = None;
    for (i, &v) in vals.iter().enumerate() {
        let x = ((i as f64 / n.saturating_sub(1).max(1) as f64) * width.saturating_sub(1) as f64)
            .round() as i32;
        let y = (cy - v as f64 * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
    let _ = seed;
}

/// Mertens function room.
#[derive(Debug, Default)]
pub struct Mertens {
    seed: u64,
}

impl Mertens {
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

impl Room for Mertens {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mertens",
            title: "Mertens Function",
            wing: "Number & Pattern",
            blurb: "M(n) = sum mu(k): Mobius partial sums. t and DRAG: TUNE N.",
            accent: [70, 50, 130],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "mertens",
            root: 36.71,
            tempo: 80,
            line: &[0, 5, 3, 7, 12, 7, 3, 5],
            encodes: "Mertens M(n): running sum of Mobius mu, RH-linked growth",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as usize;
        let m = mertens_prefix(n).last().copied().unwrap_or(0);
        Some(format!("n={n}  M={m}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
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
        let n = n_param(t, hands.last().copied(), self.seed).round() as usize;
        let m = mertens_prefix(n).last().copied().unwrap_or(0);
        // Mertens conjecture (false) bound |M| < sqrt(n); report ratio.
        let bound = (n as f64).sqrt().max(1.0);
        let ratio = m as f64 / bound;
        Some(format!("n={n}  M={m}  M/sqrt={ratio:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Mertens function M(n) is the sum of the Mobius function up to n. Its \
         size is tightly tied to the Riemann hypothesis: if M(n) stays smaller \
         than about sqrt(n) for large n, RH holds in a strong form."
    }
}

#[cfg(test)]
mod tests {
    use super::{Mertens, mu};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn mu_basics() {
        assert_eq!(mu(1), 1);
        assert_eq!(mu(2), -1);
        assert_eq!(mu(6), 1);
        assert_eq!(mu(4), 0);
    }

    #[test]
    fn status_invites() {
        let s = Mertens::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("M="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Mertens::new();
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
        Mertens::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
