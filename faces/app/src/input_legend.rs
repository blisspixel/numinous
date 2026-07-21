//! Input-aware control copy for the windowed App.
//!
//! Routing remains in the focused keyboard, pointer, and controller adapters.
//! This module is the single presentation vocabulary for those semantic
//! actions, so each screen describes the controls that actually reach it.
//! Adaptive face glyphs (Xbox / PlayStation / generic) live here so HUD copy
//! can name the buttons a player actually sees.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    KeyboardMouse,
    Controller,
}

/// Which face-button vocabulary to show for a standard controller.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ControllerFace {
    /// Semantic compass labels (SOUTH, EAST, ...). Safe default.
    #[default]
    Generic,
    /// Xbox / XInput style (A B X Y).
    Xbox,
    /// PlayStation style (cross, circle, square, triangle).
    PlayStation,
}

impl ControllerFace {
    /// Guess a face vocabulary from a controller product name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let lower = name.to_ascii_lowercase();
        if lower.contains("dualshock")
            || lower.contains("dualsense")
            || lower.contains("playstation")
            || lower.contains("sony")
            || lower.contains("ps4")
            || lower.contains("ps5")
        {
            Self::PlayStation
        } else if lower.contains("xbox")
            || lower.contains("xinput")
            || lower.contains("microsoft")
            || lower.contains("series")
            || lower.contains("360")
        {
            Self::Xbox
        } else {
            Self::Generic
        }
    }

    /// Face / system token for a semantic control on this controller family.
    #[must_use]
    pub const fn token(self, control: Control) -> &'static str {
        match (self, control) {
            (_, Control::Move) => "D-PAD",
            (_, Control::Menu) => "START",
            (_, Control::Inspect) => "SELECT",
            (_, Control::Pause) => "R3",
            (_, Control::Reset) => "L3",
            (Self::Generic, Control::Back) => "EAST",
            (Self::Generic, Control::Primary | Control::Retry) => "SOUTH",
            (Self::Generic, Control::Submit) => "NORTH",
            (Self::Xbox, Control::Back) => "B",
            (Self::Xbox, Control::Primary | Control::Retry) => "A",
            (Self::Xbox, Control::Submit) => "Y",
            (Self::PlayStation, Control::Back) => "CIRCLE",
            (Self::PlayStation, Control::Primary | Control::Retry) => "CROSS",
            (Self::PlayStation, Control::Submit) => "TRIANGLE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
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
    pub const fn token(self, control: Control) -> &'static str {
        match self {
            Self::KeyboardMouse => match control {
                Control::Back | Control::Menu => "ESC",
                Control::Inspect => "E",
                Control::Move => "WASD/ARROWS",
                Control::Pause | Control::Primary => "SPACE",
                Control::Reset => "R",
                Control::Retry | Control::Submit => "ENTER",
            },
            // Default controller legends stay generic until a face is known.
            Self::Controller => ControllerFace::Generic.token(control),
        }
    }

    /// Token for this mode, using adaptive face glyphs when on controller.
    #[must_use]
    pub const fn token_with_face(self, control: Control, face: ControllerFace) -> &'static str {
        match self {
            Self::KeyboardMouse => self.token(control),
            Self::Controller => face.token(control),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuChoice {
    Quiz,
    Munch,
    Nim,
    Gauntlet,
    Arcade,
    Show,
    Studio,
    Journey,
    WatchAgent,
}

impl MenuChoice {
    pub const ALL: [Self; 9] = [
        Self::Quiz,
        Self::Munch,
        Self::Nim,
        Self::Gauntlet,
        Self::Arcade,
        Self::Show,
        Self::Studio,
        Self::Journey,
        Self::WatchAgent,
    ];

    pub const fn at(index: usize) -> Self {
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
            Self::WatchAgent => "WATCH AGENT: LIVE MCP PLAY",
        }
    }
}

fn item(mode: InputMode, control: Control, action: &str) -> String {
    item_with_face(mode, control, action, ControllerFace::Generic)
}

fn item_with_face(mode: InputMode, control: Control, action: &str, face: ControllerFace) -> String {
    format!("{} {action}", mode.token_with_face(control, face))
}

