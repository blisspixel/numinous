//! H-tree: self-similar H strokes filling the plane (canopy / IC routing motif).
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
        (1 + (x * 7.0) as usize).clamp(1, 8)
    } else {
        (2 + (phase_unit(t) * 5.0) as usize).clamp(1, 7)
    }
}

fn h_stroke(
    canvas: &mut dyn Surface,
    cx: f64,
    cy: f64,
    half: f64,
    vertical: bool,
    n: usize,
    ink: char,
) {
    if n == 0 || half < 0.5 {
        return;
    }
    if vertical {
        // vertical bar of H
        let x0 = cx.round() as i32;
        let y0 = (cy - half).round() as i32;
        let y1 = (cy + half).round() as i32;
        canvas.line(x0, y0, x0, y1, ink);
        // two horizontal caps
        let x_l = (cx - half).round() as i32;
        let x_r = (cx + half).round() as i32;
        canvas.line(x_l, y0, x_r, y0, ink);
        canvas.line(x_l, y1, x_r, y1, ink);
        let nh = half * 0.5_f64.sqrt();
        h_stroke(canvas, cx - half, cy - half, nh, false, n - 1, ink);
        h_stroke(canvas, cx + half, cy - half, nh, false, n - 1, ink);
        h_stroke(canvas, cx - half, cy + half, nh, false, n - 1, ink);
        h_stroke(canvas, cx + half, cy + half, nh, false, n - 1, ink);
    } else {
        // horizontal bar
        let y0 = cy.round() as i32;
        let x0 = (cx - half).round() as i32;
        let x1 = (cx + half).round() as i32;
        canvas.line(x0, y0, x1, y0, ink);
        let y_t = (cy - half).round() as i32;
        let y_b = (cy + half).round() as i32;
        canvas.line(x0, y_t, x0, y_b, ink);
        canvas.line(x1, y_t, x1, y_b, ink);
        let nh = half * 0.5_f64.sqrt();
        h_stroke(canvas, cx - half, cy - half, nh, true, n - 1, ink);
        h_stroke(canvas, cx + half, cy - half, nh, true, n - 1, ink);
        h_stroke(canvas, cx - half, cy + half, nh, true, n - 1, ink);
        h_stroke(canvas, cx + half, cy + half, nh, true, n - 1, ink);
    }
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 - 2.0
        };
    let cy = (height.saturating_sub(1) / 2) as f64;
    let half = (width.min(height) as f64) * 0.28;
    let ink = if d >= 5 { '#' } else { '*' };
    h_stroke(canvas, cx, cy, half, true, d, ink);
}

/// H-tree room.
#[derive(Debug, Default)]
pub struct HTree {
    seed: u64,
}

impl HTree {
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

impl Room for HTree {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "h-tree",
            title: "H-Tree",
            wing: "Fractals",
            blurb: "Self-similar H strokes that tile the plane. t and DRAG: SET THE DEPTH.",
            accent: [80, 140, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "h tree",
            root: 123.47,
            tempo: 78,
            line: &[0, 7, 12, 7, 0, 7, 12, 0],
            encodes: "orthogonal H strokes at every scale",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  H-tree  DRAG:DEPTH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = depth(t, hands.last().copied());
        // segment count roughly 1 + 4 + 4^2 + ... for H tree
        let segs = (4_u64.pow(d as u32) - 1) / 3;
        Some(format!("DEPTH={d}  segs~{segs}"))
    }

    fn reveal(&self) -> &'static str {
        "An H-tree draws a capital H, then smaller H's on each endpoint, \
         forever. Length grows without bound while the figure stays in a box: \
         a plane-filling motif used in canopies and chip routing."
    }
}

#[cfg(test)]
mod tests {
    use super::HTree;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HTree::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = HTree::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        HTree::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
