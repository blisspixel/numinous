//! Music Engine A: the chiptune. Square waves, triangle bass, seeded melodies
//! in real scales, the 8-bit voice of Numinous (see `docs/MUSIC.md`).
//!
//! Everything here is pure synthesis: patterns render to samples
//! deterministically, so tunes are testable and identical on every machine.
//! Playback is the faces' business; this module only makes the numbers.

use std::f32::consts::TAU;

use crate::rng::SplitMix64;

/// Decorrelates tune seeds from other seeded systems.
const TUNE_MIX: u64 = 0xC417_0000_7EA1_0001;
/// Attack/release applied to every step to avoid clicks, in seconds.
const STEP_FADE: f32 = 0.008;

/// A compact chip palette with a rounded anchor voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    /// A pure, rounded tone for pads and quiet anchors.
    Sine,
    /// The lead: a hollow square wave.
    Square,
    /// The bass: a soft triangle wave.
    Triangle,
    /// The percussion: seeded noise bursts.
    Noise,
}

impl Voice {
    /// Stable lowercase identifier for structured arrangement projections.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Sine => "sine",
            Self::Square => "square",
            Self::Triangle => "triangle",
            Self::Noise => "noise",
        }
    }
}

/// One step of a pattern: a note (frequency, voice, level) or a rest.
pub type Step = Option<(f32, Voice, f32)>;

/// A pattern: steps of equal duration, the chip's sheet music.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// The steps, in order.
    pub steps: Vec<Step>,
    /// Seconds per step.
    pub step_seconds: f32,
}

/// One note in a layered chip arrangement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipNote {
    /// Frequency in Hz.
    pub frequency: f32,
    /// First arrangement step occupied by the note.
    pub start_step: usize,
    /// Number of arrangement steps sustained by the note.
    pub step_count: usize,
    /// Oscillator used for the note.
    pub voice: Voice,
    /// Linear peak level before the arrangement mix.
    pub level: f32,
    /// Stereo position from hard left at `-1.0` to hard right at `1.0`.
    pub pan: f32,
}

/// A bounded polyphonic chip arrangement on one shared musical grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// Notes in deterministic rendering order.
    pub notes: Vec<ChipNote>,
    /// Total number of steps in the arrangement.
    pub steps: usize,
    /// Duration of one step in seconds.
    pub step_seconds: f32,
}

/// Bounded engineering features for one interleaved stereo buffer.
///
/// These measurements detect signal and stereo regressions. They do not
/// measure comfort, beauty, fatigue, or musical quality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoSignalMetrics {
    /// Complete stereo frames measured. An odd trailing sample is excluded.
    pub frame_count: usize,
    /// Samples that were not part of a complete stereo frame.
    pub trailing_samples: usize,
    /// Non-finite samples, treated as silence for the remaining measurements.
    pub non_finite_samples: usize,
    /// Finite subnormal samples.
    pub subnormal_samples: usize,
    /// Samples at or beyond full scale.
    pub clipped_samples: usize,
    /// Largest absolute sample.
    pub peak: f64,
    /// RMS across both channels.
    pub rms: f64,
    /// Crest factor in decibels.
    pub crest_db: f64,
    /// Left channel RMS.
    pub left_rms: f64,
    /// Right channel RMS.
    pub right_rms: f64,
    /// Right-to-left RMS balance in decibels.
    pub channel_balance_db: f64,
    /// Left channel arithmetic mean.
    pub left_dc: f64,
    /// Right channel arithmetic mean.
    pub right_dc: f64,
    /// Uncentered left-right correlation in `[-1, 1]`.
    pub correlation: f64,
    /// Side-to-mid RMS ratio in decibels, bounded to `[-120, 120]`.
    pub side_to_mid_db: f64,
    /// Largest adjacent step within either channel.
    pub max_step: f64,
    /// Fraction of complete-frame samples that are exactly zero.
    pub zero_sample_fraction: f64,
}

