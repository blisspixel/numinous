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
/// Production drawing goes through `trace_from_state`; the physics tests
/// keep these thin wrappers as their vocabulary.
#[cfg(test)]
fn trace(offset: f64, steps: usize) -> Vec<(f64, f64)> {
    trace_from(2.0, offset, steps)
}

/// Integrate from any starting angle: the hand chooses the drop.
#[cfg(test)]
fn trace_from(start: f64, offset: f64, steps: usize) -> Vec<(f64, f64)> {
    trace_from_angles(start, start, offset, steps)
}

/// Integrate from two starting angles: the hand can re-drop both arms.
#[cfg(test)]
fn trace_from_angles(first: f64, second: f64, offset: f64, steps: usize) -> Vec<(f64, f64)> {
    trace_from_state(first, second, 0.0, 0.0, offset, steps)
}

/// Integrate from a full state: angles plus angular velocities, so a
/// released fling carries real momentum into the equations.
fn trace_from_state(
    first: f64,
    second: f64,
    w1: f64,
    w2: f64,
    offset: f64,
    steps: usize,
) -> Vec<(f64, f64)> {
    trace_and_state(first, second, w1, w2, offset, steps).0
}

fn trace_and_state(
    first: f64,
    second: f64,
    w1: f64,
    w2: f64,
    offset: f64,
    steps: usize,
) -> (Vec<(f64, f64)>, State) {
    let bounded_steps = steps.min(MAX_STEPS);
    let mut s = State {
        t1: first + offset,
        t2: second,
        w1,
        w2,
    };
    let mut path = Vec::with_capacity(bounded_steps / 3 + 1);
    for i in 0..bounded_steps {
        s = step(s);
        if i % 3 == 0 {
            let x = s.t1.sin() + s.t2.sin();
            let y = s.t1.cos() + s.t2.cos();
            path.push((x, y));
        }
    }
    (path, s)
}

fn arm_points(state: State) -> ((f64, f64), (f64, f64)) {
    let joint = (state.t1.sin(), state.t1.cos());
    let tip = (joint.0 + state.t2.sin(), joint.1 + state.t2.cos());
    (joint, tip)
}

/// The double pendulum room.
#[derive(Debug, Default)]
pub struct DoublePendulum {
    seed: u64,
}

impl DoublePendulum {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn seed_offset(&self) -> f64 {
        (self.seed % 1000) as f64 * 0.0001
    }

    fn steps_for(t: f64) -> usize {
        const ENTRY_STEPS: usize = 480;
        ENTRY_STEPS + (t.clamp(0.0, 1.0) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize
    }

    /// Draw the pendulum from a hand-chosen state: the shadow twin's path
    /// dim, the pendulum's path bright, and the arms at the final instant.
    /// Zero steps draws the held pose alone: pinned arms, no history.
    fn draw_hand_state(
        &self,
        canvas: &mut dyn Surface,
        first: f64,
        second: f64,
        w1: f64,
        w2: f64,
        steps: usize,
    ) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = canvas.safe_char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let radius = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.45;
        let to_screen = |px: f64, py: f64| -> (i32, i32) {
            (
                (cx + px * radius / 2.0) as i32,
                (cy + py * radius / 2.0 * aspect) as i32,
            )
        };
        for &(tx, ty) in &trace_from_state(first, second, w1, w2, SHADOW_OFFSET, steps) {
            let (px, py) = to_screen(tx, ty);
            canvas.plot(px, py, '-');
        }
        let (path, final_state) = trace_and_state(first, second, w1, w2, 0.0, steps);
        for &(tx, ty) in &path {
            let (px, py) = to_screen(tx, ty);
            canvas.plot(px, py, '*');
        }
        let pivot = to_screen(0.0, 0.0);
        let (joint, tip) = arm_points(final_state);
        let joint = to_screen(joint.0, joint.1);
        let tip = to_screen(tip.0, tip.1);
        canvas.line(pivot.0, pivot.1, joint.0, joint.1, '#');
        canvas.line(joint.0, joint.1, tip.0, tip.1, '#');
        let bob = (width.min(height) / 100).clamp(2, 5) as i32;
        for (x, y) in [joint, tip] {
            canvas.line(x - bob, y, x + bob, y, '#');
            canvas.line(x, y - bob, x, y + bob, '#');
        }
    }
}

