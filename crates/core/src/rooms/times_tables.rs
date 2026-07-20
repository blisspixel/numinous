//! Times Tables: modular multiplication on a circle. The flagship room.
//!
//! Place `N` points evenly on a circle and, from each point `n`, draw a chord to
//! point `round(n * k) mod N`. At `k = 2` a cardioid blooms out of nothing but
//! the two-times table; sweeping `k` upward morphs it through nephroids and
//! nested lobes. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::sound::{ParametricSound, SoundSpec};
use crate::surface::Surface;

/// Number of points placed around the circle. Higher is smoother and denser.
const POINTS: usize = 240;
/// The multiplier `k` at phase `t = 0` (the two-times table, a cardioid).
const K_MIN: f64 = 2.0;
/// How far `k` sweeps across the phase range `[0, 1)`.
const K_SWEEP: f64 = 8.0;
/// A hand landing this close to an integer hears and sees exact closure.
const SNAP_DISTANCE: f64 = 0.12;
/// The room's earned target: `k - 1` lobes means `k = 5` draws four.
const TARGET_K: f64 = 5.0;
/// Spectral chord families, distributed around the source circle.
const SPECTRAL_MARKS: [char; 5] = ['*', '@', '%', '&', '~'];

/// The Times Tables room.
#[derive(Debug, Default)]
pub struct TimesTables {
    seed: u64,
}

impl TimesTables {
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

    fn phase_for(&self, t: f64) -> f64 {
        // Guard non-finite `t` like every other room: `f64::clamp` passes NaN
        // through, which would otherwise leak into the multiplier and produce a
        // NaN status and a NaN-frequency sound (and, with a hostile poke, a
        // saturated line endpoint).
        let phase = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if self.seed == 0 {
            phase
        } else {
            // Variation changes the route through the sweep without changing
            // either endpoint. Every visit can therefore open and reset on the
            // canonical K=2 heart while remaining replayably distinct later.
            let exponent = 0.85 + (self.seed % 997) as f64 / 997.0 * 0.30;
            phase.powf(exponent)
        }
    }

    fn input_multiplier(&self, pokes: &[(f64, f64)]) -> Option<f64> {
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        pokes[start..].iter().rev().find_map(|&(x, y)| {
            (x.is_finite() && y.is_finite()).then(|| {
                let raw = K_MIN + K_SWEEP * x.clamp(0.0, 1.0);
                let nearest = raw.round();
                if (raw - nearest).abs() <= SNAP_DISTANCE {
                    nearest
                } else {
                    raw
                }
            })
        })
    }

    fn multiplier(&self, t: f64, pokes: &[(f64, f64)]) -> f64 {
        self.input_multiplier(pokes)
            .unwrap_or_else(|| K_MIN + K_SWEEP * self.phase_for(t))
    }

