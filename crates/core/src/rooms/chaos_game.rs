//! Chaos Game: a Sierpinski triangle drawn by pure chance.
//!
//! Start somewhere, then repeatedly pick a random corner of a triangle and move
//! a fixed fraction of the way toward it, leaving a dot each time. There is no
//! triangle in the rule, yet a perfect Sierpinski fractal appears. `t` tunes the
//! jump fraction (0.5 is the iconic value). See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the render reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x0DDB_1A5E_5EED_1234;
/// Iterations per cell, capped so large canvases stay fast.
const ITERS_PER_CELL: usize = 6;
const MAX_ITERS: usize = 300_000;
/// Discard the first few points so the transient toward the attractor does not show.
const WARMUP: usize = 16;

/// The Chaos Game room.
#[derive(Debug, Default)]
pub struct ChaosGame {
    seed: u64,
}

impl ChaosGame {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    /// The jump fraction selected by phase `t`; 0.5 (the Sierpinski value) at `t = 0`.
    fn ratio_for(t: f64) -> f64 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        0.5 + 0.12 * t
    }
}

fn base_vertices(width: usize, height: usize) -> [(f64, f64); 3] {
    let (fw, fh) = (width as f64, height as f64);
    [
        (fw / 2.0, 0.0),      // apex
        (0.0, fh - 1.0),      // bottom left
        (fw - 1.0, fh - 1.0), // bottom right
    ]
}

fn player_corners(pokes: &[(f64, f64)]) -> impl Iterator<Item = (f64, f64)> + '_ {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..].iter().filter_map(|&(x, y)| {
        if x.is_finite() && y.is_finite() {
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        } else {
            None
        }
    })
}

fn extend_player_vertices(
    vertices: &mut Vec<(f64, f64)>,
    pokes: &[(f64, f64)],
    width: usize,
    height: usize,
) {
    let max_x = width.saturating_sub(1) as f64;
    let max_y = height.saturating_sub(1) as f64;
    for (x, y) in player_corners(pokes) {
        let vertex = (x * max_x, y * max_y);
        if !vertices
            .iter()
            .any(|&existing| vertex_cell(existing) == vertex_cell(vertex))
        {
            vertices.push(vertex);
        }
    }
}

fn vertex_cell((x, y): (f64, f64)) -> (i32, i32) {
    (x.round() as i32, y.round() as i32)
}

fn plot_vertex(canvas: &mut dyn Surface, vx: f64, vy: f64) {
    let (cx, cy) = vertex_cell((vx, vy));
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let (Some(x), Some(y)) = (cx.checked_add(dx), cy.checked_add(dy)) {
                canvas.plot(x, y, '#');
            }
        }
    }
}

fn render_vertices(canvas: &mut dyn Surface, t: f64, seed: u64, vertices: &[(f64, f64)]) {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 || vertices.is_empty() {
        return;
    }
    let ratio = ChaosGame::ratio_for(t);
    let (fw, fh) = (width as f64, height as f64);
    let mut rng = SplitMix64::new(SEED ^ seed);
    let (mut px, mut py) = (fw / 2.0, fh / 2.0);
    let iterations = width
        .saturating_mul(height)
        .saturating_mul(ITERS_PER_CELL)
        .min(MAX_ITERS);
    let n = vertices.len() as u64;
    for i in 0..iterations {
        let corner = vertices[rng.below(n) as usize];
        px = px * (1.0 - ratio) + corner.0 * ratio;
        py = py * (1.0 - ratio) + corner.1 * ratio;
        if i >= WARMUP {
            canvas.plot(px.round() as i32, py.round() as i32, '*');
        }
    }
}

