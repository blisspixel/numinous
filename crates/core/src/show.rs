//! Face-neutral direction for bounded, replayable shows.
//!
//! A show is an ordered score, not a hidden session. The core chooses the
//! route, cue phases, and deterministic variation. Faces decide how to carry
//! one cue and require the caller to request the next one explicitly.
//!
//! Two kinds of direction live here. A [`ShowScore`] is a short authored
//! route, such as the Strange Loop walk or [`OVERTURE`], that MCP carries one
//! cue per call. A [`ShowDirector`] is the endless hands-off Show: it orders
//! every eligible room by contrast, performing each inside its
//! [`DirectorProfile`] window. Neither records a Journey visit: a room the Show
//! only passes through is watched, not visited.

mod director;
mod features;
mod overture;
mod profiles;

pub use director::{DirectorCue, ShowDirector};
pub use overture::{
    Ease, ExactRatio, LockStep, OCTAVE_LOCK, OVERTURE, Overture, OvertureBeat, OvertureKey,
    ShowHandoff,
};
pub use profiles::{DirectorProfile, PhaseWindow, director_profile};

use crate::{RoomWalk, STRANGE_LOOP_WALK, SplitMix64, room_by_id_with};

/// How much visual motion one directed cue contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShowMotion {
    /// Every exact look in the cue: arrival, the room postcard, and curtain on
    /// a walk; arrival, passage, and curtain on the Overture.
    Sampled,
    /// One exact still for a lower-motion presentation: the postcard on a
    /// walk, the finished picture of an Overture beat.
    Reduced,
}

impl ShowMotion {
    /// Stable value used by face-level replay contracts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sampled => "sampled",
            Self::Reduced => "reduced",
        }
    }
}

/// The dramatic purpose of one exact look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShowLookRole {
    /// A quiet look shortly after entering the room.
    Arrival,
    /// The phase the room itself selects for a representative still.
    Postcard,
    /// An authored phase on the way from arrival to curtain.
    Passage,
    /// A late look before returning control to the caller.
    Curtain,
}

impl ShowLookRole {
    /// Stable value used by face-level replay contracts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Arrival => "arrival",
            Self::Postcard => "postcard",
            Self::Passage => "passage",
            Self::Curtain => "curtain",
        }
    }

    /// Short noninterpretive cue for presenting this look.
    #[must_use]
    pub const fn beat(self) -> &'static str {
        match self {
            Self::Arrival => "Arrive at an early exact phase.",
            Self::Postcard => "Hold the room's representative phase.",
            Self::Passage => "Pass through an exact phase on the way.",
            Self::Curtain => "Take one late exact phase, then return control.",
        }
    }
}

/// One exact phase in a directed cue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectedShowLook {
    role: ShowLookRole,
    phase: f64,
    beat: &'static str,
}

impl DirectedShowLook {
    const fn of_role(role: ShowLookRole, phase: f64) -> Self {
        Self {
            role,
            phase,
            beat: role.beat(),
        }
    }

    /// Dramatic purpose of this look.
    #[must_use]
    pub const fn role(self) -> ShowLookRole {
        self.role
    }

    /// Short noninterpretive cue for presenting this look.
    #[must_use]
    pub const fn beat(self) -> &'static str {
        self.beat
    }

    /// Exact normalized room phase in `[0, 1)`.
    #[must_use]
    pub const fn phase(self) -> f64 {
        self.phase
    }
}

/// One replayable room cue selected from a show score.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectedShowCue {
    position: usize,
    room_id: &'static str,
    question: &'static str,
    variation: u64,
    looks: Vec<DirectedShowLook>,
    sound_phase: f64,
    octave_lock: Option<&'static [LockStep]>,
    handoff: Option<ShowHandoff>,
}

