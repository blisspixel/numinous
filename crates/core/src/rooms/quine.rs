//! Quine: a pattern that contains a copy of itself.
//!
//! Draw a circle of points; inside it, draw a smaller rotated copy of the same
//! circle. The copy contains a yet smaller copy, and so on. A finite rule
//! produces an infinite regress that still "describes" itself at every scale.
//! Poke to move the inner copy; t adds recursion depth. For a digital mind,
//! this is the visual of self-reference and the strange loop at the heart of
//! "I". See docs/INSIGHTS.md and DIGITAL_MINDS.md.

use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

#[derive(Debug, Clone, Copy, PartialEq)]
struct CopyPoint {
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

fn safe_aspect(canvas: &dyn Surface) -> f64 {
    let aspect = canvas.char_aspect();
    if aspect.is_finite() && aspect > 0.0 {
        aspect.clamp(0.05, 1.0)
    } else {
        1.0
    }
}

fn screen_coord(norm: f64, extent: usize) -> i32 {
    debug_assert!(extent > 0);
    (norm.clamp(0.0, 1.0) * extent.saturating_sub(1) as f64).round() as i32
}

fn bounded_copy_points(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<CopyPoint> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(x, y)| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let nx = x.clamp(0.0, 1.0);
            let ny = y.clamp(0.0, 1.0);
            Some(CopyPoint {
                x: screen_coord(nx, width),
                y: screen_coord(ny, height),
                nx,
                ny,
            })
        })
        .collect()
}

/// The Quine room (self-containing pattern).
#[derive(Debug, Default)]
pub struct Quine {
    seed: u64,
}

impl Quine {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn points_for(t: f64) -> usize {
        60 + (finite_phase(t) * 120.0) as usize
    }

    fn depth_for(t: f64) -> usize {
        1 + (finite_phase(t) * 4.0) as usize
    }

    fn seed_rotation(&self) -> f64 {
        if self.seed == 0 {
            0.0
        } else {
            ((self.seed % 23) + 1) as f64 * 0.05
        }
    }
}

impl Room for Quine {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "quine",
            title: "The Quine",
            wing: "Mind & Computation",
            blurb: "A circle that draws a smaller copy of itself inside; the copy draws a smaller copy, forever. A finite rule that contains its own description at every scale.",
            accent: [200, 150, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let n = Self::points_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let cx = fw / 2.0;
        let cy = fh / 2.0;
        let r = fw.min(fh) / 2.5;
        let aspect = safe_aspect(canvas);
        // draw recursive copies, depth from t
        let depth = Self::depth_for(t);
        self.draw_copy(canvas, cx, cy, r, self.seed_rotation(), n, depth, aspect);
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let n = Self::points_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let cx = fw / 2.0;
        let cy = fh / 2.0;
        let r = fw.min(fh) / 2.5;
        let aspect = safe_aspect(canvas);
        let depth = Self::depth_for(t);
        let points = bounded_copy_points(pokes, width, height);
        if points.is_empty() {
            self.render(canvas, t);
            return;
        }
        // base
        self.draw_copy(canvas, cx, cy, r, self.seed_rotation(), n, depth, aspect);
        // Poked copies are centered on the bounded clicked cell.
        for point in points {
            let rot = (point.nx + point.ny) * 1.5 + self.seed_rotation();
            self.draw_copy(
                canvas,
                point.x as f64,
                point.y as f64,
                r * 0.35,
                rot,
                n,
                depth.saturating_sub(1).max(1),
                aspect,
            );
            canvas.plot(point.x, point.y, '*');
        }
    }

    fn reveal(&self) -> &'static str {
        "This pattern draws a smaller copy of the exact same pattern inside itself. The copy draws a smaller copy. A finite description that contains its own full description, at every scale. This is how a mind can refer to itself."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "A self-printing canon",
            root: 220.0,
            tempo: 90,
            line: &[0, 4, 7, 0, 12, 7, 4, 0],
            encodes: "a phrase that returns as its own description",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLACE A COPY")
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }
}

