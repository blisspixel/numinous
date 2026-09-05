//! Henon-Heiles system: finite trajectories in a galactic model potential.
//!
//! The selected energy belongs to one initial state, shared by the picture and
//! readout. Gallery phase selects energy, not elapsed physical time. RK4 is a
//! finite-horizon approximation, not an exactly energy-preserving method.
//! See `docs/ROOMS.md` and `docs/MATHEMATICS.md`.

use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;

// Dimensionless time: at most 100 units, with an explicit spatial cutoff.
// Sampled-domain energy and short-time refinement budgets live in the tests.
const STEPS: usize = 10_000;
const DT: f64 = 0.01;
const COORD_LIMIT: f64 = 3.0;
const SADDLE_ENERGY: f64 = 1.0 / 6.0;

type State = [f64; 4]; // x, y, px, py

fn initial_state(e: f64) -> State {
    // V(0, 0.1) = 7/1500. Equal momentum components spend kinetic energy p^2.
    let p = (e - 7.0 / 1500.0).sqrt();
    [0.0, 0.1, p, p]
}

fn flow([x, y, px, py]: State) -> State {
    // Hamilton's equations for H = (px^2+py^2+x^2+y^2)/2 + x^2*y - y^3/3.
    [px, py, -x - 2.0 * x * y, -y - x * x + y * y]
}

fn step(state: State, dt: f64) -> State {
    crate::numerics::rk4(state, dt, flow)
}

fn barrier(e: f64) -> &'static str {
    if e < SADDLE_ENERGY {
        "closed"
    } else if e > SADDLE_ENERGY {
        "open"
    } else {
        "saddle"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Termination {
    Horizon,
    BoxLimit,
    NonFinite,
}

struct Trajectory {
    energy: f64,
    states: Vec<State>,
    termination: Termination,
}

impl Trajectory {
    fn new(energy: f64) -> Self {
        let mut state = initial_state(energy);
        let mut states = Vec::with_capacity(STEPS + 1);
        states.push(state);
        let mut termination = Termination::Horizon;
        for _ in 0..STEPS {
            let next = step(state, DT);
            if !next.iter().all(|v| v.is_finite()) {
                termination = Termination::NonFinite;
                break;
            }
            if next[0].abs() > COORD_LIMIT || next[1].abs() > COORD_LIMIT {
                termination = Termination::BoxLimit;
                break;
            }
            states.push(next);
            state = next;
        }
        Self {
            energy,
            states,
            termination,
        }
    }

    fn elapsed(&self) -> f64 {
        (self.states.len() - 1) as f64 * DT
    }

    fn status(&self) -> String {
        // The time belongs to the last retained sample. "box" means the next
        // numerical step left the box, not proof of an orbit escaping forever.
        let end = match self.termination {
            Termination::Horizon => "end",
            Termination::BoxLimit => "box",
            Termination::NonFinite => "invalid",
        };
        format!(
            "E~{:.3} {} {end}@{:.2} DRAG:ENERGY",
            self.energy,
            barrier(self.energy),
            self.elapsed()
        )
    }
}

fn energy(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.005
    };
    if let Some((x, _)) = hand {
        0.05 + x * 0.15 + s
    } else {
        0.08 + phase_unit(t) * 0.08 + s
    }
}

