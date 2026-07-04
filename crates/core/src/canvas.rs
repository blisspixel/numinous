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

impl Canvas {
    /// Create a blank canvas of the given size, filled with spaces.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![' '; width.saturating_mul(height)],
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
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.plot(x, y, c);
            if x == x1 && y == y1 {
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
}
