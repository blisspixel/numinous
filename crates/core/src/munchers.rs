//! Munch: eat the numbers that fit the rule. Number Munchers, reborn.
//!
//! A seeded grid of numbers and a rule (primes, multiples, squares). Eat the
//! right ones for points, bite a wrong one and it costs you, clear the board
//! perfectly for a bonus. The same seed gives the same board to a human in a
//! terminal and an agent over MCP, so scores are directly comparable: the first
//! game here that humans and digital minds can play against each other on even
//! terms. See `docs/PLAYFUL.md`.

use crate::rng::SplitMix64;

/// Decorrelates the munch seed from other seeded systems.
const MUNCH_MIX: u64 = 0x0EA7_0EA7_0EA7_0EA7;
/// Board dimensions.
pub const COLS: usize = 6;
/// Board rows.
pub const ROWS: usize = 5;
/// Points for eating a number that fits the rule.
pub const HIT_POINTS: i64 = 10;
/// Points lost for biting a number that does not.
pub const MISS_PENALTY: i64 = 5;
/// Bonus for a perfect round: every fit eaten, nothing else touched.
pub const PERFECT_BONUS: i64 = 20;

/// The rule a round asks you to munch by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Numbers divisible only by themselves and one.
    Primes,
    /// Multiples of `n`.
    MultiplesOf(u64),
    /// Perfect squares.
    Squares,
    /// Numbers whose digits sum to `n`.
    DigitSum(u64),
    /// Numbers with at least two prime factors (not prime, not one).
    Composites,
    /// Fibonacci numbers.
    Fibonacci,
}

impl Rule {
    /// The rule, as the board announces it.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Rule::Primes => "Eat the PRIMES".to_string(),
            Rule::MultiplesOf(n) => format!("Eat the MULTIPLES OF {n}"),
            Rule::Squares => "Eat the PERFECT SQUARES".to_string(),
            Rule::DigitSum(n) => format!("Eat where the DIGITS SUM TO {n}"),
            Rule::Composites => "Eat the COMPOSITES (not prime, not one)".to_string(),
            Rule::Fibonacci => "Eat the FIBONACCI NUMBERS".to_string(),
        }
    }

    /// Does `value` fit the rule?
    #[must_use]
    pub fn fits(self, value: u64) -> bool {
        match self {
            Rule::Primes => is_prime(value),
            Rule::MultiplesOf(n) => value % n == 0,
            Rule::Squares => {
                let root = value.isqrt();
                root * root == value
            }
            Rule::DigitSum(n) => {
                let mut sum = 0;
                let mut v = value;
                while v > 0 {
                    sum += v % 10;
                    v /= 10;
                }
                sum == n
            }
            Rule::Composites => value > 1 && !is_prime(value),
            Rule::Fibonacci => {
                matches!(value, 1 | 2 | 3 | 5 | 8 | 13 | 21 | 34 | 55 | 89)
            }
        }
    }
}

/// One board: a grid of numbers and the rule to munch by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// The numbers, row-major, `ROWS` x `COLS`.
    pub numbers: Vec<u64>,
    /// The rule for this round.
    pub rule: Rule,
}

/// The graded outcome of a set of bites, dense enough to learn from: not just
/// the counts but the numbers themselves, so a player (or an agent mining its
/// own trajectory) can see exactly which judgments were wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Munched {
    /// Right bites: numbers eaten that fit the rule.
    pub hits: u32,
    /// Wrong bites: numbers eaten that do not fit.
    pub bad_bites: u32,
    /// Fits left on the board uneaten.
    pub left_behind: u32,
    /// The score: hits and bonus minus penalties, floored at zero.
    pub score: i64,
    /// The numbers wrongly eaten (the bad judgments, by value).
    pub wrongly_eaten: Vec<u64>,
    /// The fitting numbers left uneaten (the misses, by value).
    pub missed: Vec<u64>,
}

/// Trial-division primality, plenty for board-sized numbers.
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

/// Build the deterministic board for `seed` and `round`.
///
/// Boards always contain at least one number that fits, so there is always
/// something to eat.
#[must_use]
pub fn build_board(seed: u64, round: u64) -> Board {
    let mut rng = SplitMix64::new(seed ^ MUNCH_MIX ^ round.wrapping_mul(0x9E37_79B9));
    // Early rounds stay classic; from round two the deeper rules join the
    // deck, so the game grows judgment instead of bookkeeping.
    let deck = if round < 2 { 4 } else { 7 };
    let rule = match rng.below(deck) {
        0 => Rule::Primes,
        1 => Rule::MultiplesOf(2 + rng.below(4)), // 2..=5
        2 => Rule::Squares,
        3 => Rule::MultiplesOf(6 + rng.below(4)), // 6..=9
        4 => Rule::DigitSum(5 + rng.below(9)),    // 5..=13
        5 => Rule::Composites,
        _ => Rule::Fibonacci,
    };
    loop {
        let numbers: Vec<u64> = (0..ROWS * COLS).map(|_| 1 + rng.below(99)).collect();
        if numbers.iter().any(|&n| rule.fits(n)) {
            return Board { numbers, rule };
        }
    }
}

