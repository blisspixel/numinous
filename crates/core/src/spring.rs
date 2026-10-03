//! Critically damped springs: how a presented value follows a written one.
//!
//! A hand writes a parameter, a dial position or a Studio knob, and the
//! written value is the truth. Grading, replay, receipts, and every other face
//! read it exactly. What a real-time face presents between writes may ease, so
//! a dial reads as a physical thing with inertia rather than a value that
//! teleports (`docs/VISUALS.md`, "Everything eases").
//!
//! A [`Spring`] holds only the difference between the presented value and the
//! written one, and the rate at which that difference is changing. A write
//! moves the written value and leaves the presented value where it was, so
//! the picture stays continuous. Time then closes the difference with critical
//! damping, the fastest approach that never overshoots. Once the difference is
//! inside the spring's tolerance it is set to exactly zero, so a spring at rest
//! presents the written value bit for bit and cannot change what a room
//! computes.
//!
//! # The step
//!
//! The difference `d` obeys `d'' + 2 w d' + w^2 d = 0` for the stiffness `w`.
//! Over a step of `h` seconds, with `e = exp(-w h)` and `c = v + w d`, the
//! exact solution is
//!
//! ```text
//! d <- (d + c h) e
//! v <- (v - w c h) e
//! ```
//!
//! It is the solution, not a numerical integration, so it is stable for any
//! frame time: a stalled frame lands where the motion would have been, never
//! past it, and many short steps arrive where one long step does.
//!
//! # Never past the written value
//!
//! The spring keeps its velocity pointing toward the written value and no
//! faster than `w |d|`. Under that invariant `c` has the sign of `d` or is
//! zero, so `d` keeps its sign and shrinks monotonically: the presented value
//! approaches the written one from one side and never passes it. A write that
//! would break the invariant, a hand reversing behind the presented value, has
//! its velocity clamped back into it, so the presented value turns toward the
//! new target instead of swinging past the old one. A presented value is
//! therefore always one the hand has passed through.
//!
//! Reduced motion applies every write at once (see [`Motion`]).

use crate::Motion;

/// A critically damped spring that eases a presented value toward the value
/// that was written, without ever passing it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    /// Stiffness in radians per second. Zero means writes apply at once.
    omega: f64,
    /// The difference below which the spring snaps to rest.
    tolerance: f64,
    /// Presented minus written.
    offset: f64,
    /// Rate of change of `offset`, per second.
    velocity: f64,
}

impl Spring {
    /// A spring at rest with stiffness `omega` (radians per second) that
    /// snaps to rest once the presented value is within `tolerance` of the
    /// written one.
    ///
    /// A critically damped spring covers about half of a step in
    /// `1.68 / omega` seconds and all but one percent in `6.64 / omega`.
    /// A stiffness that is not a positive finite number makes every write
    /// apply at once, and a tolerance that is not a finite non-negative
    /// number is treated as zero. Invalid stiffness disables easing; invalid
    /// tolerance disables snapping before the difference reaches zero.
    #[must_use]
    pub fn new(omega: f64, tolerance: f64) -> Self {
        Self {
            omega: if omega.is_finite() && omega > 0.0 {
                omega
            } else {
                0.0
            },
            tolerance: if tolerance.is_finite() && tolerance > 0.0 {
                tolerance
            } else {
                0.0
            },
            offset: 0.0,
            velocity: 0.0,
        }
    }

    /// The value to present while `written` is the value the hand wrote.
    ///
    /// At rest this is `written` itself, bit for bit.
    #[must_use]
    pub fn present(&self, written: f64) -> f64 {
        if self.offset == 0.0 {
            written
        } else {
            written + self.offset
        }
    }

    /// Presented minus written: zero exactly when the spring is at rest.
    #[must_use]
    pub fn offset(&self) -> f64 {
        self.offset
    }

