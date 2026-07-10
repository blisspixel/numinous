//! Galton Board: pure chance piling into a bell curve.
//!
//! Each ball falls through a field of pegs, taking a left/right coin flip at
//! every row; its final bin is how many times it went right. No single ball is
//! predictable, yet thousands of them always settle into the same bell curve.
//! `t` biases the coin and skews the curve. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

/// Fixed seed so the pile reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x6A17_0B04_5EED_ABCD;
/// How many balls to drop.
const BALLS: usize = 20_000;
/// How far `t` biases the coin away from fair.
const MAX_BIAS: f64 = 0.25;
/// Cap on the simulated bin count, so the work stays bounded no matter how wide
/// the canvas is. Wider canvases stretch this many bins across their columns.
const MAX_SIM_BINS: usize = 256;

fn drawing_dims(canvas: &dyn Surface) -> Option<(usize, usize)> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width.min(MAX_DIM), height.min(MAX_DIM)))
    }
}

fn poked_balls(pokes: &[(f64, f64)]) -> Vec<(usize, f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(px, py)| {
            if px.is_finite() && py.is_finite() {
                Some((px.clamp(0.0, 1.0), py.clamp(0.0, 1.0)))
            } else {
                None
            }
        })
        .enumerate()
        .map(|(which, (px, py))| (which, px, py))
        .collect()
}

/// The Galton Board room.
#[derive(Debug, Default)]
pub struct GaltonBoard {
    seed: u64,
}

impl GaltonBoard {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// Drop the balls and tally how many land in each of `bins` bins.
    ///
    /// A ball takes `bins - 1` coin flips, so its bin (the number of rights) is
    /// always in `0..bins`. `t` biases the probability of going right.
    fn histogram(bins: usize, t: f64, variation: u64) -> Vec<u64> {
        let mut counts = vec![0u64; bins];
        if bins == 0 {
            return counts;
        }
        let rows = bins - 1;
        let p_right = 0.5 + MAX_BIAS * t.clamp(0.0, 1.0);
        let mut rng = SplitMix64::new(SEED ^ variation);
        for _ in 0..BALLS {
            let mut bin = 0usize;
            for _ in 0..rows {
                if rng.next_f64() < p_right {
                    bin += 1;
                }
            }
            counts[bin] += 1;
        }
        counts
    }

    fn ball_trace(bins: usize, x: f64, y: f64, variation: u64, which: usize) -> Vec<usize> {
        let bins = bins.clamp(2, MAX_SIM_BINS);
        let rows = bins - 1;
        let mut column = (unit(x, 0.5) * rows as f64).round() as usize;
        let p_right = 0.15 + 0.70 * unit(y, 0.5);
        let mut rng = SplitMix64::new(
            SEED ^ variation ^ 0xB411_7ACE_u64 ^ (which as u64).wrapping_mul(0x9E37_79B9),
        );
        let mut trace = Vec::with_capacity(rows + 1);
        trace.push(column);
        for _ in 0..rows {
            if rng.next_f64() < p_right {
                column = (column + 1).min(rows);
            } else {
                column = column.saturating_sub(1);
            }
            trace.push(column);
        }
        trace
    }

    fn draw_ball_trace(canvas: &mut dyn Surface, trace: &[usize], bins: usize) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        if trace.is_empty() {
            return;
        }
        let x_span = (bins.clamp(2, MAX_SIM_BINS) - 1) as f64;
        let y_span = (trace.len() - 1).max(1) as f64;
        let mut previous = None;
        for (row, &bin) in trace.iter().enumerate() {
            let sx = ((bin as f64 / x_span) * (width.saturating_sub(1)) as f64).round() as i32;
            let sy = ((row as f64 / y_span) * (height.saturating_sub(1)) as f64).round() as i32;
            if let Some((px, py)) = previous {
                canvas.line(px, py, sx, sy, 'o');
            } else {
                canvas.plot(sx, sy, 'o');
            }
            previous = Some((sx, sy));
        }
        if let Some(&bin) = trace.last() {
            let sx = ((bin as f64 / x_span) * (width.saturating_sub(1)) as f64).round() as i32;
            canvas.plot(sx, height.saturating_sub(1) as i32, '#');
        }
    }
}

