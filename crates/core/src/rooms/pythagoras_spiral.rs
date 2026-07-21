//! Spiral of Theodorus: right triangles with legs 1 and sqrt(n) stacked.
//!
//! DRAG: SET THE STEPS. See `docs/ROOMS.md`.

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

fn steps(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (4 + (x * 28.0) as usize).clamp(4, 36)
    } else {
        (8 + (phase_unit(t) * 20.0) as usize).clamp(4, 32)
    }
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let scale = (width.min(height) as f64) * 0.12 / (1.0 + (n as f64).sqrt() * 0.15).max(1.0);
    let rot0 = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.05
    };
    // Start with triangle on unit segment along x
    let mut px = 1.0_f64;
    let mut py = 0.0_f64;
    let mut ox = 0.0_f64;
    let mut oy = 0.0_f64;
    let to_screen = |x: f64, y: f64| -> (i32, i32) {
        let c = rot0.cos();
        let s = rot0.sin();
        let xr = c * x - s * y;
        let yr = s * x + c * y;
        (
            (cx + xr * scale * 8.0).round() as i32,
            (cy - yr * scale * 8.0).round() as i32,
        )
    };
    let (sx0, sy0) = to_screen(ox, oy);
    let (sx1, sy1) = to_screen(px, py);
    canvas.line(sx0, sy0, sx1, sy1, '#');
    for k in 1..=n {
        // unit perpendicular to current radial edge, outward
        let dx = px - ox;
        let dy = py - oy;
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        // rotate 90 deg for the new unit leg
        let ux = -dy / len;
        let uy = dx / len;
        let nx = px + ux;
        let ny = py + uy;
        let (ax, ay) = to_screen(px, py);
        let (bx, by) = to_screen(nx, ny);
        let (cxp, cyp) = to_screen(ox, oy);
        canvas.line(ax, ay, bx, by, if k % 2 == 0 { '*' } else { '+' });
        canvas.line(bx, by, cxp, cyp, '.');
        // next outer point is (nx,ny); origin stays; "previous outer" becomes current
        ox = px;
        oy = py;
        px = nx;
        py = ny;
        let _ = k;
    }
}

/// Spiral of Theodorus room.
#[derive(Debug, Default)]
pub struct PythagorasSpiral {
    seed: u64,
}

impl PythagorasSpiral {
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

impl Room for PythagorasSpiral {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "theodorus",
            title: "Spiral of Theodorus",
            wing: "Number & Pattern",
            blurb: "Stacked right triangles build a root spiral. t and DRAG: SET THE STEPS.",
            accent: [160, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, steps(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "theodorus",
            root: 116.54,
            tempo: 76,
            line: &[0, 2, 5, 7, 10, 12, 10, 7],
            encodes: "unit legs on growing hypotenuses spiral roots",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE STEPS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = steps(t, None);
        Some(format!("n={n}  roots  DRAG:STEPS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = steps(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = steps(t, hands.last().copied());
        let tip = (n as f64).sqrt();
        Some(format!("STEPS n={n}  |r|~{tip:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Theodorus of Cyrene stacked right triangles with a unit outer leg on \
         each prior hypotenuse. The free vertices spiral: at step n the radius \
         is sqrt(n), a classical picture of successive square roots."
    }
}

#[cfg(test)]
mod tests {
    use super::PythagorasSpiral;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PythagorasSpiral::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("STEPS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn steps_change() {
        let r = PythagorasSpiral::new();
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
        PythagorasSpiral::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
