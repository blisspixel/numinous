//! The Barnsley fern: a living plant grown from four random rules.
//!
//! Start at a point and, over and over, pick one of four affine transformations
//! at random (with fixed probabilities) and apply it. The points settle onto a
//! fern that looks convincingly alive, an iterated function system. `t` grows the
//! fern by drawing more points. See `docs/ROOMS.md`.

use crate::MAX_ROOM_POKES;
use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the fern grows the same way every time.
const SEED: u64 = 0xFE87_0000_5EED_1234;
/// Points drawn per canvas cell at the start of the sweep, and the extra
/// per-cell points `t` adds. Scaling to the canvas keeps the density constant
/// across resolutions: a coarse ASCII grid gets a sparse dusting that leaves the
/// fronds and the negative space between them visible (a filled `#` blob is not a
/// fern), while a high-res raster gets enough points to fill the plant smoothly.
const BASE_POINTS_PER_CELL: f64 = 0.35;
const SWEEP_POINTS_PER_CELL: f64 = 0.9;
/// A floor so a tiny canvas still grows a recognizable plant.
const MIN_POINTS: usize = 1_200;
/// A ceiling so a very large (or hostile) surface cannot drive an unbounded
/// chaos-game loop: it bounds the per-frame cost and keeps a high-resolution
/// fern lush without paying to fill every pixel, which the eye does not need.
const MAX_POINTS: usize = 120_000;
const FERN_X_MIN: f64 = -2.5;
const FERN_X_SPAN: f64 = 5.5;
const FERN_Y_MAX: f64 = 10.0;

/// The Barnsley fern room.
#[derive(Debug, Default)]
pub struct BarnsleyFern {
    seed: u64,
}

impl BarnsleyFern {
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

    /// How many points to draw at phase `t` on a canvas of `cells` cells. Scaled
    /// to the canvas so the fern's density (and so its legible structure) is the
    /// same whether it is drawn on an 80-column terminal or a full-res window.
    fn points_for(t: f64, cells: usize) -> usize {
        let per_cell = BASE_POINTS_PER_CELL + Self::phase_for(t) * SWEEP_POINTS_PER_CELL;
        ((cells as f64 * per_cell) as usize).clamp(MIN_POINTS, MAX_POINTS)
    }

