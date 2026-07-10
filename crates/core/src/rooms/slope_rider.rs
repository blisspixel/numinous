//! Slope Rider: the derivative as a ride, not a rule.
//!
//! A rider slides along a curve; the tangent line under it is its board, and
//! the board's tilt at every instant is the derivative. Below, the tilt traces
//! its own curve as the ride goes on, and that trace is f prime, drawing
//! itself. Nobody says the word derivative; your speed is the controls. See
//! the Full Map in `docs/ROOMS.md`.

use super::variation_unit;
use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The domain, symmetric about zero.
const X_SPAN: f64 = 12.56;
/// Samples across the domain.
const STEPS: usize = 600;

/// The hill being ridden.
fn f(x: f64) -> f64 {
    x.sin() + x / 3.0
}

/// Its exact slope, the thing the ride teaches.
fn slope(x: f64) -> f64 {
    x.cos() + 1.0 / 3.0
}

/// Slope Rider.
#[derive(Debug, Default)]
pub struct SlopeRider {
    seed: u64,
}

impl SlopeRider {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn phase_for(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        if self.seed == 0 {
            t
        } else {
            (t + variation_unit(self.seed, 0x534C_4F50_4552_0001) * 0.45).fract()
        }
    }
}

impl Room for SlopeRider {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "slope-rider",
            title: "Slope Rider",
            wing: "Change",
            blurb: "Ride the tangent line along a curve. The board's tilt is the slope, and the \
                    tilt traces its own curve below as you go: the derivative, drawing itself.",
            accent: [255, 190, 70],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let x_t = (self.phase_for(t) - 0.5) * X_SPAN;
        let x_min = -X_SPAN / 2.0;
        let to_px = |x: f64| ((x - x_min) / X_SPAN * (width as f64 - 1.0)) as i32;

        // The hill, upper two thirds.
        let f_min = -X_SPAN / 6.0 - 1.0;
        let f_max = X_SPAN / 6.0 + 1.0;
        let hill_py = |x: f64| {
            let norm = (f(x) - f_min) / (f_max - f_min);
            ((1.0 - norm) * (height as f64 * 0.60 - 2.0)) as i32 + 1
        };
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = x_min + X_SPAN * i as f64 / STEPS as f64;
            let point = (to_px(x), hill_py(x));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '*');
            }
            previous = Some(point);
        }

        // The board: the tangent at the rider, drawn bright.
        let m = slope(x_t);
        let reach = X_SPAN / 10.0;
        let (x0, x1) = (x_t - reach, x_t + reach);
        let y_at = |x: f64| f(x_t) + m * (x - x_t);
        let tangent_py = |y: f64| {
            let norm = (y - f_min) / (f_max - f_min);
            ((1.0 - norm) * (height as f64 * 0.60 - 2.0)) as i32 + 1
        };
        canvas.line(
            to_px(x0),
            tangent_py(y_at(x0)),
            to_px(x1),
            tangent_py(y_at(x1)),
            '#',
        );

        // The tilt trace, lower third: f prime drawing itself up to the rider.
        let s_min = -0.7;
        let s_max = 1.4;
        let trace_top = height as f64 * 0.66;
        let trace_py = |x: f64| {
            let norm = (slope(x) - s_min) / (s_max - s_min);
            (trace_top + (1.0 - norm) * (height as f64 * 0.32 - 2.0)) as i32
        };
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = x_min + X_SPAN * i as f64 / STEPS as f64;
            if x > x_t {
                break;
            }
            let point = (to_px(x), trace_py(x));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '#');
            }
            previous = Some(point);
        }
    }

    fn reveal(&self) -> &'static str {
        "The lower curve is nothing but the tilt of your board, written down \
         moment by moment. Where you crested, it crosses zero; where the climb \
         was steepest, it peaks. Every smooth curve secretly carries this second \
         curve inside it, and riding is how you read it out."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Fermat was finding tangent lines and extrema a generation before \
             Newton was born, with no limits and no derivatives, just a vanishing \
             little e he quietly threw away at the end. The scandalous trick \
             worked, and it took two centuries to make it honest.",
            "Where your board tilts level at the top of a rise is where functions \
             are maximized, which is why economics, physics, and machine learning \
             are all, at bottom, riders looking for the flat spots.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.62
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "A tangent ride",
            root: 220.0,
            tempo: 128,
            line: &[0, 3, 7, 10, 7, 3, -2, 0],
            encodes: "the board climbing, cresting, descending, and leveling",
        })
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The board hums with the tilt: uphill high, downhill low.
        let x = (self.phase_for(t) - 0.5) * X_SPAN;
        let norm = ((slope(x) + 0.7) / 2.1) as f32;
        SoundSpec::tone(110.0 + 330.0 * norm.clamp(0.0, 1.0), 1.5, 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::{SlopeRider, f, slope};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_slope_is_the_true_derivative() {
        for i in 0..40 {
            let x = -6.0 + 12.0 * f64::from(i) / 40.0;
            let h = 1e-6;
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            assert!((numeric - slope(x)).abs() < 1e-4, "at {x}");
        }
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = SlopeRider::new();
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        room.render(&mut a, 0.62);
        room.render(&mut b, 0.62);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = SlopeRider::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_reads_the_second_curve() {
        assert!(SlopeRider::new().reveal().contains("second curve"));
    }
}
