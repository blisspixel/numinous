//! Simple pendulum: analytic energy contours on the phase cylinder.
//!
//! Dimensionless energy is E = omega^2/2 - cos(theta). Gallery phase sweeps
//! energy; it is not elapsed pendulum time. Dragging selects energy directly,
//! from equilibrium through small-angle swings to rotations. See `docs/ROOMS.md`.

use std::f64::consts::{PI, TAU};

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

const MIN_ENERGY: f64 = -1.0;
const MAX_ENERGY: f64 = 3.0;
// The fastest admitted state has |omega| = sqrt(2 * (MAX_ENERGY + 1)) = sqrt(8).
// A fixed [-3, 3] speed axis contains it with margin and keeps comparisons honest.
const SPEED_LIMIT: f64 = 3.0;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn latest_hand(pokes: &[(f64, f64)]) -> Option<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .rev()
        .copied()
        .find(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
}

fn energy(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    // Seed variation shifts only the untouched sweep. Every hand can reach
    // the full range, with equilibrium at x=0 and the separatrix at x=0.5.
    let position = hand.map_or_else(
        || (phase_unit(t) + (seed % 5) as f64 * 0.02).min(1.0),
        |(x, _)| x,
    );
    MIN_ENERGY + (MAX_ENERGY - MIN_ENERGY) * position
}

fn regime(e: f64) -> &'static str {
    if e == MIN_ENERGY {
        "stable equilibrium"
    } else if e < 1.0 {
        "libration"
    } else if e == 1.0 {
        "separatrix"
    } else {
        "rotation"
    }
}

fn status_line(e: f64) -> String {
    // The value is rounded; the regime uses the actual selected energy.
    format!("E~{e:.3}  {}  DRAG:E", regime(e))
}

/// A point on the positive-speed branch, including its exact endpoints.
/// The fraction parameter samples angle, not physical time along the orbit.
fn contour_point(e: f64, fraction: f64) -> (f64, f64) {
    let extent = if e < 1.0 { (-e).acos() } else { PI };
    let theta = extent * (2.0 * fraction - 1.0);
    let speed = if e <= 1.0 && (fraction == 0.0 || fraction == 1.0) {
        // acos(-E) is a turning point; roundoff must not open a libration.
        0.0
    } else {
        (2.0 * (e + theta.cos())).max(0.0).sqrt()
    };
    (theta, speed)
}

fn draw_contour(canvas: &mut dyn Surface, e: f64, mark: char, width: usize, height: usize) {
    // Even sampling includes theta=0 as well as both exact angular endpoints.
    let segments = width.max(height).max(2).next_multiple_of(2);
    let to_pixel = |fraction: f64, direction: f64| {
        let (theta, speed) = contour_point(e, fraction);
        (
            ((theta / TAU + 0.5) * width.saturating_sub(1) as f64).round() as i32,
            ((0.5 - direction * speed / (2.0 * SPEED_LIMIT)) * height.saturating_sub(1) as f64)
                .round() as i32,
        )
    };
    for direction in [-1.0, 1.0] {
        let mut previous = to_pixel(0.0, direction);
        canvas.plot(previous.0, previous.1, mark);
        for index in 1..=segments {
            let current = to_pixel(index as f64 / segments as f64, direction);
            canvas.line(previous.0, previous.1, current.0, current.1, mark);
            previous = current;
        }
    }
}

fn draw(canvas: &mut dyn Surface, e: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Both reference branches go behind the selected orbit, including at E=1.
    draw_contour(canvas, 1.0, '.', width, height);
    draw_contour(canvas, e, '#', width, height);
}

/// Simple pendulum room.
#[derive(Debug, Default)]
pub struct SimplePendulum {
    seed: u64,
}

impl SimplePendulum {
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

impl Room for SimplePendulum {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, energy(t, None, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "simple-pendulum",
            root: 9.18,
            tempo: 76,
            line: &[0, 5, 3, 7, 12, 7, 3, 5],
            encodes: "pendulum motif: librations below separatrix E=1",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ENERGY")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(status_line(energy(t, None, self.seed)))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw(canvas, energy(t, latest_hand(pokes), self.seed));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        Some(status_line(energy(t, latest_hand(&pokes), self.seed)))
    }

    fn reveal(&self) -> &'static str {
        "The simple pendulum lives on a phase cylinder: angle and scaled angular speed, \
         with E = omega^2/2 - cos(theta). The left and right edges are the same angle. \
         E=-1 is rest; -1<E<1 gives closed swings, nearly ellipses near rest. \
         The dotted E=1 separatrix divides swings from rotations at E>1. \
         Phase selects energy, not elapsed pendulum time."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_ENERGY, MIN_ENERGY, SimplePendulum, contour_point, draw, energy, latest_hand, regime,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput, inputs_from_pokes};
    use crate::surface::Surface;

