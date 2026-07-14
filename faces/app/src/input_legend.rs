//! Input-aware control copy for the windowed App.
//!
//! Routing remains in the focused keyboard, pointer, and controller adapters.
//! This module is the single presentation vocabulary for those semantic
//! actions, so each screen describes the controls that actually reach it.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMode {
    #[default]
    KeyboardMouse,
    Controller,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    Back,
    Inspect,
    Menu,
    Move,
    Pause,
    Primary,
    Reset,
    Retry,
    Submit,
}

impl InputMode {
    pub(crate) const fn token(self, control: Control) -> &'static str {
        match (self, control) {
            (Self::KeyboardMouse, Control::Back | Control::Menu) => "ESC",
            (Self::KeyboardMouse, Control::Inspect) => "E",
            (Self::KeyboardMouse, Control::Move) => "WASD/ARROWS",
            (Self::KeyboardMouse, Control::Pause | Control::Primary) => "SPACE",
            (Self::KeyboardMouse, Control::Reset) => "R",
            (Self::KeyboardMouse, Control::Retry | Control::Submit) => "ENTER",
            (Self::Controller, Control::Back) => "EAST",
            (Self::Controller, Control::Inspect) => "SELECT",
            (Self::Controller, Control::Menu) => "START",
            (Self::Controller, Control::Move) => "D-PAD",
            (Self::Controller, Control::Pause) => "R3",
            (Self::Controller, Control::Primary | Control::Retry) => "SOUTH",
            (Self::Controller, Control::Reset) => "L3",
            (Self::Controller, Control::Submit) => "NORTH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuChoice {
    Quiz,
    Munch,
    Nim,
    Gauntlet,
    Arcade,
    Show,
    Studio,
    Journey,
}

impl MenuChoice {
    pub(crate) const ALL: [Self; 8] = [
        Self::Quiz,
        Self::Munch,
        Self::Nim,
        Self::Gauntlet,
        Self::Arcade,
        Self::Show,
        Self::Studio,
        Self::Journey,
    ];

    pub(crate) const fn at(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    const fn controller_label(self) -> &'static str {
        match self {
            Self::Quiz => "THE QUIZ: NAME THE MATH",
            Self::Munch => "MUNCH: EAT WHAT FITS",
            Self::Nim => "NIM: BEAT THE ORDER",
            Self::Gauntlet => "THE GAUNTLET: ONE RUN",
            Self::Arcade => "THE ARCADE: EAT WHILE HUNTED",
            Self::Show => "THE SHOW: LET THE WORLD WANDER",
            Self::Studio => "THE STUDIO: TYPE A CURVE",
            Self::Journey => "THE JOURNEY: WHAT PLAY MADE",
        }
    }
}

fn item(mode: InputMode, control: Control, action: &str) -> String {
    format!("{} {action}", mode.token(control))
}

pub(crate) fn room_action(mode: InputMode, action: &str) -> String {
    if mode == InputMode::KeyboardMouse {
        return action.to_string();
    }
    if let Some((gesture, result)) = action.split_once(':') {
        if gesture == "AIM + CLICK" {
            return format!("LEFT STICK + SOUTH: {}", result.trim_start());
        }
        if let Some(qualifier) = gesture.strip_prefix("CLICK") {
            return format!("SOUTH{qualifier}: {}", result.trim_start());
        }
        if let Some(qualifier) = gesture.strip_prefix("DRAG") {
            return format!(
                "HOLD SOUTH + LEFT STICK{qualifier}: {}",
                result.trim_start()
            );
        }
    }
    format!("SOUTH / LEFT STICK: {action}")
}

pub(crate) fn room_inspect(mode: InputMode) -> String {
    item(mode, Control::Inspect, "INSPECT")
}

pub(crate) fn room_controls(mode: InputMode) -> String {
    format!(
        "{}   {}",
        item(mode, Control::Reset, "RESET ROOM"),
        item(mode, Control::Menu, "MENU")
    )
}

pub(crate) fn show_controls(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "B EXIT SHOW   SPACE PAUSE".to_string(),
        InputMode::Controller => "EAST EXIT SHOW   R3 PAUSE".to_string(),
    }
}

pub(crate) fn studio_controls(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "TYPE A FORMULA   TAB / ESC CLOSE".to_string(),
        InputMode::Controller => "KEYBOARD TYPES   EAST CLOSES   START HELP".to_string(),
    }
}

pub(crate) fn journey_close(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "J CLOSES".to_string(),
        InputMode::Controller => item(mode, Control::Back, "CLOSES"),
    }
}

pub(crate) fn pause_resume(mode: InputMode) -> String {
    item(mode, Control::Pause, "RESUME")
}

pub(crate) fn quiz_result(mode: InputMode) -> String {
    format!(
        "{}   {}",
        item(mode, Control::Retry, "NEXT"),
        item(mode, Control::Back, "LEAVE")
    )
}

