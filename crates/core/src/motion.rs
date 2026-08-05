//! Motion preference: how much a surface may move without being asked.
//!
//! Numinous animates by advancing a phase `t` through `[0, 1)`. Reduced motion
//! stops that advance and nothing else: the picture holds still, but the player
//! can still touch it, change rooms, and read its status. Removing the player's
//! own agency would be a different and worse thing than removing the motion.
//!
//! Every face reads the preference from here so they cannot disagree about what
//! it means. See `docs/ROADMAP.md` 0.5 Sensory Alpha.

use std::ffi::OsStr;

/// The environment variable a player sets to hold the picture still.
pub const REDUCED_MOTION_VAR: &str = "NUMINOUS_REDUCED_MOTION";

/// Whether an environment setting counts as switched on.
///
/// Present and not empty, which is the rule `NO_COLOR` uses, so a player who
/// has learned one accessibility switch has learned all of them. Truthiness
/// deliberately does not enter into it: someone who wrote `=0` still wrote it.
#[must_use]
pub fn setting_is_on(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// How much a surface may animate on its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    /// Animation runs. The default, and what every existing receipt records.
    #[default]
    Full,
    /// Nothing moves unless the player moves it.
    Reduced,
}

impl Motion {
    /// Decide from a raw setting value, without reading the environment, so
    /// the rule can be tested exhaustively.
    #[must_use]
    pub fn from_setting(value: Option<&OsStr>) -> Self {
        if setting_is_on(value) {
            Self::Reduced
        } else {
            Self::Full
        }
    }

    /// Decide from [`REDUCED_MOTION_VAR`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::from_setting(std::env::var_os(REDUCED_MOTION_VAR).as_deref())
    }

    /// Whether a view may advance its own phase.
    #[must_use]
    pub fn animates(self) -> bool {
        matches!(self, Self::Full)
    }

    /// The next phase for a view that advances on its own, wrapping at 1.
    ///
    /// Reduced motion returns `t` unchanged, which is the whole mechanism: a
    /// loop that keeps drawing keeps responding, it just stops moving.
    /// Non-finite input resolves to the start of the cycle rather than
    /// poisoning every later frame.
    #[must_use]
    pub fn next_phase(self, t: f64, step: f64) -> f64 {
        if !t.is_finite() {
            return 0.0;
        }
        if !self.animates() {
            return t;
        }
        let next = t + step;
        if !next.is_finite() || next >= 1.0 {
            0.0
        } else {
            next
        }
    }

    /// Choose between the phase a moving view would show and the one a held
    /// view should rest on.
    ///
    /// A gallery that steps between rooms uses this to show each room at its
    /// best still phase instead of catching it mid-sweep.
    #[must_use]
    pub fn phase(self, moving: f64, still: f64) -> f64 {
        if self.animates() { moving } else { still }
    }
}

#[cfg(test)]
mod tests {
    use super::{Motion, REDUCED_MOTION_VAR, setting_is_on};
    use std::ffi::OsStr;

    #[test]
    fn the_switch_reads_presence_not_truthiness() {
        // Same rule as NO_COLOR. Writing =0 is still writing it.
        for (value, on) in [
            (None, false),
            (Some(""), false),
            (Some("1"), true),
            (Some("0"), true),
            (Some("false"), true),
            (Some("no"), true),
            (Some(" "), true),
        ] {
            let value = value.map(OsStr::new);
            assert_eq!(setting_is_on(value), on, "{value:?}");
            assert_eq!(
                Motion::from_setting(value),
                if on { Motion::Reduced } else { Motion::Full },
                "{value:?}"
            );
        }
    }

    #[test]
    fn full_motion_is_the_default() {
        assert_eq!(Motion::default(), Motion::Full);
        assert!(Motion::default().animates());
    }

    #[test]
    fn reduced_motion_holds_the_phase_wherever_it_is() {
        for t in [0.0, 0.25, 0.5, 0.999] {
            assert_eq!(Motion::Reduced.next_phase(t, 0.01), t);
            assert_eq!(Motion::Reduced.next_phase(t, 0.5), t);
        }
    }

    #[test]
    fn full_motion_advances_and_wraps_at_one() {
        assert!((Motion::Full.next_phase(0.0, 0.01) - 0.01).abs() < 1e-12);
        assert!((Motion::Full.next_phase(0.5, 0.25) - 0.75).abs() < 1e-12);
        assert_eq!(Motion::Full.next_phase(0.995, 0.01), 0.0);
        assert_eq!(Motion::Full.next_phase(0.999, 0.005), 0.0);
    }

    #[test]
    fn a_held_view_rests_on_the_still_phase() {
        assert!((Motion::Full.phase(0.3, 0.8) - 0.3).abs() < 1e-12);
        assert!((Motion::Reduced.phase(0.3, 0.8) - 0.8).abs() < 1e-12);
    }

    #[test]
    fn non_finite_input_cannot_poison_the_cycle() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(Motion::Full.next_phase(bad, 0.01), 0.0);
            assert_eq!(Motion::Reduced.next_phase(bad, 0.01), 0.0);
        }
        assert_eq!(Motion::Full.next_phase(0.5, f64::NAN), 0.0);
        assert_eq!(Motion::Full.next_phase(0.5, f64::INFINITY), 0.0);
    }

    #[test]
    fn the_variable_is_named_for_what_it_does() {
        assert_eq!(REDUCED_MOTION_VAR, "NUMINOUS_REDUCED_MOTION");
    }
}
