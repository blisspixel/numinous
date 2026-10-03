//! The master chain: one shared reverb and a soft limiter.
//!
//! Pure, deterministic DSP with no device in sight, so every property the
//! audio callback relies on is a unit test here. All storage is allocated when
//! the mixer is built on the control thread; processing a frame never
//! allocates, locks, or calls into the system.
//!
//! The reverb is an eight-line feedback delay network after Jot and Chaigne
//! (1991). Its Householder feedback matrix is orthogonal, so it neither adds
//! nor removes energy, and the decay is set entirely by one gain per line,
//! computed from the target reverberation time. A one-pole low-pass in each
//! line gives high frequencies a shorter tail, the way air and soft surfaces
//! absorb them, and distinct prime line lengths keep the echoes from stacking
//! into a pitched ring. A short pre-delay keeps each dry attack clear, and four
//! input all-passes blur an onset into a wash before it enters the network.

use std::f64::consts::TAU;

/// Seconds for a low tone's tail to fall 60 dB (its RT60).
pub(crate) const DECAY_SECONDS: f64 = 2.2;
/// Seconds for a bright tone's tail, at [`BRIGHT_HZ`], to fall 60 dB.
const BRIGHT_DECAY_SECONDS: f64 = 1.1;
/// The frequency where [`BRIGHT_DECAY_SECONDS`] holds.
const BRIGHT_HZ: f64 = 6_000.0;
/// Silence before the first reflection.
const PRE_DELAY_SECONDS: f64 = 0.018;
/// Delay-line lengths at 48 kHz: distinct primes from 30 to 58 milliseconds.
const LINE_SAMPLES_48K: [usize; LINES] = [1_433, 1_601, 1_867, 2_053, 2_251, 2_399, 2_617, 2_797];
/// Input all-pass lengths at 48 kHz, 3.6 to 12.6 milliseconds.
const DIFFUSER_SAMPLES_48K: [usize; 4] = [173, 229, 443, 607];
/// All-pass coefficient: enough to smear an onset, not enough to ring.
const DIFFUSER_GAIN: f32 = 0.6;
const LINES: usize = 8;
/// Signs that read the two outputs from the lines. The patterns are
/// orthogonal, so the channels are decorrelated and a mono fold keeps the
/// sum of their power instead of cancelling it.
const LEFT_TAPS: [f32; LINES] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
const RIGHT_TAPS: [f32; LINES] = [1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0];
/// Input gain into every line.
const INPUT_GAIN: f32 = 0.5;
/// Output scale, calibrated so a steady broadband signal sent at full level
/// returns at about its own level (pinned by a test). A send level is then a
/// direct statement of how loud the wet return sits under the dry signal.
const OUTPUT_GAIN: f32 = 0.5;
/// Feedback values below this are flushed to zero. That is far below
/// hearing (about -400 dB) and far above where `f32` turns subnormal, so a
/// decaying tail reaches exact silence instead of crawling through the slow
/// subnormal range on x86.
const FLUSH_BELOW: f32 = 1.0e-20;

/// Level where the limiter starts to act: -3 dBFS.
pub(crate) const LIMITER_KNEE: f32 = 0.707_945_8;
/// The most the limiter ever passes: -0.3 dBFS.
pub(crate) const LIMITER_CEILING: f32 = 0.966_051;

/// Pass a sample unchanged below the knee and bend it smoothly toward the
/// ceiling above it.
///
/// This is a memoryless soft knee, not a look-ahead limiter. Below -3 dBFS it
/// is exactly the identity, so ordinary room sound is untouched. Above it, a
/// `tanh` curve that leaves the knee at unit slope bends peaks toward -0.3
/// dBFS, which adds gentle saturation on those peaks instead of the hard
/// clipping it replaces. Only a loud sum reaches it: full-scale music near
/// full master volume, or several buses peaking together.
pub(crate) fn soft_limit(sample: f32) -> f32 {
    let magnitude = sample.abs();
    if magnitude <= LIMITER_KNEE {
        return sample;
    }
    let room = LIMITER_CEILING - LIMITER_KNEE;
    let bent = LIMITER_KNEE + room * ((magnitude - LIMITER_KNEE) / room).tanh();
    bent.min(LIMITER_CEILING).copysign(sample)
}

fn flush(value: f32) -> f32 {
    if value.abs() < FLUSH_BELOW {
        0.0
    } else {
        value
    }
}

/// A fixed-length delay: `read` returns what was written `len` samples ago.
struct DelayLine {
    buffer: Vec<f32>,
    index: usize,
}

