//! How the director presents one room hands-off.
//!
//! A director profile names the span of phase the Show performs for a room and
//! the still it holds under reduced motion. Most rooms take a profile derived
//! from the phase they are proudest of, their postcard. The table below holds
//! the rooms where that default is wrong: ends of a sweep that look like mush,
//! and staged rooms whose phase would perform the answer a player is meant to
//! reach. Profiles are keyed by room id and never change catalog metadata.

use crate::{canonical_room_id, room_by_id, room_meta_by_id};

/// Thousandths of the phase range in one unit of a [`PhaseWindow`].
const PERMILLE: u16 = 1000;
/// Largest phase a window may reach, keeping every phase inside `[0, 1)`.
const LAST_PERMILLE: u16 = PERMILLE - 1;
/// Width of a derived window, centered on the room's postcard where it fits.
const DERIVED_WIDTH: u16 = 400;

/// A span of room phase the Show performs, in thousandths of the phase range.
///
/// Whole thousandths keep profile data exact: two faces that read the same
/// window compute the same phases, and a test can name a boundary without a
/// floating-point tolerance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhaseWindow {
    start: u16,
    end: u16,
}

impl PhaseWindow {
    /// A window from `start` to `end` thousandths, or `None` unless
    /// `start < end <= 999`.
    #[must_use]
    pub const fn new(start: u16, end: u16) -> Option<Self> {
        if start < end && end <= LAST_PERMILLE {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// First phase of the window, in thousandths.
    #[must_use]
    pub const fn start_permille(self) -> u16 {
        self.start
    }

    /// Last phase of the window, in thousandths.
    #[must_use]
    pub const fn end_permille(self) -> u16 {
        self.end
    }

    /// First phase of the window.
    #[must_use]
    pub fn start(self) -> f64 {
        phase_of(self.start)
    }

    /// Last phase of the window.
    #[must_use]
    pub fn end(self) -> f64 {
        phase_of(self.end)
    }

    /// Whether `permille` lies inside the window, ends included.
    #[must_use]
    pub const fn contains_permille(self, permille: u16) -> bool {
        self.start <= permille && permille <= self.end
    }

    /// The phase after `progress` of a cue, eased in and out.
    ///
    /// `progress` runs from 0 at arrival to 1 at the curtain and is clamped,
    /// with a non-finite value treated as arrival. The smoothstep easing starts
    /// and ends at rest, so a face that dissolves between rooms never cuts
    /// away from a picture that is still accelerating.
    #[must_use]
    pub fn phase_at(self, progress: f64) -> f64 {
        let p = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let eased = p * p * (3.0 - 2.0 * p);
        self.start() + (self.end() - self.start()) * eased
    }

    /// `count` evenly spaced phases from the start to the end, both included.
    ///
    /// Fewer than two samples return the start alone.
    #[must_use]
    pub fn samples(self, count: usize) -> Vec<f64> {
        if count < 2 {
            return vec![self.start()];
        }
        let span = f64::from(self.end - self.start);
        (0..count)
            .map(|index| {
                let offset = span * index as f64 / (count - 1) as f64;
                (f64::from(self.start) + offset) / f64::from(PERMILLE)
            })
            .collect()
    }
}

/// How the director presents one room hands-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirectorProfile {
    room_id: &'static str,
    window: PhaseWindow,
    still: u16,
    authored: bool,
}

impl DirectorProfile {
    /// Canonical catalog room identifier.
    #[must_use]
    pub const fn room_id(self) -> &'static str {
        self.room_id
    }

    /// The span of phase the Show performs for this room.
    #[must_use]
    pub const fn window(self) -> PhaseWindow {
        self.window
    }

    /// The phase held under reduced motion, always inside the window.
    #[must_use]
    pub fn still(self) -> f64 {
        phase_of(self.still)
    }

    /// The reduced-motion still, in thousandths.
    #[must_use]
    pub const fn still_permille(self) -> u16 {
        self.still
    }

