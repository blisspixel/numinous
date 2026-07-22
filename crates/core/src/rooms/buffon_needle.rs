//! Buffon's Needle: pi out of thrown sticks.
//!
//! Drop needles on a floor of evenly spaced parallel lines and count how many
//! cross a line. The crossing fraction is `2 l / (pi d)`, so pi falls out of an
//! experiment with no circle anywhere in it. This room drops needles on a lined
//! canvas (crossing needles highlighted) and can estimate pi. `t` changes the
//! needle length. See `docs/ROOMS.md`.

use std::f64::consts::PI;

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the throw reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x0B0F_0000_5EED_F00D;
/// Number of needles dropped.
const NEEDLES: usize = 1500;
/// The shared normalized floor. Six strips keep the boards legible in a
/// terminal while making displayed intersections identical across faces.
const FLOOR_STRIPS: f64 = 6.0;

/// The Buffon's Needle room.
#[derive(Debug, Default)]
pub struct BuffonNeedle {
    seed: u64,
}

impl BuffonNeedle {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replay.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// The needle-length-to-spacing ratio for phase `t`; 1.0 (the classic case) at `t = 0`.
    fn length_ratio_for(t: f64) -> f64 {
        1.0 - 0.6 * Self::phase_for(t)
    }

    fn phase_for(t: f64) -> f64 {
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn floor_spacing(canvas: &dyn Surface) -> f64 {
        let (_, height) = canvas.draw_bounds();
        (height.saturating_sub(1) as f64 / FLOOR_STRIPS).max(1.0)
    }

    fn screen_cell(px: f64, py: f64, width: usize, height: usize) -> Option<(f64, f64, i32, i32)> {
        if width == 0 || height == 0 || !px.is_finite() || !py.is_finite() {
            return None;
        }
        let sx = (px.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as usize;
        let sy = (py.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as usize;
        let sx = sx.min(width - 1);
        let sy = sy.min(height - 1);
        Some((sx as f64, sy as f64, sx as i32, sy as i32))
    }

    fn dropped_needles(
        pokes: &[(f64, f64)],
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = (f64, f64, i32, i32)> + '_ {
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        pokes[start..]
            .iter()
            .filter_map(move |&(px, py)| Self::screen_cell(px, py, width, height))
    }

    fn player_results(
        pokes: &[(f64, f64)],
        t: f64,
        seed: u64,
    ) -> impl Iterator<Item = (f64, bool)> + '_ {
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        let half = Self::length_ratio_for(t) / 2.0;
        let mut rng = SplitMix64::new(SEED ^ seed);
        pokes[start..].iter().filter_map(move |&(px, py)| {
            if !px.is_finite() || !py.is_finite() {
                return None;
            }
            let angle = rng.next_f64() * PI;
            let center = py.clamp(0.0, 1.0) * FLOOR_STRIPS;
            Some((angle, crosses(center, angle, half, 1.0)))
        })
    }

    fn click_pokes(inputs: &[RoomInput]) -> Vec<(f64, f64)> {
        let mut points: Vec<_> = inputs
            .iter()
            .filter_map(|input| match *input {
                RoomInput::PointerDown { x, y, .. } => Some((x, y)),
                _ => None,
            })
            .collect();
        if points.len() > MAX_ROOM_POKES {
            points.drain(..points.len() - MAX_ROOM_POKES);
        }
        points
    }

    /// Finite player throw count under the same click contract as the plate.
    ///
    /// Faces use this when staging the engineered aha so throw priming and the
    /// status line never disagree.
    #[must_use]
    pub fn throw_count(inputs: &[RoomInput]) -> usize {
        Self::click_pokes(inputs)
            .iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .count()
    }

    /// Estimate pi by dropping `needles` needles with the given length ratio.
    ///
    /// Deterministic (fixed seed). Returns infinity if nothing crosses. Exposed
    /// so a face can display the running estimate; the render itself only draws
    /// the experiment.
    #[must_use]
    pub fn estimate_pi(needles: usize, length_ratio: f64) -> f64 {
        Self::estimate_pi_with_variation(needles, length_ratio, 0)
    }

    /// Estimate pi with an explicit replay seed.
    #[must_use]
    pub fn estimate_pi_with_variation(needles: usize, length_ratio: f64, variation: u64) -> f64 {
        let mut rng = SplitMix64::new(SEED ^ variation);
        let half = length_ratio / 2.0;
        let mut crossings = 0usize;
        for _ in 0..needles {
            let center = rng.next_f64(); // within one unit-spaced strip
            let angle = rng.next_f64() * PI;
            if crosses(center, angle, half, 1.0) {
                crossings += 1;
            }
        }
        if crossings == 0 {
            return f64::INFINITY;
        }
        2.0 * length_ratio * needles as f64 / crossings as f64
    }

    fn render_scene(&self, canvas: &mut dyn Surface, t: f64, dim_ambient: bool) {
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let spacing = Self::floor_spacing(canvas);
        let mut row = 0.0_f64;
        while row < height as f64 {
            for x in 0..width {
                canvas.plot(x as i32, row.round() as i32, '*');
            }
            row += spacing;
        }

        let half_len = Self::length_ratio_for(t) * spacing / 2.0;
        let (fw, fh) = (width as f64, height as f64);
        let mut rng = SplitMix64::new(SEED ^ self.seed);
        let count = (width * height / 300).clamp(150, NEEDLES);
        for _ in 0..count {
            let cx = rng.next_f64() * fw;
            let cy = rng.next_f64() * fh;
            let angle = rng.next_f64() * PI;
            let (hx, hy) = (half_len * angle.cos(), half_len * angle.sin());
            let mark = if dim_ambient {
                '-'
            } else if crosses(cy, angle, half_len, spacing) {
                '#'
            } else {
                '*'
            };
            canvas.line(
                (cx - hx).round() as i32,
                (cy - hy).round() as i32,
                (cx + hx).round() as i32,
                (cy + hy).round() as i32,
                mark,
            );
        }
    }
}

impl Room for BuffonNeedle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "buffon-needle",
            title: "Buffon's Needle",
            wing: "Chance & Order",
            blurb: "Throw sticks onto parallel floorboards. Bright sticks cross a line; the ratio \
                    of crossings to throws wanders toward pi, with no circle anywhere in sight.",
            accent: [140, 100, 230],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.render_scene(canvas, t, false);
    }

