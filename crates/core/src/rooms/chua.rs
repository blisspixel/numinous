//! Chua's circuit: double-scroll chaotic attractor from a piecewise-linear diode.
//!
//! DRAG: TUNE ALPHA. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

const DT: f64 = 0.02;
const BETA: f64 = 14.286;
const WARMUP: usize = 200;
const BASE_STEPS: usize = 2_400;
const SWEEP_STEPS: usize = 400;
const ALPHA_SWEEP: f64 = 2.5;
// Fixed voltage scales for the oscilloscope. A tuned orbit can leave the view.
const X_LIMIT: f64 = 3.0;
const Y_LIMIT: f64 = 0.8;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn last_dial(pokes: &[(f64, f64)]) -> Option<f64> {
    pokes
        .iter()
        .rev()
        .take(MAX_ROOM_POKES)
        .find(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, _)| x.clamp(0.0, 1.0))
}

// The total x characteristic is x plus the piecewise-linear diode current.
// Its inner and outer slopes are -1/7 and 2/7, with bends at x = +/-1.
fn characteristic(x: f64) -> f64 {
    (2.0 * x - 3.0 * x.clamp(-1.0, 1.0)) / 7.0
}

fn step(state: [f64; 4], dt: f64, dial_rate: f64) -> [f64; 4] {
    // Integrate the dial alongside the voltages and current so all four
    // Runge-Kutta stages see the instantaneous value of alpha.
    crate::numerics::rk4(state, dt, |[x, y, z, a]| {
        [a * (y - characteristic(x)), x - y + z, -BETA * y, dial_rate]
    })
}

struct Reading {
    min_x: f64,
    max_x: f64,
    flips: u32,
    left_view: bool,
}

/// Chua circuit room.
#[derive(Debug, Default)]
pub struct Chua {
    seed: u64,
}

impl Chua {
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

    fn dial_offset(&self) -> f64 {
        (self.seed % 5) as f64 * 0.05
    }

    fn sweep_alpha(&self, t: f64) -> f64 {
        9.0 + ALPHA_SWEEP * phase_unit(t) + self.dial_offset()
    }

    fn held_alpha(&self, x: f64) -> f64 {
        8.0 + x * 4.0 + self.dial_offset()
    }

    fn trace(&self, t: f64, dial: Option<f64>, mut plot: impl FnMut(f64, f64, char)) -> Reading {
        let alpha = dial.map_or(self.sweep_alpha(0.0), |x| self.held_alpha(x));
        let mut state = [0.1 + (self.seed % 7) as f64 * 0.01, 0.0, 0.0, alpha];
        for _ in 0..WARMUP {
            state = step(state, DT, 0.0);
        }

        let mut previous_x = state[0];
        let mut reading = Reading {
            min_x: previous_x,
            max_x: previous_x,
            flips: 0,
            left_view: false,
        };
        plot(state[0], state[1], '+');
        let mut advance = |dt, rate, mark| {
            state = step(state, dt, rate);
            let [x, y, _, _] = state;
            if x.abs() > X_LIMIT || y.abs() > Y_LIMIT {
                reading.left_view = true;
                return false;
            }
            reading.min_x = reading.min_x.min(x);
            reading.max_x = reading.max_x.max(x);
            reading.flips += u32::from(x.is_sign_negative() != previous_x.is_sign_negative());
            previous_x = x;
            plot(x, y, mark);
            true
        };
        for _ in 0..BASE_STEPS {
            if !advance(DT, 0.0, '+') {
                return reading;
            }
        }
        let (progress, rate) = if dial.is_some() {
            (1.0, 0.0)
        } else {
            (phase_unit(t), ALPHA_SWEEP / (SWEEP_STEPS as f64 * DT))
        };
        let steps = progress * SWEEP_STEPS as f64;
        for _ in 0..steps as usize {
            if !advance(DT, rate, '*') {
                return reading;
            }
        }
        if steps.fract() > 0.0 {
            advance(DT * steps.fract(), rate, '*');
        }
        reading
    }

    fn draw(&self, canvas: &mut dyn Surface, t: f64, dial: Option<f64>) {
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let mut previous = None;
        self.trace(t, dial, |x, y, mark| {
            let px = ((0.5 + 0.5 * x / X_LIMIT) * (width - 1) as f64).round() as i32;
            let py = ((0.5 - 0.5 * y / Y_LIMIT) * (height - 1) as f64).round() as i32;
            if let Some((old_x, old_y)) = previous {
                canvas.line(old_x, old_y, px, py, mark);
            } else {
                canvas.plot(px, py, mark);
            }
            previous = Some((px, py));
        });
    }
}

