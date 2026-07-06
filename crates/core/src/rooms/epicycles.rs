//! Fourier Epicycles: circles on circles draw anything at all.
//!
//! A target shape is decomposed into rotating circles (its discrete Fourier
//! series). Chain the circles tip to tail, let each spin at its own speed, and
//! the end of the chain traces the shape back into existence. Ptolemy called
//! this machinery epicycles and used it on planets; Fourier proved it works on
//! everything. `t` runs the pen around the drawing. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// Sample points along the target shape.
const SAMPLES: usize = 128;
/// Fourier terms kept (frequencies -K..=K).
const K: i64 = 10;

/// The target: a five-pointed star, closed, centered, radius about one.
fn target(i: usize) -> (f64, f64) {
    let s = i as f64 / SAMPLES as f64;
    // Walk the star's 10 outline vertices (outer, inner, outer, ...).
    let verts: Vec<(f64, f64)> = (0..10)
        .map(|v| {
            let angle = TAU * v as f64 / 10.0 - TAU / 4.0;
            let r = if v % 2 == 0 { 1.0 } else { 0.45 };
            (r * angle.cos(), r * angle.sin())
        })
        .collect();
    let pos = s * 10.0;
    let edge = (pos as usize) % 10;
    let frac = pos.fract();
    let (x0, y0) = verts[edge];
    let (x1, y1) = verts[(edge + 1) % 10];
    (x0 + (x1 - x0) * frac, y0 + (y1 - y0) * frac)
}

/// One Fourier coefficient of the target, as (re, im) for frequency `k`.
fn coefficient(k: i64) -> (f64, f64) {
    let (mut re, mut im) = (0.0, 0.0);
    for i in 0..SAMPLES {
        let (x, y) = target(i);
        let angle = -TAU * k as f64 * i as f64 / SAMPLES as f64;
        let (c, s) = (angle.cos(), angle.sin());
        // (x + iy) * (c + is)
        re += x * c - y * s;
        im += x * s + y * c;
    }
    (re / SAMPLES as f64, im / SAMPLES as f64)
}

/// All kept coefficients with their frequencies, largest circle first
/// (skipping the constant term, which is just the center).
fn epicycles() -> Vec<(i64, f64, f64)> {
    let mut list: Vec<(i64, f64, f64)> = (-K..=K)
        .filter(|&k| k != 0)
        .map(|k| {
            let (re, im) = coefficient(k);
            (k, re, im)
        })
        .collect();
    list.sort_by(|a, b| {
        let ra = a.1.hypot(a.2);
        let rb = b.1.hypot(b.2);
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });
    list
}

/// The chain's tip at time `tau` in [0, 1): the partial Fourier sum.
fn tip(tau: f64) -> (f64, f64) {
    let (c0re, c0im) = coefficient(0);
    let (mut x, mut y) = (c0re, c0im);
    for (k, re, im) in epicycles() {
        let angle = TAU * k as f64 * tau;
        let (c, s) = (angle.cos(), angle.sin());
        x += re * c - im * s;
        y += re * s + im * c;
    }
    (x, y)
}

/// The Fourier Epicycles room.
#[derive(Debug, Default)]
pub struct Epicycles;

impl Epicycles {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for Epicycles {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "epicycles",
            title: "Fourier Epicycles",
            wing: "Waves & Sound",
            blurb: "Circles on circles, each spinning at its own speed, and the tip of the chain \
                    draws a star. Fourier proved the circles can draw anything. t runs the pen.",
            accent: [180, 130, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = canvas.char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let scale = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.7;
        let to_screen =
            |x: f64, y: f64| ((cx + x * scale) as i32, (cy + y * scale * aspect) as i32);

        let tau_now = t.clamp(0.0, 1.0);
        // The traced path so far: the star, drawing itself.
        let steps = (tau_now * 360.0) as usize;
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let (x, y) = tip(i as f64 / 360.0);
            let point = to_screen(x, y);
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '#');
            }
            previous = Some(point);
        }
        // The machinery: the chain of circles at this instant, dim.
        let (c0re, c0im) = coefficient(0);
        let (mut x, mut y) = (c0re, c0im);
        for (k, re, im) in epicycles().into_iter().take(7) {
            let radius = re.hypot(im);
            let ring = 90;
            for r in 0..ring {
                let a = TAU * r as f64 / ring as f64;
                let (px, py) = to_screen(x + radius * a.cos(), y + radius * a.sin());
                canvas.plot(px, py, '*');
            }
            let angle = TAU * k as f64 * tau_now;
            let (c, s) = (angle.cos(), angle.sin());
            x += re * c - im * s;
            y += re * s + im * c;
        }
        // The pen.
        let (px, py) = to_screen(x, y);
        canvas.plot(px, py, '#');
    }

    fn reveal(&self) -> &'static str {
        "Any closed drawing, any at all, can be traced by circles rotating on \
         circles at fixed speeds. The star was never stored as a star: it is a \
         short list of circle sizes and speeds. Compressing shapes into spinning \
         circles is what your phone does to every song and photo it holds."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Ptolemy used exactly this machinery to predict planets from a fixed \
             Earth, and it worked for fourteen centuries, because epicycles are a \
             Fourier series and Fourier series can fit anything. He was not wrong; \
             he was curve-fitting in the sky.",
            "Fourier's 1807 paper claiming any function splits into waves was \
             rejected, with Lagrange among the objectors. The referees demanded \
             rigor nobody alive had. He was right anyway, and the proof machinery \
             built to settle it became modern analysis.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.8
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The first harmonics as a chord: the shape, heard.
        let _ = t;
        SoundSpec::chord(&[110.0, 220.0, 330.0, 550.0], 1.5, 0.12)
    }
}

#[cfg(test)]
mod tests {
    use super::{Epicycles, K, SAMPLES, coefficient, target, tip};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_target_is_closed_and_bounded() {
        let (x0, y0) = target(0);
        let (xn, yn) = target(SAMPLES - 1);
        assert!(
            ((x0 - xn).powi(2) + (y0 - yn).powi(2)).sqrt() < 0.2,
            "closed"
        );
        for i in 0..SAMPLES {
            let (x, y) = target(i);
            assert!(x.hypot(y) <= 1.01, "within the unit star");
        }
    }

    #[test]
    fn the_series_reconstructs_the_star() {
        // The partial sum with K terms lands near the target everywhere.
        let mut worst = 0.0f64;
        for i in 0..SAMPLES {
            let tau = i as f64 / SAMPLES as f64;
            let (tx, ty) = target(i);
            let (fx, fy) = tip(tau);
            worst = worst.max(((tx - fx).powi(2) + (ty - fy).powi(2)).sqrt());
        }
        assert!(
            worst < 0.15,
            "K={K} terms trace the star: worst gap {worst}"
        );
    }

    #[test]
    fn the_constant_term_is_the_centroid() {
        let (re, im) = coefficient(0);
        assert!(re.abs() < 0.05 && im.abs() < 0.05, "the star is centered");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Epicycles::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.8);
        room.render(&mut b, 0.8);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Epicycles::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_connects_to_compression() {
        assert!(Epicycles::new().reveal().contains("Compressing"));
    }
}
