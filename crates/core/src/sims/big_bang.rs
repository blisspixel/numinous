//! Big Bang: one number, the density of the universe, decides how it all ends.
//!
//! The density parameter omega compares the actual density to the critical
//! density. Below one, the universe is open and expands forever into a cold heat
//! death; above one, gravity wins and it recollapses in a Big Crunch; exactly one
//! is flat, expanding forever but ever more slowly. The lever is omega.

use crate::sim::{Lever, Sim, SimMeta, lever_value};
use crate::surface::Surface;

/// Integration steps of the scale factor.
const STEPS: usize = 2_000;
/// Integration time step.
const DT: f64 = 0.01;

/// The levers, in `params` order.
const LEVERS: [Lever; 1] = [Lever {
    name: "omega",
    min: 0.2,
    max: 2.0,
    default: 1.0,
    unit: "density / critical",
}];

/// The Big Bang sim.
#[derive(Debug, Default)]
pub struct BigBang;

impl BigBang {
    /// Create the sim.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// The scale factor of the universe over time for a given omega (Friedmann).
fn scale_factor(omega: f64) -> Vec<f64> {
    let mut a = 0.001;
    let mut expanding = true;
    let mut series = Vec::with_capacity(STEPS + 1);
    series.push(a);
    for _ in 0..STEPS {
        // (da/dt)^2 = omega/a + (1 - omega). For omega > 1 the bracket goes
        // negative at maximum expansion; there the universe turns around.
        let bracket = omega / a + (1.0 - omega);
        if expanding && bracket <= 0.0 {
            expanding = false;
        }
        let rate = bracket.max(0.0).sqrt();
        if expanding {
            a += rate * DT;
        } else {
            // Contracting: guarantee progress off the peak, where the rate is ~0.
            a -= rate.max(0.05) * DT;
        }
        if a <= 0.0 {
            series.push(0.0);
            break;
        }
        series.push(a);
    }
    series
}

impl Sim for BigBang {
    fn meta(&self) -> SimMeta {
        SimMeta {
            id: "big-bang",
            title: "Big Bang",
            blurb: "One number, the density omega, decides the fate of everything. Under one, the \
                    universe expands forever; over one, it recollapses in a Big Crunch.",
            accent: [230, 120, 200],
            levers: &LEVERS,
        }
    }

    fn render(&self, surface: &mut dyn Surface, params: &[f64]) {
        let meta = self.meta();
        let omega = lever_value(&meta, params, 0);
        let width = surface.width();
        let height = surface.height();
        if width == 0 || height == 0 {
            return;
        }
        // Plot the scale factor over time: it rises, and for a crunch, falls again.
        let series = scale_factor(omega);
        let peak = series.iter().copied().fold(1e-6_f64, f64::max);
        for (i, &a) in series.iter().enumerate() {
            let x = (i * (width - 1)) / series.len().max(2);
            let y = (height as f64 - 1.0) - (a / peak) * (height as f64 - 1.0);
            surface.plot(x as i32, y as i32, '#');
        }
    }

    fn readout(&self, params: &[f64]) -> String {
        let meta = self.meta();
        let omega = lever_value(&meta, params, 0);
        if omega > 1.02 {
            let series = scale_factor(omega);
            let peak = series.iter().copied().fold(0.0_f64, f64::max);
            format!(
                "Omega {omega:.2}: closed. Gravity wins. The universe expands to {peak:.1} times \
                 its start, halts, and falls back into a Big Crunch, a Big Bang run in reverse."
            )
        } else if omega < 0.98 {
            format!(
                "Omega {omega:.2}: open. There is not enough matter to stop it. Everything drifts \
                 apart forever into a cold, dark heat death. Bring a sweater."
            )
        } else {
            format!(
                "Omega {omega:.2}: flat, balanced on a knife edge. It expands forever, but ever \
                 more slowly, coasting to a stop it never quite reaches. This is our universe."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BigBang, scale_factor};
    use crate::canvas::Canvas;
    use crate::sim::Sim;

    #[test]
    fn a_dense_universe_recollapses() {
        // Omega > 1: the scale factor rises then returns toward zero.
        let series = scale_factor(1.5);
        let peak = series.iter().copied().fold(0.0_f64, f64::max);
        let last = *series.last().unwrap();
        assert!(peak > series[0]);
        assert!(last < peak, "a closed universe should turn around");
    }

    #[test]
    fn a_sparse_universe_keeps_growing() {
        let series = scale_factor(0.5);
        assert!(series.last().unwrap() > &series[0]);
    }

    #[test]
    fn omega_decides_the_fate() {
        assert!(BigBang::new().readout(&[1.5]).contains("Crunch"));
        assert!(BigBang::new().readout(&[0.5]).contains("heat death"));
        assert!(BigBang::new().readout(&[1.0]).contains("flat"));
    }

    #[test]
    fn render_has_ink_and_does_not_panic() {
        let sim = BigBang::new();
        let mut canvas = Canvas::new(50, 20);
        sim.render(&mut canvas, &[1.5]);
        assert!(canvas.ink_count() > 5);
        let mut empty = Canvas::new(0, 0);
        sim.render(&mut empty, &[]);
    }
}
