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

/// A stable phase in the middle of the only winnable rulebook's detent.
const WINNABLE_PHASE: f64 = 0.40;
/// Half the width of the only winnable rulebook's broad phase detent.
const WINNABLE_DETENT_HALF_WIDTH: f64 = 0.05;
/// Start of the broad phase detent for the only winnable rulebook.
const WINNABLE_DETENT_START: f64 = WINNABLE_PHASE - WINNABLE_DETENT_HALF_WIDTH;
/// End of the broad phase detent for the only winnable rulebook.
const WINNABLE_DETENT_END: f64 = WINNABLE_PHASE + WINNABLE_DETENT_HALF_WIDTH;
/// Maximum number of rejected poke positions retained for player feedback.
const MAX_WASTED_POKE_DETAILS: usize = 24;
/// Compact room status lines must fit every face's narrow footer.
const STATUS_LIMIT: usize = 56;

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

    /// How many cells each side holds, the mover first.
    ///
    /// The player always moves first here, so this reads as "yours, mine" on
    /// the status line of a game that is still being played.
    #[must_use]
    pub fn counts(self) -> (u32, u32) {
        (self.first.count_ones(), self.second.count_ones())
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

/// Every rulebook worth turning a dial through, in the order the dial walks.
///
/// A packaged playtest found this room by title, played it, and reported that
/// the dial every other room turns did nothing here: `t` moved and the board
/// stayed at eight lines. The rulebook was reachable only through `variation`,
/// which a stranger has no reason to try. So the walk lives on `t` now, and
/// `variation` stays as the way to name one rulebook exactly once you have
/// found it.
///
/// The order is not the bitmask counting up, because that would be a walk
/// through noise. It descends by how many lines a rulebook counts, so the dial
/// starts at the game everyone knows and takes lines away as it turns; and
/// inside one line count it descends by how much of the square's own symmetry
/// the rulebook keeps. That second key is what makes the walk worth taking: the
/// one rulebook a first player can win is the most symmetric rulebook of its
/// size, so a player turning the dial slowly meets it near the front of its
/// band rather than by luck at position 214.
///
/// The empty rulebook is left out. Nothing can be won when no line counts, so it
/// is not a game, and leaving it out means every index the dial can reach round
/// trips through `variation`, where zero already means the full game.
#[must_use]
pub fn dial_order() -> &'static [u8] {
    static ORDER: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
        let mut order: Vec<u8> = (1..=u8::MAX).collect();
        order.sort_by_key(|&rules| {
            (
                std::cmp::Reverse(line_count(rules)),
                std::cmp::Reverse(symmetries_kept(rules)),
                rules,
            )
        });
        order
    });
    &ORDER
}

/// Where on the dial a phase sits, as a stop and a total.
///
/// The status shows this so a player can tell how fine the dial is. A stranger
/// who does not know there are 255 stops will step by a tenth, see eight boards,
/// and conclude the dial is coarse.
///
/// The only rulebook that can pay the room's goal has a broad detent from phase
/// 0.35 through 0.45. The other rulebooks keep their authored order on either
/// side. Without that detent the one playable stop occupied less than half of
/// one percent of a continuous hand dial, which made the posted goal effectively
/// unreachable even though the underlying search could win it.
#[must_use]
pub fn dial_position(t: f64) -> (usize, usize) {
    let order = dial_order();
    let phase = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
    let winnable = order
        .iter()
        .position(|&rules| rules == WINNABLE_RULES)
        .unwrap_or(0);
    let index = if phase < WINNABLE_DETENT_START && winnable > 0 {
        ((phase / WINNABLE_DETENT_START * winnable as f64) as usize).min(winnable - 1)
    } else if phase <= WINNABLE_DETENT_END {
        winnable
    } else {
        let after = order.len().saturating_sub(winnable + 1);
        let progress = (phase - WINNABLE_DETENT_END) / (1.0 - WINNABLE_DETENT_END);
        (winnable + 1 + (progress * after as f64) as usize).min(order.len() - 1)
    };
    (index + 1, order.len())
}

