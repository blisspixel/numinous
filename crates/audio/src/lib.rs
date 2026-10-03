//! Numinous audio.
//!
//! Adaptive output through `cpal`: it uses the system default output device and
//! its default configuration, so it "just works" and follows the machine's sound
//! settings on Windows (WASAPI), macOS (CoreAudio), and Linux (ALSA). The tone
//! synthesis is a pure, testable function; opening and driving the device is kept
//! separate. Every looping source and one-shot names its [`Bus`]. The buses
//! share one reverb, each at its own send, then pass the master level and a
//! soft limiter, so the whole App plays as one instrument in one space. An
//! optional fixed capture ring taps the mixed output for the visualizer path,
//! and [`capture`] can open a loopback-like input when the OS exposes one. See
//! `docs/SOUND.md` and `docs/ARCHITECTURE.md`.

use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod bus;
pub mod capture;
use bus::{Reverb, soft_limit};
pub use capture::{CaptureRing, InputCapture, looks_like_loopback_name};

/// A gentle amplitude so a test tone is never harsh.
const AMPLITUDE: f32 = 0.2;
/// Upper device-rate boundary accepted by the real-time audio path.
const MAX_DEVICE_SAMPLE_RATE: u32 = 384_000;

fn validate_output_dimensions(sample_rate: u32, channels: u16) -> Result<(), String> {
    if sample_rate == 0 {
        return Err("output device reported a zero sample rate".to_string());
    }
    if sample_rate > MAX_DEVICE_SAMPLE_RATE {
        return Err(format!(
            "output device sample rate exceeds {MAX_DEVICE_SAMPLE_RATE} Hz"
        ));
    }
    if channels == 0 {
        return Err("output device reported zero channels".to_string());
    }
    Ok(())
}

/// The environment variable a player sets to collapse audio to one signal.
pub const MONO_AUDIO_VAR: &str = "NUMINOUS_MONO_AUDIO";

