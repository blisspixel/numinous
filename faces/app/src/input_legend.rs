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
#[non_exhaustive]
#[repr(u8)]
pub enum ControllerButton {
    South,
    East,
    North,
    West,
    Start,
    Select,
    LeftThumb,
    RightThumb,
    LeftTrigger,
    RightTrigger,
    LeftTrigger2,
    RightTrigger2,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

impl ControllerButton {
    pub(crate) const ALL: [Self; 16] = [
        Self::South,
        Self::East,
        Self::North,
        Self::West,
        Self::Start,
        Self::Select,
        Self::LeftThumb,
        Self::RightThumb,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::LeftTrigger2,
        Self::RightTrigger2,
        Self::DPadUp,
        Self::DPadDown,
        Self::DPadLeft,
        Self::DPadRight,
    ];

    const fn token(self, face: ControllerFace) -> &'static str {
        match (self, face) {
            (Self::South, ControllerFace::Generic) => "SOUTH",
            (Self::East, ControllerFace::Generic) => "EAST",
            (Self::North, ControllerFace::Generic) => "NORTH",
            (Self::West, ControllerFace::Generic) => "WEST",
            (Self::South, ControllerFace::Xbox) => "A",
            (Self::East, ControllerFace::Xbox) => "B",
            (Self::North, ControllerFace::Xbox) => "Y",
            (Self::West, ControllerFace::Xbox) => "X",
            (Self::South, ControllerFace::PlayStation) => "CROSS",
            (Self::East, ControllerFace::PlayStation) => "CIRCLE",
            (Self::North, ControllerFace::PlayStation) => "TRIANGLE",
            (Self::West, ControllerFace::PlayStation) => "SQUARE",
            (Self::Start, _) => "START",
            (Self::Select, _) => "SELECT",
            (Self::LeftThumb, _) => "L3",
            (Self::RightThumb, _) => "R3",
            (Self::LeftTrigger, ControllerFace::PlayStation) => "L1",
            (Self::RightTrigger, ControllerFace::PlayStation) => "R1",
            (Self::LeftTrigger, _) => "LB",
            (Self::RightTrigger, _) => "RB",
            (Self::LeftTrigger2, ControllerFace::PlayStation) => "L2",
            (Self::RightTrigger2, ControllerFace::PlayStation) => "R2",
            (Self::LeftTrigger2, _) => "LT",
            (Self::RightTrigger2, _) => "RT",
            (Self::DPadUp, _) => "UP",
            (Self::DPadDown, _) => "DOWN",
            (Self::DPadLeft, _) => "LEFT",
            (Self::DPadRight, _) => "RIGHT",
        }
    }

    const fn bit(self) -> u16 {
        1 << self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[repr(u8)]
pub enum ControllerAction {
    Primary,
    Back,
    Menu,
    Inspect,
    Reset,
    PreviousRoom,
    NextRoom,
    Slower,
    Faster,
    Up,
    Down,
    Left,
    Right,
    CycleEra,
    CycleRadio,
    ToggleMute,
    VolumeDown,
    VolumeUp,
    Pause,
}

impl ControllerAction {
    const COUNT: usize = 19;

    const fn index(self) -> usize {
        self as usize
    }
}

/// Immutable controller vocabulary derived from one effective routing table.
///
/// A standard vocabulary keeps the established controller-family labels used
/// by deterministic screenshots. A mapped vocabulary names the buttons that
/// the current App instance actually routes. Private fields prevent consumers
/// from constructing a partially initialized mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerCopy {
    face: ControllerFace,
    actions: Option<[u16; ControllerAction::COUNT]>,
    default_audio_chord: bool,
}

impl Default for ControllerCopy {
    fn default() -> Self {
        ControllerFace::Generic.into()
    }
}

impl From<ControllerFace> for ControllerCopy {
    fn from(face: ControllerFace) -> Self {
        Self {
            face,
            actions: None,
            default_audio_chord: true,
        }
    }
}

impl ControllerCopy {
    pub const fn empty(face: ControllerFace) -> Self {
        Self {
            face,
            actions: Some([0; ControllerAction::COUNT]),
            default_audio_chord: false,
        }
    }

