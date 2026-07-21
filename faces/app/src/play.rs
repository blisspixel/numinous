use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

/// The in-window quiz state: what is asked, and how the last answer landed.
pub struct QuizPlay {
    pub round: numinous_core::QuizRound,
    /// After an answer: (was it right, frames left on the flash).
    pub flash: Option<(bool, u64)>,
}

/// The in-window Munch: a board, a cursor, your bites, and the verdict.
pub struct MunchPlay {
    pub board: numinous_core::Board,
    pub seed: u64,
    pub round: u64,
    /// Cursor cell, 0-based (5 rows of 6).
    pub cursor: usize,
    /// Cells bitten so far, 0-based.
    pub bites: BTreeSet<usize>,
    /// After Enter: the graded outcome, shown until a key.
    pub graded: Option<numinous_core::Munched>,
    /// Bite juice: cell index and remaining presentation frames.
    pub bite_flash: Option<(usize, u64)>,
}

/// Frames a bite flash holds so a toggle is felt before the board settles.
pub const MUNCH_BITE_FLASH_FRAMES: u64 = 12;

impl MunchPlay {
    /// Count down bite juice; returns true while a flash still shows.
    pub fn tick_bite_flash(&mut self) -> bool {
        if let Some((_, frames)) = &mut self.bite_flash {
            *frames = frames.saturating_sub(1);
            if *frames == 0 {
                self.bite_flash = None;
            }
        }
        self.bite_flash.is_some()
    }

    /// Mark a cell as just toggled for a short bright flash.
    pub fn flash_bite(&mut self, cell: usize) {
        self.bite_flash = Some((cell, MUNCH_BITE_FLASH_FRAMES));
    }
}

#[cfg(test)]
mod juice_tests {
    use super::*;

    #[test]
    fn bite_flash_counts_down_and_clears() {
        let mut play = MunchPlay {
            board: numinous_core::build_board(1, 0),
            seed: 1,
            round: 0,
            cursor: 0,
            bites: BTreeSet::new(),
            graded: None,
            bite_flash: None,
        };
        play.flash_bite(4);
        assert_eq!(play.bite_flash, Some((4, MUNCH_BITE_FLASH_FRAMES)));
        for _ in 0..MUNCH_BITE_FLASH_FRAMES {
            let _ = play.tick_bite_flash();
        }
        assert!(play.bite_flash.is_none());
    }
}

const MUNCH_RULE_LOOKAHEAD: u64 = 16;

pub fn same_rule_family(
    a: numinous_core::munchers::Rule,
    b: numinous_core::munchers::Rule,
) -> bool {
    use numinous_core::munchers::Rule;
    matches!(
        (a, b),
        (Rule::Primes, Rule::Primes)
            | (Rule::MultiplesOf(_), Rule::MultiplesOf(_))
            | (Rule::Squares, Rule::Squares)
            | (Rule::DigitSum(_), Rule::DigitSum(_))
            | (Rule::Composites, Rule::Composites)
            | (Rule::Fibonacci, Rule::Fibonacci)
    )
}

pub fn deal_munch(
    seed: u64,
    start_round: u64,
    previous: Option<numinous_core::munchers::Rule>,
) -> (u64, numinous_core::Board) {
    let start_round = start_round.max(numinous_core::FULL_DECK_ROUND);
    for offset in 0..MUNCH_RULE_LOOKAHEAD {
        let round = start_round.saturating_add(offset);
        let board = numinous_core::build_board(seed, round);
        if previous.is_none_or(|rule| !same_rule_family(rule, board.rule)) {
            return (round, board);
        }
    }
    let board = numinous_core::build_board(seed, start_round);
    (start_round, board)
}

/// The in-window Gauntlet: four stages riding the other games' state.
pub struct GauntletPlay {
    pub seed: u64,
    /// 0 munch, 1 shape, 2 sky, 3 bomb, 4 done.
    pub stage: usize,
    pub munch: MunchPlay,
    pub quiz: QuizPlay,
    pub scan: numinous_core::SetiScan,
    pub secret: Vec<u8>,
    /// The bomb keypad: what is typed, and the feedback so far.
    pub wire: String,
    pub wire_lines: Vec<String>,
    /// Stage scores and clean flags, in order.
    pub scores: Vec<i64>,
    pub cleared: Vec<bool>,
    /// The running narration line.
    pub message: String,
}

/// The in-window arcade: the run, its beat, and the last event's flash.
pub struct ArcadePlay {
    pub run: numinous_core::munch_arcade::Arcade,
    pub seed: u64,
    /// Flash frames left and what happened (true = caught, false = clear).
    pub flash: Option<(bool, u64)>,
    /// The run has ended; any key leaves.
    pub over: bool,
}

/// The in-window Nim: the heaps, your aim, and the Order's last word.
pub struct NimPlay {
    pub heaps: Vec<u32>,
    pub seed: u64,
    /// Which heap you are aiming at.
    pub selected: usize,
    /// How many stones you mean to take.
    pub take: u32,
    /// The Order's last move, narrated.
    pub message: String,
    /// The end: Some(true) is your win (the secret shows), Some(false) is not.
    pub over: Option<bool>,
}

/// Today's seed: everyone who plays today plays the same boards.
pub fn daily_seed() -> u64 {
    daily_seed_from(SystemTime::now())
}

fn daily_seed_from(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(1)
}

