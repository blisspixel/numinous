//! Supernova: how much star you start with decides what corpse you get.
//!
//! A star fuses elements until it hits iron, which costs more energy to fuse than
//! it yields; the core collapses in under a second and rebounds as a supernova.
//! What is left depends only on the mass: a white dwarf, a neutron star, or a
//! black hole, gated by the Chandrasekhar and Tolman-Oppenheimer-Volkoff limits.

use std::f64::consts::TAU;

use crate::sim::{Lever, Sim, SimMeta, lever_value};
use crate::surface::Surface;

/// The levers, in `params` order.
const LEVERS: [Lever; 1] = [Lever {
    name: "mass",
    min: 0.5,
    max: 50.0,
    default: 15.0,
    unit: "solar masses",
}];

/// The supernova sim.
#[derive(Debug, Default)]
pub struct Supernova;

impl Supernova {
    /// Create the sim.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Sim for Supernova {
    fn meta(&self) -> SimMeta {
        SimMeta {
            id: "supernova",
            title: "Supernova",
            blurb: "Set a star's mass and watch it die. Below eight suns it puffs into a white \
                    dwarf; heavier and the core collapses into a neutron star, or a black hole.",
            accent: [255, 140, 60],
            levers: &LEVERS,
        }
    }

    fn render(&self, surface: &mut dyn Surface, params: &[f64]) {
        let meta = self.meta();
        let mass = lever_value(&meta, params, 0);
        let width = surface.width();
        let height = surface.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = surface.char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let max_r = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.9;
        // Bigger stars throw more, brighter shells.
        let shells = 2 + (mass / 8.0) as usize;
        for shell in 1..=shells {
            let r = max_r * shell as f64 / shells as f64;
            let mark = if shell == 1 { '#' } else { '*' };
            let steps = 480;
            for i in 0..steps {
                let theta = TAU * i as f64 / steps as f64;
                let x = cx + r * theta.cos();
                let y = cy + r * theta.sin() * aspect;
                surface.plot(x as i32, y as i32, mark);
            }
        }
    }

    fn readout(&self, params: &[f64]) -> String {
        let meta = self.meta();
        let mass = lever_value(&meta, params, 0);
        if mass < 8.0 {
            format!(
                "{mass:.1} suns: no supernova. It sheds its outer layers as a glowing planetary \
                 nebula and leaves a white dwarf, an Earth-sized ember that cools for eternity."
            )
        } else if mass < 20.0 {
            format!(
                "{mass:.1} suns: core-collapse supernova. For a few weeks it outshines its entire \
                 galaxy, leaving a neutron star, a teaspoon of which weighs a billion tons."
            )
        } else {
            format!(
                "{mass:.1} suns: the core is too heavy for anything to hold it up. It collapses \
                 straight through into a black hole. The light does not escape to tell the tale."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Supernova;
    use crate::canvas::Canvas;
    use crate::sim::Sim;

    #[test]
    fn mass_decides_the_corpse() {
        assert!(Supernova::new().readout(&[5.0]).contains("white dwarf"));
        assert!(Supernova::new().readout(&[15.0]).contains("neutron star"));
        assert!(Supernova::new().readout(&[30.0]).contains("black hole"));
    }

    #[test]
    fn render_has_ink_and_does_not_panic() {
        let sim = Supernova::new();
        let mut canvas = Canvas::new(40, 20);
        sim.render(&mut canvas, &[15.0]);
        assert!(canvas.ink_count() > 10);
        let mut empty = Canvas::new(0, 0);
        sim.render(&mut empty, &[]);
    }
}
