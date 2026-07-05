//! The Mandelbrot set: infinite complexity from one line of arithmetic.
//!
//! For each point `c` in the complex plane, iterate `z -> z*z + c` from zero and
//! ask whether it stays bounded. The points that do form the set; the points that
//! escape, shaded by how fast, form its infinitely detailed halo. `t` zooms from
//! the whole set toward the seahorse valley. See `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Escape-iteration budget (also the "in the set" sentinel).
const MAX_ITER: u32 = 160;

/// The Mandelbrot room.
#[derive(Debug, Default)]
pub struct Mandelbrot;

impl Mandelbrot {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// How many iterations `z -> z*z + c` survives before escaping `|z| > 2`.
fn escape_iters(cx: f64, cy: f64, max: u32) -> u32 {
    let (mut zx, mut zy) = (0.0, 0.0);
    let mut i = 0;
    while i < max && zx * zx + zy * zy <= 4.0 {
        let next_x = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = next_x;
        i += 1;
    }
    i
}

/// Linear interpolation from `a` to `b` by `t`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

impl Room for Mandelbrot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mandelbrot",
            title: "Mandelbrot Set",
            wing: "Fractals & the Infinite",
            blurb: "Iterate z into z squared plus c and ask if it stays bounded. The points that \
                    do form the most complex object in mathematics. t zooms toward the seahorses.",
            accent: [70, 130, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let t = t.clamp(0.0, 1.0);
        // Zoom from the whole set toward the seahorse valley at -0.745 + 0.113i.
        let zoom = 1.5 * 0.15_f64.powf(t);
        let center_x = lerp(-0.5, -0.745, t);
        let center_y = lerp(0.0, 0.113, t);
        let scale = 2.0 * zoom / width as f64;
        let half_w = width as f64 / 2.0;
        let half_h = height as f64 / 2.0;

        for py in 0..height {
            for px in 0..width {
                let cx = center_x + (px as f64 - half_w) * scale;
                let cy = center_y + (py as f64 - half_h) * scale;
                let iters = escape_iters(cx, cy, MAX_ITER);
                let mark = if iters == MAX_ITER {
                    '#'
                } else if iters > 24 {
                    '*'
                } else if iters > 6 {
                    '-'
                } else {
                    continue;
                };
                canvas.plot(px as i32, py as i32, mark);
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "You can zoom into this shape forever and never stop finding new detail, \
         and tiny copies of the whole set hide infinitely deep inside it. All of \
         it comes from squaring a number and adding a constant."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Nobody knows the exact area of this set. It is about 1.5065918849, \
             measured by throwing billions of points at it, and there is no known \
             closed form. One of the most famous objects in mathematics, and we \
             cannot tell you how big it is.",
            "Shishikura proved in 1991 that the boundary you are zooming along has \
             Hausdorff dimension exactly 2: a curve so wrinkled it is, in the fractal \
             sense, as thick as the plane it lives in.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{Mandelbrot, escape_iters};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn origin_is_in_the_set() {
        assert_eq!(escape_iters(0.0, 0.0, 160), 160);
    }

    #[test]
    fn far_points_escape_quickly() {
        assert!(escape_iters(2.0, 2.0, 160) < 6);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Mandelbrot::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Mandelbrot::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_forever() {
        assert!(Mandelbrot::new().reveal().contains("forever"));
    }
}
