//! The hands-off Show's director: which room comes next, and why.
//!
//! Catalog order puts neighbors from one wing side by side and parks on rooms
//! that barely move. The director instead scores every eligible room against
//! the one on stage, preferring a contrast of color and density, an energy
//! that follows a set-shaped arc, a tempo and key the ear can follow, and the
//! Front Hall. It then picks among the best few with a seeded draw.
//!
//! Everything is integer arithmetic over the derived features, and every
//! choice is a pure function of the seed and what came before, so a face can
//! replay any cue by replaying from the start. A cue is a performance, not a
//! visit: a face records no Journey visit for a room the Show only passes
//! through.

use super::features::{RoomFeatures, feature_table, fifths_distance};
use super::profiles::{DirectorProfile, PhaseWindow, director_profile};
use crate::{FRONT_HALL, SplitMix64, catalog_index, is_engineered_aha_room};

/// Cues a room must wait before it can return, at most half the eligible rooms.
const RECENCY: usize = 60;
/// Target energy for successive cues: a build, a peak, a breather, a lift.
const ENERGY_ARC: [u32; 8] = [2, 3, 4, 5, 3, 5, 4, 1];
/// How many of the best-scored rooms the seeded draw chooses among.
const FINALISTS: usize = 3;
/// Integer odds for the first, second, and third finalist.
const FINALIST_ODDS: [u64; FINALISTS] = [5, 3, 2];
/// Weight of the hue step from the room on stage.
const HUE_WEIGHT: i64 = 10;
/// Weight of the change in ink density from the room on stage.
const DENSITY_WEIGHT: i64 = 6;
/// Weight of fitting the energy arc's target for this cue.
const ENERGY_WEIGHT: i64 = 8;
/// Weight of a tempo the ear can follow from the room on stage.
const TEMPO_WEIGHT: i64 = 7;
/// Weight of a key close on the circle of fifths.
const KEY_WEIGHT: i64 = 5;
/// Weight of membership in the Front Hall: the weighted playlist.
const FRONT_HALL_WEIGHT: i64 = 16;
/// Penalty for repeating the wing of the room two cues back.
const RECENT_WING_PENALTY: i64 = 20;
/// Hue step at which contrast stops earning more credit, in degrees.
const HUE_SATURATION_DEGREES: u16 = 120;
/// Term value, out of 100, credited when one side of a comparison is unknown.
const UNKNOWN_TERM: i64 = 50;

/// One cue of the directed Show: a room, its variation, and how to perform it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectorCue {
    position: usize,
    profile: DirectorProfile,
    catalog_index: usize,
    variation: u64,
}

impl DirectorCue {
    /// Zero-based position after the opening room.
    #[must_use]
    pub const fn position(self) -> usize {
        self.position
    }

    /// Canonical catalog room identifier.
    #[must_use]
    pub const fn room_id(self) -> &'static str {
        self.profile.room_id()
    }

    /// Index of the room in the catalog.
    #[must_use]
    pub const fn catalog_index(self) -> usize {
        self.catalog_index
    }

    /// Room variation for this cue. Staged rooms always keep the canonical
    /// variation, the one their spoiler-safe window is verified against.
    #[must_use]
    pub const fn variation(self) -> u64 {
        self.variation
    }

    /// The span of phase to perform, eased with [`PhaseWindow::phase_at`].
    #[must_use]
    pub const fn window(self) -> PhaseWindow {
        self.profile.window()
    }

    /// The phase to hold under reduced motion.
    #[must_use]
    pub fn still(self) -> f64 {
        self.profile.still()
    }
}

/// A replayable, contrast-aware sequence of Show cues.
///
/// Iterating yields cues in order. Two directors with the same seed and
/// opening yield the same cues, so a stateless face can reach position `n`
/// with `nth(n)`. The App passes a day number to give every player the same
/// Show that day; the core never reads a clock.
#[derive(Debug, Clone)]
pub struct ShowDirector {
    seed: u64,
    generator: SplitMix64,
    history: Vec<usize>,
    position: usize,
}

