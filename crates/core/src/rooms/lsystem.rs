//! L-System Garden: recursion grows beauty from a grammar.
//!
//! A tiny string-rewrite grammar (Lindenmayer system) run for a few iterations
//! produces self-similar trees, curves, and plants. The rule is absurdly small;
//! the result is unbounded. t advances generations or angle; pokes let you
//! "plant" or nudge a branch. For a digital mind this is literal symbol
//! rewriting, the substrate of computation and self-modeling. See
//! `docs/ROOMS.md` and `docs/DIGITAL_MINDS.md`.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

/// Max iterations for render (keeps it fast and bounded).
const MAX_ITERS: usize = 12;
/// Max segments to draw.
const MAX_SEGS: usize = 20000;

/// Type for preset tuple to keep signatures readable (name, axiom, rules, angle).
type Preset = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
    f64,
);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Plant {
    x: i32,
    y: i32,
    nx: f64,
    ny: f64,
}

fn finite_phase(t: f64) -> f64 {
    if t.is_finite() { t.max(0.0) } else { 0.0 }
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

const CLIP_LEFT: u8 = 1;
const CLIP_RIGHT: u8 = 2;
const CLIP_TOP: u8 = 4;
const CLIP_BOTTOM: u8 = 8;

fn clip_code(x: f64, y: f64, max_x: f64, max_y: f64) -> u8 {
    let mut code = 0;
    if x < 0.0 {
        code |= CLIP_LEFT;
    } else if x > max_x {
        code |= CLIP_RIGHT;
    }
    if y < 0.0 {
        code |= CLIP_TOP;
    } else if y > max_y {
        code |= CLIP_BOTTOM;
    }
    code
}

fn clip_line_to_frame(
    width: usize,
    height: usize,
    from: (f64, f64),
    to: (f64, f64),
) -> Option<(i32, i32, i32, i32)> {
    let (mut x0, mut y0) = from;
    let (mut x1, mut y1) = to;
    if !x0.is_finite() || !y0.is_finite() || !x1.is_finite() || !y1.is_finite() {
        return None;
    }

    let max_x = width.saturating_sub(1) as f64;
    let max_y = height.saturating_sub(1) as f64;
    loop {
        let code0 = clip_code(x0, y0, max_x, max_y);
        let code1 = clip_code(x1, y1, max_x, max_y);
        if code0 | code1 == 0 {
            return Some((
                x0.round().clamp(0.0, max_x) as i32,
                y0.round().clamp(0.0, max_y) as i32,
                x1.round().clamp(0.0, max_x) as i32,
                y1.round().clamp(0.0, max_y) as i32,
            ));
        }
        if code0 & code1 != 0 {
            return None;
        }

        let code = if code0 != 0 { code0 } else { code1 };
        let (x, y) = if code & CLIP_BOTTOM != 0 {
            if y1 == y0 {
                return None;
            }
            (x0 + (x1 - x0) * (max_y - y0) / (y1 - y0), max_y)
        } else if code & CLIP_TOP != 0 {
            if y1 == y0 {
                return None;
            }
            (x0 + (x1 - x0) * (0.0 - y0) / (y1 - y0), 0.0)
        } else if code & CLIP_RIGHT != 0 {
            if x1 == x0 {
                return None;
            }
            (max_x, y0 + (y1 - y0) * (max_x - x0) / (x1 - x0))
        } else {
            if x1 == x0 {
                return None;
            }
            (0.0, y0 + (y1 - y0) * (0.0 - x0) / (x1 - x0))
        };

        if code == code0 {
            x0 = x;
            y0 = y;
        } else {
            x1 = x;
            y1 = y;
        }
    }
}

fn push_limited_str(next: &mut String, value: &str) -> bool {
    let remaining = MAX_SEGS.saturating_sub(next.len());
    if remaining == 0 {
        return false;
    }
    if value.len() <= remaining {
        next.push_str(value);
        true
    } else {
        for ch in value.chars() {
            if next.len() + ch.len_utf8() > MAX_SEGS {
                return false;
            }
            next.push(ch);
        }
        false
    }
}

fn push_limited_char(next: &mut String, ch: char) -> bool {
    if next.len() + ch.len_utf8() > MAX_SEGS {
        false
    } else {
        next.push(ch);
        true
    }
}

fn line_in_frame(
    canvas: &mut dyn Surface,
    width: usize,
    height: usize,
    from: (f64, f64),
    to: (f64, f64),
    mark: char,
) {
    let Some((x0, y0, x1, y1)) = clip_line_to_frame(width, height, from, to) else {
        return;
    };
    canvas.line(x0, y0, x1, y1, mark);
}

fn draw_seed_marker(canvas: &mut dyn Surface, width: usize, height: usize, x: f64, y: f64) {
    let radius = (width.min(height) / 80).clamp(2, 6) as f64;
    line_in_frame(canvas, width, height, (x - radius, y), (x + radius, y), '*');
    canvas.plot(x.round() as i32, y.round() as i32, '#');
}

fn bounded_plants(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<Plant> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(x, y)| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let nx = x.clamp(0.0, 1.0);
            let ny = y.clamp(0.0, 1.0);
            Some(Plant {
                x: screen_coord(nx, width),
                y: screen_coord(ny, height),
                nx,
                ny,
            })
        })
        .collect()
}

