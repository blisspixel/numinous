//! Site percolation on a square grid: open sites and cluster flood.
//!
//! DRAG: TUNE OPEN PROB. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
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

fn open_p(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        (x * 0.9 + 0.05 + s).clamp(0.05, 0.95)
    } else {
        // Sweep through critical ~0.59 for square site percolation
        (0.35 + phase_unit(t) * 0.4 + s).clamp(0.05, 0.95)
    }
}

/// The fewest cluster cells joining the left edge to the right edge, as a
/// mask over the grid.
///
/// Breadth-first from every cluster cell on the left column, so the first
/// right-column cell reached closes a shortest crossing. Empty when the
/// cluster does not span.
fn shortest_crossing(cluster: &[bool], w: usize, h: usize) -> Vec<bool> {
    // A grid predecessor needs a direction, not a machine-sized index. The
    // unseen state also owns visitation, avoiding a second full-grid buffer.
    #[derive(Clone, Copy)]
    #[repr(u8)]
    enum Parent {
        Unseen,
        Start,
        Left,
        Right,
        Above,
        Below,
    }
    let mut on_path = vec![false; w * h];
    let mut previous = vec![Parent::Unseen; w * h];
    let mut queue = std::collections::VecDeque::new();
    for y in 0..h {
        let i = y * w;
        if cluster[i] {
            previous[i] = Parent::Start;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        if x == w - 1 {
            let mut at = i;
            loop {
                on_path[at] = true;
                at = match previous[at] {
                    Parent::Start => return on_path,
                    Parent::Left => at - 1,
                    Parent::Right => at + 1,
                    Parent::Above => at - w,
                    Parent::Below => at + w,
                    Parent::Unseen => unreachable!("a queued cell has a predecessor"),
                };
            }
        }
        for (dx, dy, parent) in [
            (1i32, 0, Parent::Left),
            (0, -1, Parent::Below),
            (0, 1, Parent::Above),
            (-1, 0, Parent::Right),
        ] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if cluster[j] && matches!(previous[j], Parent::Unseen) {
                previous[j] = parent;
                queue.push_back(j);
            }
        }
    }
    Vec::new()
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) -> f64 {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return 0.0;
    }
    // Grid tracks the canvas so a drag changes real open density, not a
    // tiny upsampled stamp. Leave a strip for the p meter.
    let meter_h = 2usize;
    let h = height.saturating_sub(meter_h).max(4);
    let w = width.max(8);
    let mut open = vec![false; w * h];
    // Full 0..1 unit randoms (old >>33 / u32::MAX only reached ~0.5, so any
    // p above half opened every site and made high-p frames look identical).
    let mut state = seed ^ 0xC0FF_EE00_D15E_A5E5;
    let mut next_u = || {
        state = state
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0x5851_F42D_4C95_7F2D);
        (state >> 11) as f64 / ((1u64 << 53) as f64)
    };
    for cell in &mut open {
        *cell = next_u() < p;
    }
    // Flood from left edge: connected open cluster
    let mut seen = vec![false; w * h];
    let mut stack = Vec::new();
    for y in 0..h {
        let i = y * w;
        if open[i] {
            stack.push(i);
            seen[i] = true;
        }
    }
    let mut cluster = 0usize;
    while let Some(i) = stack.pop() {
        cluster += 1;
        let x = i % w;
        let y = i / w;
        for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if open[j] && !seen[j] {
                seen[j] = true;
                stack.push(j);
            }
        }
    }
    let mut right_touch = false;
    for y in 0..h {
        if seen[y * w + (w - 1)] {
            right_touch = true;
            break;
        }
    }
    // The open/closed pattern is the picture, so it is drawn as shape rather
    // than as shades of one slab: a closed site draws nothing and leaves the
    // stage showing, an open site is faint, the cluster joined to the left
    // edge is the idea. Once it spans, its shortest crossing runs hot so the
    // path reads as a separate glyph in text and a brighter stroke in pixels.
    let crossing = if right_touch {
        shortest_crossing(&seen, w, h)
    } else {
        Vec::new()
    };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let ch = if seen[i] {
                '*'
            } else if open[i] {
                '.'
            } else {
                continue;
            };
            canvas.plot(x as i32, y as i32, ch);
            if crossing.get(i).copied().unwrap_or(false) {
                canvas.plot(x as i32, y as i32, '#');
            }
        }
    }
    // Open-probability meter: domain consequence even when masks look busy.
    let meter_y = height.saturating_sub(1) as i32;
    let filled = ((p.clamp(0.0, 1.0)) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(0, meter_y, width.saturating_sub(1) as i32, meter_y, '-');
    if filled > 0 {
        canvas.line(0, meter_y, filled, meter_y, '=');
    }
    // Critical tick near square-site p_c ≈ 0.59.
    let pc_x = (0.592_746 * width.saturating_sub(1) as f64).round() as i32;
    canvas.plot(pc_x, meter_y.saturating_sub(1), '|');
    cluster as f64 / (w * h) as f64
}

/// Site percolation room.
#[derive(Debug, Default)]
pub struct Percolation {
    seed: u64,
}

