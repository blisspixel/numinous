//! Power-user console for the windowed App.
//!
//! Open with backtick or tilde (` or ~), type a command, Enter runs it, Esc
//! closes. Aimed at hackery power users: jump to any catalog room by id, set
//! phase and variation, flip mute/era/speed, and list rooms. Not a player
//! tutorial surface; the ordinary keys stay the main path.

use std::collections::VecDeque;

use numinous_core::{Era, Raster, Room, Surface};

/// Longest typed line kept in the input buffer.
pub(crate) const MAX_BUFFER: usize = 96;
/// Recent output lines retained for the panel.
pub(crate) const MAX_LOG: usize = 14;
/// Command history depth (Up / Down recall).
pub(crate) const MAX_HISTORY: usize = 24;

/// Parsed console command ready for the App to apply.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Command {
    Help,
    Clear,
    Where,
    /// Room id, title fragment, or decimal catalog index.
    Goto(String),
    List {
        query: Option<String>,
    },
    Reset,
    Era(String),
    Mute,
    Unmute,
    Volume(f32),
    Speed(f64),
    Phase(f64),
    Vary(u64),
    Studio,
    Show,
    Close,
    Empty,
    Unknown(String),
}

/// In-window console state: open flag, buffer, log, and history.
#[derive(Debug, Clone, Default)]
pub(crate) struct Console {
    open: bool,
    buffer: String,
    log: VecDeque<String>,
    history: VecDeque<String>,
    /// Index into history while browsing (0 = newest). None when not browsing.
    history_cursor: Option<usize>,
}

impl Console {
    #[must_use]
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Open and seed a one-line welcome if the log is empty.
    pub(crate) fn open(&mut self) {
        self.open = true;
        self.history_cursor = None;
        if self.log.is_empty() {
            self.push_log("CONSOLE  type help  Esc closes".to_string());
        }
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.history_cursor = None;
    }

    pub(crate) fn buffer(&self) -> &str {
        &self.buffer
    }

    pub(crate) fn log_lines(&self) -> impl Iterator<Item = &String> {
        self.log.iter()
    }

    pub(crate) fn push_log(&mut self, line: String) {
        let line = if line.chars().count() > 120 {
            let mut out = String::new();
            for (i, ch) in line.chars().enumerate() {
                if i >= 117 {
                    break;
                }
                out.push(ch);
            }
            out.push_str("...");
            out
        } else {
            line
        };
        self.log.push_back(line);
        while self.log.len() > MAX_LOG {
            self.log.pop_front();
        }
    }

    pub(crate) fn clear_log(&mut self) {
        self.log.clear();
    }

    pub(crate) fn push_char(&mut self, ch: char) {
        if self.buffer.chars().count() >= MAX_BUFFER {
            return;
        }
        if ch.is_control() {
            return;
        }
        self.buffer.push(ch);
        self.history_cursor = None;
    }

    pub(crate) fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            self.push_char(ch);
        }
    }

    pub(crate) fn backspace(&mut self) {
        self.buffer.pop();
        self.history_cursor = None;
    }

    /// Take the buffer as a command line, push history, clear buffer.
    pub(crate) fn take_line(&mut self) -> String {
        let line = self.buffer.trim().to_string();
        self.buffer.clear();
        self.history_cursor = None;
        if !line.is_empty() && self.history.front().map(String::as_str) != Some(line.as_str()) {
            self.history.push_front(line.clone());
            while self.history.len() > MAX_HISTORY {
                self.history.pop_back();
            }
        }
        line
    }

    pub(crate) fn history_older(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next = match self.history_cursor {
            None => 0,
            Some(i) => (i + 1).min(self.history.len() - 1),
        };
        self.history_cursor = Some(next);
        self.buffer = self.history[next].clone();
    }

    pub(crate) fn history_newer(&mut self) {
        let Some(i) = self.history_cursor else {
            return;
        };
        if i == 0 {
            self.history_cursor = None;
            self.buffer.clear();
            return;
        }
        let next = i - 1;
        self.history_cursor = Some(next);
        self.buffer = self.history[next].clone();
    }
}

