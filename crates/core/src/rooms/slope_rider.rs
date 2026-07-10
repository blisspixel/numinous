//! Slope Rider: the derivative as a ride, not a rule.
//!
//! A rider slides along a curve; the tangent line under it is its board, and
//! the board's tilt at every instant is the derivative. Below, the tilt traces
//! its own curve as the ride goes on, and that trace is f prime, drawing
//! itself. Nobody says the word derivative; your speed is the controls. See
//! the Full Map in `docs/ROOMS.md`.

use super::variation_unit;
use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The domain, symmetric about zero.
const X_SPAN: f64 = 12.56;
/// Samples across the domain.
const STEPS: usize = 600;

/// The hill being ridden.
fn f(x: f64) -> f64 {
    x.sin() + x / 3.0
}

/// Its exact slope, the thing the ride teaches.
fn slope(x: f64) -> f64 {
    x.cos() + 1.0 / 3.0
}

/// The left edge of the domain.
const X_MIN: f64 = -X_SPAN / 2.0;
/// The hill's vertical range on screen.
const F_MIN: f64 = -X_SPAN / 6.0 - 1.0;
const F_MAX: f64 = X_SPAN / 6.0 + 1.0;
/// The tilt trace's vertical range.
const S_MIN: f64 = -0.7;
const S_MAX: f64 = 1.4;
/// How far a board extends on each side of its rider, in domain units.
const BOARD_REACH: f64 = X_SPAN / 10.0;

/// Screen geometry shared by the hill, the trace, and dropped riders.
fn to_px(x: f64, width: usize) -> i32 {
    ((x - X_MIN) / X_SPAN * (width as f64 - 1.0)) as i32
}

fn hill_py(y: f64, height: usize) -> i32 {
    let norm = (y - F_MIN) / (F_MAX - F_MIN);
    ((1.0 - norm) * (height as f64 * 0.60 - 2.0)) as i32 + 1
}

fn trace_py(x: f64, height: usize) -> i32 {
    let trace_top = height as f64 * 0.66;
    let norm = (slope(x) - S_MIN) / (S_MAX - S_MIN);
    (trace_top + (1.0 - norm) * (height as f64 * 0.32 - 2.0)) as i32
}

/// A dropped rider's board, in domain space: both endpoints of the tangent
/// through (x, f(x)) with slope f'(x). The derivative is the drawing
/// instruction, exactly as in the live ride.
fn board_points(x: f64) -> ((f64, f64), (f64, f64)) {
    let m = slope(x);
    let x0 = x - BOARD_REACH;
    let x1 = x + BOARD_REACH;
    ((x0, f(x) + m * (x0 - x)), (x1, f(x) + m * (x1 - x)))
}

/// Slope Rider.
#[derive(Debug, Default)]
pub struct SlopeRider {
    seed: u64,
}

impl SlopeRider {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn phase_for(&self, t: f64) -> f64 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if self.seed == 0 {
            t
        } else {
            (t + variation_unit(self.seed, 0x534C_4F50_4552_0001) * 0.45).fract()
        }
    }
}

impl Room for SlopeRider {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "slope-rider",
            title: "Slope Rider",
            wing: "Change",
            blurb: "Ride the tangent line along a curve. The board's tilt is the slope, and the \
                    tilt traces its own curve below as you go: the derivative, drawing itself.",
            accent: [255, 190, 70],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let x_t = (self.phase_for(t) - 0.5) * X_SPAN;

