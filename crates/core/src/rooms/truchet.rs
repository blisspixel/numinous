//! Truchet tiles / 10 PRINT: one tile, two rotations, endless weave.
//!
//! Each cell flips a coin and draws one of two diagonal arcs (or lines). Bias
//! the coin and mazes become loops, or loops become mazes. DRAG: PAINT THE
//! BIAS. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0x0010_0717_714E_0001;

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

fn bias(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((x, _)) = hand {
        x.clamp(0.05, 0.95)
    } else {
        0.35 + phase_unit(t) * 0.3
    }
}

fn draw(canvas: &mut dyn Surface, p_bias: f64, seed: u64, cell: usize) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cell = cell.clamp(4, 12);
    let cols = (width / cell).max(1);
    let rows = (height / cell).max(1);
    let mut rng = SplitMix64::new(SEED ^ seed ^ ((p_bias * 1000.0) as u64));
    for row in 0..rows {
        for col in 0..cols {
            let flip = rng.next_f64() < p_bias;
            let x0 = (col * cell) as i32;
            let y0 = (row * cell) as i32;
            let x1 = x0 + cell as i32 - 1;
            let y1 = y0 + cell as i32 - 1;
            if flip {
                // / style: bottom-left to top-right via two quarter arcs approx as diagonals.
                canvas.line(x0, y1, x1, y0, '*');
            } else {
                canvas.line(x0, y0, x1, y1, '#');
            }
        }
    }
}

/// Truchet / Weave room.
#[derive(Debug, Default)]
pub struct Truchet {
    seed: u64,
}

impl Truchet {
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

impl Room for Truchet {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "truchet",
            title: "The Weave",
            wing: "Emergence",
            blurb: "One tile, two rotations, a coin flip per cell: Truchet and 10 PRINT mazes from \
                    nothing. t drifts bias; DRAG: PAINT THE BIAS.",
            accent: [100, 180, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let b = bias(t, None);
        let cell = 6 + (phase_unit(t) * 4.0) as usize;
        draw(canvas, b, self.seed, cell);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tile flip",
            root: 146.83,
            tempo: 132,
            line: &[0, 0, 5, 7, 0, 5, 7, 12],
            encodes: "one coin per cell weaving mazes from two strokes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: PAINT THE BIAS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = bias(t, None);
        Some(format!("bias={b:.2}  tile=Truchet  DRAG:BIAS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = bias(t, hands.last().copied());
        let cell = 5 + (b * 5.0) as usize;
        draw(canvas, b, self.seed ^ hands.len() as u64, cell);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 1, px, py + 1, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = bias(t, hands.last().copied());
        let kind = if b < 0.35 {
            "MAZE"
        } else if b > 0.65 {
            "LOOPS"
        } else {
            "MIX"
        };
        Some(format!("BIAS={b:.2}  {kind}  PAINT"))
    }

    fn reveal(&self) -> &'static str {
        "Truchet tiles (and the C64 one-liner 10 PRINT) show how two local \
         strokes plus randomness produce global mazes and loops. Bias the coin \
         and the topology shifts without changing the rule."
    }
}

#[cfg(test)]
mod tests {
    use super::Truchet;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Truchet::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("BIAS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn bias_changes() {
        let r = Truchet::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 24);
        Truchet::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Truchet::new().motif().unwrap().line.len() >= 6);
    }
}