impl DirectedShowCue {
    /// Zero-based position in the show score.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Canonical room identifier.
    #[must_use]
    pub const fn room_id(&self) -> &'static str {
        self.room_id
    }

    /// Nonspoiling question carried into the room.
    #[must_use]
    pub const fn question(&self) -> &'static str {
        self.question
    }

    /// Deterministic room variation selected from the show seed and position.
    #[must_use]
    pub const fn variation(&self) -> u64 {
        self.variation
    }

    /// Exact ordered looks in this cue.
    #[must_use]
    pub fn looks(&self) -> &[DirectedShowLook] {
        &self.looks
    }

    /// The phase whose sound represents the cue: the room's postcard on a
    /// walk, the finished still of an Overture beat. It is the reduced-motion
    /// look's phase too, so the picture and the sound agree.
    #[must_use]
    pub const fn sound_phase(&self) -> f64 {
        self.sound_phase
    }

    /// The exact octave lock this cue's looks perform, if any.
    #[must_use]
    pub const fn octave_lock(&self) -> Option<&'static [LockStep]> {
        self.octave_lock
    }

    /// Where the score leaves the player after this cue, on its last cue only
    /// when the score hands over a room rather than simply ending.
    #[must_use]
    pub const fn handoff(&self) -> Option<ShowHandoff> {
        self.handoff
    }
}

/// The route a score follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScoreRoute {
    /// A curated walk: each step's room at arrival, postcard, and curtain.
    Walk(RoomWalk),
    /// The authored [`OVERTURE`], beat by beat.
    Overture,
}

/// An ordered, face-neutral score for a caller-paced show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowScore {
    id: &'static str,
    title: &'static str,
    invitation: &'static str,
    route_version: u32,
    route: ScoreRoute,
}

impl ShowScore {
    /// The score with this stable identifier, if any.
    #[must_use]
    pub fn by_id(id: &str) -> Option<Self> {
        SHOW_SCORES.into_iter().find(|score| score.id == id)
    }

    /// Stable score identifier.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// Player-facing score title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        self.title
    }

    /// Short reason to enter without disclosing the destination.
    #[must_use]
    pub const fn invitation(self) -> &'static str {
        self.invitation
    }

    /// Version of the ordered route and its nonspoiling questions.
    #[must_use]
    pub const fn route_version(self) -> u32 {
        self.route_version
    }

    /// Number of caller-paced cues in the score.
    #[must_use]
    pub const fn cue_count(self) -> usize {
        match self.route {
            ScoreRoute::Walk(walk) => walk.steps.len(),
            ScoreRoute::Overture => OVERTURE.beats().len(),
        }
    }

    /// Direct one exact cue, or return `None` when `position` is outside the score.
    ///
    /// A walk varies its rooms with `seed`. The Overture is one authored
    /// piece written against each room's canonical variation, so it plays the
    /// same for every seed.
    #[must_use]
    pub fn direct(self, seed: u64, position: usize, motion: ShowMotion) -> Option<DirectedShowCue> {
        let cue = match self.route {
            ScoreRoute::Walk(walk) => direct_walk(walk, seed, position, motion)?,
            ScoreRoute::Overture => direct_overture(position, motion)?,
        };
        debug_assert!(
            cue.looks
                .iter()
                .all(|look| look.phase.is_finite() && (0.0..1.0).contains(&look.phase))
        );
        Some(cue)
    }
}

fn direct_walk(
    walk: RoomWalk,
    seed: u64,
    position: usize,
    motion: ShowMotion,
) -> Option<DirectedShowCue> {
    let step = walk.steps.get(position)?;
    let variation = variation_for(seed, position);
    let room = room_by_id_with(step.room_id, variation)?;
    let postcard = room.postcard_t();
    let looks = match motion {
        ShowMotion::Sampled => vec![
            DirectedShowLook::of_role(ShowLookRole::Arrival, 0.08),
            DirectedShowLook::of_role(ShowLookRole::Postcard, postcard),
            DirectedShowLook::of_role(ShowLookRole::Curtain, 0.92),
        ],
        ShowMotion::Reduced => vec![DirectedShowLook::of_role(ShowLookRole::Postcard, postcard)],
    };
    Some(DirectedShowCue {
        position,
        room_id: step.room_id,
        question: step.question,
        variation,
        looks,
        sound_phase: postcard,
        octave_lock: None,
        handoff: None,
    })
}

