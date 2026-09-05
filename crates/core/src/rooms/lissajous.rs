//! Lissajous: two perpendicular oscillations tracing a curve.
//!
//! Unit-amplitude harmonic oscillators drive both axes. A rational frequency
//! ratio gives the ideal motion a common period; a finite trace or a return
//! to one position does not establish that period. Gallery phase `t` sweeps
//! the second frequency, or the relative phase after a hand tuning. Each frame
//! samples a separate oscillator-time window. See `docs/ROOMS.md`.

use std::f64::consts::{FRAC_PI_2, TAU};

use super::variation_unit;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, pokes_from_inputs};
use crate::sound::{ParametricSound, SoundSpec};
use crate::surface::Surface;

/// The fixed x-axis frequency; `t` sweeps the y-axis frequency against it.
const FREQ_X: f64 = 3.0;
/// The y-axis frequency at `t = 0` (a 2:3 ratio, a perfect fifth).
const FREQ_Y_MIN: f64 = 2.0;
/// How far `t` sweeps the y-axis frequency.
const FREQ_Y_SWEEP: f64 = 3.0;
/// Number of samples along the curve; consecutive samples are connected.
const SAMPLES: usize = 1500;
/// The largest whole number either oscillator can be tuned to by hand.
const MAX_TUNE: f64 = 8.0;
/// Convert oscillator frequency ratios to an audible register, in Hz.
const VOICE_BASE_HZ: f32 = 110.0;
/// Per-voice gain for the continuous mathematical interval.
const VOICE_GAIN: f32 = 0.06;

#[derive(Debug, Clone, Copy)]
struct OscillatorPair {
    frequency_x: f64,
    frequency_y: f64,
    phase_x: f64,
    phase_y: f64,
}

impl OscillatorPair {
    fn point(self, theta: f64) -> (f64, f64) {
        (
            (self.frequency_x * theta + FRAC_PI_2 + self.phase_x).sin(),
            (self.frequency_y * theta + self.phase_y).sin(),
        )
    }

    fn audio_parameters(self) -> (f32, f32) {
        (
            VOICE_BASE_HZ * self.frequency_x as f32,
            (self.frequency_y / self.frequency_x) as f32,
        )
    }

    fn voice(self) -> Option<ParametricSound> {
        let (root, ratio) = self.audio_parameters();
        // The audio interface carries frequencies, not the displayed phases.
        ParametricSound::new(root, ratio, VOICE_GAIN)
    }

    fn sound(self) -> SoundSpec {
        let (root, ratio) = self.audio_parameters();
        SoundSpec::chord(&[root, root * ratio], 1.5, VOICE_GAIN)
    }

    fn draw(self, canvas: &mut dyn Surface, mark: char) {
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let cx = width.saturating_sub(1) as f64 * 0.5;
        let cy = height.saturating_sub(1) as f64 * 0.5;
        let aspect = canvas.safe_char_aspect();
        // Both coordinates have unit amplitude in the same spatial units.
        // Text cells need aspect correction before those units reach pixels.
        // A common margin keeps turning points clear of the App room chrome.
        let radius_x = cx.min(cy / aspect) * 0.75;
        let radius_y = radius_x * aspect;
        let to_pixel = |theta| {
            let (x, y) = self.point(theta);
            (
                (cx + x * radius_x).round() as i32,
                (cy + y * radius_y).round() as i32,
            )
        };
        let mut previous = to_pixel(0.0);
        for index in 1..=SAMPLES {
            let theta = index as f64 / SAMPLES as f64 * TAU;
            let current = to_pixel(theta);
            canvas.line(previous.0, previous.1, current.0, current.1, mark);
            previous = current;
        }
    }
}

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn accepted_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

/// The Lissajous room.
#[derive(Debug, Default)]
pub struct Lissajous {
    seed: u64,
}

impl Lissajous {
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

    /// The y-axis frequency selected by phase `t`.
    fn freq_y_for(t: f64) -> f64 {
        FREQ_Y_MIN + FREQ_Y_SWEEP * phase_unit(t)
    }

    fn phase_offsets(&self) -> (f64, f64) {
        (
            variation_unit(self.seed, 0x4C49_5353_584A_0001) * TAU,
            variation_unit(self.seed, 0x4C49_5353_584A_0002) * TAU,
        )
    }

