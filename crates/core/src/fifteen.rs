//! Fifteen's Bet: half of all scrambles are lies. Learn to smell which half.
//!
//! The classic 4x4 sliding puzzle has a secret: exactly half of the ways you
//! could scatter its tiles can never be solved, and one invisible quantity,
//! parity, decides which. The game deals scrambles and you bet: solvable or
//! stuck forever. See `docs/PLAYFUL.md`.

use crate::rng::SplitMix64;

/// Decorrelates fifteen seeds from other seeded systems.
const TILE_MIX: u64 = 0x0F1F_7EE0_0000_0003;

/// A scramble: the 16 cells row-major, 0 is the hole.
pub type Scramble = [u8; 16];

/// Deal scramble `n` from `seed`: a uniform random permutation, so about
/// half of all deals are honestly unsolvable.
#[must_use]
pub fn deal(seed: u64, n: u64) -> Scramble {
    let mut rng = SplitMix64::new(seed ^ TILE_MIX ^ n.wrapping_mul(0x9E37_79B9));
    let mut tiles: Scramble = core::array::from_fn(|i| i as u8);
    for i in (1..16).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        tiles.swap(i, j);
    }
    tiles
}

/// The truth: a 4x4 scramble is solvable exactly when the permutation's
/// inversion count plus the hole's row from the top is odd... stated for
/// the standard goal (1..15 with the hole last). This is the invariant no
/// amount of sliding can change.
#[must_use]
pub fn solvable(tiles: &Scramble) -> bool {
    let mut inversions = 0usize;
    let flat: Vec<u8> = tiles.iter().copied().filter(|&t| t != 0).collect();
    for i in 0..flat.len() {
        for j in (i + 1)..flat.len() {
            if flat[i] > flat[j] {
                inversions += 1;
            }
        }
    }
    let hole_row_from_top = tiles.iter().position(|&t| t == 0).unwrap_or(0) / 4;
    // Standard 4x4 rule: solvable iff inversions + hole row (from top) is odd
    // ... with rows counted from the bottom it is the classic statement; from
    // the top on a 4-row board the parity flips to this form.
    (inversions + hole_row_from_top) % 2 == 1
}

/// Why, in one breath, for the reveal after each bet.
#[must_use]
pub fn why(tiles: &Scramble) -> String {
    let mut inversions = 0usize;
    let flat: Vec<u8> = tiles.iter().copied().filter(|&t| t != 0).collect();
    for i in 0..flat.len() {
        for j in (i + 1)..flat.len() {
            if flat[i] > flat[j] {
                inversions += 1;
            }
        }
    }
    let row = tiles.iter().position(|&t| t == 0).unwrap_or(0) / 4;
    format!(
        "{inversions} inversions + hole in row {} = {}, which is {}: {}",
        row + 1,
        inversions + row,
        if (inversions + row) % 2 == 1 {
            "odd"
        } else {
            "even"
        },
        if solvable(tiles) {
            "solvable"
        } else {
            "stuck forever"
        }
    )
}

/// The board as text, four rows, the hole as dots.
#[must_use]
pub fn board_text(tiles: &Scramble) -> String {
    let mut out = String::new();
    for row in 0..4 {
        for col in 0..4 {
            let t = tiles[row * 4 + col];
            if t == 0 {
                out.push_str("  ..");
            } else {
                out.push_str(&format!("  {t:>2}"));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Scramble, board_text, deal, solvable, why};

    #[test]
    fn the_solved_board_is_reachable_from_itself() {
        // 1..15 in order with the hole last: zero inversions, hole in row 4
        // (index 3): 0 + 3 = 3, odd, solvable. The goal must be solvable.
        let mut goal: Scramble = core::array::from_fn(|i| (i as u8 + 1) % 16);
        assert!(solvable(&goal));
        // One swap of two tiles flips the permutation parity: unsolvable.
        goal.swap(0, 1);
        assert!(
            !solvable(&goal),
            "the classic 14-15 swap is the classic lie"
        );
    }

    #[test]
    fn sliding_moves_preserve_the_invariant() {
        // A slide swaps the hole with an adjacent tile. Horizontal slides
        // change nothing in the formula; vertical slides change both terms
        // by odd amounts. Either way the parity is invariant.
        let start = deal(7, 0);
        let verdict = solvable(&start);
        let mut tiles = start;
        // Walk 50 random-ish legal slides and re-check every time.
        let mut hole = tiles.iter().position(|&t| t == 0).unwrap();
        for step in 0..50 {
            let (row, col) = (hole / 4, hole % 4);
            let mut options = Vec::new();
            if row > 0 {
                options.push(hole - 4);
            }
            if row < 3 {
                options.push(hole + 4);
            }
            if col > 0 {
                options.push(hole - 1);
            }
            if col < 3 {
                options.push(hole + 1);
            }
            let next = options[step % options.len()];
            tiles.swap(hole, next);
            hole = next;
            assert_eq!(
                solvable(&tiles),
                verdict,
                "no slide may ever change the answer"
            );
        }
    }

    #[test]
    fn deals_are_deterministic_and_both_kinds_occur() {
        assert_eq!(deal(3, 1), deal(3, 1));
        let verdicts: Vec<bool> = (0..24).map(|n| solvable(&deal(3, n))).collect();
        assert!(verdicts.iter().any(|&v| v), "some deals are honest");
        assert!(verdicts.iter().any(|&v| !v), "and some are lies");
    }

    #[test]
    fn the_board_prints_and_the_why_explains() {
        let tiles = deal(5, 0);
        let text = board_text(&tiles);
        assert_eq!(text.lines().count(), 4);
        assert!(text.contains(".."), "the hole shows");
        let explanation = why(&tiles);
        assert!(explanation.contains("inversions"));
        assert!(explanation.contains("odd") || explanation.contains("even"));
    }
}
