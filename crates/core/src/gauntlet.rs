//! The Gauntlet's shared puzzle identity and scoring.
//!
//! App, CLI, MCP, and Watch Agent all present the same four-stage run. This
//! module owns the seeded stage construction, normalized grades, combo math,
//! reveal lines, and leaderboard key so faces cannot drift on game truth.

use crate::codebreaker::{Feedback, grade as grade_code, hint, secret_code};
use crate::munchers::{Board, Munched, build_board, clean_win, grade as grade_munch};
use crate::quiz::{QuizRound, build_round};
use crate::seti::{SetiScan, build_scan};

/// Number of stages in one complete Gauntlet run.
pub const GAUNTLET_STAGES: usize = 4;
/// Maximum number of valid bomb guesses in one run.
pub const GAUNTLET_MAX_WIRES: usize = 5;
/// Digits in the Gauntlet bomb code.
pub const GAUNTLET_BOMB_DIGITS: usize = 4;
/// Points awarded for a correct shape or sky answer.
pub const GAUNTLET_CHOICE_POINTS: i64 = 25;

const GAUNTLET_BOMB_MIX: u64 = 0x0000_6A17_0000_0B0B;
const GAUNTLET_SHAPE_ROUND: u64 = 1;
const GAUNTLET_SHAPE_WIDTH: usize = 44;
const GAUNTLET_SHAPE_HEIGHT: usize = 18;
const GAUNTLET_SKY_CHANNELS: usize = 4;
const GAUNTLET_BOMB_POINTS_PER_SPARE_WIRE: i64 = 10;

/// The deterministic four-stage puzzle for one seed.
#[derive(Debug, Clone)]
pub struct GauntletPuzzle {
    /// Shared seed naming this run.
    pub seed: u64,
    /// Stage one Munch board.
    pub munch: Board,
    /// Stage two mystery shape.
    pub shape: QuizRound,
    /// Stage three sky scan.
    pub sky: SetiScan,
    bomb_code: Vec<u8>,
}

/// Typed answers for a complete Gauntlet grade.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GauntletAnswers {
    /// Zero-based Munch cells selected by the player.
    pub bites: Vec<usize>,
    /// Shape choice letter, if supplied.
    pub shape: Option<char>,
    /// Sky channel letter, if supplied.
    pub sky: Option<char>,
    /// Bomb guesses in arrival order, with one decimal digit per element.
    pub wires: Vec<Vec<u8>>,
}

/// Score and clean flag for one Gauntlet stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GauntletStageGrade {
    /// Raw stage score before the combo multiplier.
    pub score: i64,
    /// Whether the stage was cleared without a mistake.
    pub clean: bool,
}

impl GauntletStageGrade {
    fn choice(correct: bool) -> Self {
        Self {
            score: if correct { GAUNTLET_CHOICE_POINTS } else { 0 },
            clean: correct,
        }
    }
}

/// Stage one grade, including the Munch learning details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GauntletMunchGrade {
    /// Detailed Munch outcome.
    pub outcome: Munched,
    /// Score and clean flag used by the combo.
    pub stage: GauntletStageGrade,
}

/// One valid bomb guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GauntletWireGrade {
    /// One-based valid attempt number.
    pub attempt: usize,
    /// Bulls-and-cows feedback for this guess.
    pub feedback: Feedback,
    /// Score and clean flag if this wire solved the bomb.
    pub stage: GauntletStageGrade,
}

/// Complete stage four grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GauntletBombGrade {
    /// Score and clean flag used by the combo.
    pub stage: GauntletStageGrade,
    /// Number of valid four-digit guesses examined.
    pub valid_attempts: usize,
    /// One-based attempt that solved the code, if any.
    pub solved_attempt: Option<usize>,
    /// Feedback from the last valid guess, if any.
    pub last_feedback: Option<Feedback>,
}

