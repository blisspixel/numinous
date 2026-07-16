//! Sphere geodesics: great circles are the shortest paths.
//!
//! DRAG: TUNE TILT. See `docs/ROOMS.md`.

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

fn tilt(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::FRAC_PI_2 + s
    } else {
        phase_unit(t) * std::f64::consts::FRAC_PI_2 + s
    }
}

fn draw(canvas: &mut dyn Surface, alpha: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.42;
    // Sphere outline.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=64 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 64.0);
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Equator.
    prev = None;
    for i in 0..=48 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
        let px = (cx + r * th.cos()).round() as i32;
        let py = cy.round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '-');
        }
        prev = Some((px, py));
    }
    // Great circle tilted by alpha about x-axis, orthographic.
    let alpha = alpha.clamp(0.0, std::f64::consts::FRAC_PI_2);
    let n_gc = 3 + if seed == 0 { 0 } else { (seed % 2) as usize };
    for g in 0..n_gc {
        let yaw = g as f64 * std::f64::consts::PI / n_gc as f64;
        prev = None;
        for i in 0..=80 {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / 80.0);
            // Point on unit circle in yz after tilt, then rotate by yaw.
            let y0 = th.cos();
            let z0 = th.sin();
            // Tilt about x: y' = y cos a - z sin a, z' = y sin a + z cos a
            let y1 = y0 * alpha.cos() - z0 * alpha.sin();
            let z1 = y0 * alpha.sin() + z0 * alpha.cos();
            let x1 = 0.0;
            // Yaw about z.
            let x = x1 * yaw.cos() - y1 * yaw.sin();
            let y = x1 * yaw.sin() + y1 * yaw.cos();
            let z = z1;
            // Backface cull lightly: only z >= -0.05
            if z < -0.15 {
                prev = None;
                continue;
            }
            let px = (cx + r * x).round() as i32;
            let py = (cy - r * y * 0.55).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if g == 0 { '#' } else { '*' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
}

/// Sphere geodesic room.
#[derive(Debug, Default)]
pub struct Geodesic {
    seed: u64,
}

impl Geodesic {
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

impl Room for Geodesic {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "geodesic",
            title: "Sphere Geodesics",
            wing: "Shape & Space",
            blurb: "Great circles are shortest paths. t and DRAG: TUNE TILT.",
            accent: [30, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, tilt(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "geodesic",
            root: 196.0,
            tempo: 74,
            line: &[0, 5, 7, 9, 12, 9, 7, 5],
            encodes: "on a sphere the shortest path is always a great circle arc",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TILT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = tilt(t, None, self.seed);
        let deg = a.to_degrees();
        Some(format!("tilt={deg:.0}deg  DRAG:TILT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = tilt(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = tilt(t, hands.last().copied(), self.seed);
        let deg = a.to_degrees();
        Some(format!("TILT={deg:.1}  great circ"))
    }

    fn reveal(&self) -> &'static str {
        "On a sphere the geodesics are great circles: planes through the center \
         cut the shortest paths. Airlines and ships follow them; parallel transport \
         around a triangle of geodesics shows the sphere's curvature as a turn."
    }
}

#[cfg(test)]
mod tests {
    use super::Geodesic;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Geodesic::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("tilt"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tilt_changes() {
        let r = Geodesic::new();
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
        Geodesic::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
