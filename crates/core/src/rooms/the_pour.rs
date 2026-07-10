//! The Pour: integration as pouring, the fundamental theorem, felt.
//!
//! A curve holds water. As `t` sweeps, area pours in from the left and the
//! fill line rises; the running total traces a second curve above, and that
//! curve is the antiderivative. Reverse the pour and you are differentiating.
//! Nobody says the word integral; the accumulation is the controls. See the
//! Full Map in `docs/ROOMS.md`.

use super::variation_unit;
use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The domain: zero to this many radians.
const X_MAX: f64 = 9.42;
/// Samples across the domain.
const STEPS: usize = 600;

/// The curve being poured into: positive everywhere, so the water only rises.
fn f(x: f64) -> f64 {
    1.2 + (x).sin()
}

/// The running area under `f` from 0 to `x` (closed form, for exactness).
fn area(x: f64) -> f64 {
    1.2 * x - x.cos() + 1.0
}

/// The largest value the vessel curve reaches.
const F_MAX: f64 = 2.2;
/// How far a probe's tangent segment extends on each side, in domain units.
const TANGENT_REACH: f64 = 0.7;

/// Screen geometry shared by the vessel, the total, and the hand's probes.
fn to_px(x: f64, width: usize) -> i32 {
    (x / X_MAX * (width as f64 - 1.0)) as i32
}

fn curve_py(x: f64, height: usize) -> i32 {
    let curve_band = height as f64 * 0.62;
    let curve_top = height as f64 * 0.36;
    (curve_top + (1.0 - f(x) / F_MAX) * (curve_band - 4.0)) as i32 + 2
}

fn total_py(x: f64, height: usize) -> i32 {
    let curve_top = height as f64 * 0.36;
    ((1.0 - area(x) / area(X_MAX)) * (curve_top - 3.0)) as i32 + 1
}

/// The tangent segment a probe draws on the total curve, in domain space:
/// both endpoints of the line through (x, area(x)) with slope f(x). The
/// fundamental theorem is the drawing instruction: the tangent's slope IS
/// the vessel's height at that x.
fn tangent_points(x: f64) -> ((f64, f64), (f64, f64)) {
    let x0 = (x - TANGENT_REACH).max(0.0);
    let x1 = (x + TANGENT_REACH).min(X_MAX);
    let a = area(x);
    let slope = f(x);
    ((x0, a + slope * (x0 - x)), (x1, a + slope * (x1 - x)))
}

/// The Pour.
#[derive(Debug, Default)]
pub struct ThePour {
    seed: u64,
}

impl ThePour {
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
            (t + variation_unit(self.seed, 0x504F_5552_0000_0001) * 0.35).fract()
        }
    }
}