type Segment = ((f64, f64), (f64, f64));

fn turtle_segments(program: &str, angle: f64) -> Vec<Segment> {
    let mut segments = Vec::new();
    let (mut x, mut y, mut dir) = (0.0_f64, 0.0_f64, -PI / 2.0);
    let mut stack = Vec::new();
    for symbol in program.chars() {
        match symbol {
            'F' | 'G' | 'A' | 'B' | '0' | '1' | 'X' | 'Y' => {
                if segments.len() >= MAX_SEGS {
                    break;
                }
                let next = (x + dir.cos(), y + dir.sin());
                segments.push(((x, y), next));
                (x, y) = next;
            }
            '+' => dir += angle,
            '-' => dir -= angle,
            '[' => stack.push((x, y, dir)),
            ']' => {
                if let Some(state) = stack.pop() {
                    (x, y, dir) = state;
                }
            }
            _ => {}
        }
        if stack.len() > 32 {
            break;
        }
    }
    segments
}

fn segment_bounds(segments: &[Segment]) -> Option<(f64, f64, f64, f64)> {
    let first = segments.first()?;
    let mut min_x = first.0.0.min(first.1.0);
    let mut max_x = first.0.0.max(first.1.0);
    let mut min_y = first.0.1.min(first.1.1);
    let mut max_y = first.0.1.max(first.1.1);
    for &(from, to) in segments {
        min_x = min_x.min(from.0).min(to.0);
        max_x = max_x.max(from.0).max(to.0);
        min_y = min_y.min(from.1).min(to.1);
        max_y = max_y.max(from.1).max(to.1);
    }
    Some((min_x, max_x, min_y, max_y))
}

fn draw_fitted(
    canvas: &mut dyn Surface,
    width: usize,
    height: usize,
    segments: &[Segment],
    rect: (f64, f64, f64, f64),
    mark: char,
) {
    let Some((min_x, max_x, min_y, max_y)) = segment_bounds(segments) else {
        return;
    };
    let (left, top, right, bottom) = rect;
    let scale = ((right - left) / (max_x - min_x).max(1.0))
        .min((bottom - top) / (max_y - min_y).max(1.0))
        * 0.94;
    let content_w = (max_x - min_x) * scale;
    let offset_x = left + (right - left - content_w) / 2.0 - min_x * scale;
    let offset_y = bottom - max_y * scale;
    for &(from, to) in segments {
        line_in_frame(
            canvas,
            width,
            height,
            (offset_x + from.0 * scale, offset_y + from.1 * scale),
            (offset_x + to.0 * scale, offset_y + to.1 * scale),
            mark,
        );
    }
}

