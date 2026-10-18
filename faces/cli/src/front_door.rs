//! The CLI front door: one home menu, grouped help, and shell completion.
//!
//! Help headings are written here because clap 4 has a single subcommand
//! heading. The command list is the derived parser, so a new subcommand fails
//! the grouping test until it is placed in exactly one group.

use std::collections::BTreeMap;
use std::ffi::OsStr;

use numinous_core::Surface;

/// Help groups, in the order a player reads them.
const GROUPS: &[(&str, &[&str])] = &[
    ("Start here", &["tour", "watch", "play"]),
    ("Play", &["render", "describe", "reveal", "rooms", "call"]),
    (
        "Games",
        &[
            "quiz",
            "munch",
            "crack",
            "seti",
            "aliens",
            "arcade",
            "hackenbush",
            "party",
            "fifteen",
            "nim",
            "gauntlet",
            "bench",
        ],
    ),
    (
        "Create",
        &[
            "plot",
            "open-studio",
            "fork",
            "sing",
            "tune",
            "tune2",
            "radio",
            "sonify",
            "share",
            "loop",
            "gallery",
            "contact-sheet",
            "route-lab",
        ],
    ),
    ("Study", &["study", "sims", "sim", "jokes", "access"]),
    (
        "Your journey",
        &["journey", "scores", "trophies", "choose", "answer"],
    ),
    (
        "Maintain",
        &[
            "update",
            "uninstall",
            "forget",
            "project",
            "settings",
            "completion",
        ],
    ),
];

/// The verbs on the home screen, shared by a terminal and a pipe.
///
/// `{id}` is today's room. The description column is the longest rendered
/// command, so a long id moves every description together.
const HOME_MENU: &[(&str, &str)] = &[
    (
        "numinous play",
        "pick a game (munch, quiz, nim, the gauntlet...)",
    ),
    (
        "numinous play munch",
        "or name one and go (fresh deal; --daily on its own command)",
    ),
    (
        "numinous tour",
        "sit back: every room, full color, narrated",
    ),
    ("numinous watch {id}", "any one room, live, with its sound"),
    (
        "numinous radio",
        "the music stations (Y in the app tunes them)",
    ),
    (
        "numinous journey",
        "your constellation, level, locks, resonances",
    ),
    (
        "numinous rooms",
        "the whole catalog; describe <room> for the story",
    ),
    ("numinous --help", "every command, in groups"),
];

/// True when the process arguments are exactly the root help request.
pub(crate) fn args_request_root_help(args: &[impl AsRef<OsStr>]) -> bool {
    let Some(arg) = args.first() else {
        return false;
    };
    args.len() == 1 && (arg.as_ref() == "--help" || arg.as_ref() == "-h" || arg.as_ref() == "help")
}

/// Columns for wrapping: `COLUMNS` when it is set, else the terminal, else 100.
///
/// Clap uses the same fallback. Preferring `COLUMNS` lets a captured run, and
/// a player who set the variable, agree on a width.
pub(crate) fn help_columns() -> usize {
    if let Some(value) = std::env::var_os("COLUMNS")
        && let Some(text) = value.to_str()
        && let Ok(width) = text.trim().parse::<usize>()
        && width > 0
    {
        return width;
    }
    if let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size()
        && width > 0
    {
        return usize::from(width);
    }
    100
}

/// A picture for a dumb terminal: ASCII only, no color and no block art.
///
/// One character covers two pixel rows, the same geometry as the color frame,
/// so the home screen keeps its shape without asking the terminal for glyphs
/// it does not have.
pub(crate) fn ascii_picture(raster: &numinous_core::Raster) -> String {
    const RAMP: &[u8] = b" .:-=+*#%@";
    let width = raster.width();
    let height = raster.height();
    let rgba = raster.to_rgba();
    let level = |x: usize, y: usize| -> u32 {
        if y >= height {
            return 0;
        }
        let offset = (y * width + x) * 4;
        (u32::from(rgba[offset]) * 299
            + u32::from(rgba[offset + 1]) * 587
            + u32::from(rgba[offset + 2]) * 114)
            / 1000
    };
    let mut out = String::new();
    for y in (0..height).step_by(2) {
        for x in 0..width {
            let mean = (level(x, y) + level(x, y + 1)) / 2;
            let index = (mean as usize * (RAMP.len() - 1)) / 255;
            out.push(RAMP[index] as char);
        }
        out.push('\n');
    }
    out
}

/// The picture a terminal can actually draw.
///
/// Color, when the caller asked for it, stays truecolor even if `TERM` is
/// dumb. Production asks only when [`crate::access::color_allowed`] says so,
/// and that is false for a dumb terminal, the same way `NO_COLOR` is.
/// A colorless dumb terminal gets ASCII, because the usual mono picture is
/// block art. `NO_COLOR` on a normal terminal keeps that block-glyph picture.
pub(crate) fn present_picture(
    raster: &numinous_core::Raster,
    color: bool,
    term: Option<&OsStr>,
) -> String {
    if color {
        numinous_core::to_terminal(raster, true)
    } else if crate::access::term_is_dumb(term) {
        ascii_picture(raster)
    } else {
        numinous_core::to_terminal(raster, false)
    }
}