pub fn room_action(mode: InputMode, action: &str) -> String {
    room_action_with_face(mode, action, ControllerFace::Generic)
}

/// Room action copy with adaptive controller face glyphs.
pub fn room_action_with_face(mode: InputMode, action: &str, face: ControllerFace) -> String {
    if mode == InputMode::KeyboardMouse {
        return action.to_string();
    }
    let primary = face.token(Control::Primary);
    if let Some((gesture, result)) = action.split_once(':') {
        if gesture == "AIM + CLICK" {
            return format!("LEFT STICK + {primary}: {}", result.trim_start());
        }
        if let Some(qualifier) = gesture.strip_prefix("CLICK") {
            return format!("{primary}{qualifier}: {}", result.trim_start());
        }
        if let Some(qualifier) = gesture.strip_prefix("DRAG") {
            return format!(
                "HOLD {primary} + LEFT STICK{qualifier}: {}",
                result.trim_start()
            );
        }
    }
    format!("{primary} / LEFT STICK: {action}")
}

pub fn room_inspect(mode: InputMode) -> String {
    item(mode, Control::Inspect, "INSPECT")
}

pub fn room_inspect_with_face(mode: InputMode, face: ControllerFace) -> String {
    item_with_face(mode, Control::Inspect, "INSPECT", face)
}

pub fn room_controls(mode: InputMode) -> String {
    room_controls_with_face(mode, ControllerFace::Generic)
}

/// Room chrome controls with adaptive controller face glyphs.
pub fn room_controls_with_face(mode: InputMode, face: ControllerFace) -> String {
    format!(
        "{}   {}",
        item_with_face(mode, Control::Reset, "RESET ROOM", face),
        item_with_face(mode, Control::Menu, "MENU", face)
    )
}

pub fn show_controls(mode: InputMode) -> String {
    show_controls_with_face(mode, ControllerFace::Generic)
}

/// Show-mode controls with adaptive face glyphs.
pub fn show_controls_with_face(mode: InputMode, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => "B EXIT SHOW   SPACE PAUSE".to_string(),
        InputMode::Controller => format!(
            "{} EXIT SHOW   {} PAUSE",
            face.token(Control::Back),
            face.token(Control::Pause)
        ),
    }
}

pub fn studio_controls(mode: InputMode) -> String {
    studio_controls_with_face(mode, ControllerFace::Generic)
}

/// Studio chrome with adaptive face glyphs.
pub fn studio_controls_with_face(mode: InputMode, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => "TYPE  F1 HELP  F2 RANDOM  F3 AUTO  TAB/ESC CLOSE".to_string(),
        InputMode::Controller => format!(
            "KEYBOARD TYPES   {} CLOSES   {} HELP",
            face.token(Control::Back),
            face.token(Control::Menu)
        ),
    }
}

pub fn journey_close(mode: InputMode) -> String {
    journey_close_with_face(mode, ControllerFace::Generic)
}

/// Journey close copy with adaptive face glyphs.
pub fn journey_close_with_face(mode: InputMode, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => "J CLOSES".to_string(),
        InputMode::Controller => item_with_face(mode, Control::Back, "CLOSES", face),
    }
}

pub fn pause_resume(mode: InputMode) -> String {
    pause_resume_with_face(mode, ControllerFace::Generic)
}

/// Pause resume copy with adaptive face glyphs.
pub fn pause_resume_with_face(mode: InputMode, face: ControllerFace) -> String {
    item_with_face(mode, Control::Pause, "RESUME", face)
}

pub fn quiz_result(mode: InputMode) -> String {
    quiz_result_with_face(mode, ControllerFace::Generic)
}

/// Quiz result chrome with adaptive face glyphs.
pub fn quiz_result_with_face(mode: InputMode, face: ControllerFace) -> String {
    format!(
        "{}   {}",
        item_with_face(mode, Control::Retry, "NEXT", face),
        item_with_face(mode, Control::Back, "LEAVE", face)
    )
}

pub const fn quiz_direction(mode: InputMode, index: usize) -> &'static str {
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

pub fn munch_live(mode: InputMode) -> String {
    munch_live_with_face(mode, ControllerFace::Generic)
}

