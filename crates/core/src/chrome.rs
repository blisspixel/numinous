//! The chrome around room art: one palette, two appearances, no room accent.
//!
//! Room art keeps its own accent. The Cabinet, the reader, banners, and the
//! mark do not borrow that accent. Dark is the appearance the window draws.
//! Light is checked here so a later theme has a contrast floor already, and
//! the App does not offer it.
//!
//! The mark stays `#4effff` on the dark stage. Text and selection use a
//! calmer cyan of the same hue: green and blue still match, and the red
//! channel sits closer to them, so the color is less saturated.

use crate::palette;

/// Which chrome appearance a caller is painting.
///
/// [`Appearance::Dark`] is the window. [`Appearance::Light`] is the checked
/// counterpart, not a switch the App exposes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Appearance {
    /// The near-black stage the window uses.
    Dark,
    /// The light set, held for contrast and not offered as a theme.
    Light,
}

/// A job chrome color does, independent of whichever room is waiting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// Behind room art, and behind a cleared chrome band.
    Stage,
    /// A panel sitting on the stage.
    Surface,
    /// A panel or track sitting on a surface.
    SurfaceRaised,
    /// Primary lettering.
    Text,
    /// Secondary lettering.
    TextMuted,
    /// The logo mark. On dark this is `#4effff`.
    Identity,
    /// Focused Cabinet text and the reader selection. Calmer than the mark.
    Selection,
    /// A completed goal or a saved note.
    Success,
    /// A caution that is not a refusal.
    Warning,
    /// A refusal or a failed write.
    Danger,
    /// An ordinary status.
    Info,
    /// The keyboard and controller focus ring.
    Focus,
}

const DARK: [[u8; 3]; 12] = [
    palette::STAGE,
    [22, 27, 38],
    [34, 41, 56],
    [233, 237, 240],
    [169, 181, 191],
    [78, 255, 255],
    [140, 196, 196],
    [56, 224, 132],
    [242, 148, 36],
    [230, 72, 72],
    [120, 170, 255],
    [78, 255, 255],
];

const LIGHT: [[u8; 3]; 12] = [
    [244, 246, 248],
    [255, 255, 255],
    [228, 232, 236],
    [20, 24, 32],
    [64, 74, 88],
    [0, 118, 128],
    [0, 108, 118],
    [0, 110, 62],
    [138, 78, 0],
    [168, 32, 40],
    [20, 72, 158],
    [0, 118, 128],
];

/// The RGB triple for `role` in `appearance`.
#[must_use]
pub const fn rgb(appearance: Appearance, role: Role) -> [u8; 3] {
    let table = match appearance {
        Appearance::Dark => &DARK,
        Appearance::Light => &LIGHT,
    };
    table[role as usize]
}

/// `rgb` with an opaque alpha, for readers that paint bytes directly.
#[must_use]
pub const fn rgba(appearance: Appearance, role: Role) -> [u8; 4] {
    let [red, green, blue] = rgb(appearance, role);
    [red, green, blue, 255]
}

/// The 16-color SGR for a chrome role the terminal can say.
///
/// Surfaces have no 16-color form. Identity, selection, and focus share cyan,
/// because a 16-color terminal has one cyan. Success is green, warning
/// yellow, danger red, info blue, and muted text is dim.
#[must_use]
pub const fn terminal_sgr(role: Role) -> Option<&'static str> {
    match role {
        Role::Stage | Role::Surface | Role::SurfaceRaised => None,
        Role::Text => Some("37"),
        Role::TextMuted => Some("2"),
        Role::Identity | Role::Selection | Role::Focus => Some("36"),
        Role::Success => Some("32"),
        Role::Warning => Some("33"),
        Role::Danger => Some("31"),
        Role::Info => Some("34"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Appearance, Role, rgb, terminal_sgr};

    fn ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
        let luminance = |color: [u8; 3]| {
            crate::photosensitivity::relative_luminance(color[0], color[1], color[2])
        };
        let (left, right) = (luminance(foreground), luminance(background));
        (left.max(right) + 0.05) / (left.min(right) + 0.05)
    }

    const TEXT: [Role; 3] = [Role::Text, Role::TextMuted, Role::Selection];
    const GROUNDS: [Role; 3] = [Role::Stage, Role::Surface, Role::SurfaceRaised];
    const INDICATORS: [Role; 6] = [
        Role::Identity,
        Role::Success,
        Role::Warning,
        Role::Danger,
        Role::Info,
        Role::Focus,
    ];
    const BANNER_WORDS: [Role; 4] = [Role::Success, Role::Warning, Role::Danger, Role::Info];

    #[test]
    fn every_text_pair_clears_the_body_text_floor() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            for role in TEXT {
                for ground in GROUNDS {
                    let contrast = ratio(rgb(appearance, role), rgb(appearance, ground));
                    assert!(
                        contrast >= 4.5,
                        "{appearance:?} {role:?} on {ground:?} is {contrast}"
                    );
                }
            }
            for role in BANNER_WORDS {
                let contrast = ratio(rgb(appearance, role), rgb(appearance, Role::Stage));
                assert!(
                    contrast >= 4.5,
                    "{appearance:?} {role:?} banner on the stage is {contrast}"
                );
            }
        }
    }

    #[test]
    fn every_indicator_clears_the_non_text_floor() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            for role in INDICATORS {
                for ground in GROUNDS {
                    let contrast = ratio(rgb(appearance, role), rgb(appearance, ground));
                    assert!(
                        contrast >= 3.0,
                        "{appearance:?} {role:?} on {ground:?} is {contrast}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_dark_mark_is_the_logo_cyan_and_selection_is_calmer() {
        assert_eq!(rgb(Appearance::Dark, Role::Identity), [78, 255, 255]);
        assert_eq!(rgb(Appearance::Dark, Role::Focus), [78, 255, 255]);
        let selection = rgb(Appearance::Dark, Role::Selection);
        assert_eq!(selection[1], selection[2]);
        assert!(selection[1] > selection[0]);
        let spread = |color: [u8; 3]| {
            let (mut low, mut high) = (255u8, 0u8);
            for channel in color {
                low = low.min(channel);
                high = high.max(channel);
            }
            high - low
        };
        assert!(spread(selection) < spread(rgb(Appearance::Dark, Role::Identity)));
        assert_ne!(rgb(Appearance::Light, Role::Identity), [78, 255, 255]);
    }

    #[test]
    fn the_terminal_map_uses_the_sixteen_color_roles() {
        assert_eq!(terminal_sgr(Role::Identity), Some("36"));
        assert_eq!(terminal_sgr(Role::Selection), Some("36"));
        assert_eq!(terminal_sgr(Role::Success), Some("32"));
        assert_eq!(terminal_sgr(Role::Warning), Some("33"));
        assert_eq!(terminal_sgr(Role::Danger), Some("31"));
        assert_eq!(terminal_sgr(Role::Info), Some("34"));
        assert_eq!(terminal_sgr(Role::TextMuted), Some("2"));
        assert_eq!(terminal_sgr(Role::Stage), None);
    }
}
