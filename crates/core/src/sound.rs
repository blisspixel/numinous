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
        let duration = if self.duration.is_finite() {
            self.duration.max(0.0)
        } else {
            0.0
        };
        let total = (duration * rate) as usize;
        let mut buffer = vec![0.0f32; total];
        for note in &self.notes {
            if !note.freq.is_finite()
                || !note.start.is_finite()
                || !note.dur.is_finite()
                || !note.amp.is_finite()
            {
                continue;
            }
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
            *sample = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }
        buffer
    }

    /// Render straight to the bytes of a mono 16-bit PCM WAV file.
    ///
    /// Six packaged playtests have ended on the same sentence: an agent can
    /// read the notes, name the intervals, and still not hear the thing. A
    /// player who reaches this house over a protocol cannot open an audio
    /// device, so the sound has to become something that can be sent. These are
    /// those bytes.
    #[must_use]
    pub fn wav(&self, sample_rate: u32) -> Vec<u8> {
        wav_bytes(&self.render(sample_rate), sample_rate)
    }
}

/// Wrap mono samples in a 16-bit PCM WAV container.
///
/// Pure bytes, written by hand rather than by a library, because the whole
/// point is that this path needs no audio device and no dependency: a face that
/// can only send text can still send a sound.
#[must_use]
pub fn wav_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    const HEADER: usize = 44;
    const BITS: u16 = 16;
    const CHANNELS: u16 = 1;
    let rate = sample_rate.max(1);
    // A WAV size field is 32 bits, so the container itself sets the ceiling.
    // Truncating loudly here is better than writing a header that lies.
    let max_samples = ((u32::MAX as usize - HEADER) / 2).min(samples.len());
    let samples = &samples[..max_samples];
    let data_len = samples.len() * 2;
    let mut bytes = Vec::with_capacity(HEADER + data_len);
    let mut chunk = |tag: &[u8; 4], size: u32| {
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(&size.to_le_bytes());
    };
    chunk(b"RIFF", (36 + data_len) as u32);
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM, uncompressed
    bytes.extend_from_slice(&CHANNELS.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(CHANNELS) * u32::from(BITS / 8)).to_le_bytes());
    bytes.extend_from_slice(&(CHANNELS * BITS / 8).to_le_bytes());
    bytes.extend_from_slice(&BITS.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &sample in samples {
        let clamped = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        // Scale by 32767 rather than 32768 so full scale cannot wrap to silence.
        bytes.extend_from_slice(&((clamped * 32_767.0) as i16).to_le_bytes());
    }
    bytes
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
    use super::{Note, ParametricSound, SoundSpec, envelope, wav_bytes};
    use crate::Motif;

    #[test]
    fn a_wav_is_a_real_wav_and_not_a_hopeful_one() {
        // These bytes leave the house and get decoded by something that is not
        // us, so the header has to be right by inspection rather than by luck.
        let spec = SoundSpec::tone(440.0, 0.5, 0.5);
        let rate = 16_000;
        let bytes = spec.wav(rate);
        let field = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("field"));
        let short = |at: usize| u16::from_le_bytes(bytes[at..at + 2].try_into().expect("field"));
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(field(16), 16, "PCM format chunks are sixteen bytes");
        assert_eq!(short(20), 1, "uncompressed PCM");
        assert_eq!(short(22), 1, "mono");
        assert_eq!(field(24), rate);
        assert_eq!(field(28), rate * 2, "byte rate is rate times block align");
        assert_eq!(short(32), 2, "block align");
        assert_eq!(short(34), 16, "bits per sample");
        assert_eq!(&bytes[36..40], b"data");
        // Every size field has to describe the bytes that are actually here,
        // because a decoder trusts them over the file.
        let data_len = field(40) as usize;
        assert_eq!(data_len, bytes.len() - 44);
        assert_eq!(field(4) as usize, bytes.len() - 8);
        assert_eq!(data_len, spec.render(rate).len() * 2);
        // And it has to carry the sound, not silence.
        assert!(
            bytes[44..]
                .chunks_exact(2)
                .any(|pair| { i16::from_le_bytes([pair[0], pair[1]]).unsigned_abs() > 1_000 }),
            "the file is silent"
        );
    }

    #[test]
    fn a_hostile_sample_cannot_poison_the_file() {
        let bytes = wav_bytes(&[f32::NAN, f32::INFINITY, -12.0, 1.0, -1.0], 8_000);
        let words: Vec<i16> = bytes[44..]
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        // A value that is not a number is silenced rather than clamped, the
        // same way `render` treats it: infinity is a bug upstream, and a bug
        // should not arrive as a full-scale click in someone's ears.
        assert_eq!(words, vec![0, 0, -32_767, 32_767, -32_767]);
    }

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
    fn render_neutralizes_non_finite_specs() {
        let spec = SoundSpec {
            duration: 0.1,
            notes: vec![Note {
                freq: f32::NAN,
                start: 0.0,
                dur: 0.1,
                amp: 0.5,
            }],
        };
        assert!(spec.render(1_000).iter().all(|sample| *sample == 0.0));

        let invalid_duration = SoundSpec {
            duration: f32::INFINITY,
            notes: Vec::new(),
        };
        assert!(invalid_duration.render(1_000).is_empty());
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
