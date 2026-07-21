//! Pedal curve of a circle with respect to a point (limaçon family).
//!
//! DRAG: TUNE FOCUS. See `docs/ROOMS.md`.

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

fn focus(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    // distance of pedal point from center, in radii
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.1 + x * 1.8 + s
    } else {
        0.3 + phase_unit(t) * 1.4 + s
    }
}

fn draw(canvas: &mut dyn Surface, d: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.28;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.03
    };
    // base circle
    for i in 0..80 {
        let th = std::f64::consts::TAU * (i as f64 / 80.0);
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin()).round() as i32;
        canvas.plot(px, py, '.');
    }
    // focus point on x-axis of circle
    let fx = cx + d * r * 0.5;
    let fy = cy;
    canvas.plot(fx.round() as i32, fy.round() as i32, 'o');
    // pedal: foot of perpendicular from focus to each tangent
    // for circle, pedal is limaçon: rho = r + d cos phi in polar about focus
    let a = r;
    let b = d * r * 0.5;
    let steps = 360;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = std::f64::consts::TAU * (i as f64 / steps as f64) + j;
        // polar about focus: rho = a + b cos th  (relative to line to center)
        let rho = a + b * th.cos();
        let px = (fx + rho * th.cos()).round() as i32;
        let py = (fy - rho * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Pedal curve room.
#[derive(Debug, Default)]
pub struct Pedal {
    seed: u64,
}

impl Pedal {
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

impl Room for Pedal {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "pedal",
            title: "Pedal Curve",
            wing: "Shape & Space",
            blurb: "Feet of perpendiculars from a focus to circle tangents. t and DRAG: TUNE \
                    FOCUS.",
            accent: [160, 80, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, focus(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "pedal",
            root: 311.13,
            tempo: 90,
            line: &[0, 5, 7, 12, 14, 12, 7, 5],
            encodes: "perpendicular feet from a point to every tangent",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE FOCUS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = focus(t, None, self.seed);
        Some(format!("d={d:.2}  pedal  DRAG:FOCUS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = focus(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = focus(t, hands.last().copied(), self.seed);
        Some(format!("d={d:.2}  pedal focus"))
    }

    fn reveal(&self) -> &'static str {
        "The pedal of a curve with respect to a point P is the locus of feet \
         of perpendiculars from P to the tangents. For a circle that pedal is \
         a limaçon, a classical dual construction."
    }
}

#[cfg(test)]
mod tests {
    use super::Pedal;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Pedal::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("FOCUS") || s.contains("pedal"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn focus_changes() {
        let r = Pedal::new();
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
        Pedal::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
