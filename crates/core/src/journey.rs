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
    /// Rounds played, sims run, curves plotted: showing up counts.
    pub plays: u32,
    /// Boons redeemed: early unlocks chosen at level-ups (for example
    /// `cut:mandelbrot:0`). Choices shape the order; levels still open all.
    pub chosen: BTreeSet<String>,
    /// Consecutive daily-challenge days, current run.
    pub streak: u32,
    /// The last daily day played (days since the epoch), 0 for never.
    pub last_daily: u64,
}

/// The level cap. The answer to how far you can go.
pub const MAX_LEVEL: u32 = 42;

/// The unlocks: level, what opens, and how it reads on the wall.
/// These gate extras and harder modes, never the base experience.
pub const UNLOCKS: &[(u32, &str, &str)] = &[
    (3, "quiz --hard", "six shapes to tell apart instead of four"),
    (5, "crack --digits 5+", "longer bomb codes"),
    (7, "seti --channels 5+", "a wider sky to scan"),
    (42, "answer", "the answer"),
];

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
    /// The journey's sparks: how much of the record has accumulated. Showing up
    /// (visits and plays) counts as much as anything: being right earns a little
    /// more, but the road to the cap is paved with playing, which is why anyone
    /// who keeps playing gets there.
    #[must_use]
    pub fn sparks(&self) -> u32 {
        self.visited.len() as u32 + self.plays + 2 * self.wins + 5 * self.secrets
    }

    /// The level these sparks confer: 1 through [`MAX_LEVEL`]. Level `n` needs
    /// the triangular number T(n-1) sparks, so early levels come fast and the
    /// cap is a real road.
    #[must_use]
    pub fn level(&self) -> u32 {
        let sparks = self.sparks();
        let mut level = 1;
        while level < MAX_LEVEL && sparks >= triangular(level) {
            level += 1;
        }
        level
    }

    /// The 8-bit progress bar toward the next level, `width` cells wide.
    /// At the cap the bar is simply full.
    #[must_use]
    pub fn level_bar(&self, width: usize) -> String {
        let level = self.level();
        if level >= MAX_LEVEL {
            return "#".repeat(width);
        }
        let floor = triangular(level - 1);
        let ceiling = triangular(level);
        let into = self.sparks().saturating_sub(floor);
        let span = (ceiling - floor).max(1);
        let filled = (into as usize * width) / span as usize;
        format!("{}{}", "#".repeat(filled), "-".repeat(width - filled))
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

    /// Record a round played, a sim run, or a curve made. Showing up counts.
    pub fn play(&mut self) {
        self.plays += 1;
    }

    /// Boons waiting to be chosen: every level past the first banks one, and
    /// each redemption spends one. Never expires; never nags.
    #[must_use]
    pub fn boons_available(&self) -> u32 {
        (self.level() - 1).saturating_sub(self.chosen.len() as u32)
    }

    /// Record playing today's daily (a day count since the epoch). Consecutive
    /// days grow the chain; a gap quietly starts a new one; the same day twice
    /// changes nothing. Returns the streak if it moved. No scolding, ever.
    pub fn record_daily(&mut self, day: u64) -> Option<u32> {
        if day == self.last_daily {
            return None;
        }
        self.streak = if day == self.last_daily + 1 && self.last_daily != 0 {
            self.streak + 1
        } else {
            1
        };
        self.last_daily = day;
        Some(self.streak)
    }

    /// Serialize to the journey file format (plain lines, stable order).
    #[must_use]
    pub fn to_text(&self) -> String {
        let visited: Vec<&str> = self.visited.iter().map(String::as_str).collect();
        let chosen: Vec<&str> = self.chosen.iter().map(String::as_str).collect();
        format!(
            "visited {}\nwins {}\nsecrets {}\nplays {}\nchosen {}\nstreak {} {}\n",
            visited.join(" "),
            self.wins,
            self.secrets,
            self.plays,
            chosen.join(" "),
            self.streak,
            self.last_daily
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
                Some("plays") => {
                    journey.plays = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                }
                Some("chosen") => {
                    journey.chosen = parts.map(str::to_string).collect();
                }
                Some("streak") => {
                    journey.streak = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    journey.last_daily = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                }
                _ => {}
            }
        }
        journey
    }
}

/// A boon on offer: an early unlock the player may choose at a level-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boon {
    /// The redemption id stored in the journey (for example `cut:lorenz:0`).
    pub id: String,
    /// How the offer reads on the menu.
    pub label: String,
}

/// The levels at which a room's deep cuts unlock by level alone.
/// (Kept in sync with the faces; boons open them early, levels open them all.)
pub const CUT_LEVELS: [u32; 2] = [5, 12];

