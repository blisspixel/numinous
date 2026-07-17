//! The Sandpile: Bak-Tang-Wiesenfeld self-organized criticality.
//!
//! Drop grains on a grid. Any cell that reaches four topples: it keeps the
//! remainder and sends one grain to each neighbor; edge grains fall off the
//! board. The order of drops never changes the final height field (the abelian
//! property), yet one more grain can set off an avalanche that reshapes the
//! whole pile. `t` pours into the center; HOLD pours where the hand is. See
//! `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Fixed simulation grid (open boundaries). Independent of the surface size.
const GRID_W: usize = 41;
const GRID_H: usize = 41;
/// Critical height: four or more grains topple.
const THRESHOLD: u16 = 4;
/// Grains poured by each hand press.
const POUR_GRAINS: u16 = 24;
/// Center pour at first contact (enough ink for a legible opening pile).
const ENTRY_GRAINS: u32 = 80;
/// Additional center grains as `t` sweeps to 1.
const SWEEP_GRAINS: u32 = 720;
/// Salt mixed into nonzero variation so pour sites drift.
const VARIATION_SALT: u64 = 0x5A4D_D71E_5EED_0001;

/// One fully relaxed sandpile configuration plus avalanche accounting.
#[derive(Clone, Debug)]
struct Pile {
    heights: Vec<u16>,
    /// Grains added since the pile was empty (including those that later fell off).
    poured: u32,
    /// Topples performed during the most recent stabilize pass.
    last_topples: u64,
    /// Cumulative topples since the pile was empty.
    total_topples: u64,
}

impl Pile {
    fn empty() -> Self {
        Self {
            heights: vec![0; GRID_W * GRID_H],
            poured: 0,
            last_topples: 0,
            total_topples: 0,
        }
    }

    fn index(x: usize, y: usize) -> usize {
        y * GRID_W + x
    }

    fn mass(&self) -> u64 {
        self.heights.iter().map(|&h| u64::from(h)).sum()
    }

    fn peak(&self) -> u16 {
        self.heights.iter().copied().max().unwrap_or(0)
    }

    fn critical_cells(&self) -> usize {
        self.heights.iter().filter(|&&h| h + 1 >= THRESHOLD).count()
    }

    /// Add `grains` at `(x, y)` and relax until every cell is below threshold.
    fn pour_at(&mut self, x: usize, y: usize, grains: u16) {
        if grains == 0 || x >= GRID_W || y >= GRID_H {
            return;
        }
        let i = Self::index(x, y);
        self.heights[i] = self.heights[i].saturating_add(grains);
        self.poured = self.poured.saturating_add(u32::from(grains));
        let topples = stabilize(&mut self.heights);
        self.last_topples = topples;
        self.total_topples = self.total_topples.saturating_add(topples);
    }
}

/// Topple every unstable cell until the configuration is stable.
///
/// Open boundaries: a grain sent past an edge leaves the board. Returns the
/// number of individual topple events (each subtracts four and sends four).
fn stabilize(heights: &mut [u16]) -> u64 {
    let mut topples = 0u64;
    let mut queue: Vec<usize> = heights
        .iter()
        .enumerate()
        .filter_map(|(i, &h)| (h >= THRESHOLD).then_some(i))
        .collect();

    while let Some(i) = queue.pop() {
        while heights[i] >= THRESHOLD {
            heights[i] -= THRESHOLD;
            topples = topples.saturating_add(1);
            let x = i % GRID_W;
            let y = i / GRID_W;
            // Four orthogonal neighbors; off-board grain is lost.
            if x > 0 {
                push_grain(heights, i - 1, &mut queue);
            }
            if x + 1 < GRID_W {
                push_grain(heights, i + 1, &mut queue);
            }
            if y > 0 {
                push_grain(heights, i - GRID_W, &mut queue);
            }
            if y + 1 < GRID_H {
                push_grain(heights, i + GRID_W, &mut queue);
            }
        }
    }
    topples
}

