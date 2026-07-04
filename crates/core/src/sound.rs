//! A tiny sound model: a room's audio as a set of timed sine notes.
//!
//! Every room can describe its own sound (the "everything is an instrument"
//! pillar, see `docs/SOUND.md`). Rendering to samples is pure (std `sin`),
//! deterministic, and needs no audio device, so it is testable and can be
//! written straight to a WAV. Real-time playback (the `audio` crate) renders the
//! same `SoundSpec`.

use std::f32::consts::TAU;

/// A short attack in seconds, so notes do not click on.
const ATTACK: f32 = 0.01;
/// A short release in seconds, so notes do not click off.
const RELEASE: f32 = 0.05;

/// A single sine note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    /// Frequency in Hz.
    pub freq: f32,
    /// Start time in seconds from the beginning of the sound.
    pub start: f32,
    /// Duration in seconds.
    pub dur: f32,
    /// Peak amplitude in `[0, 1]`.
    pub amp: f32,
}

/// A room's sound: notes over a total duration, in seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundSpec {
    /// Total length of the sound in seconds.
    pub duration: f32,
    /// The notes that make it up.
    pub notes: Vec<Note>,
}

impl SoundSpec {
    /// A single tone for `duration` seconds.
    #[must_use]
    pub fn tone(freq: f32, duration: f32, amp: f32) -> Self {
        Self {
            duration,
            notes: vec![Note {
                freq,
                start: 0.0,
                dur: duration,
                amp,
            }],
        }
    }

    /// Several simultaneous tones (a chord) for `duration` seconds.
    #[must_use]
    pub fn chord(freqs: &[f32], duration: f32, amp: f32) -> Self {
        Self {
            duration,
            notes: freqs
                .iter()
                .map(|&freq| Note {
                    freq,
                    start: 0.0,
                    dur: duration,
                    amp,
                })
                .collect(),
        }
    }

    /// Render to mono `f32` samples at `sample_rate`, clamped to `[-1, 1]`.
    ///
    /// Deterministic and device-free.
    #[must_use]
    pub fn render(&self, sample_rate: u32) -> Vec<f32> {
        let rate = f32::from(u16::try_from(sample_rate.max(1)).unwrap_or(u16::MAX));
        let total = (self.duration.max(0.0) * rate) as usize;
        let mut buffer = vec![0.0f32; total];
        for note in &self.notes {
            let start = (note.start.max(0.0) * rate) as usize;
            let len = (note.dur.max(0.0) * rate) as usize;
            for i in 0..len {
                let idx = start + i;
                if idx >= total {
                    break;
                }
                let seconds = i as f32 / rate;
                let env = envelope(seconds, note.dur);
                buffer[idx] += (TAU * note.freq * seconds).sin() * note.amp * env;
            }
        }
        for sample in &mut buffer {
            *sample = sample.clamp(-1.0, 1.0);
        }
        buffer
    }
}

/// A short attack/release envelope so notes do not click.
fn envelope(t: f32, dur: f32) -> f32 {
    if t < ATTACK {
        (t / ATTACK).clamp(0.0, 1.0)
    } else if t > dur - RELEASE {
        ((dur - t) / RELEASE).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundSpec, envelope};

    #[test]
    fn tone_has_one_note_and_the_right_length() {
        let spec = SoundSpec::tone(440.0, 1.0, 0.3);
        assert_eq!(spec.notes.len(), 1);
        assert_eq!(spec.render(44_100).len(), 44_100);
    }

    #[test]
    fn chord_has_a_note_per_frequency() {
        let spec = SoundSpec::chord(&[220.0, 330.0], 0.5, 0.2);
        assert_eq!(spec.notes.len(), 2);
    }

    #[test]
    fn render_is_deterministic_and_bounded() {
        let spec = SoundSpec::tone(440.0, 0.25, 0.9);
        let a = spec.render(44_100);
        let b = spec.render(44_100);
        assert_eq!(a, b);
        assert!(a.iter().all(|s| (-1.0..=1.0).contains(s)));
    }

    #[test]
    fn render_actually_produces_signal() {
        let spec = SoundSpec::tone(440.0, 0.25, 0.5);
        let peak = spec.render(44_100).iter().cloned().fold(0.0f32, f32::max);
        assert!(peak > 0.1, "the tone should be audible, peak was {peak}");
    }

    #[test]
    fn envelope_fades_in_and_out() {
        assert!(envelope(0.0, 1.0) < 0.01);
        assert!((envelope(0.5, 1.0) - 1.0).abs() < 1e-6);
        assert!(envelope(1.0, 1.0) < 0.01);
    }
}
