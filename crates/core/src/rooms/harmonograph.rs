//! The harmonograph: the drawings a Victorian pendulum machine made.
//!
//! Two decaying oscillations on each axis trace a curve that spirals slowly
//! inward as the swings die away, like a Lissajous figure caught in amber. `t`
//! detunes the frequencies, opening and closing the weave. See `docs/ROOMS.md`.

use super::variation_signed;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Points along the traced curve.
const STEPS: usize = 4_000;
/// How far the pendulum's parameter runs (many swings).
const S_MAX: f64 = 60.0;
/// Damping: how fast the swings decay.
const DAMP: f64 = 0.012;

/// The harmonograph room.
#[derive(Debug, Default)]
pub struct Harmonograph {
    seed: u64,
}

impl Harmonograph {
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

    fn seed_detune(&self) -> f64 {
        variation_signed(self.seed, 0x4841_524D_4F4E_0001) * 0.035
    }

    fn detune_for(&self, t: f64) -> f64 {
        let phase = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (phase - 0.5) * 0.06 + self.seed_detune()
    }

    /// The machine's two real knobs from a hand point: x sets the detune
    /// (how open the weave blooms), y sets the damping (a slow ghost that
    /// swings for ages, or a rose that dies quickly). Both ranges are wider
    /// than the phase sweep's, so the hand reaches figures the sweep never
    /// visits.
    fn hand_params(&self, x: f64, y: f64) -> (f64, f64) {
        let detune = (x.clamp(0.0, 1.0) - 0.5) * 0.12 + self.seed_detune();
        let damp = 0.004 + y.clamp(0.0, 1.0) * 0.030;
        (detune, damp)
    }

    /// Draw one decaying trace with the given physics, sampled at `steps`
    /// segments. The live trace draws at full resolution; lingering ghosts
    /// draw coarser, because a full drag trail of full-resolution ghosts
    /// would blow the frame budget on large windows.
    fn draw_traced(
        &self,
        canvas: &mut dyn Surface,
        detune: f64,
        damp: f64,
        mark: char,
        steps: usize,
    ) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 || steps == 0 {
            return;
        }
        let aspect = canvas.safe_char_aspect();
        let center_x = width as f64 / 2.0;
        let center_y = height as f64 / 2.0;
        let radius = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.9;

        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let s = S_MAX * i as f64 / steps as f64;
            let (x, y) = point_damped(s, detune, damp);
            let sx = (center_x + x * radius) as i32;
            let sy = (center_y - y * radius * aspect) as i32;
            if let Some((px, py)) = previous {
                canvas.line(px, py, sx, sy, mark);
            }
            previous = Some((sx, sy));
        }
    }
}

/// The pen position at parameter `s`, with a small frequency `detune`. In
/// `[-1, 1]` on each axis. Production drawing goes through `point_damped`;
/// the physics tests keep this thin wrapper as their vocabulary.
#[cfg(test)]
fn point(s: f64, detune: f64) -> (f64, f64) {
    point_damped(s, detune, DAMP)
}

/// The pen position with the damping itself in hand: how fast the swings die.
fn point_damped(s: f64, detune: f64, damp: f64) -> (f64, f64) {
    let decay = (-damp * s).exp();
    let x = ((2.0 * s).sin() + ((3.0 + detune) * s + 1.0).sin()) * 0.5 * decay;
    let y = ((3.0 * s).sin() + ((2.0 - detune) * s + 0.7).sin()) * 0.5 * decay;
    (x, y)
}

impl Room for Harmonograph {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "harmonograph",
            title: "Harmonograph",
            wing: "Waves & Sound",
            blurb: "Two dying oscillations on each axis draw a curve that spirals inward as the \
                    pendulums lose energy. t detunes the frequencies to open and close the weave.",
            accent: [200, 80, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.draw_traced(canvas, self.detune_for(t), DAMP, '#', STEPS);
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: RETUNE THE PENDULUMS")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // The newest bounded raw tail first, finite filtering after, matching
        // the catalog input contract.
        let start = pokes.len().saturating_sub(crate::room::MAX_ROOM_POKES);
        let tuned: Vec<(f64, f64)> = pokes[start..]
            .iter()
            .copied()
            .filter(|&(x, y)| x.is_finite() && y.is_finite())
            .collect();
        let Some((&newest, older)) = tuned.split_last() else {
            self.render(canvas, t);
            return;
        };
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // The hand holds the machine's knobs: clicked physics replace the
        // sweep. Older tunings linger dim; the newest draws bright.
        for &(x, y) in older {
            let (detune, damp) = self.hand_params(x, y);
            self.draw_traced(canvas, detune, damp, '.', STEPS / 4);
        }
        let (detune, damp) = self.hand_params(newest.0, newest.1);
        self.draw_traced(canvas, detune, damp, '#', STEPS);
        for &(x, y) in &tuned {
            let px = (x.clamp(0.0, 1.0) * (width - 1) as f64).round() as i32;
            let py = (y.clamp(0.0, 1.0) * (height - 1) as f64).round() as i32;
            canvas.plot(px, py, '+');
        }
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(format!("DETUNE {:+.3}", self.detune_for(t)))
    }

