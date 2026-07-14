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

/// A continuous, low-level mathematical voice controlled by room input.
///
/// Faces with a real-time mixer can glide this voice without restarting the
/// room bed. Text and protocol faces can call [`ParametricSound::snapshot`] to
/// hear the same accepted parameter as a short deterministic chord.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametricSound {
    /// Fundamental frequency in Hz.
    root_hz: f32,
    /// Frequency ratio between the upper and lower voices.
    ratio: f32,
    /// Peak amplitude per voice in `(0, 0.08]`.
    gain: f32,
}

impl ParametricSound {
    /// Highest supported gain for a continuously mixed parameter voice.
    pub const MAX_GAIN: f32 = 0.08;

    /// Build a safe voice, rejecting values that could poison an audio mixer.
    #[must_use]
    pub fn new(root_hz: f32, ratio: f32, gain: f32) -> Option<Self> {
        (root_hz.is_finite()
            && ratio.is_finite()
            && gain.is_finite()
            && root_hz > 0.0
            && ratio > 0.0
            && root_hz.mul_add(ratio, 0.0).is_finite()
            && (0.0..=Self::MAX_GAIN).contains(&gain)
            && gain > 0.0)
            .then_some(Self {
                root_hz,
                ratio,
                gain,
            })
    }

    /// Fundamental frequency in Hz.
    #[must_use]
    pub const fn root_hz(self) -> f32 {
        self.root_hz
    }

    /// Frequency ratio between the upper and lower voices.
    #[must_use]
    pub const fn ratio(self) -> f32 {
        self.ratio
    }

    /// Peak amplitude per voice.
    #[must_use]
    pub const fn gain(self) -> f32 {
        self.gain
    }

    /// Render the current parameter as a short two-voice chord.
    #[must_use]
    pub fn snapshot(self) -> SoundSpec {
        SoundSpec::chord(&[self.root_hz, self.root_hz * self.ratio], 1.5, self.gain)
    }
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

    /// Several tones in sequence (an arpeggio), evenly spaced across
    /// `duration`, each sustaining until the next begins. A room with no
    /// bespoke sound still speaks a short phrase this way, rather than one
    /// held tone.
    #[must_use]
    pub fn arpeggio(freqs: &[f32], duration: f32, amp: f32) -> Self {
        let step = duration / freqs.len().max(1) as f32;
        Self {
            duration,
            notes: freqs
                .iter()
                .enumerate()
                .map(|(i, &freq)| Note {
                    freq,
                    start: i as f32 * step,
                    dur: step,
                    amp,
                })
                .collect(),
        }
    }

    /// Play a motif's notated line as a spacious counterphrase.
    ///
    /// Its total duration matches the four-cycle chiptune arrangement, so faces
    /// can combine the two without independent loop lengths drifting against
    /// each other. Every notated pitch appears once, separated by a short rest.
    #[must_use]
    pub fn from_motif(motif: &crate::motifs::Motif) -> Self {
        let duration = motif.pattern().seconds();
        let spacing = duration / motif.line.len().max(1) as f32;
        let notes: Vec<Note> = motif
            .line
            .iter()
            .enumerate()
            .map(|(i, &degree)| Note {
                freq: crate::chiptune::pitch(motif.root, degree),
                start: i as f32 * spacing,
                dur: spacing * 0.72,
                amp: 0.12,
            })
            .collect();
        Self { duration, notes }
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
        let rate = sample_rate.max(1) as f32;
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
    use super::{ParametricSound, SoundSpec, envelope};
    use crate::Motif;

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
    fn parametric_sound_rejects_hostile_values_and_preserves_its_ratio() {
        assert!(ParametricSound::new(f32::NAN, 1.5, 0.1).is_none());
        assert!(ParametricSound::new(220.0, 0.0, 0.1).is_none());
        assert!(ParametricSound::new(f32::MAX, 2.0, 0.1).is_none());
        assert!(ParametricSound::new(220.0, 1.5, 0.0).is_none());
        assert!(ParametricSound::new(220.0, 1.5, 0.081).is_none());

        let voice = ParametricSound::new(220.0, 1.25, 0.04).expect("valid voice");
        assert_eq!(voice.root_hz(), 220.0);
        assert_eq!(voice.ratio(), 1.25);
        assert_eq!(voice.gain(), 0.04);
        let spec = voice.snapshot();
        assert_eq!(spec.notes.len(), 2);
        assert_eq!(spec.notes[0].freq, 220.0);
        assert_eq!(spec.notes[1].freq, 275.0);
        assert_eq!(spec.notes[0].amp, 0.04);
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

    #[test]
    fn native_device_rates_preserve_duration_and_pitch() {
        for sample_rate in [44_100, 48_000, 96_000, 192_000] {
            let samples = SoundSpec::tone(440.0, 1.0, 0.3).render(sample_rate);
            assert_eq!(samples.len(), sample_rate as usize);

            let middle = &samples[sample_rate as usize / 4..sample_rate as usize * 3 / 4];
            let rising_crossings = middle
                .windows(2)
                .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
                .count();
            assert!(
                (219..=221).contains(&rising_crossings),
                "440 Hz drifted to {rising_crossings} half-second cycles at {sample_rate} Hz"
            );
        }
    }

    #[test]
    fn motif_counterphrase_matches_arrangement_length_and_breathes() {
        let motif = Motif {
            key: "A minor",
            root: 220.0,
            tempo: 120,
            line: &[0, 3, 7, 12, 7, 3],
            encodes: "test",
        };
        let spec = SoundSpec::from_motif(&motif);
        assert_eq!(spec.duration, motif.pattern().seconds());
        assert_eq!(spec.notes.len(), motif.line.len());
        assert!(
            spec.notes
                .windows(2)
                .all(|notes| { notes[0].start + notes[0].dur < notes[1].start })
        );
        assert!(spec.notes.iter().all(|note| note.amp <= 0.12));
    }
}