    /// Whether the profile was written by hand rather than derived from the
    /// room's postcard.
    #[must_use]
    pub const fn is_authored(self) -> bool {
        self.authored
    }
}

const fn authored(room_id: &'static str, start: u16, end: u16, still: u16) -> DirectorProfile {
    let window = match PhaseWindow::new(start, end) {
        Some(window) => window,
        None => panic!("an authored director window is ordered and inside [0, 1)"),
    };
    assert!(
        window.contains_permille(still),
        "an authored still lies inside its window"
    );
    DirectorProfile {
        room_id,
        window,
        still,
        authored: true,
    }
}

/// Every hand-written director profile, keyed by canonical room id.
///
/// An entry exists only where the derived window, centered on the postcard,
/// would be wrong. The reason for each is beside it, because a window without
/// a reason is a taste nobody can revisit.
pub(crate) const DIRECTOR_PROFILES: [DirectorProfile; 6] = [
    // The graded goal is exactly four lobes at K=5. Stopping at K=3.92 keeps
    // the performance a whole step of the dial short of it, and the still is
    // the canonical K=2 heart an ordinary visit opens on.
    authored("times-tables", 0, 240, 0),
    // The staged question is where the shadow twin ends up. Inside this span
    // the twins still swing as one pendulum, so the Show sets up the question
    // and never shows the answer. The default would center on the postcard,
    // where the twins are already far apart.
    authored("double-pendulum", 0, 300, 200),
    // The staged call is how speed changes near the sun. Near-circular orbits
    // keep every sector almost the same length, so the unequal arcs the player
    // is asked to predict never appear.
    authored("kepler-laws", 0, 130, 100),
    // The staged call is which policy wins. Only the two losing games are
    // performed; the third of the phase range that selects the winning
    // schedule is left to the player's hand.
    authored("parrondo", 0, 600, 0),
    // The iconic gasket lives at a jump of exactly one half. Past about 0.55 the
    // triangle breaks into dust, so the performance stays where it is a
    // triangle, and holds the exact one half.
    authored("chaos-game", 0, 250, 0),
    // A small detune shatters the packing into spokes, which is worth seeing;
    // the far end of the sweep is only noise. The still is exactly golden.
    authored("golden-angle", 0, 150, 0),
];

/// The director profile for a catalog room, authored or derived.
///
/// Returns `None` when `room_id` is not a listed room. Aliases resolve to
/// their canonical room. A derived profile performs a window centered on the
/// room's postcard where it fits, and holds the postcard as its still.
#[must_use]
pub fn director_profile(room_id: &str) -> Option<DirectorProfile> {
    let metadata = room_meta_by_id(room_id)?;
    let id = canonical_room_id(metadata.id);
    if let Some(profile) = DIRECTOR_PROFILES
        .iter()
        .find(|profile| profile.room_id == id)
    {
        return Some(*profile);
    }
    let postcard = room_by_id(id)?.postcard_t();
    Some(derived(metadata.id, postcard))
}

fn derived(room_id: &'static str, postcard: f64) -> DirectorProfile {
    let still = permille_of(postcard);
    let start = still
        .saturating_sub(DERIVED_WIDTH / 2)
        .min(LAST_PERMILLE - DERIVED_WIDTH);
    let window = PhaseWindow {
        start,
        end: start + DERIVED_WIDTH,
    };
    DirectorProfile {
        room_id,
        window,
        still,
        authored: false,
    }
}

fn permille_of(phase: f64) -> u16 {
    if !phase.is_finite() {
        return 0;
    }
    let scaled = (phase.clamp(0.0, 1.0) * f64::from(PERMILLE)).round();
    // The clamp bounds the value to [0, 1000], so the conversion is exact.
    (scaled as u16).min(LAST_PERMILLE)
}

