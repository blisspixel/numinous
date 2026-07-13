//! Julia sets: the Mandelbrot set's infinite family of cousins.
//!
//! Same iteration as Mandelbrot (`z -> z*z + c`), but here `c` is a fixed
//! constant and the starting point is the pixel. Each value of `c` gives a
//! completely different fractal; `t` walks `c` around a circle, morphing the
//! shape from a connected blob into scattered dust and back. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

use super::FRACTAL_MAX_ITER;
/// Radius of the circle in the `c` plane that `t` walks around.
const C_RADIUS: f64 = 0.7885;

#[derive(Debug, Clone, Copy, PartialEq)]
struct MorphPoint {
    x: i32,
    y: i32,
    nx: f64,
    ny: f64,
}

fn finite_phase(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn drawing_dims(canvas: &dyn Surface) -> Option<(usize, usize)> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width.min(MAX_DIM), height.min(MAX_DIM)))
    }
}

fn screen_coord(norm: f64, extent: usize) -> i32 {
    debug_assert!(extent > 0);
    (norm.clamp(0.0, 1.0) * extent.saturating_sub(1) as f64).round() as i32
}

fn bounded_morph_points(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<MorphPoint> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(x, y)| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let nx = x.clamp(0.0, 1.0);
            let ny = y.clamp(0.0, 1.0);
            Some(MorphPoint {
                x: screen_coord(nx, width),
                y: screen_coord(ny, height),
                nx,
                ny,
            })
        })
        .collect()
}

/// The Julia set room.
#[derive(Debug, Default)]
pub struct Julia {
    seed: u64,
}

impl Julia {
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

    /// The constant `c` at phase `t`.
    fn c_for(&self, t: f64) -> (f64, f64) {
        automatic_c(t, self.seed)
    }
}

/// Return the automatic complex constant for the current sweep.
#[must_use]
pub fn automatic_c(t: f64, seed: u64) -> (f64, f64) {
    let theta = TAU * finite_phase(t);
    let seed_offset = (seed % 1000) as f64 * 0.00001;
    (
        C_RADIUS * theta.cos() + seed_offset,
        C_RADIUS * theta.sin() + seed_offset,
    )
}

/// Return the complex constant selected by the newest finite hand point.
///
/// The accelerated app and core renderer share this function so a morph keeps
/// the same whole-fractal response and rendering pipeline on GPU systems.
#[must_use]
pub fn selected_c(t: f64, seed: u64, pokes: &[(f64, f64)]) -> (f64, f64) {
    let (base_x, base_y) = automatic_c(t, seed);
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    let selected = pokes[start..]
        .iter()
        .rev()
        .find(|(x, y)| x.is_finite() && y.is_finite());
    selected.map_or((base_x, base_y), |&(x, y)| {
        (
            base_x + (x.clamp(0.0, 1.0) - 0.5) * 0.2,
            base_y + (y.clamp(0.0, 1.0) - 0.5) * 0.2,
        )
    })
}

/// Iterations of `z -> z*z + c` from `(zx, zy)` before escaping.
fn escape_iters(mut zx: f64, mut zy: f64, cx: f64, cy: f64, max: u32) -> u32 {
    let mut i = 0;
    while i < max && zx * zx + zy * zy <= 4.0 {
        let next_x = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = next_x;
        i += 1;
    }
    i
}

fn render_with_c(canvas: &mut dyn Surface, width: usize, height: usize, cx: f64, cy: f64) {
    let scale = 3.2 / width as f64;
    let half_w = width as f64 / 2.0;
    let half_h = height as f64 / 2.0;
    for py in 0..height {
        for px in 0..width {
            let zx = (px as f64 - half_w) * scale;
            let zy = (py as f64 - half_h) * scale;
            let iters = escape_iters(zx, zy, cx, cy, FRACTAL_MAX_ITER);
            let mark = if iters > 20 {
                '#'
            } else if iters > 5 {
                '*'
            } else if iters > 2 {
                '-'
            } else {
                continue;
            };
            canvas.plot(px as i32, py as i32, mark);
        }
    }
}