    fn reveal(&self) -> &'static str {
        "There is no circle here, just sticks on a floor, yet pi, the circle's own \
         number, appears out of nowhere. This is the seed of the Monte Carlo \
         method, which helped design the atom bomb and powers modern finance and AI. \
         You can compute the universe by throwing dice."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Buffon posed this in 1777 as a gambling question about floorboards. It \
             is the first problem in what is now called geometric probability, and \
             the ancestor of every Monte Carlo simulation run today.",
            "In 1901 Lazzarini reported pi as 355 over 113 after exactly 3408 throws, \
             an implausibly perfect result. He almost certainly stopped the moment \
             the estimate looked good, which makes him a cautionary tale in every \
             statistics course since.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "A dorian fall",
            root: 220.0,
            tempo: 76,
            line: &[0, 5, 2, 7, 3, 8, 5, 10],
            encodes: "needles falling until crossings estimate pi",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: THROW A NEEDLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        // First contact is an invitation, not a finished Monte Carlo report.
        // The rain of ambient needles is Watch mode; the estimate belongs to
        // the player's own throws (see status_input). Scrubbing phase only
        // changes the needle length ratio L/D that the throws will use.
        let ratio = Self::length_ratio_for(t);
        let theory = 2.0 * ratio / PI;
        Some(format!(
            "L/D {ratio:.2}   CROSS CHANCE {theory:.3}   CLICK: THROW"
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = Self::click_pokes(inputs);
        let results: Vec<_> = Self::player_results(&pokes, t, self.seed).collect();
        let throws = results.len();
        if throws == 0 {
            return self.status(t);
        }
        let crossings = results.iter().filter(|(_, crossed)| *crossed).count();
        let latest = if results.last().is_some_and(|(_, crossed)| *crossed) {
            "CROSS"
        } else {
            "MISS"
        };
        if crossings == 0 {
            return Some(format!("{throws} THROW  LAST {latest}  0 CROSS  NEED HIT"));
        }
        let estimate = 2.0 * Self::length_ratio_for(t) * throws as f64 / crossings as f64;
        Some(format!(
            "{throws} THROW  LAST {latest}  {crossings}X  PI~{estimate:.3}"
        ))
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let pokes = Self::click_pokes(inputs);
        self.render_poked(canvas, t, &pokes);
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        self.render_scene(canvas, t, true);
        let spacing = Self::floor_spacing(canvas);
        let half_len = Self::length_ratio_for(t) * spacing / 2.0;
        for ((cx, cy, sx, sy), (angle, crossed)) in Self::dropped_needles(pokes, width, height)
            .zip(Self::player_results(pokes, t, self.seed))
        {
            let (hx, hy) = (half_len * angle.cos(), half_len * angle.sin());
            let mark = if crossed { '#' } else { '*' };
            let x0 = (cx - hx).round() as i32;
            let y0 = (cy - hy).round() as i32;
            let x1 = (cx + hx).round() as i32;
            let y1 = (cy + hy).round() as i32;
            for offset in -1..=1 {
                canvas.line(x0, y0 + offset, x1, y1 + offset, mark);
            }
            canvas.line(sx - 2, sy, sx + 2, sy, '#');
            canvas.line(sx, sy - 2, sx, sy + 2, '#');
        }
    }
}

/// Whether a needle whose center sits at `center` (in strips of width `spacing`)
/// and makes angle `angle` with the lines crosses a line, given half its length.
fn crosses(center: f64, angle: f64, half_len: f64, spacing: f64) -> bool {
    let reach = half_len * angle.sin().abs();
    let within_strip = center.rem_euclid(spacing);
    let distance_to_nearest_line = within_strip.min(spacing - within_strip);
    distance_to_nearest_line <= reach
}

#[cfg(test)]
mod tests {
    use super::{BuffonNeedle, crosses};
    use crate::MAX_ROOM_POKES;
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{Room, RoomInput};
    use std::f64::consts::PI;

