//! Duffing oscillator: finite driven, damped double-well portraits.
//!
//! x'' + delta x' + alpha x + beta x^3 = gamma cos(omega t).
//! DRAG: TUNE DRIVE. See `docs/ROOMS.md`.

use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;

// Dimensionless physical time, independent of the gallery phase that tunes g.
const STEPS: usize = 12_000;
const DT: f64 = 0.01;
const DAMPING: f64 = 0.3;
const DRIVE_FREQUENCY: f64 = 1.2;
const INITIAL: State = [0.1, 0.0];
type State = [f64; 2]; // position and velocity

fn gamma(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.1 + x * 0.8 + s
    } else {
        0.2 + phase_unit(t) * 0.5 + s
    }
}

fn flow([x, v]: State, time: f64, drive: f64) -> State {
    [
        v,
        x - x.powi(3) - DAMPING * v + drive * (DRIVE_FREQUENCY * time).cos(),
    ]
}

fn step(state: State, time: f64, dt: f64, drive: f64) -> State {
    // Augment with the clock so every RK stage evaluates the force at its
    // own time. The outer step index supplies the next clock without drift.
    let [x, v, _] = crate::numerics::rk4([state[0], state[1], time], dt, |[x, v, t]| {
        let [dx, dv] = flow([x, v], t, drive);
        [dx, dv, 1.0]
    });
    [x, v]
}

struct Trajectory {
    drive: f64,
    states: Vec<State>,
}

impl Trajectory {
    fn new(drive: f64) -> Self {
        let mut state = INITIAL;
        let mut states = Vec::with_capacity(STEPS + 1);
        states.push(state);
        for index in 0..STEPS {
            let next = step(state, index as f64 * DT, DT, drive);
            if !next.into_iter().all(f64::is_finite) {
                break;
            }
            states.push(next);
            state = next;
        }
        Self { drive, states }
    }

    fn elapsed(&self) -> f64 {
        (self.states.len() - 1) as f64 * DT
    }

    fn peaks(&self) -> State {
        self.states.iter().fold([0.0_f64; 2], |[x, v], state| {
            [x.max(state[0].abs()), v.max(state[1].abs())]
        })
    }

    fn status(&self) -> String {
        let [x, v] = self.peaks();
        let end = if self.states.len() == STEPS + 1 {
            "end"
        } else {
            "invalid"
        };
        format!(
            "DRAG:g~{:.2} max|x|~{x:.2} max|v|~{v:.2} {end}@{:.2}",
            self.drive,
            self.elapsed(),
        )
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
    let mut previous = plane.point(pts[0][0], pts[0][1]);
    for (i, &[x, v]) in pts.iter().enumerate().skip(1) {
        let current = plane.point(x, v);
        // Preserve the eight-time-unit highlight when refining the step.
        let ch = if i + 800 >= pts.len() {
            '#'
        } else if i % 10 == 0 {
            '+'
        } else {
            '*'
        };
        canvas.line(previous.0, previous.1, current.0, current.1, ch);
        previous = current;
    }
}

/// Duffing oscillator room.
#[derive(Debug, Default)]
pub struct Duffing {
    seed: u64,
}

impl Duffing {
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
        Trajectory::new(gamma(t, latest_hand(pokes), self.seed))
    }
}

