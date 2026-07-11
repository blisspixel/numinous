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
    use super::{apply, finished, new_game, order_move, the_secret};

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
    fn the_secret_teaches_the_xor() {
        assert!(the_secret().contains("xor"));
        assert!(the_secret().contains("zero"));
    }
}
