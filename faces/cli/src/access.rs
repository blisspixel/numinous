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
    // The counts and names come from the same public lists the tests enforce,
    // so this report cannot drift from the code.
    out.push_str(&format!(
        "Known and not yet fixed, so you can decide for yourself:\n\
         {} flash faster than the WCAG budget allows,\n\
         and {} answer a touch\n\
         in a way the color-free renderer cannot show.\n\n",
        numinous_core::KNOWN_OVER_FLASH_BUDGET.join(", "),
        numinous_core::RESPONSE_INVISIBLE_WITHOUT_COLOR.join(", "),
    ));
    out.push_str(
        "One boundary, stated plainly: the keyboard reaches every menu, game,\n\
         quiz, and formula on every face, but the hand verbs inside App rooms\n\
         (drag, click, hold) need a mouse or a controller today. One exception\n\
         narrows it: U calls a room's readout, aimed with the arrow keys and\n\
         committed with Enter.\n",
    );
    out
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

/// Whether this run may add color to its output.
pub(crate) fn color_allowed() -> bool {
    color_allowed_for(std::env::var_os("NO_COLOR").as_deref())
}