/// Grade a set of bites (indices into the board, duplicates ignored).
#[must_use]
pub fn grade(board: &Board, bites: &[usize]) -> Munched {
    let mut eaten = vec![false; board.numbers.len()];
    for &bite in bites {
        if bite < eaten.len() {
            eaten[bite] = true;
        }
    }
    let mut hits = 0u32;
    let mut wrongly_eaten = Vec::new();
    let mut missed = Vec::new();
    for (i, &value) in board.numbers.iter().enumerate() {
        match (board.rule.fits(value), eaten[i]) {
            (true, true) => hits += 1,
            (true, false) => missed.push(value),
            (false, true) => wrongly_eaten.push(value),
            (false, false) => {}
        }
    }
    let bad_bites = wrongly_eaten.len() as u32;
    let left_behind = missed.len() as u32;
    let mut score = i64::from(hits) * HIT_POINTS - i64::from(bad_bites) * MISS_PENALTY;
    if left_behind == 0 && bad_bites == 0 && hits > 0 {
        score += PERFECT_BONUS;
    }
    Munched {
        hits,
        bad_bites,
        left_behind,
        score: score.max(0),
        wrongly_eaten,
        missed,
    }
}

/// The board as a text grid with 1-based cell numbers, for any face to print.
#[must_use]
pub fn board_text(board: &Board) -> String {
    let mut out = String::new();
    for row in 0..ROWS {
        let cells: Vec<String> = (0..COLS)
            .map(|col| {
                let i = row * COLS + col;
                format!("[{:>2}]{:>3}", i + 1, board.numbers[i])
            })
            .collect();
        out.push_str(&cells.join("  "));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_deeper_rules_judge_correctly() {
        use super::Rule;
        assert!(Rule::DigitSum(9).fits(45), "4 + 5 = 9");
        assert!(!Rule::DigitSum(9).fits(46));
        assert!(Rule::Composites.fits(91), "91 = 7 x 13, the classic trap");
        assert!(!Rule::Composites.fits(89), "89 is prime");
        assert!(!Rule::Composites.fits(1), "one is neither");
        assert!(Rule::Fibonacci.fits(89));
        assert!(!Rule::Fibonacci.fits(90));
        for round in 2..30 {
            let board = super::build_board(7, round);
            assert!(board.numbers.iter().any(|&n| board.rule.fits(n)));
        }
    }

    use super::{Board, Rule, build_board, grade, is_prime};

    #[test]
    fn rules_judge_correctly() {
        assert!(Rule::Primes.fits(97) && !Rule::Primes.fits(91));
        assert!(Rule::MultiplesOf(7).fits(49) && !Rule::MultiplesOf(7).fits(50));
        assert!(Rule::Squares.fits(81) && !Rule::Squares.fits(80));
        assert!(is_prime(2) && !is_prime(1));
    }

    #[test]
    fn boards_are_deterministic_and_always_edible() {
        for round in 0..20 {
            let a = build_board(7, round);
            let b = build_board(7, round);
            assert_eq!(a, b);
            assert!(a.numbers.iter().any(|&n| a.rule.fits(n)));
            assert_eq!(a.numbers.len(), super::ROWS * super::COLS);
        }
    }

    #[test]
    fn grading_scores_hits_penalties_and_perfection() {
        let board = Board {
            numbers: vec![2, 4, 5, 9], // primes: 2 and 5
            rule: Rule::Primes,
        };
        // Perfect: both primes, nothing else.
        let perfect = grade(&board, &[0, 2]);
        assert_eq!(perfect.hits, 2);
        assert_eq!(perfect.score, 2 * 10 + 20);
        // One right, one wrong bite, one prime left behind.
        let sloppy = grade(&board, &[0, 1]);
        assert_eq!(
            (sloppy.hits, sloppy.bad_bites, sloppy.left_behind),
            (1, 1, 1)
        );
        assert_eq!(sloppy.score, 10 - 5);
        // Dense feedback: the exact numbers, not just the counts.
        assert_eq!(sloppy.wrongly_eaten, vec![4]);
        assert_eq!(sloppy.missed, vec![5]);
        // All wrong bites floor at zero, never negative.
        let awful = grade(&board, &[1, 3]);
        assert_eq!(awful.score, 0);
        // Out-of-range and duplicate bites are ignored.
        let weird = grade(&board, &[0, 0, 99]);
        assert_eq!(weird.hits, 1);
    }

    #[test]
    fn the_board_prints_every_cell() {
        let board = build_board(1, 0);
        let text = super::board_text(&board);
        assert_eq!(text.lines().count(), super::ROWS);
        assert!(text.contains("[ 1]") && text.contains("[30]"));
    }
}