    /// Whether the presented value is the written value.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.offset == 0.0 && self.velocity == 0.0
    }

    /// Come to rest on the written value at once.
    pub fn settle(&mut self) {
        self.offset = 0.0;
        self.velocity = 0.0;
    }

    /// The written value moved from `from` to `to`.
    ///
    /// The presented value stays where it was and eases toward `to` as time
    /// advances. Reduced motion, a spring with no stiffness, and a write that
    /// is not finite on both ends all apply the write at once instead, because
    /// a jump the player asked for is better than an ease toward a value that
    /// cannot be shown.
    pub fn retarget(&mut self, from: f64, to: f64, motion: Motion) {
        if !motion.animates() || self.omega == 0.0 || !from.is_finite() || !to.is_finite() {
            self.settle();
            return;
        }
        self.offset += from - to;
        if !self.offset.is_finite() {
            self.settle();
            return;
        }
        self.hold_invariant();
    }

    /// Let `seconds` of presentation time pass.
    ///
    /// Time that is negative or not a number moves nothing. Unbounded time
    /// settles, which is where any finite amount of enough time arrives.
    pub fn advance(&mut self, seconds: f64) {
        if self.is_settled() || seconds.is_nan() || seconds <= 0.0 {
            return;
        }
        let omega = self.omega;
        let decay = (-omega * seconds).exp();
        let toward = self.velocity + omega * self.offset;
        let offset = (self.offset + toward * seconds) * decay;
        let velocity = (self.velocity - omega * toward * seconds) * decay;
        if !offset.is_finite() || !velocity.is_finite() || offset.abs() <= self.tolerance {
            self.settle();
            return;
        }
        self.offset = offset;
        self.velocity = velocity;
        self.hold_invariant();
    }

    /// Keep the velocity pointing at the written value and no faster than
    /// `omega |offset|`, the condition under which the approach can neither
    /// turn away nor cross. See the module notes.
    fn hold_invariant(&mut self) {
        let limit = self.omega * self.offset.abs();
        // Velocity measured toward the written value: positive closes the gap.
        let closing = -self.velocity * self.offset.signum();
        let closing = closing.clamp(0.0, limit);
        self.velocity = -closing * self.offset.signum();
        if self.offset == 0.0 {
            self.velocity = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Spring;
    use crate::Motion;

    const OMEGA: f64 = 30.0;
    const TOLERANCE: f64 = 1e-4;

    fn spring() -> Spring {
        Spring::new(OMEGA, TOLERANCE)
    }

    /// Frame times from a tenth of a millisecond to ten seconds.
    fn frame_times() -> Vec<f64> {
        let mut times = Vec::new();
        let mut dt = 1e-4;
        while dt <= 10.0 {
            times.push(dt);
            dt *= 1.37;
        }
        times
    }

    #[test]
    fn a_spring_at_rest_presents_the_written_value_bit_for_bit() {
        let rest = spring();
        for written in [0.0, -0.0, 0.25, 1.0, -3.5, f64::MIN_POSITIVE, 1e300] {
            assert_eq!(
                rest.present(written).to_bits(),
                written.to_bits(),
                "{written}"
            );
        }
        assert!(rest.is_settled());
    }

    #[test]
    fn a_write_keeps_the_presented_value_where_it_was() {
        let mut moving = spring();
        moving.retarget(0.2, 0.8, Motion::Full);
        assert!((moving.present(0.8) - 0.2).abs() < 1e-15);
        moving.advance(0.03);
        let before = moving.present(0.8);
        moving.retarget(0.8, 0.5, Motion::Full);
        assert!((moving.present(0.5) - before).abs() < 1e-15);
    }

    #[test]
    fn the_approach_follows_the_critically_damped_curve_at_any_frame_rate() {
        // From rest, the difference is d0 (1 + w t) e^(-w t). One long step
        // and a thousand short ones must both land on it.
        for (steps, dt) in [(1, 0.1), (10, 0.01), (1000, 0.0001)] {
            let mut eased = spring();
            eased.retarget(0.0, 1.0, Motion::Full);
            for _ in 0..steps {
                eased.advance(dt);
            }
            let t: f64 = 0.1;
            let expected = -(1.0 + OMEGA * t) * (-OMEGA * t).exp();
            assert!(
                (eased.offset() - expected).abs() < 1e-11,
                "{steps} steps of {dt}: {} against {expected}",
                eased.offset()
            );
        }
    }

    #[test]
    fn no_frame_time_or_starting_state_overshoots_or_turns_away() {
        // Every pairing of a frame time with a start: from rest, mid-flight,
        // and after a write that reverses a moving value.
        for dt in frame_times() {
            for (from, to, then) in [
                (0.0, 1.0, None),
                (1.0, 0.0, None),
                (0.3, 0.31, None),
                (0.0, 1.0, Some(0.2)),
                (0.0, 1.0, Some(0.9)),
                (0.0, 1.0, Some(-0.5)),
                (1.0, -2.0, Some(0.0)),
            ] {
                let mut eased = spring();
                eased.retarget(from, to, Motion::Full);
                let mut written = to;
                if let Some(next) = then {
                    eased.advance(0.02);
                    eased.retarget(written, next, Motion::Full);
                    written = next;
                }
                let side = eased.offset().signum();
                let mut gap = eased.offset().abs();
                // A second is ample: from a gap of 1.5 the tolerance is reached
                // at w t = 12.1, about 0.4 s.
                let steps = (1.0 / dt).ceil() as usize + 2;
                for _ in 0..steps {
                    eased.advance(dt);
                    let offset = eased.offset();
                    assert!(
                        offset == 0.0 || offset.signum() == side,
                        "dt {dt}: crossed the written value {written}"
                    );
                    assert!(
                        offset.abs() <= gap,
                        "dt {dt}: turned away from {written}: {} after {gap}",
                        offset.abs()
                    );
                    gap = offset.abs();
                    if eased.is_settled() {
                        break;
                    }
                }
                assert!(eased.is_settled(), "dt {dt}: never came to rest");
                assert_eq!(eased.present(written).to_bits(), written.to_bits());
            }
        }
    }

    #[test]
    fn a_reversing_hand_turns_the_value_without_swinging_past() {
        // Moving fast toward 1.0, the hand comes back to just behind the
        // presented value. The presented value must stop and come back, never
        // continuing on toward the abandoned target.
        let mut eased = spring();
        eased.retarget(0.0, 1.0, Motion::Full);
        eased.advance(0.03);
        let presented = eased.present(1.0);
        let behind = presented - 0.01;
        eased.retarget(1.0, behind, Motion::Full);
        let mut previous = eased.present(behind);
        for _ in 0..200 {
            eased.advance(1.0 / 120.0);
            let now = eased.present(behind);
            assert!(now <= previous + 1e-15, "{now} after {previous}");
            assert!(now >= behind, "{now} passed {behind}");
            previous = now;
        }
        assert!(eased.is_settled());
    }

    #[test]
    fn a_spring_snaps_to_rest_inside_its_tolerance_and_stays_there() {
        let mut eased = spring();
        eased.retarget(0.0, 1.0, Motion::Full);
        // From a unit step, (1 + w t) e^(-w t) falls below 1e-4 at w t = 11.6.
        let bound = 11.7 / OMEGA;
        let mut elapsed = 0.0;
        while !eased.is_settled() {
            eased.advance(1.0 / 60.0);
            elapsed += 1.0 / 60.0;
            assert!(elapsed < bound + 1.0 / 60.0, "still moving at {elapsed}s");
        }
        assert_eq!(eased.offset(), 0.0);
        eased.advance(5.0);
        assert!(eased.is_settled());
        assert_eq!(eased.present(1.0), 1.0);
    }

    #[test]
    fn reduced_motion_applies_every_write_at_once() {
        let mut direct = spring();
        direct.retarget(0.0, 1.0, Motion::Reduced);
        assert!(direct.is_settled());
        assert_eq!(direct.present(1.0), 1.0);
        // Turning motion down mid-flight also lands the value at once.
        let mut eased = spring();
        eased.retarget(0.0, 1.0, Motion::Full);
        eased.advance(0.01);
        assert!(!eased.is_settled());
        eased.retarget(1.0, 0.5, Motion::Reduced);
        assert!(eased.is_settled());
        assert_eq!(eased.present(0.5), 0.5);
    }

    #[test]
    fn hostile_input_can_only_make_a_spring_exact() {
        for omega in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut stiffless = Spring::new(omega, TOLERANCE);
            stiffless.retarget(0.0, 1.0, Motion::Full);
            assert!(stiffless.is_settled(), "omega {omega}");
        }
        for (from, to) in [(f64::NAN, 1.0), (0.0, f64::INFINITY), (f64::MAX, -f64::MAX)] {
            let mut eased = spring();
            eased.retarget(0.0, 1.0, Motion::Full);
            eased.retarget(from, to, Motion::Full);
            assert!(eased.is_settled(), "{from} to {to}");
        }
        let mut eased = spring();
        eased.retarget(0.0, 1.0, Motion::Full);
        let held = eased;
        for seconds in [f64::NAN, -1.0, 0.0, f64::NEG_INFINITY] {
            eased.advance(seconds);
            assert_eq!(eased, held, "{seconds} moved the spring");
        }
        eased.advance(f64::INFINITY);
        assert!(eased.is_settled());
        let mut untolerant = Spring::new(OMEGA, f64::NAN);
        untolerant.retarget(0.0, 1.0, Motion::Full);
        untolerant.advance(1e6);
        assert!(untolerant.is_settled());
    }
}
