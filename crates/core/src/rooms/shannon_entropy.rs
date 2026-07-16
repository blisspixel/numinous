//! Shannon entropy of a binary source with tunable bias.
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

fn bias(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        (0.02 + x * 0.96 + s).clamp(0.01, 0.99)
    } else {
        (0.05 + phase_unit(t) * 0.9 + s).clamp(0.01, 0.99)
    }
}

fn entropy_bits(p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    let q = 1.0 - p;
    -(p * p.log2() + q * q.log2())
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let h = entropy_bits(p);
    // H(p) curve across width
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = (col as f64) / width.saturating_sub(1).max(1) as f64;
        let px = (0.01 + x * 0.98).clamp(0.01, 0.99);
        let hy = entropy_bits(px);
        let y = ((1.0 - hy) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, y, '#');
        }
        prev = Some((col as i32, y));
    }
    // marker at current p
    let mx = (p * width.saturating_sub(1) as f64).round() as i32;
    let my = ((1.0 - h) * height.saturating_sub(1) as f64).round() as i32;
    canvas.line(mx, 0, mx, height as i32 - 1, '|');
    canvas.line(mx - 2, my, mx + 2, my, 'o');
    // bar for bits
    let bar = ((h / 1.0) * (width as f64) * 0.9).round() as i32;
    let by = height as i32 - 2 - if seed == 0 { 0 } else { (seed % 2) as i32 };
    if by > 0 {
        canvas.line(1, by, 1 + bar, by, '=');
    }
}

/// Shannon entropy room.
#[derive(Debug, Default)]
pub struct ShannonEntropy {
    seed: u64,
}

impl ShannonEntropy {
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

impl Room for ShannonEntropy {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "shannon-entropy",
            title: "Shannon Entropy",
            wing: "Chance & Noise",
            blurb: "H(p) for a biased coin. t and DRAG: TUNE P.",
            accent: [50, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, bias(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "shannon-entropy",
            root: 261.63,
            tempo: 74,
            line: &[0, 4, 7, 11, 7, 4, 0, 4],
            encodes: "binary entropy H(p)=-p log p -(1-p)log(1-p) peaks at fair",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = bias(t, None, self.seed);
        let h = entropy_bits(p);
        Some(format!("p={p:.2}  H={h:.2}b  DRAG:P"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = bias(t, hands.last().copied(), self.seed);
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
        let p = bias(t, hands.last().copied(), self.seed);
        let h = entropy_bits(p);
        Some(format!("P={p:.3}  H={h:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Shannon entropy measures surprise. For a coin with heads probability p, \
         H(p) = -p log2 p - (1-p) log2(1-p) bits. A fair coin is one full bit; \
         a two-headed coin is zero: no surprise left."
    }
}

#[cfg(test)]
mod tests {
    use super::ShannonEntropy;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ShannonEntropy::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains('H'));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = ShannonEntropy::new();
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
        ShannonEntropy::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
