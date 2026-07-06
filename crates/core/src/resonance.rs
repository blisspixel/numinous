//! Resonances: the synergy layer. When two things you have done start to rhyme,
//! a link lights up and hands you the line that connects them.
//!
//! This is the RPG genre's "evolution" mechanic in our key: the evidence is
//! your own play (rooms entered, games on the table), the reward is the
//! connection itself, knowledge that only makes sense because you did both
//! halves. Computed purely from the record; impossible to hold unearned.

use crate::journey::Journey;
use crate::scores::Scoreboard;

/// One resonance: a link between things done, and the line it unlocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resonance {
    /// The link's name.
    pub name: &'static str,
    /// The connection itself, revealed only when active.
    pub lore: &'static str,
    /// Whether this player's record lights it.
    pub active: bool,
}

/// Whether the table has any entry for a game prefix.
fn played(board: &Scoreboard, prefix: &str) -> bool {
    board.entries.keys().any(|key| key.starts_with(prefix))
}

/// Compute the resonances from the record.
#[must_use]
pub fn resonances(journey: &Journey, board: &Scoreboard) -> Vec<Resonance> {
    let visited = |id: &str| journey.visited.contains(id);
    vec![
        Resonance {
            name: "The Sieve",
            lore: "The spiral you walked and the numbers you ate are the same list: \
                   Eratosthenes just wrote down what Munch serves.",
            active: visited("prime-spirals") && played(board, "munch"),
        },
        Resonance {
            name: "Sensitive Dependence",
            lore: "The bifurcation cascade and the butterfly are one storm: Feigenbaum's \
                   constant paces the road that ends in Lorenz's weather.",
            active: visited("logistic-map") && visited("lorenz"),
        },
        Resonance {
            name: "The Atlas",
            lore: "Every point of the set you zoomed is a whole Julia world folded shut. \
                   One room is the atlas; the other is its pages.",
            active: visited("mandelbrot") && visited("julia"),
        },
        Resonance {
            name: "First Contact",
            lore: "You found the signal in the static and read the picture in the bits: \
                   detection and decoding are the two halves of hello.",
            active: visited("arecibo") && played(board, "seti"),
        },
        Resonance {
            name: "The Chord Made Visible",
            lore: "The figure on the screen and the pendulum's drawing agree: every \
                   interval you can hear is a shape you can trace.",
            active: visited("lissajous") && visited("harmonograph"),
        },
        Resonance {
            name: "Rate and Total",
            lore: "The tilt you rode is the pour's own rising line, read backward. \
                   Differentiating and integrating are one door, entered from both sides.",
            active: visited("slope-rider") && visited("the-pour"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::resonances;
    use crate::journey::Journey;
    use crate::scores::Scoreboard;

    #[test]
    fn a_fresh_record_lights_nothing() {
        let all = resonances(&Journey::default(), &Scoreboard::default());
        assert!(all.len() >= 6);
        assert!(all.iter().all(|r| !r.active));
        assert!(all.iter().all(|r| !r.name.is_empty() && r.lore.len() > 40));
    }

    #[test]
    fn both_halves_are_required() {
        let mut journey = Journey::default();
        journey.visit("mandelbrot");
        let board = Scoreboard::default();
        let one_half = resonances(&journey, &board);
        assert!(
            !one_half
                .iter()
                .find(|r| r.name == "The Atlas")
                .unwrap()
                .active
        );
        journey.visit("julia");
        let both = resonances(&journey, &board);
        assert!(both.iter().find(|r| r.name == "The Atlas").unwrap().active);
    }

    #[test]
    fn game_evidence_counts_too() {
        let mut journey = Journey::default();
        journey.visit("prime-spirals");
        let mut board = Scoreboard::default();
        assert!(!resonances(&journey, &board)[0].active);
        board.record("munch seed:7 board:0", 40);
        assert!(resonances(&journey, &board)[0].active, "the sieve lights");
    }
}
