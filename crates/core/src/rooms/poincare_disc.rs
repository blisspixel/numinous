//! Poincare disc: hyperbolic geometry in the unit disk.
//!
//! DRAG: SET THE ORDER. See `docs/ROOMS.md`.

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

fn order(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (5 + (x * 6.0) as usize).clamp(5, 12)
    } else {
        (5 + (phase_unit(t) * 5.0) as usize).clamp(5, 11)
    }
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.46;
    // boundary circle
    for i in 0..128 {
        let th = std::f64::consts::TAU * (i as f64 / 128.0);
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin()).round() as i32;
        canvas.plot(px, py, '#');
    }
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 13) as f64 * 0.04
    };
    // hyperbolic geodesics: circular arcs orthogonal to the boundary
    // Draw n radial "spokes" as diameters and nested arcs
    for k in 0..n {
        let th = rot + std::f64::consts::TAU * (k as f64 / n as f64);
        // diameter geodesic
        let x0 = (cx + rad * th.cos()).round() as i32;
        let y0 = (cy - rad * th.sin()).round() as i32;
        let x1 = (cx - rad * th.cos()).round() as i32;
        let y1 = (cy + rad * th.sin()).round() as i32;
        canvas.line(x0, y0, x1, y1, if k % 2 == 0 { '*' } else { '+' });
    }
    // concentric Euclidean circles (not geodesics, but show conformal shells)
    for ring in 1..5 {
        let r = rad * (ring as f64 / 5.0);
        for i in 0..64 {
            let th = std::f64::consts::TAU * (i as f64 / 64.0) + rot * 0.3;
            let px = (cx + r * th.cos()).round() as i32;
            let py = (cy - r * th.sin()).round() as i32;
            canvas.plot(px, py, '.');
        }
    }
    // Sample orthogonal arcs: circles through two boundary points
    for k in 0..n.min(8) {
        let a = rot + std::f64::consts::TAU * (k as f64 / n as f64);
        let b = a + std::f64::consts::PI * 0.55;
        // Midpoint chord; approximate arc by polyline inside disk
        let mut prev: Option<(i32, i32)> = None;
        for s in 0..=40 {
            let u = s as f64 / 40.0;
            let th = a + (b - a) * u;
            // Push inward with a bulge factor so chord bows as hyperbolic arc
            let bulge = 0.55 + 0.35 * (std::f64::consts::PI * u).sin();
            let px = (cx + rad * bulge * th.cos()).round() as i32;
            let py = (cy - rad * bulge * th.sin()).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, '+');
            }
            prev = Some((px, py));
        }
    }
}

/// Poincare disc room.
#[derive(Debug, Default)]
pub struct PoincareDisc {
    seed: u64,
}

impl PoincareDisc {
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

impl Room for PoincareDisc {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "poincare-disc",
            title: "Poincare Disc",
            wing: "Shape & Space",
            blurb: "Hyperbolic plane inside a circle. t and DRAG: SET THE ORDER.",
            accent: [100, 60, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, order(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "poincare disc",
            root: 277.18,
            tempo: 82,
            line: &[0, 4, 8, 12, 8, 4, 0, 6],
            encodes: "infinite hyperbolic area packed into a disk",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = order(t, None);
        Some(format!("n={n}  disc  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = order(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
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
        let n = order(t, hands.last().copied());
        // n geodesic diameters; interior angle of ideal n-gon is 0 in H2.
        let step_deg = 360.0 / n as f64;
        Some(format!("n={n}  step={step_deg:.0}deg  H2"))
    }

    fn reveal(&self) -> &'static str {
        "The Poincare disc model places the entire hyperbolic plane inside a \
         unit circle. Geodesics are diameters or arcs orthogonal to the rim; \
         angles are true, distances explode near the boundary."
    }
}

#[cfg(test)]
mod tests {
    use super::PoincareDisc;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PoincareDisc::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = PoincareDisc::new();
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
        PoincareDisc::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