impl ShowDirector {
    /// A director that follows the Overture's hand-off, the threshold room.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::opening_at(super::OVERTURE.handoff().room_id(), seed)
            .expect("the overture hands off to a catalog room")
    }

    /// A director whose first cue follows `room_id`, or `None` when the id
    /// is not a listed room.
    #[must_use]
    pub fn opening_at(room_id: &str, seed: u64) -> Option<Self> {
        let opening = catalog_index(room_id)?;
        Some(Self {
            seed,
            generator: SplitMix64::new(seed),
            history: vec![opening],
            position: 0,
        })
    }

    fn choose(&mut self) -> Option<DirectorCue> {
        let table = feature_table();
        let on_stage = table.get(*self.history.last()?)?;
        let two_back = self
            .history
            .len()
            .checked_sub(2)
            .and_then(|index| table.get(self.history[index]))
            .map(|features| features.wing);
        let eligible = table.iter().filter(|room| !room.is_near_static()).count();
        let recent = &self.history[self.history.len().saturating_sub(RECENCY.min(eligible / 2))..];
        let rotation = (self.seed % ENERGY_ARC.len() as u64) as usize;
        let target = ENERGY_ARC[(self.position + rotation) % ENERGY_ARC.len()];
        let mut ranked = table
            .iter()
            .filter(|room| {
                !room.is_near_static()
                    && room.wing != on_stage.wing
                    && !recent.contains(&room.catalog_index)
            })
            .map(|room| (score(on_stage, room, target, two_back), room))
            .collect::<Vec<_>>();
        ranked.sort_by(|(left, a), (right, b)| {
            right.cmp(left).then(a.catalog_index.cmp(&b.catalog_index))
        });
        ranked.truncate(FINALISTS);
        let odds = &FINALIST_ODDS[..ranked.len()];
        let mut draw = self.generator.below(odds.iter().sum());
        let mut chosen = None;
        for ((_, room), &weight) in ranked.iter().zip(odds) {
            if draw < weight {
                chosen = Some(*room);
                break;
            }
            draw -= weight;
        }
        let chosen = chosen?;
        let staged = is_engineered_aha_room(chosen.room_id);
        let variation = if self.seed == 0 || staged {
            0
        } else {
            self.generator.next_u64()
        };
        Some(DirectorCue {
            position: self.position,
            profile: director_profile(chosen.room_id)?,
            catalog_index: chosen.catalog_index,
            variation,
        })
    }
}

impl Iterator for ShowDirector {
    type Item = DirectorCue;

    /// The next cue, or `None` only if no eligible room satisfies the
    /// director's constraints, which the catalog tests rule out.
    fn next(&mut self) -> Option<DirectorCue> {
        let cue = self.choose()?;
        // Only the recency window and the last two wings are ever read, so an
        // endless Show keeps a bounded history.
        if self.history.len() == RECENCY {
            self.history.remove(0);
        }
        self.history.push(cue.catalog_index);
        self.position += 1;
        Some(cue)
    }
}

/// The integer score of `next` following `on_stage` at an arc `target`.
fn score(
    on_stage: &RoomFeatures,
    next: &RoomFeatures,
    target: u32,
    two_back_wing: Option<&str>,
) -> i64 {
    let hue = match (on_stage.hue, next.hue) {
        (Some(a), Some(b)) => {
            let step = a.abs_diff(b).min(360 - a.abs_diff(b));
            i64::from(step.min(HUE_SATURATION_DEGREES)) * 100 / i64::from(HUE_SATURATION_DEGREES)
        }
        _ => UNKNOWN_TERM,
    };
    let density = i64::from(on_stage.ink_level().abs_diff(next.ink_level())) * 25;
    let energy = 100 - i64::from(next.energy().abs_diff(target)) * 25;
    let tempo = match (on_stage.tempo, next.tempo) {
        (Some(a), Some(b)) if tempos_follow(a, b) => 100,
        (Some(_), Some(_)) => 0,
        _ => UNKNOWN_TERM,
    };
    let key = match (on_stage.pitch_class, next.pitch_class) {
        (Some(a), Some(b)) => match fifths_distance(a, b) {
            0 | 1 => 100,
            2 => 50,
            _ => 0,
        },
        _ => UNKNOWN_TERM,
    };
    let front = if FRONT_HALL.contains(next.room_id) {
        100
    } else {
        10
    };
    let repeat = if two_back_wing == Some(next.wing) {
        100
    } else {
        0
    };
    HUE_WEIGHT * hue
        + DENSITY_WEIGHT * density
        + ENERGY_WEIGHT * energy
        + TEMPO_WEIGHT * tempo
        + KEY_WEIGHT * key
        + FRONT_HALL_WEIGHT * front
        - RECENT_WING_PENALTY * repeat
}

