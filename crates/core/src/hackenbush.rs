//! Hackenbush: cut grass, and the grass turns out to be made of numbers.
//!
//! Red-blue stalks grow from a ground line. On your turn you cut one segment
//! of YOUR color; everything above the cut falls. When you have nothing left
//! to cut, you lose. The trapdoor: Conway proved every position is worth a
//! NUMBER, often a fraction like 3/4, and perfect play is just arithmetic.
//! The Order plays Blue by computing those numbers. Win, and it hands you
//! the whole idea. See `docs/PLAYFUL.md`.

use crate::rng::SplitMix64;

/// Decorrelates hackenbush seeds from other seeded systems.
const GRASS_MIX: u64 = 0x6EA5_5000_0000_0001;
/// Dyadic values are held in units of 1/65536 (heights stay small).
pub const UNIT: i64 = 65_536;

/// One segment of grass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// Yours.
    Red,
    /// The Order's.
    Blue,
}

/// A position: stalks of segments, each growing up from the ground.
pub type Stalks = Vec<Vec<Color>>;

/// The Berlekamp sign-expansion value of one stalk, in UNITs: the first run
/// of one color counts whole units; after the first color change each
/// further segment is worth half the previous, signed by its color.
#[must_use]
pub fn stalk_value(stalk: &[Color]) -> i64 {
    let mut value = 0i64;
    let mut unit = UNIT;
    let mut changed = false;
    let mut previous: Option<Color> = None;
    for &color in stalk {
        let sign = match color {
            Color::Red => 1,
            Color::Blue => -1,
        };
        if let Some(prev) = previous
            && (prev != color || changed)
        {
            changed = true;
            unit /= 2;
        }
        value += sign * unit;
        previous = Some(color);
    }
    value
}

/// The whole position's value: the sum of its stalks.
#[must_use]
pub fn value(stalks: &Stalks) -> i64 {
    stalks.iter().map(|s| stalk_value(s)).sum()
}

/// Cut segment `height` (0-based) of `stalk`: it and everything above fall.
/// False if there is no such segment or it is not `color`.
pub fn cut(stalks: &mut Stalks, stalk: usize, height: usize, color: Color) -> bool {
    let Some(s) = stalks.get_mut(stalk) else {
        return false;
    };
    if s.get(height) != Some(&color) {
        return false;
    }
    s.truncate(height);
    true
}

/// Whether `color` has any segment left to cut.
#[must_use]
pub fn can_move(stalks: &Stalks, color: Color) -> bool {
    stalks.iter().any(|s| s.contains(&color))
}

/// The Order's move: try every Blue cut and keep the one that leaves the
/// position most negative (Blue's arithmetic is its strategy, plainly).
#[must_use]
pub fn order_move(stalks: &Stalks) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), i64)> = None;
    for (i, s) in stalks.iter().enumerate() {
        for (h, &c) in s.iter().enumerate() {
            if c != Color::Blue {
                continue;
            }
            let mut trial = stalks.clone();
            trial[i].truncate(h);
            let v = value(&trial);
            if best.is_none_or(|(_, bv)| v < bv) {
                best = Some(((i, h), v));
            }
        }
    }
    best.map(|(m, _)| m)
}

/// A fresh seeded garden: three to five mixed stalks with total value
/// positive, so Red (you, moving first) can win with perfect play.
#[must_use]
pub fn new_garden(seed: u64) -> Stalks {
    let mut rng = SplitMix64::new(seed ^ GRASS_MIX);
    loop {
        let count = 3 + rng.below(3) as usize;
        let stalks: Stalks = (0..count)
            .map(|_| {
                (0..2 + rng.below(4))
                    .map(|_| {
                        if rng.below(2) == 0 {
                            Color::Red
                        } else {
                            Color::Blue
                        }
                    })
                    .collect()
            })
            .collect();
        let v = value(&stalks);
        if v > 0 && can_move(&stalks, Color::Red) && can_move(&stalks, Color::Blue) && v <= 2 * UNIT
        {
            return stalks;
        }
    }
}

/// The idea, handed over in full when it has been earned.
#[must_use]
pub fn the_secret() -> &'static str {
    "Every garden you just played was secretly a number. One red blade is worth \
     1. Red with blue on top is worth exactly 1/2, blue on that, 1/4. Add the \
     stalks up: if the total is positive, Red wins no matter who moves. Conway \
     followed this idea past every fraction into numbers no one had ever seen, \
     the surreal numbers, and they came out of children's games. You were doing \
     arithmetic with them the whole time."
}

#[cfg(test)]
mod tests {
    use super::{Color::*, UNIT, can_move, cut, new_garden, order_move, stalk_value, value};

    #[test]
    fn stalk_values_match_conway() {
        assert_eq!(stalk_value(&[Red]), UNIT, "one red blade is 1");
        assert_eq!(stalk_value(&[Red, Red]), 2 * UNIT, "two reds are 2");
        assert_eq!(stalk_value(&[Red, Blue]), UNIT / 2, "red-blue is 1/2");
        assert_eq!(stalk_value(&[Red, Blue, Blue]), UNIT / 4, "and 1/4");
        assert_eq!(stalk_value(&[Red, Blue, Red]), 3 * UNIT / 4, "and 3/4");
        assert_eq!(stalk_value(&[Blue, Red]), -UNIT / 2, "mirrored, negated");
        assert_eq!(stalk_value(&[]), 0);
    }

    #[test]
    fn positive_gardens_are_red_wins_under_the_order_vs_itself() {
        // Red plays the same greedy arithmetic as Blue; from value > 0 with
        // Red to move, Red must take the last cut. The theorem, exercised.
        for seed in 0..12 {
            let mut stalks = new_garden(seed);
            assert!(value(&stalks) > 0);
            let mut red_turn = true;
            loop {
                let color = if red_turn { Red } else { Blue };
                if !can_move(&stalks, color) {
                    break;
                }
                let mv = if red_turn {
                    // Red mirrors the Order: most positive result.
                    let mut best: Option<((usize, usize), i64)> = None;
                    for (i, s) in stalks.iter().enumerate() {
                        for (h, &c) in s.iter().enumerate() {
                            if c != Red {
                                continue;
                            }
                            let mut trial = stalks.clone();
                            trial[i].truncate(h);
                            let v = value(&trial);
                            if best.is_none_or(|(_, bv)| v > bv) {
                                best = Some(((i, h), v));
                            }
                        }
                    }
                    best.map(|(m, _)| m)
                } else {
                    order_move(&stalks)
                };
                let (i, h) = mv.expect("a legal move exists");
                assert!(cut(&mut stalks, i, h, color));
                red_turn = !red_turn;
            }
            assert!(!red_turn, "Blue ran out first: Red took the last cut");
        }
    }

    #[test]
    fn cuts_drop_everything_above_and_refuse_wrong_colors() {
        let mut stalks = vec![vec![Red, Blue, Red]];
        assert!(!cut(&mut stalks, 0, 1, Red), "that segment is blue");
        assert!(!cut(&mut stalks, 0, 9, Red), "no such height");
        assert!(!cut(&mut stalks, 9, 0, Red), "no such stalk");
        assert!(cut(&mut stalks, 0, 1, Blue));
        assert_eq!(stalks[0], vec![Red], "everything above fell");
    }

    #[test]
    fn the_secret_hands_over_the_surreals() {
        assert!(super::the_secret().contains("surreal"));
        assert!(super::the_secret().contains("1/2"));
    }
}
