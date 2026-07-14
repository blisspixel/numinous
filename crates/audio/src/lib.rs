//! Numinous audio.
//!
//! Adaptive output through `cpal`: it uses the system default output device and
//! its default configuration, so it "just works" and follows the machine's sound
//! settings on Windows (WASAPI), macOS (CoreAudio), and Linux (ALSA). The tone
//! synthesis is a pure, testable function; opening and driving the device is kept
//! separate. See `docs/SOUND.md` and `docs/ARCHITECTURE.md`.

use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// A gentle amplitude so a test tone is never harsh.
const AMPLITUDE: f32 = 0.2;

fn validate_output_dimensions(sample_rate: u32, channels: u16) -> Result<(), String> {
    if sample_rate == 0 {
        return Err("output device reported a zero sample rate".to_string());
    }
    if channels == 0 {
        return Err("output device reported zero channels".to_string());
    }
    Ok(())
}

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
        validate_output_dimensions(self.sample_rate(), self.channels())?;
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

const SOURCE_CROSSFADE_SECONDS: f32 = 0.03;
const GAIN_RAMP_SECONDS: f32 = 0.025;
const PARAMETER_RAMP_SECONDS: f32 = 0.04;
const PARAMETER_MAX_GAIN: f32 = 0.08;
const PARAMETER_MIN_FREQUENCY: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct ParameterTarget {
    root_hz: f32,
    ratio: f32,
    gain: f32,
}

/// A quiet two-oscillator voice that follows a continuously changing ratio.
///
/// Phases persist across target changes. Frequency, ratio, and gain approach
/// their targets inside the callback, so control-thread updates never restart
/// either oscillator or introduce an abrupt parameter step.
struct ParameterVoice {
    target: Option<ParameterTarget>,
    current_root_hz: f32,
    current_ratio: f32,
    current_gain: f32,
    root_phase: f32,
    ratio_phase: f32,
    sample_rate: f32,
    smoothing: f32,
    gain_step: f32,
}

impl ParameterVoice {
    fn new(sample_rate: u32) -> Self {
        let sample_rate = sample_rate.max(1) as f32;
        let ramp_frames = (PARAMETER_RAMP_SECONDS * sample_rate).max(1.0);
        Self {
            target: None,
            current_root_hz: 220.0,
            current_ratio: 1.0,
            current_gain: 0.0,
            root_phase: 0.0,
            ratio_phase: 0.0,
            sample_rate,
            smoothing: 1.0 - (-1.0 / ramp_frames).exp(),
            gain_step: PARAMETER_MAX_GAIN / ramp_frames,
        }
    }

    fn set_target(&mut self, root_hz: f32, ratio: f32, gain: f32) -> bool {
        let upper_frequency = self.sample_rate * 0.45;
        let valid = root_hz.is_finite()
            && ratio.is_finite()
            && gain.is_finite()
            && root_hz >= PARAMETER_MIN_FREQUENCY
            && ratio > 0.0
            && gain > 0.0
            && gain <= PARAMETER_MAX_GAIN
            && root_hz <= upper_frequency
            && root_hz * ratio <= upper_frequency;
        if !valid {
            self.clear_target();
            return false;
        }

        if self.target.is_none() && self.current_gain == 0.0 {
            self.current_root_hz = root_hz;
            self.current_ratio = ratio;
        }
        self.target = Some(ParameterTarget {
            root_hz,
            ratio,
            gain,
        });
        true
    }

    fn clear_target(&mut self) {
        self.target = None;
    }

