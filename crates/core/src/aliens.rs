//! Talk to the Aliens: they speak only in number sequences.
//!
//! The aliens transmit the start of a famous integer sequence; you answer with
//! the next term to prove you understand. Any species that does math will know
//! these, which is exactly why they were proposed as a first contact language.
//! Deterministic from a seed. See `docs/PLAYFUL.md`.

use crate::rng::SplitMix64;

/// Decorrelates the alien seed from other seeded systems.
const ALIEN_MIX: u64 = 0xA11E_45EE_D000_0001;

/// A first-contact puzzle: some terms shown, one term to answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlienMessage {
    /// The sequence's name, revealed only after the guess.
    pub name: &'static str,
    /// The terms the aliens transmitted.
    pub terms: Vec<u64>,
    /// The next term (the correct answer).
    pub answer: u64,
    /// The explanation, revealed after the guess.
    pub explanation: &'static str,
}

/// A sequence the aliens might speak: its name, an explanation, and a generator
/// mapping a 0-based index to that term.
type Sequence = (&'static str, &'static str, fn(usize) -> u64);

/// The sequences the aliens might speak. Each generates its first `count` terms.
const SEQUENCES: [Sequence; 5] = [
    (
        "the prime numbers",
        "Every number divisible only by itself and one, the atoms of arithmetic.",
        nth_prime,
    ),
    (
        "the Fibonacci numbers",
        "Each term is the sum of the two before it; it hides in sunflowers and pine cones.",
        nth_fibonacci,
    ),
    (
        "the square numbers",
        "n times n. The gaps between them are exactly the odd numbers.",
        nth_square,
    ),
    (
        "the triangular numbers",
        "1, then 1+2, then 1+2+3: how many dots stack into a triangle.",
        nth_triangular,
    ),
    (
        "the powers of two",
        "Double each time, the heartbeat of every computer.",
        nth_power_of_two,
    ),
];

/// The `n`-th prime (0-indexed: `nth_prime(0) == 2`).
fn nth_prime(n: usize) -> u64 {
    let mut found = 0usize;
    let mut candidate = 1u64;
    loop {
        candidate += 1;
        if is_prime(candidate) {
            if found == n {
                return candidate;
            }
            found += 1;
        }
    }
}

/// Trial-division primality for small numbers.
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

/// The `n`-th Fibonacci number (0-indexed: 1, 1, 2, 3, 5, ...).
fn nth_fibonacci(n: usize) -> u64 {
    let (mut a, mut b) = (1u64, 1u64);
    for _ in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

/// The `n`-th square (0-indexed: 1, 4, 9, ...).
fn nth_square(n: usize) -> u64 {
    let k = n as u64 + 1;
    k * k
}

/// The `n`-th triangular number (0-indexed: 1, 3, 6, ...).
fn nth_triangular(n: usize) -> u64 {
    let k = n as u64 + 1;
    k * (k + 1) / 2
}

/// The `n`-th power of two starting at 2 (0-indexed: 2, 4, 8, ...).
fn nth_power_of_two(n: usize) -> u64 {
    1u64 << (n + 1)
}

/// Build a first-contact puzzle from a `seed`, showing `shown` terms.
#[must_use]
pub fn alien_message(seed: u64, shown: usize) -> AlienMessage {
    let mut rng = SplitMix64::new(seed ^ ALIEN_MIX);
    let shown = shown.max(2);
    let (name, explanation, generator) = SEQUENCES[(rng.below(SEQUENCES.len() as u64)) as usize];
    let terms = (0..shown).map(generator).collect();
    AlienMessage {
        name,
        terms,
        answer: generator(shown),
        explanation,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        alien_message, is_prime, nth_fibonacci, nth_power_of_two, nth_prime, nth_square,
        nth_triangular,
    };

    #[test]
    fn sequences_start_correctly() {
        assert_eq!(
            (0..5).map(nth_prime).collect::<Vec<_>>(),
            vec![2, 3, 5, 7, 11]
        );
        assert_eq!(
            (0..5).map(nth_fibonacci).collect::<Vec<_>>(),
            vec![1, 1, 2, 3, 5]
        );
        assert_eq!(
            (0..4).map(nth_square).collect::<Vec<_>>(),
            vec![1, 4, 9, 16]
        );
        assert_eq!(
            (0..4).map(nth_triangular).collect::<Vec<_>>(),
            vec![1, 3, 6, 10]
        );
        assert_eq!(
            (0..4).map(nth_power_of_two).collect::<Vec<_>>(),
            vec![2, 4, 8, 16]
        );
    }

    #[test]
    fn primality_is_correct() {
        assert!(is_prime(2) && is_prime(97));
        assert!(!is_prime(1) && !is_prime(91)); // 91 = 7 * 13
    }

    #[test]
    fn message_answer_continues_the_sequence() {
        for seed in 0..20 {
            let m = alien_message(seed, 5);
            // The answer must not equal the last shown term (sequences are strictly growing here).
            assert!(m.answer > *m.terms.last().unwrap());
            assert_eq!(m.terms.len(), 5);
        }
    }

    #[test]
    fn message_is_deterministic() {
        assert_eq!(alien_message(11, 5), alien_message(11, 5));
    }
}