impl DelayLine {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            index: 0,
        }
    }

    fn read(&self) -> f32 {
        self.buffer[self.index]
    }

    fn write(&mut self, value: f32) {
        self.buffer[self.index] = value;
        self.index += 1;
        if self.index == self.buffer.len() {
            self.index = 0;
        }
    }
}

/// A Schroeder all-pass: flat magnitude, smeared phase.
struct AllPass {
    line: DelayLine,
}

impl AllPass {
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.line.read();
        let fed = flush(input - DIFFUSER_GAIN * delayed);
        self.line.write(fed);
        delayed + DIFFUSER_GAIN * fed
    }
}

/// The shared reverb: mono in, decorrelated stereo out.
pub(crate) struct Reverb {
    pre_delay: DelayLine,
    diffusers: [AllPass; 4],
    lines: [DelayLine; LINES],
    /// Per-line loop gain at low frequency.
    gains: [f32; LINES],
    /// Per-line one-pole coefficient; larger is darker.
    damping: [f32; LINES],
    /// Per-line one-pole state.
    damped: [f32; LINES],
}

impl Reverb {
    /// Build every line for `sample_rate`. This allocates, so it belongs on
    /// the control thread.
    pub(crate) fn new(sample_rate: u32) -> Self {
        let rate = f64::from(sample_rate.max(1));
        let scale = rate / 48_000.0;
        let lengths = distinct_primes(LINE_SAMPLES_48K.map(|length| scaled(length, scale)));
        let gains = lengths.map(|length| loop_gain(length, DECAY_SECONDS, rate));
        let damping = lengths.map(|length| {
            let low = f64::from(loop_gain(length, DECAY_SECONDS, rate));
            let high = f64::from(loop_gain(length, BRIGHT_DECAY_SECONDS, rate));
            one_pole_for_ratio(high / low, TAU * BRIGHT_HZ / rate)
        });
        Self {
            pre_delay: DelayLine::new((PRE_DELAY_SECONDS * rate).round() as usize),
            diffusers: DIFFUSER_SAMPLES_48K.map(|length| AllPass {
                line: DelayLine::new(scaled(length, scale)),
            }),
            lines: lengths.map(DelayLine::new),
            gains,
            damping,
            damped: [0.0; LINES],
        }
    }

    /// Process one stereo send frame and return the wet stereo frame.
    pub(crate) fn process(&mut self, send: (f32, f32)) -> (f32, f32) {
        let delayed = self.pre_delay.read();
        self.pre_delay.write(flush((send.0 + send.1) * 0.5));
        let diffused = self
            .diffusers
            .iter_mut()
            .fold(delayed, |signal, diffuser| diffuser.process(signal));

        let outputs = self.lines.each_ref().map(DelayLine::read);
        let mut returned = [0.0f32; LINES];
        for (((state, returned), output), (pole, gain)) in self
            .damped
            .iter_mut()
            .zip(returned.iter_mut())
            .zip(outputs)
            .zip(self.damping.iter().zip(self.gains))
        {
            *state = flush(output + pole * (*state - output));
            *returned = *state * gain;
        }
        // Householder reflection, I - (2 / N) 11^T: orthogonal, and it costs
        // one sum instead of a matrix product.
        let reflected = returned.iter().sum::<f32>() * (2.0 / LINES as f32);
        for (line, returned) in self.lines.iter_mut().zip(returned) {
            line.write(returned - reflected + diffused * INPUT_GAIN);
        }

        let (left, right) = outputs
            .iter()
            .zip(LEFT_TAPS.iter().zip(RIGHT_TAPS))
            .fold((0.0f32, 0.0f32), |(left, right), (output, (l, r))| {
                (output.mul_add(*l, left), output.mul_add(r, right))
            });
        (left * OUTPUT_GAIN, right * OUTPUT_GAIN)
    }
}

fn scaled(length_at_48k: usize, scale: f64) -> usize {
    ((length_at_48k as f64 * scale).round() as usize).max(1)
}

/// The gain that makes a loop of `length` samples fall 60 dB in `seconds`.
fn loop_gain(length: usize, seconds: f64, rate: f64) -> f32 {
    10f64.powf(-3.0 * length as f64 / (seconds * rate)) as f32
}