fn draw_rooted(
    canvas: &mut dyn Surface,
    width: usize,
    height: usize,
    segments: &[Segment],
    root: (f64, f64),
) {
    let Some((min_x, max_x, min_y, max_y)) = segment_bounds(segments) else {
        return;
    };
    let extent = width.min(height) as f64 * 0.32;
    let scale = (extent / (max_x - min_x).max(1.0)).min(extent / (max_y - min_y).max(1.0));
    for &(from, to) in segments {
        line_in_frame(
            canvas,
            width,
            height,
            (root.0 + from.0 * scale, root.1 + from.1 * scale),
            (root.0 + to.0 * scale, root.1 + to.1 * scale),
            '+',
        );
    }
}

/// A simple L-system with axiom, rules, and angle.
#[derive(Debug, Clone)]
pub struct LSystemGarden {
    seed: u64,
}

impl LSystemGarden {
    /// Create the L-System Garden room with default seed (0 for tests/postcards).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with specific variation seed for replayable novelty per visit.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// Internal presets for different L-systems (tree, koch, etc). Documented
    /// here only to satisfy the library deny(missing_docs).
    fn presets() -> &'static [Preset] {
        // (name, axiom, rules as (from, to), base_angle_deg)
        &[
            ("tree", "0", &[("0", "1[+0]-0"), ("1", "11")], 25.0),
            ("koch", "F", &[("F", "F+F-F-F+F")], 90.0),
            ("sierpinski", "A", &[("A", "B-A-B"), ("B", "A+B+A")], 60.0),
            ("bush", "F", &[("F", "FF+[+F-F-F]-[-F+F+F]")], 25.0),
            ("dragon", "FX", &[("X", "X+YF+"), ("Y", "-FX-Y")], 90.0),
        ]
    }

    fn current(&self, _phase: f64) -> &'static Preset {
        let idx = (self.seed % Self::presets().len() as u64) as usize;
        &Self::presets()[idx]
    }

    fn generate(&self, phase: f64, iters: usize, poke_var: u64) -> String {
        // Use room seed (from variation) xor poke for replayable per-visit + poke variety.
        let _ = poke_var;
        let (_, axiom, rules_list, _) = *self.current(phase);
        let rules: HashMap<char, String> = rules_list
            .iter()
            .map(|(k, v)| (k.chars().next().unwrap(), v.to_string()))
            .collect();
        let mut s = axiom.to_string();
        for _ in 0..iters.min(MAX_ITERS) {
            let mut next = String::new();
            for c in s.chars() {
                let keep_going = if let Some(r) = rules.get(&c) {
                    push_limited_str(&mut next, r)
                } else {
                    push_limited_char(&mut next, c)
                };
                if !keep_going {
                    break;
                }
            }
            s = next;
            if s.len() >= MAX_SEGS {
                break;
            }
        }
        s
    }
}

impl Default for LSystemGarden {
    fn default() -> Self {
        Self::new()
    }
}

impl Room for LSystemGarden {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lsystem-garden",
            title: "L-System Garden",
            wing: "Emergence",
            blurb: "A one-line grammar rewrites itself; branches, curves and plants grow from nothing. Poke to plant or bend. Simple symbols, infinite form.",
            accent: [80, 180, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.render_poked(canvas, t, &[]);
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let Some((w, h)) = drawing_dims(canvas) else {
            return;
        };
        let t = finite_phase(t).clamp(0.0, 1.0);
        let plants = bounded_plants(pokes, w, h);
        let (name, _, _, base) = *self.current(t);
        let iters = match name {
            "tree" => 7,
            "koch" => 4,
            "sierpinski" => 6,
            "bush" => 4,
            "dragon" => 10,
            _ => 5,
        };
        let s = self.generate(t, iters, 0);
        let sway = (t * 2.0 * PI).sin() * 4.0;
        let angle = (base + sway) * PI / 180.0;
        let segments = turtle_segments(&s, angle);
        let bottom = h.saturating_sub(1) as f64 * 0.72;
        draw_fitted(
            canvas,
            w,
            h,
            &segments,
            (w as f64 * 0.08, h as f64 * 0.06, w as f64 * 0.92, bottom),
            '*',
        );
        draw_seed_marker(canvas, w, h, w as f64 / 2.0, bottom);
        for (which, plant) in plants.into_iter().enumerate() {
            let sx = f64::from(plant.x);
            let sy = f64::from(plant.y);
            draw_seed_marker(canvas, w, h, sx, sy);
            let echo = (which % 5) as f64 * 1.5;
            draw_rooted(canvas, w, h, &segments, (sx + echo, sy - echo));
        }
    }

