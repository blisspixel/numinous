//! A deterministic character canvas for headless (terminal and agent) rendering.
//!
//! Rooms draw into a [`Canvas`], which produces reproducible text output. This is
//! the Teletype face of the render pipeline (see `docs/VISUALS.md`); the GPU
//! renderer will later target the same room logic.

/// A fixed-size grid of characters that rooms draw into.
///
/// Coordinates are column (`x`, left to right) and row (`y`, top to bottom).
/// Drawing is deterministic and out-of-bounds writes are silently clipped, so a
/// room can never panic on geometry.
#[derive(Debug, Clone)]
pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<char>,
}

/// The largest canvas dimension, in cells. ASCII output never needs more, and
/// the cap keeps an absurd request (from a face or an agent) from attempting a
/// multi-gigabyte allocation that would abort the process.
const MAX_DIM: usize = 2048;

impl Canvas {
    /// Create a blank canvas of the given size, filled with spaces.
    ///
    /// Each dimension is clamped to [`MAX_DIM`] so any request is safe.
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

    /// The canvas width in columns.
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    /// The canvas height in rows.
    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Set the character at integer coordinates, clipping if out of bounds.
    pub fn plot(&mut self, x: i32, y: i32, c: char) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = c;
        }
    }

    /// Draw a straight line between two integer points using Bresenham's
    /// algorithm. Endpoints outside the canvas are clipped by [`Canvas::plot`].
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: char) {
        // Step in i64 so extreme coordinates cannot overflow the arithmetic.
        // Callers (the rooms) always pass coordinates within the canvas.
        let (x1i, y1i) = (i64::from(x1), i64::from(y1));
        let dx = (x1i - i64::from(x0)).abs();
        let dy = -(y1i - i64::from(y0)).abs();
        let sx: i64 = if x0 < x1 { 1 } else { -1 };
        let sy: i64 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (i64::from(x0), i64::from(y0));
        loop {
            self.plot(x as i32, y as i32, c);
            if x == x1i && y == y1i {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
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

    /// The number of non-space cells. Useful for tests and simple density checks.
    #[must_use]
    pub fn ink_count(&self) -> usize {
        self.cells.iter().filter(|&&c| c != ' ').count()
    }
}

#[cfg(test)]
mod tests {
    use super::Canvas;

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
        assert!(c.width() <= super::MAX_DIM);
        assert_eq!(c.height(), 3);
    }

    #[test]
    fn line_with_large_but_bounded_coordinates_does_not_overflow() {
        let mut c = Canvas::new(8, 8);
        // Endpoints far outside the canvas are clipped by plot; the i64 stepping
        // must not overflow. Kept within a small span so it terminates quickly.
        c.line(-100, -100, 100, 100, '*');
        assert!(c.ink_count() > 0);
    }
}
