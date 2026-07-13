//! Fourier Epicycles: circles on circles draw anything at all.
//!
//! A target shape is decomposed into rotating circles (its discrete Fourier
//! series). Chain the circles tip to tail, let each spin at its own speed, and
//! the end of the chain traces the shape back into existence. Ptolemy called
//! this machinery epicycles and used it on planets; Fourier proved it works on
//! everything. `t` runs the pen around the drawing. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::{MAX_DIM, Surface};

/// Sample points along the target shape.
const SAMPLES: usize = 128;
/// Fourier terms kept (frequencies -K..=K).
const K: i64 = 10;

/// One Fourier circle: its frequency `k` and its coefficient as `(re, im)`.
type Circle = (i64, f64, f64);
/// The target's constant decomposition: the center (frequency 0) and the kept
/// circles, largest first.
type Decomposition = ((f64, f64), Vec<Circle>);

/// One of the star's 10 outline vertices (outer, inner, outer, ...).
fn star_vertex(v: usize) -> (f64, f64) {
    let angle = TAU * v as f64 / 10.0 - TAU / 4.0;
    let r = if v % 2 == 0 { 1.0 } else { 0.45 };
    (r * angle.cos(), r * angle.sin())
}

/// The target: a five-pointed star, closed, centered, radius about one.
fn target(i: usize) -> (f64, f64) {
    let s = i as f64 / SAMPLES as f64;
    let pos = s * 10.0;
    let edge = (pos as usize) % 10;
    let frac = pos.fract();
    // Compute only the two vertices this segment needs, not a fresh Vec of all
    // ten every call; `target` is called thousands of times when the constant
    // Fourier series is built.
    let (x0, y0) = star_vertex(edge);
    let (x1, y1) = star_vertex((edge + 1) % 10);
    (x0 + (x1 - x0) * frac, y0 + (y1 - y0) * frac)
}

/// One Fourier coefficient of the target, as (re, im) for frequency `k`.
fn coefficient(k: i64) -> (f64, f64) {
    let (mut re, mut im) = (0.0, 0.0);
    for i in 0..SAMPLES {
        let (x, y) = target(i);
        let angle = -TAU * k as f64 * i as f64 / SAMPLES as f64;
        let (c, s) = (angle.cos(), angle.sin());
        // (x + iy) * (c + is)
        re += x * c - y * s;
        im += x * s + y * c;
    }
    (re / SAMPLES as f64, im / SAMPLES as f64)
}

/// All kept coefficients with their frequencies, largest circle first
/// (skipping the constant term, which is just the center).
fn epicycles() -> Vec<Circle> {
    let mut list: Vec<(i64, f64, f64)> = (-K..=K)
        .filter(|&k| k != 0)
        .map(|k| {
            let (re, im) = coefficient(k);
            (k, re, im)
        })
        .collect();
    list.sort_by(|a, b| {
        let ra = a.1.hypot(a.2);
        let rb = b.1.hypot(b.2);
        rb.partial_cmp(&ra).unwrap_or(std::cmp::Ordering::Equal)
    });
    list
}

/// The constant Fourier decomposition of the target star: the center (frequency
/// 0) and the kept circles, largest first. It does not depend on `tau`, the
/// seed, or the surface, so it is computed once and reused across every `tip`
/// call and every frame (building it was the room's dominant cost, recomputed
/// hundreds of times per frame with a heap allocation per sample).
fn series() -> &'static Decomposition {
    static SERIES: std::sync::OnceLock<Decomposition> = std::sync::OnceLock::new();
    SERIES.get_or_init(|| (coefficient(0), epicycles()))
}

/// The chain's tip at time `tau` in [0, 1): the partial Fourier sum.
fn tip(tau: f64) -> (f64, f64) {
    let ((c0re, c0im), circles) = series();
    let (mut x, mut y) = (*c0re, *c0im);
    for &(k, re, im) in circles {
        let angle = TAU * k as f64 * tau;
        let (c, s) = (angle.cos(), angle.sin());
        x += re * c - im * s;
        y += re * s + im * c;
    }
    (x, y)
}

fn phase_for(t: f64) -> f64 {
    if t.is_finite() {
        t.rem_euclid(1.0)
    } else {
        0.0
    }
}

fn phase_offset(seed: u64) -> f64 {
    if seed == 0 {
        0.0
    } else {
        let mut rng = SplitMix64::new(seed ^ 0xE91C_1A55_D5E0_E11C);
        0.0025 + rng.next_f64() * 0.795
    }
}

fn safe_aspect(aspect: f64) -> f64 {
    if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    }
}

