//! The Munch arcade: eat the right numbers while hunted.
//!
//! You are the Muncher, a bright ring with a bite taken out. The board is the
//! classic Munch board (numbers and a rule); the pressure is the Vexations,
//! the Order's lesser spirits, wrong answers given legs. Turn discipline:
//! you act, then every Vexation steps, all seeded and deterministic, so a
//! whole run replays from its action list on any face. See `docs/ARCADE.md`.

use crate::munchers::{Board, COLS, ROWS, build_board};
use crate::rng::SplitMix64;

/// Decorrelates arcade seeds from other seeded systems.
const ARCADE_MIX: u64 = 0xA5CA_DE00_0000_0007;
/// Lives at the start of a run.
pub const LIVES: u32 = 3;
const CELLS: usize = ROWS * COLS;

/// A Vexation's mind: one behavior each, one line of math each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mind {
    /// A random legal step: the drunkard.
    Drifter,
    /// Steps to reduce Manhattan distance: greedy pursuit.
    Tracker,
    /// Never chases; rewrites the number it stands on. Anti-camping.
    Editor,
}

/// One of the Order's lesser spirits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vexation {
    /// Where it stands (cell index, row-major).
    pub cell: usize,
    /// How it thinks.
    pub mind: Mind,
}

/// What one player action was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Step up (clamped at walls).
    Up,
    /// Step down.
    Down,
    /// Step left.
    Left,
    /// Step right.
    Right,
    /// Eat the number under the Muncher.
    Eat,
}

/// What a turn came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Turn {
    /// Play continues.
    Going,
    /// A Vexation caught the Muncher: a life lost, positions scattered.
    Caught,
    /// Every fit is eaten: the level is clear.
    Cleared,
    /// Lives are gone: the run ends.
    Over,
}

/// A run in progress.
#[derive(Debug, Clone)]
pub struct Arcade {
    /// The board (numbers and the rule).
    pub board: Board,
    /// The Muncher's cell.
    pub muncher: usize,
    /// The spirits.
    pub vexations: Vec<Vexation>,
    /// Which cells have been eaten.
    pub eaten: Vec<bool>,
    /// Lives left.
    pub lives: u32,
    /// The level (1-based). More spirits, faster world, deeper rules.
    pub level: u64,
    /// The run score so far.
    pub score: i64,
    /// The seed (fixed for the run; turns advance an internal counter).
    seed: u64,
    /// Turn counter, so every step's randomness is fresh but replayable.
    turns: u64,
}

/// Manhattan distance between two cells.
fn distance(a: usize, b: usize) -> usize {
    let (ar, ac) = (a / COLS, a % COLS);
    let (br, bc) = (b / COLS, b % COLS);
    ar.abs_diff(br) + ac.abs_diff(bc)
}

/// The legal one-step neighbors of a cell.
fn neighbors(cell: usize) -> Vec<usize> {
    let (row, col) = (cell / COLS, cell % COLS);
    let mut out = Vec::with_capacity(4);
    if row > 0 {
        out.push(cell - COLS);
    }
    if row + 1 < ROWS {
        out.push(cell + COLS);
    }
    if col > 0 {
        out.push(cell - 1);
    }
    if col + 1 < COLS {
        out.push(cell + 1);
    }
    out
}