/// Typed result for all four Gauntlet stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GauntletGrade {
    /// Stage one Munch grade.
    pub munch: GauntletMunchGrade,
    /// Stage two shape grade.
    pub shape: GauntletStageGrade,
    /// Stage three sky grade.
    pub sky: GauntletStageGrade,
    /// Stage four bomb grade.
    pub bomb: GauntletBombGrade,
}

impl GauntletPuzzle {
    /// Build the exact four-stage run for `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            munch: build_board(seed, 0),
            shape: build_round(
                seed,
                GAUNTLET_SHAPE_ROUND,
                GAUNTLET_SHAPE_WIDTH,
                GAUNTLET_SHAPE_HEIGHT,
            ),
            sky: build_scan(seed, GAUNTLET_SKY_CHANNELS),
            bomb_code: secret_code(seed ^ GAUNTLET_BOMB_MIX, GAUNTLET_BOMB_DIGITS),
        }
    }

    /// Player-facing clue for stage four.
    #[must_use]
    pub fn bomb_hint(&self) -> String {
        hint(&self.bomb_code)
    }

    /// Exact stage four code, for grading and the post-run reveal.
    #[must_use]
    pub fn bomb_code(&self) -> &[u8] {
        &self.bomb_code
    }

    /// Stage four code rendered as decimal digits.
    #[must_use]
    pub fn bomb_code_text(&self) -> String {
        self.bomb_code
            .iter()
            .map(|&digit| char::from(b'0' + digit))
            .collect()
    }

    /// Grade stage one against this run's Munch board.
    #[must_use]
    pub fn grade_munch(&self, bites: &[usize]) -> GauntletMunchGrade {
        let outcome = grade_munch(&self.munch, bites);
        let clean = clean_win(&outcome);
        GauntletMunchGrade {
            stage: GauntletStageGrade {
                score: outcome.score,
                clean,
            },
            outcome,
        }
    }

    /// Grade stage two after normalizing the supplied letter to uppercase.
    #[must_use]
    pub fn grade_shape(&self, guess: Option<char>) -> GauntletStageGrade {
        gauntlet_choice_grade(
            guess.map(|letter| letter.to_ascii_uppercase()) == Some(self.shape.answer),
        )
    }

    /// Grade stage three after normalizing the supplied letter to uppercase.
    #[must_use]
    pub fn grade_sky(&self, guess: Option<char>) -> GauntletStageGrade {
        gauntlet_choice_grade(
            guess.map(|letter| letter.to_ascii_uppercase()) == Some(self.sky.answer),
        )
    }

    /// Grade one valid stage four guess.
    ///
    /// `attempt` is one-based. An unsupported attempt number, a non-four-digit
    /// guess, or a value outside 0 through 9 returns `None` and does not burn a
    /// wire.
    #[must_use]
    pub fn grade_wire(&self, attempt: usize, guess: &[u8]) -> Option<GauntletWireGrade> {
        gauntlet_wire_grade(&self.bomb_code, attempt, guess)
    }

    /// Grade stage four from an arrival-ordered sequence of guesses.
    ///
    /// Invalid guesses are ignored and do not consume one of the five wires.
    #[must_use]
    pub fn grade_bomb(&self, wires: &[Vec<u8>]) -> GauntletBombGrade {
        let mut valid_attempts = 0;
        let mut last_feedback = None;
        for guess in wires {
            if valid_attempts == GAUNTLET_MAX_WIRES {
                break;
            }
            let attempt = valid_attempts + 1;
            let Some(grade) = self.grade_wire(attempt, guess) else {
                continue;
            };
            valid_attempts = attempt;
            last_feedback = Some(grade.feedback);
            if grade.stage.clean {
                return GauntletBombGrade {
                    stage: grade.stage,
                    valid_attempts,
                    solved_attempt: Some(attempt),
                    last_feedback,
                };
            }
        }
        GauntletBombGrade {
            stage: GauntletStageGrade {
                score: 0,
                clean: false,
            },
            valid_attempts,
            solved_attempt: None,
            last_feedback,
        }
    }

    /// Grade all four stages with one typed request.
    #[must_use]
    pub fn grade(&self, answers: &GauntletAnswers) -> GauntletGrade {
        GauntletGrade {
            munch: self.grade_munch(&answers.bites),
            shape: self.grade_shape(answers.shape),
            sky: self.grade_sky(answers.sky),
            bomb: self.grade_bomb(&answers.wires),
        }
    }
}

