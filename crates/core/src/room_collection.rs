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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::FRONT_HALL;
    use crate::{THRESHOLD_ROOM_ID, room_meta_by_id};

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
