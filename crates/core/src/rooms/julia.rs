//! Julia sets: the Mandelbrot set's infinite family of cousins.
//!
//! Same iteration as Mandelbrot (`z -> z*z + c`), but here `c` is a fixed
//! constant and the starting point is the pixel. Each value of `c` gives a
//! completely different fractal; `t` walks `c` around a circle, morphing the
//! shape from a connected blob into scattered dust and back. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Escape-iteration budget.
const MAX_ITER: u32 = 160;
/// Radius of the circle in the `c` plane that `t` walks around.
const C_RADIUS: f64 = 0.7885;

/// The Julia set room.
#[derive(Debug, Default)]
pub struct Julia;

impl Julia {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The constant `c` at phase `t`.
    fn c_for(t: f64) -> (f64, f64) {
        let theta = TAU * t.clamp(0.0, 1.0);
        (C_RADIUS * theta.cos(), C_RADIUS * theta.sin())
    }
}

/// Iterations of `z -> z*z + c` from `(zx, zy)` before escaping.
fn escape_iters(mut zx: f64, mut zy: f64, cx: f64, cy: f64, max: u32) -> u32 {
    let mut i = 0;
    while i < max && zx * zx + zy * zy <= 4.0 {
        let next_x = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = next_x;
        i += 1;
    }
    i
}

impl Room for Julia {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "julia",
            title: "Julia Set",
            wing: "Fractals & the Infinite",
            blurb: "The same rule as Mandelbrot, but c is fixed and the whole plane is the seed. \
                    Every c grows a different fractal; t walks c around a circle to morph it.",
            accent: [255, 120, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let (cx, cy) = Self::c_for(t);
        // A fixed window on the z plane, roughly [-1.6, 1.6] on the shorter axis.
        let scale = 3.2 / width as f64;
        let half_w = width as f64 / 2.0;
        let half_h = height as f64 / 2.0;

        for py in 0..height {
            for px in 0..width {
                let zx = (px as f64 - half_w) * scale;
                let zy = (py as f64 - half_h) * scale;
                let iters = escape_iters(zx, zy, cx, cy, MAX_ITER);
                let mark = if iters == MAX_ITER {
                    '#'
                } else if iters > 20 {
                    '*'
                } else if iters > 5 {
                    '-'
                } else {
                    continue;
                };
                canvas.plot(px as i32, py as i32, mark);
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "There is one Julia set for every point in the plane, an uncountable \
         infinity of them. Whether each one is a single connected piece or a \
         cloud of dust is decided by that point's place in the Mandelbrot set."
    }
}

#[cfg(test)]
mod tests {
    use super::{Julia, escape_iters};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn origin_survives_for_a_small_c() {
        // With c near zero the origin is a fixed point and never escapes.
        assert_eq!(escape_iters(0.0, 0.0, 0.0, 0.0, 160), 160);
    }

    #[test]
    fn far_start_escapes_quickly() {
        assert!(escape_iters(3.0, 3.0, -0.4, 0.6, 160) < 5);
    }

    #[test]
    fn c_walks_a_circle() {
        let (x0, y0) = Julia::c_for(0.0);
        let (x1, y1) = Julia::c_for(0.25);
        assert!((x0 - 0.7885).abs() < 1e-9 && y0.abs() < 1e-9);
        assert!(x1.abs() < 1e-9 && (y1 - 0.7885).abs() < 1e-9);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Julia::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.3);
        room.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 10);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Julia::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_infinity() {
        assert!(Julia::new().reveal().contains("infinity"));
    }
}
