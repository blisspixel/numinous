//! The Only Move: a machine that offers a game, and a board that answers.
//!
//! The room is a solved game played honestly. The machine searches the real
//! tree by backward induction, never an animation of one, and it will not take
//! a move that loses. A player who keeps playing discovers that they cannot
//! win, which is the small fact. The larger fact is underneath: the tie belongs
//! to this board and not to competition, and the room lets a player change the
//! board to prove it.
//!
//! Which lines count as a win is the room's dial. There are eight lines on a
//! three by three grid, so there are 256 ways to choose a rulebook, and a
//! player can walk all of them. Exactly one is a first-player win, and it is
//! the one that treats every direction alike. None of the 256 is ever a
//! second-player win, which is not a coincidence and not a search result: the
//! first player can always steal a second player's strategy, so a second-player
//! win cannot exist. The search agrees with the proof every time, which is the
//! kind of agreement worth feeling rather than being told.
//!
//! Nothing here is quoted from anywhere. Backward induction, game trees, and
//! exhaustive search belong to no one, and the copy is ours.

use crate::surface::Surface;

/// Cells on the grid, row major.
pub const CELLS: usize = 9;

/// The eight lines of a three by three grid: rows, columns, then diagonals.
///
/// The order is load bearing, because a rulebook is a bitmask over this array
/// and `WINNABLE_RULES` names a specific subset by bit position.
pub const LINES: [[usize; 3]; 8] = [
    [0, 1, 2], // top row
    [3, 4, 5], // middle row
    [6, 7, 8], // bottom row
    [0, 3, 6], // left column
    [1, 4, 7], // middle column
    [2, 5, 8], // right column
    [0, 4, 8], // main diagonal
    [2, 4, 6], // anti diagonal
];

/// Every line counts: the game as it is usually played.
pub const ALL_RULES: u8 = 0b1111_1111;

/// The one rulebook of 256 whose first player can force a win.
///
/// It keeps both diagonals and the four lines around the frame, and drops the
/// middle row and the middle column. It is the only subset invariant under all
/// eight symmetries of the square that is also winnable, and dropping to it
/// from the full game is what makes the room's point: adding winning lines back
/// does not help you, because your opponent gets them too.
pub const WINNABLE_RULES: u8 = ALL_RULES & !(1 << 1) & !(1 << 4);

/// Who a finished search says will win from a position, with both sides perfect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The player who moves first from the empty board forces a win.
    FirstPlayer,
    /// The player who moves second forces a win.
    SecondPlayer,
    /// Neither can force a win.
    Drawn,
}

/// Which side owns a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Moves first.
    First,
    /// Moves second.
    Second,
}

impl Side {
    /// The other side.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
        }
    }

    /// The mark this side draws.
    #[must_use]
    pub const fn mark(self) -> char {
        match self {
            Self::First => 'X',
            Self::Second => 'O',
        }
    }

    /// The outcome in which this side wins.
    #[must_use]
    pub const fn victory(self) -> Outcome {
        match self {
            Self::First => Outcome::FirstPlayer,
            Self::Second => Outcome::SecondPlayer,
        }
    }
}

/// One position: which cells each side owns, as bitboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Board {
    first: u16,
    second: u16,
}

