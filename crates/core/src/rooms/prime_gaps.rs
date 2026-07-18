//! Prime Gap Weather: gaps as a landscape, not a lecture.
//!
//! Walk n along the integers; gap to the next prime is the weather. Twin primes
//! are calm days. DRAG: ALONG N. See `docs/ROOMS.md`.

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

fn next_prime(mut n: u64) -> u64 {
    if n < 2 {
        return 2;
    }
    n += 1;
    while !is_prime(n) {
        n += 1;
        if n > 1_000_000 {
            return n;
        }
    }
    n
}

fn start_n(t: f64, hand: Option<(f64, f64)>, seed: u64) -> u64 {
    let base = if let Some((x, _)) = hand {
        10.0 + x * 900.0
    } else {
        20.0 + phase_unit(t) * 500.0
    };
    let s = if seed == 0 { 0 } else { seed % 50 };
    base as u64 + s
}

fn gap_series(start: u64, count: usize) -> Vec<(u64, u32)> {
    let mut out = Vec::with_capacity(count);
    let mut p = if is_prime(start) {
        start
    } else {
        next_prime(start)
    };
    for _ in 0..count {
        let q = next_prime(p);
        out.push((p, (q - p) as u32));
        p = q;
    }
    out
}

fn draw(canvas: &mut dyn Surface, gaps: &[(u64, u32)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || gaps.is_empty() {
        return;
    }
    let max_g = gaps.iter().map(|g| g.1).max().unwrap_or(1).max(1);
    for (i, &(_p, g)) in gaps.iter().enumerate() {
        let x = ((i as f64 + 0.5) / gaps.len() as f64 * width as f64).round() as i32;
        let h = (g as f64 / max_g as f64) * height as f64 * 0.8;
        let y1 = height.saturating_sub(1) as i32;
        let y0 = (y1 as f64 - h).round() as i32;
        let ch = if g == 2 {
            '#'
        } else if g <= 4 {
            '*'
        } else {
            '+'
        };
        canvas.line(x, y1, x, y0, ch);
    }
}

/// Prime Gaps room.
#[derive(Debug, Default)]
pub struct PrimeGaps {
    seed: u64,
}

impl PrimeGaps {
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

impl Room for PrimeGaps {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "prime-gaps",
            title: "Prime Gap Weather",
            wing: "Number & Pattern",
            blurb: "Gaps between primes as a landscape; twins are calm. t and DRAG: ALONG N. Open \
                    doors stay open.",
            accent: [100, 200, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = start_n(t, None, self.seed);
        let gaps = gap_series(n, 48);
        draw(canvas, &gaps);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "prime weather",
            root: 164.81,
            tempo: 100,
            line: &[0, 2, 5, 7, 12, 7, 5, 2],
            encodes: "twin primes as calm days in a gap landscape",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: ALONG N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = start_n(t, None, self.seed);
        let gaps = gap_series(n, 40);
        let twins = gaps.iter().filter(|g| g.1 == 2).count();
        let max_g = gaps.iter().map(|g| g.1).max().unwrap_or(0);
        Some(format!("n={n}  twins={twins}  maxg={max_g}  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = start_n(t, hands.last().copied(), self.seed);
        let gaps = gap_series(n, 48);
        draw(canvas, &gaps);
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
        let n = start_n(t, hands.last().copied(), self.seed);
        let gaps = gap_series(n, 40);
        let twins = gaps.iter().filter(|g| g.1 == 2).count();
        let max_g = gaps.iter().map(|g| g.1).max().unwrap_or(0);
        Some(format!("ALONG n={n}  twins={twins}  max={max_g}"))
    }

    fn reveal(&self) -> &'static str {
        "Prime gaps are the weather between primes. Twin primes (gap 2) are \
         calm days; large gaps are storms. Whether infinitely many twins exist \
         remains open: the landscape is still being mapped."
    }
}

#[cfg(test)]
mod tests {
    use super::{PrimeGaps, gap_series, is_prime};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PrimeGaps::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("n="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn along_changes() {
        let r = PrimeGaps::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
    fn gaps_positive() {
        assert!(is_prime(17));
        let g = gap_series(10, 10);
        assert!(g.iter().all(|x| x.1 >= 1));
        assert!(g.iter().any(|x| x.1 == 2));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        PrimeGaps::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(PrimeGaps::new().motif().unwrap().line.len() >= 6);
    }
}