/// The one-pole low-pass `y = x + a (y' - x)` whose gain at `omega` radians
/// per sample is `ratio` of its unity gain at DC.
///
/// Solving `(1 - a)^2 = ratio^2 (1 - 2a cos(omega) + a^2)` for the root in
/// `[0, 1)`. Above Nyquist the target is moved to Nyquist, where the answer is
/// `(1 - ratio) / (1 + ratio)`.
fn one_pole_for_ratio(ratio: f64, omega: f64) -> f32 {
    let ratio = ratio.clamp(0.0, 1.0);
    if ratio >= 1.0 {
        return 0.0;
    }
    let cosine = omega.min(std::f64::consts::PI).cos();
    let r2 = ratio * ratio;
    let b = 1.0 - r2 * cosine;
    let c = 1.0 - r2;
    ((b - (b * b - c * c).max(0.0).sqrt()) / c) as f32
}

/// Nudge each length up to a prime not already used, in order.
fn distinct_primes(lengths: [usize; LINES]) -> [usize; LINES] {
    let mut chosen = [0usize; LINES];
    for (index, length) in lengths.into_iter().enumerate() {
        let mut candidate = length.max(2);
        while !is_prime(candidate) || chosen[..index].contains(&candidate) {
            candidate += 1;
        }
        chosen[index] = candidate;
    }
    chosen
}

fn is_prime(value: usize) -> bool {
    value >= 2
        && (2..)
            .take_while(|divisor| divisor * divisor <= value)
            .all(|divisor| !value.is_multiple_of(divisor))
}

#[cfg(test)]
mod tests {
    use super::{
        DECAY_SECONDS, LIMITER_CEILING, LIMITER_KNEE, LINES, Reverb, distinct_primes, is_prime,
        loop_gain, one_pole_for_ratio, soft_limit,
    };

    /// A fixed-seed uniform noise source in `[-1, 1)`, so tests need no RNG.
    struct Noise(u64);

    impl Noise {
        fn next(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
        }
    }

    fn rms(samples: impl IntoIterator<Item = f32>) -> f64 {
        let (sum, count) = samples
            .into_iter()
            .fold((0.0f64, 0usize), |(sum, count), sample| {
                (sum + f64::from(sample) * f64::from(sample), count + 1)
            });
        (sum / count.max(1) as f64).sqrt()
    }

    /// Seconds for the Schroeder energy decay curve of the impulse response
    /// to fall 60 dB, measured over the curve's -5 to -35 dB span and
    /// extrapolated, the way an RT60 is read from a real room.
    fn measured_decay_seconds(rate: u32, seconds: f64) -> f64 {
        let mut reverb = Reverb::new(rate);
        let frames = (f64::from(rate) * seconds) as usize;
        let mut energy = Vec::with_capacity(frames);
        for index in 0..frames {
            let input = if index == 0 { 1.0 } else { 0.0 };
            let (left, right) = reverb.process((input, input));
            energy.push(f64::from(left).powi(2) + f64::from(right).powi(2));
        }
        let mut remaining = energy.iter().sum::<f64>();
        let total = remaining;
        let mut at_minus_5 = None;
        let mut at_minus_35 = None;
        for (index, sample) in energy.iter().enumerate() {
            let level = 10.0 * (remaining / total).log10();
            if at_minus_5.is_none() && level <= -5.0 {
                at_minus_5 = Some(index);
            }
            if level <= -35.0 {
                at_minus_35 = Some(index);
                break;
            }
            remaining -= sample;
        }
        let span = (at_minus_35.expect("-35 dB") - at_minus_5.expect("-5 dB")) as f64;
        span / f64::from(rate) * 2.0
    }

    #[test]
    fn an_impulse_decays_on_the_designed_schedule() {
        // The broadband tail is a little shorter than the low-frequency
        // design time because bright energy is damped faster. It must still
        // land between three quarters of the design time and the time itself.
        for rate in [16_000, 48_000] {
            let decay = measured_decay_seconds(rate, 4.0);
            assert!(
                (DECAY_SECONDS * 0.75..=DECAY_SECONDS * 1.05).contains(&decay),
                "{rate} Hz: measured RT60 {decay:.3} s"
            );
        }
    }