    fn next_sample(&mut self) -> f32 {
        let (target_root, target_ratio, target_gain) = self
            .target
            .map_or((self.current_root_hz, self.current_ratio, 0.0), |target| {
                (target.root_hz, target.ratio, target.gain)
            });
        self.current_root_hz += (target_root - self.current_root_hz) * self.smoothing;
        self.current_ratio += (target_ratio - self.current_ratio) * self.smoothing;
        if self.current_gain < target_gain {
            self.current_gain = (self.current_gain + self.gain_step).min(target_gain);
        } else if self.current_gain > target_gain {
            self.current_gain = (self.current_gain - self.gain_step).max(target_gain);
        }

        let upper_frequency = self.sample_rate * 0.45;
        let ratio_frequency = (self.current_root_hz * self.current_ratio).min(upper_frequency);
        self.root_phase =
            (self.root_phase + TAU * self.current_root_hz / self.sample_rate).rem_euclid(TAU);
        self.ratio_phase =
            (self.ratio_phase + TAU * ratio_frequency / self.sample_rate).rem_euclid(TAU);
        let pair = (self.root_phase.sin() + self.ratio_phase.sin()) * 0.5;
        (pair * self.current_gain).clamp(-PARAMETER_MAX_GAIN, PARAMETER_MAX_GAIN)
    }
}

/// One mono or interleaved-stereo looping source.
struct LoopBuffer {
    samples: Arc<Vec<f32>>,
    sample_len: usize,
    pos: usize,
    fraction: f64,
    step: f64,
    channels: usize,
    identity: u64,
}

impl LoopBuffer {
    fn new(samples: impl Into<Arc<Vec<f32>>>, channels: usize) -> Self {
        Self::new_at_rate(samples, channels, 1, 1)
    }

    fn new_at_rate(
        samples: impl Into<Arc<Vec<f32>>>,
        channels: usize,
        source_rate: u32,
        output_rate: u32,
    ) -> Self {
        let samples = samples.into();
        let channels = channels.clamp(1, 2);
        let sample_len = samples.len() - samples.len() % channels;
        let source_rate = source_rate.max(1);
        let identity = source_identity(&samples[..sample_len], channels, source_rate);
        Self {
            samples,
            sample_len,
            pos: 0,
            fraction: 0.0,
            step: f64::from(source_rate) / f64::from(output_rate.max(1)),
            channels,
            identity,
        }
    }

    fn silent() -> Self {
        Self::new(Vec::new(), 1)
    }

    fn next_frame(&mut self) -> (f32, f32) {
        if self.sample_len == 0 {
            return (0.0, 0.0);
        }
        let frames = self.sample_len / self.channels;
        let next_pos = (self.pos + self.channels) % self.sample_len;
        let interpolate = |channel: usize| {
            let current = self.samples[self.pos + channel];
            let next = self.samples[next_pos + channel];
            current + (next - current) * self.fraction as f32
        };
        let frame = if self.channels == 2 {
            (interpolate(0), interpolate(1))
        } else {
            let value = interpolate(0);
            (value, value)
        };
        let advance = self.fraction + self.step;
        let whole_frames = (advance.floor() as u64 % frames as u64) as usize;
        self.pos = (self.pos + whole_frames * self.channels) % self.sample_len;
        self.fraction = advance.fract();
        frame
    }
}

fn source_identity(samples: &[f32], channels: usize, source_rate: u32) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET ^ channels as u64 ^ u64::from(source_rate).rotate_left(17);
    for sample in samples {
        for byte in sample.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
    }
    hash ^ samples.len() as u64
}

/// Callback-owned state. All storage is prepared by the control thread, so
/// producing a frame performs no allocation.
struct MixerState {
    current: LoopBuffer,
    previous: Option<LoopBuffer>,
    pending: Option<LoopBuffer>,
    retired: Option<LoopBuffer>,
    crossfade_frames: usize,
    crossfade_remaining: usize,
    master_gain: f32,
    current_gain: f32,
    active: bool,
    gain_step: f32,
    parameter_voice: ParameterVoice,
}