/// Deal a fresh app quiz round and update the no-repeat history.
pub fn deal_quiz<I>(seed: u64, plays: u32, room_ids: I, recent: &mut Vec<&'static str>) -> QuizPlay
where
    I: IntoIterator<Item = &'static str>,
{
    let (base, choices): (Vec<&'static str>, usize) = if plays < 6 {
        (numinous_core::ICONIC.to_vec(), 3)
    } else {
        (room_ids.into_iter().collect(), 4)
    };
    let fresh: Vec<&'static str> = base
        .iter()
        .copied()
        .filter(|id| !recent.contains(id))
        .collect();
    let pool = if fresh.len() > choices { fresh } else { base };
    let round = numinous_core::build_round_pool(seed, u64::from(plays), 10, 10, choices, &pool);
    if let Some(choice) = round.choices.iter().find(|c| c.letter == round.answer) {
        recent.push(choice.id);
        let keep = if plays < 6 { 4 } else { 10 };
        while recent.len() > keep {
            recent.remove(0);
        }
    }
    QuizPlay { round, flash: None }
}

/// Accept one quiz letter. Returns whether the accepted answer was correct.
pub fn answer_quiz(quiz: &mut QuizPlay, letter: char) -> Option<bool> {
    if quiz.flash.is_some() || !quiz.round.choices.iter().any(|c| c.letter == letter) {
        return None;
    }
    let correct = letter == quiz.round.answer;
    quiz.flash = Some((correct, 300));
    Some(correct)
}

/// Combo math: cleared stages multiply what follows.
pub fn gauntlet_total(scores: &[i64], cleared: &[bool]) -> i64 {
    let mut total = 0;
    let mut combo = 1;
    for (score, &clear) in scores.iter().zip(cleared) {
        total += score * combo;
        combo = if clear { combo + 1 } else { 1 };
    }
    total
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn daily_seed_counts_whole_utc_days() {
        let moment = UNIX_EPOCH + Duration::from_secs((42 * 86_400) + 123);
        assert_eq!(super::daily_seed_from(moment), 42);
    }

    #[test]
    fn deal_quiz_starts_with_the_iconic_three_choice_hand() {
        let mut recent = Vec::new();
        let quiz = super::deal_quiz(7, 0, ["times-tables", "mandelbrot"], &mut recent);

        assert_eq!(quiz.round.choices.len(), 3);
        assert!(quiz.flash.is_none());
        for choice in &quiz.round.choices {
            assert!(numinous_core::ICONIC.contains(&choice.id));
        }
        assert_eq!(recent.len(), 1, "the answer enters no-repeat history");
        assert!(
            quiz.round
                .choices
                .iter()
                .any(|choice| choice.id == recent[0] && choice.letter == quiz.round.answer)
        );
    }

    #[test]
    fn deal_quiz_uses_fresh_iconic_choices_when_possible() {
        let mut recent = numinous_core::ICONIC[..4].to_vec();
        let excluded = recent.clone();
        let quiz = super::deal_quiz(9, 2, std::iter::empty(), &mut recent);

        assert_eq!(quiz.round.choices.len(), 3);
        assert!(
            quiz.round
                .choices
                .iter()
                .all(|choice| !excluded.contains(&choice.id)),
            "a large enough fresh opening pool should sit out recent answers"
        );
        assert!(recent.len() <= 4, "opening quiz history stays capped");
    }

    #[test]
    fn deal_quiz_switches_to_catalog_after_opening_rounds() {
        let catalog = [
            "times-tables",
            "golden-angle",
            "mandelbrot",
            "collatz",
            "lissajous",
            "harmonograph",
        ];
        let mut opening_recent = Vec::new();
        let opening = super::deal_quiz(13, 5, catalog, &mut opening_recent);

        assert_eq!(opening.round.choices.len(), 3);
        assert!(
            opening
                .round
                .choices
                .iter()
                .all(|choice| numinous_core::ICONIC.contains(&choice.id)),
            "the last opening round should still use the iconic hand"
        );
        assert!(opening_recent.len() <= 4);

        let mut catalog_recent = vec!["times-tables"];
        let excluded = catalog_recent.clone();
        let catalog_round = super::deal_quiz(13, 6, catalog, &mut catalog_recent);

        assert_eq!(catalog_round.round.choices.len(), 4);
        assert!(
            catalog_round
                .round
                .choices
                .iter()
                .all(|choice| catalog.contains(&choice.id)),
            "the first post-opening round should use the app room ids"
        );
        assert!(
            catalog_round
                .round
                .choices
                .iter()
                .all(|choice| !excluded.contains(&choice.id)),
            "a large enough catalog pool should sit out recent answers"
        );
        assert!(catalog_recent.len() <= 10);
    }

    #[test]
    fn answer_quiz_accepts_only_a_live_choice_once() {
        let mut recent = Vec::new();
        let mut quiz = super::deal_quiz(11, 7, numinous_core::ICONIC, &mut recent);
        let wrong = quiz
            .round
            .choices
            .iter()
            .find(|choice| choice.letter != quiz.round.answer)
            .map(|choice| choice.letter)
            .expect("round has a distractor");

        assert_eq!(super::answer_quiz(&mut quiz, '!'), None);
        assert!(quiz.flash.is_none());
        assert_eq!(super::answer_quiz(&mut quiz, wrong), Some(false));
        assert_eq!(quiz.flash.map(|flash| flash.0), Some(false));
        let answer = quiz.round.answer;
        assert_eq!(super::answer_quiz(&mut quiz, answer), None);
    }

    #[test]
    fn munch_sessions_start_with_the_full_deck_and_change_rule_family() {
        let seed = 20_647;
        let (first_round, first) = super::deal_munch(seed, 0, None);
        let (second_round, second) = super::deal_munch(seed, first_round + 1, Some(first.rule));
        let replay = super::deal_munch(seed, first_round + 1, Some(first.rule));

        assert!(first_round >= numinous_core::FULL_DECK_ROUND);
        assert!(second_round > first_round);
        assert!(!super::same_rule_family(first.rule, second.rule));
        assert_eq!((second_round, second), replay);
    }
}