    #[test]
    fn a_low_tone_rings_for_the_design_decay_time() {
        // Feed a steady 150 Hz tone, stop it, and read the tail's decay rate
        // between -5 and -35 dB of its starting level. At low frequency the
        // damping filter is transparent, so the tail should fall 60 dB in
        // the design time.
        let rate = 16_000;
        let mut reverb = Reverb::new(rate);
        for index in 0..rate * 2 {
            let sample =
                (std::f64::consts::TAU * 150.0 * f64::from(index) / f64::from(rate)).sin() as f32;
            let _ = reverb.process((sample, sample));
        }
        let window = (rate / 20) as usize;
        let levels = (0..60)
            .map(|_| rms((0..window).map(|_| reverb.process((0.0, 0.0)).0)))
            .collect::<Vec<_>>();
        let start = levels[0];
        let db = |level: f64| 20.0 * (level / start).log10();
        let first = levels
            .iter()
            .position(|level| db(*level) <= -5.0)
            .expect("-5 dB");
        let last = levels
            .iter()
            .position(|level| db(*level) <= -35.0)
            .expect("-35 dB");
        let seconds_per_db = (last - first) as f64 * window as f64
            / f64::from(rate)
            / (db(levels[first]) - db(levels[last]));
        let decay = seconds_per_db * 60.0;
        assert!(
            (DECAY_SECONDS * 0.85..=DECAY_SECONDS * 1.15).contains(&decay),
            "low-tone RT60 {decay:.3} s"
        );
    }

    #[test]
    fn a_full_send_of_broadband_sound_returns_near_its_own_level() {
        let rate = 48_000;
        let mut reverb = Reverb::new(rate);
        let mut noise = Noise(7);
        let mut input = Vec::new();
        let mut output = Vec::new();
        for index in 0..rate * 4 {
            let sample = noise.next() * 0.25;
            let (left, right) = reverb.process((sample, sample));
            if index >= rate * 2 {
                input.push(sample);
                output.push(left);
                output.push(right);
            }
        }
        let ratio_db = 20.0 * (rms(output) / rms(input)).log10();
        assert!(
            ratio_db.abs() < 3.0,
            "wet return sits {ratio_db:.2} dB from its send"
        );
    }

    #[test]
    fn full_scale_noise_stays_bounded_and_finite() {
        let mut reverb = Reverb::new(48_000);
        let mut noise = Noise(11);
        let mut peak = 0.0f32;
        for _ in 0..48_000 * 6 {
            let (left, right) = reverb.process((noise.next(), noise.next()));
            assert!(left.is_finite() && right.is_finite());
            peak = peak.max(left.abs()).max(right.abs());
        }
        assert!(peak < 4.0, "the tail peaked at {peak}");
    }

    #[test]
    fn a_tail_reaches_exact_silence_without_crossing_the_subnormal_range() {
        // Thirty seconds of silence carries a full-scale tail far past where
        // an unguarded f32 network would have gone subnormal.
        let rate = 8_000;
        let mut reverb = Reverb::new(rate);
        let mut noise = Noise(3);
        for _ in 0..rate / 4 {
            let _ = reverb.process((noise.next(), noise.next()));
        }
        let mut silent_since = None;
        for index in 0..rate * 30 {
            let (left, right) = reverb.process((0.0, 0.0));
            for sample in [left, right] {
                assert!(
                    sample == 0.0 || sample.is_normal(),
                    "subnormal output {sample:e} at {index}"
                );
            }
            if left == 0.0 && right == 0.0 {
                silent_since.get_or_insert(index);
            } else {
                silent_since = None;
            }
        }
        let silent_since = silent_since.expect("the tail ends in exact silence");
        assert!(
            silent_since < rate * 25,
            "silent only from frame {silent_since}"
        );
        assert!(
            reverb
                .damped
                .iter()
                .chain(reverb.lines.iter().flat_map(|line| line.buffer.iter()))
                .all(|state| *state == 0.0),
            "every line state is exactly zero"
        );
    }

    #[test]
    fn a_mono_fold_keeps_the_wet_signal() {
        let mut reverb = Reverb::new(48_000);
        let mut noise = Noise(5);
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut folded = Vec::new();
        for index in 0..48_000 * 3 {
            let sample = noise.next() * 0.25;
            let frame = reverb.process((sample, sample));
            if index >= 48_000 {
                left.push(frame.0);
                right.push(frame.1);
                folded.push((frame.0 + frame.1) * 0.5);
            }
        }
        let channels = (rms(left) + rms(right)) * 0.5;
        let fold_db = 20.0 * (rms(folded) / channels).log10();
        assert!(fold_db > -6.0, "the mono fold lost {fold_db:.2} dB");
    }

    #[test]
    fn the_reverb_is_deterministic_and_linear() {
        let mut first = Reverb::new(44_100);
        let mut second = Reverb::new(44_100);
        let mut half = Reverb::new(44_100);
        let mut noise = Noise(9);
        for _ in 0..44_100 {
            let input = (noise.next(), noise.next());
            let a = first.process(input);
            assert_eq!(a, second.process(input));
            let h = half.process((input.0 * 0.5, input.1 * 0.5));
            assert!((a.0 * 0.5 - h.0).abs() < 1.0e-5 && (a.1 * 0.5 - h.1).abs() < 1.0e-5);
        }
    }