pub(crate) const fn quiz_direction(mode: InputMode, index: usize) -> &'static str {
    match mode {
        InputMode::KeyboardMouse => "",
        InputMode::Controller => match index {
            0 => "UP",
            1 => "RIGHT",
            2 => "DOWN",
            _ => "LEFT",
        },
    }
}

pub(crate) fn munch_live(mode: InputMode) -> String {
    format!(
        "{}   {}   {}   {}",
        item(mode, Control::Move, "MOVE"),
        item(mode, Control::Primary, "EAT"),
        item(mode, Control::Submit, "DONE"),
        item(mode, Control::Back, "LEAVE")
    )
}

pub(crate) fn munch_result(mode: InputMode) -> String {
    format!(
        "{}   {}",
        item(mode, Control::Retry, "NEXT BOARD"),
        item(mode, Control::Back, "LEAVE")
    )
}

pub(crate) fn arcade_live(mode: InputMode) -> String {
    format!(
        "{}   {}   DON'T BE CAUGHT   {}",
        item(mode, Control::Move, "RUN"),
        item(mode, Control::Primary, "EAT"),
        item(mode, Control::Back, "LEAVE")
    )
}

pub(crate) fn arcade_over(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "ANY KEY LEAVES".to_string(),
        InputMode::Controller => item(mode, Control::Retry, "LEAVES"),
    }
}

pub(crate) fn nim_live(mode: InputMode, take: u32) -> String {
    match mode {
        InputMode::KeyboardMouse => {
            format!("W/S HEAP   A/D TAKE {take}   ENTER TAKE   ESC LEAVE")
        }
        InputMode::Controller => {
            format!("D-PAD U/D HEAP   D-PAD L/R TAKE {take}   SOUTH TAKE   EAST LEAVE")
        }
    }
}

pub(crate) fn nim_result(mode: InputMode) -> String {
    format!(
        "{}   {}",
        item(mode, Control::Retry, "RETRY"),
        item(mode, Control::Back, "LEAVE")
    )
}

pub(crate) fn gauntlet_choice(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "PRESS THE LETTER".to_string(),
        InputMode::Controller => "D-PAD: UP A   RIGHT B   DOWN C   LEFT D".to_string(),
    }
}

pub(crate) fn gauntlet_bomb(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "TYPE DIGITS   ENTER CUTS   BACKSPACE FIXES".to_string(),
        InputMode::Controller => "UP/DOWN DIGIT   SOUTH ADD   LEFT FIX   NORTH CUT".to_string(),
    }
}

pub(crate) fn gauntlet_done(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "ANY KEY LEAVES".to_string(),
        InputMode::Controller => item(mode, Control::Retry, "LEAVES"),
    }
}

