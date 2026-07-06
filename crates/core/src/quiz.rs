//! A "guess the shape" quiz, shared by every face.
//!
//! Given a seed and a round number, [`build_round`] renders a mystery room and
//! offers lettered choices, deterministically (so the CLI, the app, an agent over
//! MCP, and the tests all agree). You see a shape; you name the math behind it.
//! See `docs/PLAYFUL.md`.

use crate::canvas::Canvas;
use crate::registry::all_rooms;
use crate::rng::SplitMix64;

/// The letters used for choices (A, B, C, ...).
const LETTERS: [char; 6] = ['A', 'B', 'C', 'D', 'E', 'F'];
/// A constant to decorrelate successive rounds from the same seed.
const ROUND_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

/// One lettered option in a quiz round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuizChoice {
    /// The letter the player types to pick this option.
    pub letter: char,
    /// The room id this option refers to.
    pub id: &'static str,
    /// The room's display title.
    pub title: &'static str,
}

/// A single quiz round: a mystery shape and the choices for what made it.
#[derive(Debug, Clone)]
pub struct QuizRound {
    /// The mystery shape, rendered as ASCII.
    pub art: String,
    /// The lettered choices (the answer plus distractors), shuffled.
    pub choices: Vec<QuizChoice>,
    /// The letter of the correct choice.
    pub answer: char,
    /// The correct room's title.
    pub answer_title: &'static str,
    /// The correct room's reveal, to show after the guess.
    pub answer_reveal: &'static str,
}

/// Build a deterministic quiz round from a `seed` and `round` index.
///
/// The same inputs always produce the same mystery and choices, so a round can
/// be shared, replayed, and tested.
#[must_use]
pub fn build_round(seed: u64, round: u64, width: usize, height: usize) -> QuizRound {
    build_round_sized(seed, round, width, height, 4)
}

/// [`build_round`] with a chosen number of choices (hard mode uses six).
#[must_use]
pub fn build_round_sized(
    seed: u64,
    round: u64,
    width: usize,
    height: usize,
    choices: usize,
) -> QuizRound {
    let rooms = all_rooms();
    let n = rooms.len();
    let mut rng = SplitMix64::new(seed ^ round.wrapping_mul(ROUND_MIX));

    let answer_idx = rng.below(n as u64) as usize;
    let want = LETTERS.len().min(choices.max(2)).min(n);
    let mut picks = vec![answer_idx];
    while picks.len() < want {
        let candidate = rng.below(n as u64) as usize;
        if !picks.contains(&candidate) {
            picks.push(candidate);
        }
    }
    // Fisher-Yates so the answer is not always first.
    for i in (1..picks.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        picks.swap(i, j);
    }

    let t = rng.next_f64();
    let mut canvas = Canvas::new(width, height);
    rooms[answer_idx].render(&mut canvas, t);
    if canvas.ink_count() < 5 {
        // A mystery with nothing to see is no mystery: fall back to the
        // room's own postcard phase, which is guaranteed to have ink.
        canvas = Canvas::new(width, height);
        rooms[answer_idx].render(&mut canvas, rooms[answer_idx].postcard_t());
    }
    let art = canvas.to_text();

    let mut choices = Vec::with_capacity(picks.len());
    let mut answer = LETTERS[0];
    for (i, &idx) in picks.iter().enumerate() {
        let meta = rooms[idx].meta();
        let letter = LETTERS[i];
        if idx == answer_idx {
            answer = letter;
        }
        choices.push(QuizChoice {
            letter,
            id: meta.id,
            title: meta.title,
        });
    }

    QuizRound {
        art,
        choices,
        answer,
        answer_title: rooms[answer_idx].meta().title,
        answer_reveal: rooms[answer_idx].reveal(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_round;
    use crate::registry::all_rooms;

    #[test]
    fn round_is_deterministic() {
        let a = build_round(42, 0, 30, 15);
        let b = build_round(42, 0, 30, 15);
        assert_eq!(a.art, b.art);
        assert_eq!(a.answer, b.answer);
    }

    #[test]
    fn the_answer_is_always_among_the_choices() {
        for round in 0..30 {
            let r = build_round(7, round, 24, 12);
            assert!(r.choices.iter().any(|c| c.letter == r.answer));
            assert_eq!(r.choices.len(), 4.min(all_rooms().len()));
        }
    }

    #[test]
    fn the_answer_letter_names_the_answer_title() {
        let r = build_round(3, 5, 24, 12);
        let chosen = r.choices.iter().find(|c| c.letter == r.answer).unwrap();
        assert_eq!(chosen.title, r.answer_title);
    }

    #[test]
    fn the_mystery_has_a_shape() {
        assert!(!build_round(1, 1, 20, 10).art.trim().is_empty());
    }

    #[test]
    fn hard_mode_offers_six_choices() {
        let round = super::build_round_sized(5, 0, 20, 10, 6);
        assert_eq!(round.choices.len(), 6);
        assert!(round.choices.iter().any(|c| c.letter == round.answer));
    }
}