    pub fn bind(&mut self, action: ControllerAction, button: ControllerButton) {
        let Some(actions) = &mut self.actions else {
            return;
        };
        actions[action.index()] |= button.bit();
    }

    pub fn enable_default_audio_chord(&mut self) {
        if self.actions.is_none() {
            return;
        }
        self.default_audio_chord = true;
        self.bind(ControllerAction::CycleRadio, ControllerButton::North);
    }

    #[must_use]
    pub const fn uses_default_audio_chord(self) -> bool {
        self.default_audio_chord
    }

    fn face(self) -> ControllerFace {
        self.face
    }

    fn mapped_buttons(self, action: ControllerAction) -> u16 {
        self.actions.map_or(0, |actions| actions[action.index()])
    }

    fn default_action_token(self, action: ControllerAction) -> &'static str {
        let face = self.face();
        match action {
            ControllerAction::Primary => face.token(Control::Primary),
            ControllerAction::Back => face.token(Control::Back),
            ControllerAction::Menu => "START",
            ControllerAction::Inspect => "SELECT",
            ControllerAction::Reset => "L3",
            ControllerAction::PreviousRoom => match face {
                ControllerFace::PlayStation => "L1",
                _ => "LB",
            },
            ControllerAction::NextRoom => match face {
                ControllerFace::PlayStation => "R1",
                _ => "RB",
            },
            ControllerAction::Slower => match face {
                ControllerFace::PlayStation => "L2",
                _ => "LT",
            },
            ControllerAction::Faster => match face {
                ControllerFace::PlayStation => "R2",
                _ => "RT",
            },
            ControllerAction::Up => "UP",
            ControllerAction::Down => "DOWN",
            ControllerAction::Left => "LEFT",
            ControllerAction::Right => "RIGHT",
            ControllerAction::CycleEra => ControllerButton::West.token(face),
            ControllerAction::CycleRadio => ControllerButton::North.token(face),
            ControllerAction::ToggleMute => "NORTH+SOUTH",
            ControllerAction::VolumeDown => "NORTH+DOWN",
            ControllerAction::VolumeUp => "NORTH+UP",
            ControllerAction::Pause => "R3",
        }
    }

    pub fn action_token(self, action: ControllerAction) -> String {
        if self.actions.is_none() {
            return self.default_action_token(action).to_string();
        }
        let buttons = self.mapped_buttons(action);
        let count = buttons.count_ones();
        let Some(first) = ControllerButton::ALL
            .into_iter()
            .find(|button| buttons & button.bit() != 0)
        else {
            return "UNBOUND".to_string();
        };
        let token = first.token(self.face());
        if count == 1 {
            token.to_string()
        } else {
            format!("{token}/+{}", count - 1)
        }
    }

    /// Token for one screen-level semantic control.
    #[must_use]
    pub fn token(self, control: Control) -> String {
        let action = match control {
            Control::Back => ControllerAction::Back,
            Control::Inspect => ControllerAction::Inspect,
            Control::Menu => ControllerAction::Menu,
            Control::Pause => ControllerAction::Pause,
            Control::Primary | Control::Retry => ControllerAction::Primary,
            Control::Reset => ControllerAction::Reset,
            Control::Submit => ControllerAction::CycleRadio,
            Control::Move => return self.move_token(),
        };
        self.action_token(action)
    }

    fn move_token(self) -> String {
        if self.actions.is_none()
            || self.mapped_buttons(ControllerAction::Up) == ControllerButton::DPadUp.bit()
                && self.mapped_buttons(ControllerAction::Right) == ControllerButton::DPadRight.bit()
                && self.mapped_buttons(ControllerAction::Down) == ControllerButton::DPadDown.bit()
                && self.mapped_buttons(ControllerAction::Left) == ControllerButton::DPadLeft.bit()
        {
            "D-PAD".to_string()
        } else {
            "CUSTOM".to_string()
        }
    }

    #[must_use]
    pub fn direction_summary(self) -> String {
        if self.move_token() == "D-PAD" {
            return "D-PAD".to_string();
        }
        format!(
            "U:{} R:{} D:{} L:{}",
            self.action_token(ControllerAction::Up),
            self.action_token(ControllerAction::Right),
            self.action_token(ControllerAction::Down),
            self.action_token(ControllerAction::Left)
        )
    }

