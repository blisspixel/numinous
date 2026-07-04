//! Numinous audio.
//!
//! Adaptive output through `cpal`: it uses the system default output device and
//! its default configuration, so it "just works" and follows the machine's sound
//! settings on Windows (WASAPI), macOS (CoreAudio), and Linux (ALSA). The tone
//! synthesis is a pure, testable function; opening and driving the device is kept
//! separate. See `docs/SOUND.md` and `docs/ARCHITECTURE.md`.

use std::f32::consts::TAU;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// A gentle amplitude so a test tone is never harsh.
const AMPLITUDE: f32 = 0.2;

/// The system default output device and its default configuration.
pub struct AudioContext {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

impl AudioContext {
    /// Open the system default output device.
    ///
    /// # Errors
    /// Returns an error string if there is no default output device or its
    /// configuration cannot be queried.
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no default output device".to_string())?;
        let config = device
            .default_output_config()
            .map_err(|e| format!("no default output config: {e}"))?;
        Ok(Self { device, config })
    }

    /// The output device name (for example "Speakers").
    #[must_use]
    pub fn device_name(&self) -> String {
        self.device.name().unwrap_or_else(|_| "unknown".to_string())
    }

    /// The device's default sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate().0
    }

    /// The device's default channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.config.channels()
    }

    /// Play a sine tone of `frequency` Hz for `seconds` on the default device.
    ///
    /// Blocks for the duration, then stops. Adapts to the device's sample format
    /// (f32 or i16).
    ///
    /// # Errors
    /// Returns an error string if the stream cannot be built or started, or if
    /// the device uses an unsupported sample format.
    pub fn play_tone(&self, frequency: f32, seconds: f32) -> Result<(), String> {
        let sample_rate = self.sample_rate() as f32;
        let channels = self.channels() as usize;
        let config: cpal::StreamConfig = self.config.clone().into();
        let err_fn = |e| eprintln!("audio stream error: {e}");

        let mut phase = 0.0f32;
        let mut next = move || {
            let value = (TAU * frequency * phase / sample_rate).sin() * AMPLITUDE;
            phase += 1.0;
            value
        };

        let stream = match self.config.sample_format() {
            cpal::SampleFormat::F32 => self.device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(channels) {
                        let value = next();
                        for sample in frame {
                            *sample = value;
                        }
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => self.device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    for frame in data.chunks_mut(channels) {
                        let value = (next() * f32::from(i16::MAX)) as i16;
                        for sample in frame {
                            *sample = value;
                        }
                    }
                },
                err_fn,
                None,
            ),
            other => return Err(format!("unsupported sample format: {other:?}")),
        }
        .map_err(|e| format!("could not build stream: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("could not start stream: {e}"))?;
        std::thread::sleep(std::time::Duration::from_secs_f32(seconds.max(0.0)));
        Ok(())
    }
}

/// Generate `count` mono samples of a sine wave at `frequency` Hz.
///
/// Pure and deterministic, so it can be tested or written to a file without an
/// audio device.
#[must_use]
pub fn synthesize_sine(frequency: f32, sample_rate: u32, count: usize) -> Vec<f32> {
    let rate = sample_rate.max(1) as f32;
    (0..count)
        .map(|i| {
            let t = i as f32 / rate;
            (TAU * frequency * t).sin() * AMPLITUDE
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{AMPLITUDE, synthesize_sine};

    #[test]
    fn synthesize_has_the_requested_length() {
        assert_eq!(synthesize_sine(440.0, 44_100, 1000).len(), 1000);
    }

    #[test]
    fn samples_stay_within_amplitude() {
        for s in synthesize_sine(440.0, 44_100, 44_100) {
            assert!(s.abs() <= AMPLITUDE + 1e-6, "sample {s} out of range");
        }
    }

    #[test]
    fn starts_at_zero_and_is_deterministic() {
        let a = synthesize_sine(440.0, 44_100, 100);
        let b = synthesize_sine(440.0, 44_100, 100);
        assert!(a[0].abs() < 1e-6);
        assert_eq!(a, b);
    }

    #[test]
    fn a_440hz_tone_completes_one_cycle_each_period() {
        // After exactly one period (sample_rate / freq samples) it returns near zero.
        let sample_rate = 44_100u32;
        let freq = 441.0; // sample_rate / freq = 100 samples per cycle
        let samples = synthesize_sine(freq, sample_rate, 101);
        assert!(
            samples[100].abs() < 1e-2,
            "value after one cycle was {}",
            samples[100]
        );
    }
}