impl MixerState {
    fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(1) as f32;
        Self {
            current: LoopBuffer::silent(),
            previous: None,
            pending: None,
            retired: None,
            crossfade_frames: (SOURCE_CROSSFADE_SECONDS * rate).max(1.0) as usize,
            crossfade_remaining: 0,
            master_gain: 1.0,
            current_gain: 1.0,
            active: true,
            gain_step: 1.0 / (GAIN_RAMP_SECONDS * rate).max(1.0),
            parameter_voice: ParameterVoice::new(sample_rate),
        }
    }

    fn replace(&mut self, next: LoopBuffer) -> (bool, Option<LoopBuffer>) {
        if self.current.identity == next.identity {
            return (false, self.pending.take());
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.identity == next.identity)
        {
            return (false, None);
        }
        if self.crossfade_remaining > 0 || self.retired.is_some() {
            let superseded = self.pending.replace(next);
            return (true, superseded);
        }
        let previous = std::mem::replace(&mut self.current, next);
        self.previous = Some(previous);
        self.crossfade_remaining = self.crossfade_frames;
        (true, None)
    }

    /// Reclaim callback-retired storage and begin the newest deferred switch.
    ///
    /// This runs on the control thread. The audio callback only moves the old
    /// buffer into `retired`, so it never frees a potentially large recording.
    fn service_transitions(&mut self) -> Option<LoopBuffer> {
        let retired = self.retired.take();
        if self.crossfade_remaining == 0
            && self.previous.is_none()
            && let Some(next) = self.pending.take()
        {
            let previous = std::mem::replace(&mut self.current, next);
            self.previous = Some(previous);
            self.crossfade_remaining = self.crossfade_frames;
        }
        retired
    }

    fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = if gain.is_finite() {
            gain.clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    fn set_parameter_voice(&mut self, root_hz: f32, ratio: f32, gain: f32) -> bool {
        self.parameter_voice.set_target(root_hz, ratio, gain)
    }

    fn clear_parameter_voice(&mut self) {
        self.parameter_voice.clear_target();
    }

    fn next_frame(&mut self) -> (f32, f32) {
        let current = self.current.next_frame();
        let mixed = if self.crossfade_remaining == 0 {
            current
        } else {
            let previous = self
                .previous
                .as_mut()
                .map_or((0.0, 0.0), LoopBuffer::next_frame);
            let progress = if self.crossfade_frames <= 1 {
                1.0
            } else {
                1.0 - (self.crossfade_remaining - 1) as f32 / (self.crossfade_frames - 1) as f32
            };
            let old_gain = (1.0 - progress).sqrt();
            let new_gain = progress.sqrt();
            let gain_sum = old_gain + new_gain;
            self.crossfade_remaining -= 1;
            let mixed = (
                (previous.0 * old_gain + current.0 * new_gain) / gain_sum,
                (previous.1 * old_gain + current.1 * new_gain) / gain_sum,
            );
            if self.crossfade_remaining == 0 {
                debug_assert!(self.retired.is_none());
                self.retired = self.previous.take();
            }
            mixed
        };

        let parameter = self.parameter_voice.next_sample();
        let mixed = (mixed.0 + parameter, mixed.1 + parameter);
        let target = if self.active { self.master_gain } else { 0.0 };
        if self.current_gain < target {
            self.current_gain = (self.current_gain + self.gain_step).min(target);
        } else if self.current_gain > target {
            self.current_gain = (self.current_gain - self.gain_step).max(target);
        }
        (
            (mixed.0 * self.current_gain).clamp(-1.0, 1.0),
            (mixed.1 * self.current_gain).clamp(-1.0, 1.0),
        )
    }
}

/// Plays a sample buffer on the default device, looping, in the background.
///
/// Swap the buffer at any time with [`LoopPlayer::set_samples`] (for example
/// when the visible room changes). The stream keeps running until the player is
/// dropped.
pub struct LoopPlayer {
    _context: AudioContext,
    _stream: cpal::Stream,
    sample_rate: u32,
    state: Arc<Mutex<MixerState>>,
}