impl Room for Chua {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.draw(canvas, t, None);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chua",
            root: 207.65,
            tempo: 94,
            line: &[0, 3, 7, 10, 14, 10, 7, 3],
            encodes: "two scrolls joined by a piecewise diode",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ALPHA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = self.sweep_alpha(t);
        Some(format!("a={a:.2}  sweep  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        self.draw(canvas, t, last_dial(pokes));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let Some(x) = last_dial(&crate::pokes_from_inputs(inputs)) else {
            return self.status(t);
        };
        let a = self.held_alpha(x);
        let reading = self.trace(t, Some(x), |_, _, _| {});
        let span = reading.max_x - reading.min_x;
        let flips = reading.flips;
        let boundary = if reading.left_view { "  left view" } else { "" };
        Some(format!("a={a:.1}  span={span:.2}  flips={flips}{boundary}"))
    }

    fn reveal(&self) -> &'static str {
        "Each point plots one capacitor's voltage against the other's. Their \
         path can wind around two scrolls. The tour draws a dim starting trail, \
         then a brighter trace as alpha rises. Drag to hold alpha and see \
         whether the trace stays on one side, crosses zero, or leaves the view."
    }
}

#[cfg(test)]
mod tests {
    use super::{BASE_STEPS, Chua, DT, SWEEP_STEPS, characteristic, step};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Chua::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn an_existing_parameter_challenge_still_lands_on_its_dial_setting() {
        let room = Chua::new();
        let goal = crate::pose_parameter_goal(&room, 17).expect("Chua dial");
        assert_eq!(goal.label, "a");
        assert_eq!(goal.target, 10.25);
        let on_target = crate::grade_parameter(&room, &goal, 0.5).expect("dial reading");
        assert!(on_target.within);
        assert_eq!(on_target.value, 10.25);
        let early = crate::grade_parameter(&room, &goal, 0.125).expect("dial reading");
        assert_eq!(early.value, 9.31);
        assert!(!early.within);
    }

    #[test]
    fn diode_and_short_linear_run_match_the_circuit() {
        for (x, h) in [
            (-2.0, -1.0),
            (-1.0, 1.0),
            (0.0, 0.0),
            (1.0, -1.0),
            (2.0, 1.0),
        ] {
            assert_eq!(characteristic(x), h / 7.0);
        }
        // In the inner region the equations are linear. The first row of
        // A(t) is [a(t)/7, a(t), 0]. The others are [1,-1,1] and [0,-beta,0].
        // The Taylor series of u' = A(t)u gives these independent references
        // at time 0.3, both with the dial held and with it rising.
        for (rate, reference) in [
            (
                0.0,
                [0.193_639_706_5, 0.030_105_318_9, -0.064_729_647_1, 9.0],
            ),
            (
                0.3125,
                [0.194_328_857_0, 0.030_154_121_4, -0.064_775_318_0, 9.09375],
            ),
        ] {
            let mut state = [0.1, 0.0, 0.0, 9.0];
            for _ in 0..15 {
                state = step(state, DT, rate);
            }
            for (got, want) in state.into_iter().zip(reference) {
                assert!((got - want).abs() < 1e-7);
            }
        }
    }

    #[test]
    fn later_phases_extend_the_same_orbit_in_display_order() {
        let room = Chua::new();
        let points = |t| {
            let mut points = Vec::new();
            room.trace(t, None, |x, y, mark| points.push((x, y, mark)));
            points
        };
        let full = points(1.0);
        let half = points(0.5);
        assert!(full.starts_with(&half));
        assert_eq!(half.len(), 1 + BASE_STEPS + SWEEP_STEPS / 2);
        assert!(full[..=BASE_STEPS].iter().all(|p| p.2 == '+'));
        assert!(full[BASE_STEPS + 1..].iter().all(|p| p.2 == '*'));
        let partial = points(0.501);
        assert!(partial.starts_with(&half));
        assert_ne!(partial.last(), half.last());
        assert_eq!(points(f64::NAN), points(0.0));
    }

    #[test]
    fn all_dealt_tours_fit_the_fixed_voltage_window() {
        // The seed chooses one of five dial offsets and seven initial voltages.
        for seed in 0..35 {
            let room = Chua::new_with(seed);
            let trace = room.trace(1.0, None, |_, _, _| {});
            assert!(!trace.left_view, "seed {seed}");
        }
        let default = Chua::new().trace(1.0, None, |_, _, _| {});
        assert!(default.min_x < -1.5 && default.max_x > 1.5);
    }

    #[test]
    fn held_dial_reports_the_visible_trace_and_flags_departure() {
        let room = Chua::new();
        for (dial, leaves) in [(0.0, false), (0.25, false), (1.0, true)] {
            let mut x_values = Vec::new();
            room.trace(1.0, Some(dial), |x, _, _| x_values.push(x));
            let lo = x_values.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let flips = x_values
                .windows(2)
                .filter(|p| p[0].is_sign_negative() != p[1].is_sign_negative())
                .count();
            let inputs = [RoomInput::PointerDown {
                x: dial,
                y: 0.1,
                t: 0.0,
            }];
            let status = room.status_input(0.0, &inputs).unwrap();
            assert!(status.contains(&format!("span={:.2}  flips={flips}", hi - lo)));
            assert_eq!(status.contains("left view"), leaves);
            assert_eq!(room.status_input(1.0, &inputs).unwrap(), status);
        }
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Chua::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
