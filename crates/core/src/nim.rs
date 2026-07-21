//! Nim: lose to the Order, then be handed the secret and become unbeatable.
//!
//! Three heaps; take any amount from one heap; whoever takes the last stone
//! wins. The Order plays perfectly, which is possible because of one strange
//! fact: xor the heap sizes together, and the winning move is whatever leaves
//! that xor at zero. The game exists to transfer that power to you (see
//! `docs/ROOMS.md`, the Full Map). Deterministic from a seed.

use crate::rng::SplitMix64;

/// Decorrelates nim seeds from other seeded systems.
const NIM_MIX: u64 = 0x0000_1D1E_0000_D00D;

/// Maximum player turns accepted in one serialized stateless replay.
///
/// A legal three-heap game ends well before this cap because the opening board
/// contains at most 21 stones. The extra headroom preserves compatibility while
/// bounding padded histories before replay.
pub const MAX_REPLAY_TURNS: usize = 64;

/// One zero-based heap removal in a deterministic Nim replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NimTurn {
    /// Zero-based heap index.
    pub heap: usize,
    /// Positive number of stones removed.
    pub take: u32,
}

/// The side that took the final stone in a completed replay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NimWinner {
    /// The player supplied the final move.
    Player,
    /// The deterministic Order reply supplied the final move.
    Order,
}

/// Complete public state produced by replaying one player move history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NimReplay {
    /// Heap sizes after every accepted player move and deterministic reply.
    pub heaps: Vec<u32>,
    /// Deterministic Order replies made before the replay ended.
    pub order: Vec<NimTurn>,
    /// Winner when the board is empty, otherwise `None`.
    pub winner: Option<NimWinner>,
}

/// A replay that cannot follow the supplied or internally selected move.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NimReplayError {
    /// The player attempted an illegal removal from the current heaps.
    IllegalPlayerMove {
        /// Rejected zero-based move.
        turn: NimTurn,
        /// Heap state immediately before the rejected move.
        heaps: Vec<u32>,
    },
    /// The Order reducer produced no legal move from a nonempty board.
    InvalidOrderMove {
        /// Rejected zero-based Order move.
        turn: NimTurn,
        /// Heap state immediately before the rejected move.
        heaps: Vec<u32>,
    },
}

/// A fresh game's heaps: three piles, never a lost cause for the first mover.
#[must_use]
pub fn new_game(seed: u64) -> Vec<u32> {
    let mut rng = SplitMix64::new(seed ^ NIM_MIX);
    loop {
        let heaps: Vec<u32> = (0..3).map(|_| 1 + rng.below(7) as u32).collect();
        // Starting xor of zero means the first mover loses to perfect play;
        // the Order offers only winnable boards. Fair fights, always.
        if heaps.iter().fold(0, |x, &h| x ^ h) != 0 {
            return heaps;
        }
    }
}

/// Apply a move: take `amount` from `heap` (0-based). False if illegal.
pub fn apply(heaps: &mut [u32], heap: usize, amount: u32) -> bool {
    if heap >= heaps.len() || amount == 0 || amount > heaps[heap] {
        return false;
    }
    heaps[heap] -= amount;
    true
}

/// The board is empty: whoever just moved took the last stone and won.
#[must_use]
pub fn finished(heaps: &[u32]) -> bool {
    heaps.iter().all(|&h| h == 0)
}

/// The Order's move: perfect play. If the xor is nonzero, move to zero it;
/// otherwise (a lost position) take one stone from the largest heap and wait
/// for a mistake.
#[must_use]
pub fn order_move(heaps: &[u32]) -> (usize, u32) {
    let x = heaps.iter().fold(0u32, |acc, &h| acc ^ h);
    if x != 0 {
        for (i, &h) in heaps.iter().enumerate() {
            let target = h ^ x;
            if target < h {
                return (i, h - target);
            }
        }
    }
    // A lost position: take one from the largest heap and wait for a mistake.
    // If there is no legal move at all (an empty or all-zero board, which every
    // caller gates out with `finished` first), pass safely rather than panic on
    // the empty case or return an illegal take from a zero heap.
    match heaps.iter().enumerate().max_by_key(|&(_, &h)| h) {
        Some((i, &h)) if h > 0 => (i, 1),
        _ => (0, 0),
    }
}

/// Replays a player's complete move history and every deterministic Order reply.
///
/// Reaching either winner ends the replay immediately, matching the faces that
/// treat a completed game as terminal even if a caller supplied trailing moves.
pub fn replay(seed: u64, player_turns: &[NimTurn]) -> Result<NimReplay, NimReplayError> {
    let mut heaps = new_game(seed);
    let mut order = Vec::new();
    for &turn in player_turns {
        if !apply(&mut heaps, turn.heap, turn.take) {
            return Err(NimReplayError::IllegalPlayerMove { turn, heaps });
        }
        if finished(&heaps) {
            return Ok(NimReplay {
                heaps,
                order,
                winner: Some(NimWinner::Player),
            });
        }

        let (heap, take) = order_move(&heaps);
        let turn = NimTurn { heap, take };
        if !apply(&mut heaps, turn.heap, turn.take) {
            return Err(NimReplayError::InvalidOrderMove { turn, heaps });
        }
        order.push(turn);
        if finished(&heaps) {
            return Ok(NimReplay {
                heaps,
                order,
                winner: Some(NimWinner::Order),
            });
        }
    }
    Ok(NimReplay {
        heaps,
        order,
        winner: None,
    })
}