    fn compact_action_token(self, action: ControllerAction) -> String {
        compact_token(&self.action_token(action))
    }

    fn compact_control_token(self, control: Control) -> String {
        compact_token(&self.token(control))
    }

    fn compact_direction_summary(self) -> String {
        if self.move_token() == "D-PAD" {
            return "D-PAD".to_string();
        }
        format!(
            "U{} R{} D{} L{}",
            self.compact_action_token(ControllerAction::Up),
            self.compact_action_token(ControllerAction::Right),
            self.compact_action_token(ControllerAction::Down),
            self.compact_action_token(ControllerAction::Left)
        )
    }
}

fn compact_token(token: &str) -> String {
    let (first, suffix) = token
        .split_once('/')
        .map_or((token, ""), |(first, suffix)| (first, suffix));
    let first = match first {
        "SOUTH" => "S",
        "EAST" => "E",
        "NORTH" => "N",
        "WEST" => "W",
        "CROSS" => "CR",
        "CIRCLE" => "CI",
        "SQUARE" => "SQ",
        "TRIANGLE" => "TR",
        "SELECT" => "SEL",
        "START" => "ST",
        "UNBOUND" => "NONE",
        other => other,
    };
    if suffix.is_empty() {
        first.to_string()
    } else {
        format!("{first}/{suffix}")
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

    const fn keyboard_token(self) -> &'static str {
        match self {
            Self::Quiz => "G",
            Self::Munch => "C",
            Self::Nim => "N",
            Self::Gauntlet => "T",
            Self::Arcade => "V",
            Self::Show => "B/ENTER",
            Self::Studio => "TAB",
            Self::Journey => "J",
            Self::WatchAgent => "X",
        }
    }

    const fn menu_label(self) -> &'static str {
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
    item_with_controller(mode, control, action, face.into())
}

fn item_with_controller(
    mode: InputMode,
    control: Control,
    action: &str,
    copy: ControllerCopy,
) -> String {
    let token = match mode {
        InputMode::KeyboardMouse => mode.token(control).to_string(),
        InputMode::Controller => copy.token(control),
    };
    format!("{token} {action}")
}

pub fn room_action(mode: InputMode, action: &str) -> String {
    room_action_with_face(mode, action, ControllerFace::Generic)
}

/// Room action copy with adaptive controller face glyphs.
pub fn room_action_with_face(mode: InputMode, action: &str, face: ControllerFace) -> String {
    room_action_with_controller(mode, action, face.into())
}

/// Room action copy using the effective player mapping.
pub fn room_action_with_controller(mode: InputMode, action: &str, copy: ControllerCopy) -> String {
    if mode == InputMode::KeyboardMouse {
        return action.to_string();
    }
    let primary = copy.token(Control::Primary);
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
    item(mode, Control::Inspect, "EXPLAIN")
}

/// Room explain control with adaptive face glyphs (concept + reveal).
pub fn room_inspect_with_face(mode: InputMode, face: ControllerFace) -> String {
    room_inspect_with_controller(mode, face.into())
}

pub fn room_inspect_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    item_with_controller(mode, Control::Inspect, "EXPLAIN", copy)
}

pub fn room_controls(mode: InputMode) -> String {
    room_controls_with_face(mode, ControllerFace::Generic)
}

/// Room chrome controls with adaptive controller face glyphs.
pub fn room_controls_with_face(mode: InputMode, face: ControllerFace) -> String {
    room_controls_with_controller(mode, face.into())
}

pub fn room_controls_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    format!(
        "{}   {}",
        item_with_controller(mode, Control::Reset, "RESET ROOM", copy),
        item_with_controller(mode, Control::Menu, "MENU", copy)
    )
}

pub fn show_controls(mode: InputMode) -> String {
    show_controls_with_face(mode, ControllerFace::Generic)
}

/// Show-mode controls with adaptive face glyphs.
pub fn show_controls_with_face(mode: InputMode, face: ControllerFace) -> String {
    show_controls_with_controller(mode, face.into())
}