impl Room for ChaosGame {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "chaos-game",
            title: "Chaos Game",
            wing: "Emergence",
            blurb: "Jump halfway to a random corner of a triangle, over and over, and pure chance \
                    resolves into a perfect Sierpinski fractal. t tunes the jump fraction.",
            accent: [40, 200, 170],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        render_vertices(canvas, t, self.seed, &base_vertices(width, height));
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "three corners",
            root: 164.81,
            tempo: 132,
            line: &[0, 12, 4, 12, 7, 12, 0, 12],
            encodes: "always jumping halfway home: three notes and their echo",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: ADD A CORNER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ratio = Self::ratio_for(t);
        Some(format!("3 CORNERS   JUMP {ratio:.2}   CLICK: ADD A CORNER"))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        // Match the default render_input poke bridge so gesture and poke paths
        // report the same corner count.
        let pokes = crate::pokes_from_inputs(inputs);
        let added = player_corners(&pokes).count();
        if added == 0 {
            return self.status(t);
        }
        let total = 3 + added;
        let ratio = Self::ratio_for(t);
        Some(format!(
            "{added} ADDED   {total} CORNERS   JUMP {ratio:.2}   ATTRACTOR REBUILT"
        ))
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
        let mut vertices = Vec::with_capacity(3 + MAX_ROOM_POKES);
        vertices.extend(base_vertices(width, height));
        extend_player_vertices(&mut vertices, pokes, width, height);
        if vertices.len() == 3 {
            render_vertices(canvas, t, self.seed, &vertices);
            return;
        }
        render_vertices(canvas, t, self.seed, &vertices);
        // The corners themselves, bright, so the hand sees what it built.
        for &(vx, vy) in &vertices {
            plot_vertex(canvas, vx, vy);
        }
    }

    fn reveal(&self) -> &'static str {
        "Every dot landed at random, and there is no triangle anywhere in the \
         rule, only 'jump halfway to a random corner'. Yet a perfect Sierpinski \
         fractal appears every time. Randomness has a shape."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChaosGame, base_vertices, extend_player_vertices, player_corners, render_vertices,
    };
    use crate::canvas::Canvas;
    use crate::room::MAX_ROOM_POKES;
    use crate::room::Room;

    fn char_at(canvas: &Canvas, x: usize, y: usize) -> char {
        canvas
            .to_text()
            .lines()
            .nth(y)
            .and_then(|line| line.chars().nth(x))
            .unwrap_or(' ')
    }

    fn attractor_text(vertices: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 24);
        render_vertices(&mut canvas, 0.0, 0, vertices);
        canvas.to_text()
    }

    #[test]
    fn first_contact_status_invites_an_extra_corner() {
        let room = ChaosGame::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("3 CORNERS"), "{open}");
        assert!(open.contains("JUMP 0.50"), "{open}");
        assert!(open.contains("CLICK: ADD A CORNER"), "{open}");
        let inputs = [crate::room::RoomInput::PointerDown {
            x: 0.5,
            y: 0.25,
            t: 0.0,
        }];
        let added = room.status_input(0.0, &inputs).expect("added");
        assert!(added.contains("1 ADDED"), "{added}");
        assert!(added.contains("4 CORNERS"), "{added}");
    }

    #[test]
    fn ratio_is_one_half_at_zero() {
        assert!((ChaosGame::ratio_for(0.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn ratio_uses_safe_nonfinite_fallback() {
        assert_eq!(ChaosGame::ratio_for(f64::NAN), ChaosGame::ratio_for(0.0));
        assert_eq!(
            ChaosGame::ratio_for(f64::INFINITY),
            ChaosGame::ratio_for(0.0)
        );
    }

    #[test]
    fn render_is_deterministic() {
        let room = ChaosGame::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = ChaosGame::new_with(0);
        let r_def = ChaosGame::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        let r42 = ChaosGame::new_with(42);
        let mut c = Canvas::new(48, 24);
        r42.render(&mut c, 0.5);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = ChaosGame::new();
        let mut canvas = Canvas::new(48, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = ChaosGame::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-3.0, 0.0, 0.999, 4.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
        }
    }

    #[test]
    fn player_corners_preserve_order_clamp_and_filter() {
        let corners: Vec<_> =
            player_corners(&[(0.2, 0.3), (f64::NAN, 0.5), (2.0, -1.0), (0.4, 0.6)]).collect();
        assert_eq!(corners, vec![(0.2, 0.3), (1.0, 0.0), (0.4, 0.6)]);
    }

    #[test]
    fn added_corner_is_visible_and_changes_the_attractor() {
        let room = ChaosGame::new();
        let base_vertices = base_vertices(48, 24);
        let mut player_vertices = Vec::from(base_vertices);
        extend_player_vertices(&mut player_vertices, &[(0.5, 0.25)], 48, 24);
        assert_ne!(
            attractor_text(&base_vertices),
            attractor_text(&player_vertices),
            "added corner must alter the attractor before marker plotting"
        );
        let mut poked = Canvas::new(48, 24);
        room.render_poked(&mut poked, 0.0, &[(0.5, 0.25)]);
        assert_eq!(char_at(&poked, 24, 6), '#');
    }

    #[test]
    fn edge_corner_remains_visible() {
        let room = ChaosGame::new();
        let mut poked = Canvas::new(48, 24);
        room.render_poked(&mut poked, 0.0, &[(1.0, 0.5)]);
        assert_eq!(char_at(&poked, 47, 12), '#');
    }

    #[test]
    fn duplicate_player_corners_are_collapsed_before_rendering() {
        let room = ChaosGame::new();
        let mut duplicate = Canvas::new(48, 24);
        let mut single = Canvas::new(48, 24);
        room.render_poked(&mut duplicate, 0.0, &[(0.5, 0.25), (0.5, 0.25)]);
        room.render_poked(&mut single, 0.0, &[(0.5, 0.25)]);
        assert_eq!(duplicate.to_text(), single.to_text());
    }

    #[test]
    fn player_corners_do_not_duplicate_triangle_vertices() {
        let mut vertices = Vec::from(base_vertices(48, 24));
        extend_player_vertices(&mut vertices, &[(0.5, 0.0), (0.0, 1.0), (1.0, 1.0)], 48, 24);
        assert_eq!(vertices, Vec::from(base_vertices(48, 24)));
    }

    #[test]
    fn added_corners_use_the_newest_bounded_finite_points() {
        let room = ChaosGame::new();
        let newest = vec![(0.7, 0.3); MAX_ROOM_POKES];
        let mut old = vec![(0.2, 0.8); MAX_ROOM_POKES + 12];
        old.extend(newest.clone());

        let mut expected = Canvas::new(48, 24);
        let mut actual = Canvas::new(48, 24);
        room.render_poked(&mut expected, 0.0, &newest);
        room.render_poked(&mut actual, 0.0, &old);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_corner_identity() {
        let room = ChaosGame::new();
        let finite = [(0.4, 0.6)];
        let with_bad_points = [
            (f64::NAN, 0.1),
            (f64::INFINITY, 0.2),
            finite[0],
            (0.3, f64::NEG_INFINITY),
        ];
        let mut expected = Canvas::new(48, 24);
        let mut actual = Canvas::new(48, 24);
        room.render_poked(&mut expected, 0.0, &finite);
        room.render_poked(&mut actual, 0.0, &with_bad_points);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = ChaosGame::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        let mut expected = Canvas::new(48, 24);
        let mut actual = Canvas::new(48, 24);
        room.render(&mut expected, 0.0);
        room.render_poked(&mut actual, 0.0, &with_invalid_tail);
        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn nonzero_variation_changes_poked_attractor() {
        let default = ChaosGame::new();
        let zero = ChaosGame::new_with(0);
        let varied = ChaosGame::new_with(42);
        let pokes = [(0.4, 0.6)];
        let mut default_poked = Canvas::new(48, 24);
        let mut zero_poked = Canvas::new(48, 24);
        let mut varied_poked = Canvas::new(48, 24);
        default.render_poked(&mut default_poked, 0.0, &pokes);
        zero.render_poked(&mut zero_poked, 0.0, &pokes);
        varied.render_poked(&mut varied_poked, 0.0, &pokes);
        assert_eq!(default_poked.to_text(), zero_poked.to_text());
        assert_ne!(default_poked.to_text(), varied_poked.to_text());
    }

    #[test]
    fn reveal_mentions_the_shape_of_randomness() {
        assert!(ChaosGame::new().reveal().contains("Sierpinski"));
    }
}