fn unit(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

impl Room for GaltonBoard {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "galton-board",
            title: "Galton Board",
            wing: "Chance & Order",
            blurb: "Drop thousands of balls through pegs, each a coin flip left or right, and pure \
                    chaos piles into the same bell curve every time. t biases the coin.",
            accent: [80, 120, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let sim_bins = width.min(MAX_SIM_BINS);
        let counts = Self::histogram(sim_bins, t, self.seed);
        let max = counts.iter().copied().max().unwrap_or(0);
        if max == 0 {
            return;
        }
        for x in 0..width {
            // Map the canvas column onto a simulated bin. This is the identity
            // when the canvas fits, and stretches the bins across wider canvases.
            let bin = (x * sim_bins / width).min(sim_bins - 1);
            let count = counts[bin];
            let bar = ((count as f64 / max as f64) * height as f64).round() as usize;
            for row in 0..bar {
                canvas.plot(x as i32, (height - 1 - row) as i32, '*');
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "You cannot predict where a single ball lands, yet together thousands of \
         them form the same bell curve every time, to the millimeter. This is the \
         Central Limit Theorem, the reason the bell curve rules heights, test \
         scores, and the stock market. Chaos, in bulk, is perfectly predictable."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C bell curve",
            root: 130.81,
            tempo: 120,
            line: &[0, 2, 4, 5, 7, 5, 4, 2, 0],
            encodes: "left-right falls gathering around the center",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DROP A BALL")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let sim_bins = width.min(MAX_SIM_BINS);
        let counts = Self::histogram(sim_bins, t, self.seed);
        let max = counts.iter().copied().max().unwrap_or(0);
        if max == 0 {
            return;
        }
        // base
        for x in 0..width {
            let bin = (x * sim_bins / width).min(sim_bins - 1);
            let count = counts[bin];
            let bar = ((count as f64 / max as f64) * height as f64).round() as usize;
            for row in 0..bar {
                canvas.plot(x as i32, (height - 1 - row) as i32, '*');
            }
        }
        // A click drops a single visible ball over the crowd. Horizontal hand
        // position chooses the lane; vertical hand position tilts that ball's
        // coin, making the individual path contrast with the aggregate curve.
        for (which, px, py) in poked_balls(pokes) {
            let trace = Self::ball_trace(sim_bins, px, py, self.seed, which);
            Self::draw_ball_trace(canvas, &trace, sim_bins);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GaltonBoard, poked_balls};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room};
    use crate::surface::{MAX_DIM, Surface};

    fn argmax(counts: &[u64]) -> usize {
        counts
            .iter()
            .enumerate()
            .max_by_key(|&(_, c)| *c)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    #[test]
    fn fair_coin_peaks_at_the_center() {
        let counts = GaltonBoard::histogram(21, 0.0, 0);
        // 21 bins means 20 flips, so the mean bin is 10.
        assert!((argmax(&counts) as i64 - 10).abs() <= 2);
    }

    #[test]
    fn biasing_shifts_the_peak_right() {
        let fair = GaltonBoard::histogram(21, 0.0, 0);
        let biased = GaltonBoard::histogram(21, 1.0, 0);
        assert!(argmax(&biased) > argmax(&fair));
    }

    #[test]
    fn total_count_is_conserved() {
        let counts = GaltonBoard::histogram(15, 0.3, 0);
        assert_eq!(counts.iter().sum::<u64>(), super::BALLS as u64);
    }

    #[test]
    fn render_is_deterministic() {
        let room = GaltonBoard::new();
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = GaltonBoard::new_with(0);
        let r_def = GaltonBoard::new();
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);
        r0.render(&mut a, 0.0);
        r_def.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        let r42 = GaltonBoard::new_with(42);
        let mut c = Canvas::new(41, 16);
        r42.render(&mut c, 0.0);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = GaltonBoard::new();
        let mut canvas = Canvas::new(41, 16);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn wide_canvas_stays_bounded_and_fills() {
        // Wider than MAX_SIM_BINS: exercises the stretch path and stays fast.
        let room = GaltonBoard::new();
        let mut canvas = Canvas::new(600, 12);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = GaltonBoard::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(5, 5);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn poked_balls_preserve_order_clamp_and_filter() {
        assert_eq!(
            poked_balls(&[
                (-1.0, 2.0),
                (0.25, 0.75),
                (f64::INFINITY, 0.25),
                (0.5, f64::NAN),
            ]),
            vec![(0, 0.0, 1.0), (1, 0.25, 0.75)]
        );
    }

    #[test]
    fn poked_balls_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        many.extend(
            (0..MAX_ROOM_POKES).map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.5)),
        );
        let newest = many[many.len() - MAX_ROOM_POKES..].to_vec();

        assert_eq!(poked_balls(&many), poked_balls(&newest));
        assert_eq!(poked_balls(&many).len(), MAX_ROOM_POKES);
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(poked_balls(&with_invalid_tail).is_empty());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_ball_identity() {
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(poked_balls(&with_bad_points), poked_balls(&finite));
    }

    #[test]
    fn duplicate_pokes_replay_as_distinct_balls() {
        let balls = poked_balls(&[(0.5, 0.5), (0.5, 0.5)]);
        let first = GaltonBoard::ball_trace(21, balls[0].1, balls[0].2, 0, balls[0].0);
        let second = GaltonBoard::ball_trace(21, balls[1].1, balls[1].2, 0, balls[1].0);

        assert_eq!(balls, vec![(0, 0.5, 0.5), (1, 0.5, 0.5)]);
        assert_ne!(first, second);
    }

    #[test]
    fn variation_changes_the_dropped_ball_trace_directly() {
        assert_ne!(
            GaltonBoard::ball_trace(21, 0.5, 0.5, 0, 0),
            GaltonBoard::ball_trace(21, 0.5, 0.5, 42, 0)
        );
    }

    #[test]
    fn reveal_names_the_theorem() {
        assert!(
            GaltonBoard::new()
                .reveal()
                .contains("Central Limit Theorem")
        );
    }

    #[test]
    fn poked_changes_output() {
        let r0 = GaltonBoard::new_with(0);
        let mut a = Canvas::new(41, 16);
        let mut p = Canvas::new(41, 16);
        r0.render(&mut a, 0.0);
        r0.render_poked(&mut p, 0.0, &[(0.5, 0.5)]);
        assert_ne!(p.to_text(), a.to_text());
        assert!(p.ink_count() >= a.ink_count());
        assert!(p.to_text().contains('#'), "the dropped ball lands visibly");
    }

    #[test]
    fn dropped_ball_uses_both_hand_coordinates() {
        let left = GaltonBoard::ball_trace(21, 0.1, 0.5, 0, 0);
        let right = GaltonBoard::ball_trace(21, 0.9, 0.5, 0, 0);
        assert!(left.first().unwrap() < right.first().unwrap());

        let biased_left = GaltonBoard::ball_trace(21, 0.5, 0.0, 0, 0);
        let biased_right = GaltonBoard::ball_trace(21, 0.5, 1.0, 0, 0);
        assert!(biased_left.last().unwrap() < biased_right.last().unwrap());
    }

    #[test]
    fn poked_ball_is_deterministic_and_safe() {
        let room = GaltonBoard::new_with(7);
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);
        room.render_poked(&mut a, 0.0, &[(0.25, 0.8), (f64::NAN, f64::INFINITY)]);
        room.render_poked(&mut b, 0.0, &[(0.25, 0.8), (f64::NAN, f64::INFINITY)]);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.to_text().contains('o'));
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = GaltonBoard::new();
        let newest = vec![(0.85, 0.85); MAX_ROOM_POKES];
        let mut all = vec![(0.1, 0.1); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();
        let mut expected = Canvas::new(41, 16);
        let mut actual = Canvas::new(41, 16);
        let mut prefix_only = Canvas::new(41, 16);

        room.render_poked(&mut expected, 0.0, &newest);
        room.render_poked(&mut actual, 0.0, &all);
        room.render_poked(&mut prefix_only, 0.0, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(actual.to_text(), prefix_only.to_text());
    }

    #[test]
    fn all_invalid_pokes_match_base_render() {
        let room = GaltonBoard::new();
        let mut base = Canvas::new(41, 16);
        let mut poked = Canvas::new(41, 16);

        room.render(&mut base, f64::NAN);
        room.render_poked(
            &mut poked,
            f64::NAN,
            &[(f64::NAN, f64::INFINITY), (f64::INFINITY, 0.5)],
        );

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn all_invalid_newest_tail_discards_older_valid_pokes() {
        let room = GaltonBoard::new();
        let mut with_valid_prefix = vec![(0.5, 0.5); MAX_ROOM_POKES];
        with_valid_prefix.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);
        let mut base = Canvas::new(41, 16);
        let mut poked = Canvas::new(41, 16);

        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &with_valid_prefix);

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn nonzero_seed_changes_poked_render() {
        let r0 = GaltonBoard::new_with(0);
        let r42 = GaltonBoard::new_with(42);
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);

        r0.render_poked(&mut a, 0.0, &[(0.5, 0.5)]);
        r42.render_poked(&mut b, 0.0, &[(0.5, 0.5)]);

        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_columns() {
        #[derive(Default)]
        struct HugeSurface {
            width: usize,
            height: usize,
            plots: usize,
            max_abs_coord: i32,
        }

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = GaltonBoard::new();
        for (width, height) in [(usize::MAX, 12), (12, usize::MAX), (usize::MAX, usize::MAX)] {
            let mut surface = HugeSurface {
                width,
                height,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, 0.0, &[(1.0, 1.0)]);

            assert!(surface.plots < MAX_DIM * MAX_DIM);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn sound_uses_the_default_tone() {
        // Galton does not override sound, so it gets the default single tone.
        let spec = GaltonBoard::new().sound(0.0);
        assert_eq!(spec.notes.len(), 1);
    }
}
