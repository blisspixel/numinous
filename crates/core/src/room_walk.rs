//! Curated room walks that connect several phenomena into one question.
//!
//! A walk is face-neutral content. Faces decide how to present its doorway,
//! while the core owns the ordered rooms and the question carried between
//! them.

/// One room in a curated walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomWalkStep {
    /// Canonical catalog room identifier.
    pub room_id: &'static str,
    /// Nonspoiling question to carry into this room.
    pub question: &'static str,
}

/// An ordered route through related rooms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomWalk {
    /// Stable walk identifier.
    pub id: &'static str,
    /// Player-facing title.
    pub title: &'static str,
    /// Short reason to enter without giving away the destination.
    pub invitation: &'static str,
    /// Ordered room steps.
    pub steps: &'static [RoomWalkStep],
}

/// The Strange Loop walk, from local rules through self-reference.
pub const STRANGE_LOOP_WALK: RoomWalk = RoomWalk {
    id: "strange-loop",
    title: "Walk the Strange Loop",
    invitation: "Follow six rooms from a tiny local rule toward a system that can point back at itself.",
    steps: &[
        RoomWalkStep {
            room_id: "cellular-automata",
            question: "How much can one local rule grow?",
        },
        RoomWalkStep {
            room_id: "game-of-life",
            question: "When does a grid begin to look alive?",
        },
        RoomWalkStep {
            room_id: "rule-110",
            question: "Can one tiny rule carry any computation?",
        },
        RoomWalkStep {
            room_id: "busy-beaver",
            question: "Where does computation outrun prediction?",
        },
        RoomWalkStep {
            room_id: "quine",
            question: "What does it take for a system to describe itself?",
        },
        RoomWalkStep {
            room_id: "strange-loop",
            question: "What changes when description turns back on the describer?",
        },
    ],
};

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::STRANGE_LOOP_WALK;

    #[test]
    fn strange_loop_walk_is_ordered_canonical_and_complete() {
        let ids = STRANGE_LOOP_WALK
            .steps
            .iter()
            .map(|step| step.room_id)
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            [
                "cellular-automata",
                "game-of-life",
                "rule-110",
                "busy-beaver",
                "quine",
                "strange-loop",
            ]
        );
        assert_eq!(ids.iter().copied().collect::<HashSet<_>>().len(), ids.len());
        for step in STRANGE_LOOP_WALK.steps {
            let metadata = crate::room_meta_by_id(step.room_id).expect("walk room in catalog");
            assert_eq!(metadata.id, step.room_id, "walk ids must be canonical");
            assert!(!step.question.trim().is_empty());
            assert!(step.question.ends_with('?'));
        }
    }
}
