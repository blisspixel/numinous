use numinous_core::{Room, RoomInput, all_rooms_with};

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

pub(crate) fn reset_room_view(
    t: &mut f64,
    room_card: &mut u64,
    pokes: &mut Vec<(f64, f64)>,
    inputs: &mut Vec<RoomInput>,
) {
    *t = 0.0;
    *room_card = ROOM_CARD_FRAMES;
    pokes.clear();
    inputs.clear();
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

pub(crate) fn tick_room_card(room_card: &mut u64, obscured: bool) {
    if !obscured {
        *room_card = room_card.saturating_sub(1);
    }
}

fn keep_newest_inputs(inputs: &mut Vec<RoomInput>) {
    let overflow = inputs.len().saturating_sub(numinous_core::MAX_ROOM_INPUTS);
    if overflow > 0 {
        inputs.drain(0..overflow);
    }
}

/// Record the pointer landing: the start of a gesture, stamped with the
/// room phase. Points are normalized exactly like pokes.
pub(crate) fn record_pointer_down(inputs: &mut Vec<RoomInput>, point: (f64, f64), t: f64) -> bool {
    let Some((x, y)) = normalize_poke(point) else {
        keep_newest_inputs(inputs);
        return false;
    };
    inputs.push(RoomInput::PointerDown { x, y, t });
    keep_newest_inputs(inputs);
    true
}

/// Record a held move, stamped with the room phase. Callers gate this on the
/// same trail decimation as pokes, so gestures and poke trails stay in step.
pub(crate) fn record_pointer_move(inputs: &mut Vec<RoomInput>, point: (f64, f64), t: f64) -> bool {
    let Some((x, y)) = normalize_poke(point) else {
        keep_newest_inputs(inputs);
        return false;
    };
    inputs.push(RoomInput::PointerMove { x, y, t });
    keep_newest_inputs(inputs);
    true
}

/// Record the lift that completes a gesture, stamped with the room phase.
pub(crate) fn record_pointer_up(inputs: &mut Vec<RoomInput>, point: (f64, f64), t: f64) -> bool {
    let Some((x, y)) = normalize_poke(point) else {
        keep_newest_inputs(inputs);
        return false;
    };
    inputs.push(RoomInput::PointerUp { x, y, t });
    keep_newest_inputs(inputs);
    true
}

/// Close a gesture that ended without a lift (focus loss, a modal opening).
/// A cancel is appended only while a gesture is actually open, so releases
/// recorded normally are never followed by a stray cancel.
pub(crate) fn cancel_open_gesture(inputs: &mut Vec<RoomInput>) -> bool {
    let open = matches!(
        inputs.last(),
        Some(RoomInput::PointerDown { .. }) | Some(RoomInput::PointerMove { .. })
    );
    if open {
        inputs.push(RoomInput::PointerCancel);
        keep_newest_inputs(inputs);
    }
    open
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
        let mut inputs = Vec::new();
        reset_room_view(&mut t, &mut room_card, &mut pokes, &mut inputs);
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
    fn a_recorded_gesture_bridges_to_the_same_pokes_as_the_trail() {
        // Down and moves recorded beside the poke trail must bridge back to
        // an identical poke list, so legacy rooms see no wiring change.
        let mut pokes = Vec::new();
        let mut inputs = Vec::new();
        assert!(push_poke(&mut pokes, (0.30, 0.40)));
        assert!(record_pointer_down(&mut inputs, (0.30, 0.40), 0.10));
        // (0.305, 0.40) is inside the decimation step and must be rejected
        // by the trail AND therefore never recorded as a gesture move.
        for (i, point) in [(0.305, 0.40), (0.40, 0.40), (0.50, 0.45), (0.60, 0.50)]
            .iter()
            .enumerate()
        {
            if extend_poke_trail(&mut pokes, *point) {
                assert!(record_pointer_move(
                    &mut inputs,
                    *point,
                    0.11 + i as f64 * 0.01
                ));
            }
        }
        assert!(record_pointer_up(&mut inputs, (0.60, 0.50), 0.15));
        assert_eq!(numinous_core::pokes_from_inputs(&inputs), pokes);
    }

    #[test]
    fn gesture_recording_normalizes_bounds_and_rejects_bad_points() {
        let mut inputs = Vec::new();
        assert!(record_pointer_down(&mut inputs, (-0.5, 1.5), 0.0));
        assert_eq!(
            inputs.last(),
            Some(&numinous_core::RoomInput::PointerDown {
                x: 0.0,
                y: 1.0,
                t: 0.0
            })
        );
        assert!(!record_pointer_move(&mut inputs, (f64::NAN, 0.5), 0.1));
        assert!(!record_pointer_up(&mut inputs, (0.5, f64::INFINITY), 0.2));
        assert_eq!(inputs.len(), 1);
        for i in 0..numinous_core::MAX_ROOM_INPUTS + 9 {
            record_pointer_move(&mut inputs, (0.5, 0.5), i as f64 * 0.001);
        }
        assert_eq!(inputs.len(), numinous_core::MAX_ROOM_INPUTS);
    }

    #[test]
    fn a_cancel_closes_only_an_open_gesture() {
        let mut inputs = Vec::new();
        assert!(!cancel_open_gesture(&mut inputs), "nothing to cancel");
        record_pointer_down(&mut inputs, (0.5, 0.5), 0.1);
        assert!(cancel_open_gesture(&mut inputs));
        assert_eq!(
            inputs.last(),
            Some(&numinous_core::RoomInput::PointerCancel)
        );
        assert!(
            !cancel_open_gesture(&mut inputs),
            "a closed gesture is not re-cancelled"
        );
        record_pointer_down(&mut inputs, (0.6, 0.5), 0.2);
        record_pointer_up(&mut inputs, (0.6, 0.5), 0.3);
        assert!(
            !cancel_open_gesture(&mut inputs),
            "a released gesture is not cancelled"
        );
    }

    #[test]
    fn reset_room_view_clears_gesture_state_too() {
        let mut t = 0.7;
        let mut room_card = 3;
        let mut pokes = vec![(0.1, 0.2)];
        let mut inputs = vec![numinous_core::RoomInput::PointerCancel];
        reset_room_view(&mut t, &mut room_card, &mut pokes, &mut inputs);
        assert!(pokes.is_empty());
        assert!(inputs.is_empty());
    }

    #[test]
    fn room_card_tick_saturates() {
        let mut room_card = 1;
        tick_room_card(&mut room_card, false);
        assert_eq!(room_card, 0);
        tick_room_card(&mut room_card, false);
        assert_eq!(room_card, 0);
    }

    #[test]
    fn obscured_room_card_keeps_its_full_visible_lifetime() {
        let mut room_card = ROOM_CARD_FRAMES;
        tick_room_card(&mut room_card, true);
        assert_eq!(room_card, ROOM_CARD_FRAMES);
        tick_room_card(&mut room_card, false);
        assert_eq!(room_card, ROOM_CARD_FRAMES - 1);
    }
}
