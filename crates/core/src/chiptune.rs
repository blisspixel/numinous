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
    use super::{Arrangement, ChipNote, Pattern, Voice, compose, pitch, wave};
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
}