impl Room for Julia {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "julia",
            title: "Julia Set",
            wing: "Fractals & the Infinite",
            blurb: "The same rule as Mandelbrot, but c is fixed and the whole plane is the seed. \
                    Every c grows a different fractal; t walks c around a circle to morph it.",
            accent: [255, 120, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let (cx, cy) = self.c_for(t);
        render_with_c(canvas, width, height, cx, cy);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn reveal(&self) -> &'static str {
        "There is one Julia set for every point in the plane, an uncountable \
         infinity of them. Whether each one is a single connected piece or a \
         cloud of dust is decided by that point's place in the Mandelbrot set."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Gaston Julia and Pierre Fatou mapped these sets between 1917 and 1919 \
             with no computer, by pure reasoning about iteration. Julia did much of \
             the work while recovering from losing his nose in the First World War.",
            "Run Newton's method to find the roots of an equation and color each \
             starting point by which root it lands on: the boundaries between the \
             basins are Julia sets. The fractal was hiding inside the calculus \
             classroom the whole time.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C# minor boundary",
            root: 138.59,
            tempo: 100,
            line: &[0, 3, 7, 11, 8, 4, 1, 5],
            encodes: "one complex constant pulling the edge of infinity",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: MORPH C")
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes: Vec<_> = inputs
            .iter()
            .filter_map(|input| match *input {
                RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. } => {
                    Some((x, y))
                }
                _ => None,
            })
            .collect();
        if !pokes.iter().any(|(x, y)| x.is_finite() && y.is_finite()) {
            return None;
        }
        let (cx, cy) = selected_c(t, self.seed, &pokes);
        Some(format!(
            "C MORPHED TO {cx:+.3} {cy:+.3}I   WHOLE FRACTAL UPDATED"
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let morphs = bounded_morph_points(pokes, width, height);
        if morphs.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (cx, cy) = selected_c(t, self.seed, pokes);
        render_with_c(canvas, width, height, cx, cy);
        if let Some(morph) = morphs.last() {
            let marker = (width.min(height) / 70).clamp(3, 10) as i32;
            canvas.line(morph.x - marker, morph.y, morph.x + marker, morph.y, '#');
            canvas.line(morph.x, morph.y - marker, morph.x, morph.y + marker, '#');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Julia, MorphPoint, bounded_morph_points, escape_iters, finite_phase};
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
    use crate::surface::{MAX_DIM, Surface};

    #[test]
    fn status_reports_only_a_finite_selected_constant() {
        let room = Julia::new();
        assert_eq!(room.status_input(0.3, &[]), None);
        let ignored = [
            RoomInput::PointerUp {
                x: 0.2,
                y: 0.8,
                t: 0.0,
            },
            RoomInput::PointerMove {
                x: f64::NAN,
                y: 0.4,
                t: 0.1,
            },
        ];
        assert_eq!(room.status_input(0.3, &ignored), None);
        let selected = [
            ignored[1],
            RoomInput::PointerDown {
                x: 0.75,
                y: 0.25,
                t: 0.2,
            },
        ];
        let status = room.status_input(0.3, &selected).expect("finite selection");
        assert!(status.starts_with("C MORPHED TO "));
        assert!(status.ends_with("I   WHOLE FRACTAL UPDATED"));
    }

    #[test]
    fn origin_survives_for_a_small_c() {
        // With c near zero the origin is a fixed point and never escapes.
        assert_eq!(escape_iters(0.0, 0.0, 0.0, 0.0, 160), 160);
    }

    #[test]
    fn far_start_escapes_quickly() {
        assert!(escape_iters(3.0, 3.0, -0.4, 0.6, 160) < 5);
    }

    #[test]
    fn c_walks_a_circle() {
        let room = Julia::new();
        let (x0, y0) = room.c_for(0.0);
        let (x1, y1) = room.c_for(0.25);
        assert!((x0 - 0.7885).abs() < 1e-9 && y0.abs() < 1e-9);
        assert!(x1.abs() < 1e-9 && (y1 - 0.7885).abs() < 1e-9);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Julia::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.3);
        room.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 10);
    }

    #[test]
    fn phase_zero_has_a_readable_iteration_band() {
        let room = Julia::new();
        let mut raster = Raster::with_accent(256, 192, [255, 120, 60]);
        room.render(&mut raster, 0.0);
        let readable = raster
            .to_rgba()
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 80)
            .count();