/// Munch live chrome with adaptive face glyphs.
pub fn munch_live_with_face(mode: InputMode, face: ControllerFace) -> String {
    format!(
        "{}   {}   {}   {}",
        item_with_face(mode, Control::Move, "MOVE", face),
        item_with_face(mode, Control::Primary, "EAT", face),
        item_with_face(mode, Control::Submit, "DONE", face),
        item_with_face(mode, Control::Back, "LEAVE", face)
    )
}

pub fn munch_result(mode: InputMode) -> String {
    munch_result_with_face(mode, ControllerFace::Generic)
}

/// Munch result chrome with adaptive face glyphs.
pub fn munch_result_with_face(mode: InputMode, face: ControllerFace) -> String {
    format!(
        "{}   {}",
        item_with_face(mode, Control::Retry, "NEXT BOARD", face),
        item_with_face(mode, Control::Back, "LEAVE", face)
    )
}

pub fn arcade_live(mode: InputMode) -> String {
    arcade_live_with_face(mode, ControllerFace::Generic)
}

/// Arcade live chrome with adaptive face glyphs.
pub fn arcade_live_with_face(mode: InputMode, face: ControllerFace) -> String {
    format!(
        "{}   {}   DON'T BE CAUGHT   {}",
        item_with_face(mode, Control::Move, "RUN", face),
        item_with_face(mode, Control::Primary, "EAT", face),
        item_with_face(mode, Control::Back, "LEAVE", face)
    )
}

pub fn arcade_over(mode: InputMode) -> String {
    arcade_over_with_face(mode, ControllerFace::Generic)
}

/// Arcade over chrome with adaptive face glyphs.
pub fn arcade_over_with_face(mode: InputMode, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => "ANY KEY LEAVES".to_string(),
        InputMode::Controller => item_with_face(mode, Control::Retry, "LEAVES", face),
    }
}

pub fn nim_live(mode: InputMode, take: u32) -> String {
    nim_live_with_face(mode, take, ControllerFace::Generic)
}

/// Nim live chrome with adaptive face glyphs.
pub fn nim_live_with_face(mode: InputMode, take: u32, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => {
            format!("W/S HEAP   A/D TAKE {take}   ENTER TAKE   ESC LEAVE")
        }
        InputMode::Controller => format!(
            "D-PAD U/D HEAP   D-PAD L/R TAKE {take}   {} TAKE   {} LEAVE",
            face.token(Control::Primary),
            face.token(Control::Back)
        ),
    }
}

pub fn nim_result(mode: InputMode) -> String {
    nim_result_with_face(mode, ControllerFace::Generic)
}

/// Nim result chrome with adaptive face glyphs.
pub fn nim_result_with_face(mode: InputMode, face: ControllerFace) -> String {
    format!(
        "{}   {}",
        item_with_face(mode, Control::Retry, "RETRY", face),
        item_with_face(mode, Control::Back, "LEAVE", face)
    )
}

pub fn gauntlet_choice(mode: InputMode) -> String {
    match mode {
        InputMode::KeyboardMouse => "PRESS THE LETTER".to_string(),
        InputMode::Controller => "D-PAD: UP A   RIGHT B   DOWN C   LEFT D".to_string(),
    }
}

pub fn gauntlet_bomb(mode: InputMode) -> String {
    gauntlet_bomb_with_face(mode, ControllerFace::Generic)
}

/// Gauntlet bomb chrome with adaptive face glyphs.
pub fn gauntlet_bomb_with_face(mode: InputMode, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => "TYPE DIGITS   ENTER CUTS   BACKSPACE FIXES".to_string(),
        InputMode::Controller => format!(
            "UP/DOWN DIGIT   {} ADD   LEFT FIX   {} CUT",
            face.token(Control::Primary),
            face.token(Control::Submit)
        ),
    }
}

pub fn gauntlet_done(mode: InputMode) -> String {
    gauntlet_done_with_face(mode, ControllerFace::Generic)
}

