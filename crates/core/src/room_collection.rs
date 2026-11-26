//! Curated collections that gather rooms without refiling them.
//!
//! A wing is a taxonomy: every room has exactly one. A collection is a
//! curation laid over it, so a room can sit in a collection and keep its home
//! wing, and nothing outside the collection is hidden or removed by it.

/// A curated set of rooms, listed in a stable presentation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomCollection {
    /// Stable collection identifier.
    pub id: &'static str,
    /// Player-facing title.
    pub title: &'static str,
    /// Short reason to enter without giving away what any room holds.
    pub invitation: &'static str,
    /// Canonical catalog room identifiers, in presentation order.
    pub rooms: &'static [&'static str],
}

impl RoomCollection {
    /// Whether the collection holds a room, by canonical id or alias.
    #[must_use]
    pub fn contains(&self, room_id: &str) -> bool {
        let id = crate::canonical_room_id(room_id);
        self.rooms.contains(&id)
    }
}

/// The Front Hall: the rooms with the most depth behind them.
///
/// Chosen by evidence of depth rather than taste alone: a bespoke sound, a
/// staged experiment, a goal, a persistent session, a rebuild after
/// playtests, or a GPU path. Curation, not deletion: every other room stays
/// in its wing, and the Show weights toward this hall without confining
/// itself to it.
pub const FRONT_HALL: RoomCollection = RoomCollection {
    id: "front-hall",
    title: "Front Hall",
    invitation: "The rooms with the most behind their doors. Start anywhere.",
    rooms: &[
        "times-tables",
        "mandelbrot",
        "julia",
        "game-of-life",
        "galton-board",
        "double-pendulum",
        "lorenz",
        "golden-angle",
        "lissajous",
        "chladni",
        "cult-of-pi",
        "epicycles",
        "the-pour",
        "zeta-walk",
        "kepler-laws",
        "buffon-needle",
        "parrondo",
        "nontransitive",
        "the-only-move",
        "degree-720",
        "first-rain",
        "the-magnet",
        "phantom-jam",
        "ripple",
        "collatz",
    ],
};

/// Optional entries a player can find while browsing a room.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RoomSignposts {
    /// The room belongs to the curated Front Hall.
    pub front_hall: bool,
    /// Optional authored mathematics is available in the study reader.
    pub authored_mathematics: bool,
}

impl RoomSignposts {
    /// Read membership from the collection and study owners, including aliases.
    #[must_use]
    pub fn for_room(id: &str) -> Self {
        let canonical = crate::canonical_room_id(id);
        Self {
            front_hall: FRONT_HALL.contains(canonical),
            authored_mathematics: crate::rooms_with_authored_depth(crate::StudyDepth::Mathematics)
                .contains(&canonical),
        }
    }

    /// Short text labels shared by catalog browsers.
    #[must_use]
    pub fn label(self) -> &'static str {
        match (self.front_hall, self.authored_mathematics) {
            (true, true) => "Front Hall / Math study",
            (true, false) => FRONT_HALL.title,
            (false, true) => "Math study",
            (false, false) => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::FRONT_HALL;
    use crate::{THRESHOLD_ROOM_ID, room_meta_by_id};

    #[test]
    fn signposts_follow_both_content_owners() {
        use crate::{RoomSignposts, StudyDepth, rooms_with_authored_depth};
        for room in crate::ROOM_CATALOG {
            let signs = RoomSignposts::for_room(room.id);
            assert_eq!(signs.front_hall, FRONT_HALL.rooms.contains(&room.id));
            assert_eq!(
                signs.authored_mathematics,
                rooms_with_authored_depth(StudyDepth::Mathematics).contains(&room.id)
            );
        }
        for (id, label) in [
            ("times-tables", "Front Hall / Math study"),
            ("mandelbrot", "Front Hall"),
            ("fermat-spiral", "Math study"),
            ("route-lab", ""),
        ] {
            assert_eq!(RoomSignposts::for_room(id).label(), label);
        }
    }

    #[test]
    fn the_front_hall_is_twenty_five_distinct_canonical_catalog_rooms() {
        assert_eq!(FRONT_HALL.title, "Front Hall");
        assert_eq!(FRONT_HALL.rooms.len(), 25);
        let unique = FRONT_HALL.rooms.iter().copied().collect::<HashSet<_>>();
        assert_eq!(unique.len(), FRONT_HALL.rooms.len());
        for id in FRONT_HALL.rooms {
            let metadata =
                room_meta_by_id(id).unwrap_or_else(|| panic!("{id} is not a catalog room"));
            assert_eq!(metadata.id, *id, "collection ids are canonical");
        }
        assert_eq!(FRONT_HALL.rooms[0], THRESHOLD_ROOM_ID);
        assert!(FRONT_HALL.contains("times-tables"));
        assert!(!FRONT_HALL.contains("strange-loop"));
    }
}
