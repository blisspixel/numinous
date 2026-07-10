#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeftPressAction {
    GameClick,
    RoomPoke,
    PhaseDrag,
    Ignore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PointerState {
    pub(crate) dragging: bool,
    pub(crate) poking: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct LeftPressContext {
    pub(crate) game_click_mode: bool,
    pub(crate) studio: bool,
    pub(crate) show_help: bool,
    pub(crate) show_journey: bool,
    pub(crate) arcade: bool,
    pub(crate) nim: bool,
    pub(crate) gauntlet: bool,
    pub(crate) room_has_verb: bool,
}

pub(crate) fn left_press_action(context: LeftPressContext) -> LeftPressAction {
    if context.game_click_mode {
        return LeftPressAction::GameClick;
    }
    if blocked_by_modal_context(context) {
        return LeftPressAction::Ignore;
    }
    if context.room_has_verb {
        return LeftPressAction::RoomPoke;
    }
    if phase_drag_allowed(context) {
        LeftPressAction::PhaseDrag
    } else {
        LeftPressAction::Ignore
    }
}

pub(crate) fn pointer_state_after_left_press(action: LeftPressAction) -> PointerState {
    match action {
        LeftPressAction::RoomPoke => PointerState {
            dragging: false,
            poking: true,
        },
        LeftPressAction::PhaseDrag => PointerState {
            dragging: true,
            poking: false,
        },
        LeftPressAction::GameClick | LeftPressAction::Ignore => PointerState::default(),
    }
}

pub(crate) fn pointer_state_after_left_release() -> PointerState {
    PointerState::default()
}

pub(crate) fn retain_pointer_state(state: PointerState, context: LeftPressContext) -> PointerState {
    PointerState {
        dragging: state.dragging && phase_drag_allowed(context),
        poking: state.poking && room_poke_allowed(context),
    }
}

pub(crate) fn room_poke_allowed(context: LeftPressContext) -> bool {
    !context.studio
        && !context.show_help
        && !context.show_journey
        && !context.arcade
        && !context.nim
        && !context.gauntlet
        && context.room_has_verb
}

pub(crate) fn phase_drag_allowed(context: LeftPressContext) -> bool {
    !context.game_click_mode && !blocked_by_modal_context(context) && !context.room_has_verb
}

fn blocked_by_modal_context(context: LeftPressContext) -> bool {
    context.studio
        || context.show_help
        || context.show_journey
        || context.arcade
        || context.nim
        || context.gauntlet
}

pub(crate) fn normalized_window_point(mouse: (f64, f64), size: (u32, u32)) -> Option<(f64, f64)> {
    if size.0 == 0 || size.1 == 0 {
        return None;
    }
    Some((mouse.0 / f64::from(size.0), mouse.1 / f64::from(size.1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poke_context() -> LeftPressContext {
        LeftPressContext {
            room_has_verb: true,
            ..LeftPressContext::default()
        }
    }

    #[test]
    fn game_clicks_take_priority_over_room_pokes() {
        let context = LeftPressContext {
            game_click_mode: true,
            room_has_verb: true,
            ..LeftPressContext::default()
        };

        assert_eq!(left_press_action(context), LeftPressAction::GameClick);
    }

    #[test]
    fn poke_requires_visible_room_interaction_context() {
        assert_eq!(left_press_action(poke_context()), LeftPressAction::RoomPoke);

        for blocked in [
            LeftPressContext {
                studio: true,
                ..poke_context()
            },
            LeftPressContext {
                show_help: true,
                ..poke_context()
            },
            LeftPressContext {
                show_journey: true,
                ..poke_context()
            },
            LeftPressContext {
                arcade: true,
                ..poke_context()
            },
            LeftPressContext {
                nim: true,
                ..poke_context()
            },
            LeftPressContext {
                gauntlet: true,
                ..poke_context()
            },
        ] {
            assert_eq!(left_press_action(blocked), LeftPressAction::Ignore);
        }
    }

    #[test]
    fn quiet_rooms_can_still_scrub_phase() {
        assert_eq!(
            left_press_action(LeftPressContext::default()),
            LeftPressAction::PhaseDrag
        );
    }

    #[test]
    fn normalized_points_are_window_relative_and_zero_safe() {
        assert_eq!(
            normalized_window_point((25.0, 75.0), (100, 300)),
            Some((0.25, 0.25))
        );
        assert_eq!(normalized_window_point((2.0, 3.0), (0, 0)), None);
    }

    #[test]
    fn pointer_press_and_release_updates_state() {
        assert_eq!(
            pointer_state_after_left_press(LeftPressAction::RoomPoke),
            PointerState {
                dragging: false,
                poking: true
            }
        );
        assert_eq!(
            pointer_state_after_left_press(LeftPressAction::PhaseDrag),
            PointerState {
                dragging: true,
                poking: false
            }
        );
        assert_eq!(
            pointer_state_after_left_press(LeftPressAction::GameClick),
            PointerState::default()
        );
        assert_eq!(pointer_state_after_left_release(), PointerState::default());
    }

    #[test]
    fn pointer_state_is_cleared_when_context_no_longer_allows_it() {
        let poking = PointerState {
            dragging: false,
            poking: true,
        };
        assert_eq!(retain_pointer_state(poking, poke_context()), poking);
        assert_eq!(
            retain_pointer_state(
                poking,
                LeftPressContext {
                    show_help: true,
                    ..poke_context()
                }
            ),
            PointerState::default()
        );

        let dragging = PointerState {
            dragging: true,
            poking: false,
        };
        assert_eq!(
            retain_pointer_state(dragging, LeftPressContext::default()),
            dragging
        );
        assert_eq!(
            retain_pointer_state(
                dragging,
                LeftPressContext {
                    studio: true,
                    ..LeftPressContext::default()
                }
            ),
            PointerState::default()
        );
    }
}
