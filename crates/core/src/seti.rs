//! SETI: find the signal in the noise, the step before you talk back.
//!
//! Nature is full of rhythms, a pulsar blinks like a metronome, but nature does
//! not count in prime numbers. You scan several channels of static near the
//! hydrogen line and pick the one that is artificial: the one counting 2, 3, 5,
//! 7, 11. Only then have you found someone to talk to (see [`crate::aliens`]).
//! Deterministic from a seed. See `docs/PLAYFUL.md`.

use crate::rng::SplitMix64;

/// Decorrelates the SETI seed from other seeded systems.
const SETI_MIX: u64 = 0x5E71_0000_0FFE_ED01;
/// The prime run an intelligent signal transmits.
const PRIMES: [usize; 5] = [2, 3, 5, 7, 11];
/// The letters used to label channels.
const LETTERS: [char; 6] = ['A', 'B', 'C', 'D', 'E', 'F'];

/// What is actually on a channel (hidden from the player).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalKind {
    Noise,
    Pulsar,
    Artificial,
}

/// One scanned channel: a labeled frequency and its blip trace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetiChannel {
    /// The letter the player types to pick this channel.
    pub letter: char,
    /// A frequency label near the 1420 MHz hydrogen line.
    pub frequency: String,
    /// The received blips, `#` for a pulse and spaces for silence.
    pub trace: String,
}

/// A full scan: several channels, one of which is artificial.
#[derive(Debug, Clone)]
pub struct SetiScan {
    /// The channels, shuffled.
    pub channels: Vec<SetiChannel>,
    /// The letter of the artificial channel.
    pub answer: char,
    /// The artificial channel's frequency, for the reveal.
    pub answer_frequency: String,
}

/// The artificial trace: pulse groups counting the primes.
fn prime_trace() -> String {
    PRIMES
        .iter()
        .map(|&n| "#".repeat(n))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A pulsar: perfectly regular pulses (structured, but not intelligent).
fn pulsar_trace(rng: &mut SplitMix64) -> String {
    let group = 1 + rng.below(2) as usize; // 1 or 2 pulses
    let gap = 3 + rng.below(3) as usize; // constant spacing
    let unit = format!("{}{}", "#".repeat(group), " ".repeat(gap));
    unit.repeat(6).trim_end().to_string()
}

/// Noise: random runs and gaps, no pattern.
fn noise_trace(rng: &mut SplitMix64) -> String {
    let mut out = String::new();
    while out.len() < 30 {
        let run = 1 + rng.below(4) as usize;
        let gap = 1 + rng.below(3) as usize;
        out.push_str(&"#".repeat(run));
        out.push_str(&" ".repeat(gap));
    }
    out.trim_end().to_string()
}

/// Build a deterministic SETI scan with `channels` channels (one artificial).
#[must_use]
pub fn build_scan(seed: u64, channels: usize) -> SetiScan {
    let count = channels.clamp(2, LETTERS.len());
    let mut rng = SplitMix64::new(seed ^ SETI_MIX);
    let answer_index = rng.below(count as u64) as usize;

    let mut out = Vec::with_capacity(count);
    let mut answer = LETTERS[0];
    let mut answer_frequency = String::new();
    for (i, &letter) in LETTERS.iter().take(count).enumerate() {
        // Frequencies clustered around the 1420.4 MHz hydrogen line.
        let frequency = format!("{:.1} MHz", 1418.0 + (rng.below(60) as f64) / 10.0);
        let (kind, trace) = if i == answer_index {
            (SignalKind::Artificial, prime_trace())
        } else if rng.below(2) == 0 {
            (SignalKind::Pulsar, pulsar_trace(&mut rng))
        } else {
            (SignalKind::Noise, noise_trace(&mut rng))
        };
        if kind == SignalKind::Artificial {
            answer = letter;
            answer_frequency = frequency.clone();
        }
        out.push(SetiChannel {
            letter,
            frequency,
            trace,
        });
    }

    SetiScan {
        channels: out,
        answer,
        answer_frequency,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_scan, prime_trace};

    #[test]
    fn a_scan_has_exactly_one_artificial_channel() {
        for seed in 0..30 {
            let scan = build_scan(seed, 4);
            let artificial = scan
                .channels
                .iter()
                .filter(|c| c.trace == prime_trace())
                .count();
            assert_eq!(artificial, 1, "seed {seed} did not have one signal");
        }
    }

    #[test]
    fn the_answer_points_at_the_artificial_channel() {
        let scan = build_scan(7, 4);
        let chosen = scan
            .channels
            .iter()
            .find(|c| c.letter == scan.answer)
            .unwrap();
        assert_eq!(chosen.trace, prime_trace());
        assert_eq!(chosen.frequency, scan.answer_frequency);
    }

    #[test]
    fn scans_are_deterministic() {
        assert_eq!(
            build_scan(3, 4).answer_frequency,
            build_scan(3, 4).answer_frequency
        );
    }

    #[test]
    fn channel_count_is_clamped() {
        assert_eq!(build_scan(1, 99).channels.len(), 6);
        assert_eq!(build_scan(1, 0).channels.len(), 2);
    }
}
