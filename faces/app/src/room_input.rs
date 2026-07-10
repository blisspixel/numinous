use numinous_core::{Room, all_rooms_with};

pub(crate) const ROOM_CARD_FRAMES: u64 = 240;
const POKE_TRAIL_STEP: f64 = 0.02;

pub(crate) fn wrapped_room_index(current: usize, delta: isize, room_count: usize) -> usize {
    if room_count == 0 {
        return 0;
    }
    let n = room_count as isize;
    (((current as isize + delta) % n + n) % n) as usize
}

pub(crate) fn redeal_rooms(variation: &mut u64, current: &mut usize) -> Vec<Box<dyn Room>> {
    *variation = variation.wrapping_add(1);
    let rooms = all_rooms_with(*variation);
    if !rooms.is_empty() {
        *current = (*current).min(rooms.len() - 1);
    }
    rooms
}

pub(crate) fn reset_room_view(t: &mut f64, room_card: &mut u64, pokes: &mut Vec<(f64, f64)>) {
    *t = 0.0;
    *room_card = ROOM_CARD_FRAMES;
    pokes.clear();
}

fn normalize_poke(point: (f64, f64)) -> Option<(f64, f64)> {
    if !point.0.is_finite() || !point.1.is_finite() {
        return None;
    }
    Some((point.0.clamp(0.0, 1.0), point.1.clamp(0.0, 1.0)))
}

fn keep_newest_pokes(pokes: &mut Vec<(f64, f64)>) {
    let overflow = pokes.len().saturating_sub(numinous_core::MAX_ROOM_POKES);
    if overflow > 0 {
        pokes.drain(0..overflow);
    }
}

pub(crate) fn push_poke(pokes: &mut Vec<(f64, f64)>, point: (f64, f64)) -> bool {
    let Some(point) = normalize_poke(point) else {
        keep_newest_pokes(pokes);
        return false;
    };
    pokes.push(point);
    keep_newest_pokes(pokes);
    true
}

pub(crate) fn extend_poke_trail(pokes: &mut Vec<(f64, f64)>, point: (f64, f64)) -> bool {
    let Some(point) = normalize_poke(point) else {
        keep_newest_pokes(pokes);
        return false;
    };
    let far_enough = pokes.last().is_none_or(|&(lx, ly)| {
        ((point.0 - lx).powi(2) + (point.1 - ly).powi(2)).sqrt() > POKE_TRAIL_STEP
    });
    if far_enough {
        return push_poke(pokes, point);
    }
    keep_newest_pokes(pokes);
    false
}

pub(crate) fn tick_room_card(room_card: &mut u64) {
    *room_card = room_card.saturating_sub(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_room_index_handles_both_directions() {
        assert_eq!(wrapped_room_index(0, -1, 5), 4);
        assert_eq!(wrapped_room_index(4, 1, 5), 0);
        assert_eq!(wrapped_room_index(2, -2, 5), 0);
        assert_eq!(wrapped_room_index(2, 3, 5), 0);
        assert_eq!(wrapped_room_index(2, 1, 0), 0);
    }

    #[test]
    fn redeal_rooms_advances_variation_and_keeps_index_valid() {
        let mut variation = 0;
        let mut current = usize::MAX;
        let rooms = redeal_rooms(&mut variation, &mut current);
        assert_eq!(variation, 1);
        assert!(!rooms.is_empty());
        assert_eq!(current, rooms.len() - 1);
    }

    #[test]
    fn reset_room_view_clears_visit_state() {
        let mut t = 0.7;
        let mut room_card = 3;
        let mut pokes = vec![(0.1, 0.2)];
        reset_room_view(&mut t, &mut room_card, &mut pokes);
        assert_eq!(t, 0.0);
        assert_eq!(room_card, ROOM_CARD_FRAMES);
        assert!(pokes.is_empty());
    }

    #[test]
    fn poke_history_is_bounded_and_keeps_newest_points() {
        let mut pokes = Vec::new();
        for i in 0..(numinous_core::MAX_ROOM_POKES + 3) {
            assert!(push_poke(&mut pokes, (i as f64, 0.0)));
        }
        assert_eq!(pokes.len(), numinous_core::MAX_ROOM_POKES);
        assert_eq!(pokes[0], (1.0, 0.0));
    }

    #[test]
    fn poke_history_repairs_oversized_vectors() {
        let mut pokes = vec![(0.0, 0.0); numinous_core::MAX_ROOM_POKES + 7];
        assert!(push_poke(&mut pokes, (0.25, 0.75)));
        assert_eq!(pokes.len(), numinous_core::MAX_ROOM_POKES);
        assert_eq!(pokes.last(), Some(&(0.25, 0.75)));
    }

    #[test]
    fn poke_points_are_normalized_before_storage() {
        let mut pokes = Vec::new();
        assert!(push_poke(&mut pokes, (-0.2, 1.4)));
        assert_eq!(pokes, vec![(0.0, 1.0)]);
        assert!(!push_poke(&mut pokes, (f64::NAN, 0.5)));
        assert_eq!(pokes, vec![(0.0, 1.0)]);
    }

    #[test]
    fn poke_trail_only_extends_after_meaningful_motion() {
        let mut pokes = vec![(0.5, 0.5)];
        assert!(!extend_poke_trail(&mut pokes, (0.51, 0.5)));
        assert_eq!(pokes.len(), 1);
        assert!(extend_poke_trail(&mut pokes, (1.3, -0.1)));
        assert_eq!(pokes, vec![(0.5, 0.5), (1.0, 0.0)]);
        assert!(!extend_poke_trail(&mut pokes, (f64::INFINITY, 0.0)));
        assert_eq!(pokes, vec![(0.5, 0.5), (1.0, 0.0)]);
    }

    #[test]
    fn room_card_tick_saturates() {
        let mut room_card = 1;
        tick_room_card(&mut room_card);
        assert_eq!(room_card, 0);
        tick_room_card(&mut room_card);
        assert_eq!(room_card, 0);
    }
}