/// Convert one normalized sample to the PCM16 representation used by exports.
#[must_use]
pub fn quantize_pcm16(sample: f32) -> i16 {
    if sample.is_finite() {
        (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
    } else {
        0
    }
}

/// Measure one interleaved stereo buffer with fixed accumulation order.
#[must_use]
pub fn stereo_signal_metrics(samples: &[f32]) -> StereoSignalMetrics {
    let frame_count = samples.len() / 2;
    let mut non_finite_samples = 0usize;
    let mut subnormal_samples = 0usize;
    let mut clipped_samples = 0usize;
    let mut zero_samples = 0usize;
    let mut peak = 0.0f64;
    let mut left_sum = 0.0f64;
    let mut right_sum = 0.0f64;
    let mut left_square = 0.0f64;
    let mut right_square = 0.0f64;
    let mut cross = 0.0f64;
    let mut mid_square = 0.0f64;
    let mut side_square = 0.0f64;
    let mut previous: Option<[f64; 2]> = None;
    let mut max_step = 0.0f64;

    for frame in samples[..frame_count * 2].chunks_exact(2) {
        let mut channel = [0.0f64; 2];
        for (index, sample) in frame.iter().copied().enumerate() {
            if !sample.is_finite() {
                non_finite_samples += 1;
                continue;
            }
            subnormal_samples += usize::from(sample.is_subnormal());
            clipped_samples += usize::from(sample.abs() >= 1.0);
            zero_samples += usize::from(sample == 0.0);
            let value = f64::from(sample);
            peak = peak.max(value.abs());
            channel[index] = value;
        }
        let [left, right] = channel;
        left_sum += left;
        right_sum += right;
        left_square += left * left;
        right_square += right * right;
        cross += left * right;
        let mid = (left + right) * 0.5;
        let side = (left - right) * 0.5;
        mid_square += mid * mid;
        side_square += side * side;
        if let Some([previous_left, previous_right]) = previous {
            max_step = max_step
                .max((left - previous_left).abs())
                .max((right - previous_right).abs());
        }
        previous = Some([left, right]);
    }

    let frames = frame_count as f64;
    let left_rms = root_mean_square(left_square, frames);
    let right_rms = root_mean_square(right_square, frames);
    let rms = root_mean_square(left_square + right_square, frames * 2.0);
    let correlation_denominator = (left_square * right_square).sqrt();
    let correlation = if correlation_denominator > 0.0 {
        (cross / correlation_denominator).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    StereoSignalMetrics {
        frame_count,
        trailing_samples: samples.len() % 2,
        non_finite_samples,
        subnormal_samples,
        clipped_samples,
        peak,
        rms,
        crest_db: amplitude_ratio_db(peak, rms),
        left_rms,
        right_rms,
        channel_balance_db: amplitude_ratio_db(right_rms, left_rms),
        left_dc: if frame_count == 0 {
            0.0
        } else {
            left_sum / frames
        },
        right_dc: if frame_count == 0 {
            0.0
        } else {
            right_sum / frames
        },
        correlation,
        side_to_mid_db: amplitude_ratio_db(
            root_mean_square(side_square, frames),
            root_mean_square(mid_square, frames),
        ),
        max_step,
        zero_sample_fraction: if frame_count == 0 {
            0.0
        } else {
            zero_samples as f64 / (frames * 2.0)
        },
    }
}

fn root_mean_square(sum_of_squares: f64, count: f64) -> f64 {
    if count > 0.0 {
        (sum_of_squares / count).sqrt()
    } else {
        0.0
    }
}

fn amplitude_ratio_db(numerator: f64, denominator: f64) -> f64 {
    match (numerator > 0.0, denominator > 0.0) {
        (false, false) => 0.0,
        (false, true) => -120.0,
        (true, false) => 120.0,
        (true, true) => (20.0 * (numerator / denominator).log10()).clamp(-120.0, 120.0),
    }
}

/// One waveform sample in [-1, 1] at `phase` cycles (fractional part matters).
fn wave(voice: Voice, phase: f32, noise: &mut SplitMix64) -> f32 {
    let frac = phase.fract();
    match voice {
        Voice::Sine => (TAU * frac).sin(),
        Voice::Square => {
            if frac < 0.5 {
                0.6
            } else {
                -0.6
            }
        }
        Voice::Triangle => {
            // Rises 0..0.5, falls 0.5..1, spanning [-1, 1].
            if frac < 0.5 {
                4.0 * frac - 1.0
            } else {
                3.0 - 4.0 * frac
            }
        }
        Voice::Noise => (noise.next_f64() as f32) * 2.0 - 1.0,
    }
}

impl Pattern {
    /// Render the pattern to mono samples at `sample_rate`, clamped to [-1, 1].
    #[must_use]
    pub fn render(&self, sample_rate: u32) -> Vec<f32> {
        let rate = sample_rate.max(1);
        let mut out = vec![0.0f32; (self.seconds().max(0.0) * rate as f32) as usize];
        let mut noise = SplitMix64::new(TUNE_MIX);
        for (i, step) in self.steps.iter().enumerate() {
            let Some((freq, voice, level)) = *step else {
                continue;
            };
            let start = (i as f32 * self.step_seconds * rate as f32) as usize;
            let end = ((i + 1) as f32 * self.step_seconds * rate as f32) as usize;
            let length = end.min(out.len()).saturating_sub(start);
            for j in 0..length {
                let t = j as f32 / rate as f32;
                let phase = freq * t;
                let edge = edge_envelope(j, length, rate);
                out[start + j] = (wave(voice, phase, &mut noise) * level * edge).clamp(-1.0, 1.0);
            }
        }
        out
    }

    /// The pattern's duration in seconds.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.step_seconds * self.steps.len() as f32
    }
}

fn edge_envelope(index: usize, length: usize, sample_rate: u32) -> f32 {
    if length <= 1 {
        return 0.0;
    }
    let fade = (STEP_FADE * sample_rate.max(1) as f32)
        .max(1.0)
        .min((length - 1) as f32 / 2.0);
    let distance = index.min(length - 1 - index) as f32;
    let linear = (distance / fade).clamp(0.0, 1.0);
    0.5 - 0.5 * (std::f32::consts::PI * linear).cos()
}

/// A short noise crunch for Munch bite juice (one-shot, not a loop bed).
///
/// `seed` picks the noise draw; duration is about 45 ms with a fast decay so
/// repeated bites stay clicky without drowning the room score.
#[must_use]
pub fn munch_crunch(sample_rate: u32, seed: u64) -> Vec<f32> {
    let rate = sample_rate.max(8_000);
    let length = ((rate as f32) * 0.045).round() as usize;
    let length = length.clamp(32, rate as usize / 8);
    let mut noise = SplitMix64::new(TUNE_MIX ^ seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut out = Vec::with_capacity(length);
    for i in 0..length {
        let t = i as f32 / length as f32;
        let envelope = (1.0 - t).powi(3) * edge_envelope(i, length, rate);
        let sample = wave(Voice::Noise, 0.0, &mut noise) * 0.35 * envelope;
        out.push(sample.clamp(-1.0, 1.0));
    }
    out
}

/// A short square tick for game actions (quiz, nim, gauntlet).
///
/// `good` picks a bright upper pitch; otherwise a short low buzz for a miss.
#[must_use]
pub fn game_tick(sample_rate: u32, good: bool) -> Vec<f32> {
    let rate = sample_rate.max(8_000);
    let seconds = if good { 0.055 } else { 0.09 };
    let length = ((rate as f32) * seconds).round() as usize;
    let length = length.clamp(48, rate as usize / 6);
    let freq = if good { 784.0 } else { 165.0 };
    let mut out = Vec::with_capacity(length);
    for i in 0..length {
        let t = i as f32 / length as f32;
        let envelope = (1.0 - t).powi(2) * edge_envelope(i, length, rate);
        let sample = wave(Voice::Square, freq, &mut SplitMix64::new(1)) * 0.28 * envelope;
        out.push(sample.clamp(-1.0, 1.0));
    }
    out
}

/// A harsher buzz for bad Munch grades (pairs with a short screen shake).
#[must_use]
pub fn game_buzz(sample_rate: u32, seed: u64) -> Vec<f32> {
    let rate = sample_rate.max(8_000);
    let length = ((rate as f32) * 0.11).round() as usize;
    let length = length.clamp(64, rate as usize / 5);
    let mut noise = SplitMix64::new(TUNE_MIX ^ seed.wrapping_mul(0xA5A5_A5A5_1234_5678));
    let mut out = Vec::with_capacity(length);
    for i in 0..length {
        let t = i as f32 / length as f32;
        let envelope = (1.0 - t).powi(2) * edge_envelope(i, length, rate);
        let square = wave(Voice::Square, 110.0 + (seed % 7) as f32 * 4.0, &mut noise);
        let grit = wave(Voice::Noise, 0.0, &mut noise) * 0.35;
        out.push(((square * 0.22 + grit * 0.12) * envelope).clamp(-1.0, 1.0));
    }
    out
}

impl Arrangement {
    /// Render a stereo interleaved buffer with constant-power panning.
    #[must_use]
    pub fn render_stereo(&self, sample_rate: u32) -> Vec<f32> {
        let rate = sample_rate.max(1);
        let frames = (self.seconds().max(0.0) * rate as f32) as usize;
        let mut output = vec![0.0; frames.saturating_mul(2)];
        let mut noise = SplitMix64::new(TUNE_MIX);
        for note in &self.notes {
            if !note.frequency.is_finite()
                || note.frequency < 0.0
                || !note.level.is_finite()
                || note.step_count == 0
            {
                continue;
            }
            let start = (note.start_step as f32 * self.step_seconds * rate as f32) as usize;
            if start >= frames {
                continue;
            }
            let requested = (note.step_count as f32 * self.step_seconds * rate as f32) as usize;
            let length = requested.min(frames - start);
            let pan = if note.pan.is_finite() {
                note.pan.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            let (left_gain, right_gain) = (angle.cos(), angle.sin());
            let level = note.level.clamp(0.0, 1.0);
            for index in 0..length {
                let seconds = index as f32 / rate as f32;
                let sample = wave(note.voice, note.frequency * seconds, &mut noise)
                    * level
                    * edge_envelope(index, length, rate);
                let frame = (start + index) * 2;
                output[frame] += sample * left_gain;
                output[frame + 1] += sample * right_gain;
            }
        }
        for sample in &mut output {
            *sample = sample.clamp(-1.0, 1.0);
        }
        output
    }

    /// Render the arrangement to mono by folding its stereo field evenly.
    #[must_use]
    pub fn render(&self, sample_rate: u32) -> Vec<f32> {
        self.render_stereo(sample_rate)
            .chunks_exact(2)
            .map(|frame| ((frame[0] + frame[1]) * std::f32::consts::FRAC_1_SQRT_2).clamp(-1.0, 1.0))
            .collect()
    }

    /// Total arrangement duration in seconds.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.steps as f32 * self.step_seconds.max(0.0)
    }
}

/// The minor pentatonic degrees, in semitones: the scale that cannot miss.
const PENTATONIC: [i32; 5] = [0, 3, 5, 7, 10];

/// A note frequency: `root` shifted by `semitones` in equal temperament.
#[must_use]
pub fn pitch(root: f32, semitones: i32) -> f32 {
    root * 2.0_f32.powf(semitones as f32 / 12.0)
}

/// Compose a deterministic chip tune: a pentatonic lead over a root-fifth
/// bass with a noise tick, `bars` bars of eight steps. The same seed is the
/// same tune, forever, on every machine.
#[must_use]
pub fn compose(seed: u64, bars: usize) -> Pattern {
    let mut rng = SplitMix64::new(seed ^ TUNE_MIX);
    // A root in a comfortable register: A2 to A3.
    let root = pitch(110.0, rng.below(13) as i32);
    let steps_total = bars.clamp(1, 64) * 8;
    let mut steps = Vec::with_capacity(steps_total);
    let mut degree = 0usize;
    for i in 0..steps_total {
        let beat = i % 8;
        // Bass on the downbeats, alternating root and fifth, an octave down.
        if beat == 0 {
            steps.push(Some((root / 2.0, Voice::Triangle, 0.5)));
            continue;
        }
        if beat == 4 {
            steps.push(Some((pitch(root, 7) / 2.0, Voice::Triangle, 0.45)));
            continue;
        }
        // A noise tick on the offbeat, quiet.
        if beat == 6 && rng.below(2) == 0 {
            steps.push(Some((0.0, Voice::Noise, 0.12)));
            continue;
        }
        // The lead walks the pentatonic: mostly steps, sometimes a leap or rest.
        match rng.below(8) {
            0 | 1 => steps.push(None),
            2 => {
                degree = rng.below(PENTATONIC.len() as u64 * 2) as usize;
                let (octave, index) = (degree / PENTATONIC.len(), degree % PENTATONIC.len());
                steps.push(Some((
                    pitch(root, PENTATONIC[index] + 12 * octave as i32),
                    Voice::Square,
                    0.35,
                )));
            }
            _ => {
                let walk = rng.below(3) as i64 - 1; // -1, 0, +1
                degree =
                    (degree as i64 + walk).clamp(0, (PENTATONIC.len() * 2 - 1) as i64) as usize;
                let (octave, index) = (degree / PENTATONIC.len(), degree % PENTATONIC.len());
                steps.push(Some((
                    pitch(root, PENTATONIC[index] + 12 * octave as i32),
                    Voice::Square,
                    0.35,
                )));
            }
        }
    }
    Pattern {
        steps,
        step_seconds: 0.14,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Arrangement, ChipNote, Pattern, Voice, compose, pitch, quantize_pcm16,
        stereo_signal_metrics, wave,
    };
    use crate::rng::SplitMix64;

    #[test]
    fn waveforms_have_their_shapes() {
        let mut noise = SplitMix64::new(1);
        // Square: high in the first half cycle, low in the second.
        assert!(wave(Voice::Sine, 0.25, &mut noise) > 0.99);
        assert!(wave(Voice::Square, 0.25, &mut noise) > 0.0);
        assert!(wave(Voice::Square, 0.75, &mut noise) < 0.0);
        // Triangle: crosses its extremes at the quarter points.
        assert!((wave(Voice::Triangle, 0.5, &mut noise) - 1.0).abs() < 1e-6);
        assert!((wave(Voice::Triangle, 0.0, &mut noise) + 1.0).abs() < 1e-6);
        // Noise: bounded.
        for _ in 0..50 {
            let n = wave(Voice::Noise, 0.0, &mut noise);
            assert!((-1.0..=1.0).contains(&n));
        }
    }

    #[test]
    fn pitch_is_equal_temperament() {
        assert!((pitch(440.0, 12) - 880.0).abs() < 1e-3, "an octave doubles");
        assert!((pitch(440.0, 0) - 440.0).abs() < 1e-6);
    }

    #[test]
    fn composition_is_deterministic_and_sized() {
        let a = compose(7, 4);
        let b = compose(7, 4);
        assert_eq!(a, b, "the same seed is the same tune");
        assert_eq!(a.steps.len(), 32);
        assert!(compose(8, 4) != a, "different seeds differ");
    }

    #[test]
    fn munch_crunch_is_short_bounded_and_deterministic() {
        let a = super::munch_crunch(48_000, 7);
        let b = super::munch_crunch(48_000, 7);
        assert_eq!(a, b);
        assert!(!a.is_empty());
        assert!(a.len() < 48_000 / 10, "crunch stays a short one-shot");
        assert!(a.iter().all(|s| s.is_finite() && *s >= -1.0 && *s <= 1.0));
        assert!(a.iter().any(|s| s.abs() > 0.01), "crunch has energy");
        let other = super::munch_crunch(48_000, 8);
        assert_ne!(a, other, "seed changes the noise draw");
    }

    #[test]
    fn game_tick_and_buzz_are_short_bounded_and_deterministic() {
        let good = super::game_tick(48_000, true);
        let again = super::game_tick(48_000, true);
        let bad = super::game_tick(48_000, false);
        assert_eq!(good, again);
        assert_ne!(good, bad);
        assert!(good.len() < 48_000 / 8);
        assert!(
            bad.iter()
                .all(|s| s.is_finite() && (-1.0..=1.0).contains(s))
        );
        let buzz = super::game_buzz(48_000, 3);
        let buzz2 = super::game_buzz(48_000, 3);
        assert_eq!(buzz, buzz2);
        assert!(buzz.iter().any(|s| s.abs() > 0.01));
        assert_ne!(buzz, super::game_buzz(48_000, 4));
    }

    #[test]
    fn rendering_is_bounded_and_the_right_length() {
        let tune = compose(3, 2);
        let samples = tune.render(22_050);
        let expected = (tune.seconds() * 22_050.0) as usize;
        assert!((samples.len() as i64 - expected as i64).abs() <= 32);
        assert!(samples.iter().all(|s| (-1.0..=1.0).contains(s)));
        assert!(samples.iter().any(|&s| s != 0.0), "the chip makes sound");
    }

    #[test]
    fn rests_render_silence() {
        let rest = Pattern {
            steps: vec![None, None],
            step_seconds: 0.05,
        };
        assert!(rest.render(8_000).iter().all(|&s| s == 0.0));
    }

    #[test]
    fn every_note_and_loop_boundary_reaches_silence() {
        let pattern = Pattern {
            steps: vec![Some((440.0, Voice::Triangle, 0.4)); 4],
            step_seconds: 0.05,
        };
        let per_step = (pattern.step_seconds * 48_000.0) as usize;
        let samples = pattern.render(48_000);
        for step in 0..4 {
            assert_eq!(samples[step * per_step], 0.0);
            assert_eq!(samples[(step + 1) * per_step - 1], 0.0);
        }
    }

    #[test]
    fn arrangement_is_stereo_bounded_and_device_rate_exact() {
        let arrangement = Arrangement {
            notes: vec![
                ChipNote {
                    frequency: 220.0,
                    start_step: 0,
                    step_count: 8,
                    voice: Voice::Sine,
                    level: 0.15,
                    pan: -0.5,
                },
                ChipNote {
                    frequency: 330.0,
                    start_step: 2,
                    step_count: 4,
                    voice: Voice::Triangle,
                    level: 0.12,
                    pan: 0.5,
                },
            ],
            steps: 8,
            step_seconds: 0.125,
        };
        for rate in [44_100, 48_000, 96_000, 192_000] {
            let stereo = arrangement.render_stereo(rate);
            assert_eq!(stereo.len(), rate as usize * 2);
            assert!(stereo.iter().all(|sample| (-1.0..=1.0).contains(sample)));
            assert_eq!(stereo[0], 0.0);
            assert_eq!(stereo[stereo.len() - 1], 0.0);
            assert!(
                stereo.chunks_exact(2).any(|frame| frame[0] != frame[1]),
                "panning must create a real stereo field"
            );
        }
    }

    #[test]
    fn hostile_arrangement_events_fail_closed_without_breaking_frame_shape() {
        let arrangement = Arrangement {
            notes: vec![
                ChipNote {
                    frequency: f32::NAN,
                    start_step: 0,
                    step_count: 1,
                    voice: Voice::Sine,
                    level: 0.1,
                    pan: 0.0,
                },
                ChipNote {
                    frequency: -1.0,
                    start_step: 0,
                    step_count: 1,
                    voice: Voice::Square,
                    level: 0.1,
                    pan: 0.0,
                },
                ChipNote {
                    frequency: 220.0,
                    start_step: 0,
                    step_count: 1,
                    voice: Voice::Triangle,
                    level: f32::NAN,
                    pan: 0.0,
                },
                ChipNote {
                    frequency: 220.0,
                    start_step: 0,
                    step_count: 0,
                    voice: Voice::Noise,
                    level: 0.1,
                    pan: 0.0,
                },
                ChipNote {
                    frequency: 220.0,
                    start_step: 2,
                    step_count: 1,
                    voice: Voice::Sine,
                    level: 0.1,
                    pan: 0.0,
                },
                ChipNote {
                    frequency: 220.0,
                    start_step: 0,
                    step_count: 1,
                    voice: Voice::Sine,
                    level: 0.1,
                    pan: f32::NAN,
                },
            ],
            steps: 1,
            step_seconds: 1.0,
        };

        assert_eq!(arrangement.render_stereo(1), vec![0.0, 0.0]);
        assert_eq!(arrangement.render(1), vec![0.0]);
    }

    #[test]
    fn stereo_metrics_are_channel_aware_finite_and_total() {
        let samples = [0.0, 0.0, 0.5, -0.5, 0.0, 0.0, f32::NAN];
        let metrics = stereo_signal_metrics(&samples);

        assert_eq!(metrics.frame_count, 3);
        assert_eq!(metrics.trailing_samples, 1);
        assert_eq!(
            metrics.non_finite_samples, 0,
            "the trailing sample is excluded"
        );
        assert_eq!(metrics.subnormal_samples, 0);
        assert_eq!(metrics.clipped_samples, 0);
        assert!((metrics.peak - 0.5).abs() < 1e-12);
        assert!((metrics.rms - (1.0f64 / 12.0).sqrt()).abs() < 1e-12);
        assert!((metrics.left_rms - metrics.right_rms).abs() < 1e-12);
        assert!(metrics.channel_balance_db.abs() < 1e-12);
        assert!((metrics.left_dc - 1.0 / 6.0).abs() < 1e-12);
        assert!((metrics.right_dc + 1.0 / 6.0).abs() < 1e-12);
        assert!((metrics.correlation + 1.0).abs() < 1e-12);
        assert_eq!(metrics.side_to_mid_db, 120.0);
        assert!((metrics.max_step - 0.5).abs() < 1e-12);
        assert!((metrics.zero_sample_fraction - 2.0 / 3.0).abs() < 1e-12);

        let hostile = stereo_signal_metrics(&[f32::NAN, f32::INFINITY]);
        assert_eq!(hostile.non_finite_samples, 2);
        assert_eq!(hostile.rms, 0.0);
        assert_eq!(hostile.correlation, 0.0);
        assert_eq!(hostile.side_to_mid_db, 0.0);

        let quiet_imbalance = stereo_signal_metrics(&[1e-20, 1e-30]);
        assert_eq!(quiet_imbalance.channel_balance_db, -120.0);
        let one_sided = stereo_signal_metrics(&[0.0, 1e-20]);
        assert_eq!(one_sided.channel_balance_db, 120.0);
        let other_side = stereo_signal_metrics(&[1e-20, 0.0]);
        assert_eq!(other_side.channel_balance_db, -120.0);

        let empty = stereo_signal_metrics(&[]);
        assert_eq!(empty.frame_count, 0);
        assert_eq!(empty.left_dc, 0.0);
        assert_eq!(empty.right_dc, 0.0);
        assert_eq!(empty.zero_sample_fraction, 0.0);
        assert_eq!(empty.rms, 0.0);

        assert_eq!(Voice::Sine.id(), "sine");
        assert_eq!(Voice::Square.id(), "square");
        assert_eq!(Voice::Triangle.id(), "triangle");
        assert_eq!(Voice::Noise.id(), "noise");
    }

    #[test]
    fn pcm16_quantization_is_bounded_and_fails_nonfinite_to_silence() {
        assert_eq!(quantize_pcm16(1.5), i16::MAX);
        assert_eq!(quantize_pcm16(-1.5), -i16::MAX);
        assert_eq!(quantize_pcm16(f32::NAN), 0);
        assert_eq!(quantize_pcm16(0.5), 16_383);
    }
}
