//! Koch snowflake: infinite perimeter, finite area.
//!
//! Start with a triangle; replace every edge's middle third with two sides of
//! an equilateral bump. Iterate. Perimeter grows without bound while area
//! converges. DRAG: ADD A GENERATION. See `docs/ROOMS.md`.

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

fn gens(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 5.0) as usize).clamp(1, 5)
    } else {
        (1 + (phase_unit(t) * 4.0) as usize).clamp(1, 4)
    }
}

fn koch_edge(a: (f64, f64), b: (f64, f64), depth: usize, out: &mut Vec<(f64, f64)>) {
    if depth == 0 {
        out.push(a);
        return;
    }
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let p1 = (a.0 + dx / 3.0, a.1 + dy / 3.0);
    let p2 = (a.0 + 2.0 * dx / 3.0, a.1 + 2.0 * dy / 3.0);
    // Peak: rotate (b-a)/3 by 60 degrees around p1.
    let rx = dx / 3.0;
    let ry = dy / 3.0;
    let peak = (
        p1.0 + 0.5 * rx - 0.866_025_403_78 * ry,
        p1.1 + 0.866_025_403_78 * rx + 0.5 * ry,
    );
    koch_edge(a, p1, depth - 1, out);
    koch_edge(p1, peak, depth - 1, out);
    koch_edge(peak, p2, depth - 1, out);
    koch_edge(p2, b, depth - 1, out);
}

fn snowflake(depth: usize) -> Vec<(f64, f64)> {
    let a = (0.18, 0.72);
    let b = (0.82, 0.72);
    let c = (0.50, 0.18);
    let mut pts = Vec::new();
    koch_edge(a, b, depth, &mut pts);
    koch_edge(b, c, depth, &mut pts);
    koch_edge(c, a, depth, &mut pts);
    pts.push(a);
    pts
}

fn perimeter_factor(depth: usize) -> f64 {
    // Each generation multiplies edge count by 4/3 relative length.
    (4.0_f64 / 3.0).powi(depth as i32)
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        (
            (p.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (p.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    for w in pts.windows(2) {
        let a = to_px(w[0]);
        let b = to_px(w[1]);
        canvas.line(a.0, a.1, b.0, b.1, '*');
    }
}

/// Koch snowflake room.
#[derive(Debug, Default)]
pub struct Koch {
    seed: u64,
}

impl Koch {
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

impl Room for Koch {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "koch",
            title: "The Infinite Coast",
            wing: "Fractals",
            blurb: "Koch snowflake: every generation multiplies the coast by 4/3. Perimeter runs \
                    away; area stays finite. t and DRAG: ADD A GENERATION.",
            accent: [140, 200, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let g = gens(t, None)
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 2) as usize
            };
        let pts = snowflake(g.min(5));
        draw(canvas, &pts);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "koch coast",
            root: 293.66,
            tempo: 88,
            line: &[0, 5, 7, 12, 16, 12, 7, 5],
            encodes: "perimeter times four-thirds each generation forever",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: ADD A GENERATION")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = gens(t, None);
        let p = perimeter_factor(g);
        Some(format!("gen={g}  perim~{p:.2}x  DRAG:GEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = gens(t, hands.last().copied());
        let pts = snowflake(g);
        draw(canvas, &pts);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let g = gens(t, hands.last().copied());
        let p = perimeter_factor(g);
        let edges = 3 * 4usize.pow(g as u32);
        Some(format!("GEN={g}  edges={edges}  perim~{p:.1}x"))
    }

    fn reveal(&self) -> &'static str {
        "The Koch snowflake has infinite perimeter and finite area: each step \
         multiplies length by 4/3 while the added triangles shrink fast enough \
         to sum. A coast that never ends inside a finite sea."
    }
}

#[cfg(test)]
mod tests {
    use super::{Koch, perimeter_factor, snowflake};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Koch::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("GEN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn gen_changes() {
        let r = Koch::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
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
    fn perimeter_grows() {
        assert!((perimeter_factor(1) - 4.0 / 3.0).abs() < 1e-9);
        assert!(perimeter_factor(3) > perimeter_factor(2));
        assert!(snowflake(2).len() > snowflake(1).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Koch::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Koch::new().motif().unwrap().line.len() >= 6);
    }
}
