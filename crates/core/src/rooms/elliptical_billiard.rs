//! Elliptical billiard: caustics and string construction.
//!
//! Trajectories in an ellipse with conserved product of angular momenta.
//! DRAG: SET THE LAUNCH. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const BOUNCES: usize = 80;

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

fn launch(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    // Start angle on the ellipse and direction angle.
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.05
    };
    if let Some((x, y)) = hand {
        (x * std::f64::consts::TAU + s, y * std::f64::consts::PI)
    } else {
        let u = phase_unit(t);
        (u * std::f64::consts::TAU + s, 0.4 + u * 0.8)
    }
}

fn ellipse_point(a: f64, b: f64, phi: f64) -> (f64, f64) {
    (a * phi.cos(), b * phi.sin())
}

fn normal(a: f64, b: f64, x: f64, y: f64) -> (f64, f64) {
    // Gradient of x^2/a^2 + y^2/b^2 - 1.
    let mut nx = 2.0 * x / (a * a);
    let mut ny = 2.0 * y / (b * b);
    let n = (nx * nx + ny * ny).sqrt().max(1e-9);
    nx /= n;
    ny /= n;
    (nx, ny)
}

fn reflect(dx: f64, dy: f64, nx: f64, ny: f64) -> (f64, f64) {
    let dot = dx * nx + dy * ny;
    (dx - 2.0 * dot * nx, dy - 2.0 * dot * ny)
}

fn next_hit(a: f64, b: f64, x: f64, y: f64, dx: f64, dy: f64) -> (f64, f64, f64, f64) {
    // Ray (x,y)+t(dx,dy) hits ellipse; take smallest t>eps.
    // (x+t dx)^2/a^2 + (y+t dy)^2/b^2 = 1
    let aa = a * a;
    let bb = b * b;
    let a_coef = (dx * dx) / aa + (dy * dy) / bb;
    let b_coef = 2.0 * (x * dx / aa + y * dy / bb);
    let c_coef = (x * x) / aa + (y * y) / bb - 1.0;
    let disc = (b_coef * b_coef - 4.0 * a_coef * c_coef).max(0.0);
    let sqrt_d = disc.sqrt();
    let t1 = (-b_coef - sqrt_d) / (2.0 * a_coef);
    let t2 = (-b_coef + sqrt_d) / (2.0 * a_coef);
    let t = if t1 > 1e-6 {
        t1
    } else if t2 > 1e-6 {
        t2
    } else {
        t2.max(t1).max(1e-3)
    };
    let hx = x + t * dx;
    let hy = y + t * dy;
    let (nx, ny) = normal(a, b, hx, hy);
    let (rdx, rdy) = reflect(dx, dy, nx, ny);
    let norm = (rdx * rdx + rdy * rdy).sqrt().max(1e-9);
    (hx, hy, rdx / norm, rdy / norm)
}

fn draw(canvas: &mut dyn Surface, phi0: f64, dir: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = 1.0;
    let b = 0.65
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.02
        };
    // Draw ellipse.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=128 {
        let phi = std::f64::consts::TAU * i as f64 / 128.0;
        let (x, y) = ellipse_point(a, b, phi);
        let px = ((0.5 + 0.42 * x) * width.saturating_sub(1) as f64).round() as i32;
        let py = ((0.5 - 0.42 * y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Foci.
    let c = (a * a - b * b).max(0.0).sqrt();
    for &fx in &[-c, c] {
        let px = ((0.5 + 0.42 * fx) * width.saturating_sub(1) as f64).round() as i32;
        let py = (0.5 * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, '+');
    }
    // Trajectory.
    let (mut x, mut y) = ellipse_point(a, b, phi0);
    let mut dx = dir.cos();
    let mut dy = dir.sin();
    // Nudge inward.
    let (nx, ny) = normal(a, b, x, y);
    x -= nx * 0.01;
    y -= ny * 0.01;
    let mut last = (
        ((0.5 + 0.42 * x) * width.saturating_sub(1) as f64).round() as i32,
        ((0.5 - 0.42 * y) * height.saturating_sub(1) as f64).round() as i32,
    );
    for i in 0..BOUNCES {
        let (hx, hy, rdx, rdy) = next_hit(a, b, x, y, dx, dy);
        let px = ((0.5 + 0.42 * hx) * width.saturating_sub(1) as f64).round() as i32;
        let py = ((0.5 - 0.42 * hy) * height.saturating_sub(1) as f64).round() as i32;
        let ch = if i % 5 == 0 { '#' } else { '*' };
        canvas.line(last.0, last.1, px, py, ch);
        last = (px, py);
        x = hx - rdx * 1e-4;
        y = hy - rdy * 1e-4;
        dx = rdx;
        dy = rdy;
    }
}

/// Elliptical billiard room.
#[derive(Debug, Default)]
pub struct EllipticalBilliard {
    seed: u64,
}

impl EllipticalBilliard {
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

impl Room for EllipticalBilliard {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "elliptical-billiard",
            title: "Elliptical Billiard",
            wing: "Shape & Space",
            blurb: "Bounces in an ellipse; caustics and foci. t and DRAG: SET THE LAUNCH.",
            accent: [40, 140, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (phi, dir) = launch(t, None, self.seed);
        draw(canvas, phi, dir, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ellipse billiard",
            root: 207.65,
            tempo: 88,
            line: &[0, 5, 7, 12, 7, 5, 0, 9],
            encodes: "conserved caustic of an elliptical table",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE LAUNCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (phi, dir) = launch(t, None, self.seed);
        Some(format!(
            "phi={:.2}  dir={:.2}  DRAG",
            phi % std::f64::consts::TAU,
            dir
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (phi, dir) = launch(t, hands.last().copied(), self.seed);
        draw(canvas, phi, dir, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (phi, dir) = launch(t, hands.last().copied(), self.seed);
        let a = 1.5_f64;
        let b = 1.0_f64;
        // String construction caustic: confocal ellipse/hyperbola parameter.
        // Eccentricity of table e = sqrt(1 - (b/a)^2).
        let ecc = (1.0 - (b / a) * (b / a)).sqrt();
        let deg =
            (phi.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * 360.0).floor() as i32;
        Some(format!("phi={deg}deg  dir={dir:.2}  e={ecc:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "In an elliptical billiard every trajectory that crosses between the \
         foci forever encloses them as a caustic; trajectories outside stay \
         outside. The string construction of the ellipse is the same geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::EllipticalBilliard;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = EllipticalBilliard::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("LAUNCH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn launch_changes() {
        let r = EllipticalBilliard::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        EllipticalBilliard::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(EllipticalBilliard::new().motif().unwrap().line.len() >= 6);
    }
}