fn drawing_frame(canvas: &dyn Surface) -> Option<(f64, f64, f64, f64)> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        return None;
    }
    let width = width.min(MAX_DIM) as f64;
    let height = height.min(MAX_DIM) as f64;
    let aspect = safe_aspect(canvas.char_aspect());
    let scale = (width / 2.0).min(height / (2.0 * aspect)) * 0.7;
    Some((width, height, aspect, scale))
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MiniTrace {
    cx: f64,
    cy: f64,
    scale: f64,
    tau: f64,
}

fn mini_traces(
    pokes: &[(f64, f64)],
    phase: f64,
    seed: u64,
    width: f64,
    height: f64,
    aspect: f64,
) -> Vec<MiniTrace> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(px, py)| {
            if !px.is_finite() || !py.is_finite() {
                return None;
            }
            let px = px.clamp(0.0, 1.0);
            let py = py.clamp(0.0, 1.0);
            let shift = (px + py) * 0.1;
            Some(MiniTrace {
                cx: width / 2.0 + (px - 0.5) * width * 0.28,
                cy: height / 2.0 + (py - 0.5) * height * 0.28,
                scale: (width / 2.0).min(height / (2.0 * aspect)) * 0.22,
                tau: (phase + phase_offset(seed) + shift).rem_euclid(1.0),
            })
        })
        .collect()
}

/// The Fourier Epicycles room.
#[derive(Debug, Default)]
pub struct Epicycles {
    seed: u64,
}

impl Epicycles {
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

impl Room for Epicycles {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "epicycles",
            title: "Fourier Epicycles",
            wing: "Waves & Sound",
            blurb: "Circles on circles, each spinning at its own speed, and the tip of the chain \
                    draws a star. Fourier proved the circles can draw anything. t runs the pen.",
            accent: [180, 130, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height, aspect, scale)) = drawing_frame(canvas) else {
            return;
        };
        let cx = width / 2.0;
        let cy = height / 2.0;
        let to_screen =
            |x: f64, y: f64| ((cx + x * scale) as i32, (cy + y * scale * aspect) as i32);

