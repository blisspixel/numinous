//! Van der Pol oscillator: nonlinear damping and finite phase portraits.
//!
//! x'' - mu (1 - x^2) x' + x = 0 in dimensionless time. The gallery selects
//! mu, while the hand also selects the initial position and velocity. A fixed
//! comparison start shares the same parameter, time horizon and coordinates.
//! See `docs/ROOMS.md` and `docs/MATHEMATICS.md`.

use super::phase_plane::PhasePlane;
use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;

// RK4 resolves the admitted mu <= 5.45 over a fixed 100-unit observation.
// The tests check both energy balance and step refinement on finite fixtures.
const STEPS: usize = 20_000;
const DT: f64 = 0.005;
const STATE_LIMIT: f64 = 50.0;
const REFERENCE_START: State = [2.5, 0.0];
const HIGHLIGHT_STEPS: usize = 800; // Final four dimensionless time units.

type State = [f64; 2]; // x, v = x'

#[derive(Debug, Clone, Copy, PartialEq)]
struct Experiment {
    mu: f64,
    initial: State,
}

impl Experiment {
    fn flow(self, [x, v]: State) -> State {
        [v, self.mu * (1.0 - x * x) * v - x]
    }

    fn step(self, state: State, dt: f64) -> State {
        crate::numerics::rk4(state, dt, |point| self.flow(point))
    }

    fn trajectory(self) -> Trajectory {
        let mut state = self.initial;
        let mut states = Vec::with_capacity(STEPS + 1);
        states.push(state);
        let mut termination = Termination::Horizon;
        for _ in 0..STEPS {
            let next = self.step(state, DT);
            if !next.into_iter().all(f64::is_finite) {
                termination = Termination::NonFinite;
                break;
            }
            if next.into_iter().any(|v| v.abs() > STATE_LIMIT) {
                termination = Termination::StateLimit;
                break;
            }
            states.push(next);
            state = next;
        }
        Trajectory {
            experiment: self,
            states,
            termination,
        }
    }