pub(crate) fn help_lines(
    mode: InputMode,
    selected: Option<usize>,
    activity_paused: bool,
) -> Vec<String> {
    if activity_paused {
        let resume = match mode {
            InputMode::KeyboardMouse => "ESC RETURNS",
            InputMode::Controller => "SOUTH / START / EAST RETURN",
        };
        return ["ACTIVITY PAUSED", resume, "THE CURRENT RUN STAYS INTACT"]
            .into_iter()
            .map(str::to_string)
            .collect();
    }
    match mode {
        InputMode::KeyboardMouse => [
            "PLAY (PRESS A LETTER)",
            "G          THE QUIZ: NAME THE MATH",
            "C          MUNCH: EAT WHAT FITS",
            "N          NIM: BEAT THE ORDER",
            "T          THE GAUNTLET: ONE RUN",
            "V          THE ARCADE: EAT WHILE HUNTED",
            "",
            "WANDER",
            "A / D      PREV / NEXT ROOM    1-9 JUMP",
            "W / S      TIME SPEED   MOUSE  SCRUB",
            "E          INSPECT    Q  ERA    R  RESET",
            "B          THE SHOW   TAB  THE STUDIO",
            "J          JOURNEY    F  FULLSCREEN",
            "Y          RADIO    P  POSTCARD",
            "F9         PLAYTEST NOTE",
            "M          MUTE    [/] VOLUME   SPACE PAUSE",
            "",
            "ESC        CLOSE MENU AND WANDER",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        InputMode::Controller => {
            let mut lines = vec!["PLAY / EXPLORE   D-PAD CHOOSE   SOUTH OPEN".to_string()];
            for (index, choice) in MenuChoice::ALL.into_iter().enumerate() {
                let marker = if selected == Some(index) { "> " } else { "  " };
                lines.push(format!("{marker}{}", choice.controller_label()));
            }
            lines.extend(
                [
                    "",
                    "WANDER",
                    "LEFT STICK HAND   SOUTH TOUCH / HOLD",
                    "LB / RB ROOMS   LT / RT SPEED   RIGHT STICK TIME",
                    "SELECT INSPECT   L3 RESET   R3 PAUSE",
                    "WEST ERA   NORTH RADIO",
                    "HOLD NORTH + D-PAD VOLUME   + SOUTH MUTE",
                    "START MENU   EAST BACK",
                ]
                .into_iter()
                .map(str::to_string),
            );
            lines
        }
    }
}

pub(crate) fn compact_controller_help_lines(selected: usize) -> Vec<String> {
    let pair = |left_index: usize, left: &str, right_index: usize, right: &str| -> String {
        format!(
            "{} {:<9} {} {}",
            if selected == left_index { ">" } else { " " },
            left,
            if selected == right_index { ">" } else { " " },
            right
        )
    };
    vec![
        "D-PAD CHOOSE   SOUTH OPEN".to_string(),
        pair(0, "QUIZ", 1, "MUNCH"),
        pair(2, "NIM", 3, "GAUNTLET"),
        pair(4, "ARCADE", 5, "SHOW"),
        pair(6, "STUDIO", 7, "JOURNEY"),
        "LB/RB ROOMS  LT/RT SPEED".to_string(),
        "STICK+SOUTH TOUCH SEL INFO".to_string(),
        "HOLD NORTH: D-PAD VOL/S MUTE".to_string(),
        "L3 RESET R3 PAUSE START MENU".to_string(),
        "EAST BACK    NORTH RADIO".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_copy_names_only_routed_controller_tokens() {
        let copy = [
            room_controls(InputMode::Controller),
            quiz_result(InputMode::Controller),
            munch_live(InputMode::Controller),
            munch_result(InputMode::Controller),
            arcade_live(InputMode::Controller),
            arcade_over(InputMode::Controller),
            nim_live(InputMode::Controller, 3),
            nim_result(InputMode::Controller),
            gauntlet_choice(InputMode::Controller),
            gauntlet_bomb(InputMode::Controller),
            gauntlet_done(InputMode::Controller),
        ]
        .join("\n");
        for keyboard_only in [
            "WASD",
            "ARROWS",
            "ENTER",
            "ESC",
            "SPACE",
            "BACKSPACE",
            "TAB",
            "ANY KEY",
        ] {
            assert!(
                !copy.contains(keyboard_only),
                "leaked {keyboard_only}: {copy}"
            );
        }
    }

    #[test]
    fn every_controller_menu_choice_has_one_stable_index() {
        assert_eq!(MenuChoice::ALL.len(), 8);
        for (index, expected) in MenuChoice::ALL.into_iter().enumerate() {
            assert_eq!(MenuChoice::at(index), expected);
            assert!(!expected.controller_label().is_empty());
        }
        assert_eq!(MenuChoice::at(8), MenuChoice::Quiz);
        assert_eq!(
            help_lines(InputMode::Controller, None, true),
            [
                "ACTIVITY PAUSED",
                "SOUTH / START / EAST RETURN",
                "THE CURRENT RUN STAYS INTACT"
            ]
        );
    }

    #[test]
    fn room_actions_translate_without_losing_the_domain_copy() {
        assert_eq!(
            room_action(InputMode::Controller, "CLICK: plant a glider"),
            "SOUTH: plant a glider"
        );
        assert_eq!(
            room_action(InputMode::Controller, "DRAG: comb the curve"),
            "HOLD SOUTH + LEFT STICK: comb the curve"
        );
        assert_eq!(
            room_action(InputMode::KeyboardMouse, "CLICK: plant a glider"),
            "CLICK: plant a glider"
        );
        assert_eq!(
            room_action(
                InputMode::Controller,
                "AIM + CLICK: pick coin, drop 64 balls"
            ),
            "LEFT STICK + SOUTH: pick coin, drop 64 balls"
        );
        assert_eq!(
            room_action(
                InputMode::Controller,
                "CLICK LEFT OR RIGHT: bias and drop a ball"
            ),
            "SOUTH LEFT OR RIGHT: bias and drop a ball"
        );
    }

    #[test]
    fn every_catalog_room_controller_action_is_device_truthful() {
        for room in numinous_core::all_rooms() {
            let keyboard = numinous_core::room_touch_action(room.as_ref());
            let controller = room_action(InputMode::Controller, keyboard);
            for forbidden in ["CLICK", "DRAG", "MOUSE", "ENTER", "ESC", "SPACE"] {
                assert!(
                    !controller.contains(forbidden),
                    "{} leaked {forbidden}: {controller}",
                    room.meta().id
                );
            }
            let domain = keyboard
                .split_once(':')
                .map_or(keyboard, |(_, result)| result)
                .trim();
            assert!(
                controller.contains(domain),
                "{} lost its domain action: {controller}",
                room.meta().id
            );
        }
    }
}
