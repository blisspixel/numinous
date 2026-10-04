//! The App's small, spatial Cabinet menu.
//!
//! Menu state, navigation, geometry, hit testing, and drawing live together so
//! every input family acts on the same visible targets. The App owns the
//! resulting intents and remains the only place that changes game state.

use crate::{
    input_legend::{Control, ControllerCopy, InputMode, MenuChoice},
    menu_font,
};
use numinous_core::{Raster, Surface};

const COMPACT_WIDTH: usize = 600;
const COMPACT_HEIGHT: usize = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOrigin {
    Launch,
    Room,
    Activity(ActivityKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Quiz,
    Munch,
    Nim,
    Gauntlet,
    Arcade,
    Studio,
    SharedPlay,
    Route,
}

impl ActivityKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Quiz => "THE QUIZ",
            Self::Munch => "MUNCH",
            Self::Nim => "NIM",
            Self::Gauntlet => "THE GAUNTLET",
            Self::Arcade => "THE ARCADE",
            Self::Studio => "THE STUDIO",
            Self::SharedPlay => "SHARED PLAY",
            Self::Route => "ROUTE LAB",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuRoute {
    Home,
    Modes,
    Games,
    Settings,
    Controls,
    Wings,
    Pause(ActivityKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuItemId {
    Modes,
    Explain,
    Experiment,
    Construct,
    Watch,
    Play,
    Create,
    Games,
    Journey,
    SharedPlay,
    Settings,
    Quiz,
    Munch,
    Nim,
    Gauntlet,
    Arcade,
    MasterVolume,
    MusicVolume,
    RoomVolume,
    EffectVolume,
    Mute,
    VisualEra,
    WindowMode,
    StudyTextSize,
    SkipTrack,
    Controls,
    /// One wing of the catalog, by its position in the shared wing list.
    Wing(usize),
    /// The authored walk through several rooms.
    Walk,
    /// The one astonishing room the threshold opens with.
    Touch,
    /// The one kept question, when a project chain is present.
    Project,
    Resume,
    Restart,
    LeaveActivity,
    Quit,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuIntent {
    None,
    ExplainRoom,
    ChooseExperiment,
    ConstructRoom,
    Close,
    Choose(MenuChoice),
    /// Step one numeric Settings row down or up.
    Adjust(NumericSetting, Step),
    ToggleMute,
    CycleEra,
    CycleWindowMode,
    SkipRadioTrack,
    ToggleFullscreen,
    Quit,
    ResumeActivity,
    RestartActivity(ActivityKind),
    LeaveActivity(ActivityKind),
    /// Wander one wing, by its position in the shared wing list.
    EnterWing(usize),
    /// Follow the authored walk, in its own order, carrying its questions.
    EnterWalk,
    /// Go straight to the one room the threshold opens with.
    TouchTheFlagship,
    /// Open the one kept question. The preview does not write.
    OpenKeptProject,
    /// Leave the chosen wing and let the arrows reach the whole catalog again.
    LeaveWing,
}

/// A Settings value the player sets by stepping left and right.
///
/// A row whose action names one of these is a numeric row: left and right step
/// it, and activating it steps it up, so every input family reaches the same
/// value the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericSetting {
    /// The master level every sound passes through.
    MasterVolume,
    /// The radio, beneath master.
    MusicVolume,
    /// Room sound beneath master: the score, its voices and events, Studio,
    /// and Shared Play replay.
    RoomVolume,
    /// Game cues, beneath master.
    EffectVolume,
    /// The optional study reader's body text size.
    StudyTextSize,
}

impl NumericSetting {
    /// The row's title, also used by the App's level feedback.
    #[must_use]
    pub fn title(self) -> &'static str {
        SETTINGS_ITEMS
            .iter()
            .find(|item| item.action == MenuAction::Adjust(self))
            .map_or("", |item| item.title)
    }
}

/// Which way a numeric row moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Down,
    Up,
}

impl Step {
    /// The step as a sign, for settings measured on a number line.
    #[must_use]
    pub const fn sign(self) -> f32 {
        match self {
            Self::Down => -1.0,
            Self::Up => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuAction {
    Intent(MenuIntent),
    /// A numeric row. Activation steps it up; left and right step either way.
    Adjust(NumericSetting),
    Open(MenuRoute),
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuItem {
    id: MenuItemId,
    title: &'static str,
    description: &'static str,
    shortcut: Option<char>,
    action: MenuAction,
}

const HOME_ITEMS: [MenuItem; 6] = [
    MenuItem {
        id: MenuItemId::Modes,
        title: "MODES",
        description: "WATCH, RETURN, CREATE, OR OPEN A SHARED EXPERIENCE.",
        shortcut: Some('m'),
        action: MenuAction::Open(MenuRoute::Modes),
    },
    MenuItem {
        id: MenuItemId::Explain,
        title: "EXPLAIN",
        description: "THIS ROOM, FROM A SHORT NOTE TO THE MATHEMATICS.",
        shortcut: Some('e'),
        action: MenuAction::Intent(MenuIntent::ExplainRoom),
    },
    MenuItem {
        id: MenuItemId::Games,
        title: "GAMES",
        description: "FIVE SHORT GAMES OF PATTERN, NUMBER, AND NERVE.",
        shortcut: Some('g'),
        action: MenuAction::Open(MenuRoute::Games),
    },
    MenuItem {
        id: MenuItemId::Settings,
        title: "SETTINGS",
        description: "SOUND AND DISPLAY SETTINGS.",
        shortcut: Some('s'),
        action: MenuAction::Open(MenuRoute::Settings),
    },
    MenuItem {
        id: MenuItemId::Controls,
        title: "CONTROLS",
        description: "THE KEYS AND BUTTONS FOR THE WAY YOU ARE PLAYING.",
        shortcut: Some('c'),
        action: MenuAction::Open(MenuRoute::Controls),
    },
    MenuItem {
        id: MenuItemId::Quit,
        title: "QUIT",
        description: "CLOSE NUMINOUS AND KEEP THIS JOURNEY.",
        shortcut: Some('q'),
        action: MenuAction::Intent(MenuIntent::Quit),
    },
];

const MODE_ITEMS: [MenuItem; 7] = [
    MenuItem {
        id: MenuItemId::Watch,
        title: "WATCH",
        description: "LET THE CABINET WANDER. YOU CAN STEP IN AT ANY TIME.",
        shortcut: Some('w'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Show)),
    },
    MenuItem {
        id: MenuItemId::Play,
        title: "PLAY",
        description: "RETURN TO THE ROOM THAT IS WAITING FOR YOU.",
        shortcut: Some('p'),
        action: MenuAction::Intent(MenuIntent::Close),
    },
    MenuItem {
        id: MenuItemId::Create,
        title: "CREATE",
        description: "SHAPE A CURVE, HEAR IT SING, AND KEEP WHAT YOU MAKE.",
        shortcut: Some('c'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Studio)),
    },
    MenuItem {
        id: MenuItemId::Journey,
        title: "JOURNEY",
        description: "SEE WHAT PLAY HAS LEFT IN THIS LOCAL JOURNEY.",
        shortcut: Some('j'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Journey)),
    },
    MenuItem {
        id: MenuItemId::SharedPlay,
        title: "SHARED PLAY",
        description: "WATCH A PAIRED PLAYER'S PUBLIC ACTIONS, WITH THEIR CONSENT.",
        shortcut: Some('x'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::WatchAgent)),
    },
    MenuItem {
        id: MenuItemId::Controls,
        title: "WINGS",
        description: "CHOOSE A WING AND WANDER IT. THE CABINET HAS HUNDREDS OF ROOMS.",
        shortcut: Some('n'),
        action: MenuAction::Open(MenuRoute::Wings),
    },
    MenuItem {
        id: MenuItemId::Back,
        title: "BACK",
        description: "RETURN TO THE MAIN MENU.",
        shortcut: None,
        action: MenuAction::Back,
    },
];

const GAME_ITEMS: [MenuItem; 6] = [
    MenuItem {
        id: MenuItemId::Quiz,
        title: "THE QUIZ",
        description: "NAME THE MATHEMATICS THAT MADE THE ROOM.",
        shortcut: None,
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Quiz)),
    },
    MenuItem {
        id: MenuItemId::Munch,
        title: "MUNCH",
        description: "EAT EVERY NUMBER THAT FITS THE RULE.",
        shortcut: Some('m'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Munch)),
    },
    MenuItem {
        id: MenuItemId::Nim,
        title: "NIM",
        description: "TAKE THE LAST TOKEN BEFORE THE ORDER DOES.",
        shortcut: Some('n'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Nim)),
    },
    MenuItem {
        id: MenuItemId::Gauntlet,
        title: "THE GAUNTLET",
        description: "FOUR TRIALS. ONE RUN. KEEP YOUR NERVE.",
        shortcut: Some('g'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Gauntlet)),
    },
    MenuItem {
        id: MenuItemId::Arcade,
        title: "THE ARCADE",
        description: "EAT WHAT FITS WHILE THE VEXATIONS HUNT.",
        shortcut: Some('a'),
        action: MenuAction::Intent(MenuIntent::Choose(MenuChoice::Arcade)),
    },
    MenuItem {
        id: MenuItemId::Back,
        title: "BACK",
        description: "RETURN TO THE CABINET.",
        shortcut: None,
        action: MenuAction::Back,
    },
];