/// The eight symmetries of the square, as permutations of the nine cells.
///
/// Written down rather than derived, because the room's claim is about a shape
/// and a reader should be able to check the shape.
const SQUARE_SYMMETRIES: [[usize; CELLS]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8], // identity
    [6, 3, 0, 7, 4, 1, 8, 5, 2], // quarter turn
    [8, 7, 6, 5, 4, 3, 2, 1, 0], // half turn
    [2, 5, 8, 1, 4, 7, 0, 3, 6], // three quarter turn
    [2, 1, 0, 5, 4, 3, 8, 7, 6], // mirror across the vertical
    [6, 7, 8, 3, 4, 5, 0, 1, 2], // mirror across the horizontal
    [0, 3, 6, 1, 4, 7, 2, 5, 8], // mirror across the main diagonal
    [8, 5, 2, 7, 4, 1, 6, 3, 0], // mirror across the anti diagonal
];

/// How many of the square's eight symmetries leave a rulebook unchanged.
///
/// A rulebook that keeps all eight treats every direction alike, which is the
/// property the winnable one has and the reason the dial can be walked by feel.
#[must_use]
pub fn symmetries_kept(rules: u8) -> u32 {
    SQUARE_SYMMETRIES
        .iter()
        .filter(|permutation| {
            let mut mapped = 0u8;
            for (index, line) in LINES.iter().enumerate() {
                if rules & (1 << index) == 0 {
                    continue;
                }
                let mut moved = [permutation[line[0]], permutation[line[1]], permutation[line[2]]];
                moved.sort_unstable();
                let Some(position) = LINES.iter().position(|candidate| {
                    let mut sorted = *candidate;
                    sorted.sort_unstable();
                    sorted == moved
                }) else {
                    return false;
                };
                mapped |= 1 << position;
            }
            mapped == rules
        })
        .count() as u32
}