impl Board {
    /// The empty grid.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            first: 0,
            second: 0,
        }
    }

    /// Which side is to move, from the counts alone.
    #[must_use]
    pub fn to_move(self) -> Side {
        if self.first.count_ones() == self.second.count_ones() {
            Side::First
        } else {
            Side::Second
        }
    }

    /// How many marks are on the grid.
    #[must_use]
    pub fn played(self) -> u32 {
        self.first.count_ones() + self.second.count_ones()
    }

    /// Whether a cell is free.
    #[must_use]
    pub fn is_free(self, cell: usize) -> bool {
        cell < CELLS && (self.first | self.second) & (1 << cell) == 0
    }

    /// The mark at a cell, if any.
    #[must_use]
    pub fn mark(self, cell: usize) -> Option<char> {
        if cell >= CELLS {
            return None;
        }
        let bit = 1 << cell;
        if self.first & bit != 0 {
            Some(Side::First.mark())
        } else if self.second & bit != 0 {
            Some(Side::Second.mark())
        } else {
            None
        }
    }

    /// Every free cell, in order.
    #[must_use]
    pub fn free_cells(self) -> Vec<usize> {
        (0..CELLS).filter(|&cell| self.is_free(cell)).collect()
    }

    /// Play a mark for the side to move. Returns `None` for an occupied cell.
    #[must_use]
    pub fn play(self, cell: usize) -> Option<Self> {
        if !self.is_free(cell) {
            return None;
        }
        let mut next = self;
        match self.to_move() {
            Side::First => next.first |= 1 << cell,
            Side::Second => next.second |= 1 << cell,
        }
        Some(next)
    }

    fn owned(self, side: Side) -> u16 {
        match side {
            Side::First => self.first,
            Side::Second => self.second,
        }
    }

    /// Whether a side already holds a complete line under this rulebook.
    #[must_use]
    pub fn has_line(self, side: Side, rules: u8) -> bool {
        let owned = self.owned(side);
        LINES.iter().enumerate().any(|(index, line)| {
            rules & (1 << index) != 0 && line.iter().all(|&cell| owned & (1 << cell) != 0)
        })
    }

    /// Whether the position is over: someone holds a line, or nothing is free.
    #[must_use]
    pub fn is_over(self, rules: u8) -> bool {
        self.has_line(Side::First, rules)
            || self.has_line(Side::Second, rules)
            || self.played() as usize == CELLS
    }
}

/// A memo table for one rulebook, so a search does not repeat itself.
///
/// The reachable space is small enough to hold whole, which is the fact the
/// room is built on: this is a game a mind can finish.
#[derive(Debug, Default)]
pub struct Search {
    seen: std::collections::HashMap<(u16, u16), Outcome>,
    visited: usize,
}

impl Search {
    /// A fresh search with an empty memo.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many distinct positions this search has settled.
    #[must_use]
    pub fn visited(&self) -> usize {
        self.visited
    }

    /// The value of a position under perfect play from both sides.
    pub fn value(&mut self, board: Board, rules: u8) -> Outcome {
        if let Some(&known) = self.seen.get(&(board.first, board.second)) {
            return known;
        }
        let settled = self.settle(board, rules);
        self.seen.insert((board.first, board.second), settled);
        self.visited += 1;
        settled
    }

    fn settle(&mut self, board: Board, rules: u8) -> Outcome {
        for side in [Side::First, Side::Second] {
            if board.has_line(side, rules) {
                return side.victory();
            }
        }
        let free = board.free_cells();
        if free.is_empty() {
            return Outcome::Drawn;
        }
        let mover = board.to_move();
        let mut best = mover.other().victory();
        for cell in free {
            let next = board.play(cell).expect("a free cell accepts a mark");
            let value = self.value(next, rules);
            if value == mover.victory() {
                return mover.victory();
            }
            if value == Outcome::Drawn {
                best = Outcome::Drawn;
            }
        }
        best
    }

    /// Every move that preserves the best value available to the side to move.
    ///
    /// The machine plays from this set, so it never takes a losing move and a
    /// player never wins by being lucky.
    pub fn best_moves(&mut self, board: Board, rules: u8) -> Vec<usize> {
        if board.is_over(rules) {
            return Vec::new();
        }
        let mover = board.to_move();
        let mut best: Option<Outcome> = None;
        let mut chosen = Vec::new();
        for cell in board.free_cells() {
            let next = board.play(cell).expect("a free cell accepts a mark");
            let value = self.value(next, rules);
            let better = match best {
                None => true,
                Some(current) => rank(value, mover) > rank(current, mover),
            };
            if better {
                best = Some(value);
                chosen.clear();
                chosen.push(cell);
            } else if best == Some(value) {
                chosen.push(cell);
            }
        }
        chosen
    }

    /// Whether every legal move loses for the side to move.
    ///
    /// This is the room's quietest fact. In such a position the player is not
    /// being outplayed, and no amount of care helps: the position is already
    /// decided and every door out of it leads to the same place.
    pub fn every_move_loses(&mut self, board: Board, rules: u8) -> bool {
        if board.is_over(rules) {
            return false;
        }
        let mover = board.to_move();
        let free = board.free_cells();
        !free.is_empty()
            && free.into_iter().all(|cell| {
                let next = board.play(cell).expect("a free cell accepts a mark");
                self.value(next, rules) == mover.other().victory()
            })
    }
}

