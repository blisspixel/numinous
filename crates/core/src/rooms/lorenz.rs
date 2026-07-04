//! The Lorenz attractor: the butterfly that made "chaos" a science.
//!
//! Three simple equations for a toy weather model, and the trajectory never
//! settles and never repeats, yet never leaves a butterfly-shaped set. Two starts
//! a millionth apart diverge completely: the butterfly effect. `t` raises the
//! parameter through the onset of chaos. See `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Prandtl number.
const SIGMA: f64 = 10.0;
/// Geometric factor.
const BETA: f64 = 8.0 / 3.0;
/// Integration time step.
const DT: f64 = 0.005;
/// Total integration steps.
const STEPS: usize = 9_000;
/// Steps to discard so the path is on the attractor before drawing.
const TRANSIENT: usize = 800;

/// The Lorenz attractor room.
#[derive(Debug, Default)]
pub struct Lorenz;

impl Lorenz {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The Rayleigh parameter at phase `t`, sweeping through the onset of chaos.
    fn rho_for(t: f64) -> f64 {
        24.0 + 6.0 * t.clamp(0.0, 1.0)
    }
}

/// The Lorenz path from the default start.
fn trajectory(rho: f64) -> Vec<(f64, f64, f64)> {
    integrate(0.1, 0.0, 0.0, rho)
}

/// Integrate the Lorenz system from a start and return the `(x, y, z)` path.
fn integrate(mut x: f64, mut y: f64, mut z: f64, rho: f64) -> Vec<(f64, f64, f64)> {
    let mut points = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let dx = SIGMA * (y - x);
        let dy = x * (rho - z) - y;
        let dz = x * y - BETA * z;
        x += dx * DT;
        y += dy * DT;
        z += dz * DT;
        points.push((x, y, z));
    }
    points
}

impl Room for Lorenz {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lorenz",
            title: "Lorenz Attractor",
            wing: "Chaos & Order",
            blurb: "Three equations for toy weather. The path never repeats and never escapes its \
                    butterfly-shaped set. t raises the parameter through the onset of chaos.",
            accent: [80, 180, 230],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let points = trajectory(Self::rho_for(t));
        // Project the x-z plane (the classic butterfly). Fixed bounds so the shape
        // sits still as it is drawn: x in about [-25, 25], z in about [0, 55].
        let to_screen = |x: f64, z: f64| -> (i32, i32) {
            let sx = (x + 25.0) / 50.0 * (width as f64 - 1.0);
            let sy = (height as f64 - 1.0) - (z / 55.0) * (height as f64 - 1.0);
            (sx as i32, sy as i32)
        };
        let mut previous: Option<(i32, i32)> = None;
        for &(x, _, z) in points.iter().skip(TRANSIENT) {
            let (sx, sy) = to_screen(x, z);
            if let Some((px, py)) = previous {
                canvas.line(px, py, sx, sy, '#');
            }
            previous = Some((sx, sy));
        }
    }

    fn reveal(&self) -> &'static str {
        "Lorenz found this by rounding 0.506127 to 0.506 in a weather run and \
         watching the forecast diverge completely. That is the butterfly effect: \
         perfectly determined, and still impossible to predict."
    }
}

#[cfg(test)]
mod tests {
    use super::{Lorenz, integrate, trajectory};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_path_stays_on_the_attractor() {
        // After the transient the trajectory is bounded inside a known box.
        for &(x, y, z) in trajectory(28.0).iter().skip(800) {
            assert!(x.abs() < 40.0 && y.abs() < 60.0, "escaped: {x}, {y}");
            assert!((-5.0..80.0).contains(&z), "z escaped: {z}");
        }
    }

    #[test]
    fn tiny_start_differences_diverge() {
        // The butterfly effect: two starts a ten-thousandth apart, same system.
        let a = integrate(0.1, 0.0, 0.0, 28.0);
        let b = integrate(0.1001, 0.0, 0.0, 28.0);
        let (ax, _, az) = *a.last().unwrap();
        let (bx, _, bz) = *b.last().unwrap();
        assert!((ax - bx).abs() + (az - bz).abs() > 1.0, "did not diverge");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Lorenz::new();
        let mut first = Canvas::new(60, 30);
        let mut second = Canvas::new(60, 30);
        room.render(&mut first, 0.7);
        room.render(&mut second, 0.7);
        assert_eq!(first.to_text(), second.to_text());
        assert!(first.ink_count() > 30);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Lorenz::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_the_butterfly_effect() {
        assert!(Lorenz::new().reveal().contains("butterfly effect"));
    }
}