impl Arcade {
    /// Start a run: level one, three lives, two spirits far from the corner.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let mut run = Self {
            board: build_board(seed ^ ARCADE_MIX, 0),
            muncher: 0,
            vexations: Vec::new(),
            eaten: vec![false; ROWS * COLS],
            lives: LIVES,
            level: 1,
            score: 0,
            seed,
            turns: 0,
        };
        run.spawn_vexations();
        run
    }

    /// The spirits for the current level: one more each level, minds cycling
    /// drifter, tracker, editor, seeded placement away from the Muncher.
    fn spawn_vexations(&mut self) {
        self.repair_public_state();
        let mut rng = SplitMix64::new(self.seed ^ ARCADE_MIX ^ self.level.wrapping_mul(0xD1CE));
        let count = (self.level as usize).saturating_add(1).min(5);
        self.vexations.clear();
        let minds = [Mind::Drifter, Mind::Tracker, Mind::Editor];
        while self.vexations.len() < count {
            let cell = rng.below(CELLS as u64) as usize;
            if distance(cell, self.muncher) >= 4 && !self.vexations.iter().any(|v| v.cell == cell) {
                let mind = minds[self.vexations.len() % minds.len()];
                self.vexations.push(Vexation { cell, mind });
            }
        }
    }

    /// Whether every number that fits the rule has been eaten.
    #[must_use]
    pub fn cleared(&self) -> bool {
        if self.board.numbers.len() != CELLS || self.eaten.len() != CELLS {
            return false;
        }
        self.board
            .numbers
            .iter()
            .enumerate()
            .all(|(i, &n)| self.eaten[i] || !self.board.rule.fits(n))
    }

    /// One full turn: the player acts, then every spirit steps. (The
    /// turn-based faces use this; real time calls `act` and `tick` apart.)
    pub fn turn(&mut self, action: Action) -> Turn {
        let acted = self.act(action);
        if acted != Turn::Going {
            return acted;
        }
        self.tick()
    }

    /// The player's half alone: move or eat, and settle clears.
    pub fn act(&mut self, action: Action) -> Turn {
        self.repair_public_state();
        if self.lives == 0 {
            return Turn::Over;
        }
        // The player's half.
        match action {
            Action::Up => {
                if self.muncher >= COLS {
                    self.muncher -= COLS;
                }
            }
            Action::Down => {
                if self.muncher + COLS < CELLS {
                    self.muncher += COLS;
                }
            }
            Action::Left => {
                if !self.muncher.is_multiple_of(COLS) {
                    self.muncher -= 1;
                }
            }
            Action::Right => {
                if self.muncher % COLS + 1 < COLS {
                    self.muncher += 1;
                }
            }
            Action::Eat => {
                if !self.eaten[self.muncher] {
                    self.eaten[self.muncher] = true;
                    let n = self.board.numbers[self.muncher];
                    self.score = if self.board.rule.fits(n) {
                        self.score.saturating_add(10)
                    } else {
                        self.score.saturating_sub(5)
                    };
                }
            }
        }
        if self.cleared() {
            let bonus = i64::try_from(self.level)
                .unwrap_or(i64::MAX / 20)
                .saturating_mul(20);
            self.score = self.score.saturating_add(bonus);
            self.level = self.level.saturating_add(1);
            self.board = build_board(self.seed ^ ARCADE_MIX, self.level - 1);
            self.eaten = vec![false; CELLS];
            self.muncher = 0;
            self.spawn_vexations();
            return Turn::Cleared;
        }
        Turn::Going
    }

    /// The spirits' half alone: every Vexation steps; contact is judged.
    pub fn tick(&mut self) -> Turn {
        self.repair_public_state();
        if self.lives == 0 {
            return Turn::Over;
        }
        self.turns = self.turns.saturating_add(1);
        let mut rng = SplitMix64::new(self.seed ^ ARCADE_MIX ^ self.turns);
        let muncher = self.muncher;
        let mut caught = false;
        let occupied: Vec<usize> = self.vexations.iter().map(|v| v.cell).collect();
        for (i, vex) in self.vexations.iter_mut().enumerate() {
            match vex.mind {
                Mind::Drifter => {
                    let options = neighbors(vex.cell);
                    vex.cell = options[rng.below(options.len() as u64) as usize];
                }
                Mind::Tracker => {
                    let best = neighbors(vex.cell)
                        .into_iter()
                        .filter(|c| !occupied.iter().enumerate().any(|(j, &o)| j != i && o == *c))
                        .min_by_key(|&c| (distance(c, muncher), c));
                    if let Some(cell) = best {
                        vex.cell = cell;
                    }
                }
                Mind::Editor => {
                    // The world decays where it stands: a fresh seeded number.
                    self.board.numbers[vex.cell] = 1 + rng.below(99);
                    self.eaten[vex.cell] = false;
                    let options = neighbors(vex.cell);
                    vex.cell = options[rng.below(options.len() as u64) as usize];
                }
            }
            if vex.cell == muncher {
                caught = true;
            }
        }
        if caught {
            self.lives -= 1;
            if self.lives == 0 {
                return Turn::Over;
            }
            // Respawn at the far corner; the spirits scatter afresh.
            self.muncher = CELLS - 1;
            self.spawn_vexations();
            return Turn::Caught;
        }
        Turn::Going
    }

    fn repair_public_state(&mut self) {
        self.board.numbers.resize(CELLS, 1);
        self.eaten.resize(CELLS, false);
        self.muncher = self.muncher.min(CELLS - 1);
        self.vexations.retain(|v| v.cell < CELLS);
    }
}

