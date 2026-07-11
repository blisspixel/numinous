//! The logistic map: how a one-line population model becomes chaos.
//!
//! The rule `x -> r*x*(1-x)` models a population that grows and competes. Sweep
//! the growth rate `r` across the screen and, for each, plot where the population
//! settles: one value, then two, then four, then a chaotic smear, the famous
//! bifurcation diagram. `t` zooms the left edge into the chaos. See `docs/ROOMS.md`.

use super::variation_unit;
use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::Surface;

/// Iterations discarded so only the long-run attractor is drawn.
const TRANSIENT: usize = 300;
/// Attractor points plotted per column.
const SAMPLES: usize = 200;
/// Steps in the hand-seeded orbit overlay.
const POKED_ORBIT_STEPS: usize = 160;

/// The logistic map room.
#[derive(Debug, Default)]
pub struct LogisticMap {
    seed: u64,
}

impl LogisticMap {
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

    /// The `[r_min, r_max]` window shown at phase `t` (zooming into the chaos).
    fn r_window(t: f64) -> (f64, f64) {
        let t = finite_phase(t);
        (2.5 + t * 1.0, 4.0)
    }

    fn r_window_for(&self, t: f64) -> (f64, f64) {
        let (r_min, r_max) = Self::r_window(t);
        let shift = variation_unit(self.seed, 0x4C4F_4749_5354_0001) * 0.18;
        ((r_min + shift).min(r_max - 0.05), r_max)
    }
}

fn finite_phase(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn hand_points(pokes: &[(f64, f64)]) -> impl Iterator<Item = (f64, f64)> + '_ {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..].iter().filter_map(|&(x, y)| {
        if x.is_finite() && y.is_finite() {
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        } else {
            None
        }
    })
}

fn population_row(height: usize, x: f64) -> i32 {
    ((height as f64 - 1.0) - x.clamp(0.0, 1.0) * (height as f64 - 1.0)).round() as i32
}

/// Iterations averaged when estimating the Lyapunov exponent.
const LYAPUNOV_STEPS: usize = 2000;

