//! How a note speaks: the shared envelope of the house voice.
//!
//! Every pitched note in the room bed and in a rendered [`crate::SoundSpec`]
//! is shaped here, so the whole Cabinet articulates as one instrument. A note
//! rises over a smooth attack, decays exponentially toward a sustained level
//! while its gate is open, and releases exponentially once the gate closes.
//! Exponential decay models a struck or plucked body's falling amplitude;
//! it replaces the previous symmetric raised-cosine note envelope.
//!
//! Rendering is pure and allocation free. Each segment's per-sample factor is
//! computed once per note and the envelope advances by multiplication, so the
//! cost per sample is a multiply and an add. The attack uses a polynomial
//! smoothstep rather than a cosine, so it needs no transcendental function.

/// The named ways a note can speak.
///
/// These are the house voice's articulations. Their timings are a hypothesis
/// about taste, chosen for soft low-register beds and pinned by tests; no
/// listening study has confirmed them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Articulation {
    /// A soft sine lead: a quick bloom that settles to a held tone and rings
    /// briefly after it lifts.
    Bloom,
    /// A plucked triangle lead: a fast attack that falls toward a quiet
    /// sustain, like a string left to ring.
    Pluck,
    /// A low anchor: a slow swell that holds and fades over a long release.
    Swell,
}

/// The timing of one articulation, in seconds and linear level.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shape {
    /// Rise from silence to full level.
    attack: f32,
    /// Time to settle within five percent of the sustain level.
    decay: f32,
    /// Level held while the gate stays open, relative to the peak.
    sustain: f32,
    /// Fall from the gate-close level to exact silence.
    release: f32,
}

/// How far below its start an exponential release would sit when it ends,
/// before the floor is subtracted: e^-5, about -43 dB. Subtracting that floor
/// lands the release on exact zero without changing its exponential shape.
const RELEASE_DEPTH: f64 = 5.0;
/// An exponential settles to within e^-3, about five percent, after three time
/// constants, which is what a stated decay time means here.
const DECAY_TIME_CONSTANTS: f64 = 3.0;

impl Articulation {
    /// Stable lowercase identifier for structured projections.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Bloom => "bloom",
            Self::Pluck => "pluck",
            Self::Swell => "swell",
        }
    }

    /// Seconds of sound after the gate closes.
    #[must_use]
    pub const fn release_seconds(self) -> f32 {
        self.shape().release
    }

    const fn shape(self) -> Shape {
        match self {
            Self::Bloom => Shape {
                attack: 0.012,
                decay: 0.18,
                sustain: 0.7,
                release: 0.22,
            },
            Self::Pluck => Shape {
                attack: 0.006,
                decay: 0.16,
                sustain: 0.45,
                release: 0.18,
            },
            Self::Swell => Shape {
                attack: 0.04,
                decay: 0.0,
                sustain: 1.0,
                release: 0.3,
            },
        }
    }
}

/// One note's envelope, advanced one sample at a time.
///
/// The gate is open for `gate` samples, then the release runs for its own
/// length and ends on exact zero. A gate shorter than the attack releases from
/// wherever the attack had reached, so a short note never jumps.
#[derive(Debug, Clone)]
pub(crate) struct Envelope {
    index: usize,
    gate: usize,
    attack: usize,
    release: usize,
    sustain: f64,
    /// Remaining distance above the sustain level, `e^(-t / tau)`.
    decay_excess: f64,
    decay_factor: f64,
    /// Level when the gate closed, scaled by the release's floor correction.
    release_level: f64,
    /// `e^(-depth * t / release)`, the uncorrected release curve.
    release_curve: f64,
    release_factor: f64,
    release_floor: f64,
    level: f64,
}

impl Envelope {
    /// An envelope whose gate is open for `gate` samples, with its release
    /// following after the gate.
    pub(crate) fn new(articulation: Articulation, gate: usize, sample_rate: u32) -> Self {
        let shape = articulation.shape();
        Self::with_shape(
            shape,
            gate,
            samples(shape.release, sample_rate),
            sample_rate,
        )
    }

    /// An envelope fitted inside `total` samples: the release is shortened
    /// when needed so it never runs past the end, taking at most half the note.
    pub(crate) fn fitted(articulation: Articulation, total: usize, sample_rate: u32) -> Self {
        let shape = articulation.shape();
        let release = samples(shape.release, sample_rate).min(total / 2);
        Self::with_shape(shape, total - release, release, sample_rate)
    }

    fn with_shape(shape: Shape, gate: usize, release: usize, sample_rate: u32) -> Self {
        let decay_samples = f64::from(shape.decay) * f64::from(sample_rate.max(1));
        let decay_factor = if decay_samples > 0.0 {
            (-DECAY_TIME_CONSTANTS / decay_samples).exp()
        } else {
            0.0
        };
        let release_floor = (-RELEASE_DEPTH).exp();
        Self {
            index: 0,
            gate,
            attack: samples(shape.attack, sample_rate).max(1),
            release,
            sustain: f64::from(shape.sustain),
            decay_excess: 1.0,
            decay_factor,
            release_level: 0.0,
            release_curve: 1.0,
            release_factor: if release > 0 {
                (-RELEASE_DEPTH / release as f64).exp()
            } else {
                0.0
            },
            release_floor,
            level: 0.0,
        }
    }

    /// Samples from the note's start to the release's exact zero.
    pub(crate) fn len(&self) -> usize {
        self.gate + self.release
    }