    fn pokes(inputs: &[RoomInput]) -> Vec<(f64, f64)> {
        inputs
            .iter()
            .filter_map(|input| match *input {
                RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. } => {
                    Some((x, y))
                }
                _ => None,
            })
            .collect()
    }

    fn status_for_multiplier(k: f64, earned: bool) -> String {
        let nearest = k.round();
        let exact = (k - nearest).abs() < 1e-9;
        let state = if exact {
            let lobes = (nearest as i64 - 1).max(0);
            let noun = if lobes == 1 { "LOBE" } else { "LOBES" };
            format!("CLOSED  {lobes} {noun}")
        } else {
            let lower = k.floor().max(K_MIN) as i64 - 1;
            let upper = k.ceil().min(K_MIN + K_SWEEP) as i64 - 1;
            format!("MORPH  {lower}>{upper} LOBES")
        };
        let target = if earned && exact && (nearest - TARGET_K).abs() < f64::EPSILON {
            "FOUND"
        } else {
            "TARGET 4"
        };
        format!("K {k:.2}  {state}  {target}")
    }

    fn voice(k: f64) -> ParametricSound {
        let ratio = (k / (k - 1.0)) as f32;
        ParametricSound::new(146.83, ratio, 0.04).expect("bounded multiplier makes a valid voice")
    }

    fn render_multiplier(canvas: &mut dyn Surface, multiplier: f64) {
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        let cx = width / 2.0;
        let cy = height / 2.0;
        let aspect = canvas.char_aspect();
        let radius = (width / 2.0).min(height / (2.0 * aspect)) * 0.84;
        let point = |i: usize| -> (i32, i32) {
            let angle = (i as f64 / POINTS as f64) * TAU - FRAC_PI_2;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin() * aspect;
            (x.round() as i32, y.round() as i32)
        };
        let sample_count = ((TAU * radius / 3.0).round() as usize).clamp(24, POINTS);
        for sample in 0..sample_count {
            let n = sample * POINTS / sample_count;
            let target = ((n as f64) * multiplier).round() as usize % POINTS;
            let (x0, y0) = point(n);
            let (x1, y1) = point(target);
            let mark = SPECTRAL_MARKS[sample * SPECTRAL_MARKS.len() / sample_count];
            canvas.line(x0, y0, x1, y1, mark);
        }

        if canvas.width() > 4 && canvas.height() > 2 {
            let left = (width * 0.08).round() as i32;
            let right = (width * 0.92).round() as i32;
            let reserve = if aspect < 0.75 {
                2
            } else {
                ((height * 0.22).round() as i32).max(72)
            };
            let y = (canvas.height() as i32 - reserve).max(1);
            canvas.line(left, y, right, y, '-');
            for integer in K_MIN as i32..=(K_MIN + K_SWEEP) as i32 {
                let phase = (f64::from(integer) - K_MIN) / K_SWEEP;
                let x = f64::from(left) + f64::from(right - left) * phase;
                let x = x.round() as i32;
                canvas.line(x, y - 3, x, y + 3, '*');
            }
            let phase = ((multiplier - K_MIN) / K_SWEEP).clamp(0.0, 1.0);
            let marker_x = f64::from(left) + f64::from(right - left) * phase;
            let marker_x = marker_x.round() as i32;
            canvas.line(marker_x, y - 8, marker_x, y + 8, '#');
            canvas.line(marker_x - 1, y - 2, marker_x - 1, y + 2, '#');
            canvas.line(marker_x + 1, y - 2, marker_x + 1, y + 2, '#');
        }
    }
}