/// Grade one valid stage four guess against an established code.
///
/// `attempt` is one-based. An unsupported attempt number, a non-four-digit
/// code or guess, or a value outside 0 through 9 returns `None` and does not
/// burn a wire.
#[must_use]
pub fn gauntlet_wire_grade(code: &[u8], attempt: usize, guess: &[u8]) -> Option<GauntletWireGrade> {
    if !(1..=GAUNTLET_MAX_WIRES).contains(&attempt)
        || code.len() != GAUNTLET_BOMB_DIGITS
        || guess.len() != GAUNTLET_BOMB_DIGITS
        || code.iter().any(|&digit| digit > 9)
        || guess.iter().any(|&digit| digit > 9)
    {
        return None;
    }
    let feedback = grade_code(code, guess);
    let clean = feedback.locked == GAUNTLET_BOMB_DIGITS;
    let spare_wires = GAUNTLET_MAX_WIRES - attempt;
    Some(GauntletWireGrade {
        attempt,
        feedback,
        stage: GauntletStageGrade {
            score: if clean {
                GAUNTLET_BOMB_POINTS_PER_SPARE_WIRE * spare_wires as i64
            } else {
                0
            },
            clean,
        },
    })
}

impl GauntletGrade {
    /// Raw stage scores in play order.
    #[must_use]
    pub fn stage_scores(&self) -> [i64; GAUNTLET_STAGES] {
        [
            self.munch.stage.score,
            self.shape.score,
            self.sky.score,
            self.bomb.stage.score,
        ]
    }

    /// Clean flags in play order.
    #[must_use]
    pub fn cleared(&self) -> [bool; GAUNTLET_STAGES] {
        [
            self.munch.stage.clean,
            self.shape.clean,
            self.sky.clean,
            self.bomb.stage.clean,
        ]
    }

    /// Number of clean stages.
    #[must_use]
    pub fn clean_count(&self) -> usize {
        self.cleared().into_iter().filter(|clean| *clean).count()
    }

    /// Combo-weighted run total.
    #[must_use]
    pub fn total(&self) -> i64 {
        gauntlet_total(&self.stage_scores(), &self.cleared())
    }

    /// Canonical semantic reveal lines used by MCP and Watch Agent validation.
    #[must_use]
    pub fn reveal_lines(&self, puzzle: &GauntletPuzzle) -> Vec<String> {
        vec![
            format!(
                "MUNCH: +{}{}",
                self.munch.stage.score,
                if self.munch.stage.clean {
                    "  CLEAN"
                } else {
                    ""
                }
            ),
            format!(
                "SHAPE: it was {} ({}). +{}{}",
                puzzle.shape.answer,
                puzzle.shape.answer_title,
                self.shape.score,
                if self.shape.clean { "  CLEAN" } else { "" }
            ),
            format!(
                "SKY: the signal was {}. +{}{}",
                puzzle.sky.answer,
                self.sky.score,
                if self.sky.clean { "  CLEAN" } else { "" }
            ),
            if self.bomb.stage.clean {
                format!("BOMB: DEFUSED. +{}  CLEAN", self.bomb.stage.score)
            } else {
                format!("BOMB: BOOM. It was {}. +0", puzzle.bomb_code_text())
            },
        ]
    }
}

