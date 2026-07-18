//! Bounded sonification for one exact Game of Life generation.
//!
//! Every birth contributes to one of twelve vertical pitch rows and to that
//! row's horizontal energy centroid. The fixed reduction keeps the control
//! path independent of population size while preserving the generation's
//! total activity, register, and stereo position.

use crate::sound::{Note, SoundSpec};

const PITCH_ROWS: usize = 12;
const DURATION_MILLIS: usize = 105;
const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 384_000;
const MAX_PEAK: f32 = 0.2;
const SEMITONES: [i32; PITCH_ROWS] = [0, 2, 4, 7, 9, 12, 14, 16, 19, 21, 24, 26];
const GLIDER_SEMITONES: [i32; 4] = [0, 4, 7, 11];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PitchRow {
    births: u16,
    x_sum: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GliderPhrase {
    phase: u8,
    x: u16,
}

#[derive(Clone, Copy, Debug)]
struct SonicVoice {
    sine: f32,
    cosine: f32,
    step_sine: f32,
    step_cosine: f32,
    weight: f32,
    left: f32,
    right: f32,
}

impl SonicVoice {
    fn advance(&mut self) {
        let next_sine = self.sine * self.step_cosine + self.cosine * self.step_sine;
        let next_cosine = self.cosine * self.step_cosine - self.sine * self.step_sine;
        self.sine = next_sine;
        self.cosine = next_cosine;
    }
}

/// Fixed-size sonic summary of one B3/S23 transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifeStepSound {
    births: u16,
    width: u16,
    rows: [PitchRow; PITCH_ROWS],
    glider: Option<GliderPhrase>,
}

impl Default for LifeStepSound {
    fn default() -> Self {
        Self {
            births: 0,
            width: 1,
            rows: [PitchRow::default(); PITCH_ROWS],
            glider: None,
        }
    }
}

impl LifeStepSound {
    /// Reduce an exact birth mask without allocating per-cell voices.
    pub(crate) fn from_birth_mask(mask: &[bool], width: usize, height: usize) -> Self {
        let Some(cells) = width.checked_mul(height) else {
            return Self::default();
        };
        if width == 0
            || height == 0
            || width > u16::MAX as usize
            || mask.len() != cells
            || cells > u16::MAX as usize
        {
            return Self::default();
        }

        let mut sound = Self {
            births: 0,
            width: width as u16,
            rows: [PitchRow::default(); PITCH_ROWS],
            glider: None,
        };
        for (index, &born) in mask.iter().enumerate() {
            if !born {
                continue;
            }
            let x = index % width;
            let y = index / width;
            let row_index = ((height - 1 - y) * PITCH_ROWS / height).min(PITCH_ROWS - 1);
            let row = &mut sound.rows[row_index];
            row.births = row.births.saturating_add(1);
            row.x_sum = row.x_sum.saturating_add(x as u32);
            sound.births = sound.births.saturating_add(1);
        }
        sound
    }

    /// Exact number of births represented by this generation.
    #[must_use]
    pub fn birth_count(&self) -> usize {
        usize::from(self.births)
    }

    /// Zero-based four-step phrase phase of the newest exact isolated glider.
    #[must_use]
    pub fn glider_phase(&self) -> Option<u8> {
        self.glider.map(|glider| glider.phase)
    }

    pub(crate) fn set_tracked_glider(&mut self, phase: u8, x: usize) {
        self.glider = (usize::from(phase) < GLIDER_SEMITONES.len() && x < usize::from(self.width))
            .then_some(GliderPhrase { phase, x: x as u16 });
    }

