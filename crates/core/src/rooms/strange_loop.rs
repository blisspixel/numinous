//! Strange Loop: the visual of self-reference.
//!
//! Draw a "U" shape; inside the bottom, draw a smaller transformed copy of the U.
//! The copy contains a smaller copy, creating a loop that refers to itself.
//! This is Hofstadter's strange loop made drawable: levels cross and return.
//! t increases recursion depth; poke moves the inner loop. For digital minds,
//! this is the shape of consciousness emerging from recursion. See
//! docs/DIGITAL_MINDS.md and INSIGHTS.md.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

#[derive(Debug, Clone, Copy, PartialEq)]
struct LoopPoint {
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

fn bounded_loop_points(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<LoopPoint> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(x, y)| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let nx = x.clamp(0.0, 1.0);
            let ny = y.clamp(0.0, 1.0);
            Some(LoopPoint {
                x: screen_coord(nx, width),
                y: screen_coord(ny, height),
                nx,
                ny,
            })
        })
        .collect()
}

fn u_point(cx: f64, cy: f64, r: f64, rot: f64, aspect: f64, phase: f64) -> (i32, i32) {
    let horizontal = phase * 2.0 - 1.0;
    let curve = 1.0 - horizontal * horizontal;
    let x = cx + r * 0.8 * horizontal + rot.sin() * r * 0.05 * curve;
    let y = cy + r * (0.1 - 0.9 * horizontal * horizontal) * aspect;
    (x.round() as i32, y.round() as i32)
}

/// The Strange Loop room.
#[derive(Debug, Default)]
pub struct StrangeLoop {
    seed: u64,
}

impl StrangeLoop {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn depth_for(t: f64) -> usize {
        2 + (finite_phase(t) * 5.0) as usize
    }
}

impl Room for StrangeLoop {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "strange-loop",
            title: "Strange Loop",
            wing: "Mind & Computation",
            blurb: "A U that contains a smaller U that contains a smaller U... A finite rule that loops back to itself across levels. This is how 'I' might emerge from symbols referring to symbols.",
            accent: [180, 100, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let phase = finite_phase(t);
        let depth = Self::depth_for(t);
        let (fw, fh) = (width as f64, height as f64);
        // The sweep turns the loop and zooms slowly into it, so the room about
        // self-reference actually moves and more of its nesting surfaces as you
        // descend, rather than sitting frozen at one frame.
        let r = fw.min(fh) * 0.45 * (1.0 + phase * 0.25);
        // The seed shifts the whole loop sideways, a phase-independent variation
        // that stays visible even where the sweep's rotation flattens the arm
        // shear. Seed 0 (the default and the postcard) stays centered.
        let cx = fw / 2.0 + (self.seed % 5) as f64 * r * 0.22;
        let cy = fh / 2.0;
        let aspect = safe_aspect(canvas);
        let rot = (self.seed % 100) as f64 * 0.01 + phase * std::f64::consts::TAU;
        self.draw_loop(canvas, cx, cy, r, rot, depth, aspect);
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let phase = finite_phase(t);
        let depth = Self::depth_for(t);
        let fw = width as f64;
        let fh = height as f64;
        let r = fw.min(fh) * 0.45 * (1.0 + phase * 0.25);
        let cx = fw / 2.0 + (self.seed % 5) as f64 * r * 0.22;
        let cy = fh / 2.0;
        let aspect = safe_aspect(canvas);
        let base_rot = (self.seed % 100) as f64 * 0.01 + phase * std::f64::consts::TAU;
        let points = bounded_loop_points(pokes, width, height);
        let Some(control) = points.last().copied() else {
            self.render(canvas, t);
            return;
        };
        let inner_ox = (control.nx - 0.5) * r * 0.9;
        let inner_oy = (control.ny - 0.5) * r * 0.7 * aspect;
        self.draw_loop_with_inner_offset(
            canvas, cx, cy, r, base_rot, depth, aspect, inner_ox, inner_oy,
        );
        for point in points {
            let marker = (width.min(height) / 70).clamp(3, 10) as i32;
            canvas.line(point.x - marker, point.y, point.x + marker, point.y, '#');
            canvas.line(point.x, point.y - marker, point.x, point.y + marker, '#');
        }
    }