    #[test]
    fn crossing_test_matches_geometry() {
        // A vertical needle of length 1 centered mid-strip reaches both lines.
        assert!(crosses(0.5, PI / 2.0, 0.5, 1.0));
        // A needle parallel to the lines has no vertical reach; mid-strip it misses.
        assert!(!crosses(0.5, 0.0, 0.5, 1.0));
    }

    #[test]
    fn player_grading_matches_the_displayed_floor_across_surfaces() {
        for y in [0.13, 0.31, 0.47, 0.72, 0.91] {
            let poke = [(0.5, y)];
            let (angle, reported) = BuffonNeedle::player_results(&poke, 0.0, 0)
                .next()
                .expect("finite throw");
            for (width, height) in [(50, 24), (360, 240), (640, 480)] {
                let surface = Raster::new(width, height);
                let spacing = BuffonNeedle::floor_spacing(&surface);
                let (_, center, _, _) =
                    BuffonNeedle::screen_cell(0.5, y, width, height).expect("screen point");
                let visible = crosses(center, angle, spacing / 2.0, spacing);
                assert_eq!(
                    reported, visible,
                    "grading differs at {width}x{height}, y={y}, angle={angle}"
                );
            }
        }
    }

    #[test]
    fn estimate_converges_to_pi() {
        let estimate = BuffonNeedle::estimate_pi(200_000, 1.0);
        assert!((estimate - PI).abs() < 0.1, "estimate was {estimate}");
    }

    #[test]
    fn seeded_estimate_preserves_the_default_estimator() {
        assert_eq!(
            BuffonNeedle::estimate_pi(10_000, 1.0),
            BuffonNeedle::estimate_pi_with_variation(10_000, 1.0, 0)
        );
    }