pub fn show_controls_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "B EXIT SHOW   SPACE PAUSE".to_string(),
        InputMode::Controller => format!(
            "{} EXIT SHOW   {} PAUSE",
            copy.token(Control::Back),
            copy.token(Control::Pause)
        ),
    }
}

pub fn studio_controls(mode: InputMode) -> String {
    studio_controls_with_face(mode, ControllerFace::Generic)
}

/// Studio chrome with adaptive face glyphs.
pub fn studio_controls_with_face(mode: InputMode, face: ControllerFace) -> String {
    studio_controls_with_controller(mode, face.into())
}

pub fn studio_controls_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "TYPE  F1 HELP  F2 RANDOM  F3 AUTO  TAB/ESC CLOSE".to_string(),
        InputMode::Controller => format!(
            "KEYBOARD TYPES   {} CLOSES   {} HELP",
            copy.token(Control::Back),
            copy.token(Control::Menu)
        ),
    }
}

pub fn journey_close(mode: InputMode) -> String {
    journey_close_with_face(mode, ControllerFace::Generic)
}

/// Journey close copy with adaptive face glyphs.
pub fn journey_close_with_face(mode: InputMode, face: ControllerFace) -> String {
    journey_close_with_controller(mode, face.into())
}

pub fn journey_close_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "J CLOSES".to_string(),
        InputMode::Controller => item_with_controller(mode, Control::Back, "CLOSES", copy),
    }
}

pub fn pause_resume(mode: InputMode) -> String {
    pause_resume_with_face(mode, ControllerFace::Generic)
}

/// Pause resume copy with adaptive face glyphs.
pub fn pause_resume_with_face(mode: InputMode, face: ControllerFace) -> String {
    pause_resume_with_controller(mode, face.into())
}

pub fn pause_resume_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    item_with_controller(mode, Control::Pause, "RESUME", copy)
}

pub fn quiz_result(mode: InputMode) -> String {
    quiz_result_with_face(mode, ControllerFace::Generic)
}

/// Quiz result chrome with adaptive face glyphs.
pub fn quiz_result_with_face(mode: InputMode, face: ControllerFace) -> String {
    quiz_result_with_controller(mode, face.into())
}

pub fn quiz_result_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    format!(
        "{}   {}",
        item_with_controller(mode, Control::Retry, "NEXT", copy),
        item_with_controller(mode, Control::Back, "LEAVE", copy)
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

pub fn quiz_direction_with_controller(
    mode: InputMode,
    index: usize,
    copy: ControllerCopy,
) -> String {
    if mode == InputMode::KeyboardMouse {
        return String::new();
    }
    let action = match index {
        0 => ControllerAction::Up,
        1 => ControllerAction::Right,
        2 => ControllerAction::Down,
        _ => ControllerAction::Left,
    };
    copy.action_token(action)
}

pub fn munch_live(mode: InputMode) -> String {
    munch_live_with_face(mode, ControllerFace::Generic)
}

/// Munch live chrome with adaptive face glyphs.
pub fn munch_live_with_face(mode: InputMode, face: ControllerFace) -> String {
    munch_live_with_controller(mode, face.into())
}

pub fn munch_live_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    format!(
        "{}   {}   {}   {}",
        item_with_controller(mode, Control::Move, "MOVE", copy),
        item_with_controller(mode, Control::Primary, "EAT", copy),
        item_with_controller(mode, Control::Submit, "DONE", copy),
        item_with_controller(mode, Control::Back, "LEAVE", copy)
    )
}

pub fn munch_result(mode: InputMode) -> String {
    munch_result_with_face(mode, ControllerFace::Generic)
}

/// Munch result chrome with adaptive face glyphs.
pub fn munch_result_with_face(mode: InputMode, face: ControllerFace) -> String {
    munch_result_with_controller(mode, face.into())
}

pub fn munch_result_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    format!(
        "{}   {}",
        item_with_controller(mode, Control::Retry, "NEXT BOARD", copy),
        item_with_controller(mode, Control::Back, "LEAVE", copy)
    )
}

