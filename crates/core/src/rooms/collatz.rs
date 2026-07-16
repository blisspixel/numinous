//! Collatz: a five-year-old's rule no one can crack.
//!
//! Pick a number. If it is even, halve it; if it is odd, triple it and add one.
//! Repeat. Every number ever tested falls to 1, yet no one can prove they all
//! do. This room plots the (log-scaled) trajectory of a starting number as it
//! falls. `t` picks the number. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::sound::{Note, SoundSpec};
use crate::surface::{MAX_DIM, Surface};

/// The starting number at `t = 0` (27 is famous for its long, wild orbit).
const START_MIN: u64 = 27;
/// How far `t` sweeps the starting number.
const START_SWEEP: u64 = 100;
/// Safety cap on orbit length, so the loop is always bounded even if a value
/// saturates (Collatz is unproven, so we never assume termination).
const MAX_STEPS: usize = 1000;

/// The Collatz room.
#[derive(Debug, Default)]
pub struct Collatz {
    seed: u64,
}

impl Collatz {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// The starting number for phase `t`.
    fn start_for(t: f64, seed: u64) -> u64 {
        let base = START_MIN + (phase_for(t) * START_SWEEP as f64).round() as u64;
        if seed == 0 {
            base
        } else {
            base + 1 + (seed % 11) // small det jitter for variation, 0 exact
        }
    }
}

impl Room for Collatz {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "collatz",
            title: "Collatz",
            wing: "Emergence",
            blurb: "Halve it if even, triple it and add one if odd, and repeat. Every tested \
                    start reaches 1, but nobody has proved that all do. t picks the number.",
            accent: [220, 130, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let orbit = collatz_orbit(Self::start_for(t, self.seed));
        draw_orbit(canvas, &orbit, '*');
    }