fn push_grain(heights: &mut [u16], index: usize, queue: &mut Vec<usize>) {
    let was = heights[index];
    heights[index] = was.saturating_add(1);
    if was < THRESHOLD && heights[index] >= THRESHOLD {
        queue.push(index);
    }
}

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn ambient_grains(t: f64) -> u32 {
    let u = phase_unit(t);
    ENTRY_GRAINS + (u * SWEEP_GRAINS as f64).round() as u32
}

/// Center pour site, optionally drifted by a nonzero variation seed.
fn pour_site(seed: u64) -> (usize, usize) {
    let cx = GRID_W / 2;
    let cy = GRID_H / 2;
    if seed == 0 {
        return (cx, cy);
    }
    let mix = seed ^ VARIATION_SALT;
    let dx = ((mix % 7) as isize) - 3;
    let dy = (((mix / 7) % 5) as isize) - 2;
    let x = (cx as isize + dx).clamp(0, GRID_W as isize - 1) as usize;
    let y = (cy as isize + dy).clamp(0, GRID_H as isize - 1) as usize;
    (x, y)
}

fn build_ambient(t: f64, seed: u64) -> Pile {
    let mut pile = Pile::empty();
    let (sx, sy) = pour_site(seed);
    let grains = ambient_grains(t);
    // One bulk pour is abelian-equivalent to grain-by-grain for a single site.
    let mut remaining = grains;
    while remaining > 0 {
        let chunk = remaining.min(u32::from(u16::MAX)) as u16;
        pile.pour_at(sx, sy, chunk);
        remaining -= u32::from(chunk);
    }
    pile
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn cell_from_norm(x: f64, y: f64) -> (usize, usize) {
    let gx = ((x * GRID_W as f64) as usize).min(GRID_W - 1);
    let gy = ((y * GRID_H as f64) as usize).min(GRID_H - 1);
    (gx, gy)
}

fn apply_hand_pours(pile: &mut Pile, pokes: &[(f64, f64)]) {
    for &(x, y) in pokes {
        let (gx, gy) = cell_from_norm(x, y);
        pile.pour_at(gx, gy, POUR_GRAINS);
    }
}

fn height_char(h: u16) -> char {
    match h {
        0 => ' ',
        1 => '.',
        2 => ':',
        3 => '#',
        // Unstable cells should not remain after stabilize; show them loudly if they do.
        _ => '*',
    }
}

fn draw_pile(canvas: &mut dyn Surface, pile: &Pile) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for gy in 0..GRID_H {
        for gx in 0..GRID_W {
            let h = pile.heights[Pile::index(gx, gy)];
            if h == 0 {
                continue;
            }
            let ch = height_char(h);
            let left = gx * width / GRID_W;
            let right = (((gx + 1) * width / GRID_W).max(left + 1)).min(width);
            let top = gy * height / GRID_H;
            let bottom = (((gy + 1) * height / GRID_H).max(top + 1)).min(height);
            for py in top..bottom {
                for px in left..right {
                    canvas.plot(px as i32, py as i32, ch);
                }
            }
        }
    }
}