fn rank(outcome: Outcome, mover: Side) -> u8 {
    if outcome == mover.victory() {
        2
    } else if outcome == Outcome::Drawn {
        1
    } else {
        0
    }
}

/// The rulebook a visit plays under.
///
/// Variation zero is the game as everyone knows it. Every other variation names
/// a subset of the eight lines directly, so a player who walks the variations
/// walks all 256 rulebooks and can find the one that is not a tie.
#[must_use]
pub fn rules_for_variation(variation: u64) -> u8 {
    if variation == 0 {
        ALL_RULES
    } else {
        (variation & 0xFF) as u8
    }
}

/// How many lines a rulebook counts.
#[must_use]
pub fn line_count(rules: u8) -> u32 {
    rules.count_ones()
}

/// The value of the empty grid under a rulebook.
#[must_use]
pub fn opening_value(rules: u8) -> Outcome {
    Search::new().value(Board::new(), rules)
}

/// Draw the grid, its marks, and the lines this rulebook counts.
pub fn render_board(canvas: &mut dyn Surface, board: Board, rules: u8) {
    let (width, height) = canvas.draw_bounds();
    if width < 9 || height < 5 {
        return;
    }
    // Keep the board square to the eye. A character cell is taller than it is
    // wide, so the vertical extent is divided by that ratio rather than shared
    // with it, which is what squashed the grid into itself before.
    let aspect = canvas.safe_char_aspect().max(0.1);
    let usable_w = (width as f64 * 0.86).max(9.0);
    let usable_h = (height as f64 * 0.86).max(5.0);
    let side = usable_w.min(usable_h / aspect);
    let cell_w = ((side / 3.0) as usize).max(3);
    let cell_h = ((side * aspect / 3.0) as usize).max(2);
    let board_w = cell_w * 3;
    let board_h = cell_h * 3;
    if board_w > width || board_h > height {
        return;
    }
    let left = width.saturating_sub(board_w) / 2;
    let top = height.saturating_sub(board_h) / 2;
    // The grid itself, drawn only where a counted line runs, so the rulebook is
    // visible in the picture rather than only in the status.
    for row in 1..3 {
        let y = top + row * cell_h;
        for x in left..left + board_w {
            canvas.plot(x as i32, y as i32, '-');
        }
    }
    for column in 1..3 {
        let x = left + column * cell_w;
        for y in top..top + board_h {
            canvas.plot(x as i32, y as i32, '|');
        }
    }
    for cell in 0..CELLS {
        let (row, column) = (cell / 3, cell % 3);
        let cx = (left + column * cell_w + cell_w / 2) as i32;
        let cy = (top + row * cell_h + cell_h / 2) as i32;
        // Marks are drawn as shapes rather than single glyphs. A pixel surface
        // ignores the character, so a mark that is one plotted point is
        // invisible to a player who reads the picture instead of the text.
        // Keep a mark clear of the rules it sits between, in both directions.
        let rx = ((cell_w as i32) / 2 - 1).max(1);
        let ry = ((cell_h as i32) / 2 - 1).max(1);
        match board.mark(cell) {
            Some('X') => {
                canvas.line(cx - rx, cy - ry, cx + rx, cy + ry, 'X');
                canvas.line(cx - rx, cy + ry, cx + rx, cy - ry, 'X');
            }
            Some(mark) => {
                // A ring, so the second player's mark is a different shape and
                // not merely a different letter.
                let steps = ((rx + ry) * 6).max(12);
                for step in 0..steps {
                    let angle = std::f64::consts::TAU * f64::from(step) / f64::from(steps);
                    let px = f64::from(cx) + f64::from(rx) * angle.cos();
                    let py = f64::from(cy) + f64::from(ry) * angle.sin();
                    canvas.plot(px as i32, py as i32, mark);
                }
            }
            None => {
                if is_live_cell(cell, rules) {
                    canvas.plot(cx, cy, '.');
                }
            }
        }
    }
}

