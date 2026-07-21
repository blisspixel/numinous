//! Viviani's curve: intersection of a sphere and a tangent cylinder.
//!
//! DRAG: TUNE PHASE. See `docs/ROOMS.md`.

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

fn phase(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        x * 2.0 * std::f64::consts::PI + s
    } else {
        phase_unit(t) * 2.0 * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, ph: f64, scale: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.28 * scale.clamp(0.55, 1.45);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 8) as f64 * 0.04
    };
    // Sphere outline.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=48 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
        let px = (cx + a * th.cos()).round() as i32;
        let py = (cy - a * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Viviani: x = a(1+cos t), y = a sin t, z = 2 a sin(t/2)
    // (or classic: a(1+cos t), a sin t, 2a sin(t/2))
    prev = None;
    let steps = 200;
    for i in 0..=steps {
        let t = ph + 4.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let x = a * (1.0 + t.cos()) * 0.5;
        let y = a * t.sin();
        let z = 2.0 * a * (t * 0.5).sin() * 0.35;
        let xr = x * rot.cos() - y * rot.sin();
        let yr = x * rot.sin() + y * rot.cos();
        let d = 1.0 / (2.5 + z * 0.02);
        let px = (cx + (xr - a * 0.25) * d).round() as i32;
        let py = (cy - yr * 0.55 * d).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Viviani curve room.
#[derive(Debug, Default)]
pub struct Viviani {
    seed: u64,
}

impl Viviani {
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

impl Room for Viviani {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "viviani",
            title: "Viviani Curve",
            wing: "Shape & Space",
            blurb: "Sphere meets a tangent cylinder. t and DRAG: TUNE PHASE.",
            accent: [50, 120, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phase(t, None, self.seed), 1.0, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "viviani",
            root: 73.42,
            tempo: 78,
            line: &[0, 3, 5, 8, 12, 8, 5, 3],
            encodes: "Viviani: sphere-cylinder section, figure-eight on the sphere",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHASE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = phase(t, None, self.seed);
        Some(format!("p={p:.2}  viv  DRAG:PH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let hand = hands.last().copied();
        let p = phase(t, hand, self.seed);
        // Hand y scales the figure so phase drag is not a near-self-similar loop.
        let scale = hand.map_or(1.0, |(_, y)| 0.55 + y * 0.9);
        draw(canvas, p, scale, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let th = phase(t, hands.last().copied(), self.seed);
        let deg =
            (th.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * 360.0).floor() as i32;
        // Viviani: sphere and cylinder intersection; unit sphere curve.
        Some(format!("th={deg}deg  sph+cyl  viv"))
    }

    fn reveal(&self) -> &'static str {
        "Viviani's curve is the space curve cut by a sphere and a cylinder that \
         is tangent to the sphere and has half the sphere's radius. It draws a \
         figure-eight on the sphere and solved a 17th-century quadrature puzzle."
    }
}

#[cfg(test)]
mod tests {
    use super::Viviani;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Viviani::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("viv"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phase_changes() {
        let r = Viviani::new();
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
        Viviani::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
