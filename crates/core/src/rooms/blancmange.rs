//! Blancmange / Takagi curve: continuous nowhere-differentiable fractal graph.
//!
//! DRAG: SET THE DEPTH. See `docs/ROOMS.md`.

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

fn depth(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (2 + (x * 10.0) as usize).clamp(2, 14)
    } else {
        (4 + (phase_unit(t) * 8.0) as usize).clamp(2, 12)
    }
}

fn saw(x: f64) -> f64 {
    let f = x.fract().abs();
    if f <= 0.5 { f } else { 1.0 - f }
}

fn takagi(x: f64, n: usize) -> f64 {
    let mut s = 0.0;
    let mut w = 1.0;
    let mut xx = x;
    for _ in 0..n {
        s += w * saw(xx);
        xx *= 2.0;
        w *= 0.5;
    }
    s
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.002
    };
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64 + j;
        let y = takagi(x.rem_euclid(1.0), n);
        // y in [0, 1] roughly; map with margin
        let py = ((1.0 - y.clamp(0.0, 1.0) * 0.9 - 0.05) * height.saturating_sub(1) as f64).round()
            as i32;
        let px = col as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, if n >= 8 { '#' } else { '*' });
        } else {
            canvas.plot(px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Blancmange curve room.
#[derive(Debug, Default)]
pub struct Blancmange {
    seed: u64,
}

impl Blancmange {
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

impl Room for Blancmange {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "blancmange",
            title: "Blancmange Curve",
            wing: "Fractals",
            blurb: "Takagi's continuous graph with no tangent anywhere. t and DRAG: SET THE DEPTH.",
            accent: [220, 180, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "blancmange",
            root: 233.08,
            tempo: 72,
            line: &[0, 2, 4, 7, 4, 2, 0, 7],
            encodes: "tent sum that is continuous yet never smooth",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  blanc  DRAG:DEPTH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
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
        let d = depth(t, hands.last().copied());
        let mid = takagi(0.5, d);
        Some(format!("DEPTH={d}  T(1/2)={mid:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The blancmange (Takagi) function sums scaled tent maps. The graph is \
         continuous everywhere and differentiable nowhere: a fractal mountain \
         named for a molded dessert."
    }
}

#[cfg(test)]
mod tests {
    use super::Blancmange;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Blancmange::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = Blancmange::new();
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
        Blancmange::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}