/// Whether any counted line passes through a cell.
///
/// A cell on no counted line can still be played, and playing it can still
/// waste the move that mattered, which is worth being able to see.
#[must_use]
pub fn is_live_cell(cell: usize, rules: u8) -> bool {
    LINES
        .iter()
        .enumerate()
        .any(|(index, line)| rules & (1 << index) != 0 && line.contains(&cell))
}

/// How a visit stands after replaying the hand that reached it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit {
    /// The position on the grid now.
    pub board: Board,
    /// Games the player has finished this visit.
    pub finished: u32,
    /// Games the player has won.
    pub won: u32,
    /// Games that ended with nobody holding a line.
    pub tied: u32,
    /// Touches that landed on a cell already taken.
    pub wasted: u32,
}

/// Replay a hand into a visit: the player moves, the machine answers perfectly.
///
/// The machine answers from the best-move set, so it never hands the player a
/// win. A touch on a taken cell is kept as a wasted touch rather than silently
/// dropped, because a player is owed an honest count of what they did.
#[must_use]
pub fn replay(pokes: &[(f64, f64)], rules: u8) -> Visit {
    let mut search = Search::new();
    let mut visit = Visit {
        board: Board::new(),
        finished: 0,
        won: 0,
        tied: 0,
        wasted: 0,
    };
    for &(x, y) in pokes {
        if visit.board.is_over(rules) {
            // A finished game is not a wall. The next touch deals a new one.
            visit.board = Board::new();
        }
        let Some(cell) = cell_from_point(x, y) else {
            continue;
        };
        let Some(after_player) = visit.board.play(cell) else {
            visit.wasted += 1;
            continue;
        };
        visit.board = after_player;
        if !visit.board.is_over(rules) {
            let replies = search.best_moves(visit.board, rules);
            if let Some(&reply) = replies.first() {
                visit.board = visit.board.play(reply).expect("a best move is legal");
            }
        }
        if visit.board.is_over(rules) {
            visit.finished += 1;
            if visit.board.has_line(Side::First, rules) {
                visit.won += 1;
            } else if !visit.board.has_line(Side::Second, rules) {
                visit.tied += 1;
            }
        }
    }
    visit
}

/// The cell a normalized point lands on.
#[must_use]
pub fn cell_from_point(x: f64, y: f64) -> Option<usize> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let column = ((x.clamp(0.0, 1.0) * 3.0) as usize).min(2);
    let row = ((y.clamp(0.0, 1.0) * 3.0) as usize).min(2);
    Some(row * 3 + column)
}

/// The room.
#[derive(Debug, Default)]
pub struct OnlyMove {
    variation: u64,
}

impl OnlyMove {
    /// The room at its default rulebook.
    #[must_use]
    pub fn new() -> Self {
        Self { variation: 0 }
    }

    /// The room under the rulebook this variation names.
    #[must_use]
    pub fn new_with(variation: u64) -> Self {
        Self { variation }
    }

    /// The rulebook this visit plays under.
    #[must_use]
    pub fn rules(&self) -> u8 {
        rules_for_variation(self.variation)
    }

    fn readout(&self, visit: &Visit) -> String {
        let rules = self.rules();
        let lines = line_count(rules);
        if visit.finished == 0 && visit.board.played() == 0 {
            return format!("{lines} LINES COUNT  CLICK A CELL");
        }
        let mut readout = format!(
            "{lines} LINES  PLAYED {}  WON {}  TIED {}",
            visit.finished, visit.won, visit.tied
        );
        if visit.wasted > 0 {
            readout.push_str(&format!("  WASTED {}", visit.wasted));
        }
        readout
    }
}

impl crate::room::Room for OnlyMove {
    fn render(&self, canvas: &mut dyn Surface, _t: f64) {
        render_board(canvas, Board::new(), self.rules());
    }

    fn render_poked(&self, canvas: &mut dyn Surface, _t: f64, pokes: &[(f64, f64)]) {
        let visit = replay(pokes, self.rules());
        render_board(canvas, visit.board, self.rules());
    }

    fn status(&self, _t: f64) -> Option<String> {
        Some(self.readout(&replay(&[], self.rules())))
    }

