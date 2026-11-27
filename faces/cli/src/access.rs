//! Accessibility switch interpretation and terminal reporting.

use std::ffi::OsStr;

/// One switch a player can turn on, and whether it is on right now.
pub(crate) struct AccessSetting {
    /// The environment variable that turns it on.
    pub(crate) variable: &'static str,
    /// What it does, in the player's terms, already wrapped.
    ///
    /// Wrapped here rather than at print time because these lines are fixed
    /// text: a wrapping routine would be a moving part with nothing to decide.
    pub(crate) what: &'static [&'static str],
    /// Whether it is switched on in this run.
    pub(crate) on: bool,
}

/// Every accessibility switch Numinous honors, read from raw settings.
///
/// The values are passed in rather than read here so this can be tested without
/// touching the environment of a process that is running other tests beside it.
///
/// One list, and it is the only list. The report below prints it and a test
/// checks `docs/PLAYING.md` documents every entry, so a switch cannot ship that
/// a player has no way to find out about.
pub(crate) fn access_settings(
    reduced_motion: Option<&OsStr>,
    mono_audio: Option<&OsStr>,
    no_color: Option<&OsStr>,
) -> [AccessSetting; 3] {
    [
        AccessSetting {
            variable: numinous_core::REDUCED_MOTION_VAR,
            what: &[
                "Ambient motion stops: rooms hold a still frame rather than",
                "stopping dead, and The Show waits for you instead of changing",
                "rooms on a timer. Short feedback beats in the App (banners,",
                "aha morphs, arrival cards) still play; the terminal face",
                "holds completely still.",
            ],
            on: numinous_core::setting_is_on(reduced_motion),
        },
        AccessSetting {
            variable: numinous_audio::MONO_AUDIO_VAR,
            what: &[
                "Both channels carry the same sound, so nothing is lost on one",
                "ear or one speaker.",
            ],
            on: numinous_audio::mono_requested_for(mono_audio),
        },
        AccessSetting {
            variable: "NO_COLOR",
            what: &[
                "No color in the terminal faces: rooms, chrome and games",
                "alike. Shapes and letters carry the meaning instead. This is",
                "the shared terminal convention from no-color.org, not one of",
                "ours; the windowed App keeps its Visual Eras instead.",
            ],
            on: !color_allowed_for(no_color),
        },
    ]
}

/// The accessibility report: what can be switched on, and what is on now.
pub(crate) fn access_report(settings: &[AccessSetting]) -> String {
    let mut out = String::from(
        "ACCESSIBILITY. Each switch below is an environment variable.\n\
         Give it any value at all to turn it on. Writing =0 turns it on too,\n\
         because zero is still a value you wrote.\n\
         To turn it off, unset it. Setting it to an empty value counts as off,\n\
         not on, so =\"\" leaves the switch alone.\n\n",
    );
    for setting in settings {
        out.push_str(&format!(
            "  {:<24} {}\n",
            setting.variable,
            if setting.on { "ON" } else { "off" },
        ));
        for line in setting.what {
            out.push_str(&format!("    {line}\n"));
        }
        out.push('\n');
    }
    // The names come from the same public list the tests enforce, so this
    // report cannot drift from the code. Every room's answer to a touch now
    // shows in the color-free renderer, and a catalog test holds that.
    out.push_str("Every room's answer to a touch shows under NO_COLOR.\n");
    out.push_str(&flash_disclosure(&numinous_core::KNOWN_OVER_FLASH_BUDGET));
    out.push('\n');
    out.push_str(
        "One boundary, stated plainly: the keyboard reaches every menu, game,\n\
         quiz, and formula on every face, but the hand verbs inside App rooms\n\
         (drag, click, hold) need a mouse or a controller today. One exception\n\
         narrows it: U calls a room's readout, aimed with the arrow keys and\n\
         committed with Enter.\n",
    );
    out
}

