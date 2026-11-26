//! Interaction and presentation for the six-guest string board.

use crate::input_legend::{ControllerAction, ControllerCopy, InputMode};
use numinous_core::party::sunflower::{Board, CHALLENGE, EXPLANATION, RULES, Reading};
use numinous_core::{Raster, Surface, draw_text, wrap_text};
use winit::keyboard::{Key, NamedKey};

#[derive(Clone, Copy)]
enum Action {
    Goal,
    Why,
    Load,
    Test,
    Clear,
}

const CONTROLS: [(Action, &str, &str, Option<ControllerAction>); 5] = [
    (
        Action::Goal,
        "GOAL",
        "C",
        Some(ControllerAction::CycleRadio),
    ),
    (Action::Why, "WHY", "Y", Some(ControllerAction::Inspect)),
    (Action::Load, "LOAD", "L", None),
    (
        Action::Test,
        "TEST",
        "I",
        Some(ControllerAction::PreviousRoom),
    ),
    (Action::Clear, "CLEAR", "R", Some(ControllerAction::Reset)),
];

#[derive(Default)]
enum Panel {
    #[default]
    Board,
    Challenge,
    Inspect(Vec<[u8; 2]>),
    Why,
    Load(String),
}

pub(super) struct Play {
    board: Board,
    reading: Reading,
    cursor: u8,
    first: Option<u8>,
    panel: Panel,
    message: String,
}

impl Default for Play {
    fn default() -> Self {
        let board = Board::default();
        Self {
            board,
            reading: board.reading(),
            cursor: 1,
            first: None,
            panel: Panel::Board,
            message: String::new(),
        }
    }
}

impl Play {
    fn choose(&mut self, guest: u8) {
        self.cursor = guest;
        self.message.clear();
        if let Some(first) = self.first.take() {
            if first == guest {
                return;
            }
            if let Panel::Inspect(selected) = &mut self.panel {
                let edge = [first.min(guest), first.max(guest)];
                if !self.reading.edges.contains(&edge) {
                    self.message = "PICK A STRING ALREADY ON THE BOARD".into();
                } else if selected.len() < 3 && selected.contains(&edge) {
                    self.message = "PICK A DIFFERENT STRING".into();
                } else {
                    if selected.len() == 3 {
                        selected.clear();
                    }
                    selected.push(edge);
                }
            } else {
                self.board
                    .toggle([first, guest])
                    .expect("two guests on the ring");
                self.reading = self.board.reading();
            }
        } else {
            self.first = Some(guest);
        }
    }

    fn action(&mut self, action: Action) {
        self.first = None;
        self.message.clear();
        self.panel = match (action, &self.panel) {
            (Action::Goal, Panel::Challenge)
            | (Action::Why, Panel::Why)
            | (Action::Load, Panel::Load(_))
            | (Action::Test, Panel::Inspect(_)) => Panel::Board,
            (Action::Goal, _) => Panel::Challenge,
            (Action::Why, _) => Panel::Why,
            (Action::Load, _) => Panel::Load(String::new()),
            (Action::Test, _) => Panel::Inspect(Vec::new()),
            (Action::Clear, _) => {
                *self = Self::default();
                return;
            }
        }
    }

