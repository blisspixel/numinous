//! Sims: interactive simulations you steer with several levers at once.
//!
//! A [`Room`](crate::room::Room) has a single phase knob `t`; a [`Sim`] has a set
//! of named [`Lever`]s (a breeding rate, an angle of attack, a mass) that you
//! fiddle with while optimization or hilarity ensues. Each sim renders a picture
//! and returns a plain-language [`readout`](Sim::readout) of what just happened.
//! See `docs/PLAYFUL.md`.

use crate::surface::Surface;

/// One adjustable input to a sim.
#[derive(Debug, Clone, Copy)]
pub struct Lever {
    /// The lever's name (kebab-case; used on the command line).
    pub name: &'static str,
    /// Smallest allowed value.
    pub min: f64,
    /// Largest allowed value.
    pub max: f64,
    /// The value used when the player has not set this lever.
    pub default: f64,
    /// A human unit label, e.g. "degrees".
    pub unit: &'static str,
}

/// A sim's identity, look, and levers.
#[derive(Debug, Clone, Copy)]
pub struct SimMeta {
    /// Stable id (kebab-case).
    pub id: &'static str,
    /// Display title.
    pub title: &'static str,
    /// One-line description.
    pub blurb: &'static str,
    /// Signature color, matching the room accent convention.
    pub accent: [u8; 3],
    /// The levers, in the order `params` are indexed.
    pub levers: &'static [Lever],
}

/// An interactive simulation steered by lever values.
pub trait Sim {
    /// The sim's identity and levers.
    fn meta(&self) -> SimMeta;
    /// Draw the current state for the given lever values.
    fn render(&self, surface: &mut dyn Surface, params: &[f64]);
    /// A plain-language description of the outcome (the optimization or the joke).
    fn readout(&self, params: &[f64]) -> String;
}

/// The default value of each lever, in order.
#[must_use]
pub fn default_params(meta: &SimMeta) -> Vec<f64> {
    meta.levers.iter().map(|lever| lever.default).collect()
}

/// Read lever `index` from `params`, falling back to its default, clamped to range.
#[must_use]
pub fn lever_value(meta: &SimMeta, params: &[f64], index: usize) -> f64 {
    let lever = meta.levers[index];
    params
        .get(index)
        .copied()
        .unwrap_or(lever.default)
        .clamp(lever.min, lever.max)
}

#[cfg(test)]
mod tests {
    use super::{Lever, SimMeta, default_params, lever_value};

    const LEVERS: [Lever; 2] = [
        Lever {
            name: "a",
            min: 0.0,
            max: 10.0,
            default: 3.0,
            unit: "x",
        },
        Lever {
            name: "b",
            min: -1.0,
            max: 1.0,
            default: 0.0,
            unit: "y",
        },
    ];
    const META: SimMeta = SimMeta {
        id: "test",
        title: "Test",
        blurb: "",
        accent: [0, 0, 0],
        levers: &LEVERS,
    };

    #[test]
    fn defaults_come_from_the_levers() {
        assert_eq!(default_params(&META), vec![3.0, 0.0]);
    }

    #[test]
    fn lever_value_clamps_and_falls_back() {
        assert_eq!(lever_value(&META, &[99.0], 0), 10.0); // clamped to max
        assert_eq!(lever_value(&META, &[], 0), 3.0); // fell back to default
        assert_eq!(lever_value(&META, &[5.0, -9.0], 1), -1.0); // clamped to min
    }
}