/// The Lyapunov exponent of the map at growth rate `r`: the long-run average of
/// `ln|f'(x)|` along the orbit. It is the rate at which nearby populations pull
/// apart, negative when the orbit settles onto a cycle (order), positive once it
/// never repeats (chaos). The zero crossing is the exact border between the two.
fn lyapunov(r: f64) -> f64 {
    let mut x = 0.5;
    for _ in 0..TRANSIENT {
        x = r * x * (1.0 - x);
    }
    let mut sum = 0.0;
    let mut counted = 0u32;
    for _ in 0..LYAPUNOV_STEPS {
        let slope = (r * (1.0 - 2.0 * x)).abs();
        if slope > 1e-12 {
            sum += slope.ln();
            counted += 1;
        }
        x = r * x * (1.0 - x);
    }
    if counted == 0 {
        0.0
    } else {
        sum / counted as f64
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
        let (r_min, r_max) = self.r_window_for(t);
        for px in 0..width {
            let r = r_min + (r_max - r_min) * (px as f64 / width as f64);
            let mut x = 0.5;
            for _ in 0..TRANSIENT {
                x = r * x * (1.0 - x);
            }
            for _ in 0..SAMPLES {
                x = r * x * (1.0 - x);
                canvas.plot(px as i32, population_row(height, x), '#');
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "The point where order breaks into chaos arrives at the same rate for this \
         equation, for dripping taps, and for heartbeats: Feigenbaum's constant, \
         4.669. A single number governs how simple things fall apart."
    }

    fn status(&self, t: f64) -> Option<String> {
        // Read the middle of the visible band. At t = 0 the whole cascade is on
        // screen and the midpoint sits in the ordered, cycling regime; as the
        // sweep zooms the left edge deeper into chaos, that midpoint crosses the
        // onset and the exponent turns positive. So the readout narrates order
        // becoming chaos as one number changing sign, and because it moves the
        // Logistic Map now poses predictions and challenges too.
        let (r_min, r_max) = self.r_window_for(t);
        let r = (r_min + r_max) / 2.0;
        let exponent = lyapunov(r);
        let regime = if exponent > 0.0 { "CHAOS" } else { "ORDER" };
        Some(format!("LYAPUNOV {exponent:+.2} ({regime}) AT R {r:.2}"))
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "B bifurcation",
            root: 246.94,
            tempo: 136,
            line: &[0, 0, 7, 7, 3, 10, 1, 8],
            encodes: "one fixed point splitting into two, four, then chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: SEED POPULATION")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        self.render(canvas, t);
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let (r_min, r_max) = self.r_window_for(t);
        let max_x = width.saturating_sub(1) as f64;
        let max_i = width.saturating_sub(1) as i32;

        for (hand_x, hand_y) in hand_points(pokes) {
            let column = (hand_x * max_x).round().clamp(0.0, max_x) as i32;
            let r = r_min + (r_max - r_min) * hand_x;
            let mut population = (1.0 - hand_y).clamp(0.001, 0.999);
            canvas.plot(column, population_row(height, population), '@');
            for step in 0..POKED_ORBIT_STEPS {
                population = r * population * (1.0 - population);
                if !population.is_finite() {
                    break;
                }
                let lane = column + (step % 5) as i32 - 2;
                canvas.plot(
                    lane.clamp(0, max_i),
                    population_row(height, population),
                    '*',
                );
            }
            canvas.plot(column, population_row(height, 1.0 - hand_y), '@');
        }
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Feigenbaum found 4.669 in 1975 with an HP-65 pocket calculator, \
             computing between its keystrokes by hand. The constant now bears his \
             name and appears in systems he never touched.",
            "The constant is universal: any map with one smooth hump cascades into \
             chaos at the same ratio, and it has been measured in dripping faucets \
             and fluttering flames. One number, everywhere order breaks.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{LogisticMap, TRANSIENT};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room};

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

    #[test]
    fn verb_names_population_seeding() {
        assert_eq!(LogisticMap::new().verb(), Some("CLICK: SEED POPULATION"));
    }

    #[test]
    fn render_poked_draws_a_seeded_orbit_trace() {
        let room = LogisticMap::new();
        let mut base = Canvas::new(60, 30);
        let mut poked = Canvas::new(60, 30);

        room.render(&mut base, 0.35);
        room.render_poked(&mut poked, 0.35, &[(0.75, 0.2)]);

        assert_ne!(base.to_text(), poked.to_text());
        assert!(
            poked.to_text().matches('*').count() > 5,
            "the click should draw an orbit, not just a marker"
        );
    }

    #[test]
    fn render_poked_caps_the_newest_raw_tail_before_filtering() {
        let room = LogisticMap::new();
        let mut base = Canvas::new(50, 24);
        let mut actual = Canvas::new(50, 24);
        let mut pokes = vec![(0.6, 0.4)];
        pokes.extend(std::iter::repeat_n((f64::NAN, 0.2), MAX_ROOM_POKES));

        room.render(&mut base, 0.2);
        room.render_poked(&mut actual, 0.2, &pokes);

        assert_eq!(
            actual.to_text(),
            base.to_text(),
            "an all-invalid newest tail must discard older valid points"
        );
    }

    #[test]
    fn render_poked_clamps_finite_edge_points() {
        let room = LogisticMap::new();
        let mut canvas = Canvas::new(30, 16);

        room.render_poked(&mut canvas, 0.2, &[(2.0, -1.0), (-1.0, 2.0)]);
        let text = canvas.to_text();

        assert!(text.contains('@'));
        assert!(text.contains('*'));
    }

    #[test]
    fn the_lyapunov_readout_crosses_from_order_into_chaos() {
        use super::lyapunov;
        // Deep in a cycle the exponent is negative; deep in chaos it is positive.
        assert!(lyapunov(3.2) < 0.0, "a 2-cycle should read as order");
        assert!(lyapunov(3.9) > 0.0, "a chaotic rate should read as chaos");

        // The readout moves across the sweep, so the room can pose predictions.
        let room = LogisticMap::new();
        assert!(room.status(0.0).is_some());
        assert!(crate::pose_prediction(&room, 5).is_some());
    }

    #[test]
    fn nonfinite_phase_falls_back_to_the_first_window() {
        let room = LogisticMap::new();
        let mut finite = Canvas::new(50, 24);
        let mut nonfinite = Canvas::new(50, 24);

        room.render_poked(&mut finite, 0.0, &[(0.7, 0.3)]);
        room.render_poked(&mut nonfinite, f64::NAN, &[(0.7, 0.3)]);

        assert_eq!(finite.to_text(), nonfinite.to_text());
    }
}