/// The secret, handed over in full when it has been earned.
#[must_use]
pub fn the_secret() -> &'static str {
    "Write each heap in binary and xor them together. If the result is zero, \
     the position is lost: every move you make hands the winner back. If it is \
     not zero, there is always exactly one kind of move that makes it zero, and \
     that move is yours. That is the whole game. It was never about the stones; \
     it was about seeing the second number hiding inside the first. You are now \
     unbeatable. Use it kindly."
}

#[cfg(test)]
mod tests {
    use super::{
        NimReplayError, NimTurn, NimWinner, apply, finished, new_game, order_move, replay,
        the_secret,
    };

    fn xor(heaps: &[u32]) -> u32 {
        heaps.iter().fold(0, |x, &h| x ^ h)
    }

    #[test]
    fn order_move_is_total_on_degenerate_boards() {
        // A board with no legal move (empty, or all zero) must not panic and must
        // not return an illegal take from a zero heap; it passes with take 0.
        assert_eq!(order_move(&[]), (0, 0));
        assert_eq!(order_move(&[0, 0, 0]), (0, 0));
        // A real lost position still takes one from the largest heap.
        assert_eq!(order_move(&[2, 2]).1, 1);
    }

    #[test]
    fn new_games_are_three_winnable_heaps() {
        for seed in 0..25 {
            let heaps = new_game(seed);
            assert_eq!(heaps.len(), 3);
            assert!(heaps.iter().all(|&h| (1..=7).contains(&h)));
            assert_ne!(xor(&heaps), 0, "the first mover must have a chance");
        }
    }

    #[test]
    fn the_order_zeroes_the_xor_whenever_it_can() {
        for seed in 0..25 {
            let mut heaps = new_game(seed);
            let (heap, take) = order_move(&heaps);
            assert!(apply(&mut heaps, heap, take), "the move is legal");
            assert_eq!(xor(&heaps), 0, "perfect play leaves xor zero");
        }
    }

    #[test]
    fn perfect_play_from_a_winning_position_always_wins() {
        // Strategy vs strategy: the side that moves first from xor != 0 wins.
        for seed in 0..15 {
            let mut heaps = new_game(seed);
            let mut mover = 0; // 0 moves first, from a nonzero xor
            loop {
                let (heap, take) = order_move(&heaps);
                assert!(apply(&mut heaps, heap, take));
                if finished(&heaps) {
                    break;
                }
                mover = 1 - mover;
            }
            assert_eq!(mover, 0, "the first mover took the last stone");
        }
    }

    #[test]
    fn illegal_moves_are_refused() {
        let mut heaps = vec![3, 0, 2];
        assert!(!apply(&mut heaps, 0, 0), "taking nothing");
        assert!(!apply(&mut heaps, 0, 4), "taking too much");
        assert!(!apply(&mut heaps, 1, 1), "taking from an empty heap");
        assert!(!apply(&mut heaps, 9, 1), "taking from nowhere");
        assert!(apply(&mut heaps, 2, 2));
        assert!(!finished(&heaps));
        assert!(apply(&mut heaps, 0, 3));
        assert!(finished(&heaps));
    }

    #[test]
    fn replay_owns_player_validation_order_replies_and_terminal_state() {
        let seed = 23;
        let opening = replay(seed, &[]).expect("opening replay");
        assert_eq!(opening.heaps, new_game(seed));
        assert!(opening.order.is_empty());
        assert_eq!(opening.winner, None);

        let turn = NimTurn { heap: 0, take: 1 };
        let continued = replay(seed, &[turn]).expect("legal replay");
        assert_eq!(continued.order.len(), 1);
        assert_eq!(continued.winner, None);

        let invalid = replay(seed, &[NimTurn { heap: 3, take: 1 }]);
        assert_eq!(
            invalid,
            Err(NimReplayError::IllegalPlayerMove {
                turn: NimTurn { heap: 3, take: 1 },
                heaps: new_game(seed),
            })
        );
    }

    #[test]
    fn replay_stops_when_a_perfect_player_takes_the_last_stone() {
        for seed in 0..15 {
            let mut heaps = new_game(seed);
            let mut turns = Vec::new();
            loop {
                let (heap, take) = order_move(&heaps);
                turns.push(NimTurn { heap, take });
                assert!(apply(&mut heaps, heap, take));
                if finished(&heaps) {
                    break;
                }
                let (heap, take) = order_move(&heaps);
                assert!(apply(&mut heaps, heap, take));
            }
            let replayed = replay(seed, &turns).expect("perfect history");
            assert_eq!(replayed.winner, Some(NimWinner::Player));
            assert!(replayed.heaps.iter().all(|heap| *heap == 0));
        }
    }

    #[test]
    fn the_secret_teaches_the_xor() {
        assert!(the_secret().contains("xor"));
        assert!(the_secret().contains("zero"));
    }
}
