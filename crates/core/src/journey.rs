//! The Journey: quiet, persistent progression through the Order.
//!
//! Play marks the record: rooms you have entered light stars in a constellation,
//! games you have won and secrets you have heard add to it. The record advances
//! you through ranks named for the real Pythagorean school (listeners outside
//! the curtain, learners within), at thresholds that are triangular numbers,
//! because of course they are. Rank never gates the base experience; it only
//! opens hidden layers (see `docs/LORE.md`, the guardrails). Everything here is
//! pure and deterministic; the faces own the file IO.

use std::collections::BTreeSet;

use crate::registry::all_rooms;
use crate::rng::SplitMix64;

/// The persistent record of one player's journey.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Journey {
    /// Room ids entered at least once.
    pub visited: BTreeSet<String>,
    /// Games won (any of them).
    pub wins: u32,
    /// Secrets heard (the whispers).
    pub secrets: u32,
}

/// Rank within the Order. Never explained in the product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    /// Outside the door.
    Outsider,
    /// A listener, behind the curtain (1 spark).
    Akousmatikos,
    /// A learner, within (10 sparks: the tetractys).
    Mathematikos,
    /// A theorist of the monochord (28 sparks).
    Kanonikos,
    /// The ten itself (55 sparks).
    Dekas,
}

impl Rank {
    /// The rank's name, as spoken inside the Order.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Rank::Outsider => "Outsider",
            Rank::Akousmatikos => "Akousmatikos",
            Rank::Mathematikos => "Mathematikos",
            Rank::Kanonikos => "Kanonikos",
            Rank::Dekas => "Dekas",
        }
    }

    /// What the Order says when you arrive at this rank. Deadpan, unexplained.
    #[must_use]
    pub fn whisper(self) -> &'static str {
        match self {
            Rank::Outsider => "",
            Rank::Akousmatikos => "You are behind the curtain now. Listen.",
            Rank::Mathematikos => "Five years of silence end. You may ask why.",
            Rank::Kanonikos => "The monochord is yours. Every room is a string.",
            Rank::Dekas => "One, two, three, four. You carried them all.",
        }
    }
}

impl Journey {
    /// The journey's sparks: how much of the record has accumulated.
    /// Visits count once each; wins and secrets weigh more.
    #[must_use]
    pub fn sparks(&self) -> u32 {
        self.visited.len() as u32 + 2 * self.wins + 5 * self.secrets
    }

    /// The rank these sparks confer. Thresholds are triangular numbers.
    #[must_use]
    pub fn rank(&self) -> Rank {
        match self.sparks() {
            0 => Rank::Outsider,
            1..=9 => Rank::Akousmatikos,
            10..=27 => Rank::Mathematikos,
            28..=54 => Rank::Kanonikos,
            _ => Rank::Dekas,
        }
    }

    /// Mark a room entered. Returns true if this is new to the record.
    pub fn visit(&mut self, room_id: &str) -> bool {
        self.visited.insert(room_id.to_string())
    }

    /// Record a game won.
    pub fn win(&mut self) {
        self.wins += 1;
    }

    /// Record a secret heard.
    pub fn secret(&mut self) {
        self.secrets += 1;
    }

    /// Serialize to the journey file format (plain lines, stable order).
    #[must_use]
    pub fn to_text(&self) -> String {
        let visited: Vec<&str> = self.visited.iter().map(String::as_str).collect();
        format!(
            "visited {}\nwins {}\nsecrets {}\n",
            visited.join(" "),
            self.wins,
            self.secrets
        )
    }

    /// Parse the journey file format. Unknown lines are ignored (forward
    /// compatible); a missing or empty file is a fresh journey.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let mut journey = Journey::default();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("visited") => {
                    journey.visited = parts.map(str::to_string).collect();
                }
                Some("wins") => {
                    journey.wins = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                }
                Some("secrets") => {
                    journey.secrets = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                }
                _ => {}
            }
        }
        journey
    }
}

/// Render the constellation: one star per room, lit if you have been there.
/// Star positions are deterministic (hashed from the room id), so the sky is
/// the same for everyone; only your light differs.
#[must_use]
pub fn constellation(journey: &Journey, width: usize, height: usize) -> String {
    let mut grid = vec![vec![' '; width.max(1)]; height.max(1)];
    for room in all_rooms() {
        let id = room.meta().id;
        // Hash the id into a stable position.
        let mut seed = 0xC057_E11A_7101_u64;
        for byte in id.bytes() {
            seed = seed.wrapping_mul(31).wrapping_add(u64::from(byte));
        }
        let mut rng = SplitMix64::new(seed);
        let x = (rng.below(width.max(1) as u64)) as usize;
        let y = (rng.below(height.max(1) as u64)) as usize;
        grid[y][x] = if journey.visited.contains(id) {
            '#'
        } else {
            '.'
        };
    }
    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{Journey, Rank, constellation};
    use crate::registry::all_rooms;

    #[test]
    fn text_round_trips() {
        let mut journey = Journey::default();
        journey.visit("mandelbrot");
        journey.visit("lorenz");
        journey.win();
        journey.secret();
        let back = Journey::from_text(&journey.to_text());
        assert_eq!(back, journey);
    }

    #[test]
    fn a_fresh_journey_parses_from_anything() {
        assert_eq!(Journey::from_text(""), Journey::default());
        assert_eq!(Journey::from_text("garbage\nnoise 9"), Journey::default());
    }

    #[test]
    fn ranks_rise_at_triangular_thresholds() {
        let mut journey = Journey::default();
        assert_eq!(journey.rank(), Rank::Outsider);
        journey.visit("a");
        assert_eq!(journey.rank(), Rank::Akousmatikos); // 1 spark
        journey.wins = 5;
        assert_eq!(journey.sparks(), 11);
        assert_eq!(journey.rank(), Rank::Mathematikos); // past 10
        journey.secrets = 4;
        assert_eq!(journey.sparks(), 31);
        assert_eq!(journey.rank(), Rank::Kanonikos); // past 28
        journey.wins = 30;
        assert!(journey.sparks() >= 55);
        assert_eq!(journey.rank(), Rank::Dekas);
    }

    #[test]
    fn visits_count_once() {
        let mut journey = Journey::default();
        assert!(journey.visit("lorenz"));
        assert!(!journey.visit("lorenz"));
        assert_eq!(journey.sparks(), 1);
    }

    #[test]
    fn every_rank_has_a_name_and_late_ranks_whisper() {
        for rank in [
            Rank::Outsider,
            Rank::Akousmatikos,
            Rank::Mathematikos,
            Rank::Kanonikos,
            Rank::Dekas,
        ] {
            assert!(!rank.name().is_empty());
        }
        assert!(Rank::Outsider.whisper().is_empty());
        assert!(Rank::Dekas.whisper().contains("carried"));
    }

    #[test]
    fn the_constellation_shows_every_room_and_lights_visits() {
        let mut journey = Journey::default();
        let sky = constellation(&journey, 60, 20);
        let dim = sky.matches('.').count() + sky.matches('#').count();
        assert!(
            dim >= all_rooms().len() - 2,
            "nearly all stars placed (a couple may collide): {dim}"
        );
        journey.visit("mandelbrot");
        let lit = constellation(&journey, 60, 20);
        assert!(lit.contains('#'), "a visited room is a lit star");
        assert_eq!(lit, constellation(&journey, 60, 20), "the sky is stable");
    }
}