/// Whether two tempos can follow each other without the beat lurching:
/// within a quarter of each other, or near a 2:1 or 3:2 relation (within six
/// percent), which a listener hears as the same pulse regrouped.
fn tempos_follow(a: u32, b: u32) -> bool {
    let (low, high) = (u64::from(a.min(b)), u64::from(a.max(b)));
    let near = |numerator: u64, denominator: u64| {
        (high * denominator).abs_diff(low * numerator) * 100 <= 6 * low * numerator
    };
    high * 4 <= low * 5 || near(2, 1) || near(3, 2)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{ShowDirector, score, tempos_follow};
    use crate::show::features::{RoomFeatures, feature_table};
    use crate::{
        ENGINEERED_AHA_ROOM_IDS, FRONT_HALL, ROOM_CATALOG, THRESHOLD_ROOM_ID, catalog_index,
        director_profile, room_meta_by_id,
    };

    const CUES: usize = 60;

    fn features(id: &str) -> RoomFeatures {
        feature_table()[catalog_index(id).expect("catalog room")]
    }

    fn hue_step(a: &str, b: &str) -> u16 {
        let (a, b) = (features(a).hue.expect("hue"), features(b).hue.expect("hue"));
        a.abs_diff(b).min(360 - a.abs_diff(b))
    }

    fn walk(seed: u64, cues: usize) -> Vec<&'static str> {
        ShowDirector::new(seed)
            .take(cues)
            .map(|cue| cue.room_id())
            .collect()
    }

    #[test]
    fn the_same_seed_replays_the_same_show_and_seeds_diverge() {
        let first = ShowDirector::new(11).take(CUES).collect::<Vec<_>>();
        let replay = ShowDirector::new(11).take(CUES).collect::<Vec<_>>();
        assert_eq!(first, replay);
        assert_eq!(ShowDirector::new(11).nth(17), Some(first[17]));
        assert_ne!(walk(11, CUES), walk(12, CUES));
        for (position, cue) in first.iter().enumerate() {
            assert_eq!(cue.position(), position);
        }
        assert!(ShowDirector::opening_at("not-a-room", 1).is_none());
    }

    #[test]
    fn no_cue_shares_a_wing_with_its_neighbor_or_parks_on_a_near_static_room() {
        for seed in 0..8 {
            let mut previous = THRESHOLD_ROOM_ID;
            for id in walk(seed, CUES) {
                let wing = |id: &str| room_meta_by_id(id).expect("room").wing;
                assert_ne!(
                    wing(previous),
                    wing(id),
                    "seed {seed}: {previous} then {id}"
                );
                assert!(!features(id).is_near_static(), "seed {seed} parks on {id}");
                previous = id;
            }
        }
    }

    #[test]
    fn the_director_steps_further_around_the_color_wheel_than_catalog_order() {
        // The prototype measured a mean hue step near 100 degrees for catalog
        // order and above 130 for the director. Hold the director above 120
        // and clearly above the catalog's own first rooms.
        let catalog = ROOM_CATALOG[..=CUES]
            .windows(2)
            .map(|pair| u32::from(hue_step(pair[0].id, pair[1].id)))
            .sum::<u32>()
            / CUES as u32;
        for seed in 1..=8 {
            let mut previous = THRESHOLD_ROOM_ID;
            let mut total = 0;
            for id in walk(seed, CUES) {
                total += u32::from(hue_step(previous, id));
                previous = id;
            }
            let mean = total / CUES as u32;
            assert!(mean >= 120, "seed {seed} mean hue step {mean}");
            assert!(
                mean >= catalog + 20,
                "seed {seed}: {mean} against {catalog}"
            );
        }
    }

    #[test]
    fn the_front_hall_leads_and_the_rest_of_the_catalog_follows() {
        for seed in 1..=4 {
            let opening = walk(seed, 25);
            let front = opening.iter().filter(|id| FRONT_HALL.contains(id)).count();
            assert!(
                front * 5 >= 25 * 2,
                "seed {seed}: {front} of 25 from the hall"
            );
            let long = walk(seed, 240);
            let distinct = long.iter().collect::<HashSet<_>>().len();
            assert!(distinct >= 120, "seed {seed}: {distinct} distinct rooms");
            assert!(
                long.iter()
                    .take(super::RECENCY)
                    .all(|id| *id != THRESHOLD_ROOM_ID),
                "seed {seed} returned to the opening room too soon"
            );
        }
    }

    #[test]
    fn staged_rooms_keep_their_canonical_variation_and_their_profile() {
        let mut staged_seen = 0;
        for seed in 1..=8 {
            for cue in ShowDirector::new(seed).take(120) {
                let profile = director_profile(cue.room_id()).expect("profile");
                assert_eq!(cue.window(), profile.window());
                assert_eq!(cue.still(), profile.still());
                if ENGINEERED_AHA_ROOM_IDS.contains(&cue.room_id()) {
                    staged_seen += 1;
                    assert_eq!(cue.variation(), 0, "{} varied", cue.room_id());
                }
            }
        }
        assert!(staged_seen > 0, "the walk never reached a staged room");
        assert!(
            ShowDirector::new(0)
                .take(CUES)
                .all(|cue| cue.variation() == 0),
            "seed zero keeps every room canonical"
        );
    }

    #[test]
    fn a_long_show_never_runs_out_of_rooms() {
        assert_eq!(ShowDirector::new(3).take(600).count(), 600);
    }

    #[test]
    fn scoring_rewards_contrast_and_punishes_a_recent_wing() {
        let on_stage = features("times-tables");
        let next = features("lorenz");
        let fresh = score(&on_stage, &next, 3, None);
        let repeated = score(&on_stage, &next, 3, Some(next.wing));
        assert_eq!(fresh - repeated, 2_000);
        let mut gray = next;
        gray.hue = None;
        gray.tempo = None;
        gray.pitch_class = None;
        assert!(score(&on_stage, &gray, 3, None) > 0);
    }

    #[test]
    fn tempos_follow_within_a_quarter_or_by_a_simple_regrouping() {
        assert!(tempos_follow(96, 96));
        assert!(tempos_follow(96, 120));
        assert!(!tempos_follow(96, 121));
        assert!(tempos_follow(64, 128));
        assert!(tempos_follow(80, 120));
        assert!(!tempos_follow(64, 112));
        assert!(tempos_follow(128, 64));
        assert!(
            !tempos_follow(120, 64),
            "1.875 is more than six percent off 2:1"
        );
    }
}
