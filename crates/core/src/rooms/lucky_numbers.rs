//! Lucky numbers: sieve by counting positions, not multiples.
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

fn limit(t: f64, hand: Option<(f64, f64)>, seed: u64) -> usize {
    let s = if seed == 0 {
        0
    } else {
        (seed % 7) as usize * 4
    };
    let base = if let Some((x, _)) = hand {
        30.0 + x * 150.0
    } else {
        40.0 + phase_unit(t) * 120.0
    };
    (base as usize + s).clamp(20, 220)
}

/// Josephus-style lucky sieve: survivors of successive count-strike.
fn lucky_upto(n: usize) -> Vec<usize> {
    if n < 1 {
        return Vec::new();
    }
    let mut list: Vec<usize> = (1..=n).collect();
    let mut step_idx = 1;
    while step_idx < list.len() {
        let strike = list[step_idx];
        if strike < 2 {
            break;
        }
        let mut next = Vec::with_capacity(list.len());
        for (i, &v) in list.iter().enumerate() {
            if (i + 1) % strike != 0 {
                next.push(v);
            }
        }
        if next.len() == list.len() {
            break;
        }
        list = next;
        step_idx += 1;
        if step_idx >= list.len() {
            break;
        }
    }
    list
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || n == 0 {
        return;
    }
    let lucky = lucky_upto(n);
    let cols = ((width as f64).sqrt().ceil() as usize).clamp(4, width.max(1));
    let cell_w = (width / cols).max(1);
    let rows = n.div_ceil(cols).max(1);
    let cell_h = (height / rows).max(1);
    let jitter = if seed == 0 { 0 } else { (seed % 3) as i32 };
    for k in 1..=n {
        let idx = k - 1;
        let cx = ((idx % cols) * cell_w + cell_w / 2) as i32 + jitter;
        let cy = ((idx / cols) * cell_h + cell_h / 2) as i32;
        let is_lucky = lucky.binary_search(&k).is_ok();
        let ch = if is_lucky { '#' } else { '.' };
        if cx >= 0 && cy >= 0 && (cx as usize) < width && (cy as usize) < height {
            canvas.line(cx, cy, cx, cy, ch);
        }
    }
}

/// Lucky numbers room.
#[derive(Debug, Default)]
pub struct LuckyNumbers {
    seed: u64,
}

impl LuckyNumbers {
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

impl Room for LuckyNumbers {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lucky-numbers",
            title: "Lucky Numbers",
            wing: "Number & Pattern",
            blurb: "Sieve by counting seats, not multiples. t and DRAG: TUNE N.",
            accent: [90, 140, 70],
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
            key: "lucky-numbers",
            root: 277.18,
            tempo: 72,
            line: &[0, 2, 5, 7, 5, 2, 0, 9],
            encodes: "lucky sieve: strike every nth survivor, primes-like residue",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = limit(t, None, self.seed);
        let c = lucky_upto(n).len();
        Some(format!("n={n}  {c} lucky  DRAG:N"))
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
        let c = lucky_upto(n).len();
        Some(format!("N={n}  {c} live"))
    }

    fn reveal(&self) -> &'static str {
        "Lucky numbers are sieved like primes, but you strike every nth remaining \
         seat rather than every multiple of n. Start with odds (strike every 2nd), \
         then every 3rd survivor, and so on. Many lucky numbers are prime; the \
         analogy is deep and still partly mysterious."
    }
}

#[cfg(test)]
mod tests {
    use super::LuckyNumbers;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LuckyNumbers::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lucky"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = LuckyNumbers::new();
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
        LuckyNumbers::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
