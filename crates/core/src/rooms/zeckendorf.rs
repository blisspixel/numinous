//! Zeckendorf representation: every n as unique non-adjacent Fibonaccis.
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

fn max_n(t: f64, hand: Option<(f64, f64)>, seed: u64) -> u64 {
    let s = if seed == 0 { 0 } else { (seed % 6) * 3 };
    let base = if let Some((x, _)) = hand {
        20.0 + x * 120.0
    } else {
        30.0 + phase_unit(t) * 100.0
    };
    (base as u64 + s).clamp(12, 160)
}

fn fibs_upto(limit: u64) -> Vec<u64> {
    let mut f = vec![1u64, 2];
    while let Some(&last) = f.last() {
        let prev = f[f.len() - 2];
        let next = prev.saturating_add(last);
        if next > limit {
            break;
        }
        f.push(next);
    }
    f
}

/// Greedy Zeckendorf bits (no two consecutive Fibonaccis).
fn zeck_bits(n: u64, fibs: &[u64]) -> Vec<bool> {
    let mut bits = vec![false; fibs.len()];
    let mut rem = n;
    for i in (0..fibs.len()).rev() {
        if fibs[i] <= rem {
            bits[i] = true;
            rem -= fibs[i];
        }
    }
    bits
}

fn draw(canvas: &mut dyn Surface, max_n: u64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || max_n == 0 {
        return;
    }
    let fibs = fibs_upto(max_n.max(2));
    let cols = fibs.len().max(1);
    let rows = max_n as usize;
    let cell_w = (width / cols).max(1);
    let cell_h = if rows == 0 {
        1
    } else {
        (height / rows.max(1)).max(1)
    };
    let shift = if seed == 0 {
        0usize
    } else {
        (seed as usize) % 2
    };
    for row in 0..rows.min(height) {
        let n = (row + 1 + shift) as u64;
        if n > max_n {
            break;
        }
        let bits = zeck_bits(n, &fibs);
        for (col, &on) in bits.iter().enumerate() {
            if !on {
                continue;
            }
            let x0 = (col * cell_w) as i32;
            let y0 = (row * cell_h) as i32;
            let x1 = (x0 + cell_w as i32 - 1).min(width as i32 - 1);
            let y1 = (y0 + cell_h as i32 - 1).min(height as i32 - 1);
            if x1 >= x0 && y1 >= y0 {
                canvas.line(x0, y0, x1, y0, '#');
                if y1 > y0 {
                    canvas.line(x0, y1, x1, y1, '#');
                }
            }
        }
    }
}

/// Zeckendorf room.
#[derive(Debug, Default)]
pub struct Zeckendorf {
    seed: u64,
}

impl Zeckendorf {
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

impl Room for Zeckendorf {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "zeckendorf",
            title: "Zeckendorf",
            wing: "Number & Pattern",
            blurb: "Unique Fibonacci base, no adjacent 1s. t and DRAG: TUNE N.",
            accent: [100, 70, 130],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, max_n(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "zeckendorf",
            root: 392.0,
            tempo: 84,
            line: &[0, 2, 3, 5, 8, 5, 3, 2],
            encodes: "Zeckendorf: every n = sum nonadjacent Fibonaccis, unique",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = max_n(t, None, self.seed);
        Some(format!("n={n}  fib base  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = max_n(t, hands.last().copied(), self.seed);
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
        let n = max_n(t, hands.last().copied(), self.seed);
        let fibs = fibs_upto(n.max(2));
        let bits = zeck_bits(n, &fibs);
        let ones = bits.iter().filter(|&&b| b).count();
        Some(format!("n={n}  fibs={}  ones={ones}", fibs.len()))
    }

    fn reveal(&self) -> &'static str {
        "Zeckendorf's theorem: every positive integer is a unique sum of \
         non-consecutive Fibonacci numbers (usually starting from 2, 3, 5, ...). \
         The greedy algorithm always works. It is the Fibonacci analogue of \
         binary, with a no-adjacent-ones rule."
    }
}

#[cfg(test)]
mod tests {
    use super::Zeckendorf;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Zeckendorf::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("fib"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Zeckendorf::new();
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
        Zeckendorf::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