/// Combo math for an ordered run prefix.
///
/// A clean stage increments the multiplier for the following stage. A miss
/// resets it to one. Extra elements in either slice are ignored, matching the
/// common paired-prefix representation used by the live App while a run is in
/// progress.
#[must_use]
pub fn gauntlet_total(stage_scores: &[i64], cleared: &[bool]) -> i64 {
    let mut total = 0;
    let mut combo = 1;
    for (score, &clean) in stage_scores.iter().zip(cleared) {
        total += score * combo;
        combo = if clean { combo + 1 } else { 1 };
    }
    total
}

/// Grade either letter-choice stage from its already-established correctness.
#[must_use]
pub fn gauntlet_choice_grade(correct: bool) -> GauntletStageGrade {
    GauntletStageGrade::choice(correct)
}

/// Shared leaderboard identity for one seeded Gauntlet run.
#[must_use]
pub fn gauntlet_score_key(seed: u64) -> String {
    format!("gauntlet seed:{seed}")
}

#[cfg(test)]
mod tests {
    use super::{GauntletAnswers, GauntletPuzzle, gauntlet_score_key, gauntlet_total};

    #[test]
    fn one_seed_builds_the_exact_same_four_stages() {
        let first = GauntletPuzzle::new(17);
        let second = GauntletPuzzle::new(17);
        assert_eq!(first.munch, second.munch);
        assert_eq!(first.shape.art, second.shape.art);
        assert_eq!(first.shape.answer, second.shape.answer);
        assert_eq!(first.sky.channels, second.sky.channels);
        assert_eq!(first.sky.answer, second.sky.answer);
        assert_eq!(first.bomb_code(), second.bomb_code());
    }

    #[test]
    fn combo_multiplies_clears_and_resets_after_a_miss() {
        assert_eq!(
            gauntlet_total(&[10, 25, 25, 40], &[true, true, true, true]),
            295
        );
        assert_eq!(
            gauntlet_total(&[10, 0, 25, 40], &[true, false, true, true]),
            115
        );
        assert_eq!(gauntlet_total(&[5, 0, 0, 0], &[false; 4]), 5);
        assert_eq!(gauntlet_total(&[], &[]), 0);
    }

    #[test]
    fn a_complete_clean_run_has_one_typed_grade() {
        let puzzle = GauntletPuzzle::new(29);
        let bites = puzzle
            .munch
            .numbers
            .iter()
            .enumerate()
            .filter_map(|(index, &value)| puzzle.munch.rule.fits(value).then_some(index))
            .collect();
        let answers = GauntletAnswers {
            bites,
            shape: Some(puzzle.shape.answer.to_ascii_lowercase()),
            sky: Some(puzzle.sky.answer.to_ascii_lowercase()),
            wires: vec![puzzle.bomb_code().to_vec()],
        };
        let grade = puzzle.grade(&answers);
        assert_eq!(grade.cleared(), [true; 4]);
        assert_eq!(grade.clean_count(), 4);
        assert_eq!(grade.bomb.solved_attempt, Some(1));
        assert_eq!(grade.bomb.stage.score, 40);
        assert_eq!(
            grade.total(),
            grade.munch.stage.score + 25 * 2 + 25 * 3 + 40 * 4
        );
        assert_eq!(grade.reveal_lines(&puzzle).len(), 4);
    }

    #[test]
    fn malformed_guesses_do_not_burn_a_wire() {
        let puzzle = GauntletPuzzle::new(41);
        let grade = puzzle.grade_bomb(&[
            vec![1, 2, 3],
            vec![10, 2, 3, 4],
            puzzle.bomb_code().to_vec(),
        ]);
        assert_eq!(grade.valid_attempts, 1);
        assert_eq!(grade.solved_attempt, Some(1));
        assert_eq!(grade.stage.score, 40);
    }

    #[test]
    fn leaderboard_key_is_stable() {
        assert_eq!(gauntlet_score_key(7), "gauntlet seed:7");
    }
}