/// True when the key is the classic console toggle (` or ~).
#[must_use]
pub(crate) fn is_toggle_key(text: &str) -> bool {
    matches!(text, "`" | "~" | "\u{00b4}" | "\u{02cb}")
}

/// Parse a single command line into a [`Command`].
#[must_use]
pub(crate) fn parse_line(line: &str) -> Command {
    let line = line.trim();
    if line.is_empty() {
        return Command::Empty;
    }
    let mut parts = line.split_whitespace();
    let head = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<&str> = parts.collect();
    match head.as_str() {
        "help" | "?" | "h" | "commands" => Command::Help,
        "clear" | "cls" => Command::Clear,
        "close" | "exit" | "quit" => Command::Close,
        "where" | "here" | "id" | "who" => Command::Where,
        "reset" | "reload" => Command::Reset,
        "mute" => Command::Mute,
        "unmute" => Command::Unmute,
        "studio" | "jam" | "formula" => Command::Studio,
        "show" => Command::Show,
        "room" | "goto" | "go" | "r" | "load" | "open" => {
            if rest.is_empty() {
                Command::Unknown("usage: room <id|index|title>".into())
            } else {
                Command::Goto(rest.join(" "))
            }
        }
        "list" | "ls" | "rooms" | "find" => {
            if rest.is_empty() {
                Command::List { query: None }
            } else {
                Command::List {
                    query: Some(rest.join(" ")),
                }
            }
        }
        "era" => {
            if rest.is_empty() {
                Command::Unknown("usage: era phosphor|8bit|vector|modern".into())
            } else {
                Command::Era(rest[0].to_string())
            }
        }
        "vol" | "volume" => {
            if rest.is_empty() {
                Command::Unknown("usage: vol <0..1 or 0..100>".into())
            } else {
                match parse_volume(rest[0]) {
                    Some(v) => Command::Volume(v),
                    None => Command::Unknown(format!("bad volume: {}", rest[0])),
                }
            }
        }
        "speed" | "timescale" | "rate" => {
            if rest.is_empty() {
                Command::Unknown("usage: speed <0.25..8>".into())
            } else {
                match rest[0].parse::<f64>() {
                    Ok(v) if v.is_finite() => Command::Speed(v.clamp(0.25, 8.0)),
                    _ => Command::Unknown(format!("bad speed: {}", rest[0])),
                }
            }
        }
        "t" | "phase" | "time" => {
            if rest.is_empty() {
                Command::Unknown("usage: t <0..1>".into())
            } else {
                match rest[0].parse::<f64>() {
                    Ok(v) if v.is_finite() => Command::Phase(v.clamp(0.0, 1.0)),
                    _ => Command::Unknown(format!("bad phase: {}", rest[0])),
                }
            }
        }
        "vary" | "variation" | "seed" | "v" => {
            if rest.is_empty() {
                Command::Unknown("usage: vary <u64>".into())
            } else {
                match rest[0].parse::<u64>() {
                    Ok(v) => Command::Vary(v),
                    Err(_) => Command::Unknown(format!("bad variation: {}", rest[0])),
                }
            }
        }
        other => Command::Unknown(format!("unknown: {other}  (help)")),
    }
}

fn parse_volume(raw: &str) -> Option<f32> {
    let v: f32 = raw.parse().ok()?;
    if !v.is_finite() {
        return None;
    }
    // Accept 0..1 or percent 0..100.
    let v = if v > 1.0 { v / 100.0 } else { v };
    Some(v.clamp(0.0, 1.0))
}

/// Help lines shown by the `help` command.
#[must_use]
pub(crate) fn help_lines() -> Vec<String> {
    vec![
        "room <id|n|title>   load catalog room".into(),
        "list [query]        match rooms".into(),
        "where               current room".into(),
        "t <0..1>  vary <n>  phase / seed".into(),
        "speed <x>  era <e>  mute/unmute".into(),
        "vol <0..1>  reset   studio  show".into(),
        "clear  close   ` or Esc closes".into(),
    ]
}

