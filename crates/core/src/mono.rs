//! Mono audio: one signal, on every channel.
//!
//! Two different needs meet here, and they want the same arithmetic.
//!
//! A **mono device** has one channel and the stereo frame has to become one
//! sample or nothing plays correctly.
//!
//! A **player** may want mono on stereo hardware: someone hearing from one ear
//! loses whatever is panned to the other, and someone who finds stereo
//! movement disorienting wants it to stop. `NUMINOUS_MONO_AUDIO` puts the same
//! single signal on both channels so nothing is lost to a side the listener
//! cannot hear.
//!
//! This module owns the *convention*, not the arithmetic. `crates/audio` is a
//! hardware adapter that deliberately does not depend on this crate, so the
//! sample-level downmix lives there and a face reads the preference here and
//! passes it down.

/// The environment variable a player sets to collapse audio to one signal.
pub const MONO_AUDIO_VAR: &str = "NUMINOUS_MONO_AUDIO";

/// Whether audio should be collapsed to one signal, from a raw setting value.
///
/// Present and not empty, the same rule `NO_COLOR` and
/// [`crate::motion::REDUCED_MOTION_VAR`] use, so a player learns one
/// convention for every accessibility switch.
#[must_use]
pub fn mono_requested_for(value: Option<&std::ffi::OsStr>) -> bool {
    crate::motion::setting_is_on(value)
}

/// Whether audio should be collapsed to one signal, from the environment.
#[must_use]
pub fn mono_requested() -> bool {
    mono_requested_for(std::env::var_os(MONO_AUDIO_VAR).as_deref())
}

#[cfg(test)]
mod tests {
    use super::{MONO_AUDIO_VAR, mono_requested_for};
    use std::ffi::OsStr;

    #[test]
    fn the_switch_reads_presence_not_truthiness() {
        for (value, on) in [
            (None, false),
            (Some(""), false),
            (Some("1"), true),
            (Some("0"), true),
            (Some("false"), true),
        ] {
            assert_eq!(mono_requested_for(value.map(OsStr::new)), on, "{value:?}");
        }
    }

    #[test]
    fn the_variable_is_named_for_what_it_does() {
        assert_eq!(MONO_AUDIO_VAR, "NUMINOUS_MONO_AUDIO");
    }
}
