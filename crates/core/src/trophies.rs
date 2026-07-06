//! The trophy case: deadpan achievements, earned by evidence.
//!
//! Every trophy is computed from the journey and the score table, no separate
//! bookkeeping, no way to have one you did not earn. Unearned trophies show as
//! silhouettes, because wanting to fill the case is half the engine of an RPG
//! (see `docs/PLAYFUL.md`). Names are deadpan; each one is also true.

use crate::journey::Journey;
use crate::registry::all_rooms;
use crate::scores::Scoreboard;

/// One trophy: a name, what it honors, and whether the evidence exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trophy {
    /// The trophy's name.
    pub name: &'static str,
    /// What earned it, in one line.
    pub what: &'static str,
    /// Whether this player's record shows it.
    pub earned: bool,
}

/// The best score in the table for keys of a given game prefix.
fn best_for(board: &Scoreboard, prefix: &str) -> i64 {
    board
        .entries
        .iter()
        .filter(|(key, _)| key.starts_with(prefix))
        .map(|(_, &score)| score)
        .max()
        .unwrap_or(0)
}

/// Whether the table has any entry for a game prefix.
fn played(board: &Scoreboard, prefix: &str) -> bool {
    board.entries.keys().any(|key| key.starts_with(prefix))
}

/// Compute the full case from the evidence.
#[must_use]
pub fn trophies(journey: &Journey, board: &Scoreboard) -> Vec<Trophy> {
    let rooms = all_rooms().len();
    let games = ["munch", "quiz", "seti", "aliens", "crack"];
    let all_games = games.iter().all(|g| played(board, g));
    vec![
        Trophy {
            name: "First Light",
            what: "light your first star",
            earned: !journey.visited.is_empty(),
        },
        Trophy {
            name: "Cartographer",
            what: "light every star in the catalog",
            earned: journey.visited.len() >= rooms,
        },
        Trophy {
            name: "Regular",
            what: "show up ten times",
            earned: journey.plays >= 10,
        },
        Trophy {
            name: "Scholar",
            what: "show up one hundred times",
            earned: journey.plays >= 100,
        },
        Trophy {
            name: "First Blood",
            what: "answer well once",
            earned: journey.wins >= 1,
        },
        Trophy {
            name: "Untouchable",
            what: "answer well twenty-five times",
            earned: journey.wins >= 25,
        },
        Trophy {
            name: "Behind the Curtain",
            what: "hear something that was not listed",
            earned: journey.secrets >= 1,
        },
        Trophy {
            name: "Keeper of Silence",
            what: "hear five of them",
            earned: journey.secrets >= 5,
        },
        Trophy {
            name: "Six Seven",
            what: "reach level 7 (you know)",
            earned: journey.level() >= 7,
        },
        Trophy {
            name: "The Long Road",
            what: "reach level 21, halfway to the answer",
            earned: journey.level() >= 21,
        },
        Trophy {
            name: "The Answer",
            what: "reach the cap",
            earned: journey.level() >= crate::journey::MAX_LEVEL,
        },
        Trophy {
            name: "Century",
            what: "post one hundred points on a single board",
            earned: best_for(board, "munch") >= 100,
        },
        Trophy {
            name: "Bomb Squad",
            what: "defuse with six or more attempts to spare",
            earned: best_for(board, "crack") >= 6,
        },
        Trophy {
            name: "Ear for the Artificial",
            what: "find four signals in one sky",
            earned: best_for(board, "seti") >= 4,
        },
        Trophy {
            name: "The Chain",
            what: "seven daily challenges in a row",
            earned: journey.streak >= 7,
        },
        Trophy {
            name: "Unbroken",
            what: "thirty in a row",
            earned: journey.streak >= 30,
        },
        Trophy {
            name: "Polymath",
            what: "post a score in every game there is",
            earned: all_games,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::trophies;
    use crate::journey::Journey;
    use crate::scores::Scoreboard;

    #[test]
    fn a_fresh_record_has_a_full_case_of_silhouettes() {
        let case = trophies(&Journey::default(), &Scoreboard::default());
        assert!(case.len() >= 15);
        assert!(case.iter().all(|t| !t.earned));
        assert!(
            case.iter()
                .all(|t| !t.name.is_empty() && !t.what.is_empty())
        );
    }

    #[test]
    fn evidence_earns_the_right_trophies() {
        let mut journey = Journey::default();
        journey.visit("lorenz");
        journey.secrets = 1;
        journey.plays = 10;
        let mut board = Scoreboard::default();
        board.record("munch seed:7 board:0", 120);
        board.record("crack seed:1 digits:4", 7);
        let case = trophies(&journey, &board);
        let earned: Vec<&str> = case.iter().filter(|t| t.earned).map(|t| t.name).collect();
        assert!(earned.contains(&"First Light"));
        assert!(earned.contains(&"Regular"));
        assert!(earned.contains(&"Behind the Curtain"));
        assert!(earned.contains(&"Century"));
        assert!(earned.contains(&"Bomb Squad"));
        assert!(!earned.contains(&"Cartographer"));
        assert!(!earned.contains(&"Polymath"));
    }

    #[test]
    fn the_answer_trophy_needs_the_cap() {
        let journey = Journey {
            plays: 861,
            ..Default::default()
        };
        let case = trophies(&journey, &Scoreboard::default());
        assert!(case.iter().any(|t| t.name == "The Answer" && t.earned));
    }
}
