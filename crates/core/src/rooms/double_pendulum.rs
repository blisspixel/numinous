//! The double pendulum: two hinges, and physics gives up on prediction.
//!
//! One pendulum hanging from another. The equations are exact, the motion is
//! deterministic, and it still cannot be forecast, because any error in the
//! starting angle grows exponentially. A shadow pendulum started a breath away
//! rides along to show the divergence happen. `t` runs the clock. See the Full
//! Map in `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Gravity.
const G: f64 = 9.81;
/// Integration time step.
const DT: f64 = 0.0025;
/// The most integration steps `t` reaches.
const MAX_STEPS: usize = 6_000;
/// The breath of difference the shadow starts with, in radians.
const SHADOW_OFFSET: f64 = 1e-4;

/// The pendulum state: angles and angular velocities of the two arms.
#[derive(Debug, Clone, Copy)]
struct State {
    t1: f64,
    t2: f64,
    w1: f64,
    w2: f64,
}

/// One Euler step of the standard equal-mass, equal-length double pendulum.
fn step(s: State) -> State {
    let delta = s.t1 - s.t2;
    let den = 3.0 - (2.0 * delta).cos();
    let a1 = (-3.0 * G * s.t1.sin()
        - G * (s.t1 - 2.0 * s.t2).sin()
        - 2.0 * delta.sin() * (s.w2 * s.w2 + s.w1 * s.w1 * delta.cos()))
        / den;
    let a2 = (2.0
        * delta.sin()
        * (2.0 * s.w1 * s.w1 + 2.0 * G * s.t1.cos() + s.w2 * s.w2 * delta.cos()))
        / den;
    State {
        t1: s.t1 + s.w1 * DT,
        t2: s.t2 + s.w2 * DT,
        w1: s.w1 + a1 * DT,
        w2: s.w2 + a2 * DT,
    }
}

/// Integrate from the standard drop for `steps`, recording the tip path.
fn trace(offset: f64, steps: usize) -> Vec<(f64, f64)> {
    let mut s = State {
        t1: 2.0 + offset,
        t2: 2.0,
        w1: 0.0,
        w2: 0.0,
    };
    let mut path = Vec::with_capacity(steps / 3 + 1);
    for i in 0..steps.min(MAX_STEPS) {
        s = step(s);
        if i % 3 == 0 {
            let x = s.t1.sin() + s.t2.sin();
            let y = s.t1.cos() + s.t2.cos();
            path.push((x, y));
        }
    }
    path
}

/// The double pendulum room.
#[derive(Debug, Default)]
pub struct DoublePendulum;

impl DoublePendulum {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for DoublePendulum {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "double-pendulum",
            title: "Double Pendulum",
            wing: "Chaos & Order",
            blurb: "One pendulum hanging from another. Exact equations, deterministic motion, and \
                    still unforecastable: a shadow twin a breath away peels off before your eyes.",
            accent: [255, 110, 110],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let steps = (t.clamp(0.0, 1.0) * MAX_STEPS as f64) as usize;
        let aspect = canvas.char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let radius = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.45;
        let to_screen = |x: f64, y: f64| -> (i32, i32) {
            (
                (cx + x * radius / 2.0) as i32,
                (cy + y * radius / 2.0 * aspect) as i32,
            )
        };
        // The shadow twin's path first, dim, so the divergence reads on top.
        for &(x, y) in &trace(SHADOW_OFFSET, steps) {
            let (px, py) = to_screen(x, y);
            canvas.plot(px, py, '-');
        }
        // The pendulum's own path, bright.
        let path = trace(0.0, steps);
        for &(x, y) in &path {
            let (px, py) = to_screen(x, y);
            canvas.plot(px, py, '*');
        }
        // The arms, at the final instant (or the starting pose, before the
        // drop, so the room is never blank).
        let (x, y) = path.last().copied().unwrap_or_else(|| {
            let t1 = 2.0_f64;
            (2.0 * t1.sin(), 2.0 * t1.cos())
        });
        let pivot = to_screen(0.0, 0.0);
        let tip = to_screen(x, y);
        canvas.line(pivot.0, pivot.1, tip.0, tip.1, '#');
    }

    fn reveal(&self) -> &'static str {
        "Both pendulums obey the same exact equations; nothing here is random. \
         The shadow started one ten-thousandth of a radian away and ended \
         somewhere else entirely. Determinism and predictability are different \
         things, and this is the cheapest machine that proves it."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "The error between the twins grows exponentially, and the growth rate \
             has a name, the Lyapunov exponent. Its inverse tells you how far \
             ahead any forecast can possibly see. For weather, that horizon is \
             about two weeks, and no computer will ever move it much.",
            "There is no closed-form solution to this system and there never will \
             be: it was proven nonintegrable. Every double pendulum you have ever \
             seen simulated was computed step by tiny step, like this one.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.75
    }
}

#[cfg(test)]
mod tests {
    use super::{DoublePendulum, trace};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_tip_stays_within_reach() {
        // Two unit arms: the tip can never be farther than 2 from the pivot.
        for &(x, y) in &trace(0.0, 6_000) {
            assert!(x.hypot(y) <= 2.0 + 1e-6, "escaped: ({x}, {y})");
        }
    }

    #[test]
    fn a_breath_of_difference_diverges() {
        let a = trace(0.0, 6_000);
        let b = trace(super::SHADOW_OFFSET, 6_000);
        let (ax, ay) = *a.last().unwrap();
        let (bx, by) = *b.last().unwrap();
        let gap = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
        assert!(gap > 0.1, "the twins should part company: gap {gap}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = DoublePendulum::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.75);
        room.render(&mut b, 0.75);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
        // Never blank, even before the drop.
        let mut zero = Canvas::new(50, 30);
        room.render(&mut zero, 0.0);
        assert!(zero.ink_count() > 3, "the starting pose shows");
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = DoublePendulum::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_separates_determinism_from_prediction() {
        assert!(DoublePendulum::new().reveal().contains("predictability"));
    }
}