impl LoopPlayer {
    /// Open the default device and start a silent looping stream.
    ///
    /// # Errors
    /// Returns an error string if the device or stream cannot be set up.
    pub fn new() -> Result<Self, String> {
        let context = AudioContext::new()?;
        let sample_rate = context.sample_rate();
        let channel_count = context.channels();
        validate_output_dimensions(sample_rate, channel_count)?;
        let channels = channel_count as usize;
        let config: cpal::StreamConfig = context.config.clone().into();
        let state = Arc::new(Mutex::new(MixerState::new(sample_rate)));
        let callback_state = state.clone();
        let err_fn = |e| eprintln!("audio stream error: {e}");

        let stream = match context.config.sample_format() {
            cpal::SampleFormat::F32 => context.device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    if let Ok(mut s) = callback_state.lock() {
                        for frame in data.chunks_mut(channels) {
                            let (left, right) = s.next_frame();
                            for (i, out) in frame.iter_mut().enumerate() {
                                *out = if i % 2 == 0 { left } else { right };
                            }
                        }
                    } else {
                        data.fill(0.0);
                    }
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => context.device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    if let Ok(mut s) = callback_state.lock() {
                        for frame in data.chunks_mut(channels) {
                            let (left, right) = s.next_frame();
                            let (l, r) = (
                                (left * f32::from(i16::MAX)) as i16,
                                (right * f32::from(i16::MAX)) as i16,
                            );
                            for (i, out) in frame.iter_mut().enumerate() {
                                *out = if i % 2 == 0 { l } else { r };
                            }
                        }
                    } else {
                        data.fill(0);
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

        Ok(Self {
            _context: context,
            _stream: stream,
            sample_rate,
            state,
        })
    }

    /// The device sample rate, so callers can render sounds at the right pitch.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Crossfade to a mono looping buffer.
    ///
    /// Supplying the same sample content again is a no-op and preserves the
    /// playhead. New content starts at its beginning under a short crossfade.
    pub fn set_samples(&self, samples: Vec<f32>) {
        let next = LoopBuffer::new(samples, 1);
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.replace(next).1);
        drop(retired);
    }

    /// Crossfade to an interleaved-stereo looping buffer.
    ///
    /// An incomplete final frame is discarded. Supplying the same complete
    /// buffer again preserves the playhead.
    pub fn set_stereo(&self, interleaved: Vec<f32>) {
        let next = LoopBuffer::new(interleaved, 2);
        self.replace_source(next);
    }

    /// Crossfade to shared interleaved-stereo samples without copying them.
    ///
    /// This is useful when a caller must retain the same immutable source for
    /// later playback. An incomplete final frame is ignored without copying.
    pub fn set_shared_stereo(&self, interleaved: Arc<Vec<f32>>) {
        let next = LoopBuffer::new(interleaved, 2);
        self.replace_source(next);
    }

    /// Crossfade to shared stereo samples rendered at their original rate.
    /// The real-time loop interpolates them at the device rate, so high-rate
    /// devices do not require a second amplified copy of the entire track.
    pub fn set_shared_stereo_at_rate(&self, interleaved: Arc<Vec<f32>>, source_rate: u32) {
        let next = LoopBuffer::new_at_rate(interleaved, 2, source_rate, self.sample_rate);
        self.replace_source(next);
    }

    fn replace_source(&self, next: LoopBuffer) {
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.replace(next).1);
        drop(retired);
    }