pub fn arcade_live(mode: InputMode) -> String {
    arcade_live_with_face(mode, ControllerFace::Generic)
}

/// Arcade live chrome with adaptive face glyphs.
pub fn arcade_live_with_face(mode: InputMode, face: ControllerFace) -> String {
    arcade_live_with_controller(mode, face.into())
}

pub fn arcade_live_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    format!(
        "{}   {}   DON'T BE CAUGHT   {}",
        item_with_controller(mode, Control::Move, "RUN", copy),
        item_with_controller(mode, Control::Primary, "EAT", copy),
        item_with_controller(mode, Control::Back, "LEAVE", copy)
    )
}

pub fn arcade_over(mode: InputMode) -> String {
    arcade_over_with_face(mode, ControllerFace::Generic)
}

/// Arcade over chrome with adaptive face glyphs.
pub fn arcade_over_with_face(mode: InputMode, face: ControllerFace) -> String {
    arcade_over_with_controller(mode, face.into())
}

pub fn arcade_over_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "ANY KEY LEAVES".to_string(),
        InputMode::Controller => item_with_controller(mode, Control::Retry, "LEAVES", copy),
    }
}

pub fn nim_live(mode: InputMode, take: u32) -> String {
    nim_live_with_face(mode, take, ControllerFace::Generic)
}

/// Nim live chrome with adaptive face glyphs.
pub fn nim_live_with_face(mode: InputMode, take: u32, face: ControllerFace) -> String {
    nim_live_with_controller(mode, take, face.into())
}

pub fn nim_live_with_controller(mode: InputMode, take: u32, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => {
            format!("W/S HEAP   A/D TAKE {take}   ENTER TAKE   ESC LEAVE")
        }
        InputMode::Controller => format!(
            "{} HEAP/TAKE {take}   {} TAKE   {} LEAVE",
            copy.direction_summary(),
            copy.token(Control::Primary),
            copy.token(Control::Back)
        ),
    }
}

pub fn nim_result(mode: InputMode) -> String {
    nim_result_with_face(mode, ControllerFace::Generic)
}

/// Nim result chrome with adaptive face glyphs.
pub fn nim_result_with_face(mode: InputMode, face: ControllerFace) -> String {
    nim_result_with_controller(mode, face.into())
}

pub fn nim_result_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    format!(
        "{}   {}",
        item_with_controller(mode, Control::Retry, "RETRY", copy),
        item_with_controller(mode, Control::Back, "LEAVE", copy)
    )
}

pub fn gauntlet_choice(mode: InputMode) -> String {
    gauntlet_choice_with_controller(mode, ControllerFace::Generic.into())
}

pub fn gauntlet_choice_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "PRESS THE LETTER".to_string(),
        InputMode::Controller => format!(
            "{} A   {} B   {} C   {} D",
            copy.action_token(ControllerAction::Up),
            copy.action_token(ControllerAction::Right),
            copy.action_token(ControllerAction::Down),
            copy.action_token(ControllerAction::Left)
        ),
    }
}

pub fn gauntlet_bomb(mode: InputMode) -> String {
    gauntlet_bomb_with_face(mode, ControllerFace::Generic)
}

/// Gauntlet bomb chrome with adaptive face glyphs.
pub fn gauntlet_bomb_with_face(mode: InputMode, face: ControllerFace) -> String {
    gauntlet_bomb_with_controller(mode, face.into())
}

pub fn gauntlet_bomb_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "TYPE DIGITS   ENTER CUTS   BACKSPACE FIXES".to_string(),
        InputMode::Controller => format!(
            "{}/{} DIGIT   {} ADD   {} FIX   {} CUT",
            copy.action_token(ControllerAction::Up),
            copy.action_token(ControllerAction::Down),
            copy.token(Control::Primary),
            copy.action_token(ControllerAction::Left),
            copy.token(Control::Submit)
        ),
    }
}

pub fn gauntlet_done(mode: InputMode) -> String {
    gauntlet_done_with_face(mode, ControllerFace::Generic)
}

/// Gauntlet done chrome with adaptive face glyphs.
pub fn gauntlet_done_with_face(mode: InputMode, face: ControllerFace) -> String {
    gauntlet_done_with_controller(mode, face.into())
}

