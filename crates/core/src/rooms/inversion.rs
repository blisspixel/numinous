//! Circle inversion: the mirror that bends lines into circles.
//!
//! Invert the plane in a circle of radius R: points fly to their inverse
//! distance. Lines through the center stay lines; other lines become circles
//! through the inverse of infinity. DRAG: MOVE THE MIRROR. See `docs/ROOMS.md`.

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

fn invert(px: f64, py: f64, cx: f64, cy: f64, r2: f64) -> (f64, f64) {
    let dx = px - cx;
    let dy = py - cy;
    let d2 = dx * dx + dy * dy;
    if d2 < 1e-10 {
        return (cx + 1e6, cy);
    }
    let s = r2 / d2;
    (cx + dx * s, cy + dy * s)
}

fn draw(canvas: &mut dyn Surface, cx: f64, cy: f64, radius: f64, t: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (y.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let r2 = radius * radius;
    // Mirror circle.
    let steps = 80;
    let mut prev: Option<(i32, i32)> = None;
    for s in 0..=steps {
        let a = std::f64::consts::TAU * s as f64 / steps as f64;
        let p = to_px(cx + radius * a.cos(), cy + radius * a.sin());
        if let Some(o) = prev {
            canvas.line(o.0, o.1, p.0, p.1, '#');
        }
        prev = Some(p);
    }
    // Grid of lines and a few circles, then their inverses.
    let phase = phase_unit(t);
    for i in 0..6 {
        let y = 0.15 + i as f64 * 0.12 + phase * 0.02;
        // Horizontal line samples.
        let mut prev_l: Option<(i32, i32)> = None;
        let mut prev_i: Option<(i32, i32)> = None;
        for s in 0..=40 {
            let x = 0.05 + s as f64 * 0.022;
            let lp = to_px(x, y);
            if let Some(o) = prev_l {
                canvas.line(o.0, o.1, lp.0, lp.1, '.');
            }
            prev_l = Some(lp);
            let (ix, iy) = invert(x, y, cx, cy, r2);
            if (0.0..=1.0).contains(&ix) && (0.0..=1.0).contains(&iy) {
                let ip = to_px(ix, iy);
                if let Some(o) = prev_i {
                    canvas.line(o.0, o.1, ip.0, ip.1, '*');
                }
                prev_i = Some(ip);
            } else {
                prev_i = None;
            }
        }
    }
    // A circle not centered on the mirror.
    let scx = 0.72;
    let scy = 0.28 + phase * 0.1;
    let sr = 0.12;
    prev = None;
    let mut prev_i: Option<(i32, i32)> = None;
    for s in 0..=48 {
        let a = std::f64::consts::TAU * s as f64 / 48.0;
        let x = scx + sr * a.cos();
        let y = scy + sr * a.sin();
        let p = to_px(x, y);
        if let Some(o) = prev {
            canvas.line(o.0, o.1, p.0, p.1, '+');
        }
        prev = Some(p);
        let (ix, iy) = invert(x, y, cx, cy, r2);
        if (0.0..=1.0).contains(&ix) && (0.0..=1.0).contains(&iy) {
            let ip = to_px(ix, iy);
            if let Some(o) = prev_i {
                canvas.line(o.0, o.1, ip.0, ip.1, 'o');
            }
            prev_i = Some(ip);
        } else {
            prev_i = None;
        }
    }
    let c = to_px(cx, cy);
    canvas.plot(c.0, c.1, 'O');
}

/// Circle Inversion room.
#[derive(Debug, Default)]
pub struct Inversion {
    seed: u64,
}

impl Inversion {
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

impl Room for Inversion {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "inversion",
            title: "The Mirror That Bends",
            wing: "Shape & Space",
            blurb: "Circle inversion: lines become circles, infinity becomes a point. The hub of \
                    Apollonian and Steiner geometry. t drifts props; DRAG: MOVE THE MIRROR.",
            accent: [140, 180, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let cx = 0.48
            + if self.seed == 0 {
                0.0
            } else {
                ((self.seed % 5) as f64 - 2.0) * 0.02
            };
        let cy = 0.52;
        let radius = 0.22 + phase_unit(t) * 0.08;
        draw(canvas, cx, cy, radius, t);
    }

    fn postcard_t(&self) -> f64 {
        0.3
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "invert plane",
            root: 233.08,
            tempo: 101,
            line: &[0, 2, 7, 14, 7, 2, 9, 0],
            encodes: "distance product constant: lines become circles",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: MOVE THE MIRROR")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = 0.22 + phase_unit(t) * 0.08;
        Some(format!("R={r:.2}  invert  DRAG:MIRROR"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (cx, cy) = hands.last().copied().unwrap_or((0.48, 0.52));
        let radius = 0.2 + phase_unit(t) * 0.1;
        draw(canvas, cx, cy, radius, t);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let r = 0.2 + phase_unit(t) * 0.1;
        Some(format!(
            "MIRROR@{:.0}%{:.0}%  R={r:.2}",
            x * 100.0,
            y * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "Inversion in a circle sends each point along its ray so that the product \
         of distances from the center is R^2. Circles and lines map to circles and \
         lines; angles are preserved. It is the master key of inversive geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::{Inversion, invert};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Inversion::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("MIRROR"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn drag_changes() {
        let r = Inversion::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.3,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn double_invert_identity() {
        let (x, y) = invert(0.8, 0.3, 0.5, 0.5, 0.04);
        let (x2, y2) = invert(x, y, 0.5, 0.5, 0.04);
        assert!((x2 - 0.8).abs() < 1e-9);
        assert!((y2 - 0.3).abs() < 1e-9);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Inversion::new().render(&mut c, 0.25);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Inversion::new().motif().unwrap().line.len() >= 6);
    }
}