    #[test]
    fn sampled_contours_conserve_the_selected_energy() {
        for e in [
            -1.0, -0.999_999, -0.99, -0.5, 0.0, 0.4, 0.999_999, 1.0, 1.000_001, 2.4, 3.0,
        ] {
            for index in 0..=128 {
                let (theta, speed) = contour_point(e, index as f64 / 128.0);
                assert!(theta.is_finite() && speed.is_finite());
                assert!((-std::f64::consts::PI..=std::f64::consts::PI).contains(&theta));
                for omega in [-speed, speed] {
                    let measured_energy = omega * omega / 2.0 - theta.cos();
                    assert!(
                        (measured_energy - e).abs() < 2e-14,
                        "E={e}, theta={theta}, omega={omega}: residual {}",
                        measured_energy - e
                    );
                }
            }
        }
    }

    #[test]
    fn librations_include_both_exact_zero_speed_turning_points() {
        for (e, angle) in [
            (-1.0, 0.0),
            (-0.5, std::f64::consts::PI / 3.0),
            (0.0, std::f64::consts::FRAC_PI_2),
            (0.5, 2.0 * std::f64::consts::PI / 3.0),
            (1.0, std::f64::consts::PI),
        ] {
            let left = contour_point(e, 0.0);
            let right = contour_point(e, 1.0);
            assert!((left.0 + angle).abs() < 1e-14);
            assert!((right.0 - angle).abs() < 1e-14);
            assert_eq!(left.1, 0.0);
            assert_eq!(right.1, 0.0);
        }
        let (_, seam_speed) = contour_point(1.0 + f64::EPSILON, 0.0);
        assert!(
            seam_speed > 0.0,
            "a rotation must not gain a false turning point"
        );
    }

    #[test]
    fn separatrix_matches_both_analytic_speed_branches() {
        for index in 0..=128 {
            let (theta, speed) = contour_point(1.0, index as f64 / 128.0);
            for direction in [-1.0, 1.0] {
                let expected = direction * 2.0 * (theta / 2.0).cos();
                assert!((direction * speed - expected).abs() < 1e-14);
            }
        }
        let mut reference = Canvas::new(49, 25);
        draw(&mut reference, 0.4);
        assert_eq!(reference.cell(24, 4), Some('.'));
        assert_eq!(reference.cell(24, 20), Some('.'));
        let mut selected = Canvas::new(49, 25);
        draw(&mut selected, 1.0);
        assert_eq!(selected.cell(24, 4), Some('#'));
        assert_eq!(selected.cell(24, 20), Some('#'));
        assert!(
            !selected.to_text().contains('.'),
            "reference cannot obscure selected E=1"
        );
    }

    #[test]
    fn every_seed_admits_small_swings_and_exact_regimes() {
        assert_eq!(regime(-1.0), "stable equilibrium");
        assert_eq!(regime(1.0 - f64::EPSILON), "libration");
        assert_eq!(regime(1.0), "separatrix");
        assert_eq!(regime(1.0 + f64::EPSILON), "rotation");
        for seed in [0, 1, 4, u64::MAX] {
            for phase in [f64::NAN, f64::INFINITY, -10.0, 0.0, 0.5, 1.0, 10.0] {
                assert!((MIN_ENERGY..=MAX_ENERGY).contains(&energy(phase, None, seed)));
            }
            for (x, expected) in [(0.0, -1.0), (0.5, 1.0), (1.0, 3.0)] {
                assert_eq!(energy(0.9, Some((x, 0.5)), seed), expected);
            }
            let small = energy(0.9, Some((0.0025, 0.5)), seed);
            let (turn, _) = contour_point(small, 1.0);
            assert!((small + 0.99).abs() < 1e-14);
            assert!(
                turn > 0.0 && turn < 0.15,
                "a swing under nine degrees is reachable"
            );
        }
        let room = SimplePendulum::new();
        for (phase, expected) in [
            (0.0, "stable equilibrium"),
            (0.499_999, "libration"),
            (0.5, "separatrix"),
            (0.500_001, "rotation"),
        ] {
            let direct = room.status(phase).expect("phase status");
            let touched = room
                .status_input(0.9, &inputs_from_pokes(&[(phase, 0.5)], 0.0))
                .expect("hand status");
            assert_eq!(direct, touched);
            assert!(direct.contains(expected), "{direct}");
            assert!(direct.chars().count() <= 56);
        }
    }

