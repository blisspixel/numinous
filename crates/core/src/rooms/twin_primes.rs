//! Twin primes: primes p where p+2 is also prime.
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

fn limit(t: f64, hand: Option<(f64, f64)>, seed: u64) -> u64 {
    let s = if seed == 0 { 0 } else { (seed % 7) * 10 };
    let base = if let Some((x, _)) = hand {
        40.0 + x * 400.0
    } else {
        60.0 + phase_unit(t) * 350.0
    };
    (base as u64 + s).clamp(30, 500)
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

/// Twin count and largest twin pair (p, p+2) with p <= n.
fn twin_stats(n: u64) -> (u32, Option<(u64, u64)>) {
    let mut c = 0u32;
    let mut last = None;
    for p in 3..=n {
        if p % 2 == 1 && is_prime(p) && is_prime(p + 2) {
            c += 1;
            last = Some((p, p + 2));
        }
    }
    (c, last)
}

fn draw(canvas: &mut dyn Surface, n: u64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || n < 3 {
        return;
    }
    let jitter = if seed == 0 { 0i32 } else { (seed % 3) as i32 };
    let mut twins = 0u32;
    for p in 3..=n {
        if p % 2 == 0 {
            continue;
        }
        if is_prime(p) && is_prime(p + 2) {
            twins += 1;
            let x =
                ((p as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32 + jitter;
            let y = (height as i32 / 2) - ((twins % 7) as i32 - 3);
            if x >= 0 && x < width as i32 {
                canvas.line(x, y, x, y + 1, '#');
                let x2 =
                    (((p + 2) as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
                if x2 >= 0 && x2 < width as i32 {
                    canvas.line(x2, y, x2, y + 1, '=');
                    canvas.line(x, y, x2, y, '.');
                }
            }
        }
    }
    // density trend of twin count
    let dens = twins as f64 / (n as f64).ln().max(1.0);
    let hy = ((1.0 - dens * 0.15) * height.saturating_sub(1) as f64)
        .round()
        .clamp(0.0, height.saturating_sub(1) as f64) as i32;
    canvas.line(0, hy, width as i32 - 1, hy, '-');
}

/// Twin primes room.
#[derive(Debug, Default)]
pub struct TwinPrimes {
    seed: u64,
}

impl TwinPrimes {
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

impl Room for TwinPrimes {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "twin-primes",
            title: "Twin Primes",
            wing: "Number & Pattern",
            blurb: "Primes that come in pairs (p, p+2). t and DRAG: TUNE N.",
            accent: [90, 70, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, limit(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "twin-primes",
            root: 493.88,
            tempo: 84,
            line: &[0, 2, 7, 9, 14, 9, 7, 2],
            encodes: "twin primes: p and p+2 both prime, infinite open conjecture",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = limit(t, None, self.seed);
        let (c, last) = twin_stats(n);
        match last {
            Some((p, q)) => Some(format!("n={n}  {c} twins  last {p},{q}  DRAG:N")),
            None => Some(format!("n={n}  0 twins  DRAG:N")),
        }
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = limit(t, hands.last().copied(), self.seed);
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
        let n = limit(t, hands.last().copied(), self.seed);
        let (c, last) = twin_stats(n);
        match last {
            Some((p, q)) => Some(format!("{c} twins  last {p},{q}")),
            None => Some(format!("N={n}  0 twins")),
        }
    }

    fn reveal(&self) -> &'static str {
        "Twin primes are pairs (p, p+2) that are both prime, like (3,5), (11,13), \
         (17,19). There are infinitely many primes, but whether infinitely many \
         twins exist is still open, though bounded gaps are now known."
    }
}

#[cfg(test)]
mod tests {
    use super::TwinPrimes;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TwinPrimes::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("twins"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = TwinPrimes::new();
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
        TwinPrimes::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn action_names_last_twin_pair() {
        let s = TwinPrimes::new()
            .status_input(
                0.5,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert!(s.contains("twins") || s.contains(','));
        assert!(s.chars().any(|c| c.is_ascii_digit()));
        assert!(s.chars().count() <= 56);
    }
}