    fn reveal(&self) -> &'static str {
        "Before computers, this is how people saw a chord. Two frequencies in a \
         simple ratio draw a clean closed figure; nudge them apart and it blooms \
         into a rose that never quite repeats."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "F fading rose",
            root: 174.61,
            tempo: 84,
            line: &[0, 4, 7, 11, 9, 7, 4, 0],
            encodes: "pendulum harmonics blooming outward then decaying home",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DAMP, Harmonograph, point};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_pen_stays_in_range() {
        for i in 0..=100 {
            let (x, y) = point(i as f64 * 0.6, 0.0);
            assert!(x.abs() <= 1.0 && y.abs() <= 1.0);
        }
    }

    #[test]
    fn the_swing_decays() {
        let (x_early, _) = point(1.0, 0.0);
        let envelope_late = (-DAMP * 50.0_f64).exp();
        assert!(envelope_late < 1.0);
        // Late motion is bounded by the (smaller) envelope.
        let (x_late, y_late) = point(50.0, 0.0);
        assert!(x_late.abs() <= envelope_late + 1e-9);
        assert!(y_late.abs() <= envelope_late + 1e-9);
        assert!(x_early.abs() <= 1.0);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Harmonograph::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.4);
        room.render(&mut b, 0.4);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Harmonograph::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_a_chord() {
        assert!(Harmonograph::new().reveal().contains("chord"));
    }

    #[test]
    fn a_click_holds_both_knobs() {
        let room = Harmonograph::new();
        // x reaches detunes beyond the sweep; y reaches slow and fast decay.
        let (detune_left, damp_slow) = room.hand_params(0.0, 0.0);
        let (detune_right, damp_fast) = room.hand_params(1.0, 1.0);
        assert!(detune_left < -0.03 && detune_right > 0.03);
        assert!(damp_slow < DAMP && damp_fast > DAMP);
        // Out-of-range input clamps.
        assert_eq!(room.hand_params(7.0, -2.0), room.hand_params(1.0, 0.0));
    }

    #[test]
    fn a_poke_changes_the_trace_and_marks_the_hand() {
        let room = Harmonograph::new();
        let mut bare = Canvas::new(48, 24);
        room.render(&mut bare, 0.4);
        let mut poked = Canvas::new(48, 24);
        room.render_poked(&mut poked, 0.4, &[(0.95, 0.9)]);
        assert_ne!(bare.to_text(), poked.to_text());
        assert_eq!(
            poked.cell(
                (0.95_f64 * 47.0).round() as usize,
                (0.9_f64 * 23.0).round() as usize
            ),
            Some('+')
        );
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = Harmonograph::new();
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.3)).collect();
        flood.push((f64::INFINITY, 0.5));
        flood.push((0.6, 0.2));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(48, 24);
        room.render_poked(&mut via_flood, 0.4, &flood);
        let mut via_tail = Canvas::new(48, 24);
        room.render_poked(&mut via_tail, 0.4, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room_and_older_tunings_linger() {
        let room = Harmonograph::new();
        let mut bare = Canvas::new(48, 24);
        room.render(&mut bare, 0.4);
        let mut invalid = Canvas::new(48, 24);
        room.render_poked(
            &mut invalid,
            0.4,
            &[(f64::NAN, 0.5), (0.5, f64::NEG_INFINITY)],
        );
        assert_eq!(bare.to_text(), invalid.to_text());
        let mut layered = Canvas::new(48, 24);
        room.render_poked(&mut layered, 0.4, &[(0.05, 0.95), (0.95, 0.05)]);
        let text = layered.to_text();
        assert!(text.contains('.'), "the older tuning lingers dim");
        assert!(text.contains('#'), "the newest tuning draws bright");
    }

    #[test]
    fn seed_variation_changes_poked_renders_and_seed_zero_stays_exact() {
        let mut a = Canvas::new(48, 24);
        Harmonograph::new().render_poked(&mut a, 0.4, &[(0.7, 0.6)]);
        let mut b = Canvas::new(48, 24);
        Harmonograph::new_with(9).render_poked(&mut b, 0.4, &[(0.7, 0.6)]);
        assert_ne!(a.to_text(), b.to_text());
        let mut exact = Canvas::new(48, 24);
        Harmonograph::new_with(0).render_poked(&mut exact, 0.4, &[(0.7, 0.6)]);
        assert_eq!(a.to_text(), exact.to_text());
    }

    #[test]
    fn hostile_surfaces_and_phase_stay_bounded() {
        struct Weird(Canvas);
        impl crate::surface::Surface for Weird {
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
        let room = Harmonograph::new();
        let mut weird = Weird(Canvas::new(30, 15));
        room.render_poked(&mut weird, f64::NAN, &[(0.5, 0.5)]);
        assert!(weird.0.ink_count() > 0);
        let mut nan_phase = Canvas::new(30, 15);
        room.render(&mut nan_phase, f64::NAN);
        let mut zero_phase = Canvas::new(30, 15);
        room.render(&mut zero_phase, 0.0);
        assert_eq!(nan_phase.to_text(), zero_phase.to_text());
        let status = room.status(f64::NAN).expect("status");
        assert!(status.starts_with("DETUNE") && status.len() < 24);
    }
}