/// Up to three boons on offer: rooms whose next deep cut is still locked at
/// this level and not already chosen. Deterministic for a given journey, and
/// reshuffled per redemption, so the menu is stable until you choose.
#[must_use]
pub fn boon_options(journey: &Journey) -> Vec<Boon> {
    let level = journey.level();
    let mut candidates = Vec::new();
    for room in all_rooms() {
        let id = room.meta().id;
        for (i, _) in room.deep_cuts().iter().enumerate() {
            let by_level = CUT_LEVELS.get(i).copied().unwrap_or(u32::MAX);
            let token = format!("cut:{id}:{i}");
            if level < by_level && !journey.chosen.contains(&token) {
                let depth = if i == 0 {
                    "a deeper cut"
                } else {
                    "a deeper cut still"
                };
                candidates.push(Boon {
                    id: token,
                    label: format!("{}: {depth}, ahead of LV {by_level}", room.meta().title),
                });
                break; // only the next cut per room is on offer
            }
        }
    }
    // Deterministic 3-of-n, reshuffled by how many boons have been redeemed.
    let mut rng = SplitMix64::new(0xB007 ^ journey.chosen.len() as u64);
    let mut picks = Vec::new();
    while picks.len() < 3 && !candidates.is_empty() {
        let index = (rng.below(candidates.len() as u64)) as usize;
        picks.push(candidates.swap_remove(index));
    }
    picks
}

/// The lore of a level: one true, deadpan line for every level on the road.
/// Unironic and funny are the same thing here.
#[must_use]
pub fn level_lore(level: u32) -> &'static str {
    match level {
        1 => "Neither prime nor composite. Everyone starts as an edge case.",
        2 => "The only even prime. The odd one out is even.",
        3 => "The first odd prime, and already a triangle.",
        4 => "2 plus 2, 2 times 2, 2 to the 2: the number that cannot tell its operations apart.",
        5 => "There are exactly five Platonic solids. You have caught up with Euclid's shelf.",
        6 => "A perfect number: 6 equals the sum of everything that divides it. Few are. Savor it.",
        7 => "Humanity's favorite number in every survey ever run. Six, seven. You know.",
        8 => {
            "8 and 9 are the only consecutive proper powers there are. Proven in 2002. Edge of history."
        }
        9 => {
            "Sum the digits of any multiple of 9 and you get a multiple of 9. Accountants call it casting out nines."
        }
        10 => "One and two and three and four. You know this one, or you will.",
        11 => "The first repunit prime: all ones, and indivisible.",
        12 => {
            "More divisors than any number before it. This is why the dozen survived the decimal."
        }
        13 => {
            "Fear of this level has a medical name. It is also a Fibonacci prime. Superstition is not."
        }
        14 => "The fourth Catalan number: exactly 14 ways to cut a hexagon into triangles.",
        15 => "Every row, column, and diagonal of the 3 by 3 magic square: 15.",
        16 => "2 to the 4 equals 4 to the 2, and that never happens again with distinct numbers.",
        17 => {
            "Gauss proved the 17-gon constructible at nineteen and wanted one on his tombstone. The mason refused."
        }
        18 => "Twice the sum of its own digits. No other positive number pulls this off.",
        19 => "A centered hexagon: one bee, six around it, twelve around them.",
        20 => "God's number: any Rubik's cube position solves in 20 moves or fewer.",
        21 => "Blackjack, and the sixth triangle. The house still has no idea.",
        22 => "22 over 7 is closer to pi than 3.14 is. The fraction beat the decimal.",
        23 => "Room of 23 people: more likely than not, two share a birthday. Check your party.",
        24 => {
            "Stack 24 squared cannonballs and you get a perfect square pyramid: 70 squared. Works for no other pile."
        }
        25 => "The smallest square that is a sum of two squares. Pythagoras sends regards.",
        26 => "The only number sitting between a square and a cube. Fermat proved its loneliness.",
        27 => "The Collatz orbit of 27 takes 111 steps to reach 1. You took fewer to get here.",
        28 => "Perfect again: 1 plus 2 plus 4 plus 7 plus 14. The Order noticed too.",
        29 => "The smallest prime that is three consecutive squares added: 4 plus 9 plus 16.",
        30 => {
            "The largest number whose smaller coprimes are all prime. After 30, trust breaks down."
        }
        31 => "A Mersenne prime: 2 to the 5, minus 1. The lighthouse pattern of the primes.",
        32 => "2 to the 5. Five doublings from one. Feel the exponent.",
        33 => "The largest number that is not a sum of distinct triangular numbers. It refused.",
        34 => "The magic constant of the 4 by 4 square, the one Durer carved in 1514.",
        35 => "There are exactly 35 hexominoes. People have checked. Repeatedly.",
        36 => "A square and a triangle at once. Both guilds claim it.",
        37 => "Ask a crowd for a random two-digit number and 37 wins. Randomness has a favorite.",
        38 => "The one and only magic hexagon sums every line to 38. There is no other.",
        39 => "The first uninteresting number, which is of course interesting. This is a proof.",
        40 => {
            "In English, forty is the only number spelled in alphabetical order. The alphabet approves."
        }
        41 => {
            "Euler's n squared plus n plus 41 makes primes for forty straight levels and breaks at the next one. Hold on."
        }
        42 => "You know what to type.",
        _ => "",
    }
}

