//! Numinous audio.
//!
//! Adaptive output through `cpal`: it uses the system default output device and
//! its default configuration, so it "just works" and follows the machine's sound
//! settings on Windows (WASAPI), macOS (CoreAudio), and Linux (ALSA). The tone
//! synthesis is a pure, testable function; opening and driving the device is kept
//! separate. An optional fixed capture ring taps the mixed output for the
//! visualizer path, and [`capture`] can open a loopback-like input when the OS
//! exposes one. See `docs/SOUND.md` and `docs/ARCHITECTURE.md`.

use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub mod capture;
pub use capture::{CaptureRing, InputCapture, looks_like_loopback_name};

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

fn device_channel_sample(frame: (f32, f32), channels: usize, channel: usize) -> f32 {
    if channels == 1 {
        ((frame.0 + frame.1) * std::f32::consts::FRAC_1_SQRT_2).clamp(-1.0, 1.0)
    } else if channel.is_multiple_of(2) {
        frame.0
    } else {
        frame.1
    }
}

fn build_tone_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    mut next: impl FnMut() -> f32 + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device.build_output_stream(
        config,
        move |data: &mut [T], _| fill_tone_samples(data, channels, &mut next),
        |error| eprintln!("audio stream error: {error}"),
        None,
    )
}

fn fill_tone_samples<T>(data: &mut [T], channels: usize, next: &mut impl FnMut() -> f32)
where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    for frame in data.chunks_mut(channels) {
        let value = T::from_sample(next());
        frame.fill(value);
    }
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
        self.device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// The device's default sample rate in Hz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate()
    }

    /// The device's default channel count.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.config.channels()
    }

    /// Play a sine tone of `frequency` Hz for `seconds` on the default device.
    ///
    /// Blocks for the duration, then stops. Adapts to every PCM sample format
    /// exposed by the device.
    ///
    /// # Errors
    /// Returns an error string if the stream cannot be built or started, or if
    /// the device uses an unsupported sample format.
    pub fn play_tone(&self, frequency: f32, seconds: f32) -> Result<(), String> {
        validate_output_dimensions(self.sample_rate(), self.channels())?;
        let sample_rate = self.sample_rate() as f32;
        let channels = self.channels() as usize;
        let config: cpal::StreamConfig = self.config.into();
        let mut phase = 0.0f32;
        let next = move || {
            let value = (TAU * frequency * phase / sample_rate).sin() * AMPLITUDE;
            phase += 1.0;
            value
        };

        let stream = match self.config.sample_format() {
            cpal::SampleFormat::I8 => build_tone_stream::<i8>(&self.device, config, channels, next),
            cpal::SampleFormat::I16 => {
                build_tone_stream::<i16>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::I24 => {
                build_tone_stream::<cpal::I24>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::I32 => {
                build_tone_stream::<i32>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::I64 => {
                build_tone_stream::<i64>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::U8 => build_tone_stream::<u8>(&self.device, config, channels, next),
            cpal::SampleFormat::U16 => {
                build_tone_stream::<u16>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::U24 => {
                build_tone_stream::<cpal::U24>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::U32 => {
                build_tone_stream::<u32>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::U64 => {
                build_tone_stream::<u64>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::F32 => {
                build_tone_stream::<f32>(&self.device, config, channels, next)
            }
            cpal::SampleFormat::F64 => {
                build_tone_stream::<f64>(&self.device, config, channels, next)
            }
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
const MIN_REQUESTED_CROSSFADE_SECONDS: f32 = 0.005;
const MAX_REQUESTED_CROSSFADE_SECONDS: f32 = 2.0;
const GAIN_RAMP_SECONDS: f32 = 0.025;
const PARAMETER_RAMP_SECONDS: f32 = 0.04;
const PARAMETER_MAX_GAIN: f32 = 0.08;
const PARAMETER_MIN_FREQUENCY: f32 = 20.0;

fn crossfade_frame_count(sample_rate: u32, seconds: f32) -> Option<usize> {
    if sample_rate == 0
        || !seconds.is_finite()
        || !(MIN_REQUESTED_CROSSFADE_SECONDS..=MAX_REQUESTED_CROSSFADE_SECONDS).contains(&seconds)
    {
        return None;
    }
    Some((seconds * sample_rate as f32).max(1.0) as usize)
}

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
    identity: SourceIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceIdentity {
    Content(u64),
    SharedAllocation {
        allocation: usize,
        sample_len: usize,
        channels: usize,
        source_rate: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentityKind {
    Content,
    SharedAllocation,
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
        Self::from_samples(
            samples.into(),
            channels,
            source_rate,
            output_rate,
            IdentityKind::Content,
        )
    }

    fn new_shared_at_rate(
        samples: Arc<Vec<f32>>,
        channels: usize,
        source_rate: u32,
        output_rate: u32,
    ) -> Self {
        Self::from_samples(
            samples,
            channels,
            source_rate,
            output_rate,
            IdentityKind::SharedAllocation,
        )
    }

    fn from_samples(
        samples: Arc<Vec<f32>>,
        channels: usize,
        source_rate: u32,
        output_rate: u32,
        identity_kind: IdentityKind,
    ) -> Self {
        let channels = channels.clamp(1, 2);
        let sample_len = samples.len() - samples.len() % channels;
        let source_rate = source_rate.max(1);
        let identity = match identity_kind {
            IdentityKind::Content => SourceIdentity::Content(source_identity(
                &samples[..sample_len],
                channels,
                source_rate,
            )),
            IdentityKind::SharedAllocation => {
                shared_source_identity(&samples, sample_len, channels, source_rate)
            }
        };
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

fn shared_source_identity(
    samples: &Arc<Vec<f32>>,
    sample_len: usize,
    channels: usize,
    source_rate: u32,
) -> SourceIdentity {
    // Every comparable identity belongs to a LoopBuffer that retains this Arc,
    // so the allocator cannot reuse the address while the identity is live.
    SourceIdentity::SharedAllocation {
        allocation: Arc::as_ptr(samples) as usize,
        sample_len,
        channels,
        source_rate,
    }
}

/// One-shot overlay: play once, then drop. Control thread supplies samples.
struct OneshotPlay {
    samples: Arc<Vec<f32>>,
    pos: usize,
    gain: f32,
    channels: usize,
}

impl OneshotPlay {
    fn new(samples: Vec<f32>, gain: f32, channels: usize) -> Option<Self> {
        if samples.len() < channels {
            return None;
        }
        let gain = if gain.is_finite() {
            gain.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (gain > 0.0).then(|| Self {
            samples: Arc::new(samples),
            pos: 0,
            gain,
            channels,
        })
    }

    fn exhausted(&self) -> bool {
        self.samples.len().saturating_sub(self.pos) < self.channels
    }
}

/// Callback-owned state. All storage is prepared by the control thread, so
/// producing a frame performs no allocation.
struct PendingSource {
    buffer: LoopBuffer,
    crossfade_frames: usize,
}

/// At most two prepared buffers can be displaced by one control request: the
/// rejected input and the prior pending source. The fixed bundle is returned
/// through the mutex boundary so their storage is never destroyed under lock.
#[derive(Default)]
struct RetiredBuffers {
    first: Option<LoopBuffer>,
    second: Option<LoopBuffer>,
}

impl RetiredBuffers {
    fn one(first: LoopBuffer) -> Self {
        Self {
            first: Some(first),
            second: None,
        }
    }

    fn two(first: LoopBuffer, second: Option<LoopBuffer>) -> Self {
        Self {
            first: Some(first),
            second,
        }
    }

    fn retire(self) {
        let Self { first, second } = self;
        drop(first);
        drop(second);
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.first.is_none() && self.second.is_none()
    }
}

/// One callback-owned outgoing voice. An interrupted transition can preserve
/// its exact audible mix with two advancing sources, while repeated interrupts
/// wait behind the short default fade so callback work stays strictly bounded.
struct SourceMix {
    primary: LoopBuffer,
    secondary: Option<LoopBuffer>,
    primary_gain: f32,
    secondary_gain: f32,
}

impl SourceMix {
    fn single(primary: LoopBuffer) -> Self {
        Self {
            primary,
            secondary: None,
            primary_gain: 1.0,
            secondary_gain: 0.0,
        }
    }

    fn is_single(&self) -> bool {
        self.secondary.is_none()
    }

    fn from_interrupted_transition(
        mut previous: Self,
        current: LoopBuffer,
        old_gain: f32,
        new_gain: f32,
    ) -> Self {
        debug_assert!(previous.is_single());
        previous.primary_gain *= old_gain;
        previous.secondary = Some(current);
        previous.secondary_gain = new_gain;
        previous
    }

    fn next_frame(&mut self) -> (f32, f32) {
        let primary = self.primary.next_frame();
        let secondary = self
            .secondary
            .as_mut()
            .map_or((0.0, 0.0), LoopBuffer::next_frame);
        (
            primary
                .0
                .mul_add(self.primary_gain, secondary.0 * self.secondary_gain),
            primary
                .1
                .mul_add(self.primary_gain, secondary.1 * self.secondary_gain),
        )
    }
}

struct MixerState {
    current: LoopBuffer,
    previous: Option<SourceMix>,
    pending: Option<PendingSource>,
    retired: Option<SourceMix>,
    default_crossfade_frames: usize,
    crossfade_frames: usize,
    crossfade_remaining: usize,
    /// Same-target interruption ramps existing coefficients to `(0, 1)`.
    /// Other transitions use the ordinary equal-power law.
    coefficient_ramp_start: Option<(f32, f32)>,
    master_gain: f32,
    current_gain: f32,
    active: bool,
    gain_step: f32,
    parameter_voice: ParameterVoice,
    oneshot: Option<OneshotPlay>,
    /// Optional visualizer tap of the post-gain mixed frame.
    output_tap: Option<Arc<Mutex<CaptureRing>>>,
}

impl MixerState {
    fn new(sample_rate: u32) -> Self {
        let rate = sample_rate.max(1) as f32;
        let default_crossfade_frames = (SOURCE_CROSSFADE_SECONDS * rate).max(1.0) as usize;
        Self {
            current: LoopBuffer::silent(),
            previous: None,
            pending: None,
            retired: None,
            default_crossfade_frames,
            crossfade_frames: default_crossfade_frames,
            crossfade_remaining: 0,
            coefficient_ramp_start: None,
            master_gain: 1.0,
            current_gain: 1.0,
            active: true,
            gain_step: 1.0 / (GAIN_RAMP_SECONDS * rate).max(1.0),
            parameter_voice: ParameterVoice::new(sample_rate),
            oneshot: None,
            output_tap: None,
        }
    }

    /// Install prepared storage and return the superseded one-shot for
    /// destruction after the caller releases the callback mutex.
    fn replace_oneshot(&mut self, next: OneshotPlay) -> Option<OneshotPlay> {
        self.oneshot.replace(next)
    }

    fn service_oneshot(&mut self) -> Option<OneshotPlay> {
        self.oneshot
            .as_ref()
            .is_some_and(OneshotPlay::exhausted)
            .then(|| self.oneshot.take())
            .flatten()
    }

    fn clear_oneshot(&mut self) -> Option<OneshotPlay> {
        self.oneshot.take()
    }

    fn next_oneshot_frame(&mut self) -> (f32, f32) {
        let Some(play) = self.oneshot.as_mut() else {
            return (0.0, 0.0);
        };
        if play.exhausted() {
            // Keep exhausted storage in the slot. The next control-thread
            // enqueue replaces and destroys it without freeing memory here.
            return (0.0, 0.0);
        }
        let left = play.samples[play.pos] * play.gain;
        let right = if play.channels == 1 {
            left
        } else {
            play.samples[play.pos + 1] * play.gain
        };
        play.pos += play.channels;
        (left, right)
    }

    fn replace(&mut self, next: LoopBuffer) -> (bool, RetiredBuffers) {
        if self.current.identity == next.identity {
            if self.crossfade_remaining > 0
                && self.retired.is_none()
                && self.previous.as_ref().is_some_and(SourceMix::is_single)
            {
                let (old_gain, new_gain) = self.crossfade_gains();
                self.crossfade_frames = self.default_crossfade_frames;
                self.crossfade_remaining = self.crossfade_frames;
                self.coefficient_ramp_start = Some((old_gain, new_gain));
                return (
                    true,
                    RetiredBuffers::two(next, self.pending.take().map(|pending| pending.buffer)),
                );
            }
            return (
                false,
                RetiredBuffers::two(next, self.pending.take().map(|pending| pending.buffer)),
            );
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.buffer.identity == next.identity)
        {
            return (false, RetiredBuffers::one(next));
        }

        if self.crossfade_remaining > 0
            && self.retired.is_none()
            && self.previous.as_ref().is_some_and(SourceMix::is_single)
        {
            let (old_gain, new_gain) = self.crossfade_gains();
            let previous = self.previous.take().expect("active transition source");
            let current = std::mem::replace(&mut self.current, next);
            self.previous = Some(SourceMix::from_interrupted_transition(
                previous, current, old_gain, new_gain,
            ));
            self.crossfade_frames = self.default_crossfade_frames;
            self.crossfade_remaining = self.crossfade_frames;
            self.coefficient_ramp_start = None;
            let superseded = self.pending.take().map(|pending| pending.buffer);
            let superseded = superseded.map_or_else(RetiredBuffers::default, RetiredBuffers::one);
            return (true, superseded);
        }

        self.replace_with_crossfade(next, self.default_crossfade_frames)
    }

    fn replace_with_crossfade(
        &mut self,
        next: LoopBuffer,
        crossfade_frames: usize,
    ) -> (bool, RetiredBuffers) {
        if self.current.identity == next.identity {
            return (
                false,
                RetiredBuffers::two(next, self.pending.take().map(|pending| pending.buffer)),
            );
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.buffer.identity == next.identity)
        {
            return (false, RetiredBuffers::one(next));
        }
        let crossfade_frames = crossfade_frames.max(1);
        if self.crossfade_remaining > 0 || self.retired.is_some() {
            let superseded = self.pending.replace(PendingSource {
                buffer: next,
                crossfade_frames,
            });
            let superseded = superseded.map(|pending| pending.buffer);
            return (
                true,
                superseded.map_or_else(RetiredBuffers::default, RetiredBuffers::one),
            );
        }
        let previous = std::mem::replace(&mut self.current, next);
        self.previous = Some(SourceMix::single(previous));
        self.crossfade_frames = crossfade_frames;
        self.crossfade_remaining = self.crossfade_frames;
        self.coefficient_ramp_start = None;
        (true, RetiredBuffers::default())
    }

    /// Reclaim callback-retired storage and begin the newest deferred switch.
    ///
    /// This runs on the control thread. The audio callback only moves the old
    /// buffer into `retired`, so it never frees a potentially large recording.
    fn service_transitions(&mut self) -> Option<SourceMix> {
        let retired = self.retired.take();
        if self.crossfade_remaining == 0
            && self.previous.is_none()
            && let Some(pending) = self.pending.take()
        {
            let previous = std::mem::replace(&mut self.current, pending.buffer);
            self.previous = Some(SourceMix::single(previous));
            self.crossfade_frames = pending.crossfade_frames;
            self.crossfade_remaining = self.crossfade_frames;
            self.coefficient_ramp_start = None;
        }
        retired
    }

    fn crossfade_gains(&self) -> (f32, f32) {
        let progress = if self.crossfade_frames <= 1 {
            1.0
        } else {
            1.0 - (self.crossfade_remaining - 1) as f32 / (self.crossfade_frames - 1) as f32
        };
        self.coefficient_ramp_start.map_or_else(
            || ((1.0 - progress).sqrt(), progress.sqrt()),
            |(old_start, new_start)| {
                (
                    old_start * (1.0 - progress),
                    new_start + (1.0 - new_start) * progress,
                )
            },
        )
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
                .map_or((0.0, 0.0), SourceMix::next_frame);
            let (old_gain, new_gain) = self.crossfade_gains();
            self.crossfade_remaining -= 1;
            let mixed = (
                previous.0.mul_add(old_gain, current.0 * new_gain),
                previous.1.mul_add(old_gain, current.1 * new_gain),
            );
            if self.crossfade_remaining == 0 {
                self.coefficient_ramp_start = None;
                debug_assert!(self.retired.is_none());
                self.retired = self.previous.take();
            }
            mixed
        };

        let parameter = self.parameter_voice.next_sample();
        let oneshot = self.next_oneshot_frame();
        let mixed = (
            mixed.0 + parameter + oneshot.0,
            mixed.1 + parameter + oneshot.1,
        );
        let target = if self.active { self.master_gain } else { 0.0 };
        if self.current_gain < target {
            self.current_gain = (self.current_gain + self.gain_step).min(target);
        } else if self.current_gain > target {
            self.current_gain = (self.current_gain - self.gain_step).max(target);
        }
        let left = (mixed.0 * self.current_gain).clamp(-1.0, 1.0);
        let right = (mixed.1 * self.current_gain).clamp(-1.0, 1.0);
        if let Some(tap) = self.output_tap.as_ref()
            && let Ok(mut ring) = tap.try_lock()
        {
            ring.push_frame(left, right);
        }
        (left, right)
    }
}

fn build_loop_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    state: Arc<Mutex<MixerState>>,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            if let Ok(mut state) = state.lock() {
                for frame in data.chunks_mut(channels) {
                    let mixed = state.next_frame();
                    for (channel, output) in frame.iter_mut().enumerate() {
                        *output = T::from_sample(device_channel_sample(mixed, channels, channel));
                    }
                }
            } else {
                data.fill(T::from_sample(0.0));
            }
        },
        |error| eprintln!("audio stream error: {error}"),
        None,
    )
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
    /// Mixed-output tap for the visualizer (always present; may be empty early).
    output_tap: Arc<Mutex<CaptureRing>>,
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
        let config: cpal::StreamConfig = context.config.into();
        let output_tap = Arc::new(Mutex::new(CaptureRing::new(4_096, sample_rate)));
        let mut mixer = MixerState::new(sample_rate);
        mixer.output_tap = Some(Arc::clone(&output_tap));
        let state = Arc::new(Mutex::new(mixer));

        let stream = match context.config.sample_format() {
            cpal::SampleFormat::I8 => {
                build_loop_stream::<i8>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::I16 => {
                build_loop_stream::<i16>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::I24 => {
                build_loop_stream::<cpal::I24>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::I32 => {
                build_loop_stream::<i32>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::I64 => {
                build_loop_stream::<i64>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::U8 => {
                build_loop_stream::<u8>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::U16 => {
                build_loop_stream::<u16>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::U24 => {
                build_loop_stream::<cpal::U24>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::U32 => {
                build_loop_stream::<u32>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::U64 => {
                build_loop_stream::<u64>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::F32 => {
                build_loop_stream::<f32>(&context.device, config, channels, state.clone())
            }
            cpal::SampleFormat::F64 => {
                build_loop_stream::<f64>(&context.device, config, channels, state.clone())
            }
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
            output_tap,
        })
    }

    /// The device sample rate, so callers can render sounds at the right pitch.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Snapshot recent mixed-output frames for the visualizer (interleaved stereo).
    ///
    /// Empty when the stream has not yet produced enough audio. Always safe to
    /// call from the control thread; never blocks the audio callback long-term.
    #[must_use]
    pub fn snapshot_output_tap(&self, max_frames: usize) -> Vec<f32> {
        self.output_tap
            .lock()
            .map(|ring| ring.snapshot_frames(max_frames))
            .unwrap_or_default()
    }

    /// Crossfade to a mono looping buffer.
    ///
    /// Supplying the same sample content again is a no-op and preserves the
    /// playhead. New content starts at its beginning under a short crossfade.
    /// A default replacement interrupts a longer requested transition from its
    /// exact current audible mix, with repeated interruption bounded by one
    /// additional default fade.
    pub fn set_samples(&self, samples: Vec<f32>) {
        let next = LoopBuffer::new(samples, 1);
        self.replace_source(next);
    }

    /// Crossfade to mono samples rendered at their original rate.
    ///
    /// The real-time loop interpolates them at the device rate, so a bounded
    /// source does not need to allocate again for a high-rate output device.
    /// Content and source rate together form the replacement identity.
    pub fn set_samples_at_rate(&self, samples: Vec<f32>, source_rate: u32) {
        let next = LoopBuffer::new_at_rate(samples, 1, source_rate, self.sample_rate);
        self.replace_source(next);
    }

    /// Crossfade to a mono looping buffer over a caller-selected duration.
    ///
    /// Accepted durations are finite and between 5 milliseconds and 2 seconds.
    /// Invalid durations leave current and pending sources unchanged and return
    /// `false`. Each deferred source retains its own requested duration.
    pub fn set_samples_with_crossfade(&self, samples: Vec<f32>, seconds: f32) -> bool {
        let Some(frames) = crossfade_frame_count(self.sample_rate, seconds) else {
            return false;
        };
        let next = LoopBuffer::new(samples, 1);
        self.replace_source_with_crossfade(next, frames)
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
    /// later playback. Reusing the same shared allocation preserves the
    /// playhead. An incomplete final frame is ignored without copying.
    pub fn set_shared_stereo(&self, interleaved: Arc<Vec<f32>>) {
        let next = LoopBuffer::new_shared_at_rate(interleaved, 2, 1, 1);
        self.replace_source(next);
    }

    /// Crossfade to shared stereo samples rendered at their original rate.
    /// The real-time loop interpolates them at the device rate, so high-rate
    /// devices do not require a second amplified copy of the entire track.
    /// Reusing the same allocation and source rate preserves the playhead.
    pub fn set_shared_stereo_at_rate(&self, interleaved: Arc<Vec<f32>>, source_rate: u32) {
        let next = LoopBuffer::new_shared_at_rate(interleaved, 2, source_rate, self.sample_rate);
        self.replace_source(next);
    }

    fn replace_source(&self, next: LoopBuffer) {
        let retired = self
            .state
            .lock()
            .ok()
            .map(|mut state| state.replace(next).1);
        if let Some(retired) = retired {
            retired.retire();
        }
    }

    fn replace_source_with_crossfade(&self, next: LoopBuffer, frames: usize) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        let (changed, retired) = state.replace_with_crossfade(next, frames);
        drop(state);
        retired.retire();
        changed
    }

    /// Reclaim source storage retired by the real-time callback.
    ///
    /// Interactive faces should call this from their ordinary update loop.
    /// Destruction then happens on the control thread, never in audio time.
    pub fn service(&self) {
        let (retired_source, retired_oneshot) = self.state.lock().map_or_else(
            |_| (None, None),
            |mut state| (state.service_transitions(), state.service_oneshot()),
        );
        drop(retired_source);
        drop(retired_oneshot);
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

    /// Play a mono one-shot over the looping source without restarting it.
    ///
    /// Samples are consumed once at the device rate. A new oneshot replaces any
    /// unfinished previous oneshot. Empty or non-finite gain is ignored.
    pub fn play_oneshot(&self, samples: Vec<f32>, gain: f32) {
        let Some(next) = OneshotPlay::new(samples, gain, 1) else {
            return;
        };
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.replace_oneshot(next));
        drop(retired);
    }

    /// Play interleaved-stereo samples once without restarting the loop.
    ///
    /// Samples are consumed once at the device rate. An incomplete final frame
    /// is ignored. A new one-shot replaces any unfinished previous one-shot.
    pub fn play_stereo_oneshot(&self, samples: Vec<f32>, gain: f32) {
        let Some(next) = OneshotPlay::new(samples, gain, 2) else {
            return;
        };
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.replace_oneshot(next));
        drop(retired);
    }

    /// Stop and retire the current one-shot without changing the loop.
    pub fn clear_oneshot(&self) {
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.clear_oneshot());
        drop(retired);
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

/// Source of spectrum samples for the App visualizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizerSource {
    /// No audio available yet.
    Silent,
    /// Mixed LoopPlayer output tap (what Numinous is playing).
    OutputMix,
    /// System loopback-like input device.
    Loopback,
    /// Deterministic room-bed arrangement analysis.
    RoomBed,
}

impl VisualizerSource {
    /// Short HUD label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Silent => "SILENT",
            Self::OutputMix => "OUTPUT MIX",
            Self::Loopback => "LOOPBACK",
            Self::RoomBed => "ROOM BED",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        AMPLITUDE, LoopBuffer, MixerState, OneshotPlay, PARAMETER_MAX_GAIN, crossfade_frame_count,
        device_channel_sample, fill_tone_samples, synthesize_sine, validate_output_dimensions,
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
    fn device_channel_projection_downmixes_mono_and_preserves_stereo() {
        let frame = (0.6, 0.2);
        assert!(
            (device_channel_sample(frame, 1, 0) - 0.8 * std::f32::consts::FRAC_1_SQRT_2).abs()
                < 1.0e-6
        );
        assert_eq!(device_channel_sample(frame, 2, 0), 0.6);
        assert_eq!(device_channel_sample(frame, 2, 1), 0.2);
        assert_eq!(device_channel_sample(frame, 4, 2), 0.6);
        assert_eq!(device_channel_sample(frame, 4, 3), 0.2);
        assert_eq!(device_channel_sample((1.0, 1.0), 1, 0), 1.0);
    }

    #[test]
    fn pcm_output_conversion_preserves_frames_and_unsigned_silence() {
        let mut frame_index = 0_u8;
        let mut next = || {
            let value = if frame_index == 0 { 0.0 } else { 0.5 };
            frame_index += 1;
            value
        };
        let mut floating = [0.0_f32; 4];
        fill_tone_samples(&mut floating, 2, &mut next);
        assert_eq!(floating, [0.0, 0.0, 0.5, 0.5]);

        let mut unsigned = [0_u16; 2];
        fill_tone_samples(&mut unsigned, 2, &mut || 0.0);
        assert_eq!(unsigned, [32_768, 32_768]);
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
    fn shared_loop_identity_is_constant_time_and_tracks_the_allocation() {
        let samples = Arc::new(vec![0.1, -0.1, 0.2, -0.2]);
        let retained = Arc::downgrade(&samples);
        let first = LoopBuffer::new_shared_at_rate(samples.clone(), 2, 16_000, 48_000);
        let repeated = LoopBuffer::new_shared_at_rate(samples, 2, 16_000, 48_000);
        let equal_content =
            LoopBuffer::new_shared_at_rate(Arc::new(vec![0.1, -0.1, 0.2, -0.2]), 2, 16_000, 48_000);

        assert_eq!(first.identity, repeated.identity);
        assert_ne!(first.identity, equal_content.identity);
        assert!(
            retained.upgrade().is_some(),
            "the identity keeps its Arc live"
        );
        drop(first);
        drop(repeated);
        assert!(retained.upgrade().is_none());
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
        assert!(!superseded.is_empty());
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

        let _ = mixer.replace_with_crossfade(
            LoopBuffer::new(vec![0.6; 64], 1),
            mixer.default_crossfade_frames,
        );
        let _ = mixer.next_frame();
        let _ = mixer.replace_with_crossfade(
            LoopBuffer::new(vec![0.8; 64], 1),
            mixer.default_crossfade_frames,
        );
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
    fn requested_crossfade_duration_is_bounded_and_stays_with_its_pending_source() {
        assert_eq!(crossfade_frame_count(1_000, 0.5), Some(500));
        assert_eq!(crossfade_frame_count(1_000, 0.005), Some(5));
        assert_eq!(crossfade_frame_count(1_000, 2.0), Some(2_000));
        assert_eq!(crossfade_frame_count(0, 0.5), None);
        assert_eq!(crossfade_frame_count(1_000, 0.0), None);
        assert_eq!(crossfade_frame_count(1_000, f32::NAN), None);
        assert_eq!(crossfade_frame_count(1_000, 2.001), None);

        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(LoopBuffer::new(vec![0.2; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();

        assert!(
            mixer
                .replace_with_crossfade(LoopBuffer::new(vec![0.6; 64], 1), 500)
                .0
        );
        assert_eq!(mixer.crossfade_frames, 500);
        let _ = mixer.next_frame();
        assert!(
            mixer
                .replace_with_crossfade(LoopBuffer::new(vec![0.8; 64], 1), 250)
                .0
        );
        assert_eq!(
            mixer
                .pending
                .as_ref()
                .expect("pending source")
                .crossfade_frames,
            250
        );

        while mixer.crossfade_remaining > 0 {
            let _ = mixer.next_frame();
        }
        let retired = mixer.service_transitions();
        assert!(retired.is_some());
        assert_eq!(mixer.crossfade_frames, 250);
        assert_eq!(mixer.crossfade_remaining, 250);
        assert!((mixer.current.samples[0] - 0.8).abs() < 1.0e-6);

        while mixer.crossfade_remaining > 0 {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();
        assert!(mixer.replace(LoopBuffer::new(vec![0.4; 64], 1)).0);
        assert_eq!(mixer.crossfade_frames, mixer.default_crossfade_frames);
        assert_eq!(mixer.crossfade_frames, 30);
    }

    #[test]
    fn default_replacement_interrupts_a_long_fade_from_the_audible_mix() {
        fn long_fade_after(frames: usize) -> MixerState {
            let mut mixer = MixerState::new(1_000);
            let _ = mixer.replace(LoopBuffer::new(vec![0.2; 67], 1));
            for _ in 0..mixer.crossfade_frames {
                let _ = mixer.next_frame();
            }
            let _ = mixer.service_transitions();
            let _ = mixer.replace_with_crossfade(LoopBuffer::new(vec![0.6; 71], 1), 500);
            for _ in 0..frames {
                let _ = mixer.next_frame();
            }
            mixer
        }

        let mut reference = long_fade_after(200);
        let expected_next = reference.next_frame();
        let mut interrupted = long_fade_after(200);
        assert!(interrupted.replace(LoopBuffer::new(vec![-0.4; 73], 1)).0);
        assert!(interrupted.pending.is_none());
        assert_eq!(interrupted.crossfade_frames, 30);
        assert_eq!(interrupted.crossfade_remaining, 30);

        let first = interrupted.next_frame();
        assert!((first.0 - expected_next.0).abs() < 1.0e-6);
        assert!((first.1 - expected_next.1).abs() < 1.0e-6);
        assert!(interrupted.replace(LoopBuffer::new(vec![0.9; 79], 1)).0);
        assert_eq!(
            interrupted
                .pending
                .as_ref()
                .expect("bounded repeated interrupt")
                .crossfade_frames,
            30
        );
        while interrupted.crossfade_remaining > 0 {
            let _ = interrupted.next_frame();
        }
        let _ = interrupted.service_transitions();
        assert_eq!(interrupted.crossfade_remaining, 30);
        assert!((interrupted.next_frame().0 + 0.4).abs() < 1.0e-6);
        while interrupted.crossfade_remaining > 0 {
            let _ = interrupted.next_frame();
        }
        let _ = interrupted.service_transitions();
        assert!((interrupted.next_frame().0 - 0.9).abs() < 1.0e-6);
    }

    #[test]
    fn same_target_replacement_rebases_a_long_fade_without_restarting_playback() {
        fn long_fade_after(frames: usize) -> MixerState {
            let mut mixer = MixerState::new(1_000);
            let _ = mixer.replace(LoopBuffer::new(vec![0.2; 67], 1));
            for _ in 0..mixer.crossfade_frames {
                let _ = mixer.next_frame();
            }
            let _ = mixer.service_transitions();
            let _ = mixer.replace_with_crossfade(LoopBuffer::new(vec![0.6; 71], 1), 500);
            for _ in 0..frames {
                let _ = mixer.next_frame();
            }
            mixer
        }

        let mut reference = long_fade_after(200);
        let expected_next = reference.next_frame();
        let mut interrupted = long_fade_after(200);
        let playhead = interrupted.current.pos;

        let (changed, retired) = interrupted.replace(LoopBuffer::new(vec![0.6; 71], 1));

        assert!(changed);
        assert!(!retired.is_empty());
        assert_eq!(interrupted.current.pos, playhead);
        assert_eq!(interrupted.crossfade_remaining, 30);
        let first = interrupted.next_frame();
        assert!((first.0 - expected_next.0).abs() < 1.0e-6);
        assert!((first.1 - expected_next.1).abs() < 1.0e-6);
        let mut rebased = vec![first.0];
        while interrupted.crossfade_remaining > 0 {
            rebased.push(interrupted.next_frame().0);
        }
        assert!(
            rebased.windows(2).all(|pair| pair[1] >= pair[0]),
            "same-target coefficients must approach the target without a swell"
        );
        assert!(rebased.iter().all(|sample| *sample <= 0.6 + 1.0e-6));
        assert!((rebased.last().expect("rebase endpoint") - 0.6).abs() < 1.0e-6);
    }

    #[test]
    fn duplicate_requests_return_all_storage_for_post_lock_retirement() {
        let current_samples = Arc::new(vec![0.2; 257]);
        let pending_samples = Arc::new(vec![0.6; 263]);
        let mut mixer = MixerState::new(1_000);
        mixer.current = LoopBuffer::new_shared_at_rate(current_samples.clone(), 1, 1, 1);
        mixer.pending = Some(super::PendingSource {
            buffer: LoopBuffer::new_shared_at_rate(pending_samples.clone(), 1, 1, 1),
            crossfade_frames: 30,
        });

        let duplicate = LoopBuffer::new_shared_at_rate(current_samples.clone(), 1, 1, 1);
        let (changed, retired) = mixer.replace(duplicate);

        assert!(!changed);
        assert!(retired.first.is_some());
        assert!(retired.second.is_some());
        assert_eq!(Arc::strong_count(&current_samples), 3);
        assert_eq!(Arc::strong_count(&pending_samples), 2);
        drop(retired);
        assert_eq!(Arc::strong_count(&current_samples), 2);
        assert_eq!(Arc::strong_count(&pending_samples), 1);

        let pending = LoopBuffer::new_shared_at_rate(pending_samples.clone(), 1, 1, 1);
        mixer.pending = Some(super::PendingSource {
            buffer: pending,
            crossfade_frames: 30,
        });
        let duplicate_pending = LoopBuffer::new_shared_at_rate(pending_samples.clone(), 1, 1, 1);
        let (changed, retired) = mixer.replace_with_crossfade(duplicate_pending, 600);
        assert!(!changed);
        assert!(retired.first.is_some());
        assert!(retired.second.is_none());
        assert_eq!(Arc::strong_count(&pending_samples), 3);
        drop(retired);
        assert_eq!(Arc::strong_count(&pending_samples), 2);
    }

    #[test]
    fn crossfade_preserves_orthogonal_stereo_power_at_midpoint() {
        let mut mixer = MixerState::new(1_000);
        mixer.current = LoopBuffer::new(vec![1.0, 0.0, 1.0, 0.0], 2);
        let _ =
            mixer.replace_with_crossfade(LoopBuffer::new(vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0], 2), 3);
        let _ = mixer.next_frame();
        let midpoint = mixer.next_frame();
        let power = midpoint.0.mul_add(midpoint.0, midpoint.1 * midpoint.1);
        assert!((power - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn crossfade_clamps_correlated_full_scale_sources() {
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
    fn oneshot_plays_once_and_does_not_replace_the_loop() {
        let mut mixer = MixerState::new(8_000);
        mixer.current = LoopBuffer::new(vec![0.25, 0.25, 0.25, 0.25], 1);
        let identity = mixer.current.identity;
        assert!(
            mixer
                .replace_oneshot(OneshotPlay::new(vec![0.5, 0.0], 1.0, 1).expect("one-shot"))
                .is_none()
        );
        let first = mixer.next_frame();
        assert!(first.0 > 0.25, "oneshot adds energy: {first:?}");
        let _ = mixer.next_frame();
        let after = mixer.next_frame();
        assert!(
            (after.0 - 0.25).abs() < 1e-4,
            "oneshot ends; loop continues: {after:?}"
        );
        assert_eq!(mixer.current.identity, identity);
        assert!(mixer.service_oneshot().is_some());
        assert!(mixer.oneshot.is_none());
    }

    #[test]
    fn stereo_oneshot_preserves_pan_and_does_not_replace_the_loop() {
        let mut mixer = MixerState::new(8_000);
        mixer.current = LoopBuffer::new(vec![0.1; 8], 1);
        let identity = mixer.current.identity;

        assert!(
            mixer
                .replace_oneshot(
                    OneshotPlay::new(vec![0.6, 0.0, 0.0, 0.6], 1.0, 2).expect("stereo one-shot"),
                )
                .is_none()
        );

        let left = mixer.next_frame();
        let right = mixer.next_frame();
        let after = mixer.next_frame();
        assert!(left.0 > left.1 + 0.5, "left event preserves pan: {left:?}");
        assert!(
            right.1 > right.0 + 0.5,
            "right event preserves pan: {right:?}"
        );
        assert!((after.0 - 0.1).abs() < 1e-4);
        assert!((after.1 - 0.1).abs() < 1e-4);
        assert_eq!(mixer.current.identity, identity);
        assert!(
            mixer.oneshot.is_some(),
            "the callback retains finished storage for control-thread replacement"
        );
        assert!(mixer.service_oneshot().is_some());
        assert!(mixer.oneshot.is_none());
    }

    #[test]
    fn clearing_a_oneshot_stops_it_without_touching_the_loop() {
        let mut mixer = MixerState::new(8_000);
        mixer.current = LoopBuffer::new(vec![0.1; 8], 1);
        let identity = mixer.current.identity;
        let next = OneshotPlay::new(vec![0.8; 16], 1.0, 1).expect("one-shot");
        assert!(mixer.replace_oneshot(next).is_none());

        let retired = mixer.clear_oneshot().expect("retired one-shot");
        let frame = mixer.next_frame();
        assert!((frame.0 - 0.1).abs() < 1.0e-4);
        assert!((frame.1 - 0.1).abs() < 1.0e-4);
        assert_eq!(mixer.current.identity, identity);
        drop(retired);
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