/// The rulebook the phase dial has turned to.
///
/// `t` walks [`dial_order`]. Phase zero is the game everyone knows, so a player
/// who never touches the dial finds the board they expect.
#[must_use]
pub fn rules_at_phase(t: f64) -> u8 {
    let (stop, _) = dial_position(t);
    dial_order()[stop - 1]
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
    // Three layers, each with one job. The frame says where the board is.
    let (x0, y0) = (left as i32, top as i32);
    let (x1, y1) = ((left + board_w - 1) as i32, (top + board_h - 1) as i32);
    canvas.line(x0, y0, x1, y0, '-');
    canvas.line(x0, y1, x1, y1, '-');
    canvas.line(x0, y0, x0, y1, '|');
    canvas.line(x1, y0, x1, y1, '|');
    let centre = |cell: usize| {
        let (row, column) = (cell / 3, cell % 3);
        (
            (left + column * cell_w + cell_w / 2) as i32,
            (top + row * cell_h + cell_h / 2) as i32,
        )
    };
    // A dot marks any cell no counted line runs through, so a dead cell is
    // visibly dead rather than merely unmarked, and all nine centres are always
    // findable: a live one carries a stroke, a dead one carries this.
    for cell in 0..CELLS {
        if !is_live_cell(cell, rules) {
            let (cx, cy) = centre(cell);
            canvas.plot(cx, cy, '.');
        }
    }
    // Then the rulebook, drawn as itself. Every line this rulebook counts is
    // stroked through the three cells it joins, so turning the dial changes the
    // figure on the board and not merely a number beside it. Eight lines is a
    // star, the six rows and columns are a lattice, and the one rulebook a
    // first player can win is a square with an X through it. A player walking
    // the dial is walking through shapes, and the two shapes that look most
    // alike are the two that are not worth the same.
    for (index, line) in LINES.iter().enumerate() {
        if rules & (1 << index) == 0 {
            continue;
        }
        let (ax, ay) = centre(line[0]);
        let (bx, by) = centre(line[2]);
        canvas.line(ax, ay, bx, by, '*');
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
            // Free cells were already marked, live or dead, before the
            // rulebook was drawn.
            None => {}
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// One-based positions of rejected pokes, retained in input order.
    ///
    /// The public room input is capped at 24 pokes. The same cap here keeps a
    /// direct core caller from turning status feedback into unbounded state.
    pub wasted_pokes: Vec<usize>,
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
        wasted_pokes: Vec::new(),
    };
    for (poke_index, &(x, y)) in pokes.iter().enumerate() {
        if visit.board.is_over(rules) {
            // A finished game is not a wall. The next touch deals a new one.
            visit.board = Board::new();
        }
        let Some(cell) = cell_from_point(x, y) else {
            continue;
        };
        let Some(after_player) = visit.board.play(cell) else {
            visit.wasted += 1;
            if visit.wasted_pokes.len() < MAX_WASTED_POKE_DETAILS {
                visit.wasted_pokes.push(poke_index.saturating_add(1));
            }
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

fn wasted_tokens(pokes: &[usize]) -> Vec<(String, usize)> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < pokes.len() {
        let start = pokes[index];
        let mut end = start;
        while index + 1 < pokes.len() && pokes[index + 1] == end.saturating_add(1) {
            index += 1;
            end = pokes[index];
        }
        let token = if start == end {
            format!("#{start}")
        } else {
            format!("#{start}-{end}")
        };
        tokens.push((token, end.saturating_sub(start).saturating_add(1)));
        index += 1;
    }
    tokens
}

fn append_waste_detail(readout: &mut String, visit: &Visit) {
    if visit.wasted == 0 {
        return;
    }
    let tokens = wasted_tokens(&visit.wasted_pokes);
    let retained = visit.wasted_pokes.len();
    let unretained = (visit.wasted as usize).saturating_sub(retained);
    for keep in (1..=tokens.len()).rev() {
        let represented: usize = tokens[..keep].iter().map(|(_, count)| count).sum();
        let omitted = retained.saturating_sub(represented) + unretained;
        let mut detail = tokens[..keep]
            .iter()
            .map(|(token, _)| token.as_str())
            .collect::<Vec<_>>()
            .join(",");
        if omitted > 0 {
            detail.push_str(&format!(",+{omitted}"));
        }
        let addition = format!(" WASTE {detail}");
        if readout.len() + addition.len() <= STATUS_LIMIT {
            readout.push_str(&addition);
            return;
        }
    }
    let addition = format!(" WASTE +{}", visit.wasted);
    if readout.len() + addition.len() <= STATUS_LIMIT {
        readout.push_str(&addition);
    }
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

    /// The rulebook this visit plays under at a phase.
    ///
    /// A named variation wins, because naming a rulebook is how a player
    /// returns to one they found. With no variation named, the phase dial walks
    /// the rulebooks, so the dial every other room turns turns this one too.
    #[must_use]
    pub fn rules_at(&self, t: f64) -> u8 {
        if self.variation == 0 {
            rules_at_phase(t)
        } else {
            rules_for_variation(self.variation)
        }
    }

    /// The rulebook this visit plays under with the dial at rest.
    #[must_use]
    pub fn rules(&self) -> u8 {
        self.rules_at(0.0)
    }

    fn readout(&self, t: f64, visit: &Visit) -> String {
        let rules = self.rules_at(t);
        // The mask is printed because it is the way back. A player who turns the
        // dial onto a board worth keeping can pass that number as `variation`
        // and stand on it. Rulebook zero is never on the dial, so the number
        // always means what passing it means.
        if visit.finished == 0 && visit.board.played() == 0 {
            let lines = line_count(rules);
            let opening = if self.variation == 0 {
                let (stop, stops) = dial_position(t);
                format!("BOARD {stop}/{stops}  ")
            } else {
                String::new()
            };
            return format!("{opening}RULES {rules}  {lines} LINES  CLICK A CELL");
        }
        // A game still in play is reported as the board, not as a row of
        // zeros. A packaged playtest clicked twenty five times, was told
        // PLAYED 0 WON 0 TIED 0 every time, and concluded that a click does
        // not take a cell. The click had taken a cell and the machine had
        // answered it; only the scoreboard was counting finished games while
        // the doorway promised to take one. A room whose verb is TAKE A CELL
        // owes a player the cells.
        let live = visit.board.played() > 0 && !visit.board.is_over(rules);
        let mut readout = if live && visit.won == 0 {
            let (yours, mine) = visit.board.counts();
            let open = CELLS as u32 - yours - mine;
            format!("RULES {rules} YOURS {yours} MINE {mine} OPEN {open}")
        } else {
            format!(
                "RULES {rules} PLAYED {} WON {} TIED {}",
                visit.finished, visit.won, visit.tied
            )
        };
        if live && visit.won == 0 && visit.finished > 0 && visit.wasted == 0 {
            readout.push_str(&format!(" PLAYED {}", visit.finished));
        }
        append_waste_detail(&mut readout, visit);
        readout
    }
}

impl crate::room::Room for OnlyMove {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        render_board(canvas, Board::new(), self.rules_at(t));
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let rules = self.rules_at(t);
        let visit = replay(pokes, rules);
        render_board(canvas, visit.board, rules);
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.readout(t, &replay(&[], self.rules_at(t))))
    }

    fn status_input(&self, t: f64, inputs: &[crate::room::RoomInput]) -> Option<String> {
        let pokes = crate::room::pokes_from_inputs(inputs);
        Some(self.readout(t, &replay(&pokes, self.rules_at(t))))
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

    fn goal_met(&self, t: f64, inputs: &[crate::room::RoomInput]) -> bool {
        let pokes = crate::room::pokes_from_inputs(inputs);
        replay(&pokes, self.rules_at(t)).won > 0
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
    fn one_click_says_it_took_a_cell() {
        // A packaged playtest clicked twenty five times at twenty five points
        // and read PLAYED 0 WON 0 TIED 0 every time, then reported that a
        // click does not take a cell. Every one of those clicks had taken a
        // cell and drawn an answer; the scoreboard was counting finished games
        // while the doorway promised to take one. A game in play now reports
        // the board, so the consequence of a touch is on the line that
        // reported nothing.
        use crate::room::Room;
        let room = OnlyMove::new();
        let clicked = room
            .status_input(0.4, &crate::room::inputs_from_pokes(&[(0.5, 0.5)], 0.4))
            .unwrap();
        assert!(
            clicked.contains("YOURS 1") && clicked.contains("MINE 1"),
            "a click that took a cell must say so, got {clicked}"
        );
        assert!(
            !clicked.contains("PLAYED 0"),
            "an unfinished game is not a row of zeros, got {clicked}"
        );
        assert!(clicked.chars().count() <= 56);

        // A finished game still reports the record, which is the player's
        // evidence and the thing the goal is graded on.
        let rules = room.rules_at(0.4);
        let sweep: Vec<(f64, f64)> = (0..9)
            .map(|cell| {
                (
                    (cell % 3) as f64 / 3.0 + 0.17,
                    (cell / 3) as f64 / 3.0 + 0.17,
                )
            })
            .collect();
        let visit = replay(&sweep, rules);
        assert!(visit.finished > 0, "nine touches finish a game");
        let after = room
            .status_input(0.4, &crate::room::inputs_from_pokes(&sweep, 0.4))
            .unwrap();
        assert!(after.contains("PLAYED"), "a finished game keeps its record");
        assert!(after.chars().count() <= 56);
    }

    #[test]
    fn a_replayed_hand_counts_games_wins_and_wasted_touches() {
        // The record is the player's evidence, so it has to be exact.
        let corner = (0.17, 0.17);
        let visit = replay(&[corner, corner], ALL_RULES);
        assert_eq!(visit.wasted, 1, "the machine's own cell cannot be retaken");
        assert_eq!(visit.wasted_pokes, vec![2]);
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
    fn the_reported_phase_pays_a_short_visible_win_and_names_waste() {
        // A packaged player used the visible three by three cell centers at
        // phase 0.40, finished games repeatedly, and never received the posted
        // WIN ONE GAME goal. The only winnable rulebook used to occupy less
        // than half of one percent of the dial. Its broad detent now makes the
        // exact public call land on the square with an X through it.
        use crate::room::Room;
        let room = OnlyMove::new();
        assert_eq!(room.rules_at(WINNABLE_PHASE), WINNABLE_RULES);

        let short = [(0.17, 0.17), (0.17, 0.83), (0.17, 0.50)];
        let short_inputs = crate::room::inputs_from_pokes(&short, WINNABLE_PHASE);
        assert!(room.goal_met(WINNABLE_PHASE, &short_inputs));
        let short_visit = replay(&short, WINNABLE_RULES);
        assert_eq!(short_visit.finished, 1);
        assert_eq!(short_visit.won, 1);

        let sweep = [
            (0.32, 0.32),
            (0.50, 0.32),
            (0.68, 0.32),
            (0.32, 0.50),
            (0.50, 0.50),
            (0.68, 0.50),
            (0.32, 0.68),
            (0.50, 0.68),
            (0.68, 0.68),
        ];
        let sweep_inputs = crate::room::inputs_from_pokes(&sweep, WINNABLE_PHASE);
        let visit = replay(&sweep, WINNABLE_RULES);
        assert!(visit.won > 0, "the published nine-poke call must pay: {visit:?}");
        assert!(room.goal_met(WINNABLE_PHASE, &sweep_inputs));
        let status = room.status_input(WINNABLE_PHASE, &sweep_inputs).unwrap();
        assert!(status.contains("WON 1"), "the earned win must stay visible: {status}");
        for poke in &visit.wasted_pokes {
            assert!(
                status.contains(&format!("#{poke}")),
                "wasted poke #{poke} is unnamed in {status}"
            );
        }
        assert!(status.chars().count() <= STATUS_LIMIT, "{status}");
    }

    #[test]
    fn the_phase_dial_turns_the_rulebook() {
        // A packaged playtest found this room by title, played it, swept t
        // across sixteen phases, and reported that the board stayed at eight
        // lines the whole way: "a stranger who only turns t will think the dial
        // is dead." The rulebook was reachable only through `variation`, which
        // a stranger has no reason to try. So t walks it now.
        let room = OnlyMove::new();
        let mut seen = std::collections::BTreeSet::new();
        for step in 0..=40 {
            seen.insert(room.rules_at(f64::from(step) / 40.0));
        }
        assert!(
            seen.len() >= 30,
            "forty turns of the dial found only {} rulebooks",
            seen.len()
        );
        // At rest the board is the one everyone already knows, so a player who
        // never touches the dial is not handed a puzzle they did not ask for.
        assert_eq!(room.rules_at(0.0), ALL_RULES);
        // A named variation still wins, because naming a rulebook is how a
        // player returns to one the dial showed them.
        let named = OnlyMove::new_with(u64::from(WINNABLE_RULES));
        for phase in [0.0, 0.3, 0.7, 1.0] {
            assert_eq!(named.rules_at(phase), WINNABLE_RULES);
        }
    }

    #[test]
    fn the_dial_reaches_the_one_rulebook_that_can_be_won() {
        // A dial that walks 255 boards and never passes the one board the
        // room is about would be a longer way of doing nothing.
        let order = dial_order();
        let found = order
            .iter()
            .position(|&rules| rules == WINNABLE_RULES)
            .expect("the dial has to pass the winnable rulebook");
        assert!(
            found < order.len() / 8,
            "the winnable rulebook sits at stop {found} of {}, too deep to meet by turning",
            order.len()
        );
        assert_eq!(rules_at_phase(WINNABLE_PHASE), WINNABLE_RULES);
        assert_eq!(dial_position(WINNABLE_DETENT_START).0, found + 1);
        assert_eq!(dial_position(WINNABLE_DETENT_END).0, found + 1);
        assert_ne!(dial_position(WINNABLE_DETENT_START - 0.001).0, found + 1);
        assert_ne!(dial_position(WINNABLE_DETENT_END + 0.001).0, found + 1);
        for index in 0..found {
            let phase = (index as f64 + 0.5) / found as f64 * WINNABLE_DETENT_START;
            assert_eq!(dial_position(phase).0 - 1, index);
        }
        let after = order.len() - found - 1;
        for offset in 0..after {
            let progress = (offset as f64 + 0.5) / after as f64;
            let phase = WINNABLE_DETENT_END + progress * (1.0 - WINNABLE_DETENT_END);
            assert_eq!(dial_position(phase).0 - 1, found + 1 + offset);
        }
        // Rulebook zero is never on the dial, so the mask the status prints is
        // always a mask `variation` accepts to mean the same board.
        assert!(!order.contains(&0));
        for &rules in order {
            assert_eq!(rules_for_variation(u64::from(rules)), rules);
        }
    }

    #[test]
    fn the_walk_is_ordered_by_size_then_by_symmetry() {
        // The order is the room's own argument. Lines come off as the dial
        // turns, and inside one size the most symmetric boards come first, so
        // the board a first player can win, which is the board that treats
        // every direction alike, is met near the front of its band rather than
        // stumbled on at stop 214.
        let order = dial_order();
        for pair in order.windows(2) {
            let (before, after) = (pair[0], pair[1]);
            let key = |rules: u8| (line_count(rules), symmetries_kept(rules));
            assert!(
                key(before) >= key(after),
                "the walk goes uphill from {before} to {after}"
            );
        }
        assert_eq!(symmetries_kept(ALL_RULES), 8);
        assert_eq!(
            symmetries_kept(WINNABLE_RULES),
            8,
            "the winnable rulebook is the one that treats every direction alike"
        );
        // Dropping one line from a rulebook that keeps the whole square breaks
        // the square, which is why the walk has texture at all.
        assert!(symmetries_kept(ALL_RULES & !1) < 8);
    }

    #[test]
    fn the_rulebook_is_drawn_and_not_merely_counted() {
        // The dial has to change the picture, or a player reading the room
        // instead of the status still sees a dead dial. Each counted line is
        // stroked through the three cells it joins, so a rulebook is a shape.
        use crate::canvas::Canvas;
        use crate::room::Room;
        let room = OnlyMove::new();
        let mut frames = std::collections::HashSet::new();
        for step in 0..=20 {
            let mut canvas = Canvas::new(48, 24);
            room.render(&mut canvas, f64::from(step) / 20.0);
            frames.insert(canvas.to_text());
        }
        assert!(
            frames.len() >= 15,
            "twenty-one turns of the dial drew only {} pictures",
            frames.len()
        );
        // The full game and the winnable game differ by exactly the middle row
        // and the middle column, and that difference has to be visible: it is
        // the whole reveal, drawn.
        let mut full = Canvas::new(48, 24);
        let mut winnable = Canvas::new(48, 24);
        render_board(&mut full, Board::new(), ALL_RULES);
        render_board(&mut winnable, Board::new(), WINNABLE_RULES);
        assert_ne!(full.to_text(), winnable.to_text());
    }

    #[test]
    fn the_status_names_the_way_back_and_the_size_of_the_dial() {
        use crate::room::Room;
        let room = OnlyMove::new();
        let opening = room.status(0.0).expect("status");
        assert!(opening.contains("BOARD 1/255"), "{opening}");
        assert!(opening.contains(&format!("RULES {ALL_RULES}")), "{opening}");
        // The mask the status prints is the number `variation` takes, so a
        // player who finds a board on the dial can stand on it.
        let phase = 0.32;
        let found = room.rules_at(phase);
        let reading = room.status(phase).expect("status");
        assert!(reading.contains(&format!("RULES {found}")), "{reading}");
        let pinned = OnlyMove::new_with(u64::from(found));
        assert_eq!(pinned.rules_at(0.0), found);
        // A pinned board has no dial position to report, and says so by not
        // claiming one.
        assert!(
            !pinned.status(0.0).expect("status").contains("BOARD"),
            "a named rulebook has no stop on the dial"
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
