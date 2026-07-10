use std::collections::BTreeSet;

use crate::play::NimPlay;
use numinous_core::munch_arcade::Action;
use numinous_core::munchers::{COLS, ROWS};
use winit::keyboard::{Key, NamedKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridControl {
    Primary,
    Right,
    Left,
    Down,
    Up,
}

fn grid_control(key: &Key) -> Option<GridControl> {
    match key {
        Key::Named(NamedKey::Space) => Some(GridControl::Primary),
        Key::Named(NamedKey::ArrowRight) => Some(GridControl::Right),
        Key::Named(NamedKey::ArrowLeft) => Some(GridControl::Left),
        Key::Named(NamedKey::ArrowDown) => Some(GridControl::Down),
        Key::Named(NamedKey::ArrowUp) => Some(GridControl::Up),
        Key::Character(c) => match c.as_str() {
            "e" => Some(GridControl::Primary),
            "d" => Some(GridControl::Right),
            "a" => Some(GridControl::Left),
            "s" => Some(GridControl::Down),
            "w" => Some(GridControl::Up),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MunchControl {
    ToggleBite,
    Right,
    Left,
    Down,
    Up,
}

fn munch_control(key: &Key) -> Option<MunchControl> {
    Some(match grid_control(key)? {
        GridControl::Primary => MunchControl::ToggleBite,
        GridControl::Right => MunchControl::Right,
        GridControl::Left => MunchControl::Left,
        GridControl::Down => MunchControl::Down,
        GridControl::Up => MunchControl::Up,
    })
}

pub(crate) fn arcade_action_for_key(key: &Key) -> Option<Action> {
    Some(match grid_control(key)? {
        GridControl::Primary => Action::Eat,
        GridControl::Right => Action::Right,
        GridControl::Left => Action::Left,
        GridControl::Down => Action::Down,
        GridControl::Up => Action::Up,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NimControl {
    PreviousHeap,
    NextHeap,
    IncreaseTake,
    DecreaseTake,
}

fn nim_control(key: &Key) -> Option<NimControl> {
    match key {
        Key::Named(NamedKey::ArrowUp) => Some(NimControl::PreviousHeap),
        Key::Named(NamedKey::ArrowDown) => Some(NimControl::NextHeap),
        Key::Character(c) => match c.as_str() {
            "w" => Some(NimControl::PreviousHeap),
            "s" => Some(NimControl::NextHeap),
            "d" => Some(NimControl::IncreaseTake),
            "a" => Some(NimControl::DecreaseTake),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn apply_nim_control(play: &mut NimPlay, key: &Key) -> bool {
    let Some(control) = nim_control(key) else {
        return false;
    };
    match control {
        NimControl::PreviousHeap => move_nim_heap(play, false),
        NimControl::NextHeap => move_nim_heap(play, true),
        NimControl::IncreaseTake => {
            play.take = play
                .take
                .saturating_add(1)
                .min(selected_nim_heap(play).max(1));
        }
        NimControl::DecreaseTake => play.take = play.take.saturating_sub(1).max(1),
    }
    true
}

fn move_nim_heap(play: &mut NimPlay, forward: bool) {
    let n = play.heaps.len();
    if n == 0 {
        play.selected = 0;
        play.take = play.take.min(1);
        return;
    }
    play.selected = play.selected.min(n - 1);
    for step in 1..=n {
        let heap = if forward {
            (play.selected + step) % n
        } else {
            (play.selected + n - step % n) % n
        };
        if play.heaps[heap] > 0 {
            play.selected = heap;
            break;
        }
    }
    play.take = play.take.min(selected_nim_heap(play).max(1));
}

fn selected_nim_heap(play: &NimPlay) -> u32 {
    play.heaps.get(play.selected).copied().unwrap_or(0)
}

pub(crate) fn toggle_munch_bite(bites: &mut BTreeSet<usize>, cell: usize) {
    if !bites.remove(&cell) {
        let _ = bites.insert(cell);
    }
}

pub(crate) fn apply_munch_control(
    cursor: &mut usize,
    bites: &mut BTreeSet<usize>,
    key: &Key,
) -> bool {
    let Some(control) = munch_control(key) else {
        return false;
    };
    let cell_count = ROWS * COLS;
    match control {
        MunchControl::ToggleBite => {
            toggle_munch_bite(bites, *cursor);
        }
        MunchControl::Right => *cursor = (*cursor + 1) % cell_count,
        MunchControl::Left => *cursor = (*cursor + cell_count - 1) % cell_count,
        MunchControl::Down => *cursor = (*cursor + COLS) % cell_count,
        MunchControl::Up => *cursor = (*cursor + cell_count - COLS) % cell_count,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nim_play(heaps: Vec<u32>) -> NimPlay {
        NimPlay {
            selected: heaps.iter().position(|&h| h > 0).unwrap_or(0),
            take: 1,
            heaps,
            seed: 1,
            message: String::new(),
            over: None,
        }
    }

    #[test]
    fn arcade_control_maps_keyboard_and_arrows() {
        assert_eq!(
            arcade_action_for_key(&Key::Character("w".into())),
            Some(Action::Up)
        );
        assert_eq!(
            arcade_action_for_key(&Key::Named(NamedKey::ArrowRight)),
            Some(Action::Right)
        );
        assert_eq!(
            arcade_action_for_key(&Key::Named(NamedKey::Space)),
            Some(Action::Eat)
        );
        assert_eq!(arcade_action_for_key(&Key::Character("x".into())), None);
    }

    #[test]
    fn nim_control_moves_between_live_heaps_and_clamps_take() {
        let mut play = nim_play(vec![0, 3, 5]);
        play.selected = 2;
        play.take = 5;

        assert!(apply_nim_control(
            &mut play,
            &Key::Named(NamedKey::ArrowDown)
        ));

        assert_eq!(play.selected, 1);
        assert_eq!(play.take, 3);
        assert!(apply_nim_control(&mut play, &Key::Character("s".into())));
        assert_eq!(play.selected, 2);
        assert_eq!(play.take, 3);
    }

    #[test]
    fn nim_control_adjusts_take_with_wasd() {
        let mut play = nim_play(vec![2, 0]);

        assert!(apply_nim_control(&mut play, &Key::Character("d".into())));
        assert_eq!(play.take, 2);
        assert!(apply_nim_control(&mut play, &Key::Character("d".into())));
        assert_eq!(play.take, 2);
        assert!(apply_nim_control(&mut play, &Key::Character("a".into())));
        assert_eq!(play.take, 1);
        assert!(apply_nim_control(&mut play, &Key::Character("a".into())));
        assert_eq!(play.take, 1);
        assert!(!apply_nim_control(&mut play, &Key::Named(NamedKey::Enter)));
    }

    #[test]
    fn munch_control_maps_keyboard_and_arrows() {
        assert_eq!(
            munch_control(&Key::Character("w".into())),
            Some(MunchControl::Up)
        );
        assert_eq!(
            munch_control(&Key::Named(NamedKey::ArrowDown)),
            Some(MunchControl::Down)
        );
        assert_eq!(
            munch_control(&Key::Named(NamedKey::Space)),
            Some(MunchControl::ToggleBite)
        );
        assert_eq!(munch_control(&Key::Character("x".into())), None);
    }

    #[test]
    fn munch_cursor_wraps_like_the_core_grid() {
        let mut cursor = 0;
        let mut bites = BTreeSet::new();

        assert!(apply_munch_control(
            &mut cursor,
            &mut bites,
            &Key::Named(NamedKey::ArrowLeft)
        ));
        assert_eq!(cursor, ROWS * COLS - 1);
        assert!(apply_munch_control(
            &mut cursor,
            &mut bites,
            &Key::Character("s".into())
        ));
        assert_eq!(cursor, COLS - 1);
        assert!(apply_munch_control(
            &mut cursor,
            &mut bites,
            &Key::Character("w".into())
        ));
        assert_eq!(cursor, ROWS * COLS - 1);
    }

    #[test]
    fn munch_toggle_bites_the_current_cell() {
        let mut cursor = 7;
        let mut bites = BTreeSet::new();

        assert!(apply_munch_control(
            &mut cursor,
            &mut bites,
            &Key::Character("e".into())
        ));
        assert!(bites.contains(&7));
        assert!(apply_munch_control(
            &mut cursor,
            &mut bites,
            &Key::Named(NamedKey::Space)
        ));
        assert!(!bites.contains(&7));
        assert_eq!(cursor, 7);
    }

    #[test]
    fn direct_bite_toggle_matches_keyboard_toggling() {
        let mut bites = BTreeSet::new();
        toggle_munch_bite(&mut bites, 11);
        assert!(bites.contains(&11));
        toggle_munch_bite(&mut bites, 11);
        assert!(!bites.contains(&11));
    }
}