fn mark_pour_sites(canvas: &mut dyn Surface, pokes: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pokes.is_empty() {
        return;
    }
    let radius = (width.min(height) / 32).clamp(2, 8) as i32;
    for &(nx, ny) in pokes {
        let x = (nx * width.saturating_sub(1) as f64).round() as i32;
        let y = (ny * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(x, y, '+');
        canvas.plot(x - radius, y, '+');
        canvas.plot(x + radius, y, '+');
        canvas.plot(x, y - radius, '+');
        canvas.plot(x, y + radius, '+');
    }
}

/// The Sandpile room.
#[derive(Debug, Default)]
pub struct Sandpile {
    seed: u64,
}

impl Sandpile {
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

    fn pile_with_pokes(&self, t: f64, pokes: &[(f64, f64)]) -> Pile {
        let mut pile = build_ambient(t, self.seed);
        let hands = finite_pokes(pokes);
        apply_hand_pours(&mut pile, &hands);
        pile
    }
}

impl Room for Sandpile {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sandpile",
            title: "The Sandpile",
            wing: "Emergence",
            blurb: "Drop grains; four topples to neighbors; self-organized criticality blooms a \
                    fractal mandala. Catastrophe is the resting state. t pours the center; HOLD \
                    pours under the hand.",
            accent: [220, 170, 70],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let pile = build_ambient(t, self.seed);
        draw_pile(canvas, &pile);
    }

    fn postcard_t(&self) -> f64 {
        0.72
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "critical cascade",
            root: 196.00,
            tempo: 108,
            line: &[0, 0, 0, 7, 0, 12, 5, 0],
            encodes: "quiet grains until one topple fans into a cascade",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("HOLD: POUR SAND")
    }

    fn status(&self, t: f64) -> Option<String> {
        let pile = build_ambient(t, self.seed);
        // Invite on first contact; mass and near-critical cells name the pile state.
        Some(format!(
            "THRESH 4  G{}  CRIT {}  HOLD: POUR",
            pile.mass(),
            pile.critical_cells()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let pile = self.pile_with_pokes(t, pokes);
        draw_pile(canvas, &pile);
        mark_pour_sites(canvas, &hands);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        self.render_poked(canvas, t, &crate::held_pokes_from_inputs(inputs));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::held_pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let pile = self.pile_with_pokes(t, &pokes);
        let (nx, ny) = *hands.last().expect("nonempty hands");
        let (gx, gy) = cell_from_norm(nx, ny);
        // Last avalanche size is the measured gasp; mass is what remains on board.
        Some(format!(
            "POUR@{gx},{gy}  TOPPLE {}  MASS {}  PK{}",
            pile.last_topples,
            pile.mass(),
            pile.peak()
        ))
    }

    fn reveal(&self) -> &'static str {
        "Every stable pile sits at the edge of catastrophe: add one grain and \
         an avalanche of any size can follow. The final heights do not care about \
         the order of drops (the abelian property), only how many fell where. \
         That resting criticality is self-organized: the pile tunes itself there \
         with no external dial."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ENTRY_GRAINS, GRID_H, GRID_W, Pile, Sandpile, THRESHOLD, ambient_grains, build_ambient,
        cell_from_norm, finite_pokes, pour_site, stabilize,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput, inputs_from_pokes};

    #[test]
    fn four_grains_at_center_topple_once_to_neighbors() {
        let mut heights = vec![0u16; GRID_W * GRID_H];
        let cx = GRID_W / 2;
        let cy = GRID_H / 2;
        let i = cy * GRID_W + cx;
        heights[i] = THRESHOLD;
        let topples = stabilize(&mut heights);
        assert_eq!(topples, 1);
        assert_eq!(heights[i], 0);
        assert_eq!(heights[i - 1], 1);
        assert_eq!(heights[i + 1], 1);
        assert_eq!(heights[i - GRID_W], 1);
        assert_eq!(heights[i + GRID_W], 1);
    }

    #[test]
    fn edge_topple_loses_grains_off_the_board() {
        let mut heights = vec![0u16; GRID_W * GRID_H];
        // Corner cell has two missing neighbors; four grains leave two on board.
        heights[0] = THRESHOLD;
        let topples = stabilize(&mut heights);
        assert_eq!(topples, 1);
        assert_eq!(heights[0], 0);
        assert_eq!(heights[1], 1);
        assert_eq!(heights[GRID_W], 1);
        let mass: u64 = heights.iter().map(|&h| u64::from(h)).sum();
        assert_eq!(mass, 2);
    }

    #[test]
    fn abelian_property_order_of_two_sites_does_not_matter() {
        let mut a = Pile::empty();
        a.pour_at(10, 10, 17);
        a.pour_at(20, 15, 23);
        let mut b = Pile::empty();
        b.pour_at(20, 15, 23);
        b.pour_at(10, 10, 17);
        assert_eq!(a.heights, b.heights);
        assert_eq!(a.mass(), b.mass());
    }

    #[test]
    fn stabilize_leaves_every_cell_below_threshold() {
        let pile = build_ambient(1.0, 0);
        assert!(pile.heights.iter().all(|&h| h < THRESHOLD));
        assert!(pile.mass() > 0);
        assert!(pile.total_topples > 0);
    }

    #[test]
    fn ambient_grains_grow_with_phase() {
        assert_eq!(ambient_grains(0.0), ENTRY_GRAINS);
        assert!(ambient_grains(1.0) > ambient_grains(0.0));
        assert_eq!(ambient_grains(f64::NAN), ENTRY_GRAINS);
    }

    #[test]
    fn variation_moves_the_pour_site() {
        assert_eq!(pour_site(0), (GRID_W / 2, GRID_H / 2));
        assert_ne!(pour_site(0), pour_site(7));
    }

    #[test]
    fn first_contact_status_invites_a_pour() {
        let room = Sandpile::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("THRESH 4"), "{open}");
        assert!(open.contains("HOLD"), "{open}");
        assert!(open.contains("POUR"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
        assert_eq!(
            room.status_input(0.0, &[]).as_deref(),
            room.status(0.0).as_deref()
        );
    }

    #[test]
    fn pour_changes_status_and_reports_topples() {
        let room = Sandpile::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("pour status");
        assert_ne!(after, open);
        assert!(after.contains("POUR@"), "{after}");
        assert!(after.contains("TOPPLE"), "{after}");
        assert!(after.contains("MASS"), "{after}");
        assert!(after.chars().any(|c| c.is_ascii_digit()), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Sandpile::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);

        let mut entry = Canvas::new(80, 40);
        room.render(&mut entry, 0.0);
        assert!(
            entry.ink_count() >= 20,
            "opening pile must show sand at first contact"
        );

        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 30);
    }

    #[test]
    fn hand_pour_changes_the_picture() {
        let room = Sandpile::new();
        let mut base = Canvas::new(60, 40);
        let mut poked = Canvas::new(60, 40);
        room.render(&mut base, 0.2);
        room.render_poked(&mut poked, 0.2, &[(0.2, 0.8)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn compact_static_hands_preserve_multiple_pour_sites() {
        let room = Sandpile::new();
        let multi = inputs_from_pokes(&[(0.2, 0.2), (0.8, 0.8)], 0.72);
        let single = inputs_from_pokes(&[(0.8, 0.8)], 0.72);
        let mut multi_render = Canvas::new(40, 20);
        let mut single_render = Canvas::new(40, 20);
        room.render_input(&mut multi_render, 0.72, &multi);
        room.render_input(&mut single_render, 0.72, &single);
        assert_ne!(multi_render.to_text(), single_render.to_text());
        assert_ne!(
            room.status_input(0.72, &multi),
            room.status_input(0.72, &single)
        );
    }

    #[test]
    fn variation_seed_changes_ambient_render() {
        let mut a = Canvas::new(48, 28);
        let mut b = Canvas::new(48, 28);
        Sandpile::new_with(0).render(&mut a, 0.55);
        Sandpile::new_with(42).render(&mut b, 0.55);
        assert_ne!(a.to_text(), b.to_text());
        let mut zero = Canvas::new(48, 28);
        Sandpile::new().render(&mut zero, 0.55);
        assert_eq!(a.to_text(), zero.to_text());
    }

    #[test]
    fn finite_pokes_use_newest_bounded_tail() {
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.4))
            .collect();
        let mut old = vec![(0.9, 0.9); MAX_ROOM_POKES + 9];
        old.extend(newest.clone());
        assert_eq!(finite_pokes(&old), finite_pokes(&newest));
    }

    #[test]
    fn cell_from_norm_clamps_to_grid() {
        assert_eq!(cell_from_norm(0.0, 0.0), (0, 0));
        assert_eq!(cell_from_norm(1.0, 1.0), (GRID_W - 1, GRID_H - 1));
        assert_eq!(cell_from_norm(-1.0, 2.0), (0, GRID_H - 1));
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Sandpile::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
            room.render_poked(&mut canvas, t, &[(0.5, 0.5)]);
        }
    }

    #[test]
    fn reveal_names_criticality_and_abelian_order() {
        let text = Sandpile::new().reveal();
        assert!(text.to_ascii_lowercase().contains("abelian") || text.contains("order"));
        assert!(text.contains("critical") || text.contains("catastrophe"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = Sandpile::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}
