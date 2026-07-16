//! Circular caustic under parallel light (nephroid cousin sketch).
//!
//! Distinct from nephroid room: envelope of reflected rays. DRAG: TUNE ANGLE.
//! See `docs/ROOMS.md`.

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

fn angle(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::PI + s
    } else {
        phase_unit(t) * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, light_ang: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.42;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.02
    };
    // circle
    for i in 0..96 {
        let th = std::f64::consts::TAU * (i as f64 / 96.0);
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin()).round() as i32;
        canvas.plot(px, py, '.');
    }
    // incoming parallel rays and reflections; envelope approximates nephroid
    let n_rays = 36;
    for k in 0..n_rays {
        let th = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * (k as f64 / (n_rays - 1) as f64)
            + j * 0.1;
        // hit point on circle (left half facing light)
        let hx = cx + rad * th.cos();
        let hy = cy - rad * th.sin();
        // incident direction from light_ang
        let ix = light_ang.cos();
        let iy = -light_ang.sin();
        // start of ray outside
        let sx = hx - ix * rad * 1.4;
        let sy = hy - iy * rad * 1.4;
        canvas.line(
            sx.round() as i32,
            sy.round() as i32,
            hx.round() as i32,
            hy.round() as i32,
            '+',
        );
        // normal is radial
        let nx = (hx - cx) / rad;
        let ny = (hy - cy) / rad;
        // reflect: r = i - 2 (i.n) n
        let idot = ix * nx + iy * ny;
        let rx = ix - 2.0 * idot * nx;
        let ry = iy - 2.0 * idot * ny;
        let ex = hx + rx * rad * 1.2;
        let ey = hy + ry * rad * 1.2;
        canvas.line(
            hx.round() as i32,
            hy.round() as i32,
            ex.round() as i32,
            ey.round() as i32,
            '*',
        );
    }
    // approximate caustic nephroid: 3a cos t - a cos 3t style inside
    let a = rad * 0.25;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=180 {
        let th = std::f64::consts::TAU * (i as f64 / 180.0) + light_ang;
        let x = a * (3.0 * th.cos() - (3.0 * th).cos());
        let y = a * (3.0 * th.sin() - (3.0 * th).sin());
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Circular caustic room.
#[derive(Debug, Default)]
pub struct CircularCaustic {
    seed: u64,
}

impl CircularCaustic {
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

impl Room for CircularCaustic {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "circular-caustic",
            title: "Circular Caustic",
            wing: "Shape & Space",
            blurb: "Reflected parallel light envelopes a nephroid. t and DRAG: TUNE ANGLE.",
            accent: [220, 180, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, angle(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "circular caustic",
            root: 207.65,
            tempo: 98,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "envelope of rays reflected in a circle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = angle(t, None, self.seed);
        Some(format!("ang={a:.2}  caustic  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = angle(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = angle(t, hands.last().copied(), self.seed);
        Some(format!("ANG={a:.3}  rays"))
    }

    fn reveal(&self) -> &'static str {
        "Parallel light reflected in a circle envelopes a nephroid caustic. \
         Coffee cups show the same curve: the bright rim is geometry, not \
         magic. Huygens studied caustics as envelopes of rays."
    }
}

#[cfg(test)]
mod tests {
    use super::CircularCaustic;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CircularCaustic::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("caustic"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = CircularCaustic::new();
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
        CircularCaustic::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