    /// The level of the next sample, in `[0, 1]`.
    pub(crate) fn next_level(&mut self) -> f32 {
        let index = self.index;
        self.index = self.index.saturating_add(1);
        if index < self.gate {
            self.level = if index < self.attack {
                // Smoothstep from silence: zero slope at both ends, so the
                // onset has no corner a listener hears as a tick.
                let u = index as f64 / self.attack as f64;
                u * u * (3.0 - 2.0 * u)
            } else {
                let level = self.sustain + (1.0 - self.sustain) * self.decay_excess;
                self.decay_excess *= self.decay_factor;
                level
            };
            return self.level as f32;
        }
        let into_release = index - self.gate;
        if into_release >= self.release {
            return 0.0;
        }
        if into_release == 0 {
            self.release_level = self.level / (1.0 - self.release_floor);
        }
        let level = self.release_level * (self.release_curve - self.release_floor);
        self.release_curve *= self.release_factor;
        level.max(0.0) as f32
    }
}

fn samples(seconds: f32, sample_rate: u32) -> usize {
    (f64::from(seconds) * f64::from(sample_rate.max(1))).round() as usize
}

#[cfg(test)]
mod tests {
    use super::{Articulation, Envelope, samples};

    const ALL: [Articulation; 3] = [
        Articulation::Bloom,
        Articulation::Pluck,
        Articulation::Swell,
    ];

    fn levels(mut envelope: Envelope) -> Vec<f32> {
        (0..envelope.len() + 64)
            .map(|_| envelope.next_level())
            .collect()
    }

    #[test]
    fn every_articulation_rises_settles_and_releases_to_exact_silence() {
        let rate = 48_000;
        for articulation in ALL {
            let shape = articulation.shape();
            let gate = samples(1.0, rate);
            let envelope = Envelope::new(articulation, gate, rate);
            assert_eq!(envelope.len(), gate + samples(shape.release, rate));
            let curve = levels(envelope);
            let attack = samples(shape.attack, rate);
            assert_eq!(curve[0], 0.0, "{articulation:?} starts in silence");
            assert!(
                curve[..attack].windows(2).all(|pair| pair[1] >= pair[0]),
                "{articulation:?} attack rises"
            );
            assert!(
                (curve[attack] - 1.0).abs() < 1.0e-6,
                "{articulation:?} peaks at full level"
            );
            assert!(
                curve[attack..gate]
                    .windows(2)
                    .all(|pair| pair[1] <= pair[0]),
                "{articulation:?} decays without bouncing"
            );
            let settled = curve[attack + samples(shape.decay, rate)];
            let excess = (settled - shape.sustain) / (1.0 - shape.sustain).max(f32::EPSILON);
            assert!(
                shape.sustain == 1.0 || (0.0..=0.051).contains(&excess),
                "{articulation:?} settles within five percent: {excess}"
            );
            assert!(
                curve[gate..].windows(2).all(|pair| pair[1] <= pair[0]),
                "{articulation:?} release falls"
            );
            let end = gate + samples(shape.release, rate);
            assert!(
                curve[end - 1] < 1.0e-3,
                "{articulation:?} release nearly done"
            );
            assert!(
                curve[end..].iter().all(|level| *level == 0.0),
                "{articulation:?} ends in exact silence"
            );
            let largest_step = curve
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).abs())
                .fold(0.0f32, f32::max);
            assert!(
                largest_step <= 1.6 / attack as f32,
                "{articulation:?} stepped by {largest_step}"
            );
        }
    }

    #[test]
    fn a_short_gate_releases_from_where_the_attack_reached() {
        let rate = 48_000;
        let envelope = Envelope::new(Articulation::Swell, 100, rate);
        let curve = levels(envelope);
        let reached = curve[99];
        assert!(reached > 0.0 && reached < 0.1, "{reached}");
        assert_eq!(
            curve[100], reached,
            "the release starts where the note let go"
        );
        assert!(curve[100..].windows(2).all(|pair| pair[1] <= pair[0]));
        assert!(curve.last().is_some_and(|level| *level == 0.0));
    }

    #[test]
    fn a_fitted_envelope_ends_inside_its_note() {
        let rate = 16_000;
        for total in [0, 1, 2, 7, 160, 1_600, 16_000] {
            for articulation in ALL {
                let mut envelope = Envelope::fitted(articulation, total, rate);
                assert_eq!(envelope.len(), total, "{articulation:?} over {total}");
                let curve = (0..total)
                    .map(|_| envelope.next_level())
                    .collect::<Vec<_>>();
                assert!(curve.iter().all(|level| (0.0..=1.0).contains(level)));
                if total > 0 {
                    assert_eq!(curve[0], 0.0);
                    assert!(
                        curve[total - 1] < 0.05,
                        "{articulation:?} {total} ends quiet"
                    );
                }
                assert_eq!(envelope.next_level(), 0.0, "no tail past the note");
            }
        }
    }

    #[test]
    fn timing_follows_the_clock_not_the_sample_count() {
        for articulation in ALL {
            let at = |rate: u32| {
                let gate = samples(0.5, rate);
                let curve = levels(Envelope::new(articulation, gate, rate));
                let index = |seconds: f32| samples(seconds, rate);
                (curve[index(0.1)], curve[index(0.5) + index(0.05)])
            };
            let (slow_hold, slow_release) = at(16_000);
            let (fast_hold, fast_release) = at(192_000);
            assert!(
                (slow_hold - fast_hold).abs() < 0.01,
                "{articulation:?} hold"
            );
            assert!(
                (slow_release - fast_release).abs() < 0.01,
                "{articulation:?} release"
            );
        }
        assert_eq!(Articulation::Bloom.id(), "bloom");
        assert_eq!(Articulation::Pluck.id(), "pluck");
        assert_eq!(Articulation::Swell.id(), "swell");
        assert!(
            ALL.iter()
                .all(|articulation| articulation.release_seconds() > 0.1)
        );
    }
}
