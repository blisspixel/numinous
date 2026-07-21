//! Maclaurin trisectrix: angle-trisecting classical curve.
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

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

fn param_a(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.35 + x * 0.6 + s
    } else {
        0.45 + phase_unit(t) * 0.45 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = a.clamp(0.3, 1.0);
    let scale = (width.min(height) as f64) * 0.35 * a;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.04
    };
    // Polar: r = 2 a sin(3 th) / sin(2 th)  for th in (-pi/3, pi/3) excluding 0 issues.
    // Equivalent: r = a (4 cos th - sec th) for |th| < pi/2.
    let steps = 280;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let th = (-0.9 + 1.8 * u) * std::f64::consts::FRAC_PI_2;
        let c = th.cos();
        if c.abs() < 0.08 {
            prev = None;
            continue;
        }
        let r = a * (4.0 * c - 1.0 / c);
        if !r.is_finite() || r.abs() > 4.0 {
            prev = None;
            continue;
        }
        let ang = th + rot;
        let x = r * ang.cos();
        let y = r * ang.sin();
        let px = (cx + x * scale).round() as i32;
        let py = (cy - y * scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // Construction circle of radius a.
    let mut prev_c: Option<(i32, i32)> = None;
    for i in 0..=48 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
        let px = (cx + a * scale * th.cos()).round() as i32;
        let py = (cy - a * scale * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev_c {
            canvas.line(ox, oy, px, py, '.');
        }
        prev_c = Some((px, py));
    }
}

/// Maclaurin trisectrix room.
#[derive(Debug, Default)]
pub struct Trisectrix {
    seed: u64,
}

impl Trisectrix {
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

impl Room for Trisectrix {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "trisectrix",
            title: "Maclaurin Trisectrix",
            wing: "Shape & Space",
            blurb: "Classical curve that trisects angles. t and DRAG: TUNE A.",
            accent: [150, 70, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, param_a(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "trisectrix",
            root: 207.65,
            tempo: 74,
            line: &[0, 3, 5, 8, 12, 8, 5, 3],
            encodes: "Maclaurin trisectrix turns angle trisection into geometry",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = param_a(t, None, self.seed);
        Some(format!("a={a:.2}  1/3  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = param_a(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = param_a(t, hands.last().copied(), self.seed);
        // Maclaurin trisectrix loop area ~ 0.5 (pi - 3sqrt3/2) a^2.
        let loop_a = 0.5 * (std::f64::consts::PI - 1.5 * 3.0_f64.sqrt()) * a * a;
        Some(format!("a={a:.2}  loopA={loop_a:.2}  1/3"))
    }

    fn reveal(&self) -> &'static str {
        "Maclaurin's trisectrix is a classical curve that solves angle trisection \
         with straightedge, compass, and one fixed curve. Its polar form r = a \
         (4 cos th - sec th) encodes the triple-angle identity in geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::Trisectrix;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Trisectrix::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("1/3"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Trisectrix::new();
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
        Trisectrix::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