/// The home menu, one row per verb, descriptions in one column.
pub(crate) fn home_menu_text(room_id: &str) -> String {
    let commands: Vec<String> = HOME_MENU
        .iter()
        .map(|(command, _)| command.replace("{id}", room_id))
        .collect();
    let width = commands
        .iter()
        .map(|command| command.chars().count())
        .max()
        .unwrap_or(0);
    commands
        .iter()
        .zip(HOME_MENU.iter())
        .map(|(command, (_, description))| format!("  {command:<width$}  {description}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Wrap `text` on spaces so each line is at most `width` characters.
///
/// A word longer than `width` is broken, so the bound holds for every line.
pub(crate) fn wrap_words(text: &str, width: usize) -> String {
    let wrapped = wrap_field("", "", text, width);
    wrapped.trim_end_matches('\n').to_string()
}

/// Root `--help`: seven groups, each command once, wrapped to `width`.
pub(crate) fn grouped_help(cmd: &clap::Command, width: usize) -> String {
    let about = cmd
        .get_about()
        .map(|text| text.to_string())
        .unwrap_or_default();
    let by_name: BTreeMap<&str, &clap::Command> = cmd
        .get_subcommands()
        .map(|subcommand| (subcommand.get_name(), subcommand))
        .collect();
    let name_width = GROUPS
        .iter()
        .flat_map(|(_, names)| names.iter().copied())
        .map(|name| name.chars().count())
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    if !about.is_empty() {
        out.push_str(&wrap_words(&about, width));
        out.push_str("\n\n");
    }
    out.push_str("Usage: numinous [COMMAND]\n");
    for (heading, names) in GROUPS {
        out.push('\n');
        out.push_str(heading);
        out.push_str(":\n");
        for name in *names {
            let about = by_name.get(name).map(|subcommand| about_text(subcommand));
            let about = about.unwrap_or_default();
            if about.is_empty() {
                out.push_str(&format!("  {name}\n"));
            } else {
                let prefix = format!("  {name:<name_width$}  ");
                let hang = " ".repeat(prefix.chars().count());
                out.push_str(&wrap_field(&prefix, &hang, &about, width));
            }
        }
    }
    out.push_str("\nOptions:\n");
    out.push_str(&wrap_field(
        "  -h, --help     ",
        "                 ",
        "Print help",
        width,
    ));
    out.push_str(&wrap_field(
        "  -V, --version  ",
        "                 ",
        "Print version",
        width,
    ));
    out
}

/// A completion script for `shell`. The binary name inside it is `numinous`.
pub(crate) fn completion_script(cmd: &mut clap::Command, shell: clap_complete::Shell) -> String {
    let mut buf = Vec::new();
    clap_complete::generate(shell, cmd, "numinous", &mut buf);
    String::from_utf8(buf).expect("completion script is utf-8")
}

fn about_text(cmd: &clap::Command) -> String {
    cmd.get_about()
        .or_else(|| cmd.get_long_about())
        .map(|text| text.to_string())
        .unwrap_or_default()
        .replace(['\n', '\r'], " ")
}

/// Wrap `text` under `prefix` on the first line and `hang` after that.
///
/// `prefix` and `hang` are the same width. Each finished line is at most
/// `width` characters unless the prefix itself is wider than `width`.
fn wrap_field(prefix: &str, hang: &str, text: &str, width: usize) -> String {
    let width = width.max(1);
    let lead_len = prefix.chars().count().max(hang.chars().count());
    let budget = width.saturating_sub(lead_len).max(1);
    let mut chunks = Vec::new();
    for word in text.split_whitespace() {
        let chars: Vec<char> = word.chars().collect();
        if chars.is_empty() {
            continue;
        }
        let mut offset = 0;
        while offset < chars.len() {
            let take = (chars.len() - offset).min(budget);
            chunks.push(chars[offset..offset + take].iter().collect::<String>());
            offset += take;
        }
    }
    if chunks.is_empty() {
        return format!("{prefix}\n");
    }
    let mut out = String::new();
    let mut first_line = true;
    let mut current = String::new();
    let mut content_len = 0usize;
    for chunk in chunks {
        let gap = usize::from(!current.is_empty());
        if !current.is_empty() && content_len + gap + chunk.chars().count() > budget {
            push_wrapped_line(&mut out, if first_line { prefix } else { hang }, &current);
            first_line = false;
            current.clear();
            content_len = 0;
        }
        if !current.is_empty() {
            current.push(' ');
            content_len += 1;
        }
        content_len += chunk.chars().count();
        current.push_str(&chunk);
    }
    if !current.is_empty() {
        push_wrapped_line(&mut out, if first_line { prefix } else { hang }, &current);
    }
    out
}

fn push_wrapped_line(out: &mut String, lead: &str, content: &str) {
    out.push_str(lead);
    out.push_str(content);
    out.push('\n');
}
