//! Lissajous: two perpendicular oscillations tracing a curve.
//!
//! One oscillation drives the x axis and another the y axis. When their
//! frequencies form a simple ratio the figure is stable and closed; off-ratio it
//! tumbles. A stable figure is a musical interval you can see. `t` sweeps the
//! second frequency. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::canvas::Canvas;
use crate::room::{Room, RoomMeta};

/// The fixed x-axis frequency; `t` sweeps the y-axis frequency against it.
const FREQ_X: f64 = 3.0;
/// The y-axis frequency at `t = 0` (a 2:3 ratio, a perfect fifth).
const FREQ_Y_MIN: f64 = 2.0;
/// How far `t` sweeps the y-axis frequency.
const FREQ_Y_SWEEP: f64 = 3.0;
/// Number of samples along the curve; consecutive samples are connected.
const SAMPLES: usize = 1500;

/// The Lissajous room.
#[derive(Debug, Default)]
pub struct Lissajous;

impl Lissajous {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The y-axis frequency selected by phase `t`.
    fn freq_y_for(t: f64) -> f64 {
        FREQ_Y_MIN + FREQ_Y_SWEEP * t.clamp(0.0, 1.0)
    }

    /// The curve point at parameter `theta`, in normalized coordinates `[-1, 1]`.
    fn point_normalized(theta: f64, freq_y: f64) -> (f64, f64) {
        let x = (FREQ_X * theta + FRAC_PI_2).sin();
        let y = (freq_y * theta).sin();
        (x, y)
    }
}

impl Room for Lissajous {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lissajous",
            title: "Lissajous",
            wing: "Waves & Sound",
            blurb: "Two perpendicular oscillations, one per axis; a simple frequency ratio traces a \
                    stable figure and off-ratio it tumbles. t sweeps the second frequency.",
        }
    }

    fn render_ascii(&self, canvas: &mut Canvas, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let freq_y = Self::freq_y_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let (cx, cy) = (fw / 2.0, fh / 2.0);
        let rx = (fw / 2.0 - 1.0).max(0.0);
        let ry = (fh / 2.0 - 1.0).max(0.0);

        let to_pixel = |theta: f64| -> (i32, i32) {
            let (nx, ny) = Self::point_normalized(theta, freq_y);
            ((cx + nx * rx).round() as i32, (cy + ny * ry).round() as i32)
        };

        let mut prev = to_pixel(0.0);
        for i in 1..=SAMPLES {
            let theta = (i as f64 / SAMPLES as f64) * TAU;
            let current = to_pixel(theta);
            canvas.line(prev.0, prev.1, current.0, current.1, '*');
            prev = current;
        }
    }

    fn reveal(&self) -> &'static str {
        "A stable figure means the two frequencies are a perfect musical interval; \
         a 2:3 ratio is a perfect fifth. You are not drawing a curve, you are \
         seeing a chord. This is exactly what old oscilloscopes showed."
    }
}

#[cfg(test)]
mod tests {
    use super::Lissajous;
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn freq_y_starts_at_two() {
        assert!((Lissajous::freq_y_for(0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn normalized_points_stay_in_range() {
        for i in 0..1000 {
            let theta = f64::from(i) * 0.017;
            let (x, y) = Lissajous::point_normalized(theta, 2.0);
            assert!((-1.0..=1.0).contains(&x));
            assert!((-1.0..=1.0).contains(&y));
        }
    }

    #[test]
    fn render_is_deterministic() {
        let room = Lissajous::new();
        let mut a = Canvas::new(40, 24);
        let mut b = Canvas::new(40, 24);
        room.render_ascii(&mut a, 0.0);
        room.render_ascii(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = Lissajous::new();
        let mut canvas = Canvas::new(40, 24);
        room.render_ascii(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = Lissajous::new();
        let mut empty = Canvas::new(0, 0);
        room.render_ascii(&mut empty, 0.5);
        let mut canvas = Canvas::new(4, 4);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render_ascii(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_interval() {
        assert!(Lissajous::new().reveal().contains("perfect fifth"));
    }
}