pub fn gauntlet_done_with_controller(mode: InputMode, copy: ControllerCopy) -> String {
    match mode {
        InputMode::KeyboardMouse => "ANY KEY LEAVES".to_string(),
        InputMode::Controller => item_with_controller(mode, Control::Retry, "LEAVES", copy),
    }
}

pub fn help_lines(mode: InputMode, selected: Option<usize>, activity_paused: bool) -> Vec<String> {
    help_lines_with_controller(
        mode,
        selected,
        activity_paused,
        ControllerFace::Generic.into(),
    )
}

pub fn help_lines_with_controller(
    mode: InputMode,
    selected: Option<usize>,
    activity_paused: bool,
    copy: ControllerCopy,
) -> Vec<String> {
    if activity_paused {
        let resume = match mode {
            InputMode::KeyboardMouse => "ESC RETURNS".to_string(),
            InputMode::Controller => format!(
                "{} / {} / {} RETURN",
                copy.token(Control::Primary),
                copy.token(Control::Menu),
                copy.token(Control::Back)
            ),
        };
        return vec![
            "ACTIVITY PAUSED".to_string(),
            resume,
            "THE CURRENT RUN STAYS INTACT".to_string(),
        ];
    }
    match mode {
        InputMode::KeyboardMouse => {
            let mut lines = vec!["PLAY / EXPLORE   CLICK OR PRESS THE KEY".to_string()];
            for (index, choice) in MenuChoice::ALL.into_iter().enumerate() {
                let marker = if selected == Some(index) { ">" } else { " " };
                lines.push(format!(
                    "{marker} {:<3} {}",
                    choice.keyboard_token(),
                    choice.menu_label()
                ));
            }
            lines.extend(
                [
                    "",
                    "WANDER",
                    "A / D      PREV / NEXT ROOM    1-9 JUMP",
                    "W / S      TIME SPEED   MOUSE SCRUB",
                    "E / ?      EXPLAIN    Q ERA    R RESET",
                    "U          CALL THE READOUT BEFORE YOU LOOK",
                    "F          FULLSCREEN    Y RADIO",
                    "P / L / K  POSTCARD / LOOP / SHARE PACK",
                    "F9         PLAYTEST NOTE",
                    "M          MUTE    [/] VOLUME   SPACE PAUSE",
                    "` / ~      POWER CONSOLE (room, list, t, vary)",
                    "",
                    "ESC        CLOSE MENU AND WANDER",
                ]
                .into_iter()
                .map(str::to_string),
            );
            lines
        }
        InputMode::Controller => {
            let mut lines = vec![format!(
                "PLAY / EXPLORE   {} CHOOSE   {} OPEN",
                copy.direction_summary(),
                copy.token(Control::Primary)
            )];
            for (index, choice) in MenuChoice::ALL.into_iter().enumerate() {
                let marker = if selected == Some(index) { "> " } else { "  " };
                lines.push(format!("{marker}{}", choice.menu_label()));
            }
            lines.push(String::new());
            lines.push("WANDER".to_string());
            lines.push(format!(
                "LEFT STICK HAND   {} TOUCH / HOLD",
                copy.token(Control::Primary)
            ));
            lines.push(format!(
                "{} / {} ROOMS   {} / {} SPEED   RIGHT STICK TIME",
                copy.action_token(ControllerAction::PreviousRoom),
                copy.action_token(ControllerAction::NextRoom),
                copy.action_token(ControllerAction::Slower),
                copy.action_token(ControllerAction::Faster)
            ));
            lines.push(format!(
                "{} EXPLAIN   {} RESET   {} PAUSE",
                copy.token(Control::Inspect),
                copy.token(Control::Reset),
                copy.token(Control::Pause)
            ));
            lines.push(format!(
                "{} ERA   {} RADIO",
                copy.action_token(ControllerAction::CycleEra),
                copy.action_token(ControllerAction::CycleRadio)
            ));
            if copy.uses_default_audio_chord() {
                lines.push(format!(
                    "HOLD {} + D-PAD VOLUME   + {} MUTE",
                    copy.action_token(ControllerAction::CycleRadio),
                    copy.token(Control::Primary)
                ));
            } else {
                lines.push(format!(
                    "{}/{} VOLUME   {} MUTE",
                    copy.action_token(ControllerAction::VolumeDown),
                    copy.action_token(ControllerAction::VolumeUp),
                    copy.action_token(ControllerAction::ToggleMute)
                ));
            }
            lines.push(format!(
                "{} MENU   {} BACK",
                copy.token(Control::Menu),
                copy.token(Control::Back)
            ));
            lines
        }
    }
}

