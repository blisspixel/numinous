//! Times Tables: modular multiplication on a circle. The flagship room.
//!
//! Place `N` points evenly on a circle and, from each point `n`, draw a chord to
//! point `round(n * k) mod N`. At `k = 2` a cardioid blooms out of nothing but
//! the two-times table; sweeping `k` upward morphs it through nephroids and
//! nested lobes. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// Number of points placed around the circle. Higher is smoother and denser.
const POINTS: usize = 240;
/// The multiplier `k` at phase `t = 0` (the two-times table, a cardioid).
const K_MIN: f64 = 2.0;
/// How far `k` sweeps across the phase range `[0, 1)`.
const K_SWEEP: f64 = 8.0;

/// The Times Tables room.
#[derive(Debug, Default)]
pub struct TimesTables;

impl TimesTables {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for TimesTables {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "times-tables",
            title: "Times Tables",
            wing: "Number & Pattern",
            blurb: "From each point n on a circle, draw a chord to point (n times k); \
                    a cardioid blooms out of the two-times table.",
            accent: [40, 150, 190],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let multiplier = K_MIN + K_SWEEP * t.clamp(0.0, 1.0);

        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        let cx = width / 2.0;
        let cy = height / 2.0;
        // Squash y by the surface's aspect (0.5 for tall terminal cells, 1.0 for
        // square pixels) so the circle stays round on any surface.
        let aspect = canvas.char_aspect();
        // Fit both extents: x uses width/2, y uses radius*aspect <= height/2.
        let radius = (width / 2.0).min(height / (2.0 * aspect)) * 0.9;

        let point = |i: usize| -> (i32, i32) {
            let angle = (i as f64 / POINTS as f64) * TAU - FRAC_PI_2;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin() * aspect;
            (x.round() as i32, y.round() as i32)
        };

        for n in 0..POINTS {
            let target = ((n as f64) * multiplier).round() as usize % POINTS;
            let (x0, y0) = point(n);
            let (x1, y1) = point(target);
            canvas.line(x0, y0, x1, y1, '*');
        }
    }

    fn reveal(&self) -> &'static str {
        "You drew a heart with the two-times table. That cardioid is the exact \
         outline of the Mandelbrot set's main body: a homework grid and the most \
         complex object in mathematics trace the same shape."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Change the multiplier and the curve changes family: times 2 draws a \
             cardioid, times 3 a nephroid, and times n a curve with n minus 1 cusps. \
             The whole zoo is one rule with the dial turned.",
            "The strings you see are modular arithmetic, the same math that lets an \
             accountant check a ledger by casting out nines and lets your bank card \
             verify itself with a check digit. The pretty curve and the checksum are \
             the same idea.",
        ]
    }
    fn sound(&self, t: f64) -> SoundSpec {
        // Pitch rises with the multiplier k; landing on a whole number sounds clean.
        let k = (K_MIN + K_SWEEP * t.clamp(0.0, 1.0)) as f32;
        SoundSpec::tone(110.0 * k, 1.5, 0.3)
    }
}

#[cfg(test)]
mod tests {
    use super::TimesTables;
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn meta_is_stable() {
        let m = TimesTables::new().meta();
        assert_eq!(m.id, "times-tables");
        assert_eq!(m.wing, "Number & Pattern");
    }

    #[test]
    fn reveal_names_the_connection() {
        assert!(TimesTables::new().reveal().contains("Mandelbrot"));
    }

    #[test]
    fn sound_is_a_single_tone() {
        let spec = TimesTables::new().sound(0.0);
        assert_eq!(spec.notes.len(), 1);
        assert!(spec.notes[0].freq > 0.0);
    }

    #[test]
    fn render_is_deterministic() {
        let room = TimesTables::new();
        let mut a = Canvas::new(80, 40);
        let mut b = Canvas::new(80, 40);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(80, 40);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 0, "the cardioid should draw something");
    }

    #[test]
    fn extreme_phase_values_do_not_panic() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(40, 20);
        for t in [-5.0, 0.0, 0.5, 0.999, 5.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn tiny_canvas_does_not_panic() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(1, 1);
        room.render(&mut canvas, 0.3);
    }
}