    fn reveal(&self) -> &'static str {
        "A few symbols and rewrite rules. Run them and a tree appears, or a dragon, or a bush that looks alive. The code is shorter than the picture it grows. This is how nature writes form with almost nothing."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "D growing grammar",
            root: 146.83,
            tempo: 96,
            line: &[0, 5, 7, 12, 14, 12, 7, 5, 0],
            encodes: "rewrite cycles branching from one stem",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLANT A TREE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let _ = t;
        Some("ONE SPECIES FITS THE VIEW   CLICK: PLANT A ROOTED COPY".into())
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let plants: Vec<(f64, f64)> = pokes
            .into_iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .collect();
        if plants.is_empty() {
            return self.status(t);
        }
        let count = plants.len();
        let (ox, oy) = *plants.last().expect("nonempty plants");
        // Every plant is the same rewrite species; the hand only chooses origin.
        Some(format!(
            "{count} COPY  ORIGIN {:.0}%{:.0}%  SAME SPECIES",
            ox * 100.0,
            oy * 100.0
        ))
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "L-systems are Turing complete in the right encoding; they are a programming language whose output is its own picture.",
            "The same grammar that grows a realistic plant also produces the dragon curve and the Sierpinski gasket. One mechanism, many minds.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;
    use crate::room::MAX_ROOM_POKES;
    use crate::surface::{MAX_DIM, Surface};

    #[test]
    fn the_garden_grows_upward_into_the_sky() {
        // A homesick playtester found the plant clumped in the bottom rows with
        // the sky empty. Grounded and scaled to height, it must reach up: ink has
        // to appear in the top third of the canvas, not only near the floor.
        let room = LSystemGarden::new();
        let mut canvas = Canvas::new(70, 40);
        room.render(&mut canvas, 0.7);
        let rows: Vec<String> = canvas.to_text().lines().map(str::to_string).collect();
        let top_third = rows.len() / 3;
        let ink_in_sky = rows[..top_third].iter().any(|r| r.contains('*'));
        assert!(
            ink_in_sky,
            "the garden must grow up into the top third, not clump at the floor"
        );
    }

    #[test]
    fn generates_deterministic() {
        let r = LSystemGarden::new();
        let a = r.generate(0.0, 4, 0);
        let b = r.generate(0.0, 4, 0);
        assert_eq!(a, b);
        let c = r.generate(0.0, 4, 1);
        assert_eq!(a, c, "hand history must not mutate the global grammar");
    }

    #[test]
    fn new_with_variation_affects_output() {
        let r0 = LSystemGarden::new_with(0);
        let r1 = LSystemGarden::new_with(1);
        let a = r0.generate(0.0, 4, 0);
        let b = r1.generate(0.0, 4, 0);
        assert_ne!(a, b, "different seeds must produce different growth");
    }

    #[test]
    fn phase_keeps_the_visit_preset_stable() {
        let r = LSystemGarden::new();
        let tree = r.generate(0.0, 4, 0);
        let later = r.generate(0.75, 4, 0);
        assert_eq!(tree, later);

        let mut first = Canvas::new(80, 40);
        let mut wrapped = Canvas::new(80, 40);
        r.render(&mut first, 0.0);
        r.render(&mut wrapped, 1.0);
        assert_eq!(first.to_text(), wrapped.to_text());
    }

