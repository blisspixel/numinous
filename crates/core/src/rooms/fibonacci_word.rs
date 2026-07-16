//! Fibonacci Word / Rabbit sequence: mechanical word of the golden slope.
//!
//! S0=0, S1=01, S_{n}=S_{n-1}S_{n-2}. Drawn as a mechanical word staircase or
//! as beats. DRAG: SET THE GENERATION. See `docs/ROOMS.md`.

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

fn word_gen(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (3 + (x * 12.0) as usize).clamp(3, 16)
    } else {
        (4 + (phase_unit(t) * 10.0) as usize).clamp(3, 14)
    }
}

fn fib_word(n: usize) -> Vec<u8> {
    if n == 0 {
        return vec![0];
    }
    if n == 1 {
        return vec![0, 1];
    }
    let mut a = vec![0u8];
    let mut b = vec![0u8, 1];
    for _ in 2..=n {
        let mut c = b.clone();
        c.extend_from_slice(&a);
        a = b;
        b = c;
        if b.len() > 4_096 {
            break;
        }
    }
    b
}

fn draw(canvas: &mut dyn Surface, word: &[u8], seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || word.is_empty() {
        return;
    }
    // Mechanical word as lattice path: 0 = east, 1 = north.
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut pts = vec![(x, y)];
    for &b in word {
        if b == 0 {
            x += 1.0;
        } else {
            y += 1.0;
        }
        pts.push((x, y));
    }
    let max_x = pts.last().map(|p| p.0).unwrap_or(1.0).max(1.0);
    let max_y = pts.last().map(|p| p.1).unwrap_or(1.0).max(1.0);
    let to_px = |u: f64, v: f64| -> (i32, i32) {
        let px = 0.08 + 0.84 * u / max_x;
        let py = 0.08 + 0.84 * v / max_y;
        (
            (px * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - py) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(u, v)) in pts.iter().enumerate() {
        let p = to_px(u, v);
        if let Some(o) = prev {
            let ch = if word.get(i.saturating_sub(1)).copied().unwrap_or(0) == 1 {
                '#'
            } else {
                '*'
            };
            canvas.line(o.0, o.1, p.0, p.1, ch);
        }
        prev = Some(p);
    }
    // Beat row at bottom.
    let yb = (0.92 * height as f64).round() as i32;
    let show = word.len().min(width.saturating_sub(4));
    for (i, &bit) in word.iter().take(show).enumerate() {
        let x = (2 + i) as i32;
        canvas.plot(x, yb, if bit == 1 { '|' } else { '.' });
    }
    let _ = seed;
}

/// Fibonacci Word room.
#[derive(Debug, Default)]
pub struct FibonacciWord {
    seed: u64,
}

impl FibonacciWord {
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

impl Room for FibonacciWord {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fibonacci-word",
            title: "The Rabbit Sequence",
            wing: "Number & Pattern",
            blurb: "Fibonacci word: 0, 01, 010, 01001, ... the mechanical word of the golden slope. \
                    t and DRAG: SET THE GENERATION.",
            accent: [200, 160, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let g = word_gen(t, None)
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 2) as usize
            };
        let w = fib_word(g);
        draw(canvas, &w, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "fib word",
            root: 277.18,
            tempo: 104,
            line: &[0, 0, 5, 0, 7, 12, 0, 5],
            encodes: "ones sparse as rabbits under golden growth",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE GENERATION")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = word_gen(t, None);
        let w = fib_word(g);
        let ones = w.iter().filter(|&&b| b == 1).count();
        Some(format!("gen={g}  len={}  1s={ones}  DRAG", w.len()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = word_gen(t, hands.last().copied());
        let w = fib_word(g);
        draw(canvas, &w, self.seed);
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
        let g = word_gen(t, hands.last().copied());
        let w = fib_word(g);
        let ones = w.iter().filter(|&&b| b == 1).count();
        let dens = ones as f64 / w.len().max(1) as f64;
        Some(format!("GEN={g}  dens1={dens:.3}  len={}", w.len()))
    }

    fn reveal(&self) -> &'static str {
        "The Fibonacci word is the mechanical word for slope 1/phi: ones appear \
         with density 1/phi^2. It is Sturmian, aperiodic, and the same sequence \
         that counts rabbit pairs in Fibonacci's original story."
    }
}

#[cfg(test)]
mod tests {
    use super::{FibonacciWord, fib_word};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FibonacciWord::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("gen="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn gen_changes() {
        let r = FibonacciWord::new();
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
    fn classic_prefix() {
        assert_eq!(fib_word(0), vec![0]);
        assert_eq!(fib_word(1), vec![0, 1]);
        assert_eq!(fib_word(2), vec![0, 1, 0]);
        assert_eq!(fib_word(3), vec![0, 1, 0, 0, 1]);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        FibonacciWord::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(FibonacciWord::new().motif().unwrap().line.len() >= 6);
    }
}