        assert!(
            readable > 500,
            "the first Julia frame must not consist only of structural gray"
        );
    }

    #[test]
    fn seed_zero_preserves_default_and_nonzero_seed_varies() {
        let default = Julia::new();
        let seed_zero = Julia::new_with(0);
        let varied = Julia::new_with(7);

        assert_eq!(
            render_text(&default, 0.3, &[]),
            render_text(&seed_zero, 0.3, &[])
        );
        assert_eq!(default.c_for(0.3), seed_zero.c_for(0.3));
        assert_ne!(default.c_for(0.3), varied.c_for(0.3));
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Julia::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    fn render_text(room: &Julia, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 32);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        let room = Julia::new();
        assert_eq!(finite_phase(f64::NAN), 0.0);
        assert_eq!(finite_phase(f64::INFINITY), 0.0);

        let mut first = Canvas::new(36, 24);
        let mut nan = Canvas::new(36, 24);
        room.render(&mut first, 0.0);
        room.render(&mut nan, f64::NAN);
        assert_eq!(nan.to_text(), first.to_text());
        assert_eq!(
            render_text(&room, f64::NEG_INFINITY, &[(0.65, 0.35)]),
            render_text(&room, 0.0, &[(0.65, 0.35)])
        );
    }

    #[test]
    fn morph_points_use_the_newest_bounded_raw_tail() {
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    i as f64 / (MAX_ROOM_POKES - 1) as f64,
                    if i % 2 == 0 { 0.2 } else { 0.8 },
                )
            })
            .collect();
        let mut all = vec![(0.2, 0.8); MAX_ROOM_POKES + 5];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            bounded_morph_points(&all, 48, 32),
            bounded_morph_points(&newest, 48, 32)
        );
        assert_ne!(
            bounded_morph_points(&all, 48, 32),
            bounded_morph_points(&discarded_prefix, 48, 32)
        );
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = Julia::new();
        let newest = vec![(0.85, 0.2); MAX_ROOM_POKES];
        let mut all = vec![(0.15, 0.8); MAX_ROOM_POKES + 5];
        all.extend(newest.iter().copied());
        let prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            render_text(&room, 0.4, &all),
            render_text(&room, 0.4, &newest)
        );
        assert_ne!(
            render_text(&room, 0.4, &all),
            render_text(&room, 0.4, &prefix)
        );
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = Julia::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(bounded_morph_points(&with_invalid_tail, 48, 32).is_empty());
        assert_eq!(
            render_text(&room, 0.4, &with_invalid_tail),
            render_text(&room, 0.4, &[])
        );
    }

    #[test]
    fn render_poked_marks_the_morph_center_for_raster_exports() {
        let room = Julia::new();
        let mut plain = Raster::new(256, 256);
        let mut poked = Raster::new(256, 256);
        room.render(&mut plain, 0.35);
        room.render_poked(&mut poked, 0.35, &[(0.9, 0.1)]);

        assert_ne!(plain.to_rgba(), poked.to_rgba());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_morph_identity() {
        let room = Julia::new();
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            bounded_morph_points(&with_bad_points, 48, 32),
            bounded_morph_points(&finite, 48, 32)
        );
        assert_eq!(
            render_text(&room, 0.35, &with_bad_points),
            render_text(&room, 0.35, &finite)
        );
    }

    #[test]
    fn finite_morph_points_clamp_to_visible_edges() {
        assert_eq!(
            bounded_morph_points(&[(1.5, -1.0)], 10, 8),
            vec![MorphPoint {
                x: 9,
                y: 0,
                nx: 1.0,
                ny: 0.0,
            }]
        );
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_regions() {
        #[derive(Default)]
        struct HugeSurface {
            width: usize,
            height: usize,
            plots: usize,
            max_abs_coord: i32,
        }

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = Julia::new();
        for (width, height) in [(usize::MAX, 8), (8, usize::MAX)] {
            let mut surface = HugeSurface {
                width,
                height,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, f64::INFINITY, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_DIM * 16);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn reveal_mentions_infinity() {
        assert!(Julia::new().reveal().contains("infinity"));
    }

    #[test]
    fn verb_and_poked_no_panic() {
        let room = Julia::new();
        assert!(room.verb().is_some());
        let mut c = Canvas::new(20, 15);
        room.render_poked(&mut c, 0.3, &[(0.5, 0.5)]);
    }
}