    /// Bounded simultaneous-note snapshot for CLI and protocol faces.
    #[must_use]
    pub fn snapshot(&self) -> Option<SoundSpec> {
        if self.births == 0 && self.glider.is_none() {
            return None;
        }
        let births = f32::from(self.births).max(1.0);
        let activity = 0.45 + 0.55 * (births / 256.0).sqrt().min(1.0);
        let mut notes = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.births > 0)
            .map(|(index, row)| Note {
                freq: pitch_hz(index),
                start: 0.0,
                dur: DURATION_MILLIS as f32 / 1_000.0,
                amp: 0.06 * activity * (f32::from(row.births) / births).sqrt(),
            })
            .collect::<Vec<_>>();
        if let Some(glider) = self.glider {
            notes.push(Note {
                freq: glider_pitch_hz(glider.phase),
                start: 0.0,
                dur: DURATION_MILLIS as f32 / 1_000.0,
                amp: 0.055,
            });
        }
        Some(SoundSpec {
            duration: DURATION_MILLIS as f32 / 1_000.0,
            notes,
        })
    }

    /// Render the generation as a short interleaved-stereo birth texture.
    ///
    /// No output is returned for silence or unsupported rates. Work is bounded
    /// by twelve birth rows, one optional glider voice, and 105 milliseconds,
    /// regardless of population.
    #[must_use]
    pub fn render_stereo(&self, sample_rate: u32) -> Vec<f32> {
        if (self.births == 0 && self.glider.is_none())
            || !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate)
        {
            return Vec::new();
        }
        let Some(frames) = (sample_rate as usize).checked_mul(DURATION_MILLIS) else {
            return Vec::new();
        };
        let frames = frames / 1_000;
        let Some(samples) = frames.checked_mul(2) else {
            return Vec::new();
        };
        if frames < 2 {
            return Vec::new();
        }

        let births = f32::from(self.births).max(1.0);
        let activity = 0.45 + 0.55 * (births / 256.0).sqrt().min(1.0);
        let harmonic_mix = 0.08 + 0.22 * (births / 512.0).min(1.0);
        let mut voices: [Option<SonicVoice>; PITCH_ROWS] = std::array::from_fn(|index| {
            let row = self.rows[index];
            if row.births == 0 {
                return None;
            }
            let weight = (f32::from(row.births) / births).sqrt();
            let pan = if self.width <= 1 {
                0.5
            } else {
                row.x_sum as f32 / (f32::from(row.births) * f32::from(self.width - 1))
            }
            .clamp(0.0, 1.0);
            let (step_sine, step_cosine) =
                (std::f32::consts::TAU * pitch_hz(index) / sample_rate as f32).sin_cos();
            Some(SonicVoice {
                sine: 0.0,
                cosine: 1.0,
                step_sine,
                step_cosine,
                weight,
                left: (1.0 - pan).sqrt(),
                right: pan.sqrt(),
            })
        });
        let mut glider_voice = self.glider.map(|glider| {
            let pan = if self.width <= 1 {
                0.5
            } else {
                f32::from(glider.x) / f32::from(self.width - 1)
            }
            .clamp(0.0, 1.0);
            let (step_sine, step_cosine) = (std::f32::consts::TAU * glider_pitch_hz(glider.phase)
                / sample_rate as f32)
                .sin_cos();
            SonicVoice {
                sine: 0.0,
                cosine: 1.0,
                step_sine,
                step_cosine,
                weight: 0.9,
                left: (1.0 - pan).sqrt(),
                right: pan.sqrt(),
            }
        });
        let mut output = Vec::with_capacity(samples);
        for frame in 0..frames {
            let time = frame as f32 / sample_rate as f32;
            let progress = frame as f32 / (frames - 1) as f32;
            let attack = (time / 0.006).min(1.0);
            let envelope = attack * (1.0 - progress).powi(2);
            let mut left = 0.0;
            let mut right = 0.0;
            for voice in &mut voices {
                let Some(voice) = voice else {
                    continue;
                };
                let tone = voice.sine * (1.0 - harmonic_mix)
                    + 2.0 * voice.sine * voice.cosine * harmonic_mix;
                left += tone * voice.weight * voice.left;
                right += tone * voice.weight * voice.right;
                voice.advance();
            }
            if let Some(voice) = &mut glider_voice {
                let phrase_envelope = (1.0 - progress).powi(3);
                let tone = voice.sine + 0.12 * 2.0 * voice.sine * voice.cosine;
                left += tone * voice.weight * voice.left * phrase_envelope;
                right += tone * voice.weight * voice.right * phrase_envelope;
                voice.advance();
            }
            let gain = 0.045 * activity * envelope;
            output.push((left * gain).clamp(-MAX_PEAK, MAX_PEAK));
            output.push((right * gain).clamp(-MAX_PEAK, MAX_PEAK));
        }
        output
    }
}

fn pitch_hz(row: usize) -> f32 {
    130.81 * 2.0_f32.powf(SEMITONES[row] as f32 / 12.0)
}

fn glider_pitch_hz(phase: u8) -> f32 {
    261.63 * 2.0_f32.powf(GLIDER_SEMITONES[usize::from(phase)] as f32 / 12.0)
}

#[cfg(test)]
mod tests {
    use super::{LifeStepSound, MAX_PEAK, PITCH_ROWS};

    #[test]
    fn every_birth_reduces_into_the_fixed_pitch_rows() {
        let mask = vec![true; 96 * 96];
        let sound = LifeStepSound::from_birth_mask(&mask, 96, 96);

        assert_eq!(sound.birth_count(), mask.len());
        assert_eq!(
            sound.rows.iter().filter(|row| row.births > 0).count(),
            PITCH_ROWS
        );
        assert_eq!(
            sound
                .rows
                .iter()
                .map(|row| usize::from(row.births))
                .sum::<usize>(),
            mask.len()
        );
    }

    #[test]
    fn vertical_position_orders_pitch_and_horizontal_position_orders_energy() {
        let mut low_left = vec![false; 96 * 96];
        low_left[95 * 96] = true;
        let mut high_right = vec![false; 96 * 96];
        high_right[95] = true;
        let low = LifeStepSound::from_birth_mask(&low_left, 96, 96);
        let high = LifeStepSound::from_birth_mask(&high_right, 96, 96);

        let low_note = low.snapshot().expect("low note").notes[0].freq;
        let high_note = high.snapshot().expect("high note").notes[0].freq;
        assert!(high_note > low_note);

        let left = low.render_stereo(48_000);
        let right = high.render_stereo(48_000);
        let energy = |samples: &[f32], channel: usize| {
            samples
                .chunks_exact(2)
                .map(|frame| frame[channel].abs())
                .sum::<f32>()
        };
        assert!(energy(&left, 0) > energy(&left, 1) * 4.0);
        assert!(energy(&right, 1) > energy(&right, 0) * 4.0);
    }