/// What the photosensitivity sweep found, in the words a player reads.
///
/// `known` is the sweep's shrink-only list: each room with the slowest App
/// speed at which its automatic tour exceeds the measured budget in an era.
/// Rooms are grouped by that speed and era.
pub(crate) fn flash_disclosure(known: &[(&str, numinous_core::Era, f64)]) -> String {
    /// Room names wrap before this width, leaving room for the comma.
    const NAME_COLUMNS: usize = 72;
    let fastest = numinous_core::MAX_TIME_SCALE;
    let mut out = String::new();
    if known.is_empty() {
        out.push_str(&format!(
            "Automatic room tours stayed within the whole-frame flash budget in every\n\
             visual era at the measured speeds up to {fastest}x.\n"
        ));
    } else {
        let mut entries = known.to_vec();
        entries.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.name().cmp(b.1.name())));
        for group in entries.chunk_by(|a, b| a.1 == b.1 && a.2 == b.2) {
            let (_, era, speed) = group[0];
            out.push_str(&format!("{} era, over budget from {speed}x:\n", era.name()));
            let rooms = group.iter().map(|&(room, _, _)| room);
            // The names get lines of their own, wrapped, so the report stays
            // inside eighty columns however long the list grows.
            let mut line = String::new();
            for room in rooms {
                if !line.is_empty() && line.len() + room.len() + 2 > NAME_COLUMNS {
                    out.push_str(&line);
                    out.push_str(",\n");
                    line.clear();
                }
                if !line.is_empty() {
                    line.push_str(", ");
                }
                line.push_str(room);
            }
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str("The App limits each listed room by its earliest over-budget speed.\n");
    }
    out.push_str("The sweep measures whole-frame general and red flashes at 240 by 140.\n");
    if !numinous_core::KNOWN_OVER_FLASH_AREA.is_empty() {
        out.push_str("Some smaller regions exceeded three flashes in one second.\n");
    }
    out.push_str("Check a room before opening it: numinous access --room <id>\n");
    out.push_str(REGIONAL_SCOPE);
    out
}

const REGIONAL_SCOPE: &str = "At 1x, 2x, 4x and 8x, the regional check averages light and color in\n\
    overlapping 80 by 47 rectangles on a 240 by 140 automatic room image.\n\
    The check looks for opposing changes in brightness or red color.\n\
    Smaller patches can average away inside a rectangle.\n\
    Hand or beat input and the final App display remain unmeasured.\n";

/// Look up the catalog measurement without starting the room or its sound.
pub(crate) fn room_flash_report(id: &str) -> Result<String, String> {
    let room = numinous_core::room_meta_by_id(id).ok_or_else(|| super::not_found_message(id))?;
    let mut out = format!("{} ({}) | Regional flashes\n", room.title, room.id);
    let mut flagged = false;
    for &(id, era, general, red) in numinous_core::KNOWN_OVER_FLASH_AREA {
        if id != room.id {
            continue;
        }
        for ((speed, general), red) in numinous_core::REGIONAL_FLASH_SPEEDS
            .into_iter()
            .zip(general)
            .zip(red)
        {
            for (kind, peak) in [("general", general), ("red", red)] {
                if peak > 0 {
                    flagged = true;
                    out.push_str(&format!(
                        "{} {speed}x: {peak} {kind} flashes in one second\n",
                        era.name()
                    ));
                }
            }
        }
    }
    if flagged {
        out.push_str("Only readings above three in one second are listed.\n");
    } else {
        out.push_str("No sampled region exceeded three general or red flashes in one second.\n");
    }
    out.push_str(REGIONAL_SCOPE);
    Ok(out)
}

/// Whether color is allowed, given whatever `NO_COLOR` was set to.
///
/// Follows the `NO_COLOR` convention (no-color.org): if the variable is
/// present and not empty, color is off, whatever its value happens to be.
/// `NO_COLOR=0` still means no color, because the convention is about
/// presence, not truthiness, and a player who set it meant it.
///
/// Split from [`color_allowed`] so the rule can be tested exhaustively
/// without mutating process-wide environment from a test thread.
pub(crate) fn color_allowed_for(no_color: Option<&OsStr>) -> bool {
    !numinous_core::setting_is_on(no_color)
}

/// Whether `TERM` names a dumb terminal.
///
/// A dumb terminal gets ASCII pictures: no color and no block art.
/// `NO_COLOR` on a normal terminal keeps the block-glyph picture.
/// Dumb is the terminal's own declaration, not a fourth access switch.
pub(crate) fn term_is_dumb(term: Option<&OsStr>) -> bool {
    term.is_some_and(|value| value.eq_ignore_ascii_case("dumb"))
}

/// Whether pictures may use color for this `NO_COLOR` and `TERM` pair.
pub(crate) fn picture_color_allowed(no_color: Option<&OsStr>, term: Option<&OsStr>) -> bool {
    color_allowed_for(no_color) && !term_is_dumb(term)
}

/// Whether this run may add color to its output.
pub(crate) fn color_allowed() -> bool {
    picture_color_allowed(
        std::env::var_os("NO_COLOR").as_deref(),
        std::env::var_os("TERM").as_deref(),
    )
}