/// Resolve a free-text room target against the live catalog.
///
/// Order: decimal index, exact id, exact title, unique id substring, unique
/// title substring. Case-insensitive for text.
pub(crate) fn resolve_room(query: &str, rooms: &[Box<dyn Room>]) -> Result<usize, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("empty room target".into());
    }
    if let Ok(index) = q.parse::<usize>() {
        if index < rooms.len() {
            return Ok(index);
        }
        return Err(format!("index {index} out of range 0..{}", rooms.len()));
    }
    let lower = q.to_ascii_lowercase();
    // Exact id.
    if let Some(i) = rooms
        .iter()
        .position(|r| r.meta().id.eq_ignore_ascii_case(&lower))
    {
        return Ok(i);
    }
    // Exact title.
    if let Some(i) = rooms
        .iter()
        .position(|r| r.meta().title.eq_ignore_ascii_case(q))
    {
        return Ok(i);
    }
    // Unique id substring.
    let id_hits: Vec<usize> = rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| r.meta().id.to_ascii_lowercase().contains(&lower))
        .map(|(i, _)| i)
        .collect();
    if id_hits.len() == 1 {
        return Ok(id_hits[0]);
    }
    if id_hits.len() > 1 {
        let preview = id_hits
            .iter()
            .take(6)
            .map(|&i| rooms[i].meta().id)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "{} id matches: {preview}{}",
            id_hits.len(),
            if id_hits.len() > 6 { "..." } else { "" }
        ));
    }
    // Unique title substring.
    let title_hits: Vec<usize> = rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| r.meta().title.to_ascii_lowercase().contains(&lower))
        .map(|(i, _)| i)
        .collect();
    if title_hits.len() == 1 {
        return Ok(title_hits[0]);
    }
    if title_hits.len() > 1 {
        let preview = title_hits
            .iter()
            .take(6)
            .map(|&i| rooms[i].meta().id)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "{} title matches: {preview}{}",
            title_hits.len(),
            if title_hits.len() > 6 { "..." } else { "" }
        ));
    }
    Err(format!("no room matches '{q}'"))
}

/// List rooms whose id or title contains the optional query (empty = first page).
#[must_use]
pub(crate) fn list_rooms(
    rooms: &[Box<dyn Room>],
    query: Option<&str>,
    limit: usize,
) -> Vec<String> {
    let lower = query
        .map(|q| q.trim().to_ascii_lowercase())
        .filter(|q| !q.is_empty());
    let mut lines = Vec::new();
    let mut shown = 0usize;
    let mut total = 0usize;
    for (i, room) in rooms.iter().enumerate() {
        let id = room.meta().id;
        let title = room.meta().title;
        let hit = match &lower {
            None => true,
            Some(q) => {
                id.to_ascii_lowercase().contains(q) || title.to_ascii_lowercase().contains(q)
            }
        };
        if !hit {
            continue;
        }
        total += 1;
        if shown < limit {
            lines.push(format!("{i:>3}  {id}  {title}"));
            shown += 1;
        }
    }
    if total == 0 {
        lines.push("no matches".into());
    } else if total > shown {
        lines.push(format!("... {total} matches, showing {shown}"));
    } else {
        lines.push(format!("{total} rooms"));
    }
    lines
}

/// Draw the console as a bottom panel over the current frame.
pub(crate) fn draw(raster: &mut Raster, console: &Console, width: usize, height: usize) {
    if !console.is_open() || width < 40 || height < 40 {
        return;
    }
    let scale = (width as i32 / 520).clamp(1, 2);
    let line_h = 9 * scale;
    let pad = 6 * scale;
    // Log + prompt + borders.
    let log_count = console.log.len() as i32;
    let rows = log_count + 1;
    let panel_h = rows * line_h + pad * 2 + 4;
    let top = (height as i32 - panel_h - 4).max(0);
    let bottom = height as i32;
    raster.clear_rows(top, bottom);
    raster.line(0, top, width.saturating_sub(1) as i32, top, '-');
    raster.line(
        0,
        bottom.saturating_sub(1),
        width.saturating_sub(1) as i32,
        bottom.saturating_sub(1),
        '-',
    );
    let mut y = top + pad;
    for line in console.log_lines() {
        numinous_core::draw_text(raster, line, pad, y, scale, '#');
        y += line_h;
    }
    let prompt = format!("> {}_", console.buffer());
    numinous_core::draw_text(raster, &prompt, pad, y, scale, '#');
}

