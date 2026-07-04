//! Black hole: set the mass and your distance, learn if you are spaghetti.
//!
//! The Schwarzschild radius is `2GM/c^2`, about 2.95 km per solar mass. Inside it
//! nothing escapes; just outside, time crawls and tides stretch you into a
//! noodle. Levers are the mass and how close you dare to get.

use std::f64::consts::TAU;

use crate::sim::{Lever, Sim, SimMeta, lever_value};
use crate::surface::Surface;

/// Schwarzschild radius per solar mass, in kilometers.
const RS_PER_SOLAR_KM: f64 = 2.95;

/// The levers, in `params` order.
const LEVERS: [Lever; 2] = [
    Lever {
        name: "mass",
        min: 1.0,
        max: 1.0e8,
        default: 10.0,
        unit: "solar masses",
    },
    Lever {
        name: "distance",
        min: 1.0,
        max: 1.0e6,
        default: 100.0,
        unit: "km from center",
    },
];

/// The black hole sim.
#[derive(Debug, Default)]
pub struct BlackHole;

impl BlackHole {
    /// Create the sim.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// The event-horizon radius in kilometers for a mass in solar masses.
fn schwarzschild_km(mass_solar: f64) -> f64 {
    RS_PER_SOLAR_KM * mass_solar
}

/// Time-dilation factor at `distance_km` from the center: one second here is this
/// many seconds far away. Zero at (or inside) the horizon.
fn time_dilation(mass_solar: f64, distance_km: f64) -> f64 {
    let rs = schwarzschild_km(mass_solar);
    if distance_km <= rs {
        0.0
    } else {
        (1.0 - rs / distance_km).sqrt()
    }
}

impl Sim for BlackHole {
    fn meta(&self) -> SimMeta {
        SimMeta {
            id: "black-hole",
            title: "Black Hole",
            blurb: "Dial in a mass and a distance and read your fate: the event-horizon size, how \
                    much time slows, and whether the tides turn you into spaghetti.",
            accent: [150, 90, 230],
            levers: &LEVERS,
        }
    }

    fn render(&self, surface: &mut dyn Surface, params: &[f64]) {
        let meta = self.meta();
        let mass = lever_value(&meta, params, 0);
        let distance = lever_value(&meta, params, 1);
        let width = surface.width();
        let height = surface.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = surface.char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let max_r = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.9;

        // The event horizon as a filled disc, and a bright photon ring at 1.5 rs.
        let rs = schwarzschild_km(mass);
        // Scale so the horizon sits at 40% of the frame regardless of the numbers.
        let scale = (0.4 * max_r) / rs.max(1e-9);
        let horizon = (rs * scale).min(max_r);
        let ring = (1.5 * rs * scale).min(max_r);
        let steps = 720;
        for i in 0..steps {
            let theta = TAU * i as f64 / steps as f64;
            // Photon ring.
            let rx = cx + ring * theta.cos();
            let ry = cy + ring * theta.sin() * aspect;
            surface.plot(rx as i32, ry as i32, '#');
            // Horizon edge.
            let hx = cx + horizon * theta.cos();
            let hy = cy + horizon * theta.sin() * aspect;
            surface.plot(hx as i32, hy as i32, '-');
        }
        // Your position along the x-axis at the chosen distance.
        let you = (distance * scale).min(max_r);
        surface.plot((cx + you) as i32, cy as i32, '#');
    }

    fn readout(&self, params: &[f64]) -> String {
        let meta = self.meta();
        let mass = lever_value(&meta, params, 0);
        let distance = lever_value(&meta, params, 1);
        let rs = schwarzschild_km(mass);
        if distance <= rs {
            return format!(
                "You are inside the event horizon ({rs:.0} km). No signal you send will ever \
                 climb back out. From the outside, you simply froze and faded. Goodbye."
            );
        }
        let dilation = time_dilation(mass, distance);
        // A crude tidal check: closer than ~3 horizons and small holes shred you.
        let spaghetti = distance < rs * 3.0 && mass < 1.0e6;
        let tide = if spaghetti {
            " The tidal force between your head and feet stretches you into a two-meter noodle."
        } else {
            ""
        };
        format!(
            "Horizon: {rs:.0} km. At {distance:.0} km, one second for you is {factor:.2} seconds \
             for the distant stars.{tide}",
            factor = if dilation > 0.0 {
                1.0 / dilation
            } else {
                f64::INFINITY
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{BlackHole, schwarzschild_km, time_dilation};
    use crate::canvas::Canvas;
    use crate::sim::Sim;

    #[test]
    fn schwarzschild_radius_matches_the_formula() {
        assert!((schwarzschild_km(1.0) - 2.95).abs() < 1e-9);
        assert!((schwarzschild_km(10.0) - 29.5).abs() < 1e-9);
    }

    #[test]
    fn time_barely_dilates_far_away_and_stops_at_the_horizon() {
        assert!(time_dilation(10.0, 1.0e6) > 0.99);
        assert_eq!(time_dilation(10.0, schwarzschild_km(10.0)), 0.0);
    }

    #[test]
    fn inside_the_horizon_the_readout_says_goodbye() {
        let text = BlackHole::new().readout(&[100.0, 10.0]); // rs = 295 km, distance 10 km
        assert!(
            text.contains("event horizon") && text.contains("Goodbye"),
            "got: {text}"
        );
    }

    #[test]
    fn render_has_ink_and_does_not_panic() {
        let sim = BlackHole::new();
        let mut canvas = Canvas::new(40, 20);
        sim.render(&mut canvas, &[10.0, 100.0]);
        assert!(canvas.ink_count() > 10);
        let mut empty = Canvas::new(0, 0);
        sim.render(&mut empty, &[]);
    }
}