impl Percolation {
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

impl Room for Percolation {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let _ = draw(canvas, open_p(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "percolation",
            root: 164.81,
            tempo: 86,
            line: &[0, 3, 5, 8, 10, 8, 5, 3],
            encodes: "open sites meet a spanning path at criticality",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE OPEN PROB")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = open_p(t, None, self.seed);
        Some(format!("p={p:.2}  perc  DRAG:OPEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = open_p(t, hands.last().copied(), self.seed);
        let _ = draw(canvas, p, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = open_p(t, hands.last().copied(), self.seed);
        // Square site percolation threshold (accepted estimate).
        let pc = 0.592_746_f64;
        let delta = p - pc;
        let side = if delta > 0.02 {
            "above"
        } else if delta < -0.02 {
            "below"
        } else {
            "near"
        };
        Some(format!("p={p:.3}  pc={pc:.3}  {side}"))
    }

    fn reveal(&self) -> &'static str {
        "In site percolation each cell is open with probability p. Below a \
         critical p_c there is no left-right open path; above it, one appears. \
         Square-site p_c is about 0.5927: a sharp phase transition in the plane."
    }
}

#[cfg(test)]
mod tests {
    use super::Percolation;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Percolation::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("OPEN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = Percolation::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn the_crossing_is_a_shortest_left_to_right_path_through_the_cluster() {
        // Two ways across: along the bottom in five cells, or over the top
        // with a detour in seven. The crossing takes the bottom.
        #[rustfmt::skip]
        let cluster = [
            true,  true,  false, true,  true,
            false, true,  true,  true,  false,
            true,  true,  true,  true,  true,
        ];
        let path = super::shortest_crossing(&cluster, 5, 3);
        assert_eq!(path.iter().filter(|cell| **cell).count(), 5);
        assert!(path[10..15].iter().all(|cell| *cell));
        // A cluster that never reaches the right edge has no crossing.
        #[rustfmt::skip]
        let stranded = [
            true, true, false,
            true, false, false,
        ];
        assert!(super::shortest_crossing(&stranded, 3, 2).is_empty());
    }

    #[test]
    fn a_spanning_path_is_distinct_in_text_and_pixels() {
        // At p=1 the shortest crossing is one straight row. The rest of the
        // open grid must remain visible without looking like part of the path.
        let mut canvas = Canvas::new(32, 18);
        super::draw(&mut canvas, 1.0, 0);
        let text = canvas.to_text();
        let rows: Vec<_> = text.lines().collect();
        assert_eq!(rows[0], "#".repeat(32));
        assert_eq!(rows[1], "*".repeat(32));
        assert_eq!(text.chars().filter(|mark| *mark == '#').count(), 32);

        let mut raster = crate::Raster::new(32, 18);
        super::draw(&mut raster, 1.0, 0);
        let pixels = raster.to_rgba();
        let lightness = |y: usize| {
            let i = (y * 32 + 16) * 4;
            crate::dichromacy::lightness([pixels[i], pixels[i + 1], pixels[i + 2]])
        };
        assert!(lightness(0) > lightness(1) + 10.0);
    }

    #[test]
    fn crossing_reconstruction_follows_turns_in_every_direction() {
        let mut cluster = vec![false; 9 * 7];
        let corners = [(0, 4), (2, 4), (2, 6), (6, 6), (6, 2), (4, 2), (4, 0), (8, 0)];
        for segment in corners.windows(2) {
            let [(x0, y0), (x1, y1)] = [segment[0], segment[1]];
            for y in y0.min(y1)..=y0.max(y1) {
                for x in x0.min(x1)..=x0.max(x1) {
                    cluster[y * 9 + x] = true;
                }
            }
        }
        assert_eq!(cluster.iter().filter(|cell| **cell).count(), 21);
        assert_eq!(super::shortest_crossing(&cluster, 9, 7), cluster);
    }

    #[test]
    fn every_small_grid_crossing_matches_independent_distance_relaxation() {
        for mask in 0..512 {
            let cluster: Vec<_> = (0..9).map(|i| mask & (1 << i) != 0).collect();
            let mut distances = [usize::MAX; 9];
            for y in 0..3 {
                if cluster[y * 3] {
                    distances[y * 3] = 0;
                }
            }
            // Bellman-Ford relaxation on the grid is an independent oracle
            // for the queue-based search and its reconstructed path length.
            for _ in 0..9 {
                for i in 0..9 {
                    if !cluster[i] || distances[i] == usize::MAX {
                        continue;
                    }
                    for j in 0..9 {
                        let adjacent = (i / 3 == j / 3 && i.abs_diff(j) == 1)
                            || (i % 3 == j % 3 && i.abs_diff(j) == 3);
                        if adjacent && cluster[j] {
                            distances[j] = distances[j].min(distances[i] + 1);
                        }
                    }
                }
            }
            let distance = [distances[2], distances[5], distances[8]].into_iter().min().unwrap();
            let path = super::shortest_crossing(&cluster, 3, 3);
            if distance == usize::MAX {
                assert!(path.is_empty(), "mask {mask}");
            } else {
                assert_eq!(path.iter().filter(|cell| **cell).count(), distance + 1, "mask {mask}");
                assert!(path.iter().zip(&cluster).all(|(path, open)| !path || *open));
            }
        }
    }

    #[test]
    fn closed_sites_leave_the_stage_and_a_drag_changes_the_lit_shape() {
        // Closed sites draw nothing, so how much of the field is lit is how
        // open it is: the answer to a drag is a change of shape, which a
        // player without color can see.
        let room = Percolation::new();
        let lit = |x: f64| {
            let mut raster = crate::Raster::new(120, 70);
            room.render_poked(&mut raster, 0.35, &[(x, 0.5)]);
            raster.lit_count()
        };
        let (sparse, dense) = (lit(0.1), lit(0.9));
        assert!(sparse > 0 && sparse * 3 < dense, "{sparse} lit at low p, {dense} at high p");
        assert!(dense < 120 * 70, "a closed site must leave the stage showing");
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Percolation::new().render(&mut c, 0.65);
        assert!(c.ink_count() > 0);
    }
}