/// Parse era text using the shared [`Era::parse`] vocabulary.
#[must_use]
pub(crate) fn parse_era(name: &str) -> Option<Era> {
    Era::parse(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use numinous_core::all_rooms;

    #[test]
    fn toggle_keys() {
        assert!(is_toggle_key("`"));
        assert!(is_toggle_key("~"));
        assert!(!is_toggle_key("t"));
    }

    #[test]
    fn parse_room_and_help() {
        assert_eq!(parse_line("help"), Command::Help);
        assert_eq!(
            parse_line("room times-tables"),
            Command::Goto("times-tables".into())
        );
        assert_eq!(parse_line("goto 12"), Command::Goto("12".into()));
        assert_eq!(
            parse_line("r double pendulum"),
            Command::Goto("double pendulum".into())
        );
        assert_eq!(
            parse_line("list chaos"),
            Command::List {
                query: Some("chaos".into())
            }
        );
        assert_eq!(parse_line("ls"), Command::List { query: None });
        assert_eq!(parse_line("t 0.5"), Command::Phase(0.5));
        assert_eq!(parse_line("vary 7"), Command::Vary(7));
        assert_eq!(parse_line("speed 2"), Command::Speed(2.0));
        assert_eq!(parse_line("vol 50"), Command::Volume(0.5));
        assert_eq!(parse_line("mute"), Command::Mute);
        assert_eq!(parse_line("era phosphor"), Command::Era("phosphor".into()));
        assert_eq!(parse_line(""), Command::Empty);
        assert!(matches!(parse_line("nope"), Command::Unknown(_)));
    }

    #[test]
    fn resolve_by_id_index_and_title() {
        let rooms = all_rooms();
        let i = resolve_room("times-tables", &rooms).unwrap();
        assert_eq!(rooms[i].meta().id, "times-tables");
        assert_eq!(resolve_room("0", &rooms).unwrap(), 0);
        let smith = resolve_room("scariest", &rooms).unwrap();
        assert_eq!(rooms[smith].meta().id, "smith-chart");
        assert!(resolve_room("zzz-no-such", &rooms).is_err());
        assert!(resolve_room("99999", &rooms).is_err());
    }

    #[test]
    fn list_filters() {
        let rooms = all_rooms();
        let all = list_rooms(&rooms, None, 5);
        assert!(all.len() >= 2);
        assert!(all[0].contains(rooms[0].meta().id));
        let filtered = list_rooms(&rooms, Some("riemann"), 10);
        assert!(
            filtered.iter().any(|l| l.contains("riemann-sphere")),
            "{filtered:?}"
        );
    }

    #[test]
    fn buffer_and_history() {
        let mut c = Console::default();
        c.open();
        assert!(c.is_open());
        c.push_text("room julia");
        assert_eq!(c.buffer(), "room julia");
        let line = c.take_line();
        assert_eq!(line, "room julia");
        assert!(c.buffer().is_empty());
        c.history_older();
        assert_eq!(c.buffer(), "room julia");
        c.history_newer();
        assert!(c.buffer().is_empty());
        c.close();
        assert!(!c.is_open());
    }

    #[test]
    fn draw_marks_ink() {
        let mut c = Console::default();
        c.open();
        c.push_log("ok".into());
        c.push_text("help");
        let mut raster = Raster::with_accent(320, 200, [120, 200, 180]);
        // Seed noise so clear_rows is visible as a real change.
        for y in 0..200 {
            for x in 0..320 {
                raster.plot(x, y, '.');
            }
        }
        let before = raster.lit_count();
        draw(&mut raster, &c, 320, 200);
        assert!(raster.lit_count() > 0);
        assert_ne!(before, raster.lit_count());
    }

    #[test]
    fn era_names_round_trip() {
        assert_eq!(parse_era("phosphor"), Some(Era::Phosphor));
        assert_eq!(parse_era("modern"), Some(Era::Modern));
        assert!(parse_era("betamax").is_none());
    }
}
