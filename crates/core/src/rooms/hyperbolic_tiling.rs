//! {7,3} hyperbolic tiling sketch in the Poincare disc.
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
        (1 + (x * 4.0) as usize).clamp(1, 5)
    } else {
        (2 + (phase_unit(t) * 2.0) as usize).clamp(1, 4)
    }
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.46;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 13) as f64 * 0.04
    };
    // boundary
    for i in 0..96 {
        let th = std::f64::consts::TAU * (i as f64 / 96.0);
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin()).round() as i32;
        canvas.plot(px, py, '#');
    }
    // heptagon layers: approximate by regular n-gons at hyperbolic radii
    for layer in 1..=d {
        let r = rad * (1.0 - (-0.55 * layer as f64).exp());
        let n = 7 * layer;
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=n {
            let th = rot + std::f64::consts::TAU * (i as f64 / n as f64);
            let px = (cx + r * th.cos()).round() as i32;
            let py = (cy - r * th.sin()).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, if layer % 2 == 0 { '*' } else { '+' });
            }
            prev = Some((px, py));
        }
        // radial spokes every 7th vertex
        for k in 0..7 {
            let th = rot + std::f64::consts::TAU * (k as f64 / 7.0);
            let x0 = (cx + 0.08 * rad * th.cos()).round() as i32;
            let y0 = (cy - 0.08 * rad * th.sin()).round() as i32;
            let x1 = (cx + r * th.cos()).round() as i32;
            let y1 = (cy - r * th.sin()).round() as i32;
            canvas.line(x0, y0, x1, y1, '.');
        }
    }
}

/// Hyperbolic tiling room.
#[derive(Debug, Default)]
pub struct HyperbolicTiling {
    seed: u64,
}

impl HyperbolicTiling {
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

impl Room for HyperbolicTiling {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hyperbolic-tiling",
            title: "Hyperbolic Tiling",
            wing: "Shape & Space",
            blurb: "{7,3}-style lattice in the Poincare disc. t and DRAG: SET THE DEPTH.",
            accent: [120, 40, 160],
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
            key: "hyperbolic tiling",
            root: 466.16,
            tempo: 78,
            line: &[0, 5, 7, 12, 14, 12, 7, 5],
            encodes: "seven around a point needs hyperbolic room",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  H2 tile  DRAG:D"))
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
        // Layer k draws 7k vertices in this sketch; total verts ~ 7 d(d+1)/2.
        let verts = 7 * d * (d + 1) / 2;
        Some(format!("d={d}  verts~{verts}  {{7,3}}"))
    }

    fn reveal(&self) -> &'static str {
        "In the plane only three, four, or six equilateral triangles fit around \
         a point. Seven need negative curvature: the hyperbolic plane, drawn \
         here as nested heptagons in the Poincare disc."
    }
}

#[cfg(test)]
mod tests {
    use super::HyperbolicTiling;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HyperbolicTiling::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH") || s.contains("depth"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = HyperbolicTiling::new();
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
        HyperbolicTiling::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