    fn active_components(canvas: &Canvas) -> usize {
        let mut remaining = std::collections::BTreeSet::new();
        for y in 0..canvas.height() {
            for x in 0..canvas.width() {
                if canvas.cell(x, y) == Some('#') {
                    remaining.insert((x as i32, y as i32));
                }
            }
        }
        let mut count = 0;
        while let Some(start) = remaining.pop_first() {
            count += 1;
            let mut pending = vec![start];
            while let Some((x, y)) = pending.pop() {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if remaining.remove(&(x + dx, y + dy)) {
                            pending.push((x + dx, y + dy));
                        }
                    }
                }
            }
        }
        count
    }

    #[test]
    fn rendered_libration_closes_and_rotations_remain_separate() {
        let mut libration = Canvas::new(48, 24);
        draw(&mut libration, 0.4);
        // E=0.4 turns at +/-1.982313173 radians. In this viewport both
        // branches must reach the zero-speed row at columns 9 and 38.
        assert_eq!(libration.cell(9, 12), Some('#'));
        assert_eq!(libration.cell(38, 12), Some('#'));
        assert_eq!(active_components(&libration), 1);

        let mut rotation = Canvas::new(49, 25);
        draw(&mut rotation, 3.0);
        assert_eq!(active_components(&rotation), 2);
        // Peak speed sqrt(8) is visible with a row to spare in either direction.
        assert_eq!(rotation.cell(24, 1), Some('#'));
        assert_eq!(rotation.cell(24, 23), Some('#'));
        for x in 0..rotation.width() {
            assert_ne!(rotation.cell(x, 0), Some('#'));
            assert_ne!(rotation.cell(x, 24), Some('#'));
        }

        let mut rest = Canvas::new(49, 25);
        draw(&mut rest, -1.0);
        assert_eq!(rest.cell(24, 12), Some('#'));
        assert_eq!(
            rest.to_text().chars().filter(|&mark| mark == '#').count(),
            1
        );
        assert!(
            !rest.to_text().contains(['+', 'o']),
            "no decorative bob obscures the orbit"
        );
    }

    #[test]
    fn all_admitted_contours_fit_the_surface_without_clipping() {
        struct BoundedSurface {
            width: usize,
            height: usize,
            plots: usize,
        }
        impl Surface for BoundedSurface {
            fn width(&self) -> usize {
                self.width
            }
            fn height(&self) -> usize {
                self.height
            }
            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                let (width, height) = self.draw_bounds();
                assert!(x >= 0 && (x as usize) < width, "x={x}, width={width}");
                assert!(y >= 0 && (y as usize) < height, "y={y}, height={height}");
                self.plots += 1;
            }
        }
        for (width, height) in [
            (0, 0),
            (0, 5),
            (5, 0),
            (1, 1),
            (1, 12),
            (12, 1),
            (2, 2),
            (48, 24),
            (97, 63),
            (usize::MAX, 1),
        ] {
            for e in [-1.0, -0.99, 0.4, 1.0, 1.01, 2.4, 3.0] {
                let mut surface = BoundedSurface {
                    width,
                    height,
                    plots: 0,
                };
                draw(&mut surface, e);
                assert_eq!(surface.plots > 0, width > 0 && height > 0);
            }
        }
    }

    #[test]
    fn accepted_hand_controls_the_same_energy_in_readout_and_geometry() {
        let room = SimplePendulum::new_with(4);
        let pokes = [(0.1, 0.2), (2.0, -3.0), (f64::NAN, 0.5)];
        assert_eq!(latest_hand(&pokes), Some((1.0, 0.0)));
        let inputs = inputs_from_pokes(&pokes, 0.0);
        assert!(
            room.status_input(0.0, &inputs)
                .expect("status")
                .contains("E~3.000  rotation")
        );
        let mut actual = Canvas::new(49, 25);
        room.render_input(&mut actual, 0.0, &inputs);
        let mut expected = Canvas::new(49, 25);
        draw(&mut expected, 3.0);
        assert_eq!(actual.to_text(), expected.to_text());
        let mut later = Canvas::new(49, 25);
        room.render_input(&mut later, 1.0, &inputs);
        assert_eq!(
            actual.to_text(),
            later.to_text(),
            "phase is not physical time"
        );

        let mut stale = vec![(1.0, 0.5)];
        stale.extend(vec![(f64::NAN, 0.5); MAX_ROOM_POKES]);
        assert_eq!(latest_hand(&stale), None);
        let inputs = inputs_from_pokes(&stale, 0.0);
        assert_eq!(room.status_input(0.2, &inputs), room.status(0.2));
        let mut actual = Canvas::new(49, 25);
        room.render_input(&mut actual, 0.2, &inputs);
        let mut expected = Canvas::new(49, 25);
        room.render(&mut expected, 0.2);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn status_invites() {
        let s = SimplePendulum::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lib") || s.contains("rot"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn energy_changes() {
        let r = SimplePendulum::new();
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
        SimplePendulum::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
