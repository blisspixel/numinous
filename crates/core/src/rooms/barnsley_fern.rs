//! The Barnsley fern: a living plant grown from four random rules.
//!
//! Start at a point and, over and over, pick one of four affine transformations
//! at random (with fixed probabilities) and apply it. The points settle onto a
//! fern that looks convincingly alive, an iterated function system. `t` grows the
//! fern by drawing more points. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the fern grows the same way every time.
const SEED: u64 = 0xFE87_0000_5EED_1234;
/// Points always drawn, and the extra points `t` adds.
const BASE_POINTS: usize = 3_000;
const SWEEP_POINTS: usize = 25_000;

/// The Barnsley fern room.
#[derive(Debug, Default)]
pub struct BarnsleyFern;

impl BarnsleyFern {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// How many points to draw at phase `t`.
    fn points_for(t: f64) -> usize {
        BASE_POINTS + (t.clamp(0.0, 1.0) * SWEEP_POINTS as f64) as usize
    }
}

/// Apply one of the fern's four affine maps, chosen by `r` in `[0, 1)`.
fn next_point(x: f64, y: f64, r: f64) -> (f64, f64) {
    if r < 0.01 {
        (0.0, 0.16 * y)
    } else if r < 0.86 {
        (0.85 * x + 0.04 * y, -0.04 * x + 0.85 * y + 1.6)
    } else if r < 0.93 {
        (0.2 * x - 0.26 * y, 0.23 * x + 0.22 * y + 1.6)
    } else {
        (-0.15 * x + 0.28 * y, 0.26 * x + 0.24 * y + 0.44)
    }
}

impl Room for BarnsleyFern {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "barnsley-fern",
            title: "Barnsley Fern",
            wing: "Fractals & the Infinite",
            blurb: "Pick one of four simple transformations at random, over and over, and a fern \
                    grows out of the noise. t adds more points, growing the plant before your eyes.",
            accent: [60, 200, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let mut rng = SplitMix64::new(SEED);
        let (mut x, mut y) = (0.0_f64, 0.0_f64);
        for _ in 0..Self::points_for(t) {
            let (nx, ny) = next_point(x, y, rng.next_f64());
            x = nx;
            y = ny;
            // Fern lives in x within [-2.5, 3.0] and y within [0, 10].
            let sx = ((x + 2.5) / 5.5 * width as f64) as i32;
            let sy = (height as f64 - (y / 10.0) * height as f64) as i32;
            canvas.plot(sx, sy, '#');
        }
    }

    fn postcard_t(&self) -> f64 {
        1.0
    }

    fn reveal(&self) -> &'static str {
        "This fern is not drawn, it is grown. Four transformations applied at \
         random build a plant, self-similar down to each frond. The entire \
         genome of this fern fits on an index card."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Barnsley's collage theorem runs the trick backward: given any image, it \
             tells you how to find transformations whose attractor approximates it. \
             This became fractal image compression, which shipped on CD-ROM \
             encyclopedias in the nineties.",
            "The fern needs the probabilities as much as the maps: pick the four \
             transformations uniformly and the fern grows stunted and patchy. The 85 \
             percent rule is what fills the frond evenly. Randomness, tuned.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{BarnsleyFern, SEED, next_point};
    use crate::canvas::Canvas;
    use crate::rng::SplitMix64;
    use crate::room::Room;

    #[test]
    fn points_stay_within_the_fern_bounds() {
        let mut rng = SplitMix64::new(SEED);
        let (mut x, mut y) = (0.0, 0.0);
        for _ in 0..5_000 {
            let (nx, ny) = next_point(x, y, rng.next_f64());
            x = nx;
            y = ny;
            assert!(x > -3.0 && x < 3.5, "x out of bounds: {x}");
            assert!((0.0..11.0).contains(&y), "y out of bounds: {y}");
        }
    }

    #[test]
    fn more_phase_grows_the_fern() {
        assert!(BarnsleyFern::points_for(1.0) > BarnsleyFern::points_for(0.0));
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = BarnsleyFern::new();
        let mut a = Canvas::new(40, 40);
        let mut b = Canvas::new(40, 40);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = BarnsleyFern::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_the_index_card() {
        assert!(BarnsleyFern::new().reveal().contains("index card"));
    }
}
