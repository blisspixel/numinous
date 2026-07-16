//! Gambler's ruin: random walk absorbed at 0 or N.
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

fn p_win(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        (0.15 + x * 0.7 + s).clamp(0.05, 0.95)
    } else {
        (0.25 + phase_unit(t) * 0.5 + s).clamp(0.05, 0.95)
    }
}

/// Probability of hitting N before 0 starting from i, with P(up)=p.
fn ruin_prob_reach_n(i: u32, n: u32, p: f64) -> f64 {
    if i == 0 {
        return 0.0;
    }
    if i >= n {
        return 1.0;
    }
    let q = 1.0 - p;
    if (p - q).abs() < 1e-12 {
        return i as f64 / n as f64;
    }
    let r = q / p;
    (1.0 - r.powi(i as i32)) / (1.0 - r.powi(n as i32))
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = 20 + if seed == 0 { 0 } else { (seed % 5) as u32 };
    let p = p.clamp(0.05, 0.95);
    // Plot P(ruin avoid) vs starting capital i
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=n {
        let pr = ruin_prob_reach_n(i, n, p);
        let x = ((i as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - pr) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05).round()
            as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
    // fair-game diagonal
    canvas.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        0,
        '.',
    );
    // sample path with LCG
    let mut state = if seed == 0 { 1u64 } else { seed };
    let mut capital = n / 2;
    let mut prev_x = 0i32;
    let mut prev_y =
        ((1.0 - capital as f64 / n as f64) * height.saturating_sub(1) as f64).round() as i32;
    for step in 1..width.min(80) {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u = ((state >> 33) as f64) / ((1u64 << 31) as f64);
        if u < p {
            capital = (capital + 1).min(n);
        } else {
            capital = capital.saturating_sub(1);
        }
        let x = step as i32;
        let y =
            ((1.0 - capital as f64 / n as f64) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(prev_x, prev_y, x, y, '*');
        prev_x = x;
        prev_y = y;
        if capital == 0 || capital == n {
            break;
        }
    }
}

/// Gambler's ruin room.
#[derive(Debug, Default)]
pub struct GamblersRuin {
    seed: u64,
}

impl GamblersRuin {
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

impl Room for GamblersRuin {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gamblers-ruin",
            title: "Gamblers Ruin",
            wing: "Chance & Order",
            blurb: "Random walk absorbed at 0 or N. t and DRAG: TUNE P.",
            accent: [130, 40, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, p_win(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gamblers-ruin",
            root: 19.45,
            tempo: 100,
            line: &[0, 3, 7, 5, 12, 7, 3, 0],
            encodes: "gambler ruin: unfair walk almost surely hits zero first",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = p_win(t, None, self.seed);
        let mid = ruin_prob_reach_n(10, 20, p);
        Some(format!("p={p:.2}  P(N)={mid:.2}  DRAG:P"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = p_win(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = p_win(t, hands.last().copied(), self.seed);
        let mid = ruin_prob_reach_n(10, 20, p);
        Some(format!("P={p:.3}  winN={mid:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "In gambler's ruin a player with capital i plays until broke or rich at N. \
         If each bet is unfair, ruin is almost sure no matter the starting pile; \
         the classic formula tilts sharply once p leaves one half."
    }
}

#[cfg(test)]
mod tests {
    use super::{GamblersRuin, ruin_prob_reach_n};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn fair_is_linear() {
        let p = ruin_prob_reach_n(5, 10, 0.5);
        assert!((p - 0.5).abs() < 1e-9);
    }

    #[test]
    fn status_invites() {
        let s = GamblersRuin::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("P(N)"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = GamblersRuin::new();
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
        GamblersRuin::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