fn direct_overture(position: usize, motion: ShowMotion) -> Option<DirectedShowCue> {
    let beat = OVERTURE.beats().get(position)?;
    let look = |index: usize| {
        let key = beat.keys()[index];
        DirectedShowLook {
            role: beat.role(index),
            phase: key.phase(),
            beat: key.beat(),
        }
    };
    let looks = match motion {
        ShowMotion::Sampled => (0..beat.keys().len()).map(look).collect(),
        ShowMotion::Reduced => vec![look(beat.still_index())],
    };
    let last = position + 1 == OVERTURE.beats().len();
    Some(DirectedShowCue {
        position,
        room_id: beat.room_id(),
        question: beat.question(),
        variation: 0,
        looks,
        sound_phase: beat.still().phase(),
        octave_lock: beat.octave_lock(),
        handoff: last.then(|| OVERTURE.handoff()),
    })
}

/// The first score for minds: six caller-paced rooms along the Strange Loop walk.
pub const MINDS_SHOW: ShowScore = ShowScore {
    id: STRANGE_LOOP_WALK.id,
    title: "The Show: Strange Loop",
    invitation: STRANGE_LOOP_WALK.invitation,
    route_version: 1,
    route: ScoreRoute::Walk(STRANGE_LOOP_WALK),
};

/// The Overture as a caller-paced score: four beats, one call each, ending
/// with a followable hand-off into Times Tables.
pub const OVERTURE_SHOW: ShowScore = ShowScore {
    id: OVERTURE.id(),
    title: OVERTURE.title(),
    invitation: OVERTURE.invitation(),
    route_version: 1,
    route: ScoreRoute::Overture,
};

/// Every caller-paced score, the default first.
pub const SHOW_SCORES: [ShowScore; 2] = [MINDS_SHOW, OVERTURE_SHOW];

fn variation_for(seed: u64, position: usize) -> u64 {
    if seed == 0 {
        return 0;
    }
    let mut generator = SplitMix64::new(seed);
    (0..=position)
        .map(|_| generator.next_u64())
        .last()
        .unwrap_or(seed)
}

#[cfg(test)]
mod tests {
    use super::{
        MINDS_SHOW, OCTAVE_LOCK, OVERTURE, OVERTURE_SHOW, SHOW_SCORES, ShowLookRole, ShowMotion,
        ShowScore,
    };
    use crate::{Canvas, STRANGE_LOOP_WALK, room_by_id_with};

    #[test]
    fn minds_show_preserves_the_curated_route_and_questions() {
        assert_eq!(MINDS_SHOW.id(), "strange-loop");
        assert_eq!(MINDS_SHOW.route_version(), 1);
        assert_eq!(MINDS_SHOW.cue_count(), 6);
        for (position, step) in STRANGE_LOOP_WALK.steps.iter().enumerate() {
            let cue = MINDS_SHOW
                .direct(0, position, ShowMotion::Sampled)
                .expect("curated cue");
            assert_eq!(cue.position(), position);
            assert_eq!(cue.room_id(), step.room_id);
            assert_eq!(cue.question(), step.question);
            assert_eq!(cue.variation(), 0);
            assert!(cue.octave_lock().is_none());
            assert!(cue.handoff().is_none());
        }
        assert!(MINDS_SHOW.direct(0, 6, ShowMotion::Sampled).is_none());
    }

    #[test]
    fn sampled_and_reduced_direction_use_exact_bounded_phases() {
        for position in 0..MINDS_SHOW.cue_count() {
            let sampled = MINDS_SHOW
                .direct(17, position, ShowMotion::Sampled)
                .expect("sampled cue");
            let reduced = MINDS_SHOW
                .direct(17, position, ShowMotion::Reduced)
                .expect("reduced cue");
            assert_eq!(sampled.looks().len(), 3);
            assert_eq!(reduced.looks().len(), 1);
            assert_eq!(sampled.looks()[0].role(), ShowLookRole::Arrival);
            assert_eq!(sampled.looks()[1].role(), ShowLookRole::Postcard);
            assert_eq!(sampled.looks()[2].role(), ShowLookRole::Curtain);
            assert_eq!(reduced.looks()[0], sampled.looks()[1]);
            assert_eq!(reduced.looks()[0].phase(), sampled.sound_phase());
            assert_eq!(
                sampled.looks()[1].beat(),
                ShowLookRole::Postcard.beat(),
                "walk looks keep their role's cue"
            );
            assert!(
                sampled
                    .looks()
                    .iter()
                    .all(|look| (0.0..1.0).contains(&look.phase()))
            );
        }
    }

