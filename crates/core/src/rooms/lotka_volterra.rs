//! Lotka-Volterra predator-prey: closed cycles of rabbits and foxes.
//!
//! DRAG: TUNE PREY RATE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

// Preserve the 48-unit observation horizon while resolving the sharpest
// cycles in the hand's alpha range. The first-integral test bounds the drift.
const STEPS: usize = 4_800;
const DT: f64 = 0.01;
const BETA: f64 = 0.5;
const DELTA: f64 = 0.4;
const ORBITS: usize = 6;

#[derive(Debug, Clone, Copy)]
struct Model {
    alpha: f64,
    gamma: f64,
}

impl Model {
    fn new(alpha: f64, seed: u64) -> Self {
        Self { alpha, gamma: 0.3 + (seed % 4) as f64 * 0.02 }
    }

    fn equilibrium(self) -> [f64; 2] {
        [self.gamma / DELTA, self.alpha / BETA]
    }

    // u=ln(x), v=ln(y) keep population positivity in the coordinates instead
    // of clamping an invalid population back into the physical state space.
    fn step(self, log_population: [f64; 2], dt: f64) -> [f64; 2] {
        crate::numerics::rk4(log_population, dt, |[u, v]| {
            [self.alpha - BETA * v.exp(), DELTA * u.exp() - self.gamma]
        })
    }

    fn trajectory(self, start: [f64; 2]) -> Vec<[f64; 2]> {
        let mut state = start.map(f64::ln);
        let mut points = Vec::with_capacity(STEPS + 1);
        points.push(start);
        for _ in 0..STEPS {
            state = self.step(state, DT);
            points.push(state.map(f64::exp));
        }
        points
    }
}

fn initial_population(index: usize) -> [f64; 2] {
    [0.5 + index as f64 * 0.35, 0.4 + index as f64 * 0.15]
}

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn prey_rate(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.6 + s
    } else {
        0.6 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, alpha: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let model = Model::new(alpha, seed);
    let orbits: Vec<_> = (0..ORBITS).map(|i| model.trajectory(initial_population(i))).collect();
    // All trajectories share one phase plane. Scaling each independently
    // invents intersections between distinct conserved level sets.
    let [max_x, max_y] = orbits.iter().flatten().fold([2.0_f64; 2], |bounds, point| {
        [bounds[0].max(point[0] * 1.1), bounds[1].max(point[1] * 1.1)]
    });
    for points in &orbits {
        for (i, &[px, py]) in points.iter().enumerate() {
            let u = px / max_x;
            let v = py / max_y;
            let ix = (u * width.saturating_sub(1) as f64).round() as i32;
            let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
            canvas.plot(ix, iy, if i % 17 == 0 { '#' } else { '*' });
        }
    }
    let [x, y] = model.equilibrium();
    let ix = (x / max_x * width.saturating_sub(1) as f64).round() as i32;
    let iy = ((1.0 - y / max_y) * height.saturating_sub(1) as f64).round() as i32;
    canvas.plot(ix, iy, 'o');
}

/// Lotka-Volterra room.
#[derive(Debug, Default)]
pub struct LotkaVolterra {
    seed: u64,
}

impl LotkaVolterra {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }
    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for LotkaVolterra {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, prey_rate(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lotka volterra",
            root: 220.0,
            tempo: 96,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "prey boom then predator boom in a closed loop",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PREY RATE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = prey_rate(t, None, self.seed);
        Some(format!("a={a:.2}  LV  DRAG:PREY"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = prey_rate(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = prey_rate(t, hands.last().copied(), self.seed);
        let [x_eq, y_eq] = Model::new(a, self.seed).equilibrium();
        Some(format!("a={a:.3}  eq~({x_eq:.2},{y_eq:.2})"))
    }

    fn reveal(&self) -> &'static str {
        "Lotka-Volterra is the classical predator-prey ODE: prey grow, predators \
         eat, predators die. Orbits close around a neutral equilibrium; real \
         ecology uses additional mechanisms such as density limits. These \
         numerical paths approximate the model's conserved contours."
    }
}

#[cfg(test)]
mod tests {
    use super::{LotkaVolterra, Model, ORBITS, initial_population, DT, BETA, DELTA};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LotkaVolterra::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("PREY"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn rate_changes() {
        let r = LotkaVolterra::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        LotkaVolterra::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn positive_orbits_preserve_the_first_integral_across_the_control_domain() {
        // H = delta*x - gamma*ln(x) + beta*y - alpha*ln(y).
        // Differentiating with the two ODEs gives dH/dt = 0. Forward Euler
        // instead grows H and fabricates outward spirals over the room's run.
        let mut worst_drift = 0.0_f64;
        for alpha in [0.4, 0.6, 1.0, 1.6, 2.0, 2.2] {
            for seed in 0..4 {
                let model = Model::new(alpha, seed);
                let invariant = |[x, y]: [f64; 2]| DELTA * x - model.gamma * x.ln() + BETA * y - alpha * y.ln();
                for orbit in 0..ORBITS {
                    let start = initial_population(orbit);
                    let initial = invariant(start);
                    let mut max_drift = 0.0_f64;
                    for state in model.trajectory(start) {
                        assert!(state.iter().all(|x| x.is_finite() && *x > 0.0));
                        max_drift = max_drift.max((invariant(state) - initial).abs());
                    }
                    assert!(max_drift < 1e-6, "alpha={alpha} gamma={} orbit={orbit}: drift={max_drift}", model.gamma);
                    worst_drift = worst_drift.max(max_drift);
                }
            }
        }
        eprintln!("Lotka-Volterra: maximum absolute first-integral drift over 144 sampled paths = {worst_drift:.3e}");
    }

    #[test]
    fn coexistence_equilibrium_is_stationary() {
        for seed in 0..4 {
            let model = Model::new(1.2, seed);
            let start = model.equilibrium().map(f64::ln);
            let next = model.step(start, DT);
            assert!((next[0] - start[0]).abs() < 1e-14);
            assert!((next[1] - start[1]).abs() < 1e-14);
        }
    }

    #[test]
    fn trajectory_refinement_has_fourth_order_convergence() {
        let model = Model::new(1.6, 2);
        let integrate = |steps| {
            let mut state = initial_population(0).map(f64::ln);
            for _ in 0..steps {
                state = model.step(state, 4.0 / steps as f64);
            }
            state
        };
        let distance = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        let coarse = integrate(100);
        let fine = integrate(200);
        let reference = integrate(400);
        let improvement = distance(coarse, fine) / distance(fine, reference);
        assert!((12.0..20.0).contains(&improvement), "refinement ratio={improvement}");
    }

    #[test]
    fn repeated_identical_tunings_do_not_change_the_predator_death_rate() {
        let room = LotkaVolterra::new_with(2);
        let mut once = Canvas::new(80, 40);
        let mut repeated = Canvas::new(80, 40);
        room.render_poked(&mut once, 0.3, &[(0.7, 0.5)]);
        room.render_poked(&mut repeated, 0.3, &[(0.7, 0.5); 2]);
        assert_eq!(once.to_text(), repeated.to_text());
        let status = room.status_input(0.3, &[RoomInput::PointerDown { x: 0.7, y: 0.5, t: 0.3 }]).unwrap();
        assert!(status.contains("eq~(0.85,3.24)"), "{status}");
    }
}