const SETTINGS_ITEMS: [MenuItem; 10] = [
    MenuItem {
        id: MenuItemId::MasterVolume,
        title: "MASTER",
        description: "PRESS LEFT OR RIGHT TO SET THE CABINET'S MASTER VOLUME.",
        shortcut: None,
        action: MenuAction::Adjust(NumericSetting::MasterVolume),
    },
    MenuItem {
        id: MenuItemId::MusicVolume,
        title: "RADIO",
        description: "PRESS LEFT OR RIGHT TO SET THE RADIO BENEATH THE MASTER VOLUME.",
        shortcut: None,
        action: MenuAction::Adjust(NumericSetting::MusicVolume),
    },
    MenuItem {
        id: MenuItemId::RoomVolume,
        title: "ROOM SOUND",
        description: "PRESS LEFT OR RIGHT TO SET EACH ROOM'S MUSIC AND VOICES, STUDIO, AND SHARED PLAY.",
        shortcut: None,
        action: MenuAction::Adjust(NumericSetting::RoomVolume),
    },
    MenuItem {
        id: MenuItemId::EffectVolume,
        title: "EFFECTS",
        description: "PRESS LEFT OR RIGHT TO SET THE GAME CUES BENEATH THE MASTER VOLUME.",
        shortcut: None,
        action: MenuAction::Adjust(NumericSetting::EffectVolume),
    },
    MenuItem {
        id: MenuItemId::Mute,
        title: "MUTE",
        description: "TURN ALL CABINET SOUND OFF OR ON.",
        shortcut: Some('m'),
        action: MenuAction::Intent(MenuIntent::ToggleMute),
    },
    MenuItem {
        id: MenuItemId::VisualEra,
        title: "VISUAL ERA",
        description: "CYCLE PHOSPHOR, 8-BIT, VECTOR, AND MODERN LOOKS.",
        shortcut: Some('e'),
        action: MenuAction::Intent(MenuIntent::CycleEra),
    },
    MenuItem {
        id: MenuItemId::WindowMode,
        title: "WINDOW MODE",
        description: "ENTER CYCLES DISPLAY MODES. F TOGGLES FULLSCREEN.",
        shortcut: None,
        action: MenuAction::Intent(MenuIntent::CycleWindowMode),
    },
    MenuItem {
        id: MenuItemId::SkipTrack,
        title: "SKIP TRACK",
        description: "PLAY THE NEXT CACHED TRACK ON THE CURRENT RADIO STATION.",
        shortcut: Some('n'),
        action: MenuAction::Intent(MenuIntent::SkipRadioTrack),
    },
    MenuItem {
        id: MenuItemId::StudyTextSize,
        title: "READING TEXT",
        description: "SET THE STUDY READER'S BODY TEXT SIZE. ITS WORDS REFLOW TO FIT.",
        shortcut: None,
        action: MenuAction::Adjust(NumericSetting::StudyTextSize),
    },
    MenuItem {
        id: MenuItemId::Back,
        title: "BACK",
        description: "RETURN TO THE PREVIOUS MENU.",
        shortcut: None,
        action: MenuAction::Back,
    },
];

const CONTROLS_ITEMS: [MenuItem; 1] = [MenuItem {
    id: MenuItemId::Back,
    title: "BACK",
    description: "RETURN TO OPTIONS.",
    shortcut: None,
    action: MenuAction::Back,
}];

fn pause_items(kind: ActivityKind) -> Vec<MenuItem> {
    let mut pause = vec![MenuItem {
        id: MenuItemId::Resume,
        title: "RESUME",
        description: "RETURN TO THE CURRENT RUN.",
        shortcut: Some('r'),
        action: MenuAction::Intent(MenuIntent::ResumeActivity),
    }];
    if !matches!(kind, ActivityKind::Studio | ActivityKind::SharedPlay) {
        pause.push(MenuItem {
            id: MenuItemId::Restart,
            title: "RESTART RUN",
            description: "BEGIN THIS ACTIVITY AGAIN FROM ITS FIRST MOVE.",
            shortcut: None,
            action: MenuAction::Intent(MenuIntent::RestartActivity(kind)),
        });
    }
    pause.extend([
        MenuItem {
            id: MenuItemId::Controls,
            title: "CONTROLS",
            description: "SEE ONLY THE CONTROLS FOR THIS ACTIVITY.",
            shortcut: Some('c'),
            action: MenuAction::Open(MenuRoute::Controls),
        },
        MenuItem {
            id: MenuItemId::Settings,
            title: "OPTIONS",
            description: "ADJUST SOUND AND DISPLAY WITHOUT ENDING THE RUN.",
            shortcut: Some('o'),
            action: MenuAction::Open(MenuRoute::Settings),
        },
        MenuItem {
            id: MenuItemId::LeaveActivity,
            title: "LEAVE ACTIVITY",
            description: "END THIS RUN AND RETURN TO THE ROOM.",
            shortcut: None,
            action: MenuAction::Intent(MenuIntent::LeaveActivity(kind)),
        },
    ]);
    pause
}

/// One entry per wing, plus a way out of the wing you are in.
///
/// Wing names are static catalog data, so this needs no owned strings. The
/// list is the shared core reading, which is the same one the protocol face
/// offers as its wander door, so the two faces cannot disagree about what
/// wings exist.
fn wing_items() -> Vec<MenuItem> {
    let mut entries = vec![
        MenuItem {
            id: MenuItemId::Touch,
            title: "TOUCH ONE ASTONISHING THING",
            description: "TURN ONE DIAL AND WATCH MULTIPLICATION DRAW A LIVING CURVE.",
            shortcut: None,
            action: MenuAction::Intent(MenuIntent::TouchTheFlagship),
        },
        MenuItem {
            id: MenuItemId::Walk,
            title: numinous_core::STRANGE_LOOP_WALK.title,
            description: numinous_core::STRANGE_LOOP_WALK.invitation,
            shortcut: None,
            action: MenuAction::Intent(MenuIntent::EnterWalk),
        },
    ];
    let wings: Vec<MenuItem> = numinous_core::wings()
        .into_iter()
        .enumerate()
        .map(|(index, wing)| MenuItem {
            id: MenuItemId::Wing(index),
            title: wing.name,
            description: "WANDER THIS WING. THE ARROWS STAY INSIDE IT.",
            shortcut: None,
            action: MenuAction::Intent(MenuIntent::EnterWing(index)),
        })
        .collect();
    entries.extend(wings);
    entries.push(MenuItem {
        id: MenuItemId::Back,
        title: "THE WHOLE CABINET",
        description: "LEAVE THIS WING. THE ARROWS REACH EVERY ROOM AGAIN.",
        shortcut: None,
        action: MenuAction::Intent(MenuIntent::LeaveWing),
    });
    entries
}

fn state_items(state: &MenuState) -> Vec<MenuItem> {
    items(
        state.route(),
        state.experiment_available,
        state.construct_available,
        state.project_available,
    )
}

fn items(
    route: MenuRoute,
    experiment_available: bool,
    construct_available: bool,
    project_available: bool,
) -> Vec<MenuItem> {
    match route {
        MenuRoute::Home => {
            let mut entries = HOME_ITEMS.to_vec();
            let mut insert_at = 2;
            if experiment_available {
                entries.insert(
                    insert_at,
                    MenuItem {
                        id: MenuItemId::Experiment,
                        title: "EXPERIMENT",
                        description: "CHOOSE THIS ROOM'S PREDICTION OR OBSERVATION PATH.",
                        shortcut: Some('u'),
                        action: MenuAction::Intent(MenuIntent::ChooseExperiment),
                    },
                );
                insert_at += 1;
            }
            if construct_available {
                entries.insert(
                    insert_at,
                    MenuItem {
                        id: MenuItemId::Construct,
                        title: "CONSTRUCT",
                        description: "CREATE WITH THIS ROOM'S MATHEMATICS. OPTIONAL.",
                        shortcut: Some('o'),
                        action: MenuAction::Intent(MenuIntent::ConstructRoom),
                    },
                );
                insert_at += 1;
            }
            if project_available {
                entries.insert(
                    insert_at,
                    MenuItem {
                        id: MenuItemId::Project,
                        title: "THE QUESTION",
                        description: "THE KEPT QUESTION. A PRESENT CREATION WAITS PAUSED.",
                        shortcut: Some('k'),
                        action: MenuAction::Intent(MenuIntent::OpenKeptProject),
                    },
                );
            }
            entries
        }
        MenuRoute::Modes => MODE_ITEMS.to_vec(),
        MenuRoute::Games => GAME_ITEMS.to_vec(),
        MenuRoute::Settings => SETTINGS_ITEMS.to_vec(),
        MenuRoute::Controls => CONTROLS_ITEMS.to_vec(),
        MenuRoute::Wings => wing_items(),
        MenuRoute::Pause(kind) => pause_items(kind),
    }
}