impl Room for Duffing {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.render_poked(canvas, t, &[]);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "duffing",
            root: 123.47,
            tempo: 86,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "a rising phrase inspired by a driven double-well spring",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DRIVE")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.trajectory(t, &[]).status())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw(canvas, &self.trajectory(t, pokes).states);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        Some(
            self.trajectory(t, &crate::pokes_from_inputs(inputs))
                .status(),
        )
    }

    fn reveal(&self) -> &'static str {
        "Duffing's cubic spring has potential -x^2/2+x^4/4, with wells at \
         x=-1 and x=1. Drag selects drive strength, not elapsed time. The \
         picture shows position against velocity from (0.1,0), for at most \
         120 dimensionless time units; damping is 0.3 and drive frequency 1.2. \
         The readout gives sampled maximum absolute position and velocity \
         over that same trace, followed by its last retained time. An invalid \
         end marks a nonfinite numerical step. Each portrait fits its own \
         extent with equal axis units. Large motion or stronger drive alone \
         does not establish chaos. The melody is a spring-inspired phrase, \
         not a measurement of the trajectory."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DAMPING, DRIVE_FREQUENCY, DT, Duffing, INITIAL, STEPS, State, Trajectory, draw, flow,
        gamma, latest_hand, step,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput, inputs_from_pokes};

    fn energy([x, v]: State) -> f64 {
        0.5 * v * v - 0.5 * x * x + 0.25 * x.powi(4)
    }

    fn distance(a: State, b: State) -> f64 {
        (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
    }

    fn evolved(drive: f64, time: f64, steps: usize) -> State {
        let dt = time / steps as f64;
        (0..steps).fold(INITIAL, |state, index| {
            step(state, index as f64 * dt, dt, drive)
        })
    }

    #[test]
    fn double_well_force_and_driven_energy_balance_follow_the_potential() {
        for x in [-2.0, -1.0, -0.1, 0.0, 0.1, 1.0, 2.0] {
            let h = 1e-5;
            let gradient = (energy([x + h, 0.0]) - energy([x - h, 0.0])) / (2.0 * h);
            assert!((gradient - (-x + x.powi(3))).abs() < 1e-8);
            for v in [-1.3, 0.0, 0.7] {
                for time in [0.0, 0.25, 1.0, 3.0] {
                    let [dx, dv] = flow([x, v], time, 0.6);
                    assert_eq!(dx, v);
                    let rate = (-x + x.powi(3)) * dx + v * dv;
                    let supplied_minus_lost = 0.6 * v * (1.2 * time).cos() - 0.3 * v * v;
                    assert!((rate - supplied_minus_lost).abs() < 1e-12);
                }
            }
        }
        for x in [-1.0, 0.0, 1.0] {
            assert_eq!(flow([x, 0.0], 2.3, 0.0), [0.0, 0.0]);
            assert_eq!(step([x, 0.0], 2.3, DT, 0.0), [x, 0.0]);
        }
        assert_eq!(energy([0.0, 0.0]) - energy([1.0, 0.0]), 0.25);
    }

    #[test]
    fn rk_stages_advance_the_drive_clock_and_converge_under_refinement() {
        // Independently computed DOP853 endpoints at t=4, rtol 1e-13 and
        // atol 1e-14. Short-time refinement supplements these fixed fixtures.
        for (drive, expected) in [
            (0.1, [1.223_026_128_754_971_5, 0.095_394_870_908_710_71]),
            (0.3, [1.048_226_996_189_900_5, -0.183_588_723_869_672_93]),
            (0.6, [0.606_130_601_420_157_7, -0.348_222_299_185_051_93]),
            (0.9, [-0.037_616_840_677_515_07, -0.802_915_950_493_660_8]),
            (1.02, [-0.367_569_345_700_153_3, -1.099_317_392_632_846]),
        ] {
            let coarse = evolved(drive, 4.0, 200);
            let shipped = evolved(drive, 4.0, 400);
            let fine = evolved(drive, 4.0, 800);
            let ratio = distance(coarse, shipped) / distance(shipped, fine);
            assert!((14.0..18.0).contains(&ratio), "g={drive}, ratio={ratio}");
            let error = distance(shipped, expected);
            assert!(error < 5e-8, "g={drive}, endpoint error={error}");
            eprintln!(
                "Duffing: g={drive}, t=4 refinement ratio={ratio:.3}, reference error={error:.3e}"
            );
        }
    }

    #[test]
    fn energy_change_matches_work_and_dissipation_over_the_drawn_horizon() {
        let power = |state: State, time: f64, drive: f64| {
            let v = state[1];
            drive * v * (DRIVE_FREQUENCY * time).cos() - DAMPING * v * v
        };
        let mut worst = 0.0_f64;
        for drive in [0.1, 0.3, 0.6, 0.9, 1.02] {
            let trajectory = Trajectory::new(drive);
            let mut work = 0.0;
            for (index, pair) in trajectory.states.windows(2).enumerate() {
                let time = index as f64 * DT;
                let middle = step(pair[0], time, DT * 0.5, drive);
                // Simpson quadrature of physical power, checked against the
                // independently evaluated kinetic-plus-potential energy.
                work += DT / 6.0
                    * (power(pair[0], time, drive)
                        + 4.0 * power(middle, time + DT * 0.5, drive)
                        + power(pair[1], time + DT, drive));
                let residual = (energy(pair[1]) - energy(INITIAL) - work).abs();
                worst = worst.max(residual);
                assert!(
                    residual < 2e-6,
                    "g={drive}, t={time}, energy-work residual={residual}"
                );
            }
        }
        eprintln!("Duffing: maximum sampled energy-work residual={worst:.3e}");
    }

    #[test]
    fn a_large_admitted_response_can_repeat_at_the_drive_period() {
        let period = std::f64::consts::TAU / DRIVE_FREQUENCY;
        let late = |steps_per_period: usize| {
            let dt = period / steps_per_period as f64;
            let mut state = INITIAL;
            let mut samples = Vec::new();
            for index in 0..100 * steps_per_period {
                state = step(state, index as f64 * dt, dt, 0.9);
                if index + 1 >= 98 * steps_per_period && (index + 1) % steps_per_period == 0 {
                    samples.push(state);
                }
            }
            samples
        };
        let coarse = late(512);
        let fine = late(1024);
        for states in [&coarse, &fine] {
            assert_eq!(states.len(), 3);
            for pair in states.windows(2) {
                assert!(distance(pair[0], pair[1]) < 1e-8);
            }
        }
        assert!(distance(coarse[2], fine[2]) < 1e-7);
        let trajectory = Trajectory::new(0.9);
        assert!(trajectory.peaks()[0] > 1.5);
        assert!(!trajectory.status().contains("chaos"));
        // These finite closure and refinement checks are a counterexample to
        // amplitude-based labeling, not a proof of a global attractor.
    }

    #[test]
    fn admitted_drive_grid_stays_finite_and_readout_peaks_cover_the_drawn_path() {
        let mut widest = [0.0_f64; 2];
        for index in 0..=92 {
            let drive = 0.1 + index as f64 * 0.01;
            let trajectory = Trajectory::new(drive);
            assert_eq!(trajectory.states.len(), STEPS + 1, "g={drive}");
            assert_eq!(trajectory.states[0], INITIAL);
            assert_eq!(trajectory.elapsed(), 120.0);
            let mut actual = [0.0_f64; 2];
            for &[x, v] in &trajectory.states {
                assert!(x.is_finite() && v.is_finite());
                actual[0] = actual[0].max(x.abs());
                actual[1] = actual[1].max(v.abs());
            }
            assert_eq!(trajectory.peaks(), actual);
            assert!(
                actual[0] < 3.0 && actual[1] < 4.0,
                "g={drive}, peaks={actual:?}"
            );
            widest[0] = widest[0].max(actual[0]);
            widest[1] = widest[1].max(actual[1]);
        }
        // An independent DOP853 run locates a late excursion at this drive:
        // the first 18 units reach |x|~1.353, while the full trace reaches ~2.004.
        let late_excursion = Trajectory::new(0.79);
        let early = late_excursion
            .states
            .iter()
            .take(1801)
            .map(|state| state[0].abs())
            .fold(0.0_f64, f64::max);
        assert!(
            late_excursion.peaks()[0] > early + 0.5,
            "the later excursion cannot be omitted"
        );
        eprintln!(
            "Duffing: 93 drives, maximum retained |x|={:.6}, |v|={:.6}",
            widest[0], widest[1]
        );
    }

    #[test]
    fn repeat_and_equivalent_accepted_input_keep_one_drive_experiment() {
        let room = Duffing::new_with(6);
        let pokes = [(0.1, 0.5), (2.0, -1.0), (f64::NAN, 0.5)];
        assert_eq!(latest_hand(&pokes), Some((1.0, 0.0)));
        let expected = Trajectory::new(1.02);
        let mut reference = Canvas::new(71, 35);
        draw(&mut reference, &expected.states);
        for inputs in [
            inputs_from_pokes(&pokes, 0.0),
            inputs_from_pokes(&[(1.0, 0.0); MAX_ROOM_POKES], 0.0),
        ] {
            let mut actual = Canvas::new(71, 35);
            room.render_input(&mut actual, 0.5, &inputs);
            assert_eq!(actual.to_text(), reference.to_text());
            assert_eq!(room.status_input(0.5, &inputs), Some(expected.status()));
        }
        for phase in [f64::NAN, f64::INFINITY, -5.0, 2.0] {
            assert!((0.2..=0.82).contains(&gamma(phase, None, 6)));
        }
        let mut discarded = vec![(1.0, 0.5)];
        discarded.extend([(f64::NAN, 0.5); MAX_ROOM_POKES]);
        assert_eq!(latest_hand(&discarded), None);
    }

    #[test]
    fn status_invites() {
        let s = Duffing::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DRIVE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn drive_changes() {
        let r = Duffing::new();
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
        Duffing::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Duffing::new().motif().unwrap().line.len() >= 6);
    }
}