    pub(super) fn key(&mut self, key: &Key) {
        if let Panel::Load(code) = &mut self.panel {
            match key {
                Key::Character(ch)
                    if code.len() < 4 && ch.len() == 1 && ch.as_bytes()[0].is_ascii_hexdigit() =>
                {
                    code.push(ch.as_bytes()[0].to_ascii_uppercase() as char);
                    self.message.clear();
                }
                Key::Named(NamedKey::Backspace) => {
                    code.pop();
                    self.message.clear();
                }
                Key::Named(NamedKey::Enter) => match Board::from_code(code) {
                    Ok(board) => {
                        self.board = board;
                        self.reading = board.reading();
                        self.panel = Panel::Board;
                        self.message.clear();
                    }
                    Err(error) => self.message = error.to_uppercase(),
                },
                _ => {}
            }
            return;
        }
        if let Key::Character(ch) = key
            && let Some((action, _, _, _)) = CONTROLS
                .iter()
                .find(|(_, _, shortcut, _)| ch.eq_ignore_ascii_case(shortcut))
        {
            self.action(*action);
            return;
        }
        match key {
            _ if matches!(self.panel, Panel::Why) => {}
            Key::Character(ch) if ch.len() == 1 && (b'1'..=b'6').contains(&ch.as_bytes()[0]) => {
                self.choose(ch.as_bytes()[0] - b'0');
            }
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowUp) => {
                self.cursor = (self.cursor + 4) % 6 + 1
            }
            Key::Named(NamedKey::ArrowRight | NamedKey::ArrowDown) => {
                self.cursor = self.cursor % 6 + 1
            }
            Key::Named(NamedKey::Enter | NamedKey::Space) => self.choose(self.cursor),
            _ => {}
        }
    }

    pub(super) fn click(&mut self, width: usize, height: usize, x: f64, y: f64) {
        let layout = Layout::new(width, height);
        if y >= f64::from(layout.actions_y - layout.scale * 10) {
            let slot = (x.max(0.0) as usize).saturating_mul(CONTROLS.len()) / width.max(1);
            if let Some((action, _, _, _)) = CONTROLS.get(slot) {
                self.action(*action);
            }
        } else if matches!(
            self.panel,
            Panel::Board | Panel::Challenge | Panel::Inspect(_)
        ) {
            let radius = f64::from(layout.node_radius() + 4);
            for v in 1..=6 {
                let (vx, vy) = layout.point(v);
                if (x - f64::from(vx)).hypot(y - f64::from(vy)) <= radius {
                    self.choose(v);
                    break;
                }
            }
        }
    }

    fn status(&self) -> String {
        if let Some(first) = self.first {
            return format!("FROM {first}: PICK THE OTHER END");
        }
        if let Panel::Inspect(selected) = &self.panel {
            if !self.message.is_empty() {
                return self.message.clone();
            }
            if let Ok(edges) = selected.as_slice().try_into() {
                let test = self
                    .board
                    .inspect(edges)
                    .expect("three different strings on the board");
                let [ab, ac, bc] = test.intersections.map(|guests| {
                    guests
                        .first()
                        .map_or_else(|| "NONE".to_string(), u8::to_string)
                });
                let result = if test.core.is_some() {
                    "FLOWER"
                } else {
                    "NO FLOWER"
                };
                return format!("{result}. SHARED GUESTS: A/B {ab}, A/C {ac}, B/C {bc}");
            }
            return format!("PICK STRING {} OF 3", selected.len() + 1);
        }
        if let Some(witness) = &self.reading.witness {
            let pairs = witness.edges.map(|[a, b]| format!("{a}-{b}")).join(" ");
            if let Some(center) = witness.core.as_ref().and_then(|v| v.first()) {
                return format!("FLOWER: {pairs} MEET AT {center}");
            }
            return format!("FLOWER: {pairs} SHARE NO GUESTS");
        }
        match self.reading.edges.len() {
            0 => "PICK TWO GUESTS TO DRAW A STRING".into(),
            3 => "THREE STRINGS. THEIR PAIRS MEET DIFFERENTLY.".into(),
            n => format!("{n} STRINGS WITHOUT A FLOWER"),
        }
    }

    pub(super) fn draw(
        &self,
        width: usize,
        height: usize,
        mode: InputMode,
        copy: ControllerCopy,
    ) -> Raster {
        let mut r = Raster::with_accent(width, height, [230, 200, 120]);
        let layout = Layout::new(r.width(), r.height());
        let s = layout.scale;
        let columns = ((r.width() as i32 - 12 * s) / (6 * s)).max(1) as usize;
        draw_text(
            &mut r,
            &format!("SUNFLOWER STRINGS   BOARD {}", self.board.code()),
            6 * s,
            5 * s,
            s,
            '#',
        );
        let caption = match &self.panel {
            Panel::Why => format!("{RULES}\n\n{EXPLANATION}").to_uppercase(),
            Panel::Load(code) => format!(
                "REPLAY A BOARD\nTYPE ITS FOUR-DIGIT CODE AND PRESS ENTER.\n\nCODE {code}_\n{}",
                self.message
            ),
            Panel::Challenge => format!("{CHALLENGE}\n{RULES}").to_uppercase(),
            Panel::Inspect(_) => "PICK THREE DRAWN PAIRS. A, B, C HAVE 1, 2, 3 TICKS.".into(),
            Panel::Board if mode == InputMode::Controller => format!(
                "AIM WITH THE D-PAD, {} PICKS A GUEST.",
                copy.action_token(ControllerAction::Primary)
            ),
            Panel::Board => "CLICK GUESTS OR PRESS 1-6. REPEAT A PAIR TO ERASE.".into(),
        };
        let mut y = 18 * s;
        for paragraph in caption.lines() {
            if paragraph.is_empty() {
                y += 5 * s;
            }
            for line in wrap_text(paragraph, columns) {
                draw_text(&mut r, &line, 6 * s, y, s, '*');
                y += 9 * s;
            }
        }
        if matches!(
            self.panel,
            Panel::Board | Panel::Challenge | Panel::Inspect(_)
        ) {
            layout.strings(&mut r, self);
            let status = self.status();
            for (i, line) in wrap_text(&status, columns).iter().enumerate() {
                draw_text(
                    &mut r,
                    line,
                    6 * s,
                    layout.actions_y - (24 - 9 * i as i32) * s,
                    s,
                    '#',
                );
            }
        }
        for (i, (_, label, key, controller)) in CONTROLS.iter().enumerate() {
            let binding = match mode {
                InputMode::KeyboardMouse => key.to_string(),
                InputMode::Controller => match controller {
                    Some(command) => copy.action_token(*command),
                    None => continue,
                },
            };
            let x = (i * r.width() / CONTROLS.len()) as i32 + 5 * s;
            draw_text(&mut r, &binding, x, layout.actions_y - 8 * s, s, '*');
            draw_text(&mut r, label, x, layout.actions_y, s, '#');
        }
        r
    }
}