/// The `n`-th triangular number: 1, 3, 6, 10, ...
fn triangular(n: u32) -> u32 {
    n * (n + 1) / 2
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
    fn levels_run_one_to_forty_two_on_triangular_xp() {
        let mut journey = Journey::default();
        assert_eq!(journey.level(), 1);
        journey.plays = 1;
        assert_eq!(journey.level(), 2); // T(1) = 1
        journey.plays = 3;
        assert_eq!(journey.level(), 3); // T(2) = 3
        journey.plays = 860;
        assert_eq!(journey.level(), 41); // one shy of T(41) = 861
        journey.plays = 861;
        assert_eq!(journey.level(), 42);
        journey.plays = 5_000;
        assert_eq!(journey.level(), 42, "the cap is the cap");
    }

    #[test]
    fn anyone_who_plays_levels_up_even_losing_every_round() {
        // No wins, no secrets: pure participation still reaches the cap.
        let journey = Journey {
            plays: 861,
            ..Journey::default()
        };
        assert_eq!(journey.level(), super::MAX_LEVEL);
    }

    #[test]
    fn the_level_bar_fills_and_caps() {
        let mut journey = Journey::default();
        assert_eq!(journey.level_bar(10), "----------");
        journey.plays = 2; // level 2 (T1=1), one spark into a span of 2
        let bar = journey.level_bar(10);
        assert!(bar.starts_with('#') && bar.contains('-'), "got {bar}");
        journey.plays = 2_000;
        assert_eq!(journey.level_bar(10), "##########");
    }

    #[test]
    fn unlocks_are_level_ordered_and_capped() {
        let mut previous = 0;
        for &(level, name, _) in super::UNLOCKS {
            assert!(level > previous, "unlocks must be sorted");
            assert!(level <= super::MAX_LEVEL);
            assert!(!name.is_empty());
            previous = level;
        }
    }

    #[test]
    fn boons_bank_per_level_and_spend_per_choice() {
        let mut journey = Journey {
            plays: 3, // level 3: two levels past the first
            ..Default::default()
        };
        assert_eq!(journey.boons_available(), 2);
        journey.chosen.insert("cut:lorenz:0".to_string());
        assert_eq!(journey.boons_available(), 1);
        // Round-trips through the file format.
        let back = Journey::from_text(&journey.to_text());
        assert_eq!(back, journey);
    }

    #[test]
    fn boon_options_offer_three_locked_cuts_and_respect_choices() {
        let journey = Journey {
            plays: 3,
            ..Default::default()
        };
        let options = super::boon_options(&journey);
        assert_eq!(options.len(), 3);
        for boon in &options {
            assert!(boon.id.starts_with("cut:"));
            assert!(boon.label.contains("ahead of LV"));
        }
        // Deterministic until a choice is made.
        assert_eq!(options, super::boon_options(&journey));
        // A choice removes that option from future menus.
        let mut chosen = journey.clone();
        chosen.chosen.insert(options[0].id.clone());
        assert!(
            super::boon_options(&chosen)
                .iter()
                .all(|b| b.id != options[0].id)
        );
    }

    #[test]
    fn daily_streaks_chain_gap_and_idempotence() {
        let mut journey = Journey::default();
        assert_eq!(
            journey.record_daily(100),
            Some(1),
            "first daily starts at 1"
        );
        assert_eq!(journey.record_daily(100), None, "same day changes nothing");
        assert_eq!(journey.record_daily(101), Some(2), "consecutive days chain");
        assert_eq!(
            journey.record_daily(105),
            Some(1),
            "a gap starts fresh, quietly"
        );
        let back = Journey::from_text(&journey.to_text());
        assert_eq!(back, journey, "streak survives the file");
    }

    #[test]
    fn every_level_on_the_road_has_lore() {
        for level in 1..=super::MAX_LEVEL {
            assert!(
                !super::level_lore(level).is_empty(),
                "level {level} has no lore"
            );
        }
        assert_eq!(super::level_lore(0), "");
        assert_eq!(super::level_lore(43), "", "there is no level 43");
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
