//! Crack the Code: a bomb-defusal deduction game, seeded and math-flavored.
//!
//! A hidden numeric code, a starting math clue, and a countdown of attempts. Each
//! guess reports how many digits are locked (right digit, right place) and how
//! many are loose (right digit, wrong place), like Bulls and Cows. Run out of
//! attempts and the bomb goes off. Deterministic from a seed, so a code can be
//! shared and raced. See `docs/PLAYFUL.md`.

use crate::rng::SplitMix64;

/// Decorrelates the code seed from other seeded systems.
const CODE_MIX: u64 = 0xB055_C0DE_1234_5678;

/// The result of grading a guess against the secret code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Feedback {
    /// Digits that are the right value in the right place.
    pub locked: usize,
    /// Digits that are the right value but in the wrong place.
    pub loose: usize,
}

/// The hidden code for a seed: `digits` values, each 0-9.
#[must_use]
pub fn secret_code(seed: u64, digits: usize) -> Vec<u8> {
    let mut rng = SplitMix64::new(seed ^ CODE_MIX);
    (0..digits).map(|_| (rng.below(10)) as u8).collect()
}

/// Grade a `guess` against the `secret`, Bulls and Cows style.
#[must_use]
pub fn grade(secret: &[u8], guess: &[u8]) -> Feedback {
    let mut locked = 0;
    let mut secret_left = [0u32; 10];
    let mut guess_left = [0u32; 10];
    for (&s, &g) in secret.iter().zip(guess.iter()) {
        if s == g {
            locked += 1;
        } else {
            secret_left[(s % 10) as usize] += 1;
            guess_left[(g % 10) as usize] += 1;
        }
    }
    let loose = (0..10)
        .map(|d| secret_left[d].min(guess_left[d]))
        .sum::<u32>() as usize;
    Feedback { locked, loose }
}

/// A true math clue to open with: the digit sum and the code's parity.
#[must_use]
pub fn hint(secret: &[u8]) -> String {
    let sum: u32 = secret.iter().map(|&d| u32::from(d)).sum();
    let last_even = secret.last().is_none_or(|&d| d % 2 == 0);
    format!(
        "The digits sum to {sum}, and the last digit is {}.",
        if last_even { "even" } else { "odd" }
    )
}

#[cfg(test)]
mod tests {
    use super::{grade, hint, secret_code};

    #[test]
    fn secret_is_deterministic_and_in_range() {
        let a = secret_code(99, 4);
        let b = secret_code(99, 4);
        assert_eq!(a, b);
        assert_eq!(a.len(), 4);
        assert!(a.iter().all(|&d| d < 10));
    }

    #[test]
    fn a_perfect_guess_locks_every_digit() {
        let secret = vec![1, 2, 3, 4];
        assert_eq!(grade(&secret, &secret).locked, 4);
        assert_eq!(grade(&secret, &secret).loose, 0);
    }

    #[test]
    fn a_reversal_is_all_loose() {
        let secret = [1, 2, 3, 4];
        let guess = [4, 3, 2, 1];
        let fb = grade(&secret, &guess);
        assert_eq!(fb.locked, 0);
        assert_eq!(fb.loose, 4);
    }

    #[test]
    fn a_partial_guess_splits_locked_and_loose() {
        let secret = [1, 2, 3, 4];
        let guess = [1, 2, 4, 3];
        let fb = grade(&secret, &guess);
        assert_eq!(fb.locked, 2);
        assert_eq!(fb.loose, 2);
    }

    #[test]
    fn duplicate_digits_are_not_double_counted() {
        let secret = [1, 1, 2, 3];
        let guess = [1, 4, 4, 1];
        let fb = grade(&secret, &guess);
        assert_eq!(fb.locked, 1); // first digit
        assert_eq!(fb.loose, 1); // one more 1 matches, the guess's second/third do not
    }

    #[test]
    fn hint_reports_the_digit_sum() {
        assert!(hint(&[1, 2, 3, 4]).contains("sum to 10"));
        assert!(hint(&[1, 2, 3, 4]).contains("even"));
        assert!(hint(&[1, 2, 3, 5]).contains("odd"));
    }
}