        // The hill, upper two thirds.
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = X_MIN + X_SPAN * i as f64 / STEPS as f64;
            let point = (to_px(x, width), hill_py(f(x), height));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '*');
            }
            previous = Some(point);
        }

        // The board: the tangent at the rider, drawn bright.
        let ((x0, y0), (x1, y1)) = board_points(x_t);
        canvas.line(
            to_px(x0, width),
            hill_py(y0, height),
            to_px(x1, width),
            hill_py(y1, height),
            '#',
        );

        // The tilt trace, lower third: f prime drawing itself up to the rider.
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = X_MIN + X_SPAN * i as f64 / STEPS as f64;
            if x > x_t {
                break;
            }
            let point = (to_px(x, width), trace_py(x, height));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '#');
            }
            previous = Some(point);
        }
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DROP A RIDER")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // The newest bounded raw tail first, finite filtering after, matching
        // the catalog input contract.
        let start = pokes.len().saturating_sub(crate::room::MAX_ROOM_POKES);
        let riders: Vec<(f64, f64)> = pokes[start..]
            .iter()
            .copied()
            .filter(|&(x, y)| x.is_finite() && y.is_finite())
            .collect();
        self.render(canvas, t);
        let Some((&newest, older)) = riders.split_last() else {
            return;
        };
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // Every click drops another rider onto the hill at that x: its board
        // is the true tangent there, and a tick lands on the tilt trace below
        // at exactly the board's slope. Two curves, one number, your hand on
        // both. The Pour reads totals; this room reads rates: the Change
        // wing's pair, now both under the hand.
        let mut drop_rider = |hand_x: f64, mark: char| {
            let x = X_MIN + hand_x.clamp(0.0, 1.0) * X_SPAN;
            let ((x0, y0), (x1, y1)) = board_points(x);
            canvas.line(
                to_px(x0, width),
                hill_py(y0, height),
                to_px(x1, width),
                hill_py(y1, height),
                mark,
            );
            let px = to_px(x, width);
            canvas.plot(px, hill_py(f(x), height), '+');
            // The tick on the trace: the board's tilt, written below.
            let ty = trace_py(x, height);
            canvas.plot(px, ty, mark);
            canvas.plot(px, ty - 1, mark);
        };
        for &(x, _) in older {
            drop_rider(x, '.');
        }
        drop_rider(newest.0, 'o');
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = (self.phase_for(t) - 0.5) * X_SPAN;
        Some(format!("TILT = {:+.2}", slope(x)))
    }

    fn reveal(&self) -> &'static str {
        "The lower curve is nothing but the tilt of your board, written down \
         moment by moment. Where you crested, it crosses zero; where the climb \
         was steepest, it peaks. Every smooth curve secretly carries this second \
         curve inside it, and riding is how you read it out."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Fermat was finding tangent lines and extrema a generation before \
             Newton was born, with no limits and no derivatives, just a vanishing \
             little e he quietly threw away at the end. The scandalous trick \
             worked, and it took two centuries to make it honest.",
            "Where your board tilts level at the top of a rise is where functions \
             are maximized, which is why economics, physics, and machine learning \
             are all, at bottom, riders looking for the flat spots.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.62
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "A tangent ride",
            root: 220.0,
            tempo: 128,
            line: &[0, 3, 7, 10, 7, 3, -2, 0],
            encodes: "the board climbing, cresting, descending, and leveling",
        })
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The board hums with the tilt: uphill high, downhill low.
        let x = (self.phase_for(t) - 0.5) * X_SPAN;
        let norm = ((slope(x) + 0.7) / 2.1) as f32;
        SoundSpec::tone(110.0 + 330.0 * norm.clamp(0.0, 1.0), 1.5, 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::{SlopeRider, f, slope};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_slope_is_the_true_derivative() {
        for i in 0..40 {
            let x = -6.0 + 12.0 * f64::from(i) / 40.0;
            let h = 1e-6;
            let numeric = (f(x + h) - f(x - h)) / (2.0 * h);
            assert!((numeric - slope(x)).abs() < 1e-4, "at {x}");
        }
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = SlopeRider::new();
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        room.render(&mut a, 0.62);
        room.render(&mut b, 0.62);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = SlopeRider::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_reads_the_second_curve() {
        assert!(SlopeRider::new().reveal().contains("second curve"));
    }

    #[test]
    fn the_board_slope_is_the_true_derivative() {
        for i in 1..20 {
            let x = super::X_MIN + super::X_SPAN * f64::from(i) / 20.0;
            let ((x0, y0), (x1, y1)) = super::board_points(x);
            let m = (y1 - y0) / (x1 - x0);
            assert!(
                (m - slope(x)).abs() < 1e-12,
                "the board's slope equals f'(x) at {x}"
            );
        }
    }

    #[test]
    fn a_dropped_rider_marks_the_hill_and_ticks_the_trace() {
        let room = SlopeRider::new();
        let mut bare = Canvas::new(60, 30);
        room.render(&mut bare, 0.5);
        let mut poked = Canvas::new(60, 30);
        room.render_poked(&mut poked, 0.5, &[(0.75, 0.5)]);
        assert_ne!(bare.to_text(), poked.to_text());
        assert!(
            poked.to_text().contains('o'),
            "the rider's board is visible"
        );
        assert!(poked.to_text().contains('+'), "the rider marks the hill");
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = SlopeRider::new();
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.4)).collect();
        flood.push((f64::INFINITY, 0.5));
        flood.push((0.25, 0.3));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(60, 30);
        room.render_poked(&mut via_flood, 0.5, &flood);
        let mut via_tail = Canvas::new(60, 30);
        room.render_poked(&mut via_tail, 0.5, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room_and_older_riders_linger() {
        let room = SlopeRider::new();
        let mut bare = Canvas::new(60, 30);
        room.render(&mut bare, 0.5);
        let mut invalid = Canvas::new(60, 30);
        room.render_poked(
            &mut invalid,
            0.5,
            &[(f64::NAN, 0.5), (0.5, f64::NEG_INFINITY)],
        );
        assert_eq!(bare.to_text(), invalid.to_text());
        let mut layered = Canvas::new(60, 30);
        room.render_poked(&mut layered, 0.5, &[(0.15, 0.5), (0.85, 0.5)]);
        let text = layered.to_text();
        assert!(text.contains('.'), "the older rider lingers dim");
        assert!(text.contains('o'), "the newest rider is bright");
    }

    #[test]
    fn seed_variation_changes_poked_renders_and_seed_zero_stays_exact() {
        let mut a = Canvas::new(60, 30);
        SlopeRider::new().render_poked(&mut a, 0.5, &[(0.75, 0.5)]);
        let mut b = Canvas::new(60, 30);
        SlopeRider::new_with(19).render_poked(&mut b, 0.5, &[(0.75, 0.5)]);
        assert_ne!(a.to_text(), b.to_text(), "the ride phase varies with seed");
        let mut exact = Canvas::new(60, 30);
        SlopeRider::new_with(0).render_poked(&mut exact, 0.5, &[(0.75, 0.5)]);
        assert_eq!(a.to_text(), exact.to_text());
    }

    #[test]
    fn hostile_surfaces_and_phase_stay_bounded() {
        struct Weird(Canvas);
        impl crate::surface::Surface for Weird {
            fn width(&self) -> usize {
                self.0.width()
            }
            fn height(&self) -> usize {
                self.0.height()
            }
            fn char_aspect(&self) -> f64 {
                f64::NAN
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                self.0.plot(x, y, mark);
            }
        }
        let room = SlopeRider::new();
        let mut weird = Weird(Canvas::new(30, 15));
        room.render_poked(&mut weird, f64::NAN, &[(0.5, 0.5)]);
        assert!(weird.0.ink_count() > 0);
        let mut nan_phase = Canvas::new(30, 15);
        room.render(&mut nan_phase, f64::NAN);
        let mut zero_phase = Canvas::new(30, 15);
        room.render(&mut zero_phase, 0.0);
        assert_eq!(nan_phase.to_text(), zero_phase.to_text());
        let status = room.status(f64::NAN).expect("status");
        assert!(status.starts_with("TILT ="));
    }
}