    /// The whole-number frequencies a hand point tunes: x picks the y-axis
    /// count, y picks the x-axis count, both 1 through 8 in equal-width bins.
    /// Every hand tuning therefore has a common oscillator period.
    fn tuned_freqs(x: f64, y: f64) -> (f64, f64) {
        let tuning = |value: f64| {
            1.0 + (value.clamp(0.0, 1.0) * MAX_TUNE)
                .floor()
                .min(MAX_TUNE - 1.0)
        };
        let fy = tuning(x);
        let fx = tuning(y);
        (fx, fy)
    }

    fn oscillators(&self, t: f64, hand: Option<(f64, f64)>) -> OscillatorPair {
        let (phase_x, phase_y) = self.phase_offsets();
        let (frequency_x, frequency_y, relative_phase) = match hand {
            Some((x, y)) => {
                let (fx, fy) = Self::tuned_freqs(x, y);
                // Zero and one select the same relative phase. This is a
                // control animation, not elapsed time along the trajectory.
                (fx, fy, phase_unit(t).fract() * TAU)
            }
            None => (FREQ_X, Self::freq_y_for(t), 0.0),
        };
        OscillatorPair {
            frequency_x,
            frequency_y,
            phase_x,
            phase_y: phase_y + relative_phase,
        }
    }

    fn selected_hand(inputs: &[RoomInput]) -> Option<(f64, f64)> {
        accepted_pokes(&pokes_from_inputs(inputs)).last().copied()
    }
}