fn phase_of(permille: u16) -> f64 {
    f64::from(permille) / f64::from(PERMILLE)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{DIRECTOR_PROFILES, PhaseWindow, director_profile};
    use crate::rooms::ROOM_OWN_ANSWER;
    use crate::rooms::kepler_laws::eccentricity_for_inputs;
    use crate::rooms::pendulum_aha::Ending;
    use crate::rooms::times_tables::TimesTables;
    use crate::show::features::feature_table;
    use crate::{
        ENGINEERED_AHA_ROOM_IDS, ROOM_CATALOG, Room, catalog_index, room_by_id, room_meta_by_id,
    };

    #[test]
    fn every_authored_profile_names_one_canonical_catalog_room() {
        let mut seen = HashSet::new();
        for profile in DIRECTOR_PROFILES {
            let metadata = room_meta_by_id(profile.room_id())
                .unwrap_or_else(|| panic!("{} is not a catalog room", profile.room_id()));
            assert_eq!(metadata.id, profile.room_id(), "profile ids are canonical");
            assert!(
                seen.insert(profile.room_id()),
                "{} twice",
                profile.room_id()
            );
            assert!(profile.is_authored());
        }
    }

    #[test]
    fn every_catalog_room_has_a_profile_whose_still_is_inside_its_window() {
        for metadata in ROOM_CATALOG {
            let profile = director_profile(metadata.id).expect("catalog room profile");
            assert_eq!(profile.room_id(), metadata.id);
            let window = profile.window();
            assert!(window.start_permille() < window.end_permille());
            assert!(window.end() < 1.0, "{} reaches phase 1", metadata.id);
            assert!(
                window.contains_permille(profile.still_permille()),
                "{} holds a still outside its window",
                metadata.id
            );
        }
        assert!(director_profile("not-a-room").is_none());
    }

    #[test]
    fn a_derived_window_centers_on_the_postcard_and_slides_inside_the_range() {
        let centered = super::derived("lissajous", 0.5);
        assert_eq!(
            centered.window(),
            PhaseWindow::new(300, 700).expect("window")
        );
        assert_eq!(centered.still_permille(), 500);
        let low = super::derived("lissajous", 0.0);
        assert_eq!(low.window(), PhaseWindow::new(0, 400).expect("window"));
        let high = super::derived("lissajous", 0.999);
        assert_eq!(high.window(), PhaseWindow::new(599, 999).expect("window"));
        assert_eq!(high.still_permille(), 999);
        assert_eq!(super::derived("lissajous", f64::NAN).still_permille(), 0);
    }

    #[test]
    fn eased_phase_rests_at_both_ends_and_stays_inside_the_window() {
        let window = PhaseWindow::new(100, 300).expect("window");
        assert_eq!(window.phase_at(0.0), window.start());
        assert_eq!(window.phase_at(1.0), window.end());
        assert_eq!(window.phase_at(-4.0), window.start());
        assert_eq!(window.phase_at(f64::NAN), window.start());
        assert!((window.phase_at(0.5) - 0.2).abs() < 1e-12);
        // Smoothstep is slow at both ends: the first tenth of the cue moves
        // the phase less than a tenth of the window.
        assert!(window.phase_at(0.1) - window.start() < 0.1 * 0.2);
        let samples = window.samples(5);
        assert_eq!(samples.len(), 5);
        assert_eq!(samples[0], 0.1);
        assert_eq!(samples[4], 0.3);
        assert!(samples.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(window.samples(1), vec![0.1]);
    }

    /// How a staged room's graded target would show at `phase`, if it does.
    ///
    /// Each answer is read from the room's own observable surface: its dial
    /// or its readout. A room whose picture phase barely moves is never
    /// performed at all, so its guard checks exactly that.
    fn staged_spoiler(room_id: &str, phase: f64) -> Option<String> {
        let room = room_by_id(room_id).expect("staged room");
        let status = room.status(phase).unwrap_or_default();
        match room_id {
            "times-tables" => {
                // The goal is exactly four lobes. The Show stays at least a
                // whole step of the dial short of it.
                let k = TimesTables::new().live_multiplier(phase, &[]);
                (k > FOUR_LOBE_K - 1.0).then(|| format!("K={k} is within a step of four lobes"))
            }
            "double-pendulum" => {
                let gap = status
                    .split_whitespace()
                    .skip_while(|word| *word != "TWINS")
                    .nth(1)
                    .and_then(|gap| gap.parse::<f64>().ok())
                    .expect("the twins readout");
                (Ending::of_gap(gap) != Ending::Together)
                    .then(|| format!("the twins are already {gap} apart"))
            }
            "kepler-laws" => {
                // The call is how speed changes near the sun. At most a
                // quarter faster at perihelion means e <= 1/9.
                let e = eccentricity_for_inputs(phase, &[], 0);
                ((1.0 + e) / (1.0 - e) > 1.25).then(|| format!("e={e} shows unequal arcs"))
            }
            "parrondo" => status
                .contains("ABB")
                .then(|| format!("performs the winning schedule: {status}")),
            "buffon-needle" => status
                .contains("PI~")
                .then(|| format!("prints an estimate: {status}")),
            "galton-board" | "nontransitive" => {
                // Phase barely moves either room, so the director never
                // performs them: a near-static room is not Show-eligible.
                let index = catalog_index(room_id).expect("catalog room");
                (!feature_table()[index].is_near_static())
                    .then(|| "phase now moves this room; give it a window guard".to_string())
            }
            other => Some(format!("{other} has no spoiler guard")),
        }
    }

    /// The Times Tables multiplier that draws the four lobes its goal asks for.
    const FOUR_LOBE_K: f64 = 5.0;

    #[test]
    fn every_staged_room_has_a_guard_and_its_window_never_shows_the_answer() {
        assert!(
            TimesTables::new().goal_met(TimesTables::phase_for_multiplier(FOUR_LOBE_K), &[]),
            "the guard's target is the room's own goal"
        );
        for room_id in ENGINEERED_AHA_ROOM_IDS {
            let profile = director_profile(room_id).expect("staged profile");
            let window = profile.window();
            for permille in window.start_permille()..=window.end_permille() {
                let phase = f64::from(permille) / 1000.0;
                if let Some(spoiler) = staged_spoiler(room_id, phase) {
                    panic!("{room_id} at phase {phase}: {spoiler}");
                }
            }
        }
    }

    #[test]
    fn the_guards_would_catch_each_default_that_was_replaced() {
        // Each authored staged window exists because the derived one, or the
        // far end of the sweep, performs the answer. The guards must see it.
        for (room_id, phase) in [
            ("times-tables", 0.375),
            ("double-pendulum", 0.75),
            ("kepler-laws", 0.55),
            ("parrondo", 0.75),
        ] {
            assert!(
                staged_spoiler(room_id, phase).is_some(),
                "{room_id} guard is blind at {phase}"
            );
        }
    }

    #[test]
    fn no_show_window_status_names_a_room_s_own_answer() {
        for (room_id, answer) in ROOM_OWN_ANSWER {
            let profile = director_profile(room_id).expect("profiled room");
            let room = room_by_id(room_id).expect("room");
            for phase in profile.window().samples(21) {
                let status = room.status(phase).unwrap_or_default().to_ascii_lowercase();
                let named = status.contains(answer)
                    || status
                        .split(|c: char| !c.is_ascii_alphabetic())
                        .any(|word| word.len() >= 3 && answer.starts_with(word));
                assert!(!named, "{room_id} says {answer:?} at {phase}: {status}");
            }
        }
    }

    #[test]
    fn malformed_windows_are_unrepresentable() {
        assert!(PhaseWindow::new(300, 300).is_none());
        assert!(PhaseWindow::new(400, 100).is_none());
        assert!(PhaseWindow::new(0, 1000).is_none());
        assert!(PhaseWindow::new(0, 999).is_some());
    }
}