        // Seed-driven phase offset for visible per-visit variation.
        let tau_now = (phase_for(t) + phase_offset(self.seed)).rem_euclid(1.0);
        // The traced path so far: the star, drawing itself.
        let steps = (tau_now * 360.0) as usize;
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let (x, y) = tip(i as f64 / 360.0);
            let point = to_screen(x, y);
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '#');
            }
            previous = Some(point);
        }
        // The machinery: the chain of circles at this instant, dim.
        let ((c0re, c0im), circles) = series();
        let (mut x, mut y) = (*c0re, *c0im);
        for &(k, re, im) in circles.iter().take(7) {
            let radius = re.hypot(im);
            let ring = 90;
            for r in 0..ring {
                let a = TAU * r as f64 / ring as f64;
                let (px, py) = to_screen(x + radius * a.cos(), y + radius * a.sin());
                canvas.plot(px, py, '*');
            }
            let angle = TAU * k as f64 * tau_now;
            let (c, s) = (angle.cos(), angle.sin());
            x += re * c - im * s;
            y += re * s + im * c;
        }
        // The pen.
        let (px, py) = to_screen(x, y);
        canvas.plot(px, py, '#');
    }

    fn reveal(&self) -> &'static str {
        "Any closed drawing can be traced by fixed-speed rotating circles; the star \
         is stored as a short list of their sizes and speeds. A cardioid needs only \
         two rotating vectors, so up to scale and rotation this same machinery draws \
         the heart wrapped by Times Tables and the main body of the Mandelbrot set."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Ptolemy used exactly this machinery to predict planets from a fixed \
             Earth, and it worked for fourteen centuries, because epicycles are a \
             Fourier series and Fourier series can fit anything. He was not wrong; \
             he was curve-fitting in the sky.",
            "Fourier's 1807 paper claiming any function splits into waves was \
             rejected, with Lagrange among the objectors. The referees demanded \
             rigor nobody alive had. He was right anyway, and the proof machinery \
             built to settle it became modern analysis.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.8
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "D lydian spiral",
            root: 146.83,
            tempo: 96,
            line: &[0, 7, 12, 16, 14, 12, 7, 4],
            encodes: "nested circles orbiting into a single drawn curve",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PERTURB THE CHAIN")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height, aspect, _scale)) = drawing_frame(canvas) else {
            return;
        };
        // base
        self.render(canvas, t);
        // poked: draw mini perturbed traces (full chain paths) at click offsets with phase shift.
        // Shows "the chain" answering the hand, not just a dot.
        let phase = phase_for(t);
        for trace in mini_traces(pokes, phase, self.seed, width, height, aspect) {
            let to_screen = |x: f64, y: f64| {
                (
                    (trace.cx + x * trace.scale) as i32,
                    (trace.cy + y * trace.scale * aspect) as i32,
                )
            };

            // Mini traced path for this perturbed phase (the "chain" response).
            let steps = (trace.tau * 180.0) as usize; // shorter trace for mini
            let mut previous: Option<(i32, i32)> = None;
            for i in 0..=steps {
                let (x, y) = tip(i as f64 / 180.0);
                let point = to_screen(x, y);
                if let Some((px0, py0)) = previous {
                    canvas.line(px0, py0, point.0, point.1, '+');
                }
                previous = Some(point);
            }
            // Pen position for the perturbed.
            let (fx, fy) = tip(trace.tau);
            let (sx, sy) = to_screen(fx, fy);
            canvas.plot(sx, sy, '#');
        }
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The first harmonics as a chord: the shape, heard.
        let _ = t;
        SoundSpec::chord(&[110.0, 220.0, 330.0, 550.0], 1.5, 0.12)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Epicycles, K, MiniTrace, SAMPLES, coefficient, mini_traces, phase_for, phase_offset,
        safe_aspect, target, tip,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room};
    use crate::surface::{MAX_DIM, Surface};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {actual} to be within 1e-9 of {expected}"
        );
    }

    #[test]
    fn the_target_is_closed_and_bounded() {
        let (x0, y0) = target(0);
        let (xn, yn) = target(SAMPLES - 1);
        assert!(
            ((x0 - xn).powi(2) + (y0 - yn).powi(2)).sqrt() < 0.2,
            "closed"
        );
        for i in 0..SAMPLES {
            let (x, y) = target(i);
            assert!(x.hypot(y) <= 1.01, "within the unit star");
        }
    }

    #[test]
    fn the_series_reconstructs_the_star() {
        // The partial sum with K terms lands near the target everywhere.
        let mut worst = 0.0f64;
        for i in 0..SAMPLES {
            let tau = i as f64 / SAMPLES as f64;
            let (tx, ty) = target(i);
            let (fx, fy) = tip(tau);
            worst = worst.max(((tx - fx).powi(2) + (ty - fy).powi(2)).sqrt());
        }
        assert!(
            worst < 0.15,
            "K={K} terms trace the star: worst gap {worst}"
        );
    }

    #[test]
    fn the_constant_term_is_the_centroid() {
        let (re, im) = coefficient(0);
        assert!(re.abs() < 0.05 && im.abs() < 0.05, "the star is centered");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Epicycles::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.8);
        room.render(&mut b, 0.8);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Epicycles::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
        room.render_poked(&mut canvas, f64::NAN, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        assert_eq!(phase_for(f64::NAN), 0.0);
        let room = Epicycles::new();
        let mut base = Canvas::new(40, 30);
        let mut nonfinite = Canvas::new(40, 30);

        room.render(&mut base, 0.0);
        room.render(&mut nonfinite, f64::INFINITY);

        assert_eq!(nonfinite.to_text(), base.to_text());
    }

    #[test]
    fn finite_phase_wraps_instead_of_clamping() {
        let epsilon = 1e-12;

        assert_eq!(phase_for(0.0), 0.0);
        assert_eq!(phase_for(1.0), 0.0);
        assert!((phase_for(1.25) - 0.25).abs() < epsilon);
        assert!((phase_for(9.75) - 0.75).abs() < epsilon);
        assert!((phase_for(-0.25) - 0.75).abs() < epsilon);
    }

    #[test]
    fn reveal_names_the_cardioid_connection() {
        let reveal = Epicycles::new().reveal();
        assert!(reveal.contains("cardioid"));
        assert!(reveal.contains("Times Tables"));
        assert!(reveal.contains("Mandelbrot"));
        assert!(reveal.contains("scale and rotation"));
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = Epicycles::new_with(0);
        let r_def = Epicycles::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(40, 30);
        r0.render_poked(&mut cp, 0.5, &[(0.5, 0.5)]);
        assert!(cp.ink_count() >= a.ink_count());
    }

    #[test]
    fn nonzero_seed_multiple_of_phase_modulus_still_varies() {
        assert_ne!(phase_offset(200), phase_offset(0));
        assert_ne!(phase_offset(201), phase_offset(1));
        let r0 = Epicycles::new_with(0);
        let r200 = Epicycles::new_with(200);
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);

        r0.render(&mut a, 0.5);
        r200.render(&mut b, 0.5);

        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn unsafe_surface_aspect_falls_back_to_one() {
        assert_eq!(safe_aspect(f64::NAN), 1.0);
        assert_eq!(safe_aspect(f64::INFINITY), 1.0);
        assert_eq!(safe_aspect(0.0), 1.0);
        assert_eq!(safe_aspect(-0.5), 1.0);
        assert_eq!(safe_aspect(f64::MIN_POSITIVE), f64::MIN_POSITIVE);
        assert_eq!(safe_aspect(f64::MAX), f64::MAX);
        assert_eq!(safe_aspect(0.5), 0.5);
    }

    #[test]
    fn poked_traces_preserve_order_clamp_and_filter() {
        let actual = mini_traces(
            &[
                (-1.0, 2.0),
                (0.25, 0.75),
                (f64::INFINITY, 0.25),
                (0.5, f64::NAN),
            ],
            0.25,
            0,
            40.0,
            30.0,
            0.5,
        );
        let expected = vec![
            MiniTrace {
                cx: 14.4,
                cy: 19.2,
                scale: 4.4,
                tau: 0.35,
            },
            MiniTrace {
                cx: 17.2,
                cy: 17.1,
                scale: 4.4,
                tau: 0.35,
            },
        ];

        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert_close(actual.cx, expected.cx);
            assert_close(actual.cy, expected.cy);
            assert_close(actual.scale, expected.scale);
            assert_close(actual.tau, expected.tau);
        }
    }

    #[test]
    fn duplicate_pokes_are_replayed_as_duplicate_traces() {
        let traces = mini_traces(&[(0.25, 0.75), (0.25, 0.75)], 0.25, 0, 40.0, 30.0, 0.5);

        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0], traces[1]);
    }

    #[test]
    fn poked_traces_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        many.extend(
            (0..MAX_ROOM_POKES).map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.5)),
        );
        let newest = many[many.len() - MAX_ROOM_POKES..].to_vec();

        assert_eq!(
            mini_traces(&many, 0.1, 0, 40.0, 30.0, 0.5),
            mini_traces(&newest, 0.1, 0, 40.0, 30.0, 0.5)
        );
        assert_eq!(
            mini_traces(&many, 0.1, 0, 40.0, 30.0, 0.5).len(),
            MAX_ROOM_POKES
        );
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(mini_traces(&with_invalid_tail, 0.1, 0, 40.0, 30.0, 0.5).is_empty());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_trace_identity() {
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            mini_traces(&with_bad_points, 0.1, 0, 40.0, 30.0, 0.5),
            mini_traces(&finite, 0.1, 0, 40.0, 30.0, 0.5)
        );
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = Epicycles::new();
        let newest = vec![(0.75, 0.25); MAX_ROOM_POKES];
        let mut all = vec![(0.1, 0.9); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();
        let mut expected = Canvas::new(48, 30);
        let mut actual = Canvas::new(48, 30);
        let mut prefix_only = Canvas::new(48, 30);

        room.render_poked(&mut expected, 0.4, &newest);
        room.render_poked(&mut actual, 0.4, &all);
        room.render_poked(&mut prefix_only, 0.4, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(actual.to_text(), prefix_only.to_text());
    }

    #[test]
    fn all_invalid_pokes_match_base_render() {
        let room = Epicycles::new();
        let mut base = Canvas::new(48, 30);
        let mut poked = Canvas::new(48, 30);

        room.render(&mut base, f64::NAN);
        room.render_poked(
            &mut poked,
            f64::NAN,
            &[(f64::NAN, f64::INFINITY), (f64::INFINITY, 0.5)],
        );

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn nonzero_seed_changes_poked_render() {
        let r0 = Epicycles::new_with(0);
        let r200 = Epicycles::new_with(200);
        let mut a = Canvas::new(48, 30);
        let mut b = Canvas::new(48, 30);

        r0.render_poked(&mut a, 0.3, &[(0.5, 0.5)]);
        r200.render_poked(&mut b, 0.3, &[(0.5, 0.5)]);

        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn nonzero_seed_changes_mini_trace_phase_directly() {
        let base = mini_traces(&[(0.5, 0.5)], 0.3, 0, 48.0, 30.0, 0.5);
        let varied = mini_traces(&[(0.5, 0.5)], 0.3, 200, 48.0, 30.0, 0.5);

        assert_eq!(base.len(), 1);
        assert_eq!(varied.len(), 1);
        assert_eq!(base[0].cx, varied[0].cx);
        assert_eq!(base[0].cy, varied[0].cy);
        assert_eq!(base[0].scale, varied[0].scale);
        assert_ne!(base[0].tau, varied[0].tau);
    }

    #[test]
    fn huge_custom_surface_does_not_overflow_or_run_unbounded_lines() {
        #[derive(Default)]
        struct HugeSurface {
            aspect: f64,
            plots: usize,
            max_abs_coord: i32,
        }

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                usize::MAX
            }

            fn height(&self) -> usize {
                usize::MAX
            }

            fn char_aspect(&self) -> f64 {
                self.aspect
            }

            fn plot(&mut self, x: i32, y: i32, _ch: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = Epicycles::new();
        for aspect in [f64::NAN, f64::MIN_POSITIVE, f64::MAX] {
            let mut surface = HugeSurface {
                aspect,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, f64::NAN, &[(1.0, 1.0)]);
            assert!(surface.plots < 50_000);
            assert!(surface.max_abs_coord <= MAX_DIM as i32);
        }
    }
}