    fn phase_for(t: f64) -> f64 {
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn screen_cell(px: f64, py: f64, width: usize, height: usize) -> Option<(usize, usize)> {
        if width == 0 || height == 0 || !px.is_finite() || !py.is_finite() {
            return None;
        }
        let sx = (px.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as usize;
        let sy = (py.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as usize;
        Some((sx.min(width - 1), sy.min(height - 1)))
    }

    fn world_at_cell(sx: usize, sy: usize, width: usize, height: usize) -> (f64, f64) {
        let x = FERN_X_MIN + ((sx as f64 + 0.5) / width as f64) * FERN_X_SPAN;
        let y = FERN_Y_MAX - ((sy as f64 + 0.5) / height as f64) * FERN_Y_MAX;
        (x, y)
    }

    fn project_point(x: f64, y: f64, width: usize, height: usize) -> Option<(i32, i32)> {
        if width == 0 || height == 0 || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let sx = (((x - FERN_X_MIN) / FERN_X_SPAN) * width as f64).floor() as i32;
        let sy = (height as f64 - (y / FERN_Y_MAX) * height as f64).floor() as i32;
        if sx >= 0 && sx < width as i32 && sy >= 0 && sy < height as i32 {
            Some((sx, sy))
        } else {
            None
        }
    }

    fn planted_points(
        pokes: &[(f64, f64)],
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = (f64, f64, i32, i32)> + '_ {
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        pokes[start..].iter().filter_map(move |&(px, py)| {
            let (sx, sy) = Self::screen_cell(px, py, width, height)?;
            let (x, y) = Self::world_at_cell(sx, sy, width, height);
            Some((x, y, sx as i32, sy as i32))
        })
    }
}

/// Apply one of the fern's four affine maps, chosen by `r` in `[0, 1)`.
fn next_point(x: f64, y: f64, r: f64) -> (f64, f64) {
    if r < 0.01 {
        (0.0, 0.16 * y)
    } else if r < 0.86 {
        (0.85 * x + 0.04 * y, -0.04 * x + 0.85 * y + 1.6)
    } else if r < 0.93 {
        (0.2 * x - 0.26 * y, 0.23 * x + 0.22 * y + 1.6)
    } else {
        (-0.15 * x + 0.28 * y, 0.26 * x + 0.24 * y + 0.44)
    }
}

impl Room for BarnsleyFern {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "barnsley-fern",
            title: "Barnsley Fern",
            wing: "Fractals & the Infinite",
            blurb: "Pick one of four simple transformations at random, over and over, and a fern \
                    grows out of the noise. t adds more points, growing the plant before your eyes.",
            accent: [60, 200, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let mut rng = SplitMix64::new(SEED ^ self.seed);
        let (mut x, mut y) = (0.0_f64, 0.0_f64);
        // A hostile Surface can report dimensions whose product overflows usize;
        // saturate so the point count stays bounded and the render never panics,
        // exactly as the Room contract requires. Real surfaces clamp to MAX_DIM.
        let cells = width.saturating_mul(height);
        for _ in 0..Self::points_for(t, cells) {
            let (nx, ny) = next_point(x, y, rng.next_f64());
            x = nx;
            y = ny;
            if let Some((sx, sy)) = Self::project_point(x, y, width, height) {
                canvas.plot(sx, sy, '#');
            }
        }
    }

    fn postcard_t(&self) -> f64 {
        1.0
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "E fern",
            root: 164.81,
            tempo: 104,
            line: &[0, 7, 12, 9, 5, 12, 7, 0],
            encodes: "contractive maps repeating stem, leaf, and leaflet",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLANT A NEW POINT")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        self.render(canvas, t);
        let mut rng = SplitMix64::new(SEED ^ self.seed);
        for (mut x, mut y, sx, sy) in Self::planted_points(pokes, width, height) {
            canvas.plot(sx, sy, '+');
            for _ in 0..100 {
                let r = rng.next_f64();
                let (nx, ny) = next_point(x, y, r);
                x = nx;
                y = ny;
                if let Some((sx, sy)) = Self::project_point(x, y, width, height) {
                    canvas.plot(sx, sy, '+');
                }
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "This fern is not drawn, it is grown. Four transformations applied at \
         random build a plant, self-similar down to each frond. The entire \
         genome of this fern fits on an index card."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Barnsley's collage theorem runs the trick backward: given any image, it \
             tells you how to find transformations whose attractor approximates it. \
             This became fractal image compression, which shipped on CD-ROM \
             encyclopedias in the nineties.",
            "The fern needs the probabilities as much as the maps: pick the four \
             transformations uniformly and the fern grows stunted and patchy. The 85 \
             percent rule is what fills the frond evenly. Randomness, tuned.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{BarnsleyFern, SEED, next_point};
    use crate::MAX_ROOM_POKES;
    use crate::canvas::Canvas;
    use crate::rng::SplitMix64;
    use crate::room::Room;

    #[test]
    fn points_stay_within_the_fern_bounds() {
        let mut rng = SplitMix64::new(SEED);
        let (mut x, mut y) = (0.0, 0.0);
        for _ in 0..5_000 {
            let (nx, ny) = next_point(x, y, rng.next_f64());
            x = nx;
            y = ny;
            assert!(x > -3.0 && x < 3.5, "x out of bounds: {x}");
            assert!((0.0..11.0).contains(&y), "y out of bounds: {y}");
        }
    }

    #[test]
    fn more_phase_grows_the_fern() {
        assert!(BarnsleyFern::points_for(1.0, 4_000) > BarnsleyFern::points_for(0.0, 4_000));
    }

    #[test]
    fn a_bigger_canvas_draws_proportionally_more_points() {
        // Density is scaled to the canvas, so structure stays legible at any size.
        assert!(BarnsleyFern::points_for(0.5, 40_000) > BarnsleyFern::points_for(0.5, 4_000));
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        assert_eq!(
            BarnsleyFern::points_for(f64::NAN, 4_000),
            BarnsleyFern::points_for(0.0, 4_000)
        );
        assert_eq!(
            BarnsleyFern::points_for(f64::INFINITY, 4_000),
            BarnsleyFern::points_for(0.0, 4_000)
        );
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = BarnsleyFern::new();
        let mut a = Canvas::new(40, 40);
        let mut b = Canvas::new(40, 40);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = BarnsleyFern::new_with(0);
        let r_def = BarnsleyFern::new();
        let mut a = Canvas::new(40, 40);
        let mut b = Canvas::new(40, 40);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        let r42 = BarnsleyFern::new_with(42);
        let mut c = Canvas::new(40, 40);
        r42.render(&mut c, 0.5);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = BarnsleyFern::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
        room.render_poked(&mut canvas, f64::NAN, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn a_hostile_huge_surface_renders_bounded_without_panicking() {
        use crate::surface::Surface;
        struct HugeSurface;
        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                usize::MAX
            }
            fn height(&self) -> usize {
                usize::MAX
            }
            fn plot(&mut self, _x: i32, _y: i32, _ch: char) {}
        }
        // width * height would overflow usize and the point count would be
        // unbounded; saturating_mul plus the MAX_POINTS cap keep it finite and
        // quick, as the Room contract requires.
        let room = BarnsleyFern::new();
        let mut surface = HugeSurface;
        room.render(&mut surface, 1.0);
        room.render_poked(&mut surface, f64::NAN, &[(1.0, 1.0)]);
    }

    #[test]
    fn reveal_mentions_the_index_card() {
        assert!(BarnsleyFern::new().reveal().contains("index card"));
    }

    #[test]
    fn poked_changes_output() {
        let r0 = BarnsleyFern::new_with(0);
        let mut cp = Canvas::new(40, 40);
        let mut c0 = Canvas::new(40, 40);
        r0.render_poked(&mut cp, 0.5, &[(0.5, 0.5)]);
        r0.render(&mut c0, 0.5);
        assert!(
            cp.ink_count() != c0.ink_count() || cp.to_text() != c0.to_text(),
            "poke should change output"
        );
    }

    #[test]
    fn planted_points_preserve_order_clamp_and_filter() {
        let points: Vec<_> = BarnsleyFern::planted_points(
            &[
                (-1.0, 0.0),
                (f64::NAN, 0.5),
                (0.5, f64::INFINITY),
                (0.5, 0.5),
                (2.0, 1.0),
            ],
            40,
            40,
        )
        .map(|(x, y, sx, sy)| {
            assert_eq!(BarnsleyFern::project_point(x, y, 40, 40), Some((sx, sy)));
            (sx, sy)
        })
        .collect();

        assert_eq!(points, vec![(0, 0), (20, 20), (39, 39)]);
    }

    #[test]
    fn planted_points_are_screen_space_faithful_at_edges() {
        let points: Vec<_> =
            BarnsleyFern::planted_points(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)], 40, 40)
                .map(|(x, y, sx, sy)| {
                    assert_eq!(BarnsleyFern::project_point(x, y, 40, 40), Some((sx, sy)));
                    (sx, sy)
                })
                .collect();

        assert_eq!(points, vec![(0, 0), (39, 0), (0, 39), (39, 39)]);
    }

    #[test]
    fn render_poked_marks_the_clicked_cell_before_growth() {
        let room = BarnsleyFern::new();
        let mut canvas = Canvas::new(40, 40);

        room.render_poked(&mut canvas, 0.5, &[(1.0, 0.0)]);

        let text = canvas.to_text();
        let top_row = text.lines().next().expect("top row");
        assert_eq!(top_row.as_bytes()[39], b'+');
    }

    #[test]
    fn planted_points_filter_bad_values_before_mapping() {
        let points: Vec<_> = BarnsleyFern::planted_points(
            &[
                (-1.0, 0.0),
                (f64::NAN, 0.5),
                (0.5, f64::INFINITY),
                (0.5, 0.5),
                (2.0, 1.0),
            ],
            40,
            40,
        )
        .collect();

        assert_eq!(points.len(), 3);
    }

    #[test]
    fn planted_points_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        let newest: Vec<_> = many[many.len() - MAX_ROOM_POKES..].to_vec();
        many[0] = (1.0, 1.0);

        let expected: Vec<_> = BarnsleyFern::planted_points(&newest, 40, 40).collect();
        let actual: Vec<_> = BarnsleyFern::planted_points(&many, 40, 40).collect();

        assert_eq!(actual, expected);
        assert_eq!(actual.len(), MAX_ROOM_POKES);
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_planted_identity() {
        let room = BarnsleyFern::new();
        let finite = [(0.25, 0.25), (0.75, 0.75)];
        let with_bad_points = [(f64::NAN, 0.0), finite[0], (0.0, f64::INFINITY), finite[1]];

        let mut expected = Canvas::new(40, 40);
        let mut actual = Canvas::new(40, 40);
        room.render_poked(&mut expected, 0.5, &finite);
        room.render_poked(&mut actual, 0.5, &with_bad_points);

        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.25, 0.25); MAX_ROOM_POKES];
        with_invalid_tail.push((f64::NAN, f64::INFINITY));

        let points: Vec<_> = BarnsleyFern::planted_points(&with_invalid_tail, 40, 40).collect();

        assert_eq!(points.len(), MAX_ROOM_POKES - 1);
        assert!(points.iter().all(|&(_, _, sx, sy)| (sx, sy) == (10, 10)));
    }

    #[test]
    fn oversized_poke_slices_render_like_their_newest_bounded_tail() {
        let room = BarnsleyFern::new();
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

        let mut expected = Canvas::new(40, 40);
        let mut actual = Canvas::new(40, 40);
        let mut prefix_only = Canvas::new(40, 40);
        room.render_poked(&mut expected, 0.5, &newest);
        room.render_poked(&mut actual, 0.5, &all);
        room.render_poked(&mut prefix_only, 0.5, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(prefix_only.to_text(), expected.to_text());
    }

    #[test]
    fn new_with_nonzero_affects_poked_output() {
        let r0 = BarnsleyFern::new_with(0);
        let r42 = BarnsleyFern::new_with(42);
        let mut cp0 = Canvas::new(40, 40);
        let mut cp42 = Canvas::new(40, 40);
        r0.render_poked(&mut cp0, 0.5, &[(0.5, 0.5)]);
        r42.render_poked(&mut cp42, 0.5, &[(0.5, 0.5)]);
        assert_ne!(
            cp0.to_text(),
            cp42.to_text(),
            "variation seed must affect poked render for replayable per-visit pokes"
        );
    }
}