/// Plain text board for CLI/MCP/viewer attestation.
///
/// The Muncher never hides an uneaten number: the selected cell prints the
/// digits with an `@` mark so a player can decide whether to eat. Empty
/// muncher cells stay `[@]`. Structured payloads still carry `muncher`.
#[must_use]
pub fn board_text(run: &Arcade) -> String {
    let mut out = String::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            let cell = row * COLS + col;
            if cell == run.muncher {
                if run.eaten[cell] {
                    out.push_str("[@]");
                } else {
                    out.push_str(&format!("[{:>2}@]", run.board.numbers[cell]));
                }
            } else if let Some(vex) = run.vexations.iter().find(|vex| vex.cell == cell) {
                let mind = match vex.mind {
                    Mind::Drifter => "d",
                    Mind::Tracker => "T",
                    Mind::Editor => "e",
                };
                out.push_str(&format!("[{mind}]"));
            } else if run.eaten[cell] {
                out.push_str("[ ]");
            } else {
                out.push_str(&format!("[{:>2}]", run.board.numbers[cell]));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Action, Arcade, LIVES, Mind, Turn, Vexation, board_text, distance, neighbors};
    use crate::munchers::{COLS, ROWS};

    #[test]
    fn board_text_keeps_the_number_under_the_muncher() {
        let mut run = Arcade::new(11);
        let n = run.board.numbers[run.muncher];
        let open = board_text(&run);
        assert!(
            open.contains(&format!("[{n:>2}@]")),
            "selected cell must show digits, got:\n{open}"
        );
        assert!(
            !open.contains("[@]"),
            "bare [@] must not hide an uneaten opening value:\n{open}"
        );
        run.eaten[run.muncher] = true;
        let empty = board_text(&run);
        assert!(
            empty.contains("[@]"),
            "empty muncher cell keeps the bare mark:\n{empty}"
        );
    }

    #[test]
    fn runs_start_fair_and_deterministic() {
        let a = Arcade::new(7);
        let b = Arcade::new(7);
        assert_eq!(a.vexations, b.vexations, "same seed, same spirits");
        assert_eq!(a.lives, LIVES);
        assert_eq!(a.vexations.len(), 2, "level one: two spirits");
        for v in &a.vexations {
            assert!(distance(v.cell, a.muncher) >= 4, "spirits spawn away");
        }
    }

    #[test]
    fn trackers_close_distance_and_drifters_stay_legal() {
        let mut run = Arcade::new(3);
        for _ in 0..8 {
            let before: Vec<(usize, Mind)> =
                run.vexations.iter().map(|v| (v.cell, v.mind)).collect();
            let muncher = run.muncher;
            let outcome = run.turn(Action::Eat);
            if outcome != Turn::Going {
                break;
            }
            for ((was, mind), now) in before.iter().zip(run.vexations.iter()) {
                assert!(
                    neighbors(*was).contains(&now.cell) || now.cell == *was,
                    "spirits step one legal cell"
                );
                if *mind == Mind::Tracker {
                    assert!(
                        distance(now.cell, muncher) <= distance(*was, muncher),
                        "the tracker never loses ground"
                    );
                }
            }
        }
    }

    #[test]
    fn the_editor_rewrites_the_world() {
        let mut run = Arcade::new(11);
        let editor_at = run
            .vexations
            .iter()
            .find(|v| v.mind == Mind::Editor)
            .map(|v| v.cell);
        // Level one has two spirits (drifter, tracker); force level two for
        // an editor by clearing... instead, spawn deterministically: level 2.
        if editor_at.is_none() {
            run.level = 2;
            run.spawn_vexations();
        }
        let editor = run
            .vexations
            .iter()
            .find(|v| v.mind == Mind::Editor)
            .expect("level two has an editor")
            .cell;
        let before = run.board.numbers[editor];
        let mut changed = false;
        for _ in 0..12 {
            let cell = run
                .vexations
                .iter()
                .find(|v| v.mind == Mind::Editor)
                .expect("still there")
                .cell;
            let value = run.board.numbers[cell];
            if run.turn(Action::Up) != Turn::Going {
                break;
            }
            if run.board.numbers[cell] != value {
                changed = true;
                break;
            }
        }
        assert!(
            changed || run.board.numbers[editor] != before,
            "the world decays"
        );
    }

    #[test]
    fn walls_hold_and_eating_scores_both_ways() {
        let mut run = Arcade::new(5);
        run.vexations.clear(); // a quiet room for the geometry test
        run.turn(Action::Up);
        run.turn(Action::Left);
        assert_eq!(run.muncher, 0, "walls hold");
        let fits = run.board.rule.fits(run.board.numbers[0]);
        let before = run.score;
        run.turn(Action::Eat);
        assert_eq!(run.score - before, if fits { 10 } else { -5 });
        let mid = run.score;
        run.turn(Action::Eat);
        assert_eq!(run.score, mid, "a cell only feeds once");
    }

    #[test]
    fn public_state_is_repaired_before_indexing() {
        let mut run = Arcade::new(5);
        run.muncher = usize::MAX;
        run.vexations.push(Vexation {
            cell: usize::MAX,
            mind: Mind::Editor,
        });
        run.eaten.clear();
        run.board.numbers.clear();
        let _ = run.turn(Action::Eat);
        assert!(run.muncher < ROWS * COLS);
        assert_eq!(run.eaten.len(), ROWS * COLS);
        assert_eq!(run.board.numbers.len(), ROWS * COLS);
        assert!(run.vexations.iter().all(|v| v.cell < ROWS * COLS));
    }

    #[test]
    fn clearing_advances_the_level_and_the_pressure() {
        let mut run = Arcade::new(9);
        run.vexations.clear();
        // Eat every fit by teleport-walking the grid deterministically.
        for cell in 0..ROWS * COLS {
            run.muncher = cell;
            if run.board.rule.fits(run.board.numbers[cell]) && !run.eaten[cell] {
                let outcome = run.turn(Action::Eat);
                if outcome == Turn::Cleared {
                    assert_eq!(run.level, 2);
                    assert_eq!(run.vexations.len(), 3, "one more spirit");
                    assert!(run.score >= 20, "the clear bonus landed");
                    return;
                }
            }
        }
        panic!("the board never cleared");
    }

    #[test]
    fn capture_costs_a_life_scatters_and_ends_at_zero() {
        let mut run = Arcade::new(13);
        // Put a tracker adjacent and walk into losing all three lives.
        let mut endings = 0;
        for _ in 0..400 {
            match run.turn(Action::Eat) {
                Turn::Caught => {
                    endings += 1;
                    assert_eq!(run.muncher, ROWS * COLS - 1, "respawn at the far corner");
                }
                Turn::Over => {
                    assert_eq!(run.lives, 0);
                    assert_eq!(endings, LIVES as usize - 1, "two catches then the end");
                    return;
                }
                _ => {}
            }
        }
        panic!("the spirits never finished the job");
    }
}
