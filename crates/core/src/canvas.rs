//! A deterministic character canvas: the ASCII (terminal and agent) surface.
//!
//! Rooms draw into a [`Canvas`] through the [`Surface`] trait, producing
//! reproducible text output. This is the Teletype face of the render pipeline
//! (see `docs/VISUALS.md`); the same room logic also renders to a pixel
//! [`crate::raster::Raster`].

use crate::surface::{MAX_DIM, Surface};

/// A fixed-size grid of characters that rooms draw into.
///
/// Coordinates are column (`x`, left to right) and row (`y`, top to bottom).
/// Drawing is deterministic and out-of-bounds writes are silently clipped.
#[derive(Debug, Clone)]
pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<char>,
}

impl Canvas {
    /// Create a blank canvas of the given size, filled with spaces.
    ///
    /// Each dimension is clamped to a safe maximum so any request is safe.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.min(MAX_DIM);
        let height = height.min(MAX_DIM);
        Self {
            width,
            height,
            cells: vec![' '; width * height],
        }
    }

    /// Render the canvas as text: one row per line, trailing spaces trimmed.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::with_capacity((self.width + 1) * self.height);
        for row in 0..self.height {
            let start = row * self.width;
            let line: String = self.cells[start..start + self.width].iter().collect();
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }

    /// The number of non-space cells. Useful for tests and density checks.
    #[must_use]
    pub fn ink_count(&self) -> usize {
        self.cells.iter().filter(|&&c| c != ' ').count()
    }

    /// The character at cell (`x`, `y`), or `None` outside the canvas.
    #[must_use]
    pub fn cell(&self, x: usize, y: usize) -> Option<char> {
        (x < self.width && y < self.height).then(|| self.cells[y * self.width + x])
    }

    /// The structured cell-level difference against another render.
    ///
    /// This is the agent faces' proof-of-touch: rendering a room with and
    /// without hand points and diffing the two frames tells an agent exactly
    /// how much the math answered, as numbers it can verify and optimize
    /// rather than prose it must trust. Returns `None` when the canvases
    /// have different dimensions, because there is no meaningful cell map.
    #[must_use]
    pub fn delta(&self, other: &Canvas) -> Option<RenderDelta> {
        if self.width != other.width || self.height != other.height {
            return None;
        }
        let mut delta = RenderDelta {
            total_cells: self.cells.len(),
            ..RenderDelta::default()
        };
        let mut bounds: Option<(usize, usize, usize, usize)> = None;
        for (index, (&base, &new)) in self.cells.iter().zip(&other.cells).enumerate() {
            if base == new {
                continue;
            }
            delta.cells_changed += 1;
            match (base == ' ', new == ' ') {
                (true, false) => delta.ink_added += 1,
                (false, true) => delta.ink_removed += 1,
                _ => delta.ink_reshaped += 1,
            }
            let (x, y) = (index % self.width, index / self.width);
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
        delta.changed_region = bounds;
        Some(delta)
    }

    /// What held still across several renders of the same room.
    ///
    /// Returns `None` for fewer than two looks, or when the renders do not
    /// share one cell map, because there is nothing to hold still across.
    #[must_use]
    pub fn invariant(looks: &[Self]) -> Option<RenderInvariant> {
        let (first, rest) = looks.split_first()?;
        if rest.is_empty()
            || rest
                .iter()
                .any(|look| look.width != first.width || look.height != first.height)
        {
            return None;
        }
        let mut invariant = RenderInvariant {
            looks: looks.len(),
            total_cells: first.cells.len(),
            ..RenderInvariant::default()
        };
        let mut bounds: Option<(usize, usize, usize, usize)> = None;
        let mut never_ink = vec![false; first.cells.len()];
        let mut always_ink = vec![false; first.cells.len()];
        for (index, &base) in first.cells.iter().enumerate() {
            let unchanged = rest.iter().all(|look| look.cells[index] == base);
            never_ink[index] = base == ' ' && rest.iter().all(|look| look.cells[index] == ' ');
            always_ink[index] = base != ' ' && rest.iter().all(|look| look.cells[index] != ' ');
            let (x, y) = (index % first.width, index / first.width);
            if unchanged {
                invariant.unchanged_cells += 1;
            } else {
                bounds = Some(match bounds {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
            if never_ink[index] {
                invariant.never_ink += 1;
            }
            if always_ink[index] {
                invariant.always_ink += 1;
            }
        }
        invariant.changed_region = bounds;
        let (width, height) = (first.width, first.height);
        for (index, _) in never_ink.iter().enumerate().filter(|(_, blank)| **blank) {
            let (x, y) = (index % width, index / width);
            if let Some((x0, y0, x1, y1)) = bounds
                && x >= x0
                && x <= x1
                && y >= y0
                && y <= y1
            {
                invariant.never_ink_in_changed_region += 1;
            }
            if x == 0 || y == 0 || x + 1 >= width || y + 1 >= height {
                continue;
            }
            let ringed = [index - 1, index + 1, index - width, index + width]
                .into_iter()
                .all(|neighbour| always_ink[neighbour]);
            if ringed {
                invariant.never_ink_enclosed += 1;
            }
        }
        Some(invariant)
    }
}

/// What held still across several renders of the same room.
///
/// [`RenderDelta`] answers what a moment changed. This answers the opposite
/// question, the one a player asks by staying: across all of these looks, what
/// refused to move. A player who keeps returning to one dark point is making a
/// measurement, and this is that measurement rather than a story about it.
///
/// `never_ink` counts cells blank in every look and is always a subset of
/// `unchanged_cells`. `always_ink` counts cells inked in every look and is not,
/// because a cell may swap one glyph for another and still never go dark.
///
/// These are measurements of the rendered character grid, not of the underlying
/// mathematics. A coarse grid stipples a lit region, so a genuine hole in a
/// room's light may not survive as a blank cell ringed by never-dark
/// neighbours. Read a positive `never_ink_enclosed` as real and a zero as
/// unproven rather than as evidence that no such point exists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderInvariant {
    /// How many renders were compared.
    pub looks: usize,
    /// Total cells compared (width times height).
    pub total_cells: usize,
    /// Cells whose character is identical in every look.
    pub unchanged_cells: usize,
    /// Cells blank in every look.
    pub never_ink: usize,
    /// Cells carrying a glyph in every look, glyph changes included.
    pub always_ink: usize,
    /// Inclusive bounding box of every cell that moved at least once, or
    /// `None` when nothing moved at all.
    pub changed_region: Option<(usize, usize, usize, usize)>,
    /// Cells that stayed blank inside `changed_region`.
    pub never_ink_in_changed_region: usize,
    /// Cells that stayed blank while every orthogonal neighbour carried ink in
    /// every look: a hole in the light rather than the dark outside the shape.
    ///
    /// Stricter than `never_ink_in_changed_region`, whose bounding box also
    /// swallows the blank margin around a figure. Border cells never qualify,
    /// because their ring is incomplete. A positive count is a real hole in the
    /// drawn light; a zero is not proof that the room has none, because a
    /// character grid can stipple a lit region until no cell is fully ringed.
    pub never_ink_enclosed: usize,
}

/// The cell-level difference between two equally-sized [`Canvas`] renders.
///
/// `cells_changed` always equals `ink_added + ink_removed + ink_reshaped`.
/// The `changed_region` is the inclusive bounding box `(x0, y0, x1, y1)` of
/// every changed cell, or `None` when the renders are identical.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderDelta {
    /// Cells whose character differs between the two renders.
    pub cells_changed: usize,
    /// Blank cells that gained a glyph.
    pub ink_added: usize,
    /// Glyph cells that went blank.
    pub ink_removed: usize,
    /// Cells that swapped one glyph for another.
    pub ink_reshaped: usize,
    /// Total cells compared (width times height).
    pub total_cells: usize,
    /// Inclusive bounding box of the change, or `None` for identical frames.
    pub changed_region: Option<(usize, usize, usize, usize)>,
}

impl Surface for Canvas {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn char_aspect(&self) -> f64 {
        // Terminal characters are about twice as tall as wide.
        0.5
    }

    fn plot(&mut self, x: i32, y: i32, mark: char) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = mark;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Canvas;
    use crate::surface::Surface;

    #[test]
    fn new_canvas_is_blank() {
        let c = Canvas::new(10, 4);
        assert_eq!(c.width(), 10);
        assert_eq!(c.height(), 4);
        assert_eq!(c.ink_count(), 0);
    }

    #[test]
    fn plot_sets_a_cell_and_clips_out_of_bounds() {
        let mut c = Canvas::new(5, 5);
        c.plot(2, 2, '*');
        c.plot(-1, 0, '*'); // clipped
        c.plot(0, 99, '*'); // clipped
        assert_eq!(c.ink_count(), 1);
    }

    #[test]
    fn line_draws_endpoints() {
        let mut c = Canvas::new(9, 9);
        c.line(0, 0, 8, 0, '#');
        assert_eq!(c.ink_count(), 9);
        let text = c.to_text();
        assert!(text.starts_with("#########"));
    }

    #[test]
    fn rendering_is_deterministic() {
        let mut a = Canvas::new(20, 10);
        let mut b = Canvas::new(20, 10);
        for canvas in [&mut a, &mut b] {
            canvas.line(1, 1, 18, 8, '*');
        }
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn new_clamps_oversized_dimensions() {
        let c = Canvas::new(usize::MAX, 3);
        assert!(c.width() <= crate::surface::MAX_DIM);
        assert_eq!(c.height(), 3);
    }

    #[test]
    fn line_with_large_but_bounded_coordinates_does_not_overflow() {
        let mut c = Canvas::new(8, 8);
        c.line(-100, -100, 100, 100, '*');
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn characters_are_tall_so_aspect_is_one_half() {
        assert!((Canvas::new(4, 4).char_aspect() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn delta_of_identical_frames_is_empty() {
        let mut a = Canvas::new(10, 5);
        a.plot(3, 2, '*');
        let d = a.delta(&a.clone()).expect("same dimensions");
        assert_eq!(d.cells_changed, 0);
        assert_eq!(d.total_cells, 50);
        assert_eq!(d.changed_region, None);
    }

    #[test]
    fn an_invariant_needs_at_least_two_matching_looks() {
        let a = Canvas::new(10, 5);
        assert_eq!(Canvas::invariant(&[]), None);
        assert_eq!(Canvas::invariant(std::slice::from_ref(&a)), None);
        assert_eq!(Canvas::invariant(&[a.clone(), Canvas::new(10, 6)]), None);
    }

    #[test]
    fn an_unmoving_room_holds_every_cell() {
        let mut a = Canvas::new(10, 5);
        a.plot(3, 2, '*');
        let held = Canvas::invariant(&[a.clone(), a.clone(), a]).expect("three matching looks");
        assert_eq!(held.looks, 3);
        assert_eq!(held.total_cells, 50);
        assert_eq!(held.unchanged_cells, 50);
        assert_eq!(held.never_ink, 49);
        assert_eq!(held.always_ink, 1);
        assert_eq!(held.changed_region, None);
        assert_eq!(held.never_ink_in_changed_region, 0);
    }

    #[test]
    fn a_dark_point_inside_the_motion_is_the_measurement() {
        // The Unlit Room in miniature: a band that lights differently on every
        // look, with one cell inside it that never lights at all. Staying is
        // what turns that from a suspicion into a count.
        let mut looks = Vec::new();
        for step in 0..4 {
            let mut canvas = Canvas::new(10, 5);
            for x in 2..8 {
                if x == 5 {
                    continue; // the point that never lights
                }
                canvas.plot(x, 2, if (x + step) % 2 == 0 { '*' } else { '+' });
            }
            looks.push(canvas);
        }
        let held = Canvas::invariant(&looks).expect("four matching looks");
        assert_eq!(held.looks, 4);
        // The lit band reshapes every look, so only the dark cell and the
        // untouched surroundings held still.
        assert_eq!(held.always_ink, 5);
        assert_eq!(held.changed_region, Some((2, 2, 7, 2)));
        assert_eq!(
            held.never_ink_in_changed_region, 1,
            "exactly one cell stayed dark inside the region that moved"
        );
        assert!(
            held.never_ink >= held.never_ink_in_changed_region,
            "the box can only ever hold a subset of the blank cells"
        );
    }

    #[test]
    fn an_enclosed_dark_cell_needs_a_complete_ring_that_never_goes_dark() {
        // A blank cell ringed by cells that were never once dark, with the ring
        // reshaping so the hole is not merely a still frame.
        let mut looks = Vec::new();
        for step in 0..3 {
            let mut canvas = Canvas::new(9, 9);
            let glyph = if step % 2 == 0 { '*' } else { '+' };
            for (x, y) in [(3, 4), (5, 4), (4, 3), (4, 5)] {
                canvas.plot(x, y, glyph);
            }
            looks.push(canvas);
        }
        let held = Canvas::invariant(&looks).expect("three matching looks");
        assert_eq!(held.never_ink_enclosed, 1, "{held:?}");

        // Break one cell of the ring on a single look and the hole is no
        // longer established, because that neighbour did go dark.
        let mut broken = looks.clone();
        broken[1] = Canvas::new(9, 9);
        broken[1].plot(3, 4, '+');
        broken[1].plot(5, 4, '+');
        broken[1].plot(4, 3, '+');
        let held = Canvas::invariant(&broken).expect("three matching looks");
        assert_eq!(held.never_ink_enclosed, 0, "{held:?}");
    }

    #[test]
    fn a_border_cell_can_never_be_enclosed() {
        let mut looks = Vec::new();
        for _ in 0..2 {
            let mut canvas = Canvas::new(5, 5);
            // Ring every neighbour a corner could have.
            canvas.plot(1, 0, '#');
            canvas.plot(0, 1, '#');
            looks.push(canvas);
        }
        let held = Canvas::invariant(&looks).expect("two matching looks");
        assert_eq!(
            held.never_ink_enclosed, 0,
            "an edge cell has no complete ring: {held:?}"
        );
    }

    #[test]
    fn never_ink_is_always_a_subset_of_what_held_still() {
        let mut looks = Vec::new();
        for step in 0..3 {
            let mut canvas = Canvas::new(12, 6);
            canvas.plot(1, 1, '#'); // never moves
            canvas.plot(4 + step, 3, '*'); // walks
            looks.push(canvas);
        }
        let held = Canvas::invariant(&looks).expect("three matching looks");
        assert!(
            held.never_ink <= held.unchanged_cells,
            "a cell blank in every look cannot have changed: {held:?}"
        );
        assert!(
            held.unchanged_cells <= held.never_ink + held.always_ink,
            "an unchanged cell is either always blank or always inked: {held:?}"
        );
        assert_eq!(held.total_cells, 72);
    }

    #[test]
    fn two_looks_agree_with_the_delta_of_the_same_pair() {
        // The invariant is the delta's dual, so on two frames they must
        // partition the same cell map rather than tell different stories.
        let mut base = Canvas::new(10, 5);
        base.plot(1, 1, '#');
        base.plot(2, 2, '#');
        let mut next = Canvas::new(10, 5);
        next.plot(2, 2, '*');
        next.plot(7, 4, '+');
        let delta = base.delta(&next).expect("same dimensions");
        let held = Canvas::invariant(&[base, next]).expect("two matching looks");
        assert_eq!(
            held.unchanged_cells + delta.cells_changed,
            held.total_cells,
            "every cell either moved or held still"
        );
        assert_eq!(held.changed_region, delta.changed_region);
    }

    #[test]
    fn delta_classifies_added_removed_and_reshaped_ink() {
        let mut base = Canvas::new(10, 5);
        base.plot(1, 1, '#'); // will go blank: removed
        base.plot(2, 2, '#'); // will become '*': reshaped
        let mut new = Canvas::new(10, 5);
        new.plot(2, 2, '*');
        new.plot(7, 4, '+'); // blank in base: added
        let d = base.delta(&new).expect("same dimensions");
        assert_eq!(d.ink_added, 1);
        assert_eq!(d.ink_removed, 1);
        assert_eq!(d.ink_reshaped, 1);
        assert_eq!(
            d.cells_changed,
            d.ink_added + d.ink_removed + d.ink_reshaped,
            "the change count invariant must hold"
        );
    }

    #[test]
    fn delta_bounding_box_spans_every_changed_cell_inclusively() {
        let base = Canvas::new(10, 5);
        let mut new = Canvas::new(10, 5);
        new.plot(2, 1, '*');
        new.plot(8, 3, '*');
        let d = base.delta(&new).expect("same dimensions");
        assert_eq!(d.changed_region, Some((2, 1, 8, 3)));
    }

    #[test]
    fn delta_of_mismatched_dimensions_is_none() {
        assert!(Canvas::new(10, 5).delta(&Canvas::new(10, 6)).is_none());
        assert!(Canvas::new(9, 5).delta(&Canvas::new(10, 5)).is_none());
    }

    #[test]
    fn delta_is_symmetric_in_count_and_region_but_swaps_direction() {
        let mut base = Canvas::new(6, 3);
        base.plot(1, 1, '#');
        let mut new = Canvas::new(6, 3);
        new.plot(4, 2, '*');
        let forward = base.delta(&new).expect("same dimensions");
        let backward = new.delta(&base).expect("same dimensions");
        assert_eq!(forward.cells_changed, backward.cells_changed);
        assert_eq!(forward.changed_region, backward.changed_region);
        assert_eq!(forward.ink_added, backward.ink_removed);
        assert_eq!(forward.ink_removed, backward.ink_added);
    }
}