    /// Reclaim source storage retired by the real-time callback.
    ///
    /// Interactive faces should call this from their ordinary update loop.
    /// Destruction then happens on the control thread, never in audio time.
    pub fn service(&self) {
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.service_transitions());
        drop(retired);
    }

    /// Set the master linear gain without replacing or restarting the source.
    /// Changes ramp over a short interval inside the audio callback.
    pub fn set_master_gain(&self, gain: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.set_master_gain(gain);
        }
    }

    /// Set a quiet continuous ratio voice over the current looping source.
    ///
    /// `root_hz` and `root_hz * ratio` must be finite, positive, and below
    /// 45 percent of the device sample rate. `gain` must be in `(0, 0.08]`.
    /// Invalid input clears the current target and returns `false`. Accepted
    /// updates preserve oscillator phases and the looping source playhead.
    pub fn set_parameter_voice(&self, root_hz: f32, ratio: f32, gain: f32) -> bool {
        self.state
            .lock()
            .is_ok_and(|mut state| state.set_parameter_voice(root_hz, ratio, gain))
    }

    /// Fade out the continuous ratio voice without changing the looping source.
    pub fn clear_parameter_voice(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.clear_parameter_voice();
        }
    }

    /// Fade output in or out without replacing the source or its playhead.
    ///
    /// The source clock continues while inactive, which keeps radio and room
    /// playback stable across a temporary focus change.
    pub fn set_active(&self, active: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.active = active;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        AMPLITUDE, LoopBuffer, MixerState, PARAMETER_MAX_GAIN, synthesize_sine,
        validate_output_dimensions,
    };

    #[test]
    fn output_dimensions_reject_degenerate_device_configs() {
        assert!(validate_output_dimensions(44_100, 2).is_ok());
        assert_eq!(
            validate_output_dimensions(0, 2).expect_err("zero rate must fail"),
            "output device reported a zero sample rate"
        );
        assert_eq!(
            validate_output_dimensions(44_100, 0).expect_err("zero channels must fail"),
            "output device reported zero channels"
        );
    }

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

    #[test]
    fn read_frame_wraps_handles_empty_and_speaks_stereo() {
        let mut empty = LoopBuffer::silent();
        assert_eq!(empty.next_frame(), (0.0, 0.0));

        let mut mono = LoopBuffer::new(vec![0.1, 0.2, 0.3], 1);
        assert_eq!(mono.next_frame(), (0.1, 0.1), "mono fills both ears");
        assert_eq!(mono.next_frame(), (0.2, 0.2));
        assert_eq!(mono.next_frame(), (0.3, 0.3));
        assert_eq!(mono.next_frame(), (0.1, 0.1), "and wraps");

        let mut stereo = LoopBuffer::new(vec![0.1, -0.1, 0.2, -0.2], 2);
        assert_eq!(stereo.next_frame(), (0.1, -0.1), "left and right differ");
        assert_eq!(stereo.next_frame(), (0.2, -0.2));
        assert_eq!(stereo.next_frame(), (0.1, -0.1), "wraps on frames");

        let odd = LoopBuffer::new(vec![0.1, -0.1, 9.0], 2);
        assert_eq!(odd.sample_len, 2);
    }

    #[test]
    fn shared_loop_buffer_reuses_complete_stereo_storage() {
        let samples = Arc::new(vec![0.1, -0.1, 0.2, -0.2]);
        let buffer = LoopBuffer::new(samples.clone(), 2);

        assert!(Arc::ptr_eq(&buffer.samples, &samples));
    }

    #[test]
    fn source_rate_conversion_interpolates_without_allocating_an_output_copy() {
        let samples = Arc::new(vec![0.0, 0.0, 1.0, -1.0]);
        let mut buffer = LoopBuffer::new_at_rate(samples.clone(), 2, 2, 4);

        assert_eq!(buffer.next_frame(), (0.0, 0.0));
        assert_eq!(buffer.next_frame(), (0.5, -0.5));
        assert_eq!(buffer.next_frame(), (1.0, -1.0));
        assert_eq!(buffer.next_frame(), (0.5, -0.5));
        assert!(Arc::ptr_eq(&buffer.samples, &samples));
    }

    #[test]
    fn identical_source_is_a_no_op_and_gain_does_not_reset_playhead() {
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        let mut mixer = MixerState::new(1_000);
        assert!(mixer.replace(LoopBuffer::new(samples.clone(), 1)).0);
        for _ in 0..mixer.crossfade_frames + 2 {
            let _ = mixer.next_frame();
        }
        let retired = mixer.service_transitions();
        assert!(retired.is_some(), "retired storage leaves the callback");
        let before = mixer.current.pos;
        let (changed, superseded) = mixer.replace(LoopBuffer::new(samples, 1));
        assert!(!changed);
        assert!(superseded.is_none());
        assert_eq!(mixer.current.pos, before);

        mixer.set_master_gain(0.25);
        let _ = mixer.next_frame();
        assert_eq!(
            mixer.current.pos,
            (before + 1) % mixer.current.samples.len()
        );
    }

    #[test]
    fn source_changes_crossfade_without_leaving_bounds() {
        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(LoopBuffer::new(vec![0.8; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();
        assert!(mixer.replace(LoopBuffer::new(vec![-0.8; 64], 1)).0);
        let frames: Vec<_> = (0..mixer.crossfade_frames + 1)
            .map(|_| mixer.next_frame())
            .collect();
        assert!(
            frames.iter().all(|(left, right)| {
                (-1.0..=1.0).contains(left) && (-1.0..=1.0).contains(right)
            })
        );
        assert!(frames.first().is_some_and(|frame| frame.0 > 0.7));
        assert!(frames.last().is_some_and(|frame| frame.0 < -0.7));
        let endpoint = &frames[frames.len() - 2..];
        assert!((endpoint[0].0 - endpoint[1].0).abs() < 1.0e-6);
        assert!((endpoint[0].1 - endpoint[1].1).abs() < 1.0e-6);
    }

    #[test]
    fn callback_retires_storage_and_control_thread_starts_latest_pending_source() {
        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(LoopBuffer::new(vec![0.2; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let retired_silence = mixer.service_transitions();
        assert!(retired_silence.is_some());

        let _ = mixer.replace(LoopBuffer::new(vec![0.6; 64], 1));
        let _ = mixer.next_frame();
        let _ = mixer.replace(LoopBuffer::new(vec![0.8; 64], 1));
        assert!(mixer.pending.is_some());
        while mixer.crossfade_remaining > 0 {
            let _ = mixer.next_frame();
        }
        assert!(mixer.retired.is_some());
        assert!(mixer.previous.is_none());

        let retired = mixer.service_transitions();
        assert!(retired.is_some());
        assert!(mixer.pending.is_none());
        assert!(mixer.previous.is_some());
        assert_eq!(mixer.crossfade_remaining, mixer.crossfade_frames);
        let first = mixer.next_frame();
        assert!((first.0 - 0.6).abs() < 1.0e-6);
    }

    #[test]
    fn normalized_crossfade_does_not_clip_correlated_full_scale_sources() {
        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(LoopBuffer::new(vec![1.0; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let frame = mixer.next_frame();
            assert!(frame.0 <= 1.0 && frame.1 <= 1.0);
        }
        let _ = mixer.service_transitions();
        let _ = mixer.replace(LoopBuffer::new(vec![1.0; 65], 1));
        for _ in 0..mixer.crossfade_frames {
            let frame = mixer.next_frame();
            assert!((frame.0 - 1.0).abs() < 1.0e-6);
            assert!((frame.1 - 1.0).abs() < 1.0e-6);
        }
    }

    #[test]
    fn active_state_ramps_gain_while_the_source_clock_continues() {
        let sample_rate = 1_000;
        let mut mixer = MixerState::new(sample_rate);
        let _ = mixer.replace(LoopBuffer::new(vec![0.5; 257], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let before = mixer.current.pos;
        mixer.active = false;
        let ramp_frames = (super::GAIN_RAMP_SECONDS * sample_rate as f32) as usize;
        let fade: Vec<f32> = (0..ramp_frames + 2).map(|_| mixer.next_frame().0).collect();
        assert!(fade.windows(2).all(|pair| pair[1] <= pair[0]));
        assert_eq!(fade.last(), Some(&0.0));
        assert_eq!(
            mixer.current.pos,
            (before + ramp_frames + 2) % mixer.current.samples.len()
        );

        mixer.active = true;
        let rise: Vec<f32> = (0..ramp_frames + 2).map(|_| mixer.next_frame().0).collect();
        assert!(rise.windows(2).all(|pair| pair[1] >= pair[0]));
        assert_eq!(rise.last(), Some(&0.5));
    }

    #[test]
    fn invalid_gain_fails_silent_and_finite_gain_clamps() {
        let mut mixer = MixerState::new(1_000);
        mixer.set_master_gain(f32::NAN);
        assert_eq!(mixer.master_gain, 0.0);
        mixer.set_master_gain(3.0);
        assert_eq!(mixer.master_gain, 1.0);
    }

    #[test]
    fn parameter_target_changes_without_restarting_the_base_playhead() {
        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(LoopBuffer::new(vec![0.1, 0.2, 0.3, 0.4], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();
        let before = mixer.current.pos;

        assert!(mixer.set_parameter_voice(110.0, 1.5, 0.04));
        assert_eq!(mixer.current.pos, before);
        let _ = mixer.next_frame();
        assert_eq!(mixer.current.pos, (before + 1) % 4);

        let before_change = mixer.current.pos;
        assert!(mixer.set_parameter_voice(110.0, 2.0, 0.04));
        assert_eq!(mixer.current.pos, before_change);
        let _ = mixer.next_frame();
        assert_eq!(mixer.current.pos, (before_change + 1) % 4);
    }

    #[test]
    fn parameter_transitions_are_smooth_low_and_bounded() {
        let mut mixer = MixerState::new(4_000);
        assert!(mixer.set_parameter_voice(110.0, 1.5, PARAMETER_MAX_GAIN));
        let first: Vec<_> = (0..300).map(|_| mixer.next_frame().0).collect();
        assert!(
            first
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= PARAMETER_MAX_GAIN)
        );

        let before = *first.last().expect("first voice sample");
        assert!(mixer.set_parameter_voice(220.0, 1.25, PARAMETER_MAX_GAIN));
        let after = mixer.next_frame().0;
        assert!(
            (after - before).abs() < 0.03,
            "parameter update stepped by {}",
            (after - before).abs()
        );
        for _ in 0..1_000 {
            let sample = mixer.next_frame().0;
            assert!(sample.is_finite());
            assert!(sample.abs() <= PARAMETER_MAX_GAIN);
        }
    }

    #[test]
    fn parameter_voice_stores_the_exact_target_ratio() {
        let mut mixer = MixerState::new(48_000);
        assert!(mixer.set_parameter_voice(146.83, 1.5, 0.04));
        assert_eq!(
            mixer.parameter_voice.target.expect("accepted target").ratio,
            1.5
        );
    }

    #[test]
    fn invalid_parameter_target_fails_closed() {
        let mut mixer = MixerState::new(48_000);
        assert!(mixer.set_parameter_voice(220.0, 1.5, 0.04));
        assert!(!mixer.set_parameter_voice(f32::NAN, 1.5, 0.04));
        assert!(mixer.parameter_voice.target.is_none());
        assert!(!mixer.set_parameter_voice(220.0, f32::INFINITY, 0.04));
        assert!(!mixer.set_parameter_voice(220.0, 1.5, PARAMETER_MAX_GAIN * 2.0));
        assert!(!mixer.set_parameter_voice(30_000.0, 1.0, 0.04));
        for _ in 0..2_000 {
            assert!(mixer.next_frame().0.is_finite());
        }
        assert_eq!(mixer.parameter_voice.current_gain, 0.0);
    }

    #[test]
    fn parameter_voice_does_not_disturb_crossfade_retirement() {
        let mut mixer = MixerState::new(1_000);
        assert!(mixer.set_parameter_voice(110.0, 1.5, 0.04));
        let _ = mixer.replace(LoopBuffer::new(vec![0.2; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let frame = mixer.next_frame();
            assert!((-1.0..=1.0).contains(&frame.0));
            assert!((-1.0..=1.0).contains(&frame.1));
        }
        let retired = mixer.service_transitions();
        assert!(retired.is_some());

        let _ = mixer.replace(LoopBuffer::new(vec![0.6; 64], 1));
        while mixer.crossfade_remaining > 0 {
            let _ = mixer.next_frame();
        }
        assert!(mixer.retired.is_some());
        assert!(mixer.previous.is_none());
        assert!(mixer.service_transitions().is_some());
    }
}