    #[test]
    fn every_supported_rate_builds_distinct_prime_lines() {
        for rate in [
            8_000, 16_000, 22_050, 44_100, 48_000, 96_000, 192_000, 384_000,
        ] {
            let reverb = Reverb::new(rate);
            let lengths = reverb.lines.each_ref().map(|line| line.buffer.len());
            for (index, length) in lengths.iter().enumerate() {
                assert!(is_prime(*length), "{rate} Hz line {index} is {length}");
                assert!(
                    !lengths[..index].contains(length),
                    "{rate} Hz repeats {length}"
                );
            }
            let milliseconds = lengths.map(|length| length as f64 * 1_000.0 / f64::from(rate));
            assert!(
                milliseconds.iter().all(|ms| (29.0..61.0).contains(ms)),
                "{rate} Hz lines {milliseconds:?}"
            );
            assert!(reverb.gains.iter().all(|gain| (0.0..1.0).contains(gain)));
            assert!(reverb.damping.iter().all(|pole| (0.0..1.0).contains(pole)));
        }
        let mut fast = Reverb::new(384_000);
        for index in 0..384_000 / 4 {
            let input = if index == 0 { 1.0 } else { 0.0 };
            let (left, right) = fast.process((input, input));
            assert!(left.is_finite() && right.is_finite() && left.abs() < 2.0);
        }
    }

    #[test]
    fn the_design_helpers_hit_their_targets() {
        // A loop gain applied once per pass reaches -60 dB in the design time.
        let rate = 48_000.0;
        let gain = f64::from(loop_gain(2_400, DECAY_SECONDS, rate));
        let passes = DECAY_SECONDS * rate / 2_400.0;
        assert!((20.0 * gain.log10() * passes + 60.0).abs() < 0.01);

        // The one-pole lands on the requested ratio at the requested frequency.
        for (ratio, omega) in [(0.9, 0.5), (0.5, 1.2), (0.95, 3.0), (0.7, 4.0)] {
            let pole = f64::from(one_pole_for_ratio(ratio, omega));
            let omega = f64::min(omega, std::f64::consts::PI);
            let magnitude = (1.0 - pole) / (1.0 - 2.0 * pole * omega.cos() + pole * pole).sqrt();
            assert!(
                (magnitude - ratio).abs() < 1.0e-4,
                "{ratio} at {omega}: {magnitude}"
            );
        }
        assert_eq!(one_pole_for_ratio(1.0, 1.0), 0.0);

        assert_eq!(
            distinct_primes([10; LINES]),
            [11, 13, 17, 19, 23, 29, 31, 37]
        );
        assert!(!is_prime(0) && !is_prime(1) && is_prime(2) && !is_prime(91));
    }

    #[test]
    fn the_limiter_is_exact_below_its_knee_and_never_passes_its_ceiling() {
        for step in 0..=10_000 {
            let sample = LIMITER_KNEE * step as f32 / 10_000.0;
            assert_eq!(soft_limit(sample), sample);
            assert_eq!(soft_limit(-sample), -sample);
        }
        // Up to +24 dB over full scale, and beyond.
        let mut previous = soft_limit(LIMITER_KNEE);
        for step in 1..=20_000 {
            let sample = LIMITER_KNEE + step as f32 * (15.85 - LIMITER_KNEE) / 20_000.0;
            let limited = soft_limit(sample);
            assert!(limited <= LIMITER_CEILING, "{sample} -> {limited}");
            assert!(limited >= previous, "the curve never folds back");
            assert_eq!(soft_limit(-sample), -limited, "symmetric");
            previous = limited;
        }
        for extreme in [f32::MAX, f32::INFINITY] {
            let limited = soft_limit(extreme);
            assert!(limited <= LIMITER_CEILING && LIMITER_CEILING - limited < 1.0e-6);
            assert_eq!(soft_limit(-extreme), -limited);
        }
        // It leaves the knee smoothly: one small step above the knee moves
        // the output by almost exactly the same small step.
        let nudge = 1.0e-3;
        let slope = (soft_limit(LIMITER_KNEE + nudge) - LIMITER_KNEE) / nudge;
        assert!((slope - 1.0).abs() < 0.01, "slope at the knee {slope}");
    }
}
