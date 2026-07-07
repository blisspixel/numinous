//! The Random Walk: stumble blindly, arrive predictably.
//!
//! A crowd of walkers leaves the center, each stumbling one random step at a
//! time. No walker knows where it is going, and yet the crowd as a whole obeys
//! a law: after n steps, the typical distance from home is the square root of
//! n. The circle drawn at that radius is the law made visible; the walkers
//! scatter around it. `t` runs the clock. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the crowd stumbles the same way every time.
const SEED: u64 = 0x0A1C_0000_5EED_0007;
/// How many walkers leave home.
const WALKERS: usize = 60;
/// The most steps `t` reaches.
const MAX_STEPS: usize = 900;

/// One walker's path: unit steps in seeded random directions.
fn walk(id: u64, steps: usize) -> Vec<(f64, f64)> {
    let mut rng = SplitMix64::new(SEED ^ id.wrapping_mul(0x9E37_79B9));
    let (mut x, mut y) = (0.0, 0.0);
    let mut path = Vec::with_capacity(steps + 1);
    path.push((x, y));
    for _ in 0..steps {
        let angle = rng.next_f64() * TAU;
        x += angle.cos();
        y += angle.sin();
        path.push((x, y));
    }
    path
}

/// The Random Walk room.
#[derive(Debug, Default)]
pub struct RandomWalk;

impl RandomWalk {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Steps taken at phase `t`.
    fn steps_for(t: f64) -> usize {
        (t.clamp(0.0, 1.0) * MAX_STEPS as f64) as usize
    }
}

impl Room for RandomWalk {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "random-walk",
            title: "Random Walk",
            wing: "Chance & Order",
            blurb: "Sixty walkers stumble one random step at a time. None knows where it is going; \
                    together they obey the square root law. The circle is the law; t is the clock.",
            accent: [140, 220, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let steps = Self::steps_for(t).max(1);
        let aspect = canvas.char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        // Scale so the sqrt(n) circle sits at 60% of the frame at full time.
        let scale = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.6
            / (MAX_STEPS as f64).sqrt();
        let to_screen =
            |x: f64, y: f64| ((cx + x * scale) as i32, (cy + y * scale * aspect) as i32);

        // The walkers: faint trails, bright endpoints.
        for id in 0..WALKERS {
            let path = walk(id as u64, steps);
            for (i, &(x, y)) in path.iter().enumerate() {
                if i % 7 == 0 {
                    let (px, py) = to_screen(x, y);
                    canvas.plot(px, py, '-');
                }
            }
            if let Some(&(x, y)) = path.last() {
                let (px, py) = to_screen(x, y);
                canvas.plot(px, py, '#');
            }
        }
        // The law: the circle of radius sqrt(steps).
        let law = (steps as f64).sqrt();
        let ring = 240;
        for i in 0..ring {
            let a = TAU * i as f64 / ring as f64;
            let (px, py) = to_screen(law * a.cos(), law * a.sin());
            canvas.plot(px, py, '#');
        }
    }

    fn reveal(&self) -> &'static str {
        "After n blind steps, the typical walker stands square root of n from \
         home: a thousand stumbles carry you only about thirty steps away. This \
         square root law is why perfume crosses a room slowly, why stock prices \
         wander the way they do, and why diffusion takes its sweet time."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Einstein's 1905 paper on this exact stumbling, pollen grains kicked \
             by unseen molecules, was the argument that finally convinced the \
             holdouts that atoms are real. The random walk proved matter is grainy.",
            "In one and two dimensions a random walker returns home with \
             certainty, given forever; in three dimensions it may wander lost for \
             eternity. Polya proved it, and it is why a drunk man finds his way \
             home but a drunk bird may not.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chromatic stumble",
            root: 174.61,
            tempo: 104,
            line: &[0, 1, -1, 2, 1, 3, 2, 0, -2, -1],
            encodes: "steps that never commit: the drunkard's walk, one semitone at a time",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLANT A WALKER")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        self.render(canvas, t);
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let steps = Self::steps_for(t).max(1);
        let aspect = canvas.char_aspect();
        let scale = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.6
            / (MAX_STEPS as f64).sqrt();
        // Each planted walker stumbles from the hand's point, bright.
        for (which, &(x, y)) in pokes.iter().enumerate() {
            let path = walk(1000 + which as u64, steps);
            let (ox, oy) = (x * width as f64, y * height as f64);
            for (i, &(px, py)) in path.iter().enumerate() {
                if i % 5 == 0 {
                    canvas.plot(
                        (ox + px * scale) as i32,
                        (oy + py * scale * aspect) as i32,
                        '*',
                    );
                }
            }
            if let Some(&(px, py)) = path.last() {
                canvas.plot(
                    (ox + px * scale) as i32,
                    (oy + py * scale * aspect) as i32,
                    '#',
                );
            }
        }
    }

    fn postcard_t(&self) -> f64 {
        0.85
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_STEPS, RandomWalk, WALKERS, walk};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn walks_are_deterministic_unit_steps() {
        let a = walk(3, 50);
        let b = walk(3, 50);
        assert_eq!(a, b);
        for pair in a.windows(2) {
            let d = (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1);
            assert!((d - 1.0).abs() < 1e-9, "every step is a unit step");
        }
    }

    #[test]
    fn the_crowd_obeys_the_square_root_law() {
        // RMS distance after n steps concentrates near sqrt(n).
        let n = MAX_STEPS;
        let mut sum_sq = 0.0;
        for id in 0..WALKERS {
            let (x, y) = *walk(id as u64, n).last().unwrap();
            sum_sq += x * x + y * y;
        }
        let rms = (sum_sq / WALKERS as f64).sqrt();
        let law = (n as f64).sqrt();
        assert!(
            (rms / law - 1.0).abs() < 0.25,
            "rms {rms} should sit near sqrt(n) = {law}"
        );
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = RandomWalk::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.85);
        room.render(&mut b, 0.85);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 40);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = RandomWalk::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_states_the_law() {
        assert!(RandomWalk::new().reveal().contains("square root"));
    }
}