impl Room for Lissajous {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.oscillators(t, None).draw(canvas, '*');
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: TUNE THE INTERVAL")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let tuned = accepted_pokes(pokes);
        let Some((&newest, older)) = tuned.split_last() else {
            self.render(canvas, t);
            return;
        };
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        // The hand tunes the instrument: clicked ratios replace the sweep.
        // Older intervals linger dim, the newest plays bright, and the
        // clicked cell is marked so the hand stays visible. Tunings quantize
        // to whole numbers, so a drag trail mostly repeats the same interval;
        // drawing each distinct older tuning once keeps a full trail inside
        // the frame budget without changing a single pixel.
        let (fx, fy) = Self::tuned_freqs(newest.0, newest.1);
        let mut drawn: Vec<(f64, f64)> = vec![(fx, fy)];
        for &(x, y) in older {
            let tuning = Self::tuned_freqs(x, y);
            if !drawn.contains(&tuning) {
                drawn.push(tuning);
                self.oscillators(t, Some((x, y))).draw(canvas, '.');
            }
        }
        self.oscillators(t, Some(newest)).draw(canvas, '*');
        for &(x, y) in &tuned {
            let px = (x.clamp(0.0, 1.0) * (width - 1) as f64).round() as i32;
            let py = (y.clamp(0.0, 1.0) * (height - 1) as f64).round() as i32;
            canvas.plot(px, py, '+');
        }
    }

    fn status(&self, t: f64) -> Option<String> {
        let pair = self.oscillators(t, None);
        Some(format!(
            "X:Y = {:.0}:{:.2}  CLICK:TUNE",
            pair.frequency_x, pair.frequency_y
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let Some(hand) = Self::selected_hand(inputs) else {
            return self.status(t);
        };
        let pair = self.oscillators(t, Some(hand));
        let (fx, fy) = (pair.frequency_x, pair.frequency_y);
        let a = fx.round() as i32;
        let b = fy.round() as i32;
        let g = {
            let mut xg = a.unsigned_abs();
            let mut yg = b.unsigned_abs();
            while yg != 0 {
                let t = yg;
                yg = xg % yg;
                xg = t;
            }
            xg.max(1)
        };
        let ra = a / g as i32;
        let rb = b / g as i32;
        let interval = match (ra.unsigned_abs(), rb.unsigned_abs()) {
            (u, v) if u == v => "UNISON",
            (1, 2) | (2, 1) => "OCTAVE",
            (2, 3) | (3, 2) => "FIFTH",
            (3, 4) | (4, 3) => "FOURTH",
            (3, 5) | (5, 3) => "SIXTH",
            (4, 5) | (5, 4) => "THIRD",
            _ => "RATIO",
        };
        Some(format!("TUNED {fx:.0}:{fy:.0}  {interval}  MOVING"))
    }

    fn reveal(&self) -> &'static str {
        // Periodic motion can revisit a position before its full state returns:
        // https://math.mit.edu/classes/18.353J/PSetAnswers/AnswerPSet_2024_07.pdf
        "A rational frequency ratio gives the ideal motion a common period. \
         A position can return sooner while moving another way. Small-integer \
         ratios can also sound consonant: the 2:3 ratio is a perfect fifth. \
         Sound follows the two frequencies; the picture also shows their \
         relative phase."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "G visible fifth",
            root: 196.0,
            tempo: 120,
            line: &[0, 7, 12, 7, 0, 5, 7, 12],
            encodes: "the two oscillator axes locking into a visible chord",
        })
    }

    fn sound(&self, t: f64) -> SoundSpec {
        self.oscillators(t, None).sound()
    }

    fn parameter_sound(&self, t: f64, inputs: &[RoomInput]) -> Option<ParametricSound> {
        self.oscillators(t, Self::selected_hand(inputs)).voice()
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use super::{Lissajous, MAX_ROOM_POKES, SAMPLES};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};
    use crate::surface::Surface;

    #[derive(Debug)]
    struct SampledSurface {
        width: usize,
        height: usize,
        aspect: f64,
        segments: Vec<(i32, i32, i32, i32)>,
        marks: Vec<(i32, i32)>,
    }

    impl SampledSurface {
        fn new(width: usize, height: usize, aspect: f64) -> Self {
            Self {
                width,
                height,
                aspect,
                segments: Vec::new(),
                marks: Vec::new(),
            }
        }

        fn points(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
            self.segments
                .first()
                .map(|&(x, y, _, _)| (x, y))
                .into_iter()
                .chain(self.segments.iter().map(|&(_, _, x, y)| (x, y)))
        }
    }

    impl Surface for SampledSurface {
        fn width(&self) -> usize {
            self.width
        }

        fn height(&self) -> usize {
            self.height
        }

        fn char_aspect(&self) -> f64 {
            self.aspect
        }

        fn plot(&mut self, x: i32, y: i32, _mark: char) {
            self.marks.push((x, y));
        }

        fn line(&mut self, x: i32, y: i32, next_x: i32, next_y: i32, _mark: char) {
            self.segments.push((x, y, next_x, next_y));
        }
    }

    fn axis_crossings(values: impl Iterator<Item = i32>, center: i32) -> usize {
        let mut previous_sign = 0;
        let mut crossings = 0;
        for value in values {
            let sign = (value - center).signum();
            if sign != 0 {
                if previous_sign != 0 && previous_sign != sign {
                    crossings += 1;
                }
                previous_sign = sign;
            }
        }
        crossings
    }

    #[test]
    fn freq_y_starts_at_two() {
        assert!((Lissajous::freq_y_for(0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn sampled_axes_are_unit_amplitude_harmonic_oscillators() {
        // A sampled sinusoid obeys q[n+1] + q[n-1] = 2*cos(w*h)*q[n].
        // Over an integer number of cycles its mean square is 1/2. Together
        // these detect both flattened extrema and changed amplitudes, without
        // copying the phase formula used to construct the path.
        let step = TAU / SAMPLES as f64;
        for seed in [0, 2, 7, u64::MAX] {
            let room = Lissajous::new_with(seed);
            let mut pairs: Vec<_> = (1..=8)
                .flat_map(|fx| (1..=8).map(move |fy| (fx, fy)))
                .map(|(fx, fy)| {
                    room.oscillators(
                        0.37,
                        Some(((fy as f64 - 0.5) / 8.0, (fx as f64 - 0.5) / 8.0)),
                    )
                })
                .collect();
            pairs.extend([0.0, 0.2, 0.5, 1.0].map(|t| room.oscillators(t, None)));
            for pair in pairs {
                let frequencies = [pair.frequency_x, pair.frequency_y];
                let mut mean = [0.0; 2];
                let mut mean_square = [0.0; 2];
                let mut previous = pair.point(-step);
                let mut current = pair.point(0.0);
                for sample in 0..SAMPLES {
                    let next = pair.point((sample + 1) as f64 / SAMPLES as f64 * TAU);
                    let values = [current.0, current.1];
                    let neighbors = [previous.0 + next.0, previous.1 + next.1];
                    for axis in 0..2 {
                        assert!(values[axis].is_finite() && (-1.0..=1.0).contains(&values[axis]));
                        let residual =
                            neighbors[axis] - 2.0 * (frequencies[axis] * step).cos() * values[axis];
                        assert!(
                            residual.abs() < 1e-12,
                            "seed={seed} axis={axis}: harmonic residual {residual}"
                        );
                        mean[axis] += values[axis] / SAMPLES as f64;
                        mean_square[axis] += values[axis].powi(2) / SAMPLES as f64;
                    }
                    previous = current;
                    current = next;
                }
                for axis in 0..2 {
                    if frequencies[axis].fract() == 0.0 {
                        assert!(
                            mean[axis].abs() < 1e-12,
                            "seed={seed} axis={axis}: mean {}",
                            mean[axis]
                        );
                        assert!(
                            (mean_square[axis] - 0.5).abs() < 1e-12,
                            "seed={seed} axis={axis}: mean square {}",
                            mean_square[axis]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rendered_circle_preserves_equal_axis_units_on_both_surface_types() {
        for aspect in [1.0, 0.5] {
            let mut surface = SampledSurface::new(181, 101, aspect);
            Lissajous::new().render_poked(&mut surface, 0.0, &[(0.0, 0.0)]);
            assert_eq!(surface.segments.len(), SAMPLES);
            let rx = surface.points().map(|(x, _)| (x - 90).abs()).max().unwrap() as f64;
            let ry = surface.points().map(|(_, y)| (y - 50).abs()).max().unwrap() as f64;
            assert!(
                (rx - ry / aspect).abs() <= 1.0,
                "aspect={aspect}: extents {rx}, {ry}"
            );
            // The radius error budget includes rounding to whole pixels/cells.
            for (x, y) in surface.points() {
                let x = (x as f64 - 90.0) / rx;
                let y = (y as f64 - 50.0) / ry;
                assert!((x * x + y * y - 1.0).abs() < 2.0 / rx + 2.0 / ry);
            }
        }
    }

    #[test]
    fn every_hand_tuning_reaches_the_drawn_axes_readout_and_sound() {
        let room = Lissajous::new_with(2);
        for fx in 1..=8 {
            for fy in 1..=8 {
                let hand = ((fy as f64 - 0.5) / 8.0, (fx as f64 - 0.5) / 8.0);
                let inputs = crate::room::inputs_from_pokes(&[hand], 0.2);
                let mut surface = SampledSurface::new(2049, 2049, 1.0);
                room.render_input(&mut surface, 0.2, &inputs);
                // Count the actual renderer's crossings, ignoring rounded
                // samples exactly on an axis. Each cycle crosses twice.
                assert_eq!(
                    axis_crossings(surface.points().map(|(x, _)| x), 1024),
                    2 * fx
                );
                assert_eq!(
                    axis_crossings(surface.points().map(|(_, y)| y), 1024),
                    2 * fy
                );
                let status = room.status_input(0.2, &inputs).unwrap();
                assert!(status.contains(&format!("TUNED {fx}:{fy}")), "{status}");
                let voice = room.parameter_sound(0.2, &inputs).unwrap();
                let sound = room.sound_input(0.2, &inputs);
                assert_eq!(sound, voice.snapshot());
                assert_eq!(sound.notes[0].freq, 110.0 * fx as f32);
                assert!((sound.notes[1].freq as f64 - 110.0 * fy as f64).abs() < 1e-4);
                assert!((voice.ratio() as f64 - fy as f64 / fx as f64).abs() < 3e-7);
            }
        }
    }

    #[test]
    fn continuous_sweep_sound_is_not_snapped_to_integer_frequencies() {
        for seed in [0, 2, 7, u64::MAX] {
            let room = Lissajous::new_with(seed);
            for t in [0.0, 0.1, 0.2, 0.5, 0.7, 1.0] {
                let sound = room.sound(t);
                assert_eq!(sound, room.sound_input(t, &[]));
                assert_eq!(sound.notes[0].freq, 330.0);
                let expected_y = 110.0 * (2.0 + 3.0 * t);
                assert!((sound.notes[1].freq as f64 - expected_y).abs() < 1e-4);
                assert!(
                    room.status(t)
                        .unwrap()
                        .contains(&format!("3:{:.2}", expected_y / 110.0))
                );
            }
            assert_ne!(room.sound(0.5), room.sound(0.51));
        }
    }

    #[test]
    fn a_position_return_can_precede_the_common_period() {
        // The swept 3:3.5 ratio is rational despite its noninteger frequency.
        // Its full state has period 4*pi. At 2*pi the starting position
        // returns, but y velocity reverses, so the motion has not repeated.
        let pair = Lissajous::new().oscillators(0.5, None);
        let start = pair.point(0.0);
        let halfway = pair.point(TAU);
        assert!((start.0 - halfway.0).hypot(start.1 - halfway.1) < 1e-12);
        let y_velocity = |theta| {
            let h = 1e-5;
            (pair.point(theta + h).1 - pair.point(theta - h).1) / (2.0 * h)
        };
        assert!((y_velocity(0.0) - 3.5).abs() < 1e-8);
        assert!((y_velocity(TAU) + 3.5).abs() < 1e-8);
        for theta in [0.0, 0.27, 1.13] {
            let p = pair.point(theta);
            let q = pair.point(theta + 2.0 * TAU);
            assert!((p.0 - q.0).hypot(p.1 - q.1) < 1e-12);
        }
    }

    #[test]
    fn render_is_deterministic() {
        let room = Lissajous::new();
        let mut a = Canvas::new(40, 24);
        let mut b = Canvas::new(40, 24);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = Lissajous::new();
        let mut canvas = Canvas::new(40, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = Lissajous::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(4, 4);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_interval() {
        let reveal = Lissajous::new().reveal();
        assert!(reveal.contains("rational frequency ratio gives the ideal motion a common period"));
        assert!(reveal.contains("Small-integer ratios"));
        assert!(reveal.contains("2:3 ratio is a perfect fifth"));
        assert!(reveal.contains("A position can return sooner"));
    }

    #[test]
    fn sound_is_a_two_note_chord() {
        let spec = Lissajous::new().sound(0.0);
        assert_eq!(spec.notes.len(), 2);
    }

    #[test]
    fn a_click_tunes_a_whole_number_interval() {
        // Corners and center map to exact whole-number oscillator counts.
        assert_eq!(Lissajous::tuned_freqs(0.0, 0.0), (1.0, 1.0));
        assert_eq!(Lissajous::tuned_freqs(1.0, 1.0), (8.0, 8.0));
        assert_eq!(Lissajous::tuned_freqs(0.5, 0.0), (1.0, 5.0));
        // Out-of-range input clamps instead of escaping the tuning range.
        assert_eq!(Lissajous::tuned_freqs(9.0, -3.0), (1.0, 8.0));
    }

    #[test]
    fn every_frequency_has_an_equal_width_selection_interval_on_both_axes() {
        for frequency in 1..=8 {
            let center = (frequency as f64 - 0.5) / 8.0;
            assert_eq!(
                Lissajous::tuned_freqs(center, center),
                (frequency as f64, frequency as f64)
            );
            if frequency < 8 {
                let boundary = frequency as f64 / 8.0;
                for (position, expected) in [
                    (boundary.next_down(), frequency as f64),
                    (boundary, (frequency + 1) as f64),
                    (boundary.next_up(), (frequency + 1) as f64),
                ] {
                    assert_eq!(Lissajous::tuned_freqs(position, 0.0), (1.0, expected));
                    assert_eq!(Lissajous::tuned_freqs(0.0, position), (expected, 1.0));
                }
            }
        }
    }

    #[test]
    fn interaction_status_reports_the_persistent_tuning() {
        let room = Lissajous::new();
        let inputs = crate::room::inputs_from_pokes(&[(0.72, 0.35)], 0.2);
        let early = room.status_input(0.2, &inputs).expect("tuned status");
        let late = room.status_input(0.8, &inputs).expect("tuned status");
        assert_eq!(early, late);
        assert!(early.contains("TUNED 3:6"), "{early}");
        assert!(
            early.contains("OCTAVE") || early.contains("RATIO"),
            "{early}"
        );
        assert!(early.contains("MOVING"), "{early}");
    }

    #[test]
    fn a_poke_changes_the_figure_and_marks_the_hand() {
        let room = Lissajous::new();
        let mut bare = Canvas::new(48, 24);
        room.render(&mut bare, 0.3);
        let mut poked = Canvas::new(48, 24);
        room.render_poked(&mut poked, 0.3, &[(0.9, 0.1)]);
        assert_ne!(bare.to_text(), poked.to_text(), "the tuned figure differs");
        // The clicked cell carries the hand marker.
        assert_eq!(poked.cell((0.9_f64 * 47.0).round() as usize, 2), Some('+'));
    }

    #[test]
    fn a_tuned_interval_keeps_moving_after_the_click() {
        let room = Lissajous::new();
        let click = [(0.83, 0.21)];
        let mut early = Canvas::new(64, 40);
        room.render_poked(&mut early, 0.2, &click);
        let mut late = Canvas::new(64, 40);
        // A half-turn of relative phase gives this 2:7 tuning the same
        // geometric curve. Use distinct shapes instead of rounding artifacts.
        room.render_poked(&mut late, 0.4, &click);

        assert_ne!(early.to_text(), late.to_text());
        let px = (click[0].0 * 63.0_f64).round() as usize;
        let py = (click[0].1 * 39.0_f64).round() as usize;
        assert_eq!(early.cell(px, py), Some('+'));
        assert_eq!(late.cell(px, py), Some('+'));
    }

    #[test]
    fn tuned_phase_control_loops_at_the_gallery_boundary() {
        let room = Lissajous::new();
        let click = [(5.5 / 8.0, 3.5 / 8.0)];
        let mut start = Canvas::new(64, 40);
        room.render_poked(&mut start, 0.0, &click);
        let mut end = Canvas::new(64, 40);
        room.render_poked(&mut end, 1.0, &click);

        assert_eq!(start.to_text(), end.to_text());
        assert_eq!(Lissajous::tuned_freqs(click[0].0, click[0].1), (4.0, 6.0));
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = Lissajous::new();
        // A flood of old points then bad newest entries: the raw tail is
        // capped first, so surviving finite points are honored while the
        // rest are ignored without panicking.
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.2)).collect();
        flood.push((f64::NAN, 0.5));
        flood.push((0.4, 0.6));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(48, 24);
        room.render_poked(&mut via_flood, 0.3, &flood);
        let mut via_tail = Canvas::new(48, 24);
        room.render_poked(&mut via_tail, 0.3, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room_and_older_intervals_linger() {
        let room = Lissajous::new();
        let mut bare = Canvas::new(48, 24);
        room.render(&mut bare, 0.3);
        let mut invalid = Canvas::new(48, 24);
        room.render_poked(&mut invalid, 0.3, &[(f64::NAN, 0.5), (0.5, f64::INFINITY)]);
        assert_eq!(bare.to_text(), invalid.to_text());
        // Two clicks: the older interval lingers dim beneath the newest.
        let mut layered = Canvas::new(48, 24);
        room.render_poked(&mut layered, 0.3, &[(0.1, 0.9), (0.9, 0.1)]);
        let text = layered.to_text();
        assert!(text.contains('.'), "the older interval lingers dim");
        assert!(text.contains('*'), "the newest interval plays bright");
    }

    #[test]
    fn seed_variation_changes_poked_renders_too() {
        let base = Lissajous::new();
        let varied = Lissajous::new_with(7);
        let mut a = Canvas::new(48, 24);
        base.render_poked(&mut a, 0.3, &[(0.7, 0.7)]);
        let mut b = Canvas::new(48, 24);
        varied.render_poked(&mut b, 0.3, &[(0.7, 0.7)]);
        assert_ne!(a.to_text(), b.to_text());
        let mut exact = Canvas::new(48, 24);
        Lissajous::new_with(0).render_poked(&mut exact, 0.3, &[(0.7, 0.7)]);
        assert_eq!(a.to_text(), exact.to_text(), "seed 0 stays the exact path");
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
                f64::NEG_INFINITY
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                self.0.plot(x, y, mark);
            }
        }
        let room = Lissajous::new();
        let mut weird = Weird(Canvas::new(30, 15));
        room.render_poked(&mut weird, f64::NAN, &[(0.5, 0.5)]);
        assert!(weird.0.ink_count() > 0);
        let mut nan_phase = Canvas::new(30, 15);
        room.render(&mut nan_phase, f64::NAN);
        let mut zero_phase = Canvas::new(30, 15);
        room.render(&mut zero_phase, 0.0);
        assert_eq!(nan_phase.to_text(), zero_phase.to_text());
    }

    #[test]
    fn all_projections_agree_on_hostile_phase_and_the_newest_raw_input_tail() {
        for seed in [0, 2, u64::MAX] {
            let room = Lissajous::new_with(seed);
            for phase in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -3.0] {
                assert_eq!(room.sound(phase), room.sound(0.0));
                assert_eq!(room.status(phase), room.status(0.0));
            }
            assert_eq!(room.sound(3.0), room.sound(1.0));
            assert_eq!(room.status(3.0), room.status(1.0));

            let mut inputs = vec![RoomInput::PointerDown {
                x: 1.0,
                y: 1.0,
                t: 0.0,
            }];
            inputs.extend(std::iter::repeat_n(
                RoomInput::PointerDown {
                    x: f64::NAN,
                    y: 0.5,
                    t: 0.0,
                },
                MAX_ROOM_POKES,
            ));
            assert_eq!(room.sound_input(0.2, &inputs), room.sound(0.2));
            assert_eq!(room.status_input(0.2, &inputs), room.status(0.2));
            let mut actual = Canvas::new(48, 24);
            let mut expected = Canvas::new(48, 24);
            room.render_input(&mut actual, 0.2, &inputs);
            room.render(&mut expected, 0.2);
            assert_eq!(actual.to_text(), expected.to_text());

            // A finite point beyond the control square clamps consistently.
            inputs.push(RoomInput::PointerMove {
                x: 9.0,
                y: -3.0,
                t: 0.2,
            });
            let sound = room.sound_input(f64::NAN, &inputs);
            assert_eq!((sound.notes[0].freq, sound.notes[1].freq), (110.0, 880.0));
            assert!(
                room.status_input(f64::NAN, &inputs)
                    .unwrap()
                    .contains("TUNED 1:8")
            );
            let voice = room.parameter_sound(f64::NAN, &inputs).unwrap();
            assert!(voice.gain() > 0.0 && voice.gain() <= crate::sound::ParametricSound::MAX_GAIN);
        }
    }

    #[test]
    fn surface_coordinates_and_segment_work_stay_bounded() {
        let room = Lissajous::new_with(2);
        let pokes: Vec<_> = (0..MAX_ROOM_POKES * 4)
            .map(|i| {
                (
                    ((i % 8) as f64 + 0.5) / 8.0,
                    ((i / 8 % 8) as f64 + 0.5) / 8.0,
                )
            })
            .collect();
        for (width, height) in [(usize::MAX, usize::MAX), (1, 1), (0, 8), (8, 0)] {
            for aspect in [
                0.5,
                1.0,
                f64::NAN,
                f64::NEG_INFINITY,
                f64::MAX,
                f64::from_bits(1),
            ] {
                let mut surface = SampledSurface::new(width, height, aspect);
                room.render_poked(&mut surface, f64::NAN, &pokes);
                assert!(surface.segments.len() <= MAX_ROOM_POKES * SAMPLES);
                assert!(surface.marks.len() <= MAX_ROOM_POKES);
                let (draw_width, draw_height) = surface.draw_bounds();
                for (x, y) in surface.points().chain(surface.marks.iter().copied()) {
                    assert!(
                        x >= 0 && (x as usize) < draw_width,
                        "width={width}, aspect={aspect}: x={x}"
                    );
                    assert!(
                        y >= 0 && (y as usize) < draw_height,
                        "height={height}, aspect={aspect}: y={y}"
                    );
                }
            }
        }
    }
}
