//! Carburetor: tune the air-fuel mix between flooded and backfiring.
//!
//! An engine wants about 14.7 parts air to 1 part fuel for clean, economical
//! burning (stoichiometric), and a slightly richer 12.6 for maximum power. Too
//! rich and it floods in black smoke; too lean and it overheats and backfires.
//! Levers are the air-fuel ratio and the throttle.

use crate::sim::{Lever, Sim, SimMeta, lever_value};
use crate::surface::Surface;

/// Air-fuel ratio for best power (slightly rich of stoichiometric).
const BEST_POWER_AFR: f64 = 12.6;
/// Stoichiometric (cleanest) air-fuel ratio.
const STOICH_AFR: f64 = 14.7;

/// The levers, in `params` order.
const LEVERS: [Lever; 2] = [
    Lever {
        name: "air-fuel-ratio",
        min: 6.0,
        max: 22.0,
        default: 14.7,
        unit: "air:fuel",
    },
    Lever {
        name: "throttle",
        min: 0.0,
        max: 100.0,
        default: 60.0,
        unit: "percent",
    },
];

/// The carburetor sim.
#[derive(Debug, Default)]
pub struct Carburetor;

impl Carburetor {
    /// Create the sim.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Power output (0 to 100) as a function of mixture and throttle.
fn power(afr: f64, throttle: f64) -> f64 {
    let spread = 3.5;
    let efficiency = (-((afr - BEST_POWER_AFR) / spread).powi(2)).exp();
    efficiency * (throttle / 100.0) * 100.0
}

impl Sim for Carburetor {
    fn meta(&self) -> SimMeta {
        SimMeta {
            id: "carburetor",
            title: "Carburetor",
            blurb: "Tune the air-fuel mix. About 14.7:1 burns clean, 12.6:1 makes the most power; \
                    too rich and it floods, too lean and it backfires and melts a valve.",
            accent: [200, 90, 70],
            levers: &LEVERS,
        }
    }

    fn render(&self, surface: &mut dyn Surface, params: &[f64]) {
        let meta = self.meta();
        let afr = lever_value(&meta, params, 0);
        let throttle = lever_value(&meta, params, 1);
        let width = surface.width();
        let height = surface.height();
        if width == 0 || height == 0 {
            return;
        }
        let (min_afr, max_afr) = (LEVERS[0].min, LEVERS[0].max);
        let span = max_afr - min_afr;
        for px in 0..width {
            let a = min_afr + span * px as f64 / width as f64;
            let value = power(a, throttle) / 100.0;
            let y = (height as f64 - 1.0) - value * (height as f64 - 1.0);
            let mark = if (a - afr).abs() < span / width as f64 {
                '#'
            } else {
                '*'
            };
            surface.plot(px as i32, y as i32, mark);
        }
    }

    fn readout(&self, params: &[f64]) -> String {
        let meta = self.meta();
        let afr = lever_value(&meta, params, 0);
        let throttle = lever_value(&meta, params, 1);
        let output = power(afr, throttle);
        if afr < 10.0 {
            format!(
                "Way too rich at {afr:.1}:1. The engine drowns in fuel, coughs black smoke, and \
                 fouls the plugs. Power: {output:.0}."
            )
        } else if afr > 17.5 {
            format!(
                "Dangerously lean at {afr:.1}:1. It runs hot, pings, and backfires; hold this and \
                 you burn a valve. Power: {output:.0}."
            )
        } else if (afr - BEST_POWER_AFR).abs() < 1.0 {
            format!(
                "Best power at {afr:.1}:1, {throttle:.0}% throttle. It pulls hard. Power: {output:.0}."
            )
        } else if (afr - STOICH_AFR).abs() < 1.0 {
            format!("Clean and economical at {afr:.1}:1. Sips fuel, runs cool. Power: {output:.0}.")
        } else {
            format!("Running at {afr:.1}:1. Power: {output:.0}. Nudge toward 12.6 for more.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BEST_POWER_AFR, Carburetor, power};
    use crate::canvas::Canvas;
    use crate::sim::Sim;

    #[test]
    fn power_peaks_near_the_best_power_mixture() {
        assert!(power(BEST_POWER_AFR, 100.0) > power(8.0, 100.0));
        assert!(power(BEST_POWER_AFR, 100.0) > power(20.0, 100.0));
    }

    #[test]
    fn throttle_scales_power() {
        assert!(power(BEST_POWER_AFR, 100.0) > power(BEST_POWER_AFR, 20.0));
    }

    #[test]
    fn readout_warns_at_the_extremes() {
        assert!(Carburetor::new().readout(&[8.0, 60.0]).contains("rich"));
        assert!(Carburetor::new().readout(&[20.0, 60.0]).contains("lean"));
        assert!(Carburetor::new().readout(&[12.6, 80.0]).contains("power"));
    }

    #[test]
    fn render_has_ink_and_does_not_panic() {
        let sim = Carburetor::new();
        let mut canvas = Canvas::new(40, 16);
        sim.render(&mut canvas, &[14.7, 60.0]);
        assert!(canvas.ink_count() > 5);
        let mut empty = Canvas::new(0, 0);
        sim.render(&mut empty, &[]);
    }
}