pub fn compact_controller_help_lines(selected: usize) -> Vec<String> {
    compact_controller_help_lines_with_controller(selected, ControllerFace::Generic.into())
}

pub fn compact_controller_help_lines_with_controller(
    selected: usize,
    copy: ControllerCopy,
) -> Vec<String> {
    let pair = |left_index: usize, left: &str, right_index: usize, right: &str| -> String {
        format!(
            "{} {:<9} {} {}",
            if selected == left_index { ">" } else { " " },
            left,
            if selected == right_index { ">" } else { " " },
            right
        )
    };
    let audio = if copy.uses_default_audio_chord() {
        format!(
            "HOLD {}: D-PAD VOL/{} MUTE",
            copy.compact_action_token(ControllerAction::CycleRadio),
            copy.compact_control_token(Control::Primary)
        )
    } else {
        format!(
            "{}/{} VOL  {} MUTE",
            copy.compact_action_token(ControllerAction::VolumeDown),
            copy.compact_action_token(ControllerAction::VolumeUp),
            copy.compact_action_token(ControllerAction::ToggleMute)
        )
    };
    vec![
        format!(
            "{}   {} OPEN",
            copy.compact_direction_summary(),
            copy.compact_control_token(Control::Primary)
        ),
        pair(0, "QUIZ", 1, "MUNCH"),
        pair(2, "NIM", 3, "GAUNTLET"),
        pair(4, "ARCADE", 5, "SHOW"),
        pair(6, "STUDIO", 7, "JOURNEY"),
        format!(
            "{}/{} ROOMS  {}/{} SPEED",
            copy.compact_action_token(ControllerAction::PreviousRoom),
            copy.compact_action_token(ControllerAction::NextRoom),
            copy.compact_action_token(ControllerAction::Slower),
            copy.compact_action_token(ControllerAction::Faster)
        ),
        format!(
            "STICK+{} TOUCH {} INFO",
            copy.compact_control_token(Control::Primary),
            copy.compact_control_token(Control::Inspect)
        ),
        audio,
        format!(
            "{} RESET {} PAUSE {} MENU",
            copy.compact_control_token(Control::Reset),
            copy.compact_control_token(Control::Pause),
            copy.compact_control_token(Control::Menu)
        ),
        format!(
            "{} WATCH AGENT   {} BACK",
            if selected == 8 { ">" } else { " " },
            copy.compact_control_token(Control::Back)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_help_offers_enter_for_the_show() {
        let lines = help_lines(InputMode::KeyboardMouse, None, false).join("\n");
        assert!(
            lines.contains("ENTER") && lines.contains("THE SHOW"),
            "front-door Enter must start The Show:\n{lines}"
        );
    }

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

    #[test]
    fn mapping_aware_copy_names_remapped_and_unbound_actions() {
        let mut copy = ControllerCopy::empty(ControllerFace::Xbox);
        copy.bind(ControllerAction::Primary, ControllerButton::West);
        copy.bind(ControllerAction::Back, ControllerButton::South);
        copy.bind(ControllerAction::Menu, ControllerButton::Select);
        copy.bind(ControllerAction::Pause, ControllerButton::East);
        copy.bind(ControllerAction::Up, ControllerButton::North);
        copy.bind(ControllerAction::Right, ControllerButton::DPadUp);
        copy.bind(ControllerAction::Down, ControllerButton::DPadRight);
        copy.bind(ControllerAction::Left, ControllerButton::DPadDown);

        assert_eq!(copy.token(Control::Primary), "X");
        assert_eq!(copy.token(Control::Back), "A");
        assert_eq!(copy.token(Control::Inspect), "UNBOUND");
        assert_eq!(copy.token(Control::Move), "CUSTOM");
        assert_eq!(copy.direction_summary(), "U:Y R:UP D:RIGHT L:DOWN");
        assert_eq!(
            room_action_with_controller(InputMode::Controller, "CLICK: plant", copy),
            "X: plant"
        );
        assert_eq!(
            show_controls_with_controller(InputMode::Controller, copy),
            "A EXIT SHOW   B PAUSE"
        );
    }

    #[test]
    fn mapping_aware_help_preserves_keyboard_copy_and_reports_custom_routes() {
        let keyboard = help_lines(InputMode::KeyboardMouse, Some(2), false);
        let mut copy = ControllerCopy::empty(ControllerFace::PlayStation);
        copy.bind(ControllerAction::Primary, ControllerButton::West);
        copy.bind(ControllerAction::Back, ControllerButton::South);
        copy.bind(ControllerAction::Menu, ControllerButton::Select);
        copy.bind(ControllerAction::Inspect, ControllerButton::Start);
        copy.bind(ControllerAction::Pause, ControllerButton::East);
        copy.bind(ControllerAction::Reset, ControllerButton::RightThumb);
        copy.bind(
            ControllerAction::PreviousRoom,
            ControllerButton::LeftTrigger2,
        );
        copy.bind(ControllerAction::NextRoom, ControllerButton::RightTrigger2);
        copy.bind(ControllerAction::Slower, ControllerButton::LeftTrigger);
        copy.bind(ControllerAction::Faster, ControllerButton::RightTrigger);
        copy.bind(ControllerAction::CycleEra, ControllerButton::North);
        copy.bind(ControllerAction::CycleRadio, ControllerButton::LeftThumb);
        copy.bind(ControllerAction::ToggleMute, ControllerButton::DPadLeft);
        copy.bind(ControllerAction::VolumeDown, ControllerButton::DPadDown);
        copy.bind(ControllerAction::VolumeUp, ControllerButton::DPadUp);
        copy.bind(ControllerAction::Up, ControllerButton::North);
        copy.bind(ControllerAction::Right, ControllerButton::East);
        copy.bind(ControllerAction::Down, ControllerButton::South);
        copy.bind(ControllerAction::Left, ControllerButton::West);

        let lines = help_lines_with_controller(InputMode::Controller, Some(2), false, copy);
        let joined = lines.join("\n");
        assert!(joined.contains("SQUARE TOUCH / HOLD"), "{joined}");
        assert!(joined.contains("L2 / R2 ROOMS"), "{joined}");
        assert!(joined.contains("L1 / R1 SPEED"), "{joined}");
        assert!(joined.contains("START EXPLAIN"), "{joined}");
        assert!(joined.contains("R3 RESET"), "{joined}");
        assert!(joined.contains("CIRCLE PAUSE"), "{joined}");
        assert!(joined.contains("TRIANGLE ERA"), "{joined}");
        assert!(joined.contains("L3 RADIO"), "{joined}");
        assert!(joined.contains("DOWN/UP VOLUME"), "{joined}");
        assert!(joined.contains("LEFT MUTE"), "{joined}");
        assert!(joined.contains("SELECT MENU"), "{joined}");
        assert!(joined.contains("CROSS BACK"), "{joined}");
        assert_eq!(
            keyboard,
            help_lines_with_controller(
                InputMode::KeyboardMouse,
                Some(2),
                false,
                ControllerFace::Xbox.into()
            )
        );
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
            assert!(!expected.menu_label().is_empty());
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
    fn keyboard_mouse_menu_exposes_every_clickable_destination() {
        for (index, choice) in MenuChoice::ALL.into_iter().enumerate() {
            let lines = help_lines(InputMode::KeyboardMouse, Some(index), false);
            assert_eq!(lines.len(), 23);
            assert!(
                lines[index + 1].starts_with('>'),
                "selected destination {index} must be visible"
            );
            assert!(lines[index + 1].contains(choice.menu_label()));
            assert_eq!(lines.iter().filter(|line| line.starts_with('>')).count(), 1);
        }
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