    #[test]
    fn snapshot_amplitude_preserves_each_pitch_rows_birth_weight() {
        let mut mask = vec![false; 12 * 12];
        mask[11 * 12] = true;
        mask.iter_mut().take(4).for_each(|born| *born = true);
        let snapshot = LifeStepSound::from_birth_mask(&mask, 12, 12)
            .snapshot()
            .expect("weighted snapshot");

        assert_eq!(snapshot.notes.len(), 2);
        assert!(snapshot.notes[1].freq > snapshot.notes[0].freq);
        assert!((snapshot.notes[1].amp / snapshot.notes[0].amp - 2.0).abs() < 1.0e-6);
    }

    #[test]
    fn tracked_glider_adds_one_distinct_note_for_each_exact_phase() {
        let mut mask = vec![false; 96 * 96];
        mask[48 * 96 + 48] = true;
        let mut frequencies = Vec::new();
        let mut renders = Vec::new();

        for phase in 0..4 {
            let mut sound = LifeStepSound::from_birth_mask(&mask, 96, 96);
            sound.set_tracked_glider(phase, 48);
            assert_eq!(sound.glider_phase(), Some(phase));
            let snapshot = sound.snapshot().expect("birth and glider phrase");
            frequencies.push(snapshot.notes.last().expect("glider note").freq);
            renders.push(sound.render_stereo(48_000));
        }

        assert!(frequencies.windows(2).all(|pair| pair[1] > pair[0]));
        assert!(renders.windows(2).all(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn malformed_glider_phase_or_position_is_rejected() {
        let mut mask = vec![false; 96 * 96];
        mask[0] = true;
        let mut sound = LifeStepSound::from_birth_mask(&mask, 96, 96);

        sound.set_tracked_glider(0, 0);
        assert_eq!(sound.glider_phase(), Some(0));
        sound.set_tracked_glider(4, 0);
        assert_eq!(sound.glider_phase(), None);
        sound.set_tracked_glider(0, 0);
        sound.set_tracked_glider(0, 96);
        assert_eq!(sound.glider_phase(), None);
    }

    #[test]
    fn tracked_glider_horizontal_position_orders_phrase_energy() {
        let mask = vec![false; 96 * 96];
        let mut left = LifeStepSound::from_birth_mask(&mask, 96, 96);
        let mut right = left.clone();
        left.set_tracked_glider(0, 0);
        right.set_tracked_glider(0, 95);
        let left = left.render_stereo(48_000);
        let right = right.render_stereo(48_000);
        let energy = |samples: &[f32], channel: usize| {
            samples
                .chunks_exact(2)
                .map(|frame| frame[channel].abs())
                .sum::<f32>()
        };

        assert!(energy(&left, 0) > energy(&left, 1) * 4.0);
        assert!(energy(&right, 1) > energy(&right, 0) * 4.0);
    }

    #[test]
    fn rendering_is_deterministic_finite_bounded_and_rate_limited() {
        let mut mask = vec![false; 96 * 96];
        for index in (0..mask.len()).step_by(17) {
            mask[index] = true;
        }
        let mut sound = LifeStepSound::from_birth_mask(&mask, 96, 96);
        sound.set_tracked_glider(3, 95);
        let first = sound.render_stereo(48_000);
        let metrics = crate::chiptune::stereo_signal_metrics(&first);
        assert_eq!(first, sound.render_stereo(48_000));
        assert_eq!(first.len(), 48_000 * 105 / 1_000 * 2);
        assert!(first.iter().all(|sample| sample.is_finite()));
        assert!(first.iter().all(|sample| sample.abs() <= MAX_PEAK));
        assert_eq!(metrics.non_finite_samples, 0);
        assert_eq!(metrics.clipped_samples, 0);
        assert!(metrics.peak > 0.01 && metrics.peak <= f64::from(MAX_PEAK));
        assert!(metrics.rms > 0.001 && metrics.rms < 0.1);
        assert!(metrics.left_dc.abs() < 0.001);
        assert!(metrics.right_dc.abs() < 0.001);
        assert!(metrics.max_step < 0.02);
        assert!(metrics.side_to_mid_db > -60.0);
        assert!(sound.render_stereo(0).is_empty());
        assert!(sound.render_stereo(384_001).is_empty());
        assert!(LifeStepSound::default().render_stereo(48_000).is_empty());
    }

    #[test]
    fn malformed_masks_fail_closed() {
        assert_eq!(
            LifeStepSound::from_birth_mask(&[true], 0, 1),
            LifeStepSound::default()
        );
        assert_eq!(
            LifeStepSound::from_birth_mask(&[true], usize::MAX, 2),
            LifeStepSound::default()
        );
        assert_eq!(
            LifeStepSound::from_birth_mask(&[true], 2, 2),
            LifeStepSound::default()
        );
    }
}
