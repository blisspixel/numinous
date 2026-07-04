//! The logistic map: how a one-line population model becomes chaos.
//!
//! The rule `x -> r*x*(1-x)` models a population that grows and competes. Sweep
//! the growth rate `r` across the screen and, for each, plot where the population
//! settles: one value, then two, then four, then a chaotic smear, the famous
//! bifurcation diagram. `t` zooms the left edge into the chaos. See `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Iterations discarded so only the long-run attractor is drawn.
const TRANSIENT: usize = 300;
/// Attractor points plotted per column.
const SAMPLES: usize = 200;

/// The logistic map room.
#[derive(Debug, Default)]
pub struct LogisticMap;

impl LogisticMap {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The `[r_min, r_max]` window shown at phase `t` (zooming into the chaos).
    fn r_window(t: f64) -> (f64, f64) {
        let t = t.clamp(0.0, 1.0);
        (2.5 + t * 1.0, 4.0)
    }
}

impl Room for LogisticMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "logistic-map",
            title: "Logistic Map",
            wing: "Chaos & Order",
            blurb: "Sweep the growth rate of x into r x (1 - x) across the screen and plot where \
                    the population lands: one value, then two, then four, then chaos. t zooms in.",
            accent: [230, 200, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let (r_min, r_max) = Self::r_window(t);
        for px in 0..width {
            let r = r_min + (r_max - r_min) * (px as f64 / width as f64);
            let mut x = 0.5;
            for _ in 0..TRANSIENT {
                x = r * x * (1.0 - x);
            }
            for _ in 0..SAMPLES {
                x = r * x * (1.0 - x);
                let py = (height as f64 - x * height as f64) as i32;
                canvas.plot(px as i32, py, '#');
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "The point where order breaks into chaos arrives at the same rate for this \
         equation, for dripping taps, and for heartbeats: Feigenbaum's constant, \
         4.669. A single number governs how simple things fall apart."
    }
}

#[cfg(test)]
mod tests {
    use super::{LogisticMap, TRANSIENT};
    use crate::canvas::Canvas;
    use crate::room::Room;

    /// The attractor value after the transient, for testing.
    fn settle(r: f64) -> f64 {
        let mut x = 0.5;
        for _ in 0..TRANSIENT {
            x = r * x * (1.0 - x);
        }
        x
    }

    #[test]
    fn low_growth_settles_to_the_fixed_point() {
        // For 1 < r < 3 the map converges to 1 - 1/r; at r = 2.5 that is 0.6.
        assert!((settle(2.5) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn period_two_has_two_distinct_values() {
        // At r = 3.2 the population alternates between two values.
        let r = 3.2;
        let a = settle(r);
        let b = r * a * (1.0 - a);
        assert!((a - b).abs() > 0.05, "expected a 2-cycle, got {a} and {b}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = LogisticMap::new();
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = LogisticMap::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_feigenbaum() {
        assert!(LogisticMap::new().reveal().contains("Feigenbaum"));
    }
}