    fn reveal(&self) -> &'static str {
        "Every number ever tested falls to 1. Nobody on Earth can prove they all \
         do. It looks like a rule a child could follow, and it has defeated every \
         mathematician for 90 years. You are playing with an open mystery."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Every starting number up to two to the sixty-eighth power has been \
             checked by computer. All of them reach 1. That is overwhelming evidence \
             and it proves nothing, which is the whole lesson.",
            "Erdos said mathematics is not yet ripe for such questions and offered \
             five hundred dollars for a proof. Terence Tao proved in 2019 that almost \
             all orbits get almost as low as you like, which is agonizingly close and \
             still not it.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "E minor staircase",
            root: 164.81,
            tempo: 108,
            line: &[12, 7, 10, 5, 8, 3, 6, 0],
            encodes: "odd-number leaps followed by halving falls",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PERTURB THE START")
    }

    fn status(&self, t: f64) -> Option<String> {
        let start = Self::start_for(t, self.seed);
        Some(format!(
            "START {start}   3N+1 ORBIT   CLICK: PERTURB THE START"
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let finite: Vec<_> = pokes
            .into_iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .collect();
        if finite.is_empty() {
            return self.status(t);
        }
        let base_start = Self::start_for(t, self.seed);
        let starts = poked_starts(&finite, base_start);
        let latest = *starts.last().unwrap_or(&base_start);
        let steps = collatz_orbit(latest).len().saturating_sub(1);
        Some(format!(
            "{} ORBIT(S)   LATEST START {latest}   {steps} STEPS TO 1",
            starts.len()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let base_start = Self::start_for(t, self.seed);
        let starts = poked_starts(pokes, base_start);
        let orbit = collatz_orbit(base_start);
        draw_orbit(canvas, &orbit, '*');
        for start in starts {
            let orbit = collatz_orbit(start);
            draw_orbit(canvas, &orbit, '+');
        }
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // Play the orbit: each value a note, pitched by its log2, stepping in time.
        let orbit = collatz_orbit(Self::start_for(t, self.seed));
        let max = orbit.iter().copied().max().unwrap_or(1);
        let log_max = (max as f32).log2().max(1.0);
        let step = 0.09_f32;
        let notes: Vec<Note> = orbit
            .iter()
            .enumerate()
            .map(|(i, &value)| Note {
                freq: 220.0 * 2.0_f32.powf((value as f32).log2() / log_max),
                start: i as f32 * step,
                dur: step * 1.6,
                amp: 0.25,
            })
            .collect();
        let duration = orbit.len() as f32 * step + 0.3;
        SoundSpec { duration, notes }
    }
}

/// The Collatz sequence from `start` down to 1 (bounded by `MAX_STEPS`).
///
/// Uses saturating arithmetic so an extreme start cannot overflow; the safety
/// cap guarantees termination regardless.
fn collatz_orbit(start: u64) -> Vec<u64> {
    let mut n = start.max(1);
    let mut sequence = vec![n];
    while n != 1 && sequence.len() <= MAX_STEPS {
        n = if n % 2 == 0 {
            n / 2
        } else {
            n.saturating_mul(3).saturating_add(1)
        };
        sequence.push(n);
    }
    sequence
}

fn phase_for(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn poked_starts(pokes: &[(f64, f64)], base_start: u64) -> Vec<u64> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(px, py)| start_from_poke(base_start, px, py))
        .collect()
}

fn start_from_poke(base_start: u64, px: f64, py: f64) -> Option<u64> {
    if !px.is_finite() || !py.is_finite() {
        return None;
    }
    let horizontal = (px.clamp(0.0, 1.0) * START_SWEEP as f64).round() as u64 + 1;
    let vertical = ((1.0 - py.clamp(0.0, 1.0)) * 24.0).round() as u64 * 2 + 1;
    Some(
        base_start
            .saturating_add(horizontal)
            .saturating_add(vertical),
    )
}

fn draw_orbit(canvas: &mut dyn Surface, orbit: &[u64], ink: char) {
    if orbit.len() < 2 {
        return;
    }
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        return;
    }
    let max_x = width.saturating_sub(1).min(MAX_DIM.saturating_sub(1)) as f64;
    let max_y = height.saturating_sub(1).min(MAX_DIM.saturating_sub(1)) as f64;
    let max = orbit.iter().copied().max().unwrap_or(1);
    let log_max = (max as f64).log2().max(1e-9);
    let last = orbit.len() - 1;

    let to_pixel = |i: usize, value: u64| -> (i32, i32) {
        let x = (i as f64 / last as f64) * max_x;
        let normalized = (value as f64).log2() / log_max;
        let y = max_y - normalized * max_y;
        (x.round() as i32, y.round() as i32)
    };

    let mut prev = to_pixel(0, orbit[0]);
    for (i, &value) in orbit.iter().enumerate().skip(1) {
        let current = to_pixel(i, value);
        canvas.line(prev.0, prev.1, current.0, current.1, ink);
        prev = current;
    }
}

#[cfg(test)]
mod tests {
    use super::{Collatz, collatz_orbit, phase_for, poked_starts, start_from_poke};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room};
    use crate::surface::Surface;

    #[test]
    fn action_status_names_the_perturbed_orbit() {
        use crate::room::RoomInput;
        let room = Collatz::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("START 27"), "{open}");
        let inputs = [RoomInput::PointerDown {
            x: 0.8,
            y: 0.5,
            t: 0.0,
        }];
        let status = room.status_input(0.0, &inputs).expect("action");
        assert!(status.contains("ORBIT"), "{status}");
        assert!(status.contains("STEPS TO 1"), "{status}");
    }

    #[test]
    fn small_orbit_is_correct() {
        assert_eq!(collatz_orbit(6), vec![6, 3, 10, 5, 16, 8, 4, 2, 1]);
        assert_eq!(collatz_orbit(1), vec![1]);
    }

    #[test]
    fn famous_orbit_peaks_at_9232_and_reaches_one() {
        let orbit = collatz_orbit(27);
        assert_eq!(orbit.iter().copied().max(), Some(9232));
        assert_eq!(orbit.last(), Some(&1));
    }

    #[test]
    fn start_defaults_to_27() {
        assert_eq!(Collatz::start_for(0.0, 0), 27);
    }

    #[test]
    fn non_finite_phase_falls_back_to_first_start() {
        assert_eq!(phase_for(f64::NAN), 0.0);
        assert_eq!(
            Collatz::start_for(f64::INFINITY, 0),
            Collatz::start_for(0.0, 0)
        );
    }

    #[test]
    fn render_is_deterministic() {
        let room = Collatz::new();
        let mut a = Canvas::new(60, 20);
        let mut b = Canvas::new(60, 20);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = Collatz::new();
        let mut canvas = Canvas::new(60, 20);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = Collatz::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(5, 5);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
        room.render_poked(&mut canvas, f64::INFINITY, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn reveal_names_the_mystery() {
        assert!(Collatz::new().reveal().contains("prove they all"));
    }

    #[test]
    fn blurb_preserves_the_open_problem() {
        let blurb = Collatz::new().meta().blurb;
        assert!(blurb.contains("Every tested start"));
        assert!(blurb.contains("nobody has proved that all do"));
        assert!(!blurb.contains("every number always"));
    }

    #[test]
    fn sound_follows_the_orbit() {
        // The orbit of 27 is long, so its melody has many notes.
        let spec = Collatz::new().sound(0.0);
        assert!(spec.notes.len() > 5);
        assert!(spec.duration > 0.0);
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = Collatz::new_with(0);
        let r_def = Collatz::new();
        let mut a = Canvas::new(60, 20);
        let mut b = Canvas::new(60, 20);
        r0.render(&mut a, 0.0);
        r_def.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(60, 20);
        r0.render_poked(&mut cp, 0.0, &[(0.5, 0.5)]);
        assert_ne!(cp.to_text(), a.to_text());
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let r0 = Collatz::new_with(0);
        let r42 = Collatz::new_with(42);
        let mut a = Canvas::new(60, 20);
        let mut c = Canvas::new(60, 20);
        r0.render(&mut a, 0.0);
        r42.render(&mut c, 0.0);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn nonzero_seed_multiple_of_jitter_modulus_still_varies() {
        let r0 = Collatz::new_with(0);
        let r11 = Collatz::new_with(11);
        let mut a = Canvas::new(60, 20);
        let mut c = Canvas::new(60, 20);
        r0.render(&mut a, 0.0);
        r11.render(&mut c, 0.0);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn poke_coordinates_choose_actual_start_values() {
        let base = Collatz::start_for(0.0, 0);
        let low_x = start_from_poke(base, 0.0, 0.5).unwrap();
        let high_x = start_from_poke(base, 1.0, 0.5).unwrap();
        let high_y = start_from_poke(base, 0.5, 0.0).unwrap();
        let low_y = start_from_poke(base, 0.5, 1.0).unwrap();

        assert!(high_x > low_x);
        assert!(high_y > low_y);
    }

    #[test]
    fn poked_starts_caps_newest_raw_tail_before_filtering() {
        let base = Collatz::start_for(0.0, 0);
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        many.extend(
            (0..MAX_ROOM_POKES).map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.5)),
        );
        let newest = many[many.len() - MAX_ROOM_POKES..].to_vec();

        assert_eq!(poked_starts(&many, base), poked_starts(&newest, base));
        assert_eq!(poked_starts(&many, base).len(), MAX_ROOM_POKES);
    }

    #[test]
    fn non_finite_points_do_not_consume_start_identity() {
        let base = Collatz::start_for(0.0, 0);
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            poked_starts(&with_bad_points, base),
            poked_starts(&finite, base)
        );
    }

    #[test]
    fn invalid_tail_is_capped_before_filtering() {
        let base = Collatz::start_for(0.0, 0);
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(poked_starts(&with_invalid_tail, base).is_empty());
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = Collatz::new();
        let newest = vec![(0.7, 0.3); MAX_ROOM_POKES];
        let mut all = vec![(0.1, 0.9); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();
        let mut expected = Canvas::new(64, 22);
        let mut actual = Canvas::new(64, 22);
        let mut prefix_only = Canvas::new(64, 22);

        room.render_poked(&mut expected, 0.25, &newest);
        room.render_poked(&mut actual, 0.25, &all);
        room.render_poked(&mut prefix_only, 0.25, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(actual.to_text(), prefix_only.to_text());
    }

    #[test]
    fn all_invalid_pokes_match_base_render() {
        let room = Collatz::new();
        let mut base = Canvas::new(64, 22);
        let mut poked = Canvas::new(64, 22);

        room.render(&mut base, f64::NAN);
        room.render_poked(
            &mut poked,
            f64::NAN,
            &[(f64::NAN, f64::INFINITY), (f64::INFINITY, 0.5)],
        );

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn nonzero_seed_changes_poked_render() {
        let r0 = Collatz::new_with(0);
        let r42 = Collatz::new_with(42);
        let mut a = Canvas::new(64, 22);
        let mut b = Canvas::new(64, 22);

        r0.render_poked(&mut a, 0.2, &[(0.5, 0.5)]);
        r42.render_poked(&mut b, 0.2, &[(0.5, 0.5)]);

        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn huge_custom_surface_does_not_overflow_coordinates() {
        struct HugeSurface;

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                usize::MAX
            }

            fn height(&self) -> usize {
                usize::MAX
            }

            fn plot(&mut self, _x: i32, _y: i32, _ch: char) {}
        }

        let room = Collatz::new();
        let mut surface = HugeSurface;
        room.render_poked(&mut surface, f64::NAN, &[(1.0, 1.0)]);
    }
}