fn draw(canvas: &mut dyn Surface, pts: &[State]) {
    if pts.len() < 2 {
        return;
    }
    let Some(plane) =
        super::phase_plane::PhasePlane::fit(canvas, pts.iter().map(|state| (state[0], state[1])))
    else {
        return;
    };
    let mut prev: Option<(i32, i32)> = None;
    for (i, &[x, y, _, _]) in pts.iter().enumerate() {
        let (px, py) = plane.point(x, y);
        if let Some(o) = prev {
            // Four hundred complete steps span the final four time units.
            let ch = if i + 400 >= pts.len() { '#' } else { '*' };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Henon-Heiles room.
#[derive(Debug, Default)]
pub struct HenonHeiles {
    seed: u64,
}

impl HenonHeiles {
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

    fn trajectory(&self, t: f64, pokes: &[(f64, f64)]) -> Trajectory {
        Trajectory::new(energy(t, latest_hand(pokes), self.seed))
    }
}

impl Room for HenonHeiles {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.render_poked(canvas, t, &[]);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "henon heiles",
            root: 82.41,
            tempo: 78,
            line: &[0, 2, 5, 9, 14, 9, 5, 2],
            encodes: "a melodic rise and return inspired by galactic orbits",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ENERGY")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.trajectory(t, &[]).status())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw(canvas, &self.trajectory(t, pokes).states);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        Some(self.trajectory(t, &pokes).status())
    }

    fn reveal(&self) -> &'static str {
        "Henon and Heiles explored a cubic model of galactic motion. Drag \
         selects energy, not elapsed time. The three saddle barriers open above \
         E=1/6; that permits escape without proving this orbit escapes or is \
         chaotic. Each trace runs for at most 100 dimensionless time units. \
         The readout gives its last sampled time: end means the horizon, box \
         means the next numerical step left |x|,|y|<=3, and invalid means it \
         became nonfinite. The picture fits each orbit with equal physical \
         axis units. These are finite approximations; the melody is \
         an orbit-inspired phrase, not a measurement of the path."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COORD_LIMIT, DT, HenonHeiles, SADDLE_ENERGY, STEPS, State, Termination, Trajectory,
        barrier, draw, energy, flow, initial_state, latest_hand, step,
    };
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput, inputs_from_pokes};
    use crate::surface::Surface;

    // Evaluate the declared Hamiltonian independently of the force and the
    // constant used to initialize the momentum.
    fn hamiltonian([x, y, px, py]: State) -> f64 {
        (px * px + py * py + x * x + y * y) / 2.0 + x * x * y - y.powi(3) / 3.0
    }

    fn evolved(e: f64, duration: f64, steps: usize) -> State {
        let mut state = initial_state(e);
        for _ in 0..steps {
            state = step(state, duration / steps as f64);
        }
        state
    }

    fn distance(a: State, b: State) -> f64 {
        a.into_iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn selected_energy_is_the_initial_hamiltonian() {
        for seed in [0, 1, 2, 3, 4, u64::MAX] {
            for index in 0..=100 {
                let phase = index as f64 / 100.0;
                for hand in [None, Some((phase, 0.5))] {
                    let e = energy(phase, hand, seed);
                    assert!((0.05..=0.22).contains(&e), "E={e}");
                    let actual = hamiltonian(initial_state(e));
                    assert!((actual - e).abs() < 1e-14, "E={e}, H={actual}");
                }
            }
        }
        // In particular, the highest admitted energy is above the barrier.
        assert!(hamiltonian(initial_state(0.22)) > SADDLE_ENERGY);
    }

    #[test]
    fn equations_follow_the_hamiltonian_gradient() {
        for state in [
            [0.2, -0.3, 0.4, -0.5],
            [-0.7, 0.6, -0.1, 0.8],
            [0.0, 0.1, 0.2, 0.2],
        ] {
            let gradient: State = std::array::from_fn(|i| {
                let mut plus = state;
                let mut minus = state;
                plus[i] += 1e-5;
                minus[i] -= 1e-5;
                (hamiltonian(plus) - hamiltonian(minus)) / 2e-5
            });
            let expected = [gradient[2], gradient[3], -gradient[0], -gradient[1]];
            assert!(distance(flow(state), expected) < 1e-9);
        }
    }

    #[test]
    fn three_stationary_saddles_share_the_exact_barrier_height() {
        for [x, y] in [
            [0.0, 1.0],
            [3.0_f64.sqrt() / 2.0, -0.5],
            [-3.0_f64.sqrt() / 2.0, -0.5],
        ] {
            let state = [x, y, 0.0, 0.0];
            assert!(distance(flow(state), [0.0; 4]) < 1e-15);
            assert!((hamiltonian(state) - 1.0 / 6.0).abs() < 1e-15);
            // det Hess(V) = -3 at each saddle, so its two curvatures differ
            // in sign. This is an escape barrier, not a chaos threshold.
            let determinant = (1.0 + 2.0 * y) * (1.0 - 2.0 * y) - 4.0 * x * x;
            assert!((determinant + 3.0).abs() < 1e-14);
            assert!(distance(step(state, DT), state) < 1e-15);
        }
        assert_eq!(flow([0.0; 4]), [0.0; 4]);
        assert_eq!(barrier(SADDLE_ENERGY.next_down()), "closed");
        assert_eq!(barrier(SADDLE_ENERGY), "saddle");
        assert_eq!(barrier(SADDLE_ENERGY.next_up()), "open");
    }

    #[test]
    fn sampled_control_domain_respects_finite_horizon_energy_budget() {
        let mut worst_drift = 0.0_f64;
        let mut completed = 0;
        let mut box_limited = 0;
        // Include both control extremes and 169 evenly spaced interior values.
        for index in 0..=170 {
            let e = 0.05 + index as f64 * 0.001;
            let trajectory = Trajectory::new(e);
            assert_eq!(trajectory.states[0], initial_state(e));
            assert!(trajectory.states.len() <= STEPS + 1);
            for &state in &trajectory.states {
                assert!(state.into_iter().all(f64::is_finite));
                assert!(state[0].abs() <= COORD_LIMIT && state[1].abs() <= COORD_LIMIT);
                let drift = (hamiltonian(state) - e).abs();
                assert!(drift < 1e-7, "E={e}, energy drift={drift}");
                worst_drift = worst_drift.max(drift);
            }
            if e < SADDLE_ENERGY {
                assert_eq!(trajectory.termination, Termination::Horizon, "E={e}");
            }
            match trajectory.termination {
                Termination::Horizon => {
                    completed += 1;
                    assert_eq!(trajectory.states.len(), STEPS + 1);
                    assert_eq!(trajectory.elapsed(), 100.0);
                }
                Termination::BoxLimit => {
                    box_limited += 1;
                    let next = step(*trajectory.states.last().unwrap(), DT);
                    assert!(next[0].abs() > COORD_LIMIT || next[1].abs() > COORD_LIMIT);
                }
                Termination::NonFinite => panic!("nonfinite trajectory at E={e}"),
            }
        }
        eprintln!(
            "Henon-Heiles: 171 energies, max |H-E|={worst_drift:.3e}, \
             {completed} at horizon, {box_limited} at box limit"
        );
    }

    #[test]
    fn short_time_refinement_converges_at_fourth_order() {
        for e in [0.05, 0.12, 0.16, 0.20] {
            let coarse = evolved(e, 4.0, 200);
            let fine = evolved(e, 4.0, 400);
            let reference = evolved(e, 4.0, 800);
            let fine_error = distance(fine, reference);
            let improvement = distance(coarse, fine) / fine_error;
            assert!(
                (14.0..18.0).contains(&improvement),
                "E={e}, ratio={improvement}"
            );
            assert!(fine_error < 1e-9, "E={e}, refined difference={fine_error}");
            eprintln!("Henon-Heiles: E={e}, four-unit refinement ratio={improvement:.3}");
        }
    }

    #[test]
    fn low_energy_fixtures_remain_close_under_full_horizon_refinement() {
        // Energy conservation alone does not guarantee trajectory accuracy.
        // These three fixtures also check position and momentum at t=100.
        // No such pointwise long-time contract is imposed on chaotic paths.
        for e in [0.05, 0.08, 0.12] {
            let trajectory = Trajectory::new(e);
            let refined = evolved(e, 100.0, 2 * STEPS);
            let error = distance(*trajectory.states.last().unwrap(), refined);
            assert!(error < 5e-8, "E={e}, endpoint difference={error}");
        }
    }

    #[test]
    fn open_barrier_and_observed_box_limit_are_different_outcomes() {
        let completed = Trajectory::new(0.17);
        assert_eq!(completed.termination, Termination::Horizon);
        assert_eq!(completed.status(), "E~0.170 open end@100.00 DRAG:ENERGY");

        let limited = Trajectory::new(0.22);
        assert_eq!(limited.termination, Termination::BoxLimit);
        assert!((limited.elapsed() - 11.37).abs() < 1e-12);
        assert_eq!(limited.status(), "E~0.220 open box@11.37 DRAG:ENERGY");
        let last = *limited.states.last().unwrap();
        assert!(last[1] <= 3.0 && step(last, DT)[1] > 3.0);
        assert_eq!(limited.elapsed(), (limited.states.len() - 1) as f64 * DT);
    }

    #[test]
    fn accepted_tuning_owns_both_picture_and_readout() {
        let room = HenonHeiles::new_with(4);
        let pokes = [(0.1, 0.5), (2.0, -3.0), (f64::NAN, 0.5)];
        let inputs = inputs_from_pokes(&pokes, 0.0);
        assert_eq!(latest_hand(&pokes), Some((1.0, 0.0)));
        let expected = Trajectory::new(0.22);
        assert_eq!(room.status_input(0.3, &inputs), Some(expected.status()));

        let mut actual = Canvas::new(71, 35);
        room.render_input(&mut actual, 0.3, &inputs);
        let mut reference = Canvas::new(71, 35);
        draw(&mut reference, &expected.states);
        assert_eq!(actual.to_text(), reference.to_text());

        let repeated = inputs_from_pokes(&[(1.0, 0.0); MAX_ROOM_POKES], 0.0);
        let mut later = Canvas::new(71, 35);
        room.render_input(&mut later, 1.0, &repeated);
        assert_eq!(actual.to_text(), later.to_text());
        assert_eq!(room.status_input(1.0, &repeated), Some(expected.status()));
    }

    #[test]
    fn hostile_input_is_bounded_before_choosing_energy() {
        let room = HenonHeiles::new_with(u64::MAX);
        for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -f64::MAX] {
            assert_eq!(room.status(t), room.status(0.0));
        }
        assert_eq!(room.status(f64::MAX), room.status(1.0));
        let mut stale = vec![(1.0, 0.5)];
        stale.extend([(f64::NAN, 0.5); MAX_ROOM_POKES]);
        assert_eq!(latest_hand(&stale), None);
        let inputs = inputs_from_pokes(&stale, 0.0);
        assert_eq!(room.status_input(0.2, &inputs), room.status(0.2));

        let mut actual = Canvas::new(41, 23);
        room.render_poked(&mut actual, f64::NAN, &stale);
        let mut reference = Canvas::new(41, 23);
        room.render(&mut reference, 0.0);
        assert_eq!(actual.to_text(), reference.to_text());
        assert_eq!(latest_hand(&[(f64::MAX, -f64::MAX)]), Some((1.0, 0.0)));
        assert_eq!(latest_hand(&[(0.5, f64::NEG_INFINITY)]), None);
    }

    #[test]
    fn small_and_hostile_surfaces_keep_drawing_bounded() {
        struct CheckedSurface {
            width: usize,
            height: usize,
            plots: usize,
        }
        impl Surface for CheckedSurface {
            fn width(&self) -> usize {
                self.width
            }
            fn height(&self) -> usize {
                self.height
            }
            fn plot(&mut self, x: i32, y: i32, _: char) {
                let (width, height) = self.draw_bounds();
                assert!(x >= 0 && (x as usize) < width);
                assert!(y >= 0 && (y as usize) < height);
                self.plots += 1;
            }
        }
        for e in [0.05, 0.17, 0.22] {
            let trajectory = Trajectory::new(e);
            for (width, height) in [
                (0, 0),
                (0, 5),
                (5, 0),
                (1, 1),
                (1, 17),
                (19, 1),
                (2, 2),
                (40, 28),
                (usize::MAX, 1),
                (1, usize::MAX),
            ] {
                let mut surface = CheckedSurface {
                    width,
                    height,
                    plots: 0,
                };
                draw(&mut surface, &trajectory.states);
                assert_eq!(surface.plots > 0, width > 0 && height > 0);
                let (bounded_width, bounded_height) = surface.draw_bounds();
                assert!(surface.plots <= STEPS * bounded_width.max(bounded_height));
            }
        }
    }

    #[test]
    fn painted_unit_square_preserves_equal_units_on_raster_and_canvas() {
        let points = [
            [-0.5, -0.5, 0.0, 0.0],
            [0.5, -0.5, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [-0.5, 0.5, 0.0, 0.0],
            [-0.5, -0.5, 0.0, 0.0],
        ];
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
            draw(&mut pixels, &points);
            let occupied = pixels
                .to_rgba()
                .chunks_exact(4)
                .zip(blank.chunks_exact(4))
                .enumerate()
                .filter(|(_, (actual, background))| actual != background)
                .map(|(i, _)| (i % width, i / width))
                .collect();
            let (x_span, y_span) = extent(occupied);
            assert!((x_span - y_span).abs() <= 1.0, "pixels: {x_span}, {y_span}");

            let mut cells = Canvas::new(width, height);
            draw(&mut cells, &points);
            let occupied = cells
                .to_text()
                .lines()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.chars()
                        .enumerate()
                        .filter(|(_, ch)| !ch.is_whitespace())
                        .map(move |(x, _)| (x, y))
                })
                .collect();
            let (x_span, y_span) = extent(occupied);
            // A terminal cell is twice as tall as it is wide.
            assert!(
                (x_span - 2.0 * y_span).abs() <= 2.0,
                "cells: {x_span}, {y_span}"
            );
        }
    }

    #[test]
    fn status_invites() {
        for e in [0.05, 0.16, SADDLE_ENERGY, 0.17, 0.22] {
            let s = Trajectory::new(e).status();
            assert!(s.contains("DRAG:ENERGY"));
            assert!(s.chars().count() <= 56, "{s}");
        }
        assert_eq!(HenonHeiles::new().verb(), Some("DRAG: TUNE ENERGY"));
    }

    #[test]
    fn tune_changes() {
        let r = HenonHeiles::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        HenonHeiles::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(HenonHeiles::new().motif().unwrap().line.len() >= 6);
    }
}
