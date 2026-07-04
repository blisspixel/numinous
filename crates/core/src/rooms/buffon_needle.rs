//! Buffon's Needle: pi out of thrown sticks.
//!
//! Drop needles on a floor of evenly spaced parallel lines and count how many
//! cross a line. The crossing fraction is `2 l / (pi d)`, so pi falls out of an
//! experiment with no circle anywhere in it. This room drops needles on a lined
//! canvas (crossing needles highlighted) and can estimate pi. `t` changes the
//! needle length. See `docs/ROOMS.md`.

use std::f64::consts::PI;

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the throw reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x0B0F_0000_5EED_F00D;
/// Number of needles dropped.
const NEEDLES: usize = 1500;
/// Rows between floor lines, in canvas cells.
const SPACING: f64 = 4.0;

/// The Buffon's Needle room.
#[derive(Debug, Default)]
pub struct BuffonNeedle;

impl BuffonNeedle {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The needle-length-to-spacing ratio for phase `t`; 1.0 (the classic case) at `t = 0`.
    fn length_ratio_for(t: f64) -> f64 {
        1.0 - 0.6 * t.clamp(0.0, 1.0)
    }

    /// Estimate pi by dropping `needles` needles with the given length ratio.
    ///
    /// Deterministic (fixed seed). Returns infinity if nothing crosses. Exposed
    /// so a face can display the running estimate; the render itself only draws
    /// the experiment.
    #[must_use]
    pub fn estimate_pi(needles: usize, length_ratio: f64) -> f64 {
        let mut rng = SplitMix64::new(SEED);
        let half = length_ratio / 2.0;
        let mut crossings = 0usize;
        for _ in 0..needles {
            let center = rng.next_f64(); // within one unit-spaced strip
            let angle = rng.next_f64() * PI;
            if crosses(center, angle, half, 1.0) {
                crossings += 1;
            }
        }
        if crossings == 0 {
            return f64::INFINITY;
        }
        2.0 * length_ratio * needles as f64 / crossings as f64
    }
}

impl Room for BuffonNeedle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "buffon-needle",
            title: "Buffon's Needle",
            wing: "Chance & Order",
            blurb: "Drop needles on a lined floor and count how many cross a line; the count \
                    secretly holds pi, with no circle in sight. t changes the needle length.",
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // Draw the floor lines.
        let mut row = 0usize;
        while row < height {
            for x in 0..width {
                canvas.plot(x as i32, row as i32, '-');
            }
            row += SPACING as usize;
        }

        let half_len = Self::length_ratio_for(t) * SPACING / 2.0;
        let (fw, fh) = (width as f64, height as f64);
        let mut rng = SplitMix64::new(SEED);
        for _ in 0..NEEDLES {
            let cx = rng.next_f64() * fw;
            let cy = rng.next_f64() * fh;
            let angle = rng.next_f64() * PI;
            let (hx, hy) = (half_len * angle.cos(), half_len * angle.sin());
            let mark = if crosses(cy, angle, half_len, SPACING) {
                '#'
            } else {
                '*'
            };
            canvas.line(
                (cx - hx).round() as i32,
                (cy - hy).round() as i32,
                (cx + hx).round() as i32,
                (cy + hy).round() as i32,
                mark,
            );
        }
    }

    fn reveal(&self) -> &'static str {
        "There is no circle here, just sticks on a floor, yet pi, the circle's own \
         number, appears out of nowhere. This is the seed of the Monte Carlo \
         method, which helped design the atom bomb and powers modern finance and AI.\
         You can compute the universe by throwing dice."
    }
}

/// Whether a needle whose center sits at `center` (in strips of width `spacing`)
/// and makes angle `angle` with the lines crosses a line, given half its length.
fn crosses(center: f64, angle: f64, half_len: f64, spacing: f64) -> bool {
    let reach = half_len * angle.sin().abs();
    let within_strip = center.rem_euclid(spacing);
    let distance_to_nearest_line = within_strip.min(spacing - within_strip);
    distance_to_nearest_line <= reach
}

#[cfg(test)]
mod tests {
    use super::{BuffonNeedle, crosses};
    use crate::canvas::Canvas;
    use crate::room::Room;
    use std::f64::consts::PI;

    #[test]
    fn crossing_test_matches_geometry() {
        // A vertical needle of length 1 centered mid-strip reaches both lines.
        assert!(crosses(0.5, PI / 2.0, 0.5, 1.0));
        // A needle parallel to the lines has no vertical reach; mid-strip it misses.
        assert!(!crosses(0.5, 0.0, 0.5, 1.0));
    }

    #[test]
    fn estimate_converges_to_pi() {
        let estimate = BuffonNeedle::estimate_pi(200_000, 1.0);
        assert!((estimate - PI).abs() < 0.1, "estimate was {estimate}");
    }

    #[test]
    fn length_ratio_defaults_to_one() {
        assert!((BuffonNeedle::length_ratio_for(0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn render_is_deterministic() {
        let room = BuffonNeedle::new();
        let mut a = Canvas::new(50, 24);
        let mut b = Canvas::new(50, 24);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = BuffonNeedle::new();
        let mut canvas = Canvas::new(50, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = BuffonNeedle::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(5, 5);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_monte_carlo() {
        assert!(BuffonNeedle::new().reveal().contains("Monte Carlo"));
    }
}