impl Room for ThePour {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "the-pour",
            title: "The Pour",
            wing: "Change",
            blurb: "A curve holds water. t pours area in from the left; the rising total traces a \
                    second curve above. You are watching the fundamental theorem of calculus.",
            accent: [80, 160, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let x_t = self.phase_for(t) * X_MAX;
        // The curve lives in the lower two thirds; the total in the upper third.
        let curve_band = height as f64 * 0.62;
        let curve_top = height as f64 * 0.36;

        // The vessel: the curve itself.
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = X_MAX * i as f64 / STEPS as f64;
            let point = (to_px(x, width), curve_py(x, height));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '*');
            }
            previous = Some(point);
        }
        // The water: filled from the left, up to the curve, as far as the pour.
        let floor = (curve_top + curve_band) as i32;
        for i in 0..=STEPS {
            let x = X_MAX * i as f64 / STEPS as f64;
            if x > x_t {
                break;
            }
            let px = to_px(x, width);
            let top = curve_py(x, height);
            let mut py = top;
            while py < floor {
                if (px + py) % 2 == 0 {
                    canvas.plot(px, py, '-');
                }
                py += 1;
            }
        }
        // The total so far: the antiderivative, tracing itself as you pour.
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=STEPS {
            let x = X_MAX * i as f64 / STEPS as f64;
            if x > x_t {
                break;
            }
            let point = (to_px(x, width), total_py(x, height));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '#');
            }
            previous = Some(point);
        }
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: READ THE SLOPE")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // The newest bounded raw tail first, finite filtering after, matching
        // the catalog input contract.
        let start = pokes.len().saturating_sub(crate::room::MAX_ROOM_POKES);
        let probes: Vec<(f64, f64)> = pokes[start..]
            .iter()
            .copied()
            .filter(|&(x, y)| x.is_finite() && y.is_finite())
            .collect();
        self.render(canvas, t);
        let Some((&newest, older)) = probes.split_last() else {
            return;
        };
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // The probe points at the theorem: at the clicked x, the tangent
        // drawn on the total curve has slope equal to the vessel's height
        // below it. Same number, two curves, one hand.
        let curve_top = height as f64 * 0.36;
        // The same mapping total_py uses, applied to the tangent's raw
        // area-space endpoints (which may overshoot the curve slightly at
        // the segment's ends; line clipping keeps them safe).
        let area_to_py = |a: f64| ((1.0 - a / area(X_MAX)) * (curve_top - 3.0)) as i32 + 1;
        let mut draw_probe = |hand_x: f64, mark: char| {
            let x = hand_x.clamp(0.0, 1.0) * X_MAX;
            let px = to_px(x, width);
            // The plumb line: from the total curve down to the vessel.
            let mut py = total_py(x, height);
            let vessel = curve_py(x, height);
            while py <= vessel {
                if py % 2 == 0 {
                    canvas.plot(px, py, mark);
                }
                py += 1;
            }
            // The tangent on the total curve, slope f(x) by the theorem.
            let ((x0, a0), (x1, a1)) = tangent_points(x);
            canvas.line(
                to_px(x0, width),
                area_to_py(a0),
                to_px(x1, width),
                area_to_py(a1),
                mark,
            );
            canvas.plot(px, vessel, '+');
        };
        for &(x, _) in older {
            draw_probe(x, '.');
        }
        draw_probe(newest.0, 'o');
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = self.phase_for(t) * X_MAX;
        Some(format!("HEIGHT = SLOPE = {:.2}", f(x)))
    }

    fn reveal(&self) -> &'static str {
        "The upper curve is the running total of the water below, and its slope \
         at every moment equals the height of the curve being filled. Total and \
         rate are the same fact read in two directions: that is the fundamental \
         theorem of calculus, and you just watched it pour."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Newton and Leibniz found this connection independently and then spent \
             years in the ugliest priority fight in the history of science. The \
             theorem was bigger than both of them, which is the only reason it \
             carries neither name.",
            "Archimedes computed areas under curves two thousand years before \
             calculus by slicing them thin, the method of exhaustion. He was one \
             good notation away; the water was always pouring.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.8
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C accumulating",
            root: 130.81,
            tempo: 92,
            line: &[0, 2, 4, 5, 7, 9, 12, 14],
            encodes: "area accumulating steadily into its antiderivative",
        })
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The fill sings: pitch rises with the accumulated area.
        let ratio = (area(self.phase_for(t) * X_MAX) / area(X_MAX)) as f32;
        SoundSpec::tone(110.0 + 330.0 * ratio, 1.5, 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::{ThePour, area, f};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_curve_is_positive_and_the_area_grows() {
        let mut last = 0.0;
        for i in 0..=100 {
            let x = super::X_MAX * f64::from(i) / 100.0;
            assert!(f(x) > 0.0, "the vessel never dips below the floor");
            let a = area(x);
            assert!(a >= last, "the pour only rises");
            last = a;
        }
    }

    #[test]
    fn the_area_is_the_true_integral() {
        // Riemann-check the closed form: area'(x) == f(x) numerically.
        for i in 1..40 {
            let x = super::X_MAX * f64::from(i) / 40.0;
            let h = 1e-6;
            let slope = (area(x + h) - area(x - h)) / (2.0 * h);
            assert!(
                (slope - f(x)).abs() < 1e-4,
                "the fundamental theorem holds at {x}: {slope} vs {}",
                f(x)
            );
        }
    }

    #[test]
    fn render_is_deterministic_and_pours_more_over_time() {
        let room = ThePour::new();
        let mut early = Canvas::new(60, 30);
        let mut late = Canvas::new(60, 30);
        room.render(&mut early, 0.2);
        room.render(&mut late, 0.9);
        assert!(late.ink_count() > early.ink_count(), "more pour, more ink");
        let mut again = Canvas::new(60, 30);
        room.render(&mut again, 0.9);
        assert_eq!(late.to_text(), again.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = ThePour::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_theorem() {
        assert!(ThePour::new().reveal().contains("fundamental theorem"));
    }

    #[test]
    fn the_tangent_slope_is_the_vessel_height() {
        // The probe's whole claim: the segment drawn on the total curve has
        // slope exactly f(x). The fundamental theorem, as geometry.
        for i in 1..20 {
            let x = super::X_MAX * f64::from(i) / 20.0;
            let ((x0, a0), (x1, a1)) = super::tangent_points(x);
            assert!((x1 - x0) > 0.0);
            let slope = (a1 - a0) / (x1 - x0);
            assert!(
                (slope - f(x)).abs() < 1e-12,
                "tangent slope equals vessel height at {x}"
            );
            assert!(
                x0 >= 0.0 && x1 <= super::X_MAX,
                "the segment stays in domain"
            );
        }
    }

    #[test]
    fn a_probe_marks_the_theorem_and_the_sweep_is_untouched() {
        let room = ThePour::new();
        let mut bare = Canvas::new(60, 30);
        room.render(&mut bare, 0.6);
        let mut poked = Canvas::new(60, 30);
        room.render_poked(&mut poked, 0.6, &[(0.4, 0.5)]);
        assert_ne!(bare.to_text(), poked.to_text());
        assert!(poked.to_text().contains('o'), "the probe is visible");
        assert!(poked.to_text().contains('+'), "the vessel point is marked");
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = ThePour::new();
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.4)).collect();
        flood.push((f64::NAN, 0.5));
        flood.push((0.7, 0.3));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(60, 30);
        room.render_poked(&mut via_flood, 0.6, &flood);
        let mut via_tail = Canvas::new(60, 30);
        room.render_poked(&mut via_tail, 0.6, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room_and_older_probes_linger() {
        let room = ThePour::new();
        let mut bare = Canvas::new(60, 30);
        room.render(&mut bare, 0.6);
        let mut invalid = Canvas::new(60, 30);
        room.render_poked(&mut invalid, 0.6, &[(f64::NAN, 0.5), (0.5, f64::INFINITY)]);
        assert_eq!(bare.to_text(), invalid.to_text());
        let mut layered = Canvas::new(60, 30);
        room.render_poked(&mut layered, 0.6, &[(0.15, 0.5), (0.8, 0.5)]);
        let text = layered.to_text();
        assert!(text.contains('.'), "the older probe lingers dim");
        assert!(text.contains('o'), "the newest probe is bright");
    }

    #[test]
    fn seed_variation_changes_poked_renders_and_seed_zero_stays_exact() {
        let mut a = Canvas::new(60, 30);
        ThePour::new().render_poked(&mut a, 0.6, &[(0.4, 0.5)]);
        let mut b = Canvas::new(60, 30);
        ThePour::new_with(17).render_poked(&mut b, 0.6, &[(0.4, 0.5)]);
        assert_ne!(a.to_text(), b.to_text(), "the pour phase varies with seed");
        let mut exact = Canvas::new(60, 30);
        ThePour::new_with(0).render_poked(&mut exact, 0.6, &[(0.4, 0.5)]);
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
                f64::INFINITY
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                self.0.plot(x, y, mark);
            }
        }
        let room = ThePour::new();
        let mut weird = Weird(Canvas::new(30, 15));
        room.render_poked(&mut weird, f64::NAN, &[(0.5, 0.5)]);
        assert!(weird.0.ink_count() > 0);
        let mut nan_phase = Canvas::new(30, 15);
        room.render(&mut nan_phase, f64::NAN);
        let mut zero_phase = Canvas::new(30, 15);
        room.render(&mut zero_phase, 0.0);
        assert_eq!(nan_phase.to_text(), zero_phase.to_text());
        let status = room.status(f64::NAN).expect("status");
        assert!(status.starts_with("HEIGHT = SLOPE"));
    }
}