/// Whether a player has asked for mono, from a raw setting value.
///
/// Present and not empty, the same rule `NO_COLOR` and
/// `NUMINOUS_REDUCED_MOTION` use, so a player learns one convention for every
/// accessibility switch rather than one per switch. Truthiness deliberately
/// does not enter into it: someone who wrote `=0` still wrote it.
///
/// This crate owns the whole mono convention, name included, because it is a
/// hardware adapter that deliberately does not depend on `numinous-core`, and
/// splitting the name from the behaviour would put the same rule in two places.
#[must_use]
pub fn mono_requested_for(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// Whether a player has asked for mono, from the environment.
#[must_use]
pub fn mono_requested() -> bool {
    mono_requested_for(std::env::var_os(MONO_AUDIO_VAR).as_deref())
}

/// Collapse one stereo frame to a single sample.
///
/// Averaging, deliberately. It cannot exceed the louder of its two inputs, so
/// it cannot clip, and a centered frame (`left == right`) passes through
/// exactly unchanged. Numinous room beds are substantially centered, so that
/// exact case is the common one.
///
/// The alternative, scaling the sum by `1/sqrt(2)`, holds total power constant
/// for two *uncorrelated* signals. It is wrong here because it overshoots on
/// correlated ones: two centered samples at 0.8 sum to 1.13, which has to be
/// clamped, and clamping is nonlinear distortion rather than a change in level.
/// That is what this replaced, and it meant a centered signal above about 0.707
/// distorted on every mono device.
///
/// The trade is stated rather than papered over: averaging costs up to 3 dB on
/// hard-panned uncorrelated material, and in exchange it never distorts. On a
/// path a listener chose for accessibility, quiet is a better failure than
/// crunchy.
///
/// Non-finite input resolves to silence rather than travelling into a device
/// buffer.
///
/// The no-clipping guarantee is about the mix, not about rescuing bad input:
/// it says the result never exceeds the louder of the two samples, so a frame
/// already inside `[-1, 1]` stays inside it. Two samples at 2.0 average to 2.0.
/// The device path feeds this frames that `finite_output_sample` has already
/// bounded, so the precondition holds there.
#[must_use]
pub fn downmix_to_mono(left: f32, right: f32) -> f32 {
    if !left.is_finite() || !right.is_finite() {
        return 0.0;
    }
    (left + right) * 0.5
}

/// Which sample a given device channel receives from one stereo frame.
///
/// A one-channel device must be downmixed. A player who asked for mono gets
/// the same downmix on every channel of a multi-channel device, so nothing is
/// panned to a side they cannot hear.
///
/// See [`downmix_to_mono`] for why it averages.
fn device_channel_sample(frame: (f32, f32), channels: usize, channel: usize, mono: bool) -> f32 {
    if channels == 1 || mono {
        downmix_to_mono(frame.0, frame.1)
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
/// Share of room sound sent to the shared reverb: enough to place it in a
/// space without blurring its pitch.
const ROOM_REVERB_SEND: f32 = 0.22;
/// Share of game cues sent to the reverb: drier, so a cue stays crisp.
const EFFECT_REVERB_SEND: f32 = 0.12;
/// The send an outgoing source reaches by the end of a wash.
const WASH_SEND: f32 = 1.0;
/// Largest reverb input admitted, about +12 dBFS. Only hostile sources come
/// near it; the bound keeps one from filling the tail with seconds of noise.
const MAX_REVERB_SEND: f32 = 4.0;

/// Where a sound plays.
///
/// Each bus has its own share of the shared reverb, so a room's mathematics,
/// a recorded song, and a game cue can sit in one space without one dictating
/// the others' character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bus {
    /// Sonified mathematics: the room score, its parameter voice and events,
    /// Studio formula audio, and Watch Agent replay.
    Room,
    /// Recorded music: the radio. It is already mastered stereo, so it stays
    /// dry.
    Music,
    /// Short game and interface cues.
    Effect,
}

impl Bus {
    /// Every bus, in mixing order.
    pub const ALL: [Self; 3] = [Self::Room, Self::Music, Self::Effect];

    const fn index(self) -> usize {
        match self {
            Self::Room => 0,
            Self::Music => 1,
            Self::Effect => 2,
        }
    }

    const fn reverb_send(self) -> f32 {
        match self {
            Self::Room => ROOM_REVERB_SEND,
            Self::Music => 0.0,
            Self::Effect => EFFECT_REVERB_SEND,
        }
    }
}

/// How a new looping source takes over from the one playing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    kind: TransitionKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TransitionKind {
    Quick,
    Fade { seconds: f32, wash: bool },
}

impl Transition {
    /// The short default crossfade, 30 milliseconds.
    ///
    /// It interrupts a longer fade from that fade's exact audible mix instead
    /// of waiting behind it, and a repeated interruption waits behind at most
    /// one more short fade, so callback work stays bounded.
    pub const QUICK: Self = Self {
        kind: TransitionKind::Quick,
    };

    /// An equal-power crossfade over `seconds`, accepted from 5 milliseconds
    /// to 2 seconds.
    ///
    /// It waits behind an active fade instead of interrupting it, and a
    /// deferred source keeps its own duration.
    #[must_use]
    pub const fn crossfade(seconds: f32) -> Option<Self> {
        Self::fade(seconds, false)
    }

    /// A crossfade in which the outgoing source swells into the shared reverb
    /// as it fades, so it dissolves into its own tail: the wash through black
    /// that goes with a room change. Accepted from 5 milliseconds to 2 seconds.
    ///
    /// The swell follows the outgoing source's bus, so a bus with no reverb
    /// send, such as music, stays dry and simply crossfades. Like
    /// [`Transition::crossfade`], it waits behind an active fade.
    #[must_use]
    pub const fn wash(seconds: f32) -> Option<Self> {
        Self::fade(seconds, true)
    }

    const fn fade(seconds: f32, wash: bool) -> Option<Self> {
        if seconds.is_finite()
            && seconds >= MIN_REQUESTED_CROSSFADE_SECONDS
            && seconds <= MAX_REQUESTED_CROSSFADE_SECONDS
        {
            Some(Self {
                kind: TransitionKind::Fade { seconds, wash },
            })
        } else {
            None
        }
    }
}

fn fade_frames(sample_rate: u32, seconds: f32) -> usize {
    (seconds * sample_rate.max(1) as f32).max(1.0) as usize
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ParameterTarget {
    root_hz: f32,
    ratio: f32,
    gain: f32,
}

/// A quiet two-oscillator voice that follows a continuously changing ratio.
///
/// Phases persist across target changes. Both absolute frequencies and gain
/// approach their targets inside the callback, so control-thread updates
/// never restart either oscillator or introduce an abrupt parameter step.
/// Smoothing root and ratio separately would bend a fixed second frequency
/// whenever a root change is balanced by the reciprocal ratio change.
struct ParameterVoice {
    target: Option<ParameterTarget>,
    current_root_hz: f32,
    current_second_hz: f32,
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
            current_second_hz: 220.0,
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
            self.current_second_hz = root_hz * ratio;
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
        let (target_root, target_second, target_gain) = self.target.map_or(
            (self.current_root_hz, self.current_second_hz, 0.0),
            |target| (target.root_hz, target.root_hz * target.ratio, target.gain),
        );
        self.current_root_hz += (target_root - self.current_root_hz) * self.smoothing;
        self.current_second_hz += (target_second - self.current_second_hz) * self.smoothing;
        if self.current_gain < target_gain {
            self.current_gain = (self.current_gain + self.gain_step).min(target_gain);
        } else if self.current_gain > target_gain {
            self.current_gain = (self.current_gain - self.gain_step).max(target_gain);
        }

        let upper_frequency = self.sample_rate * 0.45;
        let ratio_frequency = self.current_second_hz.min(upper_frequency);
        self.root_phase =
            (self.root_phase + TAU * self.current_root_hz / self.sample_rate).rem_euclid(TAU);
        self.ratio_phase =
            (self.ratio_phase + TAU * ratio_frequency / self.sample_rate).rem_euclid(TAU);
        let pair = (self.root_phase.sin() + self.ratio_phase.sin()) * 0.5;
        (pair * self.current_gain).clamp(-PARAMETER_MAX_GAIN, PARAMETER_MAX_GAIN)
    }
}

/// One mono or interleaved-stereo looping source and the bus it plays on.
struct LoopBuffer {
    samples: Arc<Vec<f32>>,
    sample_len: usize,
    pos: usize,
    fraction: f64,
    step: f64,
    channels: usize,
    identity: SourceIdentity,
    bus: Bus,
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
    #[cfg(test)]
    fn new(samples: impl Into<Arc<Vec<f32>>>, channels: usize, bus: Bus) -> Self {
        Self::new_at_rate(samples, channels, 1, 1, bus)
    }

    fn new_at_rate(
        samples: impl Into<Arc<Vec<f32>>>,
        channels: usize,
        source_rate: u32,
        output_rate: u32,
        bus: Bus,
    ) -> Self {
        Self::from_samples(
            samples.into(),
            channels,
            source_rate,
            output_rate,
            IdentityKind::Content,
            bus,
        )
    }

    fn new_shared_at_rate(
        samples: Arc<Vec<f32>>,
        channels: usize,
        source_rate: u32,
        output_rate: u32,
        bus: Bus,
    ) -> Self {
        Self::from_samples(
            samples,
            channels,
            source_rate,
            output_rate,
            IdentityKind::SharedAllocation,
            bus,
        )
    }

    fn from_samples(
        samples: Arc<Vec<f32>>,
        channels: usize,
        source_rate: u32,
        output_rate: u32,
        identity_kind: IdentityKind,
        bus: Bus,
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
            bus,
        }
    }

    fn silent() -> Self {
        Self::new_at_rate(Vec::new(), 1, 1, 1, Bus::Room)
    }

    /// Whether `other` is this same source on the same bus. The same content
    /// routed to a different bus is a different source, so the routing
    /// change crossfades instead of being ignored.
    fn same_source(&self, other: &Self) -> bool {
        self.identity == other.identity && self.bus == other.bus
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

    fn next_frame(&mut self) -> (f32, f32) {
        if self.exhausted() {
            // Keep exhausted storage in its slot. The next control-thread
            // enqueue or service replaces and destroys it without freeing
            // memory here.
            return (0.0, 0.0);
        }
        let left = self.samples[self.pos] * self.gain;
        let right = if self.channels == 1 {
            left
        } else {
            self.samples[self.pos + 1] * self.gain
        };
        self.pos += self.channels;
        (left, right)
    }
}

/// Callback-owned state. All storage is prepared by the control thread, so
/// producing a frame performs no allocation.
struct PendingSource {
    buffer: LoopBuffer,
    crossfade_frames: usize,
    wash: bool,
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
    /// How far an interrupted wash had swelled the primary into the reverb,
    /// from 0 to 1. It holds there while this mix fades, so the reverb input
    /// never jumps when a wash is cut short.
    primary_wash: f32,
}

impl SourceMix {
    fn single(primary: LoopBuffer) -> Self {
        Self {
            primary,
            secondary: None,
            primary_gain: 1.0,
            secondary_gain: 0.0,
            primary_wash: 0.0,
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
        wash: f32,
    ) -> Self {
        debug_assert!(previous.is_single());
        previous.primary_gain *= old_gain;
        previous.primary_wash = wash;
        previous.secondary = Some(current);
        previous.secondary_gain = new_gain;
        previous
    }

    fn mix_into(&mut self, gain: f32, wash: f32, mix: &mut BusMix) {
        let primary = self.primary.next_frame();
        let primary_gain = self.primary_gain * gain;
        mix.add(self.primary.bus, primary, primary_gain);
        mix.add_wash(self.primary.bus, primary, primary_gain * wash);
        if let Some(secondary) = self.secondary.as_mut() {
            let frame = secondary.next_frame();
            mix.add(secondary.bus, frame, self.secondary_gain * gain);
        }
    }
}

/// One frame's contributions, gathered per bus before the master chain.
#[derive(Default)]
struct BusMix {
    dry: [(f32, f32); 3],
    /// Extra reverb send from a washing source, beyond its bus's own send.
    wash: [(f32, f32); 3],
}

impl BusMix {
    fn add(&mut self, bus: Bus, frame: (f32, f32), gain: f32) {
        let slot = &mut self.dry[bus.index()];
        slot.0 += frame.0 * gain;
        slot.1 += frame.1 * gain;
    }

    fn add_wash(&mut self, bus: Bus, frame: (f32, f32), amount: f32) {
        let send = bus.reverb_send();
        if send <= 0.0 || amount <= 0.0 {
            return;
        }
        let extra = (WASH_SEND - send) * amount;
        let slot = &mut self.wash[bus.index()];
        slot.0 += frame.0 * extra;
        slot.1 += frame.1 * extra;
    }

    /// The dry sum and the reverb send. A non-finite bus is silenced as a
    /// unit rather than allowed into the reverb, where it would poison the
    /// tail until the stream closed.
    fn sum(&self) -> ((f32, f32), (f32, f32)) {
        let mut dry = (0.0f32, 0.0f32);
        let mut send = (0.0f32, 0.0f32);
        for bus in Bus::ALL {
            let frame = finite_frame(self.dry[bus.index()]);
            let wash = finite_frame(self.wash[bus.index()]);
            dry.0 += frame.0;
            dry.1 += frame.1;
            send.0 += frame.0.mul_add(bus.reverb_send(), wash.0);
            send.1 += frame.1.mul_add(bus.reverb_send(), wash.1);
        }
        (
            dry,
            (
                send.0.clamp(-MAX_REVERB_SEND, MAX_REVERB_SEND),
                send.1.clamp(-MAX_REVERB_SEND, MAX_REVERB_SEND),
            ),
        )
    }
}

fn finite_frame(frame: (f32, f32)) -> (f32, f32) {
    if frame.0.is_finite() && frame.1.is_finite() {
        frame
    } else {
        (0.0, 0.0)
    }
}

/// A level that moves toward its target in equal steps, so a change is a
/// short ramp instead of a step.
struct Ramp {
    current: f32,
    step: f32,
}

impl Ramp {
    fn advance(&mut self, target: f32) -> f32 {
        if self.current < target {
            self.current = (self.current + self.step).min(target);
        } else if self.current > target {
            self.current = (self.current - self.step).max(target);
        }
        self.current
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
    /// Whether the active fade washes its outgoing source into the reverb.
    wash: bool,
    master_gain: f32,
    master: Ramp,
    active: bool,
    parameter_voice: ParameterVoice,
    /// One one-shot slot per bus, so a game cue and a room event never
    /// replace each other.
    oneshots: [Option<OneshotPlay>; 3],
    reverb: Reverb,
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
            wash: false,
            master_gain: 1.0,
            master: Ramp {
                current: 1.0,
                step: 1.0 / (GAIN_RAMP_SECONDS * rate).max(1.0),
            },
            active: true,
            parameter_voice: ParameterVoice::new(sample_rate),
            oneshots: [None, None, None],
            reverb: Reverb::new(sample_rate),
            output_tap: None,
        }
    }

    /// Install prepared storage and return the superseded one-shot for
    /// destruction after the caller releases the callback mutex.
    fn replace_oneshot(&mut self, bus: Bus, next: OneshotPlay) -> Option<OneshotPlay> {
        self.oneshots[bus.index()].replace(next)
    }

    fn service_oneshots(&mut self) -> [Option<OneshotPlay>; 3] {
        self.oneshots.each_mut().map(|slot| {
            slot.as_ref()
                .is_some_and(OneshotPlay::exhausted)
                .then(|| slot.take())
                .flatten()
        })
    }

    fn clear_oneshot(&mut self, bus: Bus) -> Option<OneshotPlay> {
        self.oneshots[bus.index()].take()
    }

    fn replace(&mut self, next: LoopBuffer) -> (bool, RetiredBuffers) {
        if self.current.same_source(&next) {
            if self.crossfade_remaining > 0
                && self.retired.is_none()
                && self.previous.as_ref().is_some_and(SourceMix::is_single)
            {
                let (old_gain, new_gain) = self.crossfade_gains();
                self.hold_wash();
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
            .is_some_and(|pending| pending.buffer.same_source(&next))
        {
            return (false, RetiredBuffers::one(next));
        }

        if self.crossfade_remaining > 0
            && self.retired.is_none()
            && self.previous.as_ref().is_some_and(SourceMix::is_single)
        {
            let (old_gain, new_gain) = self.crossfade_gains();
            let wash = self.outgoing_wash();
            let previous = self.previous.take().expect("active transition source");
            let current = std::mem::replace(&mut self.current, next);
            self.previous = Some(SourceMix::from_interrupted_transition(
                previous, current, old_gain, new_gain, wash,
            ));
            self.wash = false;
            self.crossfade_frames = self.default_crossfade_frames;
            self.crossfade_remaining = self.crossfade_frames;
            self.coefficient_ramp_start = None;
            let superseded = self.pending.take().map(|pending| pending.buffer);
            let superseded = superseded.map_or_else(RetiredBuffers::default, RetiredBuffers::one);
            return (true, superseded);
        }

        self.replace_with_fade(next, self.default_crossfade_frames, false)
    }

    fn replace_with_fade(
        &mut self,
        next: LoopBuffer,
        crossfade_frames: usize,
        wash: bool,
    ) -> (bool, RetiredBuffers) {
        if self.current.same_source(&next) {
            return (
                false,
                RetiredBuffers::two(next, self.pending.take().map(|pending| pending.buffer)),
            );
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.buffer.same_source(&next))
        {
            return (false, RetiredBuffers::one(next));
        }
        let crossfade_frames = crossfade_frames.max(1);
        if self.crossfade_remaining > 0 || self.retired.is_some() {
            let superseded = self.pending.replace(PendingSource {
                buffer: next,
                crossfade_frames,
                wash,
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
        self.wash = wash;
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
            self.wash = pending.wash;
        }
        retired
    }

    /// How far the active fade has run, from 0 on its first frame to 1 on
    /// its last.
    fn crossfade_progress(&self) -> f32 {
        if self.crossfade_frames <= 1 {
            1.0
        } else {
            1.0 - (self.crossfade_remaining - 1) as f32 / (self.crossfade_frames - 1) as f32
        }
    }

    fn crossfade_gains(&self) -> (f32, f32) {
        let progress = self.crossfade_progress();
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

    /// How far the outgoing source has swelled into the reverb, from 0 to 1.
    /// A washing fade swells with its progress; otherwise the outgoing mix
    /// holds whatever an interrupted wash had reached.
    fn outgoing_wash(&self) -> f32 {
        if self.wash {
            self.crossfade_progress()
        } else {
            self.previous
                .as_ref()
                .map_or(0.0, |previous| previous.primary_wash)
        }
    }

    /// Freeze the outgoing swell where it stands before the active fade is
    /// re-timed, so the reverb input stays continuous.
    fn hold_wash(&mut self) {
        let wash = self.outgoing_wash();
        if let Some(previous) = self.previous.as_mut() {
            previous.primary_wash = wash;
        }
        self.wash = false;
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
        let mut mix = BusMix::default();
        let current = self.current.next_frame();
        if self.crossfade_remaining == 0 {
            mix.add(self.current.bus, current, 1.0);
        } else {
            let (old_gain, new_gain) = self.crossfade_gains();
            let wash = self.outgoing_wash();
            if let Some(previous) = self.previous.as_mut() {
                previous.mix_into(old_gain, wash, &mut mix);
            }
            mix.add(self.current.bus, current, new_gain);
            self.crossfade_remaining -= 1;
            if self.crossfade_remaining == 0 {
                self.coefficient_ramp_start = None;
                self.wash = false;
                debug_assert!(self.retired.is_none());
                self.retired = self.previous.take();
            }
        }

        let parameter = self.parameter_voice.next_sample();
        mix.add(Bus::Room, (parameter, parameter), 1.0);
        for bus in Bus::ALL {
            let frame = self.oneshots[bus.index()]
                .as_mut()
                .map_or((0.0, 0.0), OneshotPlay::next_frame);
            mix.add(bus, frame, 1.0);
        }

        let (dry, send) = mix.sum();
        let wet = self.reverb.process(send);
        let target = if self.active { self.master_gain } else { 0.0 };
        let gain = self.master.advance(target);
        let left = finite_output_sample(soft_limit((dry.0 + wet.0) * gain));
        let right = finite_output_sample(soft_limit((dry.1 + wet.1) * gain));
        if let Some(tap) = self.output_tap.as_ref()
            && let Ok(mut ring) = tap.try_lock()
        {
            ring.push_frame(left, right);
        }
        (left, right)
    }
}

fn finite_output_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

fn build_loop_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    state: Arc<Mutex<MixerState>>,
    mono: bool,
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
                        *output =
                            T::from_sample(device_channel_sample(mixed, channels, channel, mono));
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

/// Plays looping sources and one-shots on the default device, in the
/// background, through one shared master chain.
///
/// Every source names its [`Bus`]. The buses share one reverb, each at its
/// own send, then pass the master level and a soft limiter. Swap a source at
/// any time with [`LoopPlayer::set_samples`] or
/// [`LoopPlayer::set_shared_stereo_at_rate`] (for example when the visible
/// room changes). The stream keeps running until the player is dropped.
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
    /// Honours the player's mono preference from the environment. Use
    /// [`LoopPlayer::new_with_mono`] to decide it explicitly.
    ///
    /// # Errors
    /// Returns an error string if the device or stream cannot be set up.
    pub fn new() -> Result<Self, String> {
        Self::new_with_mono(mono_requested())
    }

    /// Open the default device with the mono preference supplied rather than
    /// read, so a caller can decide it and a test can pin it.
    ///
    /// # Errors
    /// Returns an error string if the device or stream cannot be set up.
    pub fn new_with_mono(mono: bool) -> Result<Self, String> {
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
                build_loop_stream::<i8>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::I16 => {
                build_loop_stream::<i16>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::I24 => build_loop_stream::<cpal::I24>(
                &context.device,
                config,
                channels,
                state.clone(),
                mono,
            ),
            cpal::SampleFormat::I32 => {
                build_loop_stream::<i32>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::I64 => {
                build_loop_stream::<i64>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::U8 => {
                build_loop_stream::<u8>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::U16 => {
                build_loop_stream::<u16>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::U24 => build_loop_stream::<cpal::U24>(
                &context.device,
                config,
                channels,
                state.clone(),
                mono,
            ),
            cpal::SampleFormat::U32 => {
                build_loop_stream::<u32>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::U64 => {
                build_loop_stream::<u64>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::F32 => {
                build_loop_stream::<f32>(&context.device, config, channels, state.clone(), mono)
            }
            cpal::SampleFormat::F64 => {
                build_loop_stream::<f64>(&context.device, config, channels, state.clone(), mono)
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

    /// Change the looping source to mono samples at the device rate, on `bus`.
    ///
    /// Supplying the same content on the same bus again is a no-op that
    /// preserves the playhead and returns `false`. New content starts at its
    /// beginning under `transition`.
    pub fn set_samples(&self, bus: Bus, samples: Vec<f32>, transition: Transition) -> bool {
        let next = LoopBuffer::new_at_rate(samples, 1, self.sample_rate, self.sample_rate, bus);
        self.transition_to(next, transition)
    }

    /// Change the looping source to shared interleaved-stereo samples rendered
    /// at `source_rate`, on `bus`, without copying them.
    ///
    /// The real-time loop interpolates them at the device rate, so a high-rate
    /// device needs no second amplified copy of the whole source. Reusing the
    /// same allocation, source rate, and bus preserves the playhead and
    /// returns `false`. An incomplete final frame is ignored without copying.
    pub fn set_shared_stereo_at_rate(
        &self,
        bus: Bus,
        interleaved: Arc<Vec<f32>>,
        source_rate: u32,
        transition: Transition,
    ) -> bool {
        let next =
            LoopBuffer::new_shared_at_rate(interleaved, 2, source_rate, self.sample_rate, bus);
        self.transition_to(next, transition)
    }

    fn transition_to(&self, next: LoopBuffer, transition: Transition) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        let (changed, retired) = match transition.kind {
            TransitionKind::Quick => state.replace(next),
            TransitionKind::Fade { seconds, wash } => {
                state.replace_with_fade(next, fade_frames(self.sample_rate, seconds), wash)
            }
        };
        drop(state);
        retired.retire();
        changed
    }

    /// Reclaim source storage retired by the real-time callback.
    ///
    /// Interactive faces should call this from their ordinary update loop.
    /// Destruction then happens on the control thread, never in audio time.
    pub fn service(&self) {
        let (retired_source, retired_oneshots) = self.state.lock().map_or_else(
            |_| (None, [None, None, None]),
            |mut state| (state.service_transitions(), state.service_oneshots()),
        );
        drop(retired_source);
        drop(retired_oneshots);
    }

    /// Set the master linear gain without replacing or restarting the source.
    /// Changes ramp over a short interval inside the audio callback.
    pub fn set_master_gain(&self, gain: f32) {
        if let Ok(mut state) = self.state.lock() {
            state.set_master_gain(gain);
        }
    }

    /// Set a quiet continuous ratio voice over the current looping source. It
    /// plays on [`Bus::Room`], because it is the room's mathematics.
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

    /// Play a mono one-shot on `bus` over the looping source without
    /// restarting it.
    ///
    /// Samples are consumed once at the device rate. A new one-shot replaces
    /// an unfinished one on the same bus; one-shots on different buses play
    /// together. Empty input or a non-finite or zero gain is ignored.
    pub fn play_oneshot(&self, bus: Bus, samples: Vec<f32>, gain: f32) {
        self.enqueue_oneshot(bus, OneshotPlay::new(samples, gain, 1));
    }

    /// Play interleaved-stereo samples once on `bus` without restarting the
    /// loop.
    ///
    /// Samples are consumed once at the device rate. An incomplete final
    /// frame is ignored. A new one-shot replaces an unfinished one on the same
    /// bus.
    pub fn play_stereo_oneshot(&self, bus: Bus, samples: Vec<f32>, gain: f32) {
        self.enqueue_oneshot(bus, OneshotPlay::new(samples, gain, 2));
    }

    fn enqueue_oneshot(&self, bus: Bus, next: Option<OneshotPlay>) {
        let Some(next) = next else {
            return;
        };
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.replace_oneshot(bus, next));
        drop(retired);
    }

    /// Stop and retire the one-shot on `bus` without changing the loop.
    pub fn clear_oneshot(&self, bus: Bus) {
        let retired = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.clear_oneshot(bus));
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
        AMPLITUDE, Bus, LoopBuffer, MAX_DEVICE_SAMPLE_RATE, MONO_AUDIO_VAR, MixerState,
        OneshotPlay, PARAMETER_MAX_GAIN, Transition, device_channel_sample, downmix_to_mono,
        fade_frames, fill_tone_samples, mono_requested_for, synthesize_sine,
        validate_output_dimensions,
    };
    use crate::bus::{LIMITER_CEILING, soft_limit};

    /// A source on the music bus, which has no reverb send, so a test of
    /// source switching sees exactly the samples it supplied.
    fn dry(samples: impl Into<Arc<Vec<f32>>>, channels: usize) -> LoopBuffer {
        LoopBuffer::new(samples, channels, Bus::Music)
    }

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
        assert!(validate_output_dimensions(MAX_DEVICE_SAMPLE_RATE, 2).is_ok());
        assert_eq!(
            validate_output_dimensions(MAX_DEVICE_SAMPLE_RATE + 1, 2)
                .expect_err("oversized rate must fail"),
            format!("output device sample rate exceeds {MAX_DEVICE_SAMPLE_RATE} Hz")
        );
    }

    #[test]
    fn device_channel_projection_downmixes_mono_and_preserves_stereo() {
        let frame = (0.6, 0.2);
        assert!((device_channel_sample(frame, 1, 0, false) - 0.4).abs() < 1.0e-6);
        assert_eq!(device_channel_sample(frame, 2, 0, false), 0.6);
        assert_eq!(device_channel_sample(frame, 2, 1, false), 0.2);
        assert_eq!(device_channel_sample(frame, 4, 2, false), 0.6);
        assert_eq!(device_channel_sample(frame, 4, 3, false), 0.2);
    }

    #[test]
    fn a_centered_frame_reaches_a_mono_device_undistorted() {
        // This is the regression. The previous downmix scaled the sum by
        // 1/sqrt(2) and clamped, so a centered signal above about 0.707
        // flattened against the ceiling on every mono device. The old test
        // asserted the clamped value and so recorded the defect as correct.
        for level in [0.5f32, 0.707, 0.8, 1.0, -0.9] {
            assert_eq!(
                device_channel_sample((level, level), 1, 0, false),
                level,
                "centered {level} must arrive intact"
            );
        }
    }

    #[test]
    fn a_requested_mono_mix_reaches_every_channel_of_a_stereo_device() {
        // What a listener on one ear needs: nothing panned to a side they
        // cannot hear.
        let frame = (0.9, -0.1);
        let expected = downmix_to_mono(frame.0, frame.1);
        for channel in 0..4 {
            assert_eq!(
                device_channel_sample(frame, 4, channel, true),
                expected,
                "channel {channel}"
            );
        }
        // Without the request, the same frame keeps its sides.
        assert_eq!(device_channel_sample(frame, 2, 0, false), 0.9);
        assert_eq!(device_channel_sample(frame, 2, 1, false), -0.1);
    }

    #[test]
    fn the_downmix_can_never_clip() {
        // Exhaustive over a fine grid of the legal input square: the result
        // never leaves [-1, 1] and never exceeds the louder of its inputs, so
        // no clamp is needed anywhere.
        let steps = 201;
        for l in 0..steps {
            for r in 0..steps {
                let left = -1.0 + 2.0 * l as f32 / (steps - 1) as f32;
                let right = -1.0 + 2.0 * r as f32 / (steps - 1) as f32;
                let out = downmix_to_mono(left, right);
                assert!((-1.0..=1.0).contains(&out), "{left} {right} -> {out}");
                assert!(
                    out.abs() <= left.abs().max(right.abs()) + 1e-6,
                    "{left} {right} -> {out} is louder than both"
                );
            }
        }
    }

    #[test]
    fn the_switch_reads_presence_not_truthiness() {
        for (value, on) in [
            (None, false),
            (Some(""), false),
            (Some("1"), true),
            (Some("0"), true),
            (Some("false"), true),
        ] {
            assert_eq!(
                mono_requested_for(value.map(std::ffi::OsStr::new)),
                on,
                "{value:?}"
            );
        }
        assert_eq!(MONO_AUDIO_VAR, "NUMINOUS_MONO_AUDIO");
    }

    #[test]
    fn the_downmix_bounds_the_mix_not_the_input() {
        // Stated so the guarantee is not read as wider than it is: out-of-range
        // input stays out of range, it just never gets louder than it was.
        assert_eq!(downmix_to_mono(2.0, 2.0), 2.0);
        assert_eq!(downmix_to_mono(2.0, 0.0), 1.0);
    }

    #[test]
    fn the_cost_of_the_trade_is_pinned() {
        // Hard-panned content arrives at half level. Recorded as a test so it
        // cannot drift silently into something louder that clips.
        assert_eq!(downmix_to_mono(1.0, 0.0), 0.5);
        assert_eq!(downmix_to_mono(0.0, -1.0), -0.5);
        // Antiphase genuinely cancels; a one-speaker listener would have heard
        // only one side of it anyway.
        assert_eq!(downmix_to_mono(0.7, -0.7), 0.0);
    }

    #[test]
    fn non_finite_samples_become_silence_not_a_device_hazard() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(downmix_to_mono(bad, 0.5), 0.0);
            assert_eq!(downmix_to_mono(0.5, bad), 0.0);
        }
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

        let mut mono = dry(vec![0.1, 0.2, 0.3], 1);
        assert_eq!(mono.next_frame(), (0.1, 0.1), "mono fills both ears");
        assert_eq!(mono.next_frame(), (0.2, 0.2));
        assert_eq!(mono.next_frame(), (0.3, 0.3));
        assert_eq!(mono.next_frame(), (0.1, 0.1), "and wraps");

        let mut stereo = dry(vec![0.1, -0.1, 0.2, -0.2], 2);
        assert_eq!(stereo.next_frame(), (0.1, -0.1), "left and right differ");
        assert_eq!(stereo.next_frame(), (0.2, -0.2));
        assert_eq!(stereo.next_frame(), (0.1, -0.1), "wraps on frames");

        let odd = dry(vec![0.1, -0.1, 9.0], 2);
        assert_eq!(odd.sample_len, 2);
    }

    #[test]
    fn shared_loop_buffer_reuses_complete_stereo_storage() {
        let samples = Arc::new(vec![0.1, -0.1, 0.2, -0.2]);
        let buffer = dry(samples.clone(), 2);

        assert!(Arc::ptr_eq(&buffer.samples, &samples));
    }

    #[test]
    fn shared_loop_identity_is_constant_time_and_tracks_the_allocation() {
        let samples = Arc::new(vec![0.1, -0.1, 0.2, -0.2]);
        let retained = Arc::downgrade(&samples);
        let first = LoopBuffer::new_shared_at_rate(samples.clone(), 2, 16_000, 48_000, Bus::Music);
        let repeated = LoopBuffer::new_shared_at_rate(samples, 2, 16_000, 48_000, Bus::Music);
        let equal_content = LoopBuffer::new_shared_at_rate(
            Arc::new(vec![0.1, -0.1, 0.2, -0.2]),
            2,
            16_000,
            48_000,
            Bus::Music,
        );

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
        let mut buffer = LoopBuffer::new_at_rate(samples.clone(), 2, 2, 4, Bus::Music);

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
        assert!(mixer.replace(dry(samples.clone(), 1)).0);
        for _ in 0..mixer.crossfade_frames + 2 {
            let _ = mixer.next_frame();
        }
        let retired = mixer.service_transitions();
        assert!(retired.is_some(), "retired storage leaves the callback");
        let before = mixer.current.pos;
        let (changed, superseded) = mixer.replace(dry(samples, 1));
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
        let _ = mixer.replace(dry(vec![0.8; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();
        assert!(mixer.replace(dry(vec![-0.8; 64], 1)).0);
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
        let _ = mixer.replace(dry(vec![0.2; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let retired_silence = mixer.service_transitions();
        assert!(retired_silence.is_some());

        let _ =
            mixer.replace_with_fade(dry(vec![0.6; 64], 1), mixer.default_crossfade_frames, false);
        let _ = mixer.next_frame();
        let _ =
            mixer.replace_with_fade(dry(vec![0.8; 64], 1), mixer.default_crossfade_frames, false);
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
        assert_eq!(fade_frames(1_000, 0.5), 500);
        assert_eq!(fade_frames(1_000, 0.005), 5);
        assert_eq!(fade_frames(1_000, 2.0), 2_000);
        for seconds in [0.005, 0.5, 2.0] {
            assert!(Transition::crossfade(seconds).is_some());
            assert!(Transition::wash(seconds).is_some());
        }
        for seconds in [0.0, 0.004, 2.001, f32::NAN, f32::INFINITY, -0.5] {
            assert_eq!(Transition::crossfade(seconds), None, "{seconds}");
            assert_eq!(Transition::wash(seconds), None, "{seconds}");
        }

        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(dry(vec![0.2; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();

        assert!(mixer.replace_with_fade(dry(vec![0.6; 64], 1), 500, false).0);
        assert_eq!(mixer.crossfade_frames, 500);
        let _ = mixer.next_frame();
        assert!(mixer.replace_with_fade(dry(vec![0.8; 64], 1), 250, false).0);
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
        assert!(mixer.replace(dry(vec![0.4; 64], 1)).0);
        assert_eq!(mixer.crossfade_frames, mixer.default_crossfade_frames);
        assert_eq!(mixer.crossfade_frames, 30);
    }

    #[test]
    fn default_replacement_interrupts_a_long_fade_from_the_audible_mix() {
        fn long_fade_after(frames: usize) -> MixerState {
            let mut mixer = MixerState::new(1_000);
            let _ = mixer.replace(dry(vec![0.2; 67], 1));
            for _ in 0..mixer.crossfade_frames {
                let _ = mixer.next_frame();
            }
            let _ = mixer.service_transitions();
            let _ = mixer.replace_with_fade(dry(vec![0.6; 71], 1), 500, false);
            for _ in 0..frames {
                let _ = mixer.next_frame();
            }
            mixer
        }

        let mut reference = long_fade_after(200);
        let expected_next = reference.next_frame();
        let mut interrupted = long_fade_after(200);
        assert!(interrupted.replace(dry(vec![-0.4; 73], 1)).0);
        assert!(interrupted.pending.is_none());
        assert_eq!(interrupted.crossfade_frames, 30);
        assert_eq!(interrupted.crossfade_remaining, 30);

        let first = interrupted.next_frame();
        assert!((first.0 - expected_next.0).abs() < 1.0e-6);
        assert!((first.1 - expected_next.1).abs() < 1.0e-6);
        assert!(interrupted.replace(dry(vec![0.7; 79], 1)).0);
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
        assert!((interrupted.next_frame().0 - 0.7).abs() < 1.0e-6);
    }

    #[test]
    fn same_target_replacement_rebases_a_long_fade_without_restarting_playback() {
        fn long_fade_after(frames: usize) -> MixerState {
            let mut mixer = MixerState::new(1_000);
            let _ = mixer.replace(dry(vec![0.2; 67], 1));
            for _ in 0..mixer.crossfade_frames {
                let _ = mixer.next_frame();
            }
            let _ = mixer.service_transitions();
            let _ = mixer.replace_with_fade(dry(vec![0.6; 71], 1), 500, false);
            for _ in 0..frames {
                let _ = mixer.next_frame();
            }
            mixer
        }

        let mut reference = long_fade_after(200);
        let expected_next = reference.next_frame();
        let mut interrupted = long_fade_after(200);
        let playhead = interrupted.current.pos;

        let (changed, retired) = interrupted.replace(dry(vec![0.6; 71], 1));

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
        mixer.current =
            LoopBuffer::new_shared_at_rate(current_samples.clone(), 1, 1, 1, Bus::Music);
        mixer.pending = Some(super::PendingSource {
            buffer: LoopBuffer::new_shared_at_rate(pending_samples.clone(), 1, 1, 1, Bus::Music),
            crossfade_frames: 30,
            wash: false,
        });

        let duplicate =
            LoopBuffer::new_shared_at_rate(current_samples.clone(), 1, 1, 1, Bus::Music);
        let (changed, retired) = mixer.replace(duplicate);

        assert!(!changed);
        assert!(retired.first.is_some());
        assert!(retired.second.is_some());
        assert_eq!(Arc::strong_count(&current_samples), 3);
        assert_eq!(Arc::strong_count(&pending_samples), 2);
        drop(retired);
        assert_eq!(Arc::strong_count(&current_samples), 2);
        assert_eq!(Arc::strong_count(&pending_samples), 1);

        let pending = LoopBuffer::new_shared_at_rate(pending_samples.clone(), 1, 1, 1, Bus::Music);
        mixer.pending = Some(super::PendingSource {
            buffer: pending,
            crossfade_frames: 30,
            wash: false,
        });
        let duplicate_pending =
            LoopBuffer::new_shared_at_rate(pending_samples.clone(), 1, 1, 1, Bus::Music);
        let (changed, retired) = mixer.replace_with_fade(duplicate_pending, 600, false);
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
        mixer.current = dry(vec![1.0, 0.0, 1.0, 0.0], 2);
        let _ = mixer.replace_with_fade(dry(vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0], 2), 3, false);
        let _ = mixer.next_frame();
        let midpoint = mixer.next_frame();
        let power = midpoint.0.mul_add(midpoint.0, midpoint.1 * midpoint.1);
        assert!((power - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_crossfade_of_correlated_full_scale_sources_bends_under_the_ceiling() {
        // Equal-power gains sum to as much as 1.41 when both sides carry the
        // same signal. The limiter bends that into the ceiling instead of the
        // hard clip this mix used to rely on, and never lets it through.
        let floor = soft_limit(1.0);
        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace(dry(vec![1.0; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let frame = mixer.next_frame();
            assert!(frame.0 <= LIMITER_CEILING && frame.1 <= LIMITER_CEILING);
        }
        let _ = mixer.service_transitions();
        let _ = mixer.replace(dry(vec![1.0; 65], 1));
        for _ in 0..mixer.crossfade_frames {
            let frame = mixer.next_frame();
            for sample in [frame.0, frame.1] {
                assert!(
                    (floor - 1.0e-6..=LIMITER_CEILING).contains(&sample),
                    "{sample}"
                );
            }
        }
    }

    #[test]
    fn active_state_ramps_gain_while_the_source_clock_continues() {
        let sample_rate = 1_000;
        let mut mixer = MixerState::new(sample_rate);
        let _ = mixer.replace(dry(vec![0.5; 257], 1));
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
        let _ = mixer.replace(dry(vec![0.1, 0.2, 0.3, 0.4], 1));
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
        // The voice itself, before the room bus carries it into the shared
        // reverb, whose tail is not part of the voice's own bound.
        let mut mixer = MixerState::new(4_000);
        assert!(mixer.set_parameter_voice(110.0, 1.5, PARAMETER_MAX_GAIN));
        let first: Vec<_> = (0..300)
            .map(|_| mixer.parameter_voice.next_sample())
            .collect();
        assert!(
            first
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= PARAMETER_MAX_GAIN)
        );

        let before = *first.last().expect("first voice sample");
        assert!(mixer.set_parameter_voice(220.0, 1.25, PARAMETER_MAX_GAIN));
        let after = mixer.parameter_voice.next_sample();
        assert!(
            (after - before).abs() < 0.03,
            "parameter update stepped by {}",
            (after - before).abs()
        );
        for _ in 0..1_000 {
            let sample = mixer.parameter_voice.next_sample();
            assert!(sample.is_finite());
            assert!(sample.abs() <= PARAMETER_MAX_GAIN);
        }
    }

    #[test]
    fn parameter_glides_preserve_each_oscillators_frequency_bounds() {
        const SAMPLE_RATE: u32 = 8_000;
        let frequency = |before: f32, after: f32| {
            (after - before).rem_euclid(std::f32::consts::TAU) * SAMPLE_RATE as f32
                / std::f32::consts::TAU
        };
        for (from_root, from_ratio, to_root, to_ratio) in [
            // The second oscillator stays at 880 Hz in both directions.
            (110.0, 8.0, 880.0, 1.0),
            (880.0, 1.0, 110.0, 8.0),
            // A fixed root still supports an ascending or descending ratio.
            (146.83, 1.5, 146.83, 2.0),
            (146.83, 2.0, 146.83, 1.5),
        ] {
            let mut mixer = MixerState::new(SAMPLE_RATE);
            assert!(mixer.set_parameter_voice(from_root, from_ratio, 0.04));
            for _ in 0..400 {
                let _ = mixer.next_frame();
            }
            let before = (
                mixer.parameter_voice.root_phase,
                mixer.parameter_voice.ratio_phase,
            );
            assert!(mixer.set_parameter_voice(to_root, to_ratio, 0.04));
            assert_eq!(
                (
                    mixer.parameter_voice.root_phase,
                    mixer.parameter_voice.ratio_phase,
                ),
                before,
                "a new frequency target must preserve both oscillator phases"
            );
            let start = [from_root, from_root * from_ratio];
            let end = [to_root, to_root * to_ratio];
            let mut observed = [0.0; 2];
            for _ in 0..SAMPLE_RATE {
                let before = (
                    mixer.parameter_voice.root_phase,
                    mixer.parameter_voice.ratio_phase,
                );
                let _ = mixer.next_frame();
                // Measure the phase actually advanced by each oscillator,
                // independently of its cached frequency or smoothing rule.
                observed = [
                    frequency(before.0, mixer.parameter_voice.root_phase),
                    frequency(before.1, mixer.parameter_voice.ratio_phase),
                ];
                for axis in 0..2 {
                    assert!(
                        observed[axis] >= start[axis].min(end[axis]) - 0.03
                            && observed[axis] <= start[axis].max(end[axis]) + 0.03,
                        "oscillator {axis}: {} Hz escaped [{}, {}] Hz",
                        observed[axis],
                        start[axis].min(end[axis]),
                        start[axis].max(end[axis]),
                    );
                }
            }
            for axis in 0..2 {
                assert!(
                    (observed[axis] - end[axis]).abs() < 0.03,
                    "oscillator {axis}: {} Hz did not settle at {} Hz",
                    observed[axis],
                    end[axis]
                );
            }
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
    fn mixer_neutralizes_non_finite_source_samples() {
        let mut mixer = MixerState::new(48_000);
        let _ = mixer.replace(dry(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY], 1));
        for _ in 0..6 {
            assert_eq!(mixer.next_frame(), (0.0, 0.0));
        }
    }

    #[test]
    fn oneshot_plays_once_and_does_not_replace_the_loop() {
        let mut mixer = MixerState::new(8_000);
        mixer.current = dry(vec![0.25, 0.25, 0.25, 0.25], 1);
        let identity = mixer.current.identity;
        assert!(
            mixer
                .replace_oneshot(
                    Bus::Effect,
                    OneshotPlay::new(vec![0.5, 0.0], 1.0, 1).expect("one-shot")
                )
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
        assert!(mixer.service_oneshots()[Bus::Effect.index()].is_some());
        assert!(mixer.oneshots.iter().all(Option::is_none));
    }

    #[test]
    fn stereo_oneshot_preserves_pan_and_does_not_replace_the_loop() {
        let mut mixer = MixerState::new(8_000);
        mixer.current = dry(vec![0.1; 8], 1);
        let identity = mixer.current.identity;

        assert!(
            mixer
                .replace_oneshot(
                    Bus::Room,
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
            mixer.oneshots[Bus::Room.index()].is_some(),
            "the callback retains finished storage for control-thread replacement"
        );
        assert!(mixer.service_oneshots()[Bus::Room.index()].is_some());
        assert!(mixer.oneshots.iter().all(Option::is_none));
    }

    #[test]
    fn clearing_a_oneshot_stops_it_without_touching_the_loop() {
        let mut mixer = MixerState::new(8_000);
        mixer.current = dry(vec![0.1; 8], 1);
        let identity = mixer.current.identity;
        let next = OneshotPlay::new(vec![0.8; 16], 1.0, 1).expect("one-shot");
        assert!(mixer.replace_oneshot(Bus::Room, next).is_none());

        let retired = mixer.clear_oneshot(Bus::Room).expect("retired one-shot");
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
        let _ = mixer.replace(dry(vec![0.2; 64], 1));
        for _ in 0..mixer.crossfade_frames {
            let frame = mixer.next_frame();
            assert!((-1.0..=1.0).contains(&frame.0));
            assert!((-1.0..=1.0).contains(&frame.1));
        }
        let retired = mixer.service_transitions();
        assert!(retired.is_some());

        let _ = mixer.replace(dry(vec![0.6; 64], 1));
        while mixer.crossfade_remaining > 0 {
            let _ = mixer.next_frame();
        }
        assert!(mixer.retired.is_some());
        assert!(mixer.previous.is_none());
        assert!(mixer.service_transitions().is_some());
    }

    /// A short, quiet burst at the start of a long silent loop on `bus`.
    fn burst(bus: Bus, rate: u32) -> LoopBuffer {
        let mut samples = vec![0.0; rate as usize * 4];
        for (index, sample) in samples.iter_mut().take(rate as usize / 20).enumerate() {
            *sample = 0.5 * (index as f32 * 0.37).sin();
        }
        LoopBuffer::new(samples, 1, bus)
    }

    /// Output energy in the second after a burst on `bus` has ended.
    fn tail_energy_after_burst(bus: Bus) -> f32 {
        let rate = 8_000;
        let mut mixer = MixerState::new(rate);
        mixer.current = burst(bus, rate);
        for _ in 0..rate / 20 {
            let _ = mixer.next_frame();
        }
        (0..rate)
            .map(|_| {
                let frame = mixer.next_frame();
                frame.0.mul_add(frame.0, frame.1 * frame.1)
            })
            .sum()
    }

    #[test]
    fn room_sound_rings_in_the_shared_space_while_music_stays_dry() {
        let room = tail_energy_after_burst(Bus::Room);
        let effect = tail_energy_after_burst(Bus::Effect);
        let music = tail_energy_after_burst(Bus::Music);
        assert_eq!(music, 0.0, "recorded music is never reverberated again");
        assert!(room > 0.0, "a room sound leaves a tail");
        assert!(
            effect > 0.0 && effect < room,
            "cues are drier than room sound"
        );
        // The reverb is linear, so the tails differ by exactly the square of
        // the send ratio.
        let expected = (super::EFFECT_REVERB_SEND / super::ROOM_REVERB_SEND).powi(2);
        assert!(
            (effect / room / expected - 1.0).abs() < 1.0e-3,
            "effect to room tail energy {} against {expected}",
            effect / room
        );
    }

    /// Reverb energy left in the second after a steady tone on `bus` fades
    /// out over half a second, with or without a wash.
    fn tail_energy_after_leaving(bus: Bus, wash: bool) -> f32 {
        let rate = 8_000;
        let tone = (0..rate)
            .map(|index| 0.3 * (std::f32::consts::TAU * 220.0 * index as f32 / rate as f32).sin())
            .collect::<Vec<_>>();
        let mut mixer = MixerState::new(rate);
        mixer.current = LoopBuffer::new(tone, 1, bus);
        for _ in 0..rate {
            let _ = mixer.next_frame();
        }
        let fade = rate as usize / 2;
        assert!(
            mixer
                .replace_with_fade(LoopBuffer::new(vec![0.0; 64], 1, Bus::Music), fade, wash)
                .0
        );
        for _ in 0..fade {
            let _ = mixer.next_frame();
        }
        assert!(mixer.service_transitions().is_some());
        (0..rate)
            .map(|_| {
                let frame = mixer.next_frame();
                frame.0.mul_add(frame.0, frame.1 * frame.1)
            })
            .sum()
    }

    #[test]
    fn a_wash_dissolves_the_outgoing_room_into_its_own_tail() {
        let washed = tail_energy_after_leaving(Bus::Room, true);
        let faded = tail_energy_after_leaving(Bus::Room, false);
        assert!(
            washed > faded * 4.0,
            "a wash must leave much more of the room in the space: {washed} against {faded}"
        );
        // Music has no send, so a wash cannot reverberate it.
        assert_eq!(tail_energy_after_leaving(Bus::Music, true), 0.0);
        assert_eq!(tail_energy_after_leaving(Bus::Music, false), 0.0);
    }

    #[test]
    fn a_wash_starts_from_the_ordinary_send_and_swells_with_the_fade() {
        let mut mixer = MixerState::new(1_000);
        mixer.current = LoopBuffer::new(vec![0.3; 50], 1, Bus::Room);
        let _ = mixer.replace_with_fade(LoopBuffer::new(vec![0.1; 60], 1, Bus::Room), 500, true);
        assert_eq!(mixer.outgoing_wash(), 0.0, "no step into the swell");
        let mut previous = 0.0;
        while mixer.crossfade_remaining > 1 {
            let _ = mixer.next_frame();
            let swell = mixer.outgoing_wash();
            assert!(swell >= previous, "the swell only grows");
            previous = swell;
        }
        assert!((mixer.outgoing_wash() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn interrupting_a_wash_holds_its_swell_instead_of_dropping_it() {
        fn washing_after(frames: usize) -> MixerState {
            let mut mixer = MixerState::new(1_000);
            mixer.current = LoopBuffer::new(vec![0.3; 50], 1, Bus::Room);
            let _ =
                mixer.replace_with_fade(LoopBuffer::new(vec![0.1; 60], 1, Bus::Room), 500, true);
            for _ in 0..frames {
                let _ = mixer.next_frame();
            }
            mixer
        }

        // A quick change of source merges the outgoing mix and re-times the
        // fade. The swell it had reached must hold, or the reverb input
        // would step down and click a frame later.
        let mut interrupted = washing_after(200);
        let swell = interrupted.outgoing_wash();
        let (old_gain, _) = interrupted.crossfade_gains();
        assert!((0.3..0.5).contains(&swell), "{swell}");
        assert!(
            interrupted
                .replace(LoopBuffer::new(vec![-0.2; 70], 1, Bus::Room))
                .0
        );
        assert!(!interrupted.wash);
        assert_eq!(interrupted.outgoing_wash(), swell);
        let merged = interrupted.previous.as_ref().expect("merged outgoing mix");
        assert_eq!(merged.primary_wash, swell);
        assert!((merged.primary_gain - old_gain).abs() < 1.0e-6);

        // Returning to the incoming source re-times the same fade, and the
        // swell holds through that too.
        let mut rebased = washing_after(200);
        let swell = rebased.outgoing_wash();
        assert!(
            rebased
                .replace(LoopBuffer::new(vec![0.1; 60], 1, Bus::Room))
                .0
        );
        assert!(rebased.coefficient_ramp_start.is_some());
        assert_eq!(rebased.outgoing_wash(), swell);
        while rebased.crossfade_remaining > 0 {
            let _ = rebased.next_frame();
        }
        assert_eq!(
            rebased.outgoing_wash(),
            0.0,
            "a finished fade leaves no swell"
        );
    }

    #[test]
    fn a_deferred_wash_keeps_its_own_character() {
        let mut mixer = MixerState::new(1_000);
        let _ = mixer.replace_with_fade(dry(vec![0.2; 64], 1), 100, false);
        let _ = mixer.next_frame();
        let _ = mixer.replace_with_fade(LoopBuffer::new(vec![0.4; 64], 1, Bus::Room), 300, true);
        assert!(mixer.pending.as_ref().is_some_and(|pending| pending.wash));
        while mixer.crossfade_remaining > 0 {
            let _ = mixer.next_frame();
        }
        assert!(mixer.service_transitions().is_some());
        assert!(mixer.wash, "the deferred source washes when it starts");
        assert_eq!(mixer.crossfade_frames, 300);
    }

    #[test]
    fn the_same_content_on_another_bus_is_a_new_source() {
        let samples = Arc::new(vec![0.2, 0.3, 0.4]);
        let mut mixer = MixerState::new(1_000);
        mixer.current = LoopBuffer::new(samples.clone(), 1, Bus::Room);
        let (changed, _) = mixer.replace(LoopBuffer::new(samples.clone(), 1, Bus::Room));
        assert!(!changed, "the same source on the same bus keeps playing");
        let (changed, _) = mixer.replace(LoopBuffer::new(samples, 1, Bus::Music));
        assert!(
            changed,
            "a routing change crossfades rather than being ignored"
        );
        assert_eq!(mixer.current.bus, Bus::Music);
    }

    #[test]
    fn a_game_cue_and_a_room_event_play_together() {
        let mut mixer = MixerState::new(8_000);
        let room = OneshotPlay::new(vec![0.25; 4], 1.0, 1).expect("room event");
        let cue = OneshotPlay::new(vec![0.125; 4], 1.0, 1).expect("game cue");
        assert!(mixer.replace_oneshot(Bus::Room, room).is_none());
        assert!(
            mixer.replace_oneshot(Bus::Effect, cue).is_none(),
            "a cue does not displace a room event"
        );
        let frame = mixer.next_frame();
        assert!((frame.0 - 0.375).abs() < 1.0e-6, "{frame:?}");
        let next = OneshotPlay::new(vec![0.5; 4], 1.0, 1).expect("next cue");
        assert!(mixer.replace_oneshot(Bus::Effect, next).is_some());
        assert!(mixer.oneshots[Bus::Room.index()].is_some());
        assert!(mixer.clear_oneshot(Bus::Room).is_some());
        assert!(mixer.oneshots[Bus::Effect.index()].is_some());
    }

    #[test]
    fn every_bus_at_full_scale_stays_under_the_ceiling() {
        let rate = 8_000;
        let mut mixer = MixerState::new(rate);
        mixer.current = LoopBuffer::new(vec![1.0, 1.0, -1.0, -1.0], 1, Bus::Room);
        assert!(mixer.set_parameter_voice(220.0, 1.5, PARAMETER_MAX_GAIN));
        for (bus, level) in [(Bus::Effect, 1.0), (Bus::Music, -1.0), (Bus::Room, 1.0)] {
            let play = OneshotPlay::new(vec![level; rate as usize * 2], 1.0, 1).expect("one-shot");
            let _ = mixer.replace_oneshot(bus, play);
        }
        for _ in 0..rate * 3 {
            let frame = mixer.next_frame();
            for sample in [frame.0, frame.1] {
                assert!(sample.is_finite());
                assert!(sample.abs() <= LIMITER_CEILING, "{sample}");
            }
        }
    }

    #[test]
    fn a_hostile_source_cannot_poison_the_shared_tail() {
        let rate = 8_000;
        let mut mixer = MixerState::new(rate);
        mixer.current = LoopBuffer::new(vec![f32::NAN, 1.0e30, f32::INFINITY], 1, Bus::Room);
        for _ in 0..rate {
            let frame = mixer.next_frame();
            assert!(frame.0.is_finite() && frame.1.is_finite());
        }
        let _ = mixer.replace(LoopBuffer::new(vec![0.0; 64], 1, Bus::Room));
        // Even a send held at its bound decays past the flush floor well
        // inside half a minute.
        for _ in 0..rate * 24 {
            let _ = mixer.next_frame();
        }
        let _ = mixer.service_transitions();
        for _ in 0..rate {
            assert_eq!(mixer.next_frame(), (0.0, 0.0), "the tail ends in silence");
        }
    }
}
