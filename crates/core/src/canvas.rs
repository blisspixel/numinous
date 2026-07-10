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
