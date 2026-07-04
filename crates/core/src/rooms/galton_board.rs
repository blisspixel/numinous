//! Galton Board: pure chance piling into a bell curve.
//!
//! Each ball falls through a field of pegs, taking a left/right coin flip at
//! every row; its final bin is how many times it went right. No single ball is
//! predictable, yet thousands of them always settle into the same bell curve.
//! `t` biases the coin and skews the curve. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the pile reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x6A17_0B04_5EED_ABCD;
/// How many balls to drop.
const BALLS: usize = 20_000;
/// How far `t` biases the coin away from fair.
const MAX_BIAS: f64 = 0.25;
/// Cap on the simulated bin count, so the work stays bounded no matter how wide
/// the canvas is. Wider canvases stretch this many bins across their columns.
const MAX_SIM_BINS: usize = 256;

/// The Galton Board room.
#[derive(Debug, Default)]
pub struct GaltonBoard;

impl GaltonBoard {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Drop the balls and tally how many land in each of `bins` bins.
    ///
    /// A ball takes `bins - 1` coin flips, so its bin (the number of rights) is
    /// always in `0..bins`. `t` biases the probability of going right.
    fn histogram(bins: usize, t: f64) -> Vec<u64> {
        let mut counts = vec![0u64; bins];
        if bins == 0 {
            return counts;
        }
        let rows = bins - 1;
        let p_right = 0.5 + MAX_BIAS * t.clamp(0.0, 1.0);
        let mut rng = SplitMix64::new(SEED);
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
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let sim_bins = width.min(MAX_SIM_BINS);
        let counts = Self::histogram(sim_bins, t);
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
}

#[cfg(test)]
mod tests {
    use super::GaltonBoard;
    use crate::canvas::Canvas;
    use crate::room::Room;

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
        let counts = GaltonBoard::histogram(21, 0.0);
        // 21 bins means 20 flips, so the mean bin is 10.
        assert!((argmax(&counts) as i64 - 10).abs() <= 2);
    }

    #[test]
    fn biasing_shifts_the_peak_right() {
        let fair = GaltonBoard::histogram(21, 0.0);
        let biased = GaltonBoard::histogram(21, 1.0);
        assert!(argmax(&biased) > argmax(&fair));
    }

    #[test]
    fn total_count_is_conserved() {
        let counts = GaltonBoard::histogram(15, 0.3);
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
    fn reveal_names_the_theorem() {
        assert!(
            GaltonBoard::new()
                .reveal()
                .contains("Central Limit Theorem")
        );
    }

    #[test]
    fn sound_uses_the_default_tone() {
        // Galton does not override sound, so it gets the default single tone.
        let spec = GaltonBoard::new().sound(0.0);
        assert_eq!(spec.notes.len(), 1);
    }
}