    fn reference(self) -> Self {
        Self {
            initial: REFERENCE_START,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Termination {
    Horizon,
    StateLimit,
    NonFinite,
}

struct Trajectory {
    experiment: Experiment,
    states: Vec<State>,
    termination: Termination,
}

impl Trajectory {
    fn amplitude(&self) -> f64 {
        let (min_x, max_x) = self
            .states
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &[x, _]| {
                (lo.min(x), hi.max(x))
            });
        // This is the entire retained range, including the initial transient.
        (max_x - min_x) * 0.5
    }

    fn elapsed(&self) -> f64 {
        (self.states.len() - 1) as f64 * DT
    }

    fn status(&self) -> String {
        let outcome = match self.termination {
            Termination::Horizon if self.experiment.initial == [0.0, 0.0] => "equilibrium",
            Termination::Horizon => "trace",
            Termination::StateLimit => "limit",
            Termination::NonFinite => "invalid",
        };
        format!(
            "mu~{:.2} A~{:.3} {outcome}@{:.2} DRAG:MU+START",
            self.experiment.mu,
            self.amplitude(),
            self.elapsed()
        )
    }
}

fn mu(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        0.2 + x * 4.5 + s
    } else {
        0.5 + phase_unit(t) * 3.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, selected: &[State], reference: &[State]) {
    let Some(plane) = PhasePlane::fit(
        canvas,
        selected.iter().chain(reference).map(|&[x, v]| (x, v)),
    ) else {
        return;
    };
    // Draw the comparison first so the selected path and its endpoint remain
    // visible, including the exact equilibrium, which has no moving segments.
    for (points, is_selected) in [(reference, false), (selected, true)] {
        for (i, pair) in points.windows(2).enumerate() {
            let a = plane.point(pair[0][0], pair[0][1]);
            let b = plane.point(pair[1][0], pair[1][1]);
            let mark = if !is_selected {
                '-'
            } else if i + HIGHLIGHT_STEPS >= points.len() - 1 {
                '#'
            } else {
                '*'
            };
            canvas.line(a.0, a.1, b.0, b.1, mark);
        }
    }
    if let Some(&[x, v]) = selected.last() {
        let point = plane.point(x, v);
        canvas.plot(point.0, point.1, 'o');
    }
}

/// Van der Pol room.
#[derive(Debug, Default)]
pub struct VanDerPol {
    seed: u64,
}

impl VanDerPol {
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

    fn experiment(&self, t: f64, pokes: &[(f64, f64)]) -> Experiment {
        let hand = latest_hand(pokes);
        Experiment {
            mu: mu(t, hand, self.seed),
            initial: hand.map_or([0.1, 0.0], |(x, y)| [(x - 0.5) * 4.0, (0.5 - y) * 4.0]),
        }
    }
}

impl Room for VanDerPol {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.render_poked(canvas, t, &[]);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "vdp",
            root: 130.81,
            tempo: 100,
            line: &[0, 3, 5, 7, 10, 7, 5, 3],
            encodes: "a melodic rise and return inspired by relaxation oscillations",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: MU + START")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.experiment(t, &[]).trajectory().status())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let experiment = self.experiment(t, pokes);
        let selected = experiment.trajectory();
        let reference = experiment.reference().trajectory();
        draw(canvas, &selected.states, &reference.states);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        Some(self.experiment(t, &pokes).trajectory().status())
    }

    fn reveal(&self) -> &'static str {
        "Van der Pol's nonlinear damping feeds motion when |x|<1 and drains \
         energy when |x|>1. In this model nonzero motion approaches an attracting \
         orbit, while the exact origin remains an unstable equilibrium. Reaching \
         the same orbit need not synchronize phase. Horizontal drag changes mu \
         and initial x; vertical drag changes initial velocity. A is half the \
         selected path's x range from time zero to the reported time, at most \
         100 dimensionless units. The reference starts at (2.5,0) on the same \
         axes. limit or invalid marks a numerical stop. These finite traces \
         include transients; the melody is an inspired phrase, not measured motion."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DT, Experiment, REFERENCE_START, STATE_LIMIT, STEPS, State, Termination, VanDerPol, draw,
        latest_hand,
    };
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput, inputs_from_pokes};
    use crate::surface::Surface;

    fn energy([x, v]: State) -> f64 {
        (x * x + v * v) * 0.5
    }

    fn evolved(experiment: Experiment, duration: f64, steps: usize) -> Vec<State> {
        let mut state = experiment.initial;
        let mut states = vec![state];
        for _ in 0..steps {
            state = experiment.step(state, duration / steps as f64);
            states.push(state);
        }
        states
    }

    fn distance(a: State, b: State) -> f64 {
        (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
    }

    fn half_range(states: &[State]) -> f64 {
        let max = states
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let min = states.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        (max - min) * 0.5
    }

    #[test]
    fn the_vector_field_has_the_declared_energy_balance() {
        // For E=(x^2+v^2)/2, the ODE implies E'=mu*(1-x^2)*v^2.
        // This identity checks both the restoring force and nonlinear damping.
        for mu in [0.0, 0.2, 2.45, 5.45] {
            let model = Experiment {
                mu,
                initial: [0.0; 2],
            };
            for x in [-2.5, -1.0, -0.25, 0.0, 0.25, 1.0, 2.5] {
                for v in [-2.0, -0.5, 0.0, 0.5, 2.0] {
                    let [dx, dv] = model.flow([x, v]);
                    let actual = x * dx + v * dv;
                    let expected = mu * (1.0 - x * x) * v * v;
                    assert!((actual - expected).abs() < 1e-12);
                    if mu > 0.0 && v != 0.0 {
                        assert_eq!(actual > 0.0, x.abs() < 1.0);
                        assert_eq!(actual < 0.0, x.abs() > 1.0);
                    }
                }
            }
        }
    }

    #[test]
    fn the_undamped_limit_matches_analytic_harmonic_motion() {
        let model = Experiment {
            mu: 0.0,
            initial: [1.3, -0.7],
        };
        for duration in [1.0, 4.0, std::f64::consts::TAU, 100.0] {
            let steps = (duration / DT).ceil() as usize;
            let path = evolved(model, duration, steps);
            let [x0, v0] = model.initial;
            let expected = [
                x0 * duration.cos() + v0 * duration.sin(),
                -x0 * duration.sin() + v0 * duration.cos(),
            ];
            assert!(distance(*path.last().unwrap(), expected) < 1e-8);
            for state in path {
                assert!((energy(state) - energy(model.initial)).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn exact_origin_is_an_equilibrium_and_stays_visible() {
        for seed in 0..6 {
            let room = VanDerPol::new_with(seed);
            let experiment = room.experiment(0.7, &[(0.5, 0.5)]);
            let selected = experiment.trajectory();
            assert_eq!(selected.termination, Termination::Horizon);
            assert_eq!(selected.elapsed(), 100.0);
            assert!(selected.states.iter().all(|p| *p == [0.0, 0.0]));
            assert_eq!(selected.amplitude(), 0.0);
            assert!(selected.status().contains("equilibrium@100.00"));
            let mut canvas = Canvas::new(61, 37);
            room.render_poked(&mut canvas, 0.7, &[(0.5, 0.5)]);
            assert_eq!(canvas.to_text().chars().filter(|ch| *ch == 'o').count(), 1);
            assert!(canvas.ink_count() > 10, "the comparison still moves");
        }
        let near = VanDerPol::new().experiment(0.0, &[(0.5_f64.next_up(), 0.5)]);
        assert_ne!(near.initial, [0.0, 0.0]);
        assert!(!near.trajectory().status().contains("equilibrium"));
    }

    #[test]
    fn sampled_controls_obey_balance_and_full_horizon_refinement_budgets() {
        let mut worst_balance = 0.0_f64;
        let mut worst_endpoint = 0.0_f64;
        let mut worst_amplitude = 0.0_f64;
        let mut largest_component = 0.0_f64;
        for seed in [0, 5] {
            let room = VanDerPol::new_with(seed);
            for x in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
                for y in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let experiment = room.experiment(0.0, &[(x, y)]);
                    let path = experiment.trajectory();
                    assert_eq!(path.termination, Termination::Horizon, "{experiment:?}");
                    assert_eq!(path.states.len(), STEPS + 1);
                    assert_eq!(path.states[0], experiment.initial);
                    assert_eq!(path.elapsed(), 100.0);
                    // Independent composite Simpson quadrature of the exact
                    // power law, rather than another integration of E'=power.
                    let mut power_integral = 0.0;
                    for (i, &[x, v]) in path.states.iter().enumerate() {
                        assert!(x.is_finite() && v.is_finite());
                        assert!(x.abs() <= STATE_LIMIT && v.abs() <= STATE_LIMIT);
                        largest_component = largest_component.max(x.abs()).max(v.abs());
                        let weight = if i == 0 || i == STEPS {
                            1.0
                        } else if i % 2 == 1 {
                            4.0
                        } else {
                            2.0
                        };
                        power_integral += weight * experiment.mu * (1.0 - x * x) * v * v;
                    }
                    power_integral *= DT / 3.0;
                    let last = *path.states.last().unwrap();
                    let balance =
                        (energy(last) - energy(experiment.initial) - power_integral).abs();
                    assert!(balance < 1e-4, "{experiment:?}, energy balance={balance}");
                    let refined = evolved(experiment, 100.0, 2 * STEPS);
                    let endpoint = distance(last, *refined.last().unwrap());
                    let amplitude = (path.amplitude() - half_range(&refined)).abs();
                    assert!(
                        endpoint < 5e-5,
                        "{experiment:?}, endpoint difference={endpoint}"
                    );
                    assert!(
                        amplitude < 1e-4,
                        "{experiment:?}, amplitude difference={amplitude}"
                    );
                    worst_balance = worst_balance.max(balance);
                    worst_endpoint = worst_endpoint.max(endpoint);
                    worst_amplitude = worst_amplitude.max(amplitude);
                }
            }
        }
        eprintln!(
            "Van der Pol: 70 controls, max balance={worst_balance:.3e}, endpoint refinement={worst_endpoint:.3e}, amplitude refinement={worst_amplitude:.3e}, largest component={largest_component:.6}"
        );
    }

    #[test]
    fn reference_and_ambient_paths_share_the_bounded_horizon() {
        for seed in 0..6 {
            let room = VanDerPol::new_with(seed);
            for phase in [0.0, 0.5, 1.0] {
                let experiment = room.experiment(phase, &[]);
                assert_eq!(experiment.initial, [0.1, 0.0]);
                for model in [experiment, experiment.reference()] {
                    let path = model.trajectory();
                    assert_eq!(path.termination, Termination::Horizon);
                    assert_eq!(path.elapsed(), 100.0);
                }
            }
        }
        for mu in [0.2, 2.45, 5.45] {
            let path = Experiment {
                mu,
                initial: REFERENCE_START,
            }
            .trajectory();
            assert_eq!(path.termination, Termination::Horizon);
        }
    }

    #[test]
    fn refined_independent_endpoint_fixtures_match_the_ode() {
        // DOP853 with rtol=atol=2.3e-14, maximum steps 0.02 and 0.01.
        // Those independent calculations agree within 7e-13 at t=100.
        // Degree-16 Taylor steps at 0.02 and 0.01 independently agree with
        // these references within 1e-12, using the polynomial ODE recurrence.
        // These are finite numerical references, not exact closed forms.
        for (mu, initial, expected, tolerance) in [
            (
                0.2,
                [-2.0, 0.0],
                [-1.3444388299446155, -1.633768398454978],
                1e-8,
            ),
            (
                0.5,
                [0.1, 0.0],
                [-0.5233408624678944, 1.7318478881450823],
                1e-8,
            ),
            (
                3.0,
                [2.0, 2.0],
                [1.618166034478782, -0.31113947594923475],
                1e-6,
            ),
            (
                5.45,
                [2.0, -2.0],
                [1.5725369866816237, -0.18984805280788897],
                1e-6,
            ),
            (
                5.0,
                [1.6, -2.0],
                [-0.16611529752525747, 3.548674683701946],
                5e-5,
            ),
        ] {
            let path = Experiment { mu, initial }.trajectory();
            let error = distance(*path.states.last().unwrap(), expected);
            assert!(
                error < tolerance,
                "mu={mu}, start={initial:?}, endpoint error={error}"
            );
        }
    }

    #[test]
    fn short_time_refinement_has_fourth_order_convergence() {
        for (mu, initial) in [(0.2, [-2.0, 0.0]), (2.45, [0.1, 0.0]), (5.45, [2.0, -2.0])] {
            let model = Experiment { mu, initial };
            let coarse = evolved(model, 1.0, 100);
            let fine = evolved(model, 1.0, 200);
            let reference = evolved(model, 1.0, 400);
            let improvement = distance(*coarse.last().unwrap(), *fine.last().unwrap())
                / distance(*fine.last().unwrap(), *reference.last().unwrap());
            assert!(
                (12.0..20.0).contains(&improvement),
                "mu={mu}, refinement ratio={improvement}"
            );
            eprintln!("Van der Pol: mu={mu}, one-unit refinement ratio={improvement:.3}");
        }
    }

    #[test]
    fn low_mu_late_amplitude_and_crossing_period_reject_euler_inflation() {
        let path = VanDerPol::new().experiment(0.0, &[(0.0, 0.5)]).trajectory();
        let late_amplitude = half_range(&path.states[15_000..]);
        let crossings: Vec<_> = path
            .states
            .windows(2)
            .enumerate()
            .filter(|(i, pair)| *i >= 15_000 && pair[0][0] < 0.0 && pair[1][0] >= 0.0)
            .map(|(i, pair)| DT * (i as f64 - pair[0][0] / (pair[1][0] - pair[0][0])))
            .collect();
        assert!(crossings.len() >= 3);
        let period = (crossings.last().unwrap() - crossings[0]) / (crossings.len() - 1) as f64;
        // Refined equation calculations at steps 0.0025 and 0.00125 give
        // amplitude about 2.000414 and period about 6.298877 over this window.
        assert!((late_amplitude - 2.00041367).abs() < 1e-5);
        assert!((period - 6.29887671).abs() < 1e-6);
        eprintln!("Van der Pol: mu=0.2, late amplitude={late_amplitude:.9}, period={period:.9}");
    }

    #[test]
    fn readout_amplitude_includes_the_selected_paths_transient() {
        let room = VanDerPol::new();
        let experiment = room.experiment(0.0, &[(0.0, 0.0)]);
        let path = experiment.trajectory();
        assert!(path.amplitude() > half_range(&path.states[15_000..]) + 0.1);
        assert_eq!(path.amplitude(), half_range(&path.states));
        assert!(
            path.status()
                .contains(&format!("A~{:.3} trace@100.00", path.amplitude()))
        );
    }

    #[test]
    fn accepted_controls_are_shared_by_geometry_and_readout() {
        let room = VanDerPol::new_with(5);
        let pokes = [(0.1, 0.2), (2.0, -3.0), (f64::NAN, 0.5)];
        let expected = Experiment {
            mu: 5.45,
            initial: [2.0, 2.0],
        };
        assert_eq!(room.experiment(0.0, &pokes), expected);
        assert_eq!(expected.reference().initial, REFERENCE_START);
        assert_eq!(expected.reference().mu, expected.mu);
        let inputs = inputs_from_pokes(&pokes, 0.0);
        let selected = expected.trajectory();
        let reference = expected.reference().trajectory();
        assert_eq!(room.status_input(0.0, &inputs), Some(selected.status()));
        let mut actual = Canvas::new(71, 35);
        room.render_input(&mut actual, 0.0, &inputs);
        let mut expected_frame = Canvas::new(71, 35);
        draw(&mut expected_frame, &selected.states, &reference.states);
        assert_eq!(actual.to_text(), expected_frame.to_text());
        let repeated = inputs_from_pokes(&[(1.0, 0.0); MAX_ROOM_POKES], 0.9);
        let mut later = Canvas::new(71, 35);
        room.render_input(&mut later, 1.0, &repeated);
        assert_eq!(actual.to_text(), later.to_text());
        assert_eq!(room.status_input(1.0, &repeated), Some(selected.status()));
    }

    #[test]
    fn empty_and_rejected_hands_retain_the_ambient_comparison() {
        let room = VanDerPol::new_with(u64::MAX);
        let mut stale = vec![(1.0, 0.0)];
        stale.extend([(f64::NAN, 0.5); MAX_ROOM_POKES]);
        assert_eq!(latest_hand(&stale), None);
        for pokes in [&[][..], &stale[..]] {
            let inputs = inputs_from_pokes(pokes, 0.0);
            assert_eq!(room.status_input(0.4, &inputs), room.status(0.4));
            let mut actual = Canvas::new(41, 23);
            room.render_poked(&mut actual, 0.4, pokes);
            let mut expected = Canvas::new(41, 23);
            room.render(&mut expected, 0.4);
            assert_eq!(actual.to_text(), expected.to_text());
        }
        for phase in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -f64::MAX] {
            assert_eq!(room.experiment(phase, &[]), room.experiment(0.0, &[]));
        }
        assert_eq!(room.experiment(f64::MAX, &[]), room.experiment(1.0, &[]));
        assert_eq!(latest_hand(&[(f64::MAX, -f64::MAX)]), Some((1.0, 0.0)));
        assert_eq!(latest_hand(&[(0.5, f64::INFINITY)]), None);
    }

    #[test]
    fn painted_square_keeps_equal_units_beside_a_wider_reference() {
        let square = [
            [-0.5, -0.5],
            [0.5, -0.5],
            [0.5, 0.5],
            [-0.5, 0.5],
            [-0.5, -0.5],
        ];
        let reference = [[-4.0, 0.0], [4.0, 0.0]];
        let extent = |points: Vec<(usize, usize)>| {
            let x_min = points.iter().map(|p| p.0).min().unwrap();
            let x_max = points.iter().map(|p| p.0).max().unwrap();
            let y_min = points.iter().map(|p| p.1).min().unwrap();
            let y_max = points.iter().map(|p| p.1).max().unwrap();
            ((x_max - x_min) as f64, (y_max - y_min) as f64)
        };
        for (width, height) in [(181, 101), (101, 181)] {
            let mut pixels = Raster::new(width, height);
            let blank = pixels.to_rgba();
            draw(&mut pixels, &[], &reference);
            let reference_pixels = pixels.to_rgba();
            let reference_extent = extent(
                reference_pixels
                    .chunks_exact(4)
                    .zip(blank.chunks_exact(4))
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, _)| (i % width, i / width))
                    .collect(),
            );
            let mut both = Raster::new(width, height);
            draw(&mut both, &square, &reference);
            let square_extent = extent(
                both.to_rgba()
                    .chunks_exact(4)
                    .zip(reference_pixels.chunks_exact(4))
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, _)| (i % width, i / width))
                    .collect(),
            );
            assert!((square_extent.0 - square_extent.1).abs() <= 1.0);
            // The line is eight coordinate units long, the square one unit.
            // Pixel rounding allows at most one unit per measured extent.
            assert!((reference_extent.0 - 8.0 * square_extent.0).abs() <= 9.0);

            let mut cells = Canvas::new(width, height);
            draw(&mut cells, &[], &reference);
            let reference_extent = extent(
                cells
                    .to_text()
                    .lines()
                    .enumerate()
                    .flat_map(|(y, row)| {
                        row.chars()
                            .enumerate()
                            .filter(|(_, ch)| *ch == '-')
                            .map(move |(x, _)| (x, y))
                    })
                    .collect(),
            );
            let mut both = Canvas::new(width, height);
            draw(&mut both, &square, &reference);
            let square_extent = extent(
                both.to_text()
                    .lines()
                    .enumerate()
                    .flat_map(|(y, row)| {
                        row.chars()
                            .enumerate()
                            .filter(|(_, ch)| matches!(*ch, '*' | '#' | 'o'))
                            .map(move |(x, _)| (x, y))
                    })
                    .collect(),
            );
            assert!((square_extent.0 - 2.0 * square_extent.1).abs() <= 2.0);
            assert!((reference_extent.0 - 8.0 * square_extent.0).abs() <= 9.0);
        }
    }

    #[test]
    fn hostile_surfaces_preserve_bounded_work_and_coordinates() {
        struct CheckedSurface {
            width: usize,
            height: usize,
            aspect: f64,
            plots: usize,
        }
        impl Surface for CheckedSurface {
            fn width(&self) -> usize {
                self.width
            }
            fn height(&self) -> usize {
                self.height
            }
            fn char_aspect(&self) -> f64 {
                self.aspect
            }
            fn plot(&mut self, x: i32, y: i32, _: char) {
                let (w, h) = self.draw_bounds();
                assert!(x >= 0 && (x as usize) < w);
                assert!(y >= 0 && (y as usize) < h);
                self.plots += 1;
            }
        }
        let experiment = VanDerPol::new_with(5).experiment(0.0, &[(1.0, 0.0)]);
        let selected = experiment.trajectory();
        let reference = experiment.reference().trajectory();
        for (width, height) in [
            (0, 0),
            (0, 5),
            (5, 0),
            (1, 1),
            (1, 9),
            (9, 1),
            (2, 2),
            (80, 40),
            (usize::MAX, 1),
            (1, usize::MAX),
        ] {
            for aspect in [
                0.5,
                1.0,
                0.0,
                f64::NAN,
                f64::INFINITY,
                f64::MAX,
                f64::MIN_POSITIVE,
            ] {
                let mut surface = CheckedSurface {
                    width,
                    height,
                    aspect,
                    plots: 0,
                };
                draw(&mut surface, &selected.states, &reference.states);
                assert_eq!(surface.plots > 0, width > 0 && height > 0);
                let (w, h) = surface.draw_bounds();
                assert!(surface.plots <= 2 * STEPS * w.max(h) + 1);
            }
        }
    }

    #[test]
    fn highlight_covers_exactly_the_last_four_time_units() {
        struct Marks {
            bright: usize,
        }
        impl Surface for Marks {
            fn width(&self) -> usize {
                40
            }
            fn height(&self) -> usize {
                20
            }
            fn plot(&mut self, _: i32, _: i32, _: char) {}
            fn line(&mut self, _: i32, _: i32, _: i32, _: i32, mark: char) {
                self.bright += usize::from(mark == '#');
            }
        }
        let mut marks = Marks { bright: 0 };
        draw(&mut marks, &vec![[0.0, 0.0]; STEPS + 1], &[]);
        assert_eq!(marks.bright as f64 * DT, 4.0);
    }

    #[test]
    fn status_invites() {
        for seed in [0, 5, u64::MAX] {
            let room = VanDerPol::new_with(seed);
            for pokes in [
                &[][..],
                &[(0.0, 0.0)][..],
                &[(0.5, 0.5)][..],
                &[(1.0, 1.0)][..],
            ] {
                let status = room.experiment(0.3, pokes).trajectory().status();
                assert!(status.contains("DRAG:MU+START"));
                assert!(status.chars().count() <= 56, "{status}");
            }
            assert_eq!(room.verb(), Some("DRAG: MU + START"));
        }
    }

    #[test]
    fn tune_changes() {
        let r = VanDerPol::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        VanDerPol::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(VanDerPol::new().motif().unwrap().line.len() >= 6);
    }
}