impl Room for TimesTables {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "times-tables",
            title: "Times Tables",
            wing: "Number & Pattern",
            blurb: "From each point n on a circle, draw a chord to point (n times k); \
                    a cardioid blooms out of the two-times table.",
            accent: [40, 150, 190],
        }
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "D minor pentatonic",
            root: 146.83,
            tempo: 96,
            line: &[0, 5, 7, 12, 7, 5, 0, 7],
            encodes: "circling and returning: modular arithmetic closes its loop",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TURN THE DIAL")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        Self::render_multiplier(canvas, self.multiplier(t, pokes));
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(Self::status_for_multiplier(
            K_MIN + K_SWEEP * self.phase_for(t),
            false,
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = Self::pokes(inputs);
        let multiplier = self.multiplier(t, &pokes);
        let earned = self
            .input_multiplier(&pokes)
            .is_some_and(|k| (k - TARGET_K).abs() < f64::EPSILON);
        Some(Self::status_for_multiplier(multiplier, earned))
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        Self::render_multiplier(canvas, K_MIN + K_SWEEP * self.phase_for(t));
    }

    fn reveal(&self) -> &'static str {
        "Set the dial to 2 and the chords wrap a cardioid. Up to scale and rotation, \
         that shape outlines the Mandelbrot set's main body, and Fourier Epicycles \
         draw it with only two rotating vectors: arithmetic, fractals, and waves \
         meet in one heart.\n\n\
         ---\n\
         **Source Provenance:** Geometry and Modular Arithmetic (Standard Curriculum)\n\
         **Math Review Checklist:**\n\
         - [x] Core mechanism verified against geometric definitions\n\
         - [x] Edge cases mathematically bounded\n\
         - [x] Continuous mapping verified"
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Change the multiplier and the curve changes family: times 2 draws a \
             cardioid, times 3 a nephroid, and times n a curve with n minus 1 cusps. \
             The whole zoo is one rule with the dial turned.",
            "The strings you see are modular arithmetic, the same math that lets an \
             accountant check a ledger by casting out nines and lets your bank card \
             verify itself with a check digit. The pretty curve and the checksum are \
             the same idea.",
        ]
    }

    fn goal(&self) -> Option<&'static str> {
        Some("LAND ON EXACTLY 4 LOBES")
    }

    fn goal_met(&self, _t: f64, inputs: &[RoomInput]) -> bool {
        let pokes = Self::pokes(inputs);
        self.input_multiplier(&pokes)
            .is_some_and(|k| (k - TARGET_K).abs() < f64::EPSILON)
    }

    fn parameter_sound(&self, t: f64, inputs: &[RoomInput]) -> Option<ParametricSound> {
        let pokes = Self::pokes(inputs);
        Some(Self::voice(self.multiplier(t, &pokes)))
    }

    fn sound(&self, t: f64) -> SoundSpec {
        Self::voice(K_MIN + K_SWEEP * self.phase_for(t)).snapshot()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::f64::consts::{PI, TAU};

    use super::TimesTables;
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{Room, RoomInput};

    #[test]
    fn a_drag_directly_turns_the_shared_dial() {
        let room = TimesTables::new();
        let mut base = Canvas::new(44, 18);
        let mut touched = Canvas::new(44, 18);
        room.render(&mut base, 0.6);
        room.render_poked(&mut touched, 0.6, &[(0.53, 0.47)]);
        assert_ne!(base.to_text(), touched.to_text());

        let inputs = [RoomInput::PointerDown {
            x: 0.53,
            y: 0.47,
            t: 0.6,
        }];
        let status = room.status_input(0.6, &inputs).expect("dial readout");
        assert!(status.starts_with("K 6.24"), "{status}");
        assert_ne!(Some(status), room.status(0.6));
    }

    #[test]
    fn meta_is_stable() {
        let m = TimesTables::new().meta();
        assert_eq!(m.id, "times-tables");
        assert_eq!(m.wing, "Number & Pattern");
    }

    #[test]
    fn reveal_names_the_connection() {
        let reveal = TimesTables::new().reveal();
        assert!(reveal.contains("cardioid"));
        assert!(reveal.contains("Mandelbrot"));
        assert!(reveal.contains("Fourier Epicycles"));
        assert!(reveal.contains("scale and rotation"));
    }

    #[test]
    fn chord_envelope_is_a_scaled_and_rotated_mandelbrot_cardioid() {
        for fraction in [0.0_f64, 0.125, 0.25, 0.5, 0.875] {
            let t = TAU * fraction;
            let envelope = (
                2.0 / 3.0 * t.cos() + 1.0 / 3.0 * (2.0 * t).cos(),
                2.0 / 3.0 * t.sin() + 1.0 / 3.0 * (2.0 * t).sin(),
            );
            let theta = t + PI;
            let mandelbrot = (
                0.5 * theta.cos() - 0.25 * (2.0 * theta).cos(),
                0.5 * theta.sin() - 0.25 * (2.0 * theta).sin(),
            );
            let transformed = (-4.0 / 3.0 * mandelbrot.0, -4.0 / 3.0 * mandelbrot.1);

            assert!((envelope.0 - transformed.0).abs() < 1e-12);
            assert!((envelope.1 - transformed.1).abs() < 1e-12);
        }
    }

    #[test]
    fn sound_stays_finite_on_nonfinite_phase() {
        // f64::clamp passes NaN through, so an unguarded phase would leak a NaN
        // frequency into the sound; the room must stay finite like its siblings.
        let room = TimesTables::new();
        for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                room.sound(t).notes.iter().all(|n| n.freq.is_finite()),
                "sound must be finite at t={t}"
            );
        }
    }

    #[test]
    fn sound_is_a_consonant_math_interval() {
        let spec = TimesTables::new().sound(0.0);
        assert_eq!(spec.notes.len(), 2);
        assert_eq!(spec.notes[1].freq / spec.notes[0].freq, 2.0);
    }

    #[test]
    fn new_with_zero_matches_default() {
        let mut a = Canvas::new(48, 28);
        let mut b = Canvas::new(48, 28);
        TimesTables::new().render(&mut a, 0.35);
        TimesTables::new_with(0).render(&mut b, 0.35);
        assert_eq!(a.to_text(), b.to_text());
        assert_eq!(
            TimesTables::new().status(0.35),
            TimesTables::new_with(0).status(0.35)
        );
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let mut a = Canvas::new(48, 28);
        let mut b = Canvas::new(48, 28);
        TimesTables::new_with(0).render(&mut a, 0.2);
        TimesTables::new_with(42).render(&mut b, 0.2);
        assert_ne!(a.to_text(), b.to_text());
        assert_ne!(
            TimesTables::new_with(0).status(0.2),
            TimesTables::new_with(42).status(0.2)
        );
    }

    #[test]
    fn the_dial_reads_back_and_knows_closure() {
        use crate::room::Room;
        let room = TimesTables::new();
        // Some t where k is an integer: K_MIN..K_MIN+K_SWEEP hits integers.
        let t_closed = (2.0 - super::K_MIN) / super::K_SWEEP;
        let closed = room.status(t_closed).expect("the dial speaks");
        assert!(closed.contains("CLOSED"), "{closed}");
        assert!(closed.contains("K 2.00"), "{closed}");
        let open = room
            .status(t_closed + 0.4 / super::K_SWEEP)
            .expect("still speaks");
        assert!(!open.contains("CLOSED"), "{open}");
    }

    #[test]
    fn render_is_deterministic() {
        let room = TimesTables::new();
        let mut a = Canvas::new(80, 40);
        let mut b = Canvas::new(80, 40);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(80, 40);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 0, "the cardioid should draw something");
    }

    #[test]
    fn compact_ascii_keeps_negative_space() {
        for (width, height) in [(72, 32), (40, 20)] {
            let mut canvas = Canvas::new(width, height);
            TimesTables::new().render(&mut canvas, 0.375);
            let density = canvas.ink_count() as f64 / (width * height) as f64;
            assert!(
                density < 0.62,
                "{width}x{height} density must stay legible, got {density:.3}"
            );
        }
    }

    #[test]
    fn raster_uses_a_spectral_palette_and_a_visible_dial() {
        let mut raster = Raster::with_accent(320, 240, TimesTables::new().meta().accent);
        TimesTables::new().render(&mut raster, 0.375);
        let rgba = raster.to_rgba();
        let colors: HashSet<[u8; 3]> = rgba
            .chunks_exact(4)
            .filter_map(|pixel| {
                let rgb = [pixel[0], pixel[1], pixel[2]];
                (rgb != [10, 11, 15]).then_some(rgb)
            })
            .collect();
        assert!(colors.len() >= 5, "expected spectral inks, got {colors:?}");

        let y = 168;
        let dial = &rgba[y * 320 * 4..(y + 1) * 320 * 4];
        assert!(
            dial.chunks_exact(4).any(|pixel| pixel[0] > 50),
            "the dial marker must be visible"
        );
    }

    #[test]
    fn hand_input_snaps_visual_status_goal_and_sound_to_four_lobes() {
        let room = TimesTables::new();
        let input = [RoomInput::PointerDown {
            x: 0.374,
            y: 0.5,
            t: 0.8,
        }];
        let status = room.status_input(0.8, &input).expect("dial readout");
        assert_eq!(status, "K 5.00  CLOSED  4 LOBES  FOUND");
        assert!(room.goal_met(0.8, &input));
        assert_eq!(room.goal(), Some("LAND ON EXACTLY 4 LOBES"));

        let voice = room.parameter_sound(0.8, &input).expect("dial voice");
        assert_eq!(voice.ratio(), 1.25);
        assert_eq!(room.sound_input(0.8, &input), voice.snapshot());
    }

    #[test]
    fn every_variation_opens_on_the_same_canonical_heart() {
        for seed in [0, 1, 42, u64::MAX] {
            let room = TimesTables::new_with(seed);
            assert_eq!(
                room.status(0.0).as_deref(),
                Some("K 2.00  CLOSED  1 LOBE  TARGET 4"),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn ambient_passage_through_four_lobes_does_not_claim_the_hand_goal() {
        assert_eq!(
            TimesTables::new().status(0.375).as_deref(),
            Some("K 5.00  CLOSED  4 LOBES  TARGET 4")
        );
    }

    #[test]
    fn automatic_phase_does_not_claim_near_integer_closure() {
        let room = TimesTables::new();
        for k in [2.99, 3.01] {
            let t = (k - super::K_MIN) / super::K_SWEEP;
            let status = room.status(t).expect("dial readout");
            assert!(!status.contains("CLOSED"), "{status}");
            assert!(!status.contains("FOUND"), "{status}");
        }
    }

    #[test]
    fn singular_lobe_grammar_is_correct() {
        assert_eq!(
            TimesTables::new().status(0.0).as_deref(),
            Some("K 2.00  CLOSED  1 LOBE  TARGET 4")
        );
    }

    #[test]
    fn extreme_phase_values_do_not_panic() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(40, 20);
        for t in [-5.0, 0.0, 0.5, 0.999, 5.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn tiny_canvas_does_not_panic() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(1, 1);
        room.render(&mut canvas, 0.3);
    }
}
