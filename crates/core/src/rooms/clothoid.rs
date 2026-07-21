//! Clothoid (Euler spiral): curvature linear in arc length.
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

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

fn scale(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.6 + s
    } else {
        0.6 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = a.clamp(0.3, 2.5);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.04
    };
    // Fresnel integrals: x = int cos(s^2/(2a^2)) ds, y = int sin(...)
    let steps = 400;
    let s_max = 4.0 * a;
    let ds = s_max / steps as f64;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut pts = Vec::with_capacity(steps + 1);
    pts.push((0.0, 0.0));
    for i in 1..=steps {
        let s = i as f64 * ds;
        let th = s * s / (2.0 * a * a);
        x += th.cos() * ds;
        y += th.sin() * ds;
        pts.push((x, y));
    }
    // also negative branch
    let mut xn = 0.0;
    let mut yn = 0.0;
    let mut pts_n = Vec::with_capacity(steps + 1);
    pts_n.push((0.0, 0.0));
    for i in 1..=steps {
        let s = -(i as f64) * ds;
        let th = s * s / (2.0 * a * a);
        xn += th.cos() * (-ds);
        yn += th.sin() * (-ds);
        pts_n.push((xn, yn));
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(px, py) in pts.iter().chain(pts_n.iter()) {
        min_x = min_x.min(px);
        max_x = max_x.max(px);
        min_y = min_y.min(py);
        max_y = max_y.max(py);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let c = rot.cos();
    let s = rot.sin();
    let map = |px: f64, py: f64| -> (i32, i32) {
        let rx = c * px - s * py;
        let ry = s * px + c * py;
        // recompute bounds lightly via unrotated span
        let u = ((rx - min_x) / dx).clamp(0.0, 1.0);
        let v = ((ry - min_y) / dy).clamp(0.0, 1.0);
        let ix = ((0.1 + 0.8 * u) * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((0.1 + 0.8 * (1.0 - v)) * height.saturating_sub(1) as f64).round() as i32;
        (ix, iy)
    };
    let mut prev: Option<(i32, i32)> = None;
    for &(px, py) in &pts {
        let (ix, iy) = map(px, py);
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, ix, iy, '#');
        }
        prev = Some((ix, iy));
    }
    prev = None;
    for &(px, py) in &pts_n {
        let (ix, iy) = map(px, py);
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, ix, iy, '*');
        }
        prev = Some((ix, iy));
    }
}

/// Clothoid room.
#[derive(Debug, Default)]
pub struct Clothoid {
    seed: u64,
}

impl Clothoid {
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

impl Room for Clothoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "clothoid",
            title: "Clothoid",
            wing: "Shape & Space",
            blurb: "Euler spiral: curvature grows with arc length. t and DRAG: TUNE SCALE.",
            accent: [60, 100, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "clothoid",
            root: 830.61,
            tempo: 90,
            line: &[0, 3, 5, 8, 12, 8, 5, 3],
            encodes: "highway transition curve of linear curvature",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        Some(format!("a={a:.2}  clothoid  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = scale(t, hands.last().copied(), self.seed).clamp(0.3, 2.5);
        // Clothoid: curvature kappa = s / a^2, so dkappa/ds = 1/a^2.
        let dks = 1.0 / (a * a);
        Some(format!("a={a:.2}  dkappa/ds={dks:.2}  Fresnel"))
    }

    fn reveal(&self) -> &'static str {
        "A clothoid (Euler spiral) has curvature proportional to arc length. \
         Road and rail transitions use it so steering changes at a constant \
         rate. Its coordinates are Fresnel integrals."
    }
}

#[cfg(test)]
mod tests {
    use super::Clothoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Clothoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("clothoid"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Clothoid::new();
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
        Clothoid::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