/// Gauntlet done chrome with adaptive face glyphs.
pub fn gauntlet_done_with_face(mode: InputMode, face: ControllerFace) -> String {
    match mode {
        InputMode::KeyboardMouse => "ANY KEY LEAVES".to_string(),
        InputMode::Controller => item_with_face(mode, Control::Retry, "LEAVES", face),
    }
}

pub fn help_lines(mode: InputMode, selected: Option<usize>, activity_paused: bool) -> Vec<String> {
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
            "J          JOURNEY    F  FULLSCREEN    X  WATCH AGENT",
            "Y          RADIO    P  POSTCARD   L  LOOP   K  SHARE PACK",
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

pub fn compact_controller_help_lines(selected: usize) -> Vec<String> {
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
        format!(
            "{} WATCH AGENT   EAST BACK",
            if selected == 8 { ">" } else { " " }
        ),
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
    fn adaptive_face_glyphs_name_xbox_and_playstation_buttons() {
        assert_eq!(
            ControllerFace::from_name("Xbox Series Controller"),
            ControllerFace::Xbox
        );
        assert_eq!(
            ControllerFace::from_name("DualSense Wireless Controller"),
            ControllerFace::PlayStation
        );
        assert_eq!(
            ControllerFace::from_name("Generic pad"),
            ControllerFace::Generic
        );
        assert_eq!(ControllerFace::Xbox.token(Control::Primary), "A");
        assert_eq!(ControllerFace::PlayStation.token(Control::Primary), "CROSS");
        assert_eq!(ControllerFace::Generic.token(Control::Primary), "SOUTH");
        assert_eq!(
            InputMode::Controller.token_with_face(Control::Back, ControllerFace::Xbox),
            "B"
        );
        assert_eq!(
            InputMode::Controller.token_with_face(Control::Back, ControllerFace::PlayStation),
            "CIRCLE"
        );
        // Keyboard mode ignores face.
        assert_eq!(
            InputMode::KeyboardMouse.token_with_face(Control::Primary, ControllerFace::Xbox),
            "SPACE"
        );
        // Game chrome carries face tokens for Xbox and PlayStation.
        let xbox_munch = munch_live_with_face(InputMode::Controller, ControllerFace::Xbox);
        assert!(xbox_munch.contains("A EAT"), "{xbox_munch}");
        assert!(xbox_munch.contains("B LEAVE"), "{xbox_munch}");
        let ps_arcade = arcade_live_with_face(InputMode::Controller, ControllerFace::PlayStation);
        assert!(ps_arcade.contains("CROSS EAT"), "{ps_arcade}");
        assert!(ps_arcade.contains("CIRCLE LEAVE"), "{ps_arcade}");
        let xbox_nim = nim_live_with_face(InputMode::Controller, 2, ControllerFace::Xbox);
        assert!(xbox_nim.contains("A TAKE"), "{xbox_nim}");
    }

    /// Certification roster: known product-name fragments map to face families.
    #[test]
    fn controller_cert_matrix_covers_common_pads() {
        let cases = [
            ("Xbox 360 Controller", ControllerFace::Xbox),
            ("Xbox One Controller", ControllerFace::Xbox),
            ("Xbox Series X Controller", ControllerFace::Xbox),
            ("Microsoft X-Box pad", ControllerFace::Xbox),
            ("DualShock 4", ControllerFace::PlayStation),
            ("DualSense Wireless Controller", ControllerFace::PlayStation),
            (
                "Sony Interactive Entertainment Controller",
                ControllerFace::PlayStation,
            ),
            ("Wireless Controller", ControllerFace::Generic),
            ("8BitDo Pro 2", ControllerFace::Generic),
            ("Logitech F310", ControllerFace::Generic),
        ];
        for (name, expected) in cases {
            assert_eq!(ControllerFace::from_name(name), expected, "pad name {name}");
        }
    }

    #[test]
    fn every_controller_menu_choice_has_one_stable_index() {
        assert_eq!(MenuChoice::ALL.len(), 9);
        for (index, expected) in MenuChoice::ALL.into_iter().enumerate() {
            assert_eq!(MenuChoice::at(index), expected);
            assert!(!expected.controller_label().is_empty());
        }
        assert_eq!(MenuChoice::at(9), MenuChoice::Quiz);
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