struct Layout {
    width: i32,
    scale: i32,
    center_y: i32,
    radius: f64,
    actions_y: i32,
}

impl Layout {
    fn new(width: usize, height: usize) -> Self {
        let width = width.min(4096) as i32;
        let height = height.min(4096) as i32;
        let scale = (width / 480).min(height / 330).clamp(1, 3);
        let top = 65 * scale;
        let bottom = height - 53 * scale;
        Self {
            width,
            scale,
            center_y: (top + bottom) / 2,
            radius: f64::from(((bottom - top) / 2).min(width / 2 - 24 * scale).max(0)),
            actions_y: height - 12 * scale,
        }
    }

    fn point(&self, v: u8) -> (i32, i32) {
        let angle = f64::from(v - 1) * std::f64::consts::TAU / 6.0 - std::f64::consts::FRAC_PI_2;
        let (sin, cos) = angle.sin_cos();
        (
            self.width / 2 + (cos * self.radius).round() as i32,
            self.center_y + (sin * self.radius).round() as i32,
        )
    }

    fn node_radius(&self) -> i32 {
        8 * self.scale
    }

    fn strings(&self, r: &mut Raster, play: &Play) {
        let samples = (self.radius * 0.9).round().max(12.0) as usize;
        for step in 0..samples {
            let angle = step as f64 * std::f64::consts::TAU / samples as f64;
            let (sin, cos) = angle.sin_cos();
            r.plot(
                self.width / 2 + (cos * self.radius).round() as i32,
                self.center_y + (sin * self.radius).round() as i32,
                '.',
            );
        }
        let marked = match &play.panel {
            Panel::Inspect(selected) => selected.as_slice(),
            _ => play
                .reading
                .witness
                .as_ref()
                .map_or(&[][..], |w| w.edges.as_slice()),
        };
        for &[a, b] in &play.reading.edges {
            let (x, y) = self.point(a);
            let (u, v) = self.point(b);
            let accent = marked.iter().position(|edge| edge == &[a, b]);
            let mark = if accent.is_some() { '#' } else { '*' };
            let angle = f64::from(v - y).atan2(f64::from(u - x));
            let (sin, cos) = angle.sin_cos();
            let trim = f64::from(self.node_radius());
            let (dx, dy) = ((trim * cos).round() as i32, (trim * sin).round() as i32);
            r.line(x + dx, y + dy, u - dx, v - dy, mark);
            if let Some(index) = accent {
                // Each chosen string carries a different number of crossbars.
                for n in 0..=index {
                    let off = (n as f64 - index as f64 / 2.0) * 5.0 * f64::from(self.scale);
                    let tx = f64::from(x) * 0.68 + f64::from(u) * 0.32 + off * cos;
                    let ty = f64::from(y) * 0.68 + f64::from(v) * 0.32 + off * sin;
                    let d = 3.0 * f64::from(self.scale);
                    r.line(
                        (tx - d * sin).round() as i32,
                        (ty + d * cos).round() as i32,
                        (tx + d * sin).round() as i32,
                        (ty - d * cos).round() as i32,
                        '#',
                    );
                }
            }
        }
        for v in 1..=6 {
            let (x, y) = self.point(v);
            let radius = self.node_radius();
            let mark = if play.first == Some(v) { '#' } else { '*' };
            let corners = [
                (x, y - radius),
                (x + radius, y),
                (x, y + radius),
                (x - radius, y),
            ];
            for i in 0..4 {
                let (a, b) = corners[i];
                let (c, d) = corners[(i + 1) % 4];
                r.line(a, b, c, d, mark);
            }
            draw_text(
                r,
                &v.to_string(),
                x - 2 * self.scale,
                y - 3 * self.scale,
                self.scale,
                '#',
            );
            if play.cursor == v {
                r.line(x - radius, y + radius + 3, x + radius, y + radius + 3, '#');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(play: &mut Play, text: &str) {
        for c in text.chars() {
            play.key(&Key::Character(c.to_string().into()));
        }
    }

    #[test]
    fn pointer_keyboard_and_code_replay_reach_the_same_strings() {
        let mut play = Play::default();
        let layout = Layout::new(360, 240);
        for v in [1, 2, 3, 4, 5, 6] {
            let (x, y) = layout.point(v);
            play.click(360, 240, f64::from(x), f64::from(y));
        }
        assert_eq!(play.reading.witness.as_ref().unwrap().core, Some(vec![]));
        assert!(play.status().contains("SHARE NO GUESTS"));
        let code = play.board.code();
        type_text(&mut play, "12");
        assert!(play.reading.witness.is_none());
        type_text(&mut play, "lFFFF");
        play.key(&Key::Named(NamedKey::Enter));
        assert!(play.message.contains("7FFF"));
        for _ in 0..4 {
            play.key(&Key::Named(NamedKey::Backspace));
        }
        type_text(&mut play, &code);
        play.key(&Key::Named(NamedKey::Enter));
        assert_eq!(play.board.code(), code);
        play.click(360, 240, 333.0, 235.0);
        assert!(play.reading.edges.is_empty());
    }

    #[test]
    fn sunflower_can_inspect_a_near_miss_on_a_board_with_a_real_witness() {
        let mut play = Play::default();
        type_text(&mut play, "12132314");
        let code = play.board.code();
        assert!(play.reading.witness.is_some());
        type_text(&mut play, "i561212");
        assert!(play.status().contains("DIFFERENT STRING"));
        type_text(&mut play, "1323");
        assert_eq!(
            play.status(),
            "NO FLOWER. SHARED GUESTS: A/B 1, A/C 2, B/C 3"
        );
        type_text(&mut play, "121314");
        assert_eq!(play.status(), "FLOWER. SHARED GUESTS: A/B 1, A/C 1, B/C 1");
        assert_eq!(play.board.code(), code);
    }

    #[test]
    fn sunflower_selection_reveal_and_challenge_leave_free_play_reversible() {
        let mut play = Play::default();
        for key in [
            NamedKey::ArrowLeft,
            NamedKey::Space,
            NamedKey::ArrowRight,
            NamedKey::Enter,
        ] {
            play.key(&Key::Named(key));
        }
        assert_eq!(play.reading.edges, vec![[1, 6]]);
        type_text(&mut play, "16");
        assert_eq!(play.board, Board::default());
        type_text(&mut play, "121314");
        assert!(play.status().contains("MEET AT 1"));
        let mode = InputMode::KeyboardMouse;
        let copy = ControllerCopy::default();
        let base = play.draw(360, 240, mode, copy).to_rgba();
        type_text(&mut play, "c");
        assert_ne!(base, play.draw(360, 240, mode, copy).to_rgba());
        type_text(&mut play, "y");
        let reveal = play.draw(360, 240, mode, copy).to_rgba();
        assert_ne!(base, reveal);
        type_text(&mut play, "12");
        assert_eq!(reveal, play.draw(360, 240, mode, copy).to_rgba());
        type_text(&mut play, "y");
        assert_eq!(base, play.draw(360, 240, mode, copy).to_rgba());
        assert_ne!(
            base,
            play.draw(360, 240, InputMode::Controller, copy).to_rgba()
        );
    }
}
