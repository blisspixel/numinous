//! The Pour: integration as pouring, the fundamental theorem, felt.
//!
//! A curve holds water. As `t` sweeps, area pours in from the left and the
//! fill line rises; the running total traces a second curve above, and that
//! curve is the antiderivative. Reverse the pour and you are differentiating.
//! Nobody says the word integral; the accumulation is the controls. See the
//! Full Map in `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The domain: zero to this many radians.
const X_MAX: f64 = 9.42;
/// Samples across the domain.
const STEPS: usize = 600;

/// The curve being poured into: positive everywhere, so the water only rises.
fn f(x: f64) -> f64 {
    1.2 + (x).sin()
}

/// The running area under `f` from 0 to `x` (closed form, for exactness).
fn area(x: f64) -> f64 {
    1.2 * x - x.cos() + 1.0
}

/// The Pour.
#[derive(Debug, Default)]
pub struct ThePour;

impl ThePour {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for ThePour {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "the-pour",
            title: "The Pour",
            wing: "Change",
            blurb: "A curve holds water. t pours area in from the left; the rising total traces a \
                    second curve above. You are watching the fundamental theorem of calculus.",
            accent: [80, 160, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let x_t = t.clamp(0.0, 1.0) * X_MAX;
        let f_max = 2.2; // 1.2 + 1
        let area_max = area(X_MAX);
        // The curve lives in the lower two thirds; the total in the upper third.
        let curve_band = height as f64 * 0.62;
        let curve_top = height as f64 * 0.36;
        let to_px = |x: f64| (x / X_MAX * (width as f64 - 1.0)) as i32;
        let curve_py = |x: f64| (curve_top + (1.0 - f(x) / f_max) * (curve_band - 4.0)) as i32 + 2;

        // The vessel: the curve itself.
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = X_MAX * i as f64 / STEPS as f64;
            let point = (to_px(x), curve_py(x));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '*');
            }
            previous = Some(point);
        }
        // The water: filled from the left, up to the curve, as far as the pour.
        let floor = (curve_top + curve_band) as i32;
        for i in 0..=STEPS {
            let x = X_MAX * i as f64 / STEPS as f64;
            if x > x_t {
                break;
            }
            let px = to_px(x);
            let top = curve_py(x);
            let mut py = top;
            while py < floor {
                if (px + py) % 2 == 0 {
                    canvas.plot(px, py, '-');
                }
                py += 1;
            }
        }
        // The total so far: the antiderivative, tracing itself as you pour.
        let total_py = |x: f64| ((1.0 - area(x) / area_max) * (curve_top - 3.0)) as i32 + 1;
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = X_MAX * i as f64 / STEPS as f64;
            if x > x_t {
                break;
            }
            let point = (to_px(x), total_py(x));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '#');
            }
            previous = Some(point);
        }
    }

    fn reveal(&self) -> &'static str {
        "The upper curve is the running total of the water below, and its slope \
         at every moment equals the height of the curve being filled. Total and \
         rate are the same fact read in two directions: that is the fundamental \
         theorem of calculus, and you just watched it pour."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Newton and Leibniz found this connection independently and then spent \
             years in the ugliest priority fight in the history of science. The \
             theorem was bigger than both of them, which is the only reason it \
             carries neither name.",
            "Archimedes computed areas under curves two thousand years before \
             calculus by slicing them thin, the method of exhaustion. He was one \
             good notation away; the water was always pouring.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.8
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The fill sings: pitch rises with the accumulated area.
        let ratio = (area(t.clamp(0.0, 1.0) * X_MAX) / area(X_MAX)) as f32;
        SoundSpec::tone(110.0 + 330.0 * ratio, 1.5, 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::{ThePour, area, f};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_curve_is_positive_and_the_area_grows() {
        let mut last = 0.0;
        for i in 0..=100 {
            let x = super::X_MAX * f64::from(i) / 100.0;
            assert!(f(x) > 0.0, "the vessel never dips below the floor");
            let a = area(x);
            assert!(a >= last, "the pour only rises");
            last = a;
        }
    }

    #[test]
    fn the_area_is_the_true_integral() {
        // Riemann-check the closed form: area'(x) == f(x) numerically.
        for i in 1..40 {
            let x = super::X_MAX * f64::from(i) / 40.0;
            let h = 1e-6;
            let slope = (area(x + h) - area(x - h)) / (2.0 * h);
            assert!(
                (slope - f(x)).abs() < 1e-4,
                "the fundamental theorem holds at {x}: {slope} vs {}",
                f(x)
            );
        }
    }

    #[test]
    fn render_is_deterministic_and_pours_more_over_time() {
        let room = ThePour::new();
        let mut early = Canvas::new(60, 30);
        let mut late = Canvas::new(60, 30);
        room.render(&mut early, 0.2);
        room.render(&mut late, 0.9);
        assert!(late.ink_count() > early.ink_count(), "more pour, more ink");
        let mut again = Canvas::new(60, 30);
        room.render(&mut again, 0.9);
        assert_eq!(late.to_text(), again.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = ThePour::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_theorem() {
        assert!(ThePour::new().reveal().contains("fundamental theorem"));
    }
}
