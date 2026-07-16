//! Elementary CA Rule 110: Turing-complete cellular automaton.
//!
//! DRAG: SET THE SEED ROW. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const RULE: u8 = 110;

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

fn seed_bias(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.05 + x * 0.9 + s * 0.1
    } else {
        0.15 + phase_unit(t) * 0.5 + s * 0.1
    }
}

fn next_row(row: &[u8]) -> Vec<u8> {
    let n = row.len();
    let mut out = vec![0u8; n];
    for i in 0..n {
        let left = row[(i + n - 1) % n];
        let mid = row[i];
        let right = row[(i + 1) % n];
        let idx = (left << 2) | (mid << 1) | right;
        out[i] = (RULE >> idx) & 1;
    }
    out
}

fn draw(canvas: &mut dyn Surface, bias: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let w = width.min(96);
    let h = height.min(48);
    let mut state = seed ^ 0x1100_1100_1100_1100;
    let mut next_u = || {
        state = state.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(1);
        (state >> 33) as f64 / (u32::MAX as f64)
    };
    let mut row: Vec<u8> = (0..w)
        .map(|_| if next_u() < bias { 1 } else { 0 })
        .collect();
    // guarantee at least one live cell
    if row.iter().all(|&c| c == 0) {
        row[w / 2] = 1;
    }
    let mut history = Vec::with_capacity(h);
    for _ in 0..h {
        history.push(row.clone());
        row = next_row(&row);
    }
    for y in 0..height {
        let gy = (y * h / height.max(1)).min(h - 1);
        for x in 0..width {
            let gx = (x * w / width.max(1)).min(w - 1);
            let ch = if history[gy][gx] != 0 { '#' } else { ' ' };
            canvas.plot(x as i32, y as i32, ch);
        }
    }
}

/// Rule 110 room.
#[derive(Debug, Default)]
pub struct Wolfram110 {
    seed: u64,
}

impl Wolfram110 {
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

impl Room for Wolfram110 {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "rule-110",
            title: "Rule 110",
            wing: "Emergence",
            blurb: "Wolfram's Turing-complete elementary CA. t and DRAG: SET THE SEED ROW.",
            accent: [40, 200, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, seed_bias(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "rule 110",
            root: 82.41,
            tempo: 120,
            line: &[0, 0, 5, 7, 0, 12, 7, 5],
            encodes: "local bits that can emulate any computation",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE SEED ROW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = seed_bias(t, None, self.seed);
        Some(format!("p={b:.2}  R110  DRAG:SEED"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = seed_bias(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
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
        let b = seed_bias(t, hands.last().copied(), self.seed);
        Some(format!("SEED p={b:.3}  class IV"))
    }

    fn reveal(&self) -> &'static str {
        "Elementary CA Rule 110 updates each cell from its three-bit neighborhood \
         by a fixed table. Cook proved it is Turing complete: simple local rules \
         that can, in principle, run any program."
    }
}

#[cfg(test)]
mod tests {
    use super::Wolfram110;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Wolfram110::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SEED"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn seed_changes() {
        let r = Wolfram110::new();
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
        Wolfram110::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
