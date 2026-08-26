//! Face-neutral direction for bounded, replayable shows.
//!
//! A show is an ordered score, not a hidden session. The core chooses the
//! route, cue phases, and deterministic variation. Faces decide how to carry
//! one cue and require the caller to request the next one explicitly.

use crate::{STRANGE_LOOP_WALK, SplitMix64, room_by_id_with};

/// How much visual motion one directed cue contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShowMotion {
    /// Three exact looks: arrival, the room postcard, and curtain.
    Sampled,
    /// One exact postcard look for a lower-motion presentation.
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
            Self::Curtain => "curtain",
        }
    }

    /// Short noninterpretive cue for presenting this look.
    #[must_use]
    pub const fn beat(self) -> &'static str {
        match self {
            Self::Arrival => "Arrive at an early exact phase.",
            Self::Postcard => "Hold the room's representative phase.",
            Self::Curtain => "Take one late exact phase, then return control.",
        }
    }
}

/// One exact phase in a directed cue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectedShowLook {
    role: ShowLookRole,
    phase: f64,
}

impl DirectedShowLook {
    /// Dramatic purpose of this look.
    #[must_use]
    pub const fn role(self) -> ShowLookRole {
        self.role
    }

    /// Short noninterpretive cue for presenting this look.
    #[must_use]
    pub const fn beat(self) -> &'static str {
        self.role.beat()
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
}

/// An ordered, face-neutral score for a caller-paced show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowScore {
    id: &'static str,
    title: &'static str,
    invitation: &'static str,
    route_version: u32,
}

impl ShowScore {
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
        STRANGE_LOOP_WALK.steps.len()
    }

    /// Direct one exact cue, or return `None` when `position` is outside the score.
    #[must_use]
    pub fn direct(self, seed: u64, position: usize, motion: ShowMotion) -> Option<DirectedShowCue> {
        let step = STRANGE_LOOP_WALK.steps.get(position)?;
        let variation = variation_for(seed, position);
        let room = room_by_id_with(step.room_id, variation)?;
        let postcard = room.postcard_t();
        let looks = match motion {
            ShowMotion::Sampled => vec![
                DirectedShowLook {
                    role: ShowLookRole::Arrival,
                    phase: 0.08,
                },
                DirectedShowLook {
                    role: ShowLookRole::Postcard,
                    phase: postcard,
                },
                DirectedShowLook {
                    role: ShowLookRole::Curtain,
                    phase: 0.92,
                },
            ],
            ShowMotion::Reduced => vec![DirectedShowLook {
                role: ShowLookRole::Postcard,
                phase: postcard,
            }],
        };
        debug_assert!(
            looks
                .iter()
                .all(|look| look.phase.is_finite() && (0.0..1.0).contains(&look.phase))
        );
        Some(DirectedShowCue {
            position,
            room_id: step.room_id,
            question: step.question,
            variation,
            looks,
        })
    }
}

/// The first score for minds: six caller-paced rooms along the Strange Loop walk.
pub const MINDS_SHOW: ShowScore = ShowScore {
    id: STRANGE_LOOP_WALK.id,
    title: "The Show: Strange Loop",
    invitation: STRANGE_LOOP_WALK.invitation,
    route_version: 1,
};

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
    use super::{MINDS_SHOW, ShowLookRole, ShowMotion};
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
        for position in 0..MINDS_SHOW.cue_count() {
            let cue = MINDS_SHOW
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