    #[test]
    fn direction_is_replayable_and_seeded_variations_diverge() {
        let first = MINDS_SHOW.direct(23, 4, ShowMotion::Sampled).expect("cue");
        let replay = MINDS_SHOW
            .direct(23, 4, ShowMotion::Sampled)
            .expect("replay");
        let other = MINDS_SHOW
            .direct(24, 4, ShowMotion::Sampled)
            .expect("other seed");
        assert_eq!(first, replay);
        assert_ne!(first.variation(), other.variation());
    }

    #[test]
    fn every_directed_look_renders_a_nonblank_exact_frame() {
        for score in SHOW_SCORES {
            for position in 0..score.cue_count() {
                let cue = score
                    .direct(31, position, ShowMotion::Sampled)
                    .expect("cue");
                let room = room_by_id_with(cue.room_id(), cue.variation()).expect("room");
                for look in cue.looks() {
                    let mut canvas = Canvas::new(72, 32);
                    room.render(&mut canvas, look.phase());
                    assert!(canvas.ink_count() > 0, "blank {} look", cue.room_id());
                }
            }
        }
    }

    #[test]
    fn scores_resolve_by_id_and_the_strange_loop_stays_the_default() {
        assert_eq!(SHOW_SCORES[0], MINDS_SHOW);
        assert_eq!(ShowScore::by_id("strange-loop"), Some(MINDS_SHOW));
        assert_eq!(ShowScore::by_id("overture"), Some(OVERTURE_SHOW));
        assert_eq!(ShowScore::by_id("finale"), None);
        assert_eq!(OVERTURE_SHOW.title(), "The Overture");
        assert_eq!(OVERTURE_SHOW.cue_count(), 4);
    }

    #[test]
    fn the_overture_score_carries_authored_keys_the_lock_and_the_handoff() {
        for (position, beat) in OVERTURE.beats().iter().enumerate() {
            let sampled = OVERTURE_SHOW
                .direct(99, position, ShowMotion::Sampled)
                .expect("overture cue");
            assert_eq!(sampled.room_id(), beat.room_id());
            assert_eq!(sampled.question(), beat.question());
            assert_eq!(sampled.variation(), 0, "the overture is one authored piece");
            assert_eq!(
                sampled,
                OVERTURE_SHOW
                    .direct(0, position, ShowMotion::Sampled)
                    .expect("seed zero"),
                "seed does not vary the overture"
            );
            let roles = sampled
                .looks()
                .iter()
                .map(|look| look.role())
                .collect::<Vec<_>>();
            assert_eq!(
                roles,
                [
                    ShowLookRole::Arrival,
                    ShowLookRole::Passage,
                    ShowLookRole::Curtain
                ]
            );
            for (look, key) in sampled.looks().iter().zip(beat.keys()) {
                assert_eq!(look.phase(), key.phase());
                assert_eq!(look.beat(), key.beat());
            }
            let reduced = OVERTURE_SHOW
                .direct(99, position, ShowMotion::Reduced)
                .expect("reduced cue");
            assert_eq!(reduced.looks().len(), 1);
            assert_eq!(reduced.looks()[0].phase(), beat.still().phase());
            assert_eq!(sampled.sound_phase(), beat.still().phase());
            assert_eq!(
                sampled.handoff().is_some(),
                position + 1 == OVERTURE.beats().len()
            );
        }
        let last = OVERTURE_SHOW
            .direct(0, 3, ShowMotion::Sampled)
            .expect("hand-off cue");
        assert_eq!(last.octave_lock(), Some(OCTAVE_LOCK.as_slice()));
        let handoff = last.handoff().expect("the overture hands over a room");
        assert_eq!(handoff.room_id(), "times-tables");
        assert_eq!(handoff.phase(), last.looks()[2].phase());
        assert!(OVERTURE_SHOW.direct(0, 4, ShowMotion::Sampled).is_none());
    }
}