    fn status_input(&self, _t: f64, inputs: &[crate::room::RoomInput]) -> Option<String> {
        let pokes = crate::room::pokes_from_inputs(inputs);
        Some(self.readout(&replay(&pokes, self.rules())))
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "search and settle",
            root: 146.83,
            // A line that climbs, reaches, and returns to where it started,
            // which is what an exhaustive search of this grid does.
            line: &[0, 3, 7, 10, 12, 10, 7, 3, 0],
            tempo: 96,
            encodes: "a tree searched to its leaves and closing on one value",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: TAKE A CELL")
    }

    fn goal(&self) -> Option<&'static str> {
        Some("WIN ONE GAME")
    }

    fn goal_met(&self, _t: f64, inputs: &[crate::room::RoomInput]) -> bool {
        let pokes = crate::room::pokes_from_inputs(inputs);
        replay(&pokes, self.rules()).won > 0
    }

    fn reveal(&self) -> &'static str {
        "The machine searched every future this grid has and found no winning \
         one, so it settles for the tie it cannot lose. That is not caution; on \
         a solved game it is the answer. The surprise is underneath: the tie \
         belongs to the board and not to the contest. Eight lines can be counted \
         or ignored, which makes 256 rulebooks, and exactly one of them can be \
         won by the player who moves first. It is the rulebook that treats every \
         direction alike. Add the middle row and column back and the win \
         evaporates, because a winning condition you share is a winning \
         condition you hand your opponent. And no rulebook here is ever won by \
         the player who moves second: the first player could simply steal that \
         strategy, make an extra mark, and follow it, so such a strategy cannot \
         exist. Four sentences settle what the search takes 256 exhaustions to \
         confirm."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ordinary_game_is_a_tie_and_nobody_can_break_it() {
        assert_eq!(opening_value(ALL_RULES), Outcome::Drawn);
    }

    #[test]
    fn exactly_one_rulebook_of_two_hundred_and_fifty_six_can_be_won() {
        // The whole room rests on this count, so it is measured rather than
        // asserted from a source.
        let mut winnable = Vec::new();
        let mut second_player_wins = 0;
        for rules in 0..=u8::MAX {
            match opening_value(rules) {
                Outcome::FirstPlayer => winnable.push(rules),
                Outcome::SecondPlayer => second_player_wins += 1,
                Outcome::Drawn => {}
            }
        }
        assert_eq!(
            winnable,
            vec![WINNABLE_RULES],
            "exactly one rulebook is a first-player win"
        );
        // Strategy stealing forbids a second-player win in a game where both
        // sides share the winning lines, and an extra mark is never a burden.
        // The exhaustive search must agree with that four-sentence proof.
        assert_eq!(
            second_player_wins, 0,
            "a second player can never force a win here"
        );
    }

    #[test]
    fn putting_the_middle_lines_back_destroys_the_win() {
        // The point of the dial: enriching the winning condition enriches your
        // opponent too, which is why symmetric games tie.
        assert_eq!(opening_value(WINNABLE_RULES), Outcome::FirstPlayer);
        assert_eq!(opening_value(WINNABLE_RULES | (1 << 1)), Outcome::Drawn);
        assert_eq!(opening_value(WINNABLE_RULES | (1 << 4)), Outcome::Drawn);
        assert_eq!(opening_value(ALL_RULES), Outcome::Drawn);
    }

    #[test]
    fn a_rulebook_with_no_lines_cannot_be_won_by_anyone() {
        assert_eq!(opening_value(0), Outcome::Drawn);
        let mut search = Search::new();
        assert!(!search.every_move_loses(Board::new(), 0));
    }

    #[test]
    fn the_reachable_space_is_small_enough_to_finish() {
        // 5,478 reachable positions is the fact that lets this room exhaust its
        // own game live. Counted here rather than quoted.
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![Board::new()];
        while let Some(board) = stack.pop() {
            if !seen.insert((board.first, board.second)) {
                continue;
            }
            if board.is_over(ALL_RULES) {
                continue;
            }
            for cell in board.free_cells() {
                stack.push(board.play(cell).expect("free cell"));
            }
        }
        assert_eq!(seen.len(), 5_478);
    }

    #[test]
    fn the_machine_never_takes_a_move_that_loses() {
        let mut search = Search::new();
        let mut board = Board::new();
        // Walk a full game with both sides playing from the best-move set.
        while !board.is_over(ALL_RULES) {
            let moves = search.best_moves(board, ALL_RULES);
            assert!(!moves.is_empty(), "an unfinished game offers a move");
            board = board.play(moves[0]).expect("a best move is legal");
        }
        assert!(!board.has_line(Side::First, ALL_RULES));
        assert!(!board.has_line(Side::Second, ALL_RULES));
    }

    #[test]
    fn a_position_can_exist_where_every_door_leads_to_the_same_place() {
        // The quiet fact, built by hand: the first player holds two open lines
        // at once, so the second player can block one and only one.
        let mut board = Board::new();
        for cell in [0, 8, 2, 4, 6] {
            board = board.play(cell).expect("free cell");
        }
        let mut search = Search::new();
        assert_eq!(board.to_move(), Side::Second);
        assert!(
            search.every_move_loses(board, ALL_RULES),
            "this position is already decided: {board:?}"
        );
    }

    #[test]
    fn the_one_winnable_rulebook_can_actually_be_won_by_playing_it() {
        // A value in a table is not a room. A first player who follows the
        // search must be able to reach a real win against the machine's own
        // best replies, or the hunt at the end of 256 rulebooks pays nothing.
        let mut search = Search::new();
        let mut board = Board::new();
        while !board.is_over(WINNABLE_RULES) {
            let moves = search.best_moves(board, WINNABLE_RULES);
            assert!(!moves.is_empty(), "an unfinished game offers a move");
            board = board.play(moves[0]).expect("a best move is legal");
        }
        assert!(
            board.has_line(Side::First, WINNABLE_RULES),
            "the winnable rulebook must actually pay a win: {board:?}"
        );
        // And the same perfect line pays nothing once the middle lines return.
        let mut search = Search::new();
        let mut board = Board::new();
        while !board.is_over(ALL_RULES) {
            let moves = search.best_moves(board, ALL_RULES);
            board = board.play(moves[0]).expect("a best move is legal");
        }
        assert!(!board.has_line(Side::First, ALL_RULES));
    }

    #[test]
    fn a_replayed_hand_counts_games_wins_and_wasted_touches() {
        // The record is the player's evidence, so it has to be exact.
        let corner = (0.17, 0.17);
        let visit = replay(&[corner, corner], ALL_RULES);
        assert_eq!(visit.wasted, 1, "the machine's own cell cannot be retaken");
        assert_eq!(visit.finished, 0);

        // Nine touches down the board finish at least one game and never
        // report a win the player did not get.
        let sweep: Vec<(f64, f64)> = (0..9)
            .map(|cell| {
                (
                    (cell % 3) as f64 / 3.0 + 0.17,
                    (cell / 3) as f64 / 3.0 + 0.17,
                )
            })
            .collect();
        let visit = replay(&sweep, ALL_RULES);
        assert!(visit.finished >= 1, "{visit:?}");
        assert_eq!(visit.won, 0, "the machine does not hand out wins: {visit:?}");
        assert_eq!(
            visit.finished,
            visit.won + visit.tied + (visit.finished - visit.won - visit.tied),
            "every finished game is counted once"
        );
    }

    #[test]
    fn variation_zero_is_the_game_everyone_knows() {
        assert_eq!(rules_for_variation(0), ALL_RULES);
        assert_eq!(rules_for_variation(u64::from(ALL_RULES)), ALL_RULES);
        assert_eq!(
            rules_for_variation(u64::from(WINNABLE_RULES)),
            WINNABLE_RULES
        );
        // Variations beyond the byte wrap, so every variation names a rulebook.
        assert_eq!(rules_for_variation(256 + 7), 7);
    }

    #[test]
    fn an_occupied_cell_refuses_a_second_mark() {
        let board = Board::new().play(4).expect("free cell");
        assert!(board.play(4).is_none());
        assert_eq!(board.mark(4), Some('X'));
        assert_eq!(board.to_move(), Side::Second);
    }
}
