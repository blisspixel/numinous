//! The room registry: the catalog every face enumerates.
//!
//! The registry is the only thing a face needs to know about; it never depends
//! on a room's internals (see the dependency rule in `docs/ARCHITECTURE.md`).

use crate::room::Room;
use crate::rooms;

/// All built-in rooms, in catalog order.
#[must_use]
pub fn all_rooms() -> Vec<Box<dyn Room>> {
    vec![
        Box::new(rooms::times_tables::TimesTables::new()),
        Box::new(rooms::cellular_automata::CellularAutomata::new()),
    ]
}

/// Find a built-in room by its stable id, if it exists.
#[must_use]
pub fn room_by_id(id: &str) -> Option<Box<dyn Room>> {
    all_rooms().into_iter().find(|room| room.meta().id == id)
}

#[cfg(test)]
mod tests {
    use super::{all_rooms, room_by_id};

    #[test]
    fn registry_is_non_empty() {
        assert!(!all_rooms().is_empty());
    }

    #[test]
    fn every_room_has_a_unique_id() {
        let rooms = all_rooms();
        let mut ids: Vec<&str> = rooms.iter().map(|r| r.meta().id).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(unique, ids.len(), "room ids must be unique");
    }

    #[test]
    fn lookup_by_id_works_and_misses_are_none() {
        assert!(room_by_id("times-tables").is_some());
        assert!(room_by_id("no-such-room").is_none());
    }
}
