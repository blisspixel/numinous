//! Markov chain on a few states: transition matrix walk.
//!
//! DRAG: TUNE P. See `docs/ROOMS.md`.

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

fn stay(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        (0.2 + x * 0.7 + s).clamp(0.1, 0.95)
    } else {
        (0.3 + phase_unit(t) * 0.55 + s).clamp(0.1, 0.95)
    }
}

fn step(state: usize, p_stay: f64, u: f64, n: usize) -> usize {
    if u < p_stay {
        state
    } else if u < p_stay + (1.0 - p_stay) * 0.5 {
        (state + 1) % n
    } else {
        (state + n - 1) % n
    }
}

fn hash_u(i: u64, salt: u64) -> f64 {
    let mut x = i.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(salt);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    (x as f64) / (u64::MAX as f64)
}

fn visit_peak(p_stay: f64, seed: u64) -> (usize, f64) {
    let n = 5usize;
    let mut state = if seed == 0 { 0 } else { (seed as usize) % n };
    let mut hist = [0u32; 5];
    let steps = 240usize;
    for i in 0..steps {
        let u = hash_u(i as u64, seed.wrapping_mul(31) + 7);
        state = step(state, p_stay, u, n);
        hist[state] += 1;
    }
    let total: u32 = hist.iter().sum();
    let (peak, c) = hist
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|(_, v)| *v)
        .unwrap_or((0, 0));
    let frac = if total == 0 {
        0.0
    } else {
        100.0 * c as f64 / total as f64
    };
    (peak, frac)
}

fn draw(canvas: &mut dyn Surface, p_stay: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = 5usize;
    let mut state = if seed == 0 { 0 } else { (seed as usize) % n };
    let mut hist = [0u32; 5];
    let steps = width * 3;
    let mut path_y = Vec::with_capacity(width);
    for i in 0..steps {
        let u = hash_u(i as u64, seed.wrapping_mul(31) + 7);
        state = step(state, p_stay, u, n);
        hist[state] += 1;
        if i % 3 == 0 {
            path_y.push(state);
        }
    }
    // path
    let mut prev: Option<(i32, i32)> = None;
    for (col, &st) in path_y.iter().take(width).enumerate() {
        let y = ((st as f64 + 0.5) / n as f64 * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, y, '#');
        }
        prev = Some((col as i32, y));
    }
    // stationary histogram on the right edge
    let total: u32 = hist.iter().sum();
    if total > 0 {
        let bar_w = (width / 6).max(2);
        for (st, &c) in hist.iter().enumerate() {
            let frac = c as f64 / total as f64;
            let h = (frac * height as f64 * 0.9).round() as i32;
            let y1 = height as i32 - 1;
            let y0 = (y1 - h).max(0);
            let x = width as i32 - 1 - (n as i32 - st as i32) * (bar_w as i32 / 2).max(1);
            if x >= 0 {
                canvas.line(x, y0, x, y1, '=');
            }
        }
    }
}

/// Markov chain room.
#[derive(Debug, Default)]
pub struct MarkovChain {
    seed: u64,
}

impl MarkovChain {
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

impl Room for MarkovChain {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "markov-chain",
            title: "Markov Chain",
            wing: "Chance & Noise",
            blurb: "Memoryless walk on states. t and DRAG: TUNE P.",
            accent: [90, 60, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, stay(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "markov-chain",
            root: 196.0,
            tempo: 82,
            line: &[0, 2, 4, 7, 4, 2, 0, 5],
            encodes: "Markov: next state depends only on current, not the past",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = stay(t, None, self.seed);
        let (peak, peak_frac) = visit_peak(p, self.seed);
        Some(format!("stay={p:.2}  peak s{peak}={peak_frac:.0}%  DRAG:P"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = stay(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = stay(t, hands.last().copied(), self.seed);
        let (peak, peak_frac) = visit_peak(p, self.seed ^ hands.len() as u64);
        Some(format!("stay={p:.2}  peak s{peak}={peak_frac:.0}%"))
    }

    fn reveal(&self) -> &'static str {
        "A Markov chain forgets its past: the next state depends only on the \
         current one. High stay probability makes sticky paths; low stay makes \
         rapid hopping. Long-run visit rates converge to a stationary distribution."
    }
}

#[cfg(test)]
mod tests {
    use super::MarkovChain;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MarkovChain::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("markov"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = MarkovChain::new();
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
        MarkovChain::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
