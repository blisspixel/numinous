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
}

impl Surface for Canvas {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
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
}