fn default_focus(route: MenuRoute, origin: MenuOrigin) -> MenuItemId {
    match route {
        MenuRoute::Home => MenuItemId::Modes,
        MenuRoute::Modes if origin == MenuOrigin::Launch => MenuItemId::Watch,
        MenuRoute::Modes => MenuItemId::Play,
        MenuRoute::Games => MenuItemId::Quiz,
        MenuRoute::Settings => MenuItemId::MasterVolume,
        MenuRoute::Controls => MenuItemId::Back,
        MenuRoute::Wings => MenuItemId::Touch,
        MenuRoute::Pause(_) => MenuItemId::Resume,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuState {
    open: bool,
    stack: Vec<MenuRoute>,
    focused: MenuItemId,
    hovered: Option<MenuItemId>,
    pressed: Option<MenuItemId>,
    origin: MenuOrigin,
    experiment_available: bool,
    construct_available: bool,
    project_available: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self::launch()
    }
}

impl MenuState {
    #[must_use]
    pub fn launch() -> Self {
        Self {
            open: true,
            stack: vec![MenuRoute::Home],
            focused: MenuItemId::Modes,
            hovered: None,
            pressed: None,
            origin: MenuOrigin::Launch,
            experiment_available: false,
            construct_available: false,
            project_available: false,
        }
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Supply the room capability without teaching the menu catalog IDs.
    pub fn set_experiment_available(&mut self, available: bool) {
        self.experiment_available = available;
        if !available && self.focused == MenuItemId::Experiment {
            self.focused = MenuItemId::Explain;
            self.clear_pointer();
        }
    }

    /// Supply the room's optional Studio construction without teaching ids.
    pub fn set_construct_available(&mut self, available: bool) {
        self.construct_available = available;
        if !available && self.focused == MenuItemId::Construct {
            self.focused = MenuItemId::Explain;
            self.clear_pointer();
        }
    }

    /// Offer the one kept question. Absent means the front page stays as it was.
    pub fn set_project_available(&mut self, available: bool) {
        self.project_available = available;
        if !available && self.focused == MenuItemId::Project {
            self.focused = MenuItemId::Explain;
            self.clear_pointer();
        }
    }

    #[must_use]
    pub fn route(&self) -> MenuRoute {
        self.stack.last().copied().unwrap_or(MenuRoute::Home)
    }

    #[must_use]
    pub fn focused(&self) -> MenuItemId {
        self.focused
    }

    #[must_use]
    pub fn origin(&self) -> MenuOrigin {
        self.origin
    }

    #[must_use]
    pub fn hovered(&self) -> Option<MenuItemId> {
        self.hovered
    }

    #[must_use]
    pub fn pressed(&self) -> Option<MenuItemId> {
        self.pressed
    }

    pub fn open_home(&mut self, origin: MenuOrigin) {
        self.open = true;
        self.stack.clear();
        self.stack.push(MenuRoute::Home);
        self.origin = origin;
        self.focused = default_focus(MenuRoute::Home, origin);
        self.clear_pointer();
    }

    pub fn open_pause(&mut self, kind: ActivityKind) {
        self.open = true;
        self.stack.clear();
        self.stack.push(MenuRoute::Pause(kind));
        self.origin = MenuOrigin::Activity(kind);
        self.focused = MenuItemId::Resume;
        self.clear_pointer();
    }

    pub fn close(&mut self) {
        self.open = false;
        self.stack.clear();
        self.stack.push(MenuRoute::Home);
        self.focused = MenuItemId::Modes;
        self.clear_pointer();
    }

    fn push(&mut self, route: MenuRoute) {
        self.stack.push(route);
        self.focused = default_focus(route, self.origin);
        self.clear_pointer();
    }

    pub fn back(&mut self) -> MenuIntent {
        self.clear_pointer();
        if self.stack.len() > 1 {
            self.stack.pop();
            let route = self.route();
            self.focused = default_focus(route, self.origin);
            return MenuIntent::None;
        }
        match self.route() {
            MenuRoute::Pause(_) => MenuIntent::ResumeActivity,
            _ => MenuIntent::Close,
        }
    }

    pub fn focus(&mut self, id: MenuItemId) -> bool {
        if !state_items(self).iter().any(|item| item.id == id) || self.focused == id {
            return false;
        }
        self.focused = id;
        true
    }

    pub fn focus_next(&mut self, delta: isize) {
        let route_items = state_items(self);
        let current = route_items
            .iter()
            .position(|item| item.id == self.focused)
            .unwrap_or(0);
        let next = current.wrapping_add_signed(delta) % route_items.len();
        self.focused = route_items[next].id;
        self.clear_pointer();
    }

    pub fn move_spatial(&mut self, layout: &MenuLayout, direction: Direction) {
        if let Some(next) = layout.neighbor(self.focused, direction) {
            self.focused = next;
            self.clear_pointer();
        } else if matches!(direction, Direction::Up | Direction::Down) {
            let route_items = state_items(self);
            let current = route_items.iter().position(|item| item.id == self.focused);
            let delta = if direction == Direction::Up { -1 } else { 1 };
            if let Some(next) = current
                .and_then(|index| index.checked_add_signed(delta))
                .and_then(|index| route_items.get(index))
            {
                self.focused = next.id;
                self.clear_pointer();
            }
        }
    }

    pub fn activate_focused(&mut self) -> MenuIntent {
        let Some(item) = state_items(self)
            .into_iter()
            .find(|item| item.id == self.focused)
        else {
            return MenuIntent::None;
        };
        self.apply(item.action)
    }

    pub fn activate_shortcut(&mut self, shortcut: char) -> Option<MenuIntent> {
        let shortcut = shortcut.to_ascii_lowercase();
        let item = state_items(self)
            .into_iter()
            .find(|item| item.shortcut == Some(shortcut))?;
        self.focused = item.id;
        Some(self.apply(item.action))
    }

    /// Step the focused row when it is numeric; `None` leaves left and right
    /// free to move between rows.
    #[must_use]
    pub fn adjust_focused(&self, step: Step) -> Option<MenuIntent> {
        state_items(self)
            .into_iter()
            .find(|item| item.id == self.focused)
            .and_then(|item| match item.action {
                MenuAction::Adjust(setting) => Some(MenuIntent::Adjust(setting, step)),
                _ => None,
            })
    }

    fn apply(&mut self, action: MenuAction) -> MenuIntent {
        match action {
            MenuAction::Intent(intent) => intent,
            MenuAction::Adjust(setting) => MenuIntent::Adjust(setting, Step::Up),
            MenuAction::Open(route) => {
                self.push(route);
                MenuIntent::None
            }
            MenuAction::Back => self.back(),
        }
    }

    pub fn pointer_move(&mut self, target: Option<MenuItemId>) -> bool {
        if self.hovered == target {
            return false;
        }
        self.hovered = target;
        true
    }

    pub fn pointer_down(&mut self, target: Option<MenuItemId>) {
        self.pressed = target;
    }

    pub fn pointer_up(&mut self, target: Option<MenuItemId>) -> MenuIntent {
        let pressed = self.pressed.take();
        if pressed.is_none() || pressed != target {
            return MenuIntent::None;
        }
        let id = pressed.expect("checked above");
        if !self.focus(id) {
            self.focused = id;
        }
        self.activate_focused()
    }

    pub fn clear_pointer(&mut self) {
        self.hovered = None;
        self.pressed = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    fn center(self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    fn contains(self, point: (f64, f64), size: (usize, usize)) -> bool {
        if size.0 == 0 || size.1 == 0 || !point.0.is_finite() || !point.1.is_finite() {
            return false;
        }
        let x = point.0 * size.0 as f64;
        let y = point.1 * size.1 as f64;
        x >= f64::from(self.x)
            && x <= f64::from(self.x + self.width)
            && y >= f64::from(self.y)
            && y <= f64::from(self.y + self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MenuItemLayout {
    id: MenuItemId,
    rect: Rect,
}

fn menu_text_scale(width: usize, height: usize, compact: bool) -> i32 {
    if compact {
        return 2;
    }
    let viewport_scale = (width / 150).min(height / 115);
    let balanced_scale = if viewport_scale > 6 {
        viewport_scale - 1
    } else {
        viewport_scale
    };
    i32::try_from(balanced_scale.clamp(4, 16)).unwrap_or(16)
}

fn menu_auxiliary_scale(text_scale: i32, compact: bool) -> i32 {
    if compact {
        1
    } else {
        ((text_scale + 1) / 2).max(2)
    }
}

fn menu_line_step(scale: i32) -> i32 {
    7 * scale + 6
}

fn menu_row_height(scale: i32) -> i32 {
    7 * scale + 16
}

/// The smallest text scale a desktop row uses. Its row stays above the
/// 42-pixel touch target, and its type stays readable across the room.
const MIN_DESKTOP_ROW_SCALE: i32 = 4;
const COMPACT_ROW_TOP: i32 = 50;
const COMPACT_ROW_STEP: i32 = 44;
const COMPACT_ROW_HEIGHT: i32 = 42;

/// The largest row scale, up to the page's own, at which `rows` rows fit in
/// `available` pixels. A long list steps its rows down in whole pixels
/// before it would run into the footer; when even the smallest rows cannot
/// fit, they keep the smallest scale.
fn fitting_row_scale(text_scale: i32, rows: usize, available: i32) -> i32 {
    let rows = i32::try_from(rows).unwrap_or(i32::MAX);
    (MIN_DESKTOP_ROW_SCALE..=text_scale.max(MIN_DESKTOP_ROW_SCALE))
        .rev()
        .find(|scale| menu_row_height(*scale).saturating_mul(rows) <= available)
        .unwrap_or(MIN_DESKTOP_ROW_SCALE)
}

fn menu_footer_reserve(text_scale: i32, compact: bool) -> i32 {
    // Two help lines, two control lines, and their margins. Reserve the
    // longest footer before placing rows, independent of the selected item.
    4 * menu_line_step(menu_auxiliary_scale(text_scale, compact)) + 30
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuLayout {
    size: (usize, usize),
    compact: bool,
    items: Vec<MenuItemLayout>,
    /// The text scale of the rows, shared by drawing and hit testing.
    row_scale: i32,
}

impl MenuLayout {
    #[must_use]
    pub fn new(state: &MenuState, width: usize, height: usize) -> Self {
        let compact = width < COMPACT_WIDTH || height < COMPACT_HEIGHT;
        let text_scale = menu_text_scale(width, height, compact);
        let route_items = state_items(state);
        let mut placed = Vec::with_capacity(route_items.len());
        let mut row_scale = text_scale;
        if compact && state.route() == MenuRoute::Controls {
            placed.push(MenuItemLayout {
                id: MenuItemId::Back,
                rect: Rect {
                    x: 18,
                    y: height.saturating_sub(76) as i32,
                    width: (width as i32 - 36).max(1),
                    height: 42,
                },
            });
        } else if compact {
            let focused_index = route_items
                .iter()
                .position(|item| item.id == state.focused)
                .unwrap_or(0);
            let available = height as i32 - menu_footer_reserve(text_scale, true) - COMPACT_ROW_TOP;
            let fitting = (available / COMPACT_ROW_STEP).clamp(1, 3) as usize;
            let visible_count = route_items.len().min(fitting);
            let start = focused_index
                .saturating_sub(visible_count / 2)
                .min(route_items.len() - visible_count);
            for (visible_index, item) in route_items
                .iter()
                .skip(start)
                .take(visible_count)
                .enumerate()
            {
                placed.push(MenuItemLayout {
                    id: item.id,
                    rect: Rect {
                        x: 24,
                        y: COMPACT_ROW_TOP + visible_index as i32 * COMPACT_ROW_STEP,
                        width: (width as i32 - 48).max(1),
                        height: COMPACT_ROW_HEIGHT,
                    },
                });
            }
        } else if state.route() == MenuRoute::Controls {
            let panel_width = (width as i32 * 80 / 100)
                .min(120 * text_scale)
                .max(420)
                .min((width as i32 - 48).max(1));
            let row_height = 7 * text_scale + 16;
            placed.push(MenuItemLayout {
                id: MenuItemId::Back,
                rect: Rect {
                    x: (width as i32 - panel_width) / 2,
                    y: (height as i32 - menu_footer_reserve(text_scale, false) - row_height)
                        .max(90),
                    width: panel_width,
                    height: row_height,
                },
            });
        } else {
            let panel_width = (width as i32 * 80 / 100)
                .min(120 * text_scale)
                .max(420)
                .min((width as i32 - 48).max(1));
            let title_y = (height as i32 * 4 / 100).max(28);
            let content_top = title_y + 7 * (text_scale + 1) + 24;
            let content_bottom = height as i32 - menu_footer_reserve(text_scale, false);
            row_scale =
                fitting_row_scale(text_scale, route_items.len(), content_bottom - content_top);
            let row_height = menu_row_height(row_scale);
            let available = (content_bottom - content_top).max(row_height);
            let visible_count = route_items
                .len()
                .min((available / row_height).max(1) as usize);
            let focused_index = route_items
                .iter()
                .position(|item| item.id == state.focused)
                .unwrap_or(0);
            let start = focused_index
                .saturating_sub(visible_count / 2)
                .min(route_items.len() - visible_count);
            let total_height = row_height * visible_count as i32;
            let top = content_top + (available - total_height) / 2;
            for (index, item) in route_items
                .iter()
                .skip(start)
                .take(visible_count)
                .enumerate()
            {
                placed.push(MenuItemLayout {
                    id: item.id,
                    rect: Rect {
                        x: (width as i32 - panel_width) / 2,
                        y: top + index as i32 * row_height,
                        width: panel_width,
                        height: row_height,
                    },
                });
            }
        }
        Self {
            size: (width, height),
            compact,
            items: placed,
            row_scale,
        }
    }

    #[must_use]
    pub fn item_at(&self, point: (f64, f64)) -> Option<MenuItemId> {
        self.items
            .iter()
            .find(|item| item.rect.contains(point, self.size))
            .map(|item| item.id)
    }

    #[must_use]
    pub fn is_compact(&self) -> bool {
        self.compact
    }

    fn neighbor(&self, current: MenuItemId, direction: Direction) -> Option<MenuItemId> {
        if self.compact {
            return None;
        }
        let source = self.items.iter().find(|item| item.id == current)?;
        let (sx, sy) = source.rect.center();
        self.items
            .iter()
            .filter(|item| item.id != current)
            .filter_map(|item| {
                let (x, y) = item.rect.center();
                let (dx, dy) = (x - sx, y - sy);
                let forward = match direction {
                    Direction::Up => dy < 0,
                    Direction::Right => dx > 0,
                    Direction::Down => dy > 0,
                    Direction::Left => dx < 0,
                };
                if !forward {
                    return None;
                }
                let primary = match direction {
                    Direction::Up | Direction::Down => dy.abs(),
                    Direction::Left | Direction::Right => dx.abs(),
                };
                let cross = match direction {
                    Direction::Up | Direction::Down => dx.abs(),
                    Direction::Left | Direction::Right => dy.abs(),
                };
                Some((primary + cross * 4, item.id))
            })
            .min_by_key(|(score, _)| *score)
            .map(|(_, id)| id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MenuReadout<'a> {
    pub study_text_size: numinous_core::StudyTextSize,
    pub volume_percent: u8,
    pub music_percent: u8,
    pub room_percent: u8,
    pub effect_percent: u8,
    pub muted: bool,
    pub era: &'a str,
    pub window_mode: &'a str,
    pub fullscreen: bool,
}

fn centered_x(text: &str, scale: i32, rect: Rect) -> i32 {
    rect.x + (rect.width - menu_font::text_width(text, scale)) / 2
}

fn item_value(id: MenuItemId, readout: MenuReadout<'_>) -> Option<String> {
    match id {
        MenuItemId::MasterVolume => Some(format!("{}%", readout.volume_percent)),
        MenuItemId::MusicVolume => Some(format!("{}%", readout.music_percent)),
        MenuItemId::RoomVolume => Some(format!("{}%", readout.room_percent)),
        MenuItemId::EffectVolume => Some(format!("{}%", readout.effect_percent)),
        MenuItemId::Mute => Some(if readout.muted { "ON" } else { "OFF" }.to_string()),
        MenuItemId::VisualEra => Some(readout.era.to_uppercase()),
        MenuItemId::WindowMode => Some(readout.window_mode.to_uppercase()),
        MenuItemId::StudyTextSize => Some(format!("{}%", readout.study_text_size.percent())),
        _ => None,
    }
}

fn route_title(route: MenuRoute) -> String {
    match route {
        MenuRoute::Home => "NUMINOUS".to_string(),
        MenuRoute::Modes => "MODES".to_string(),
        MenuRoute::Games => "GAMES".to_string(),
        MenuRoute::Settings => "SETTINGS".to_string(),
        MenuRoute::Controls => "CONTROLS".to_string(),
        MenuRoute::Wings => "WHERE TO START".to_string(),
        MenuRoute::Pause(kind) => format!("{} PAUSED", kind.label()),
    }
}

fn selected_item(state: &MenuState) -> MenuItem {
    state_items(state)
        .into_iter()
        .find(|item| item.id == state.focused)
        .unwrap_or(HOME_ITEMS[0])
}

pub fn draw_menu(
    raster: &mut Raster,
    state: &MenuState,
    input_mode: InputMode,
    copy: ControllerCopy,
    readout: MenuReadout<'_>,
) -> MenuLayout {
    let width = raster.width();
    let height = raster.height();
    let layout = MenuLayout::new(state, width, height);
    let text_scale = menu_text_scale(width, height, layout.compact);
    let auxiliary_scale = menu_auxiliary_scale(text_scale, layout.compact);
    raster.clear_rows(0, height as i32);
    raster.line(0, 0, width.saturating_sub(1) as i32, 0, '-');
    raster.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        height.saturating_sub(1) as i32,
        '-',
    );

    let title = route_title(state.route());
    let title_scale = if layout.compact { 2 } else { text_scale + 1 };
    let title_rect = Rect {
        x: 0,
        y: if layout.compact {
            16
        } else {
            (height as i32 * 4 / 100).max(28)
        },
        width: width as i32,
        height: 32,
    };
    menu_font::draw_text(
        raster,
        &title,
        centered_x(&title, title_scale, title_rect),
        title_rect.y,
        title_scale,
        '#',
    );

    let selected = selected_item(state);
    for item_layout in &layout.items {
        let Some(item) = state_items(state)
            .into_iter()
            .find(|item| item.id == item_layout.id)
        else {
            continue;
        };
        let focused = item.id == state.focused;
        let hovered = state.hovered == Some(item.id);
        let pressed = state.pressed == Some(item.id);
        let scale = if layout.compact {
            text_scale
        } else {
            layout.row_scale
        };
        let label_y = item_layout.rect.y + (item_layout.rect.height - 7 * scale) / 2;
        let cursor = if focused {
            ">"
        } else if hovered || pressed {
            "+"
        } else {
            " "
        };
        menu_font::draw_text(
            raster,
            cursor,
            item_layout.rect.x + 8,
            label_y,
            scale,
            if focused { '#' } else { '*' },
        );
        let shortcut_x = if layout.compact {
            item_layout.rect.x + 44
        } else {
            item_layout.rect.x + 12 * scale
        };
        let label_x = if input_mode == InputMode::KeyboardMouse && item.shortcut.is_some() {
            if layout.compact {
                item_layout.rect.x + 80
            } else {
                shortcut_x + 9 * scale
            }
        } else if layout.compact {
            item_layout.rect.x + 44
        } else {
            item_layout.rect.x + 12 * scale
        };
        menu_font::draw_text(
            raster,
            item.title,
            label_x,
            label_y,
            scale,
            if focused { '#' } else { '*' },
        );
        if let Some(value) = item_value(item.id, readout) {
            let value_scale = auxiliary_scale;
            menu_font::draw_text(
                raster,
                &value,
                item_layout.rect.x + item_layout.rect.width
                    - menu_font::text_width(&value, value_scale)
                    - 2 * value_scale
                    - 8,
                item_layout.rect.y + (item_layout.rect.height - 7 * value_scale) / 2,
                value_scale,
                '#',
            );
        }
        if input_mode == InputMode::KeyboardMouse
            && let Some(shortcut) = item.shortcut
        {
            let token = shortcut.to_ascii_uppercase().to_string();
            menu_font::draw_text(raster, &token, shortcut_x, label_y, scale, '*');
        }
    }

    if state.route() == MenuRoute::Controls {
        draw_controls(
            raster,
            input_mode,
            copy,
            width,
            height,
            layout.compact,
            text_scale,
        );
    }

    let hint = match input_mode {
        InputMode::KeyboardMouse => {
            let display = if readout.fullscreen {
                "F EXIT FULLSCREEN"
            } else {
                "F FULLSCREEN"
            };
            if layout.compact || matches!(state.origin(), MenuOrigin::Activity(_)) {
                format!("ARROWS MOVE   ENTER SELECT   ESC BACK   {display}   Q QUIT")
            } else {
                format!(
                    "ARROWS MOVE   ENTER SELECT   ESC BACK   {display}   Q QUIT   BACKTICK TEXT ENTRY"
                )
            }
        }
        InputMode::Controller => format!(
            "{} MOVE   {} SELECT   {} BACK",
            copy.direction_summary(),
            copy.token(Control::Primary),
            copy.token(Control::Back)
        ),
    };
    let hint_columns = width.saturating_sub(48)
        / usize::try_from(menu_font::advance(auxiliary_scale)).unwrap_or(1);
    let hint_lines = numinous_core::wrap_text(&hint, hint_columns.max(1));
    let hint_count = hint_lines.len().min(2);
    let hint_step = menu_line_step(auxiliary_scale);
    let hint_top = height as i32 - 12 - hint_count as i32 * hint_step;
    for (row, line) in hint_lines.iter().take(2).enumerate() {
        menu_font::draw_text(
            raster,
            line,
            ((width as i32 - menu_font::text_width(line, auxiliary_scale)) / 2).max(8),
            hint_top + row as i32 * hint_step,
            auxiliary_scale,
            '*',
        );
    }

    if state.route() != MenuRoute::Controls {
        let columns = width.saturating_sub(48)
            / usize::try_from(menu_font::advance(auxiliary_scale)).unwrap_or(1);
        let description_lines = numinous_core::wrap_text(selected.description, columns.max(1));
        let description_count = description_lines.len().min(2);
        let description_step = menu_line_step(auxiliary_scale);
        let description_y = hint_top - 18 - description_count as i32 * description_step;
        for (row, line) in description_lines.iter().take(2).enumerate() {
            menu_font::draw_text(
                raster,
                line,
                ((width as i32 - menu_font::text_width(line, auxiliary_scale)) / 2).max(8),
                description_y + row as i32 * description_step,
                auxiliary_scale,
                '*',
            );
        }
    }

    if layout.items.len() < state_items(state).len() && state.route() != MenuRoute::Controls {
        let route_items = state_items(state);
        let position = route_items
            .iter()
            .position(|item| item.id == state.focused)
            .unwrap_or(0)
            + 1;
        let counter = format!("{position} / {}", route_items.len());
        let scale = if layout.compact { 1 } else { auxiliary_scale };
        menu_font::draw_text(raster, &counter, 12, 42, scale, '*');
    }
    layout
}

fn draw_controls(
    raster: &mut Raster,
    input_mode: InputMode,
    copy: ControllerCopy,
    width: usize,
    height: usize,
    compact: bool,
    text_scale: i32,
) {
    let lines = control_lines(input_mode, copy);
    let shown = if compact { &lines[..4] } else { &lines[..] };
    let scale = menu_auxiliary_scale(text_scale, compact);
    let step = if compact {
        18
    } else {
        menu_line_step(scale) + 8
    };
    let top = if compact {
        78
    } else {
        let title_y = (height as i32 * 4 / 100).max(28);
        let content_top = title_y + 7 * (text_scale + 1) + 28;
        let row_height = 7 * text_scale + 16;
        let back_top = height as i32 - menu_footer_reserve(text_scale, false) - row_height;
        let block_height = (shown.len().saturating_sub(1)) as i32 * step + 7 * scale;
        content_top + (back_top - content_top - block_height).max(0) / 2
    };
    for (index, line) in shown.iter().enumerate() {
        menu_font::draw_text(
            raster,
            line,
            ((width as i32 - menu_font::text_width(line, scale)) / 2).max(8),
            top + index as i32 * step,
            scale,
            if index.is_multiple_of(2) { '#' } else { '*' },
        );
    }
}

fn control_lines(input_mode: InputMode, copy: ControllerCopy) -> Vec<String> {
    match input_mode {
        InputMode::KeyboardMouse => [
            "A / D     PREVIOUS / NEXT ROOM".to_string(),
            // The only way to reach a room that is not adjacent to this one.
            // Stepping is one room per press through a catalog of hundreds, so
            // a player who never learns this screen meets a couple of dozen
            // rooms and never learns the rest are there. The key is spelled
            // rather than drawn because the bitmap font has no glyph for a
            // backtick or a tilde, and a blank column would be worse than
            // silence.
            "BACKTICK  FIND ANY ROOM".to_string(),
            "W / S     TIME SPEED".to_string(),
            "CLICK     TOUCH THE ROOM".to_string(),
            "E / ?     EXPLAIN".to_string(),
            "R         RESET ROOM".to_string(),
            "SPACE     PAUSE".to_string(),
            "Y / N     RADIO / NEXT TRACK".to_string(),
            "H / ESC   MENU / BACK".to_string(),
            "Q         QUIT".to_string(),
        ]
        .to_vec(),
        InputMode::Controller => vec![
            format!("{}   MOVE OR TOUCH", copy.token(Control::Move)),
            format!(
                "{} / {}   PREVIOUS / NEXT ROOM",
                copy.action_token(crate::input_legend::ControllerAction::PreviousRoom),
                copy.action_token(crate::input_legend::ControllerAction::NextRoom)
            ),
            format!(
                "{} / {}   TIME SPEED",
                copy.action_token(crate::input_legend::ControllerAction::Slower),
                copy.action_token(crate::input_legend::ControllerAction::Faster)
            ),
            format!("{}   ACT", copy.token(Control::Primary)),
            format!("{}   EXPLAIN", copy.token(Control::Inspect)),
            format!(
                "{} RESET   {} PAUSE",
                copy.token(Control::Reset),
                copy.token(Control::Pause)
            ),
            format!(
                "{} MENU   {} BACK",
                copy.token(Control::Menu),
                copy.token(Control::Back)
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_legend::ControllerFace;

    fn readout() -> MenuReadout<'static> {
        MenuReadout {
            study_text_size: numinous_core::StudyTextSize::default(),
            volume_percent: 45,
            music_percent: 100,
            room_percent: 80,
            effect_percent: 0,
            muted: false,
            era: "phosphor",
            window_mode: "windowed",
            fullscreen: false,
        }
    }

    #[test]
    fn launch_and_reopen_have_contextual_default_focus() {
        let mut state = MenuState::launch();
        assert_eq!(state.focused(), MenuItemId::Modes);
        state.open_home(MenuOrigin::Room);
        assert_eq!(state.focused(), MenuItemId::Modes);
        assert_eq!(state.activate_shortcut('m'), Some(MenuIntent::None));
        assert_eq!(state.focused(), MenuItemId::Play);
    }

    #[test]
    fn child_routes_restore_a_valid_home_focus_on_back() {
        let mut state = MenuState::launch();
        assert_eq!(state.activate_shortcut('g'), Some(MenuIntent::None));
        assert_eq!(state.route(), MenuRoute::Games);
        assert_eq!(state.back(), MenuIntent::None);
        assert_eq!(state.route(), MenuRoute::Home);
        assert_eq!(state.focused(), MenuItemId::Modes);
    }

    #[test]
    fn shortcuts_only_activate_visible_items() {
        let mut state = MenuState::launch();
        assert_eq!(state.activate_shortcut('w'), None);
        assert_eq!(state.activate_shortcut('m'), Some(MenuIntent::None));
        assert_eq!(state.route(), MenuRoute::Modes);
        assert_eq!(state.activate_shortcut('g'), None);
        assert_eq!(
            state.activate_shortcut('w'),
            Some(MenuIntent::Choose(MenuChoice::Show))
        );
    }

    #[test]
    fn pointer_requires_down_and_up_on_the_same_card() {
        let mut state = MenuState::launch();
        assert_eq!(state.activate_shortcut('m'), Some(MenuIntent::None));
        state.pointer_down(Some(MenuItemId::Watch));
        assert_eq!(state.pointer_up(Some(MenuItemId::Play)), MenuIntent::None);
        state.pointer_down(Some(MenuItemId::Watch));
        assert_eq!(
            state.pointer_up(Some(MenuItemId::Watch)),
            MenuIntent::Choose(MenuChoice::Show)
        );
    }

    #[test]
    fn pointer_hover_does_not_steal_keyboard_focus() {
        let mut state = MenuState::launch();
        assert_eq!(state.activate_shortcut('m'), Some(MenuIntent::None));
        assert!(state.pointer_move(Some(MenuItemId::Create)));
        assert_eq!(state.hovered(), Some(MenuItemId::Create));
        assert_eq!(state.focused(), MenuItemId::Watch);
    }

    #[test]
    fn compact_layout_keeps_full_hit_targets_and_fits_rows_to_height() {
        let state = MenuState::launch();
        let layout = MenuLayout::new(&state, 360, 240);
        assert!(layout.compact);
        assert_eq!(layout.items.len(), 2);
        assert_eq!(MenuLayout::new(&state, 360, 300).items.len(), 3);
        let target = layout.items[0];
        assert!(target.rect.height >= 42);
        let point = (
            (target.rect.x + target.rect.width / 2) as f64 / 360.0,
            (target.rect.y + target.rect.height / 2) as f64 / 240.0,
        );
        assert_eq!(layout.item_at(point), Some(MenuItemId::Modes));
    }

    #[test]
    fn every_compact_setting_stays_visible_above_the_help_and_controls() {
        let mut settings = MenuState::launch();
        let _ = settings.activate_shortcut('s');
        for (width, height) in [(320, 180), (320, 240), (360, 240), (360, 300), (480, 599)] {
            for item in state_items(&settings) {
                settings.focus(item.id);
                let layout = MenuLayout::new(&settings, width, height);
                let footer_top = height as i32 - menu_footer_reserve(2, true);
                assert!(layout.items.iter().any(|placed| placed.id == item.id));
                for placed in &layout.items {
                    assert!(placed.rect.height >= 42);
                    assert!(placed.rect.y + placed.rect.height <= footer_top);
                    let (x, y) = placed.rect.center();
                    assert_eq!(
                        layout.item_at((x as f64 / width as f64, y as f64 / height as f64)),
                        Some(placed.id)
                    );
                }
            }
        }
    }

    #[test]
    fn desktop_home_keeps_large_visible_reading_and_play_choices() {
        let state = MenuState::launch();
        let layout = MenuLayout::new(&state, 900, 700);
        assert!(!layout.compact);
        assert_eq!(layout.items.len(), 6);
        for item in &layout.items {
            assert!(item.rect.width >= 600);
            assert!(item.rect.height >= 50);
        }
    }

    #[test]
    fn room_capability_adds_a_separate_experiment_and_keeps_study_available() {
        for (width, height) in [(360, 240), (900, 700)] {
            let mut state = MenuState::launch();
            assert_eq!(state.activate_shortcut('e'), Some(MenuIntent::ExplainRoom));
            assert!(state.activate_shortcut('u').is_none());
            state.set_experiment_available(true);
            assert_eq!(
                state.activate_shortcut('u'),
                Some(MenuIntent::ChooseExperiment)
            );
            let layout = MenuLayout::new(&state, width, height);
            assert!(
                layout
                    .items
                    .iter()
                    .any(|item| item.id == MenuItemId::Experiment)
            );
            state.set_experiment_available(false);
            assert_eq!(state.focused(), MenuItemId::Explain);
            assert_eq!(state.activate_focused(), MenuIntent::ExplainRoom);
            assert!(state.activate_shortcut('u').is_none());
        }
    }

    #[test]
    fn a_kept_project_adds_one_question_and_removes_it_cleanly() {
        let mut state = MenuState::launch();
        assert!(state.activate_shortcut('k').is_none());
        state.set_project_available(true);
        assert_eq!(
            state.activate_shortcut('k'),
            Some(MenuIntent::OpenKeptProject)
        );
        let layout = MenuLayout::new(&state, 900, 700);
        assert!(
            layout
                .items
                .iter()
                .any(|item| item.id == MenuItemId::Project)
        );
        assert_eq!(layout.items.len(), 7);
        state.set_project_available(false);
        assert_eq!(state.focused(), MenuItemId::Explain);
        assert!(state.activate_shortcut('k').is_none());
        assert_eq!(MenuLayout::new(&state, 900, 700).items.len(), 6);
    }

    #[test]
    fn room_construction_adds_an_optional_studio_door() {
        let mut state = MenuState::launch();
        assert!(state.activate_shortcut('o').is_none());
        state.set_construct_available(true);
        assert_eq!(
            state.activate_shortcut('o'),
            Some(MenuIntent::ConstructRoom)
        );
        let layout = MenuLayout::new(&state, 900, 700);
        assert!(
            layout
                .items
                .iter()
                .any(|item| item.id == MenuItemId::Construct)
        );
        state.set_construct_available(false);
        assert_eq!(state.focused(), MenuItemId::Explain);
        assert!(state.activate_shortcut('o').is_none());
    }

    #[test]
    fn quit_is_a_deliberate_visible_choice_with_the_global_shortcut() {
        let mut visible = MenuState::launch();
        assert!(visible.focus(MenuItemId::Quit));
        assert_eq!(visible.activate_focused(), MenuIntent::Quit);

        let mut shortcut = MenuState::launch();
        assert_eq!(shortcut.activate_shortcut('q'), Some(MenuIntent::Quit));
    }

    #[test]
    fn menu_type_scales_with_the_fullscreen_viewport() {
        assert_eq!(menu_text_scale(600, 600, false), 4);
        assert_eq!(menu_text_scale(900, 700, false), 6);
        assert_eq!(menu_text_scale(1920, 1080, false), 8);
        assert_eq!(menu_text_scale(2560, 1440, false), 11);
        assert_eq!(menu_text_scale(3840, 2160, false), 16);

        let mut state = MenuState::launch();
        let _ = state.activate_shortcut('m');
        let windowed = MenuLayout::new(&state, 900, 700);
        let fullscreen = MenuLayout::new(&state, 1920, 1080);
        assert!(
            fullscreen.items[0].rect.height > windowed.items[0].rect.height,
            "fullscreen rows must grow with the text"
        );
    }

    #[test]
    fn spatial_navigation_moves_through_the_visible_text_list() {
        let mut state = MenuState::launch();
        let layout = MenuLayout::new(&state, 900, 700);
        state.move_spatial(&layout, Direction::Down);
        assert_eq!(state.focused(), MenuItemId::Explain);
        state.move_spatial(&layout, Direction::Down);
        assert_eq!(state.focused(), MenuItemId::Games);
        state.move_spatial(&layout, Direction::Down);
        assert_eq!(state.focused(), MenuItemId::Settings);
    }

    #[test]
    fn pause_back_resumes_without_discarding_the_activity() {
        let mut state = MenuState::launch();
        state.open_pause(ActivityKind::Nim);
        assert_eq!(state.back(), MenuIntent::ResumeActivity);
    }

    #[test]
    fn options_open_on_the_master_level_and_step_it_directionally() {
        let mut state = MenuState::launch();
        assert_eq!(state.activate_shortcut('s'), Some(MenuIntent::None));
        assert_eq!(state.route(), MenuRoute::Settings);
        assert_eq!(state.focused(), MenuItemId::MasterVolume);
        let master = NumericSetting::MasterVolume;
        assert_eq!(
            state.adjust_focused(Step::Down),
            Some(MenuIntent::Adjust(master, Step::Down))
        );
        assert_eq!(
            state.adjust_focused(Step::Up),
            Some(MenuIntent::Adjust(master, Step::Up))
        );
        assert_eq!(
            state.activate_focused(),
            MenuIntent::Adjust(master, Step::Up),
            "activating a numeric row steps it up"
        );
        state.focus_next(1);
        assert_eq!(
            state.adjust_focused(Step::Up),
            Some(MenuIntent::Adjust(NumericSetting::MusicVolume, Step::Up)),
            "the next row is the radio level"
        );
        assert!(state.focus(MenuItemId::Mute));
        assert_eq!(state.adjust_focused(Step::Up), None, "mute is not a level");
    }

    #[test]
    fn every_numeric_row_and_only_a_numeric_row_answers_left_and_right() {
        // The adjustment comes from the row's own action, not from a
        // hard-wired row name, so a new numeric row works on every input
        // path the moment it is listed, and no other row can steal the arrows.
        let routes = [
            MenuRoute::Home,
            MenuRoute::Modes,
            MenuRoute::Games,
            MenuRoute::Settings,
            MenuRoute::Controls,
            MenuRoute::Wings,
            MenuRoute::Pause(ActivityKind::Quiz),
        ];
        let mut numeric_rows = 0;
        for route in routes {
            for item in items(route, true, true, true) {
                let mut state = MenuState::launch();
                state.stack = vec![route];
                state.focused = item.id;
                let adjusted = state.adjust_focused(Step::Down);
                match item.action {
                    MenuAction::Adjust(setting) => {
                        numeric_rows += 1;
                        assert_eq!(adjusted, Some(MenuIntent::Adjust(setting, Step::Down)));
                    }
                    _ => assert_eq!(adjusted, None, "{route:?} {:?}", item.id),
                }
            }
        }
        assert_eq!(
            numeric_rows, 5,
            "the Settings route lists audio levels and reading size"
        );
        for setting in [
            NumericSetting::MasterVolume,
            NumericSetting::MusicVolume,
            NumericSetting::RoomVolume,
            NumericSetting::EffectVolume,
            NumericSetting::StudyTextSize,
        ] {
            assert!(!setting.title().is_empty(), "{setting:?} is a listed row");
        }
        assert_eq!(Step::Down.sign(), -1.0);
        assert_eq!(Step::Up.sign(), 1.0);
    }

    #[test]
    fn settings_offer_master_radio_room_and_effect_levels_with_their_values() {
        let rows = items(MenuRoute::Settings, false, false, false);
        let levels = rows
            .iter()
            .filter_map(|item| match item.action {
                MenuAction::Adjust(setting) => Some((item.title, setting)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            levels,
            [
                ("MASTER", NumericSetting::MasterVolume),
                ("RADIO", NumericSetting::MusicVolume),
                ("ROOM SOUND", NumericSetting::RoomVolume),
                ("EFFECTS", NumericSetting::EffectVolume),
                ("READING TEXT", NumericSetting::StudyTextSize),
            ]
        );
        let readout = readout();
        assert_eq!(
            [
                MenuItemId::MasterVolume,
                MenuItemId::MusicVolume,
                MenuItemId::RoomVolume,
                MenuItemId::EffectVolume,
            ]
            .map(|id| item_value(id, readout)),
            [
                Some("45%".to_string()),
                Some("100%".to_string()),
                Some("80%".to_string()),
                Some("0%".to_string()),
            ]
        );
    }

    #[test]
    fn a_long_list_steps_its_rows_down_instead_of_running_into_the_footer() {
        let mut settings = MenuState::launch();
        let _ = settings.activate_shortcut('s');
        for (width, height) in [
            (3840, 2160),
            (2560, 1440),
            (1920, 1080),
            (1280, 720),
            (900, 700),
            (600, 600),
        ] {
            let layout = MenuLayout::new(&settings, width, height);
            assert!(!layout.compact);
            let text_scale = menu_text_scale(width, height, false);
            let footer_top = height as i32 - menu_footer_reserve(text_scale, false);
            let last = layout.items.last().expect("settings rows").rect;
            assert!(
                last.y + last.height <= footer_top,
                "{width}x{height}: rows end at {} below the footer at {footer_top}",
                last.y + last.height
            );
            assert!(layout.row_scale <= text_scale);
            assert!(layout.row_scale >= MIN_DESKTOP_ROW_SCALE);
            assert!(layout.items.iter().all(|item| item.rect.height >= 42));
        }
        // A short list keeps the page's own scale.
        let home = MenuLayout::new(&MenuState::launch(), 900, 700);
        assert_eq!(home.row_scale, menu_text_scale(900, 700, false));
        assert_eq!(fitting_row_scale(6, 9, 461), 5);
        assert_eq!(fitting_row_scale(6, 6, 461), 6);
        assert_eq!(fitting_row_scale(6, 40, 461), MIN_DESKTOP_ROW_SCALE);
    }

    #[test]
    fn paged_desktop_settings_keep_every_row_reachable_and_clear_of_the_footer() {
        for size in [(600, 520), (600, 600), (900, 700)] {
            let mut state = MenuState::launch();
            let _ = state.activate_shortcut('s');
            let ids: Vec<_> = state_items(&state).iter().map(|item| item.id).collect();
            for (direction, expected) in [
                (Direction::Down, ids.clone()),
                (Direction::Up, ids.iter().rev().copied().collect()),
            ] {
                for (index, id) in expected.into_iter().enumerate() {
                    if index > 0 {
                        let layout = MenuLayout::new(&state, size.0, size.1);
                        state.move_spatial(&layout, direction);
                    }
                    assert_eq!(state.focused(), id);
                    let layout = MenuLayout::new(&state, size.0, size.1);
                    let item = layout.items.iter().find(|item| item.id == id).unwrap();
                    let footer_top = size.1 as i32
                        - menu_footer_reserve(menu_text_scale(size.0, size.1, false), false);
                    assert!(item.rect.y + item.rect.height <= footer_top);
                    let point = (
                        (item.rect.x + item.rect.width / 2) as f64 / size.0 as f64,
                        (item.rect.y + item.rect.height / 2) as f64 / size.1 as f64,
                    );
                    assert_eq!(layout.item_at(point), Some(id));
                }
            }
        }
    }

    #[test]
    fn the_keyboard_reference_names_the_only_way_to_reach_a_distant_room() {
        // Stepping is one room per press through hundreds, and the console is
        // the only jump. A player who opens the screen named for the controls
        // has to be able to learn it exists from there, because nothing else
        // in the app mentions it.
        let lines = control_lines(
            InputMode::KeyboardMouse,
            ControllerCopy::empty(ControllerFace::PlayStation),
        );
        let jump = lines
            .iter()
            .find(|line| line.contains("FIND ANY ROOM"))
            .expect("the controls screen must name the jump");
        assert!(jump.starts_with("BACKTICK"));

        // The key is named and not drawn on purpose. The bitmap font carries
        // uppercase Latin, digits and a short list of marks, so a backtick or
        // a tilde would render as a gap and teach nothing. Hold the whole
        // keyboard reference to letters, digits, spaces and the two marks it
        // already uses, which are all glyphs the font is known to have.
        for line in &lines {
            for mark in line.chars() {
                assert!(
                    mark.is_ascii_uppercase()
                        || mark.is_ascii_digit()
                        || matches!(mark, ' ' | '/' | '?' | '-'),
                    "the controls reference uses {mark:?}, which the font may not draw"
                );
            }
        }
    }

    #[test]
    fn the_wings_route_offers_every_wing_and_a_way_back_out() {
        // The App's front door onto the catalog. Stepping is one room per press
        // through hundreds, so a wing has to be choosable from the menu, and a
        // player who chose one has to be able to leave without knowing a key.
        let entries = items(MenuRoute::Wings, false, false, false);
        let wings = numinous_core::wings();
        assert_eq!(
            entries.len(),
            wings.len() + 3,
            "three doors, every wing, and the way back to the catalog"
        );

        // The same three doors the protocol face offers, in the same order:
        // one astonishing room, an ordered walk, then a wander by wing.
        let touch = &entries[0];
        assert_eq!(touch.id, MenuItemId::Touch);
        assert_eq!(
            touch.action,
            MenuAction::Intent(MenuIntent::TouchTheFlagship)
        );

        let walk = &entries[1];
        assert_eq!(walk.id, MenuItemId::Walk);
        assert_eq!(walk.action, MenuAction::Intent(MenuIntent::EnterWalk));
        assert_eq!(walk.title, numinous_core::STRANGE_LOOP_WALK.title);

        for (index, wing) in wings.iter().enumerate() {
            let entry = &entries[index + 2];
            assert_eq!(entry.title, wing.name, "a wing is named by the catalog");
            assert_eq!(entry.id, MenuItemId::Wing(index));
            assert_eq!(
                entry.action,
                MenuAction::Intent(MenuIntent::EnterWing(index))
            );
        }

        let out = entries.last().expect("a way out");
        assert_eq!(out.action, MenuAction::Intent(MenuIntent::LeaveWing));

        // The route has to focus something that exists, or opening it lands
        // nowhere.
        assert_eq!(
            default_focus(MenuRoute::Wings, MenuOrigin::Room),
            MenuItemId::Touch
        );
        assert!(entries.iter().any(|item| item.id == MenuItemId::Touch));
        assert_eq!(route_title(MenuRoute::Wings), "WHERE TO START");

        // The flagship the first door opens is core's choice, so the two faces
        // cannot drift onto different rooms.
        assert!(numinous_core::catalog_index(numinous_core::THRESHOLD_ROOM_ID).is_some());
    }

    #[test]
    fn the_modes_menu_says_the_cabinet_is_larger_than_the_arrows_suggest() {
        // The gap this closes: nothing in the App told a player the catalog was
        // bigger than the couple of dozen rooms an arrow key reaches.
        let door = MODE_ITEMS
            .iter()
            .find(|item| item.action == MenuAction::Open(MenuRoute::Wings))
            .expect("modes must open the wings");
        assert!(door.description.contains("HUNDREDS"));
    }

    #[test]
    fn control_reference_uses_the_effective_controller_mapping() {
        use crate::input_legend::{ControllerAction, ControllerButton};

        let mut copy = ControllerCopy::empty(ControllerFace::PlayStation);
        copy.bind(ControllerAction::Primary, ControllerButton::West);
        copy.bind(ControllerAction::Back, ControllerButton::South);
        copy.bind(ControllerAction::Menu, ControllerButton::Select);
        copy.bind(ControllerAction::Inspect, ControllerButton::Start);
        copy.bind(ControllerAction::Reset, ControllerButton::RightThumb);
        copy.bind(ControllerAction::Pause, ControllerButton::East);
        copy.bind(
            ControllerAction::PreviousRoom,
            ControllerButton::LeftTrigger2,
        );
        copy.bind(ControllerAction::NextRoom, ControllerButton::RightTrigger2);
        copy.bind(ControllerAction::Slower, ControllerButton::LeftTrigger);
        copy.bind(ControllerAction::Faster, ControllerButton::RightTrigger);

        let joined = control_lines(InputMode::Controller, copy).join("\n");
        for expected in [
            "SQUARE   ACT",
            "L2 / R2   PREVIOUS / NEXT ROOM",
            "L1 / R1   TIME SPEED",
            "START   EXPLAIN",
            "R3 RESET   CIRCLE PAUSE",
            "SELECT MENU   CROSS BACK",
        ] {
            assert!(joined.contains(expected), "missing {expected:?}: {joined}");
        }
    }

    #[test]
    fn rendering_is_bounded_at_default_and_compact_sizes() {
        for (width, height) in [(1920, 1080), (900, 700), (360, 240)] {
            let state = MenuState::launch();
            let mut raster = Raster::with_accent(width, height, [120, 220, 190]);
            let layout = draw_menu(
                &mut raster,
                &state,
                InputMode::KeyboardMouse,
                ControllerFace::Generic.into(),
                readout(),
            );
            assert_eq!(layout.size, (width, height));
            assert!(raster.lit_count() > 100);
        }
    }

    #[test]
    fn fullscreen_state_changes_the_readable_exit_legend() {
        let state = MenuState::launch();
        let mut windowed = Raster::with_accent(900, 700, [120, 220, 190]);
        let _ = draw_menu(
            &mut windowed,
            &state,
            InputMode::KeyboardMouse,
            ControllerFace::Generic.into(),
            readout(),
        );
        let mut fullscreen_readout = readout();
        fullscreen_readout.fullscreen = true;
        let mut fullscreen = Raster::with_accent(900, 700, [120, 220, 190]);
        let _ = draw_menu(
            &mut fullscreen,
            &state,
            InputMode::KeyboardMouse,
            ControllerFace::Generic.into(),
            fullscreen_readout,
        );
        assert_ne!(windowed.to_rgba(), fullscreen.to_rgba());
    }

    #[test]
    fn every_route_target_stays_inside_supported_viewports() {
        let mut games = MenuState::launch();
        let _ = games.activate_shortcut('g');
        let mut modes = MenuState::launch();
        let _ = modes.activate_shortcut('m');
        let mut settings = MenuState::launch();
        let _ = settings.activate_shortcut('s');
        let mut controls = settings.clone();
        let _ = controls.activate_shortcut('c');
        let mut pause = MenuState::launch();
        pause.open_pause(ActivityKind::Quiz);
        let mut studio_pause = MenuState::launch();
        studio_pause.open_pause(ActivityKind::Studio);

        for state in [
            MenuState::launch(),
            modes,
            games,
            settings,
            controls,
            pause,
            studio_pause,
        ] {
            for (width, height) in [
                (3840, 2160),
                (1920, 1080),
                (900, 700),
                (600, 520),
                (360, 240),
            ] {
                let layout = MenuLayout::new(&state, width, height);
                assert!(!layout.items.is_empty());
                for item in &layout.items {
                    let rect = item.rect;
                    assert!(rect.x >= 0 && rect.y >= 0, "{state:?}: {rect:?}");
                    assert!(rect.width > 0 && rect.height >= 42, "{state:?}: {rect:?}");
                    assert!(rect.x + rect.width <= width as i32, "{state:?}: {rect:?}");
                    assert!(rect.y + rect.height <= height as i32, "{state:?}: {rect:?}");
                    let center = (
                        f64::from(rect.x + rect.width / 2) / width as f64,
                        f64::from(rect.y + rect.height / 2) / height as f64,
                    );
                    assert_eq!(layout.item_at(center), Some(item.id));
                }
            }
        }
    }
}
