//! The Mandelbrot set: infinite complexity from one line of arithmetic.
//!
//! For each point `c` in the complex plane, iterate `z -> z*z + c` from zero and
//! ask whether it stays bounded. The points that do form the set; the points that
//! escape, shaded by how fast, form its infinitely detailed halo. `t` zooms from
//! the whole set toward the seahorse valley. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

/// Escape-iteration budget (also the "in the set" sentinel).
const MAX_ITER: u32 = 160;
const DIVE_RADIUS: i32 = 5;
const DIVE_STEP: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
struct DivePoint {
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

fn bounded_dive_points(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<DivePoint> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(x, y)| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let nx = x.clamp(0.0, 1.0);
            let ny = y.clamp(0.0, 1.0);
            Some(DivePoint {
                x: screen_coord(nx, width),
                y: screen_coord(ny, height),
                nx,
                ny,
            })
        })
        .collect()
}

/// The Mandelbrot room.
#[derive(Debug, Default)]
pub struct Mandelbrot {
    seed: u64,
}

impl Mandelbrot {
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

/// How many iterations `z -> z*z + c` survives before escaping `|z| > 2`.
fn escape_iters(cx: f64, cy: f64, max: u32) -> u32 {
    let (mut zx, mut zy) = (0.0, 0.0);
    let mut i = 0;
    while i < max && zx * zx + zy * zy <= 4.0 {
        let next_x = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = next_x;
        i += 1;
    }
    i
}

/// Linear interpolation from `a` to `b` by `t`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

impl Room for Mandelbrot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mandelbrot",
            title: "Mandelbrot Set",
            wing: "Fractals & the Infinite",
            blurb: "Iterate z into z squared plus c and ask if it stays bounded. The points that \
                    do form the most complex object in mathematics. t zooms toward the seahorses.",
            accent: [70, 130, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let t = finite_phase(t);
        // Zoom from the whole set toward the seahorse valley at -0.745 + 0.113i.
        let zoom = 1.5 * 0.15_f64.powf(t);
        let s_off = (self.seed % 1000) as f64 * 0.00001;
        let center_x = lerp(-0.5, -0.745, t) + s_off;
        let center_y = lerp(0.0, 0.113, t) + s_off;
        let scale = 2.0 * zoom / width as f64;
        let half_w = width as f64 / 2.0;
        let half_h = height as f64 / 2.0;

        for py in 0..height {
            for px in 0..width {
                let cx = center_x + (px as f64 - half_w) * scale;
                let cy = center_y + (py as f64 - half_h) * scale;
                let iters = escape_iters(cx, cy, MAX_ITER);
                let mark = if iters == MAX_ITER {
                    '#'
                } else if iters > 24 {
                    '*'
                } else if iters > 6 {
                    '-'
                } else {
                    continue;
                };
                canvas.plot(px as i32, py as i32, mark);
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "You can zoom into this shape forever and keep finding new detail, all from \
         squaring a number and adding a constant. Its main body has the cardioid \
         shape wrapped by Times Tables at 2; along its real slice, the quadratic \
         family is the Logistic Map in a stretched and shifted orbit coordinate."
    }

    #[allow(dead_code)]
    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C deep boundary",
            root: 65.41,
            tempo: 80,
            line: &[0, 12, 7, 3, 8, 5, 1, 0],
            encodes: "escape-time falling back toward an infinite edge",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DIVE AT POINT")
    }

    #[allow(dead_code)]
    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        // base
        self.render(canvas, t);
        let dives = bounded_dive_points(pokes, width, height);
        // for each poke, draw a small "dive" at that normalized pos with extra zoom
        for dive in dives {
            let dive_t = (finite_phase(t) + 0.2).clamp(0.0, 1.0);
            let dive_zoom = 1.5 * 0.15_f64.powf(dive_t);
            let dive_cx = lerp(-0.5, -0.745, dive_t) + (dive.nx - 0.5) * 0.2;
            let dive_cy = lerp(0.0, 0.113, dive_t) + (dive.ny - 0.5) * 0.2;
            let scale = 2.0 * dive_zoom / width as f64;
            let half_w = width as f64 / 2.0;
            let half_h = height as f64 / 2.0;
            // draw small region around poke, scaled
            for dy in -DIVE_RADIUS..=DIVE_RADIUS {
                for dx in -DIVE_RADIUS..=DIVE_RADIUS {
                    let lx = dive.x + dx * DIVE_STEP;
                    let ly = dive.y + dy * DIVE_STEP;
                    if lx < 0 || lx >= width as i32 || ly < 0 || ly >= height as i32 {
                        continue;
                    }
                    let cx = dive_cx + (f64::from(lx) - half_w) * scale * 0.5;
                    let cy = dive_cy + (f64::from(ly) - half_h) * scale * 0.5;
                    let iters = escape_iters(cx, cy, MAX_ITER);
                    let mark = if iters == MAX_ITER {
                        '#'
                    } else if iters > 24 {
                        '*'
                    } else if iters > 6 {
                        '-'
                    } else {
                        continue;
                    };
                    canvas.plot(lx, ly, mark);
                }
            }
        }
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Nobody knows the exact area of this set. It is about 1.5065918849, \
             measured by throwing billions of points at it, and there is no known \
             closed form. One of the most famous objects in mathematics, and we \
             cannot tell you how big it is.",
            "Shishikura proved in 1991 that the boundary you are zooming along has \
             Hausdorff dimension exactly 2: a curve so wrinkled it is, in the fractal \
             sense, as thick as the plane it lives in.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{DivePoint, Mandelbrot, bounded_dive_points, escape_iters, finite_phase};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room};
    use crate::surface::{MAX_DIM, Surface};

    #[test]
    fn origin_is_in_the_set() {
        assert_eq!(escape_iters(0.0, 0.0, 160), 160);
    }

    #[test]
    fn far_points_escape_quickly() {
        assert!(escape_iters(2.0, 2.0, 160) < 6);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Mandelbrot::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    fn render_text(room: &Mandelbrot, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 32);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        let room = Mandelbrot::new();
        assert_eq!(finite_phase(f64::NAN), 0.0);
        assert_eq!(finite_phase(f64::INFINITY), 0.0);

        let mut first = Canvas::new(36, 24);
        let mut nan = Canvas::new(36, 24);
        room.render(&mut first, 0.0);
        room.render(&mut nan, f64::NAN);
        assert_eq!(nan.to_text(), first.to_text());
    }

    #[test]
    fn dive_points_use_the_newest_bounded_raw_tail() {
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    i as f64 / (MAX_ROOM_POKES - 1) as f64,
                    if i % 2 == 0 { 0.15 } else { 0.85 },
                )
            })
            .collect();
        let mut all = vec![(0.2, 0.8); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            bounded_dive_points(&all, 48, 32),
            bounded_dive_points(&newest, 48, 32)
        );
        assert_ne!(
            bounded_dive_points(&all, 48, 32),
            bounded_dive_points(&discarded_prefix, 48, 32)
        );
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = Mandelbrot::new();
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
        let room = Mandelbrot::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(bounded_dive_points(&with_invalid_tail, 48, 32).is_empty());
        assert_eq!(
            render_text(&room, 0.4, &with_invalid_tail),
            render_text(&room, 0.4, &[])
        );
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_dive_identity() {
        let room = Mandelbrot::new();
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            bounded_dive_points(&with_bad_points, 48, 32),
            bounded_dive_points(&finite, 48, 32)
        );
        assert_eq!(
            render_text(&room, 0.35, &with_bad_points),
            render_text(&room, 0.35, &finite)
        );
    }

    #[test]
    fn finite_dive_points_clamp_to_visible_edges() {
        assert_eq!(
            bounded_dive_points(&[(1.5, -1.0)], 10, 8),
            vec![DivePoint {
                x: 9,
                y: 0,
                nx: 1.0,
                ny: 0.0,
            }]
        );
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Mandelbrot::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
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

        let room = Mandelbrot::new();
        for (width, height) in [(usize::MAX, 8), (8, usize::MAX)] {
            let mut surface = HugeSurface {
                width,
                height,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, 0.8, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_DIM * 16);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn reveal_names_both_cross_room_connections() {
        let reveal = Mandelbrot::new().reveal();
        assert!(reveal.contains("forever"));
        assert!(reveal.contains("Times Tables"));
        assert!(reveal.contains("Logistic Map"));
        assert!(reveal.contains("orbit coordinate"));
    }

    #[test]
    fn verb_and_poked_no_panic() {
        let room = Mandelbrot::new();
        assert!(room.verb().is_some());
        let mut c = Canvas::new(20, 15);
        room.render_poked(&mut c, 0.5, &[(0.5, 0.5)]);
        // just no panic, ink may vary
    }
}
