//! Wing optimization: find the angle that flies; go past it and you stall.
//!
//! Levers set the angle of attack and the airspeed. Lift climbs with angle,
//! roughly the thin-airfoil law, until the airflow separates near fifteen degrees
//! and the lift collapses. The sweet spot is right at the edge of the stall.

use std::f64::consts::PI;

use crate::sim::{Lever, Sim, SimMeta, lever_value};
use crate::surface::Surface;

/// Angle of attack (degrees) where the airflow separates and lift collapses.
const STALL_DEG: f64 = 15.0;
/// Air density (kg/m^3) times a fixed wing area, folded into one constant.
const DENSITY_AREA: f64 = 0.6;

/// The levers, in `params` order.
const LEVERS: [Lever; 2] = [
    Lever {
        name: "angle-of-attack",
        min: 0.0,
        max: 25.0,
        default: 5.0,
        unit: "degrees",
    },
    Lever {
        name: "airspeed",
        min: 20.0,
        max: 300.0,
        default: 120.0,
        unit: "knots",
    },
];

/// The wing lift optimization sim.
#[derive(Debug, Default)]
pub struct Wing;

impl Wing {
    /// Create the sim.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Lift coefficient versus angle of attack: linear, then a post-stall collapse.
fn lift_coefficient(angle_deg: f64) -> f64 {
    if angle_deg <= STALL_DEG {
        (2.0 * PI * angle_deg.to_radians()).max(0.0)
    } else {
        let peak = 2.0 * PI * STALL_DEG.to_radians();
        (peak * (1.0 - (angle_deg - STALL_DEG) / 8.0)).max(0.05)
    }
}

/// Lift force (arbitrary units) from angle and airspeed.
fn lift(angle_deg: f64, airspeed: f64) -> f64 {
    0.5 * DENSITY_AREA * airspeed * airspeed * lift_coefficient(angle_deg) / 1_000.0
}

impl Sim for Wing {
    fn meta(&self) -> SimMeta {
        SimMeta {
            id: "wing",
            title: "Wing Optimization",
            blurb: "Trim the angle of attack and airspeed for maximum lift. The best angle is right \
                    at the edge of the stall; go one degree too far and you fall out of the sky.",
            accent: [90, 170, 230],
            levers: &LEVERS,
        }
    }

    fn render(&self, surface: &mut dyn Surface, params: &[f64]) {
        let meta = self.meta();
        let angle = lever_value(&meta, params, 0);
        let airspeed = lever_value(&meta, params, 1);
        let width = surface.width();
        let height = surface.height();
        if width == 0 || height == 0 {
            return;
        }
        // Draw the lift-versus-angle curve with a marker at the current angle.
        let max_lift = (0..=250)
            .map(|d| lift(d as f64 / 10.0, airspeed))
            .fold(0.0_f64, f64::max)
            .max(1e-6);
        for px in 0..width {
            let a = 25.0 * px as f64 / width as f64;
            let value = lift(a, airspeed) / max_lift;
            let y = height as f64 - value * (height as f64 - 1.0);
            let mark = if (a - angle).abs() < 25.0 / width as f64 {
                '#'
            } else {
                '*'
            };
            surface.plot(px as i32, y as i32, mark);
        }
    }

    fn readout(&self, params: &[f64]) -> String {
        let meta = self.meta();
        let angle = lever_value(&meta, params, 0);
        let airspeed = lever_value(&meta, params, 1);
        let value = lift(angle, airspeed);
        if angle > STALL_DEG {
            format!(
                "STALL at {angle:.0} degrees. The airflow tore off the wing and you are now a \
                 lawn dart. Lift: {value:.1}."
            )
        } else if angle > STALL_DEG - 2.0 {
            format!(
                "On the ragged edge: {value:.1} units of lift at {angle:.0} degrees, {airspeed:.0} \
                 knots. One more degree and you stall. This is the sweet spot."
            )
        } else if value < 3.0 {
            format!("Barely flying: {value:.1} units of lift. Pull the nose up or add speed.")
        } else {
            format!("Clean flight: {value:.1} units of lift at {angle:.0} degrees.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{STALL_DEG, Wing, lift_coefficient};
    use crate::canvas::Canvas;
    use crate::sim::Sim;

    #[test]
    fn lift_rises_to_the_stall_then_collapses() {
        assert!(lift_coefficient(10.0) > lift_coefficient(5.0));
        assert!(lift_coefficient(STALL_DEG) > lift_coefficient(5.0));
        assert!(lift_coefficient(STALL_DEG + 3.0) < lift_coefficient(STALL_DEG));
    }

    #[test]
    fn past_the_stall_the_readout_warns() {
        let text = Wing::new().readout(&[20.0, 120.0]);
        assert!(text.contains("STALL"), "got: {text}");
    }

    #[test]
    fn the_edge_is_flagged_as_the_sweet_spot() {
        let text = Wing::new().readout(&[14.0, 150.0]);
        assert!(
            text.contains("sweet spot") || text.contains("edge"),
            "got: {text}"
        );
    }

    #[test]
    fn render_has_ink_and_does_not_panic() {
        let sim = Wing::new();
        let mut canvas = Canvas::new(50, 20);
        sim.render(&mut canvas, &[5.0, 120.0]);
        assert!(canvas.ink_count() > 5);
        let mut empty = Canvas::new(0, 0);
        sim.render(&mut empty, &[]);
    }
}