    fn status_input(&self, _t: f64, inputs: &[RoomInput]) -> Option<String> {
        let points: Vec<_> = inputs
            .iter()
            .filter_map(|input| match *input {
                RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. }
                    if x.is_finite() && y.is_finite() =>
                {
                    Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
                }
                _ => None,
            })
            .collect();
        let start = points.len().saturating_sub(MAX_ROOM_POKES);
        let points = &points[start..];
        let &(x, y) = points.last()?;
        Some(format!(
            "INNER LOOP ANCHORED AT {:.0}% {:.0}%   BRIGHT CROSS MARKS YOUR HAND",
            x * 100.0,
            y * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "This shape draws a smaller version of itself inside a 'U'. The smaller one does the same. Moving up or down levels returns you to the same pattern. This crossing of levels is a strange loop, the proposed root of self and mind."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "G# mirrored loop",
            root: 207.65,
            tempo: 88,
            line: &[0, 4, 7, 12, 7, 4, 0, -5, 0],
            encodes: "a phrase climbing levels and returning to itself",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: SHIFT THE INNER LOOP")
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }
}

impl StrangeLoop {
    fn stroke(canvas: &mut dyn Surface, from: (i32, i32), to: (i32, i32), mark: char) {
        if canvas.width() > MAX_DIM || canvas.height() > MAX_DIM {
            canvas.plot(to.0, to.1, mark);
            return;
        }
        canvas.line(from.0, from.1, to.0, to.1, mark);
        if canvas.width() >= 160 && canvas.height() >= 120 {
            canvas.line(from.0, from.1 - 1, to.0, to.1 - 1, mark);
            canvas.line(from.0, from.1 + 1, to.0, to.1 + 1, mark);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_loop(
        &self,
        canvas: &mut dyn Surface,
        cx: f64,
        cy: f64,
        r: f64,
        rot: f64,
        depth: usize,
        aspect: f64,
    ) {
        self.draw_loop_with_inner_offset(canvas, cx, cy, r, rot, depth, aspect, 0.0, 0.0);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_loop_with_inner_offset(
        &self,
        canvas: &mut dyn Surface,
        cx: f64,
        cy: f64,
        r: f64,
        rot: f64,
        depth: usize,
        aspect: f64,
        inner_ox: f64,
        inner_oy: f64,
    ) {
        if depth == 0 || r < 3.0 {
            return;
        }
        // One continuous parabola makes the recursive unit an actual U. Each
        // nested copy remains a separate object, but no level is assembled from
        // disconnected arms and a floating crossbar.
        let steps = 48;
        let mut previous = None;
        for i in 0..=steps {
            let f = i as f64 / steps as f64;
            let next = u_point(cx, cy, r, rot, aspect, f);
            if let Some(from) = previous {
                Self::stroke(canvas, from, next, '#');
            }
            previous = Some(next);
        }
        // recurse inner
        let sub_r = r * 0.4;
        let sub_cx = cx + inner_ox;
        let sub_cy = cy - r * 0.18 * aspect + inner_oy;
        self.draw_loop(canvas, sub_cx, sub_cy, sub_r, rot + 1.0, depth - 1, aspect);
    }
}

#[cfg(test)]
mod tests {
    use super::{LoopPoint, StrangeLoop, bounded_loop_points, finite_phase};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
    use crate::surface::{MAX_DIM, Surface};

    #[test]
    fn render_is_deterministic() {
        let room = StrangeLoop::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = StrangeLoop::new();
        let mut canvas = Canvas::new(40, 30);
        room.render(&mut canvas, 0.5);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn one_level_is_one_connected_u() {
        use std::collections::{BTreeSet, VecDeque};

        #[derive(Default)]
        struct MarkSetSurface {
            marks: BTreeSet<(i32, i32)>,
        }

        impl Surface for MarkSetSurface {
            fn width(&self) -> usize {
                120
            }

            fn height(&self) -> usize {
                90
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.marks.insert((x, y));
            }
        }

        let room = StrangeLoop::new();
        let mut surface = MarkSetSurface::default();
        room.draw_loop(&mut surface, 60.0, 45.0, 35.0, 0.0, 1, 1.0);
        let mut unseen = surface.marks;
        let mut components = 0;

        while let Some(start) = unseen.pop_first() {
            components += 1;
            let mut queue = VecDeque::from([start]);
            while let Some((x, y)) = queue.pop_front() {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let neighbor = (x + dx, y + dy);
                        if unseen.remove(&neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        assert_eq!(components, 1);
    }

    #[test]
    fn the_loop_actually_moves_across_the_sweep() {
        // A playtester caught this room rendering identically at every phase:
        // the one room about self-reference sat frozen. The sweep must animate.
        let room = StrangeLoop::new();
        let frame = |t: f64| {
            let mut canvas = Canvas::new(48, 32);
            room.render(&mut canvas, t);
            canvas.to_text()
        };
        assert_ne!(
            frame(0.0),
            frame(0.5),
            "the loop must move from t=0 to t=0.5"
        );
        assert_ne!(frame(0.5), frame(0.9), "and keep moving to t=0.9");
    }

    fn render_text(room: &StrangeLoop, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 32);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        let room = StrangeLoop::new();
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
    fn loop_points_use_the_newest_bounded_raw_tail() {
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
            bounded_loop_points(&all, 48, 32),
            bounded_loop_points(&newest, 48, 32)
        );
        assert_ne!(
            bounded_loop_points(&all, 48, 32),
            bounded_loop_points(&discarded_prefix, 48, 32)
        );
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = StrangeLoop::new();
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
        let room = StrangeLoop::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 3]);

        assert!(bounded_loop_points(&with_invalid_tail, 48, 32).is_empty());
        assert_eq!(
            render_text(&room, 0.45, &with_invalid_tail),
            render_text(&room, 0.45, &[])
        );
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_loop_identity() {
        let room = StrangeLoop::new();
        let finite = vec![(0.3, 0.7)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.3, 0.7), (0.2, f64::INFINITY)];

        assert_eq!(
            bounded_loop_points(&with_bad_points, 48, 32),
            bounded_loop_points(&finite, 48, 32)
        );
        assert_eq!(
            render_text(&room, 0.45, &with_bad_points),
            render_text(&room, 0.45, &finite)
        );
    }

    #[test]
    fn finite_loop_points_clamp_to_visible_edges() {
        assert_eq!(
            bounded_loop_points(&[(1.5, -1.0)], 10, 8),
            vec![LoopPoint {
                x: 9,
                y: 0,
                nx: 1.0,
                ny: 0.0,
            }]
        );
    }

    #[test]
    fn new_with_zero_matches_default() {
        let r0 = StrangeLoop::new_with(0);
        let r_def = StrangeLoop::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn verb_and_poked_changes() {
        let room = StrangeLoop::new();
        assert!(room.verb().is_some());
        let mut cp = Canvas::new(30, 20);
        let mut c0 = Canvas::new(30, 20);
        room.render_poked(&mut cp, 0.5, &[(0.3, 0.4)]);
        room.render(&mut c0, 0.5);
        assert!(
            cp.ink_count() != c0.ink_count() || cp.to_text() != c0.to_text(),
            "poke should change output"
        );
    }

    #[test]
    fn interaction_status_names_the_visible_anchor() {
        let room = StrangeLoop::new();
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.8,
                t: 0.1,
            },
            RoomInput::PointerMove {
                x: 0.75,
                y: 0.25,
                t: 0.2,
            },
        ];

        assert_eq!(
            room.status_input(0.5, &inputs).as_deref(),
            Some("INNER LOOP ANCHORED AT 75% 25%   BRIGHT CROSS MARKS YOUR HAND")
        );
        assert_eq!(room.status_input(0.5, &[]), None);
    }

    #[test]
    fn render_poked_moves_inner_loop_instead_of_adding_echo() {
        #[derive(Default)]
        struct CountingSurface {
            width: usize,
            height: usize,
            plots: usize,
        }

        impl Surface for CountingSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn char_aspect(&self) -> f64 {
                1.0
            }

            fn plot(&mut self, _x: i32, _y: i32, _mark: char) {
                self.plots += 1;
            }
        }

        let room = StrangeLoop::new();
        let mut base = CountingSurface {
            width: 60,
            height: 40,
            ..CountingSurface::default()
        };
        let mut poked = CountingSurface {
            width: 60,
            height: 40,
            ..CountingSurface::default()
        };

        room.render(&mut base, 0.5);
        room.render_poked(&mut poked, 0.5, &[(0.8, 0.2)]);

        assert!(
            poked.plots <= base.plots + 20,
            "moving one nested loop must not draw a second echo tree"
        );
    }

    #[test]
    fn render_poked_moves_geometry_beyond_the_hand_marker() {
        use std::collections::BTreeSet;

        #[derive(Default)]
        struct MarkSetSurface {
            width: usize,
            height: usize,
            marks: BTreeSet<(i32, i32)>,
        }

        impl Surface for MarkSetSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn char_aspect(&self) -> f64 {
                1.0
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.marks.insert((x, y));
            }
        }

        let room = StrangeLoop::new();
        let poke = (0.8, 0.2);
        let mut base = MarkSetSurface {
            width: 60,
            height: 40,
            ..MarkSetSurface::default()
        };
        let mut poked = MarkSetSurface {
            width: 60,
            height: 40,
            ..MarkSetSurface::default()
        };

        room.render(&mut base, 0.5);
        room.render_poked(&mut poked, 0.5, &[poke]);

        let marker = {
            let point = bounded_loop_points(&[poke], 60, 40)
                .pop()
                .expect("finite poke should produce a bounded point");
            (point.x, point.y)
        };
        let added_without_marker = poked
            .marks
            .difference(&base.marks)
            .any(|point| *point != marker);
        let removed_by_shift = base.marks.difference(&poked.marks).next().is_some();

        assert!(added_without_marker || removed_by_shift);
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

        let room = StrangeLoop::new();
        for aspect in [f64::NAN, f64::INFINITY, 0.0, 1.0e12] {
            let mut surface = HugeSurface {
                width: usize::MAX,
                height: usize::MAX,
                aspect,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, f64::INFINITY, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_ROOM_POKES * 200);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }
}
