//! Tribble population: logistic growth that gets adorable, then catastrophic.
//!
//! Levers set the breeding rate, the food supply (carrying capacity), and how
//! many days pass. Turn the breeding rate up past the edge of chaos and the
//! population explodes and starves, exactly the logistic map, now with fur.

use crate::sim::{Lever, Sim, SimMeta, lever_value};
use crate::surface::Surface;

/// The levers, in `params` order.
const LEVERS: [Lever; 3] = [
    Lever {
        name: "breeding-rate",
        min: 0.1,
        max: 3.0,
        default: 1.6,
        unit: "per day",
    },
    Lever {
        name: "food",
        min: 20.0,
        max: 5_000.0,
        default: 600.0,
        unit: "capacity",
    },
    Lever {
        name: "days",
        min: 1.0,
        max: 80.0,
        default: 30.0,
        unit: "days",
    },
];

/// The Tribble population sim.
#[derive(Debug, Default)]
pub struct Tribbles;

impl Tribbles {
    /// Create the sim.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// The population each day under the discrete logistic model.
fn simulate(rate: f64, capacity: f64, days: usize) -> Vec<f64> {
    let mut population = 1.0;
    let mut series = Vec::with_capacity(days + 1);
    series.push(population);
    for _ in 0..days {
        population = (population + rate * population * (1.0 - population / capacity)).max(0.0);
        series.push(population);
    }
    series
}

impl Sim for Tribbles {
    fn meta(&self) -> SimMeta {
        SimMeta {
            id: "tribbles",
            title: "Tribble Population",
            blurb: "Set the breeding rate and the food, and watch a logistic model with fur go \
                    from a cute purring pile to a ship-eating plague. Optimize it or break it.",
            accent: [210, 160, 90],
            levers: &LEVERS,
        }
    }

    fn render(&self, surface: &mut dyn Surface, params: &[f64]) {
        let meta = self.meta();
        let rate = lever_value(&meta, params, 0);
        let capacity = lever_value(&meta, params, 1);
        let days = lever_value(&meta, params, 2) as usize;
        let width = surface.width();
        let height = surface.height();
        if width == 0 || height == 0 {
            return;
        }
        let series = simulate(rate, capacity, days);
        let peak = series.iter().copied().fold(1.0_f64, f64::max);
        // Plot population over time, scaled to the peak so busts are visible.
        for (i, &pop) in series.iter().enumerate() {
            let x = (i * (width - 1)) / series.len().max(2);
            let y = height as f64 - (pop / peak) * (height as f64 - 1.0);
            surface.plot(x as i32, y as i32, '#');
        }
    }

    fn readout(&self, params: &[f64]) -> String {
        let meta = self.meta();
        let rate = lever_value(&meta, params, 0);
        let capacity = lever_value(&meta, params, 1);
        let days = lever_value(&meta, params, 2) as usize;
        let series = simulate(rate, capacity, days);
        let final_pop = *series.last().unwrap_or(&0.0);
        // How much the population is still swinging at the end tells boom-and-bust
        // chaos apart from a settled carpet.
        let tail = &series[series.len().saturating_sub(8)..];
        let tail_min = tail.iter().copied().fold(f64::INFINITY, f64::min);
        let tail_max = tail.iter().copied().fold(0.0_f64, f64::max);
        let swing = tail_max - tail_min;

        if final_pop < 2.0 {
            format!(
                "They bred, ate everything, and starved to {final_pop:.0}. They're all dead, Jim."
            )
        } else if swing > capacity * 0.2 {
            format!(
                "Boom and bust: they swing wildly between {tail_min:.0} and {tail_max:.0}, never \
                 settling. Pure chaos, with fur."
            )
        } else if final_pop > capacity * 0.7 {
            format!(
                "A stable purring carpet of {final_pop:.0} tribbles fills the hold. \
                 Quartermaster is not amused."
            )
        } else {
            format!("Still climbing: {final_pop:.0} tribbles after {days} days, and rising.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Tribbles, simulate};
    use crate::canvas::Canvas;
    use crate::sim::Sim;

    #[test]
    fn moderate_growth_settles_near_capacity() {
        let series = simulate(1.0, 500.0, 200);
        let final_pop = *series.last().unwrap();
        assert!((final_pop - 500.0).abs() < 5.0, "settled at {final_pop}");
    }

    #[test]
    fn zero_start_grows() {
        let series = simulate(1.5, 600.0, 10);
        assert!(series.last().unwrap() > &series[0]);
    }

    #[test]
    fn readout_reports_a_stable_carpet_at_calm_settings() {
        let text = Tribbles::new().readout(&[1.0, 500.0, 200.0]);
        assert!(
            text.contains("stable") || text.contains("carpet"),
            "got: {text}"
        );
    }

    #[test]
    fn wild_breeding_causes_boom_and_bust() {
        // Rate near 3 drives the logistic model chaotic (overshoot and crash).
        let text = Tribbles::new().readout(&[2.9, 400.0, 60.0]);
        assert!(
            text.contains("bust") || text.contains("dead") || text.contains("Chaos"),
            "got: {text}"
        );
    }

    #[test]
    fn render_has_ink_and_does_not_panic() {
        let sim = Tribbles::new();
        let mut canvas = Canvas::new(50, 20);
        sim.render(&mut canvas, &[1.6, 600.0, 30.0]);
        assert!(canvas.ink_count() > 3);
        let mut empty = Canvas::new(0, 0);
        sim.render(&mut empty, &[]);
    }
}