impl Quine {
    #[allow(clippy::too_many_arguments)]
    fn draw_copy(
        &self,
        canvas: &mut dyn Surface,
        cx: f64,
        cy: f64,
        r: f64,
        rot: f64,
        n: usize,
        depth: usize,
        aspect: f64,
    ) {
        if depth == 0 || r < 2.0 {
            return;
        }
        for i in 0..n {
            let a = (i as f64 / n as f64) * std::f64::consts::TAU + rot;
            let x = cx + r * a.cos();
            let y = cy + r * a.sin() * aspect;
            canvas.plot(x as i32, y as i32, '*');
        }
        // recurse smaller copy inside
        let sub_r = r * 0.45;
        let sub_cx = cx + r * 0.1 * (rot.cos());
        let sub_cy = cy + r * 0.1 * (rot.sin()) * aspect;
        let sub_rot = rot + 0.7 + self.seed_rotation();
        self.draw_copy(canvas, sub_cx, sub_cy, sub_r, sub_rot, n, depth - 1, aspect);
    }
}

#[cfg(test)]
mod tests {
    use super::{CopyPoint, Quine, bounded_copy_points, finite_phase};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room};
    use crate::surface::{MAX_DIM, Surface};
    use std::collections::BTreeSet;

    fn render_text(room: &Quine, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 32);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    #[test]
    fn render_is_deterministic() {
        let room = Quine::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = Quine::new();
        let mut canvas = Canvas::new(40, 30);
        room.render(&mut canvas, 0.5);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn new_with_zero_matches_default() {
        let r0 = Quine::new_with(0);
        let r_def = Quine::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn new_with_nonzero_changes_render() {
        let r0 = Quine::new_with(0);
        let r1 = Quine::new_with(1);
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        r0.render(&mut a, 0.6);
        r1.render(&mut b, 0.6);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn verb_and_poked() {
        let room = Quine::new();
        assert!(room.verb().is_some());
        let mut c = Canvas::new(40, 30);
        room.render_poked(&mut c, 0.5, &[(0.5, 0.5)]);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        let room = Quine::new();
        assert_eq!(finite_phase(f64::NAN), 0.0);
        assert_eq!(finite_phase(f64::INFINITY), 0.0);

        let mut first = Canvas::new(36, 24);
        let mut nan = Canvas::new(36, 24);
        room.render(&mut first, 0.0);
        room.render(&mut nan, f64::NAN);
        assert_eq!(nan.to_text(), first.to_text());
        assert_eq!(
            render_text(&room, f64::NEG_INFINITY, &[(0.7, 0.25)]),
            render_text(&room, 0.0, &[(0.7, 0.25)])
        );
    }

    #[test]
    fn copy_points_use_the_newest_bounded_raw_tail() {
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    i as f64 / (MAX_ROOM_POKES - 1) as f64,
                    if i % 2 == 0 { 0.25 } else { 0.75 },
                )
            })
            .collect();
        let mut all = vec![(0.2, 0.8); MAX_ROOM_POKES + 3];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            bounded_copy_points(&all, 48, 32),
            bounded_copy_points(&newest, 48, 32)
        );
        assert_ne!(
            bounded_copy_points(&all, 48, 32),
            bounded_copy_points(&discarded_prefix, 48, 32)
        );
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = Quine::new();
        let newest = vec![(0.85, 0.2); MAX_ROOM_POKES];
        let mut all = vec![(0.15, 0.8); MAX_ROOM_POKES + 3];
        all.extend(newest.iter().copied());
        let prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            render_text(&room, 0.45, &all),
            render_text(&room, 0.45, &newest)
        );
        assert_ne!(
            render_text(&room, 0.45, &all),
            render_text(&room, 0.45, &prefix)
        );
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = Quine::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 3]);

        assert!(bounded_copy_points(&with_invalid_tail, 48, 32).is_empty());
        assert_eq!(
            render_text(&room, 0.45, &with_invalid_tail),
            render_text(&room, 0.45, &[])
        );
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_copy_identity() {
        let room = Quine::new();
        let finite = vec![(0.3, 0.7)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.3, 0.7), (0.2, f64::INFINITY)];

        assert_eq!(
            bounded_copy_points(&with_bad_points, 48, 32),
            bounded_copy_points(&finite, 48, 32)
        );
        assert_eq!(
            render_text(&room, 0.45, &with_bad_points),
            render_text(&room, 0.45, &finite)
        );
    }

    #[test]
    fn finite_copy_points_clamp_to_visible_edges() {
        assert_eq!(
            bounded_copy_points(&[(1.5, -1.0), (-1.0, 1.5), (-1.0, -1.0), (1.5, 1.5)], 10, 8),
            vec![
                CopyPoint {
                    x: 9,
                    y: 0,
                    nx: 1.0,
                    ny: 0.0,
                },
                CopyPoint {
                    x: 0,
                    y: 7,
                    nx: 0.0,
                    ny: 1.0,
                },
                CopyPoint {
                    x: 0,
                    y: 0,
                    nx: 0.0,
                    ny: 0.0,
                },
                CopyPoint {
                    x: 9,
                    y: 7,
                    nx: 1.0,
                    ny: 1.0,
                },
            ]
        );
    }

    #[test]
    fn render_poked_places_first_frame_copy_geometry_beyond_marker() {
        #[derive(Default)]
        struct MarkSurface {
            marks: BTreeSet<(i32, i32)>,
        }

        impl Surface for MarkSurface {
            fn width(&self) -> usize {
                80
            }

            fn height(&self) -> usize {
                48
            }

            fn char_aspect(&self) -> f64 {
                1.0
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.marks.insert((x, y));
            }
        }

        let room = Quine::new();
        let poke = (0.85, 0.15);
        let marker = {
            let points = bounded_copy_points(&[poke], 80, 48);
            (points[0].x, points[0].y)
        };
        let mut base = MarkSurface::default();
        let mut poked = MarkSurface::default();
        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &[poke]);

        let extra_geometry: Vec<_> = poked
            .marks
            .difference(&base.marks)
            .copied()
            .filter(|point| *point != marker)
            .collect();

        assert!(
            !extra_geometry.is_empty(),
            "the first-frame copy must draw recursive geometry, not only the marker"
        );
        assert!(extra_geometry.iter().any(|(x, _)| *x < marker.0));
        assert!(extra_geometry.iter().any(|(x, _)| *x > marker.0));
        assert!(extra_geometry.iter().any(|(_, y)| *y < marker.1));
        assert!(extra_geometry.iter().any(|(_, y)| *y > marker.1));
    }

    #[test]
    fn render_poked_marks_all_clamped_copy_centers() {
        let room = Quine::new();
        let mut canvas = Canvas::new(10, 8);
        room.render_poked(
            &mut canvas,
            0.0,
            &[(1.5, -1.0), (-1.0, 1.5), (-1.0, -1.0), (1.5, 1.5)],
        );
        let rows: Vec<Vec<char>> = canvas
            .to_text()
            .lines()
            .map(|line| line.chars().collect())
            .collect();

        assert_eq!(rows[0][0], '*');
        assert_eq!(rows[0][9], '*');
        assert_eq!(rows[7][0], '*');
        assert_eq!(rows[7][9], '*');
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_regions() {
        #[derive(Default)]
        struct HugeSurface {
            width: usize,
            height: usize,
            aspect: f64,
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

            fn char_aspect(&self) -> f64 {
                self.aspect
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = Quine::new();
        for aspect in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            1.0e12,
        ] {
            let mut surface = HugeSurface {
                width: usize::MAX,
                height: usize::MAX,
                aspect,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, 1.0e12, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_ROOM_POKES * 1200);
            assert!(surface.max_abs_coord <= MAX_DIM as i32);
        }
    }
}