    #[test]
    fn length_ratio_defaults_to_one() {
        assert!((BuffonNeedle::length_ratio_for(0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_length() {
        assert_eq!(
            BuffonNeedle::length_ratio_for(f64::NAN),
            BuffonNeedle::length_ratio_for(0.0)
        );
        assert_eq!(
            BuffonNeedle::length_ratio_for(f64::INFINITY),
            BuffonNeedle::length_ratio_for(0.0)
        );
        assert_eq!(
            BuffonNeedle::length_ratio_for(f64::NEG_INFINITY),
            BuffonNeedle::length_ratio_for(0.0)
        );
    }

    #[test]
    fn render_is_deterministic() {
        let room = BuffonNeedle::new();
        let mut a = Canvas::new(50, 24);
        let mut b = Canvas::new(50, 24);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = BuffonNeedle::new_with(0);
        let r_def = BuffonNeedle::new();
        let mut a = Canvas::new(50, 24);
        let mut b = Canvas::new(50, 24);
        r0.render(&mut a, 0.0);
        r_def.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        let r42 = BuffonNeedle::new_with(42);
        let mut c = Canvas::new(50, 24);
        r42.render(&mut c, 0.0);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = BuffonNeedle::new();
        let mut canvas = Canvas::new(50, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = BuffonNeedle::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(5, 5);
        for t in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -2.0,
            0.0,
            0.999,
            3.0,
        ] {
            room.render(&mut canvas, t);
        }
        room.render_poked(&mut canvas, f64::NAN, &[(f64::NAN, f64::INFINITY)]);
    }

    #[test]
    fn reveal_names_monte_carlo() {
        let reveal = BuffonNeedle::new().reveal();
        assert!(reveal.contains("Monte Carlo"));
        assert!(
            reveal.contains("AI. You"),
            "continued prose keeps its word boundary"
        );
    }

    #[test]
    fn first_contact_status_invites_a_throw_instead_of_a_finished_estimate() {
        let room = BuffonNeedle::new();
        let open = room.status(0.0).expect("open status");
        assert!(open.contains("L/D 1.00"), "{open}");
        assert!(open.contains("CROSS CHANCE"), "{open}");
        assert!(open.contains("CLICK: THROW"), "{open}");
        assert!(
            !open.contains("PI ABOUT"),
            "untouched status must not claim a pi estimate: {open}"
        );
        assert!(
            !open.contains("THROWS"),
            "untouched status must not invent throw counts: {open}"
        );

        let short = room.status(1.0).expect("short needles");
        assert!(short.contains("L/D 0.40"), "{short}");
        // 2 * 0.40 / pi ~ 0.255: the classical crossing probability.
        assert!(short.contains("CROSS CHANCE 0.255"), "{short}");

        assert_eq!(
            room.status_input(0.0, &[]).as_deref(),
            room.status(0.0).as_deref()
        );
    }

    #[test]
    fn player_status_reports_outcome_crossings_and_a_running_estimate() {
        let room = BuffonNeedle::new();
        let inputs: Vec<_> = (0..20)
            .map(|i| RoomInput::PointerDown {
                x: 0.5,
                y: (i as f64 + 0.25) / 20.0,
                t: 0.0,
            })
            .collect();
        let status = room.status_input(0.0, &inputs).expect("status");
        assert!(status.contains("20 THROW"), "{status}");
        assert!(status.contains("LAST "), "{status}");
        assert!(status.contains('X'), "{status}");
        assert!(status.contains("PI~"), "{status}");
    }

    #[test]
    fn pointer_motion_does_not_invent_extra_click_throws() {
        let room = BuffonNeedle::new();
        let inputs = [
            RoomInput::PointerDown {
                x: 0.5,
                y: 0.5,
                t: 0.0,
            },
            RoomInput::PointerMove {
                x: 0.51,
                y: 0.51,
                t: 0.01,
            },
            RoomInput::PointerUp {
                x: 0.51,
                y: 0.51,
                t: 0.02,
            },
        ];
        let status = room.status_input(0.0, &inputs).expect("status");
        assert!(status.contains("1 THROW"), "{status}");

        let mut from_input = Canvas::new(50, 24);
        let mut from_click = Canvas::new(50, 24);
        room.render_input(&mut from_input, 0.0, &inputs);
        room.render_poked(&mut from_click, 0.0, &[(0.5, 0.5)]);
        assert_eq!(from_input.to_text(), from_click.to_text());
    }

    #[test]
    fn poked_changes_output() {
        let r0 = BuffonNeedle::new_with(0);
        let mut cp = Canvas::new(50, 24);
        let mut c0 = Canvas::new(50, 24);
        r0.render_poked(&mut cp, 0.0, &[(0.5, 0.5)]);
        r0.render(&mut c0, 0.0);
        assert!(
            cp.ink_count() != c0.ink_count() || cp.to_text() != c0.to_text(),
            "poke should change output"
        );
    }

    #[test]
    fn player_needle_is_legible_on_a_large_raster() {
        let room = BuffonNeedle::new();
        let mut base = Raster::with_accent(900, 700, room.meta().accent);
        let mut poked = Raster::with_accent(900, 700, room.meta().accent);
        room.render_scene(&mut base, 0.0, true);
        room.render_poked(&mut poked, 0.0, &[(0.5, 0.5)]);

        let mut min_x = 900;
        let mut max_x = 0;
        let mut min_y = 700;
        let mut max_y = 0;
        let mut changed = 0;
        for (index, (a, b)) in base
            .to_rgba()
            .chunks_exact(4)
            .zip(poked.to_rgba().chunks_exact(4))
            .enumerate()
        {
            if a != b {
                changed += 1;
                let x = index % 900;
                let y = index / 900;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
        assert!(changed > 20, "only {changed} pixels answered the throw");
        let span = max_x.saturating_sub(min_x).max(max_y.saturating_sub(min_y));
        assert!(span >= 12, "needle still reads as a dot");
    }

    #[test]
    fn dropped_needles_preserve_order_clamp_and_filter() {
        let points: Vec<_> = BuffonNeedle::dropped_needles(
            &[
                (-1.0, 0.0),
                (f64::NAN, 0.5),
                (0.5, f64::INFINITY),
                (0.5, 0.5),
                (2.0, 1.0),
            ],
            40,
            20,
        )
        .map(|(_, _, sx, sy)| (sx, sy))
        .collect();

        assert_eq!(points, vec![(0, 0), (20, 10), (39, 19)]);
    }

    #[test]
    fn dropped_needles_are_screen_space_faithful_at_edges() {
        let points: Vec<_> = BuffonNeedle::dropped_needles(
            &[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
            40,
            20,
        )
        .map(|(cx, cy, sx, sy)| {
            assert_eq!(cx.round() as i32, sx);
            assert_eq!(cy.round() as i32, sy);
            (sx, sy)
        })
        .collect();

        assert_eq!(points, vec![(0, 0), (39, 0), (0, 19), (39, 19)]);
    }

    #[test]
    fn render_poked_marks_the_clicked_cell() {
        let room = BuffonNeedle::new();
        let mut canvas = Canvas::new(40, 20);

        room.render_poked(&mut canvas, 0.0, &[(1.0, 0.0)]);

        let text = canvas.to_text();
        let top_row = text.lines().next().expect("top row");
        assert_eq!(top_row.as_bytes()[39], b'#');
    }

    #[test]
    fn dropped_needles_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        let newest: Vec<_> = many[many.len() - MAX_ROOM_POKES..].to_vec();
        many[0] = (1.0, 1.0);

        let expected: Vec<_> = BuffonNeedle::dropped_needles(&newest, 40, 20).collect();
        let actual: Vec<_> = BuffonNeedle::dropped_needles(&many, 40, 20).collect();

        assert_eq!(actual, expected);
        assert_eq!(actual.len(), MAX_ROOM_POKES);
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_needle_identity() {
        let room = BuffonNeedle::new();
        let finite = [(0.25, 0.25), (0.75, 0.75)];
        let with_bad_points = [(f64::NAN, 0.0), finite[0], (0.0, f64::INFINITY), finite[1]];

        let mut expected = Canvas::new(40, 20);
        let mut actual = Canvas::new(40, 20);
        room.render_poked(&mut expected, 0.5, &finite);
        room.render_poked(&mut actual, 0.5, &with_bad_points);

        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.25, 0.25); MAX_ROOM_POKES];
        with_invalid_tail.push((f64::NAN, f64::INFINITY));

        let points: Vec<_> = BuffonNeedle::dropped_needles(&with_invalid_tail, 40, 20).collect();

        assert_eq!(points.len(), MAX_ROOM_POKES - 1);
        assert!(points.iter().all(|&(_, _, sx, sy)| (sx, sy) == (10, 5)));
    }

    #[test]
    fn oversized_poke_slices_render_like_their_newest_bounded_tail() {
        let room = BuffonNeedle::new();
        let discarded_prefix = vec![(1.0, 1.0), (0.9, 0.1), (0.8, 0.2)];
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    (f64::from((i % 7) as u32) + 0.5) / 7.0,
                    (f64::from((i % 5) as u32) + 0.5) / 5.0,
                )
            })
            .collect();
        let mut all = discarded_prefix.clone();
        all.extend_from_slice(&newest);

        let mut expected = Canvas::new(40, 20);
        let mut actual = Canvas::new(40, 20);
        let mut prefix_only = Canvas::new(40, 20);
        room.render_poked(&mut expected, 0.5, &newest);
        room.render_poked(&mut actual, 0.5, &all);
        room.render_poked(&mut prefix_only, 0.5, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(prefix_only.to_text(), expected.to_text());
    }

    #[test]
    fn new_with_nonzero_affects_poked_output() {
        let r0 = BuffonNeedle::new_with(0);
        let r42 = BuffonNeedle::new_with(42);
        let mut cp0 = Canvas::new(40, 20);
        let mut cp42 = Canvas::new(40, 20);
        r0.render_poked(&mut cp0, 0.5, &[(0.5, 0.5)]);
        r42.render_poked(&mut cp42, 0.5, &[(0.5, 0.5)]);
        assert_ne!(
            cp0.to_text(),
            cp42.to_text(),
            "variation seed must affect poked render for replayable per-visit pokes"
        );
    }
}