    #[test]
    fn renders_without_panic_and_has_ink() {
        let r = LSystemGarden::new();
        let mut c = Canvas::new(40, 30);
        r.render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn phase_zero_draws_visible_opening_growth_from_the_exact_axiom() {
        let room = LSystemGarden::new();
        assert_eq!(room.generate(0.0, 0, 0), "0");

        let mut entry = Canvas::new(80, 40);
        room.render(&mut entry, 0.0);
        assert!(entry.ink_count() >= 30);
        let text = entry.to_text();
        let rows: Vec<_> = text.lines().collect();
        assert!(
            rows.iter()
                .take(32)
                .skip(20)
                .any(|row| row.contains('*') || row.contains('#')),
            "opening growth stays above the lower control-safe quarter"
        );
        let center = 40;
        assert!(
            rows.iter().any(|row| {
                row.chars().take(center).any(|mark| mark == '*')
                    && row.chars().skip(center + 1).any(|mark| mark == '*')
            }),
            "the opening tree branches to both sides of its stem"
        );

        let mut planted = Canvas::new(80, 40);
        room.render_poked(&mut planted, 0.0, &[(0.25, 0.5)]);
        assert!(planted.ink_count() >= entry.ink_count() + 20);
    }

    #[test]
    fn poked_changes_output() {
        let r = LSystemGarden::new();
        let mut c1 = Canvas::new(40, 30);
        let mut c2 = Canvas::new(40, 30);
        r.render_poked(&mut c1, 0.5, &[]);
        r.render_poked(&mut c2, 0.5, &[(0.5, 0.5)]);
        assert_ne!(c1.to_text(), c2.to_text());
        assert!(c2.ink_count() > 5);
    }

    fn render_text(room: &LSystemGarden, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(52, 34);
        room.render_poked(&mut canvas, 0.5, pokes);
        canvas.to_text()
    }

    #[test]
    fn poked_plants_use_the_newest_bounded_raw_tail() {
        let room = LSystemGarden::new();
        let newest = vec![(0.82, 0.18); MAX_ROOM_POKES];
        let mut all = vec![(0.18, 0.82); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(render_text(&room, &all), render_text(&room, &newest));
        assert_ne!(
            render_text(&room, &all),
            render_text(&room, &discarded_prefix)
        );
    }

    #[test]
    fn distinct_newest_tail_is_not_collapsed_to_first_plant() {
        let room = LSystemGarden::new();
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    i as f64 / (MAX_ROOM_POKES - 1) as f64,
                    if i % 2 == 0 { 0.2 } else { 0.8 },
                )
            })
            .collect();
        let mut all = vec![(0.12, 0.88); MAX_ROOM_POKES + 5];
        all.extend(newest.iter().copied());