fn finite_unit(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn hand_drop_angles(x: f64, y: f64, seed_offset: f64) -> (f64, f64) {
    let hand_x = finite_unit(x, 0.5);
    let hand_y = finite_unit(y, 0.5);
    let first = 0.6 + hand_x * 2.4 + seed_offset;
    let second = first + (hand_y - 0.5) * 1.2;
    (first, second)
}

/// The strongest fling either arm can be given, in radians per unit time.
const MAX_FLING: f64 = 6.0;

/// The smallest phase step a fling is measured over, so a release recorded
/// within one frame cannot divide by a near-zero time and explode.
const MIN_FLING_DT: f64 = 0.002;

/// The angular velocities a release carries: the angle change from the last
/// point the hand passed to the lift point, over the phase elapsed between
/// them, clamped. A slow lift is a gentle drop; a flick is a throw.
fn fling_velocities(before: (f64, f64, f64), at: (f64, f64, f64), seed_offset: f64) -> (f64, f64) {
    // Bad timestamps mean no measurable flick, never a maximal one.
    if !before.2.is_finite() || !at.2.is_finite() {
        return (0.0, 0.0);
    }
    let (a1_before, a2_before) = hand_drop_angles(before.0, before.1, seed_offset);
    let (a1_at, a2_at) = hand_drop_angles(at.0, at.1, seed_offset);
    // Phase wraps at 1.0, so a release sampled across the sweep boundary
    // (0.99 then 0.01) is 0.02 apart, not 0.98.
    let dt = elapsed_phase(at.2, before.2).max(MIN_FLING_DT);
    (
        ((a1_at - a1_before) / dt).clamp(-MAX_FLING, MAX_FLING),
        ((a2_at - a2_before) / dt).clamp(-MAX_FLING, MAX_FLING),
    )
}

/// Phase elapsed since `since`, on the wrapping [0, 1) clock.
fn elapsed_phase(now: f64, since: f64) -> f64 {
    if !now.is_finite() || !since.is_finite() {
        return 0.0;
    }
    (now - since).rem_euclid(1.0)
}

fn divergence_status(first: f64, second: f64, w1: f64, w2: f64, steps: usize) -> String {
    let main = trace_from_state(first, second, w1, w2, 0.0, steps);
    let shadow = trace_from_state(first, second, w1, w2, SHADOW_OFFSET, steps);
    let gap = match (main.last(), shadow.last()) {
        (Some(&(mx, my)), Some(&(sx, sy))) => ((mx - sx).powi(2) + (my - sy).powi(2)).sqrt(),
        _ => 0.0,
    };
    format!("TWINS {gap:.3} APART (from a {SHADOW_OFFSET:.0e} start)")
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
        // The standard drop is the hand-state drawing with the classic
        // starting angles: first arm seeded, second arm at the textbook 2.0,
        // no momentum. One drawing path serves every way into this room.
        let steps = Self::steps_for(t);
        self.draw_hand_state(canvas, 2.0 + self.seed_offset(), 2.0, 0.0, 0.0, steps);
    }

    fn status(&self, t: f64) -> Option<String> {
        // The divergence you can see peeling apart, made a number you can feel:
        // the distance between the bright pendulum and its shadow twin, which
        // began one SHADOW_OFFSET away. It sits near zero, then runs away as
        // sensitive dependence takes hold. Because it moves, it also lets this
        // room pose predictions and challenges, and predicting a chaotic gap is
        // exactly the hard, honest kind of guess the predict keystone is for.
        let steps = Self::steps_for(t);
        Some(divergence_status(
            2.0 + self.seed_offset(),
            2.0,
            0.0,
            0.0,
            steps,
        ))
    }

    fn status_input(&self, t: f64, inputs: &[crate::room::RoomInput]) -> Option<String> {
        let seed_offset = self.seed_offset();
        let (first, second, w1, w2, steps) = match crate::room::latest_gesture(inputs) {
            None => return self.status(t),
            Some(crate::room::Gesture::Held { at, .. }) => {
                let (first, second) = hand_drop_angles(at.0, at.1, seed_offset);
                (first, second, 0.0, 0.0, 0)
            }
            Some(crate::room::Gesture::Released { before, at }) => {
                let (first, second) = hand_drop_angles(at.0, at.1, seed_offset);
                let (w1, w2) = fling_velocities(before, at, seed_offset);
                let steps = (elapsed_phase(t, at.2) * MAX_STEPS as f64) as usize;
                (first, second, w1, w2, steps)
            }
            Some(crate::room::Gesture::Cancelled { at }) => {
                let (first, second) = hand_drop_angles(at.0, at.1, seed_offset);
                let steps = (elapsed_phase(t, at.2) * MAX_STEPS as f64) as usize;
                (first, second, 0.0, 0.0, steps)
            }
        };
        Some(divergence_status(first, second, w1, w2, steps))
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "two voices drifting",
            root: 123.47,
            tempo: 88,
            line: &[0, 7, 1, 8, 2, 9, 3, 10],
            encodes: "two coupled swings sliding out of phase: deterministic, unforecastable",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: RE-DROP")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let Some(&(x, y)) = pokes.last() else {
            self.render(canvas, t);
            return;
        };
        // The hand chooses both starting angles: x raises the first arm from
        // gentle swing to over-the-top drop, y bends the second arm above or
        // below it. Same equations, a storm placed by hand.
        let (first, second) = hand_drop_angles(x, y, self.seed_offset());
        let steps = (t.clamp(0.0, 1.0) * MAX_STEPS as f64) as usize;
        self.draw_hand_state(canvas, first, second, 0.0, 0.0, steps);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[crate::room::RoomInput]) {
        let seed_offset = self.seed_offset();
        match crate::room::latest_gesture(inputs) {
            None => self.render(canvas, t),
            // Held: the hand pins the bob. No motion until you let go.
            Some(crate::room::Gesture::Held { at, .. }) => {
                let (first, second) = hand_drop_angles(at.0, at.1, seed_offset);
                self.draw_hand_state(canvas, first, second, 0.0, 0.0, 0);
            }
            // Released: the lift point sets the angles, the flick sets the
            // momentum, and the equations take it from there.
            Some(crate::room::Gesture::Released { before, at }) => {
                let (first, second) = hand_drop_angles(at.0, at.1, seed_offset);
                let (w1, w2) = fling_velocities(before, at, seed_offset);
                let steps = (elapsed_phase(t, at.2) * MAX_STEPS as f64) as usize;
                self.draw_hand_state(canvas, first, second, w1, w2, steps);
            }
            // Cancelled: let go where you were, gently. No fling.
            Some(crate::room::Gesture::Cancelled { at }) => {
                let (first, second) = hand_drop_angles(at.0, at.1, seed_offset);
                let steps = (elapsed_phase(t, at.2) * MAX_STEPS as f64) as usize;
                self.draw_hand_state(canvas, first, second, 0.0, 0.0, steps);
            }
        }
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
    use super::{
        DoublePendulum, MAX_STEPS, State, arm_points, hand_drop_angles, trace, trace_from_angles,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn the_tip_stays_within_reach() {
        // Two unit arms: the tip can never be farther than 2 from the pivot.
        for &(x, y) in &trace(0.0, 6_000) {
            assert!(x.hypot(y) <= 2.0 + 1e-6, "escaped: ({x}, {y})");
        }
    }

    #[test]
    fn arm_geometry_preserves_two_unit_links() {
        let state = State {
            t1: 1.2,
            t2: 2.1,
            w1: 0.0,
            w2: 0.0,
        };
        let (joint, tip) = arm_points(state);
        assert!((joint.0.hypot(joint.1) - 1.0).abs() < 1e-12);
        assert!(((tip.0 - joint.0).hypot(tip.1 - joint.1) - 1.0).abs() < 1e-12);
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
    fn the_divergence_readout_climbs_and_poses_a_prediction() {
        let room = DoublePendulum::new();
        // The twins start together and run apart: the readout is near zero at
        // the start and larger once sensitive dependence takes hold.
        let start = room.status(0.0).expect("has a readout");
        assert!(start.contains("TWINS 0.000"), "starts together: {start}");
        let value = |t: f64| crate::challenge::status_numbers(&room.status(t).unwrap())[0].1;
        assert!(value(0.6) > value(0.0), "the gap grows across the sweep");
        // Because the readout moves, the room now poses a prediction: a hard,
        // chaotic guess, exactly what predict-then-reveal is for.
        assert!(crate::pose_prediction(&room, 3).is_some());
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
        assert!(
            zero.ink_count() > 15,
            "both linked arms and bobs show at the start"
        );

        let mut redropped = Canvas::new(50, 30);
        room.render_poked(&mut redropped, 0.0, &[(0.22, 0.28)]);
        let changed = zero
            .to_text()
            .chars()
            .zip(redropped.to_text().chars())
            .filter(|(left, right)| left != right)
            .count();
        assert!(
            changed >= 12,
            "the first re-drop visibly moves the linked arms"
        );
    }

    #[test]
    fn poked_render_is_deterministic() {
        let room = DoublePendulum::new_with(42);
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render_poked(&mut a, 0.5, &[(0.4, 0.6)]);
        room.render_poked(&mut b, 0.5, &[(0.4, 0.6)]);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn verb_matches_cross_face_click_semantics() {
        assert_eq!(DoublePendulum::new().verb(), Some("CLICK: RE-DROP"));
    }

    #[test]
    fn new_with_variation_affects_motion() {
        let r0 = DoublePendulum::new_with(0);
        let r42 = DoublePendulum::new_with(42);
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        r0.render(&mut a, 0.75);
        r42.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn the_hand_chooses_the_drop_and_the_reach_still_holds() {
        use crate::canvas::Canvas;
        use crate::room::Room;
        assert!(trace_from_angles(1.0, 1.0, 0.0, usize::MAX).len() <= MAX_STEPS / 3 + 1);
        for &(x, _) in &[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
            let (first, second) = hand_drop_angles(x, 0.5, 0.0);
            for &(px, py) in &trace_from_angles(first, second, 0.0, 3000) {
                assert!(px.hypot(py) <= 2.0 + 1e-6, "still two unit arms");
            }
        }
        let room = DoublePendulum::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render_poked(&mut a, 0.5, &[(0.2, 0.5)]);
        room.render_poked(&mut b, 0.5, &[(0.9, 0.5)]);
        assert_ne!(
            a.to_text(),
            b.to_text(),
            "different drops, different storms"
        );
    }

    #[test]
    fn the_hand_bends_the_second_arm() {
        let (upper_first, upper_second) = hand_drop_angles(0.5, 0.0, 0.0);
        let (lower_first, lower_second) = hand_drop_angles(0.5, 1.0, 0.0);
        assert_eq!(upper_first, lower_first);
        assert!(upper_second < upper_first);
        assert!(lower_second > lower_first);

        let room = DoublePendulum::new();
        let mut upper = Canvas::new(50, 30);
        let mut lower = Canvas::new(50, 30);
        room.render_poked(&mut upper, 0.5, &[(0.5, 0.0)]);
        room.render_poked(&mut lower, 0.5, &[(0.5, 1.0)]);
        assert_ne!(
            upper.to_text(),
            lower.to_text(),
            "vertical hand movement should change the re-drop"
        );
    }

    #[test]
    fn variation_participates_in_poked_motion() {
        let r0 = DoublePendulum::new_with(0);
        let r42 = DoublePendulum::new_with(42);
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        r0.render_poked(&mut a, 0.5, &[(0.4, 0.6)]);
        r42.render_poked(&mut b, 0.5, &[(0.4, 0.6)]);
        assert_ne!(
            a.to_text(),
            b.to_text(),
            "per-visit variation should reach poked re-drops"
        );
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = DoublePendulum::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
        }
    }

    #[test]
    fn reveal_separates_determinism_from_prediction() {
        assert!(DoublePendulum::new().reveal().contains("predictability"));
    }

    #[test]
    fn a_held_bob_is_pinned_and_does_not_run() {
        let room = DoublePendulum::new();
        let held = [
            RoomInput::PointerDown {
                x: 0.3,
                y: 0.4,
                t: 0.10,
            },
            RoomInput::PointerMove {
                x: 0.6,
                y: 0.5,
                t: 0.15,
            },
        ];
        let mut early = crate::canvas::Canvas::new(60, 30);
        room.render_input(&mut early, 0.2, &held);
        let mut late = crate::canvas::Canvas::new(60, 30);
        room.render_input(&mut late, 0.9, &held);
        assert_eq!(
            early.to_text(),
            late.to_text(),
            "time must not move a pinned pendulum"
        );
        assert_eq!(
            room.status_input(0.2, &held),
            room.status_input(0.9, &held),
            "the readout must describe the same pinned state as the frame"
        );
        let mut bare = crate::canvas::Canvas::new(60, 30);
        room.render(&mut bare, 0.2);
        assert_ne!(early.to_text(), bare.to_text(), "the held pose is visible");
    }

    #[test]
    fn a_flick_throws_harder_than_a_gentle_lift() {
        let room = DoublePendulum::new();
        // Same release point and elapsed time; only the approach speed
        // differs. The paths must differ: momentum is real.
        let slow = [
            RoomInput::PointerMove {
                x: 0.58,
                y: 0.5,
                t: 0.05,
            },
            RoomInput::PointerUp {
                x: 0.6,
                y: 0.5,
                t: 0.15,
            },
        ];
        let fast = [
            RoomInput::PointerMove {
                x: 0.30,
                y: 0.5,
                t: 0.147,
            },
            RoomInput::PointerUp {
                x: 0.6,
                y: 0.5,
                t: 0.15,
            },
        ];
        let mut slow_frame = crate::canvas::Canvas::new(60, 30);
        room.render_input(&mut slow_frame, 0.35, &slow);
        let mut fast_frame = crate::canvas::Canvas::new(60, 30);
        room.render_input(&mut fast_frame, 0.35, &fast);
        assert_ne!(slow_frame.to_text(), fast_frame.to_text());
    }

    #[test]
    fn fling_velocities_are_clamped_and_safe() {
        let (w1, w2) = super::fling_velocities((0.0, 0.5, 0.100), (1.0, 0.5, 0.1001), 0.0);
        assert!(w1.abs() <= super::MAX_FLING && w2.abs() <= super::MAX_FLING);
        let (w1, w2) =
            super::fling_velocities((f64::NAN, 0.5, 0.1), (0.6, f64::INFINITY, 0.2), 0.0);
        assert!(w1.is_finite() && w2.is_finite());
        // A bad timestamp means no flick, never a maximal one.
        assert_eq!(
            super::fling_velocities((0.0, 0.5, f64::NAN), (1.0, 0.5, 0.1), 0.0),
            (0.0, 0.0)
        );
    }

    #[test]
    fn a_fling_across_the_sweep_boundary_stays_strong() {
        // Two samples straddling the phase wrap are 0.02 apart, not 0.98;
        // the flick must not be deadened by the wrap.
        let across = super::fling_velocities((0.3, 0.5, 0.99), (0.6, 0.5, 0.01), 0.0);
        let within = super::fling_velocities((0.3, 0.5, 0.49), (0.6, 0.5, 0.51), 0.0);
        assert_eq!(across, within);
    }

    #[test]
    fn a_hostile_surface_aspect_cannot_break_the_held_pose() {
        struct WeirdAspect(Canvas);
        impl crate::surface::Surface for WeirdAspect {
            fn width(&self) -> usize {
                self.0.width()
            }
            fn height(&self) -> usize {
                self.0.height()
            }
            fn char_aspect(&self) -> f64 {
                f64::NAN
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                self.0.plot(x, y, mark);
            }
        }
        let room = DoublePendulum::new();
        let mut weird = WeirdAspect(Canvas::new(40, 20));
        room.render_input(
            &mut weird,
            0.5,
            &[RoomInput::PointerDown {
                x: 0.5,
                y: 0.5,
                t: 0.5,
            }],
        );
        assert!(weird.0.ink_count() > 0, "the pose still draws");
    }

    #[test]
    fn a_release_runs_forward_and_wraps_the_phase_clock() {
        let room = DoublePendulum::new();
        let released = [
            RoomInput::PointerDown {
                x: 0.4,
                y: 0.5,
                t: 0.90,
            },
            RoomInput::PointerUp {
                x: 0.4,
                y: 0.5,
                t: 0.95,
            },
        ];
        // Phase wrapped past 1.0: elapsed must read 0.15, not go negative.
        let mut wrapped = crate::canvas::Canvas::new(60, 30);
        room.render_input(&mut wrapped, 0.10, &released);
        let mut held_only = crate::canvas::Canvas::new(60, 30);
        room.render_input(
            &mut held_only,
            0.10,
            &[RoomInput::PointerDown {
                x: 0.4,
                y: 0.5,
                t: 0.90,
            }],
        );
        assert_ne!(
            wrapped.to_text(),
            held_only.to_text(),
            "a released pendulum has run; a held one has not"
        );
        assert!((super::elapsed_phase(0.10, 0.95) - 0.15).abs() < 1e-12);
        assert_eq!(super::elapsed_phase(f64::NAN, 0.5), 0.0);
    }

    #[test]
    fn a_cancel_drops_gently_with_no_fling() {
        let room = DoublePendulum::new();
        let cancelled = [
            RoomInput::PointerDown {
                x: 0.35,
                y: 0.45,
                t: 0.10,
            },
            RoomInput::PointerCancel,
        ];
        let mut via_cancel = crate::canvas::Canvas::new(60, 30);
        room.render_input(&mut via_cancel, 0.30, &cancelled);
        // A gentle drop is exactly a zero-velocity run from the same point,
        // with the run clock starting when the hand vanished.
        let mut via_click = crate::canvas::Canvas::new(60, 30);
        let (first, second) = super::hand_drop_angles(0.35, 0.45, 0.0);
        room.draw_hand_state(
            &mut via_click,
            first,
            second,
            0.0,
            0.0,
            (super::elapsed_phase(0.30, 0.10) * MAX_STEPS as f64) as usize,
        );
        assert_eq!(via_cancel.to_text(), via_click.to_text());
    }
}