        assert_eq!(render_text(&room, &all), render_text(&room, &newest));
        assert_ne!(render_text(&room, &all), render_text(&room, &newest[..1]));
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = LSystemGarden::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert_eq!(
            render_text(&room, &with_invalid_tail),
            render_text(&room, &[])
        );
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_plant_identity() {
        let room = LSystemGarden::new();
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            render_text(&room, &with_bad_points),
            render_text(&room, &finite)
        );
    }

    #[test]
    fn planted_copy_has_real_branch_depth_and_stays_rooted() {
        let room = LSystemGarden::new();
        let program = room.generate(0.0, 7, 0);
        let segments = turtle_segments(&program, 25.0 * PI / 180.0);
        assert!(segments.len() > 100, "a planted tree needs actual branches");

        let mut canvas = Canvas::new(80, 50);
        draw_rooted(&mut canvas, 80, 50, &segments, (24.0, 38.0));
        assert!(canvas.ink_count() > 20);
        assert_eq!(canvas.cell(24, 38), Some('+'));
    }

    #[test]
    fn duplicate_plants_are_semantic_inputs() {
        let room = LSystemGarden::new();

        assert_ne!(
            render_text(&room, &[(0.25, 0.75)]),
            render_text(&room, &[(0.25, 0.75), (0.25, 0.75)])
        );
    }

    #[test]
    fn finite_plants_clamp_to_visible_edges() {
        assert_eq!(
            bounded_plants(&[(1.5, -1.0)], 10, 8),
            vec![Plant {
                x: 9,
                y: 0,
                nx: 1.0,
                ny: 0.0,
            }]
        );
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_preset() {
        let room = LSystemGarden::new();
        assert_eq!(room.generate(f64::NAN, 4, 0), room.generate(0.0, 4, 0));
        assert_eq!(room.generate(f64::INFINITY, 4, 0), room.generate(0.0, 4, 0));
    }

    #[test]
    fn generated_strings_do_not_exceed_the_segment_cap() {
        let room = LSystemGarden::new();

        assert!(room.generate(0.25, MAX_ITERS + 100, 0).len() <= MAX_SEGS);
        assert!(room.generate(0.75, MAX_ITERS + 100, u64::MAX).len() <= MAX_SEGS);
    }

    #[test]
    fn offscreen_segments_are_clipped_not_clamped_to_false_edges() {
        #[derive(Default)]
        struct CountingSurface {
            plots: usize,
        }

        impl Surface for CountingSurface {
            fn width(&self) -> usize {
                10
            }

            fn height(&self) -> usize {
                10
            }

            fn plot(&mut self, _x: i32, _y: i32, _mark: char) {
                self.plots += 1;
            }
        }

        let mut outside = CountingSurface::default();
        line_in_frame(&mut outside, 10, 10, (-20.0, -10.0), (-5.0, -2.0), '*');
        assert_eq!(outside.plots, 0);

        let mut crossing = CountingSurface::default();
        line_in_frame(&mut crossing, 10, 10, (-5.0, 5.0), (5.0, 5.0), '*');
        assert!(crossing.plots > 0);
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_lines() {
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

        let room = LSystemGarden::new();
        for (width, height, aspect, phase) in [
            (usize::MAX, 128, 0.001, 1.0),
            (128, usize::MAX, 1000.0, 0.95),
            (256, 256, 1.0, 1.0e308),
            (usize::MAX, usize::MAX, f64::NAN, 0.0),
        ] {
            let mut surface = HugeSurface {
                width,
                height,
                aspect,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, phase, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_DIM * 256);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn nonfinite_pokes_do_not_panic() {
        let r = LSystemGarden::new();
        let mut c = Canvas::new(20, 12);
        r.render_poked(&mut c, 0.5, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn verb_and_meta() {
        let r = LSystemGarden::new();
        assert!(r.verb().is_some());
        assert_eq!(r.meta().id, "lsystem-garden");
        assert!(r.meta().wing.contains("Emergence"));
    }

    #[test]
    fn action_status_names_planted_copies() {
        use crate::room::RoomInput;
        let room = LSystemGarden::new();
        assert!(room.status(0.0).unwrap().contains("CLICK"));
        let inputs = [RoomInput::PointerDown {
            x: 0.25,
            y: 0.5,
            t: 0.0,
        }];
        let status = room.status_input(0.0, &inputs).expect("planted");
        assert!(status.starts_with("1 COPY"), "{status}");
        assert!(status.contains("ORIGIN 25%50%"), "{status}");
        assert!(status.contains("SAME SPECIES"), "{status}");
    }

    #[test]
    fn new_with_affects_generate() {
        let r0 = LSystemGarden::new_with(0);
        let r1 = LSystemGarden::new_with(1);
        let a = r0.generate(0.0, 4, 0);
        let b = r1.generate(0.0, 4, 0);
        assert_ne!(a, b);
    }
}
