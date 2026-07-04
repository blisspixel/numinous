//! A drawing-surface abstraction.
//!
//! A room draws in integer coordinates through [`Surface`], so the same room can
//! render to ASCII (`Canvas`), to an RGBA pixel image (`Raster`), and later to
//! the GPU, all from one `render` method. The line drawing lives here once
//! (Bresenham), so every surface inherits it. See `docs/ARCHITECTURE.md`.

/// The largest surface dimension, in cells or pixels. Shared cap so an absurd
/// size request cannot attempt a process-aborting allocation.
pub(crate) const MAX_DIM: usize = 2048;

/// A target a room can draw into, in integer (x, y) coordinates.
///
/// `mark` is an ASCII glyph (for example `'*'`, `'#'`, or `'-'`); pixel surfaces
/// map it to a color. Out-of-bounds drawing is silently clipped, so a room can
/// never panic on geometry.
pub trait Surface {
    /// Width in cells or pixels.
    fn width(&self) -> usize;

    /// Height in cells or pixels.
    fn height(&self) -> usize;

    /// Mark a single point, clipping if out of bounds.
    fn plot(&mut self, x: i32, y: i32, mark: char);

    /// Draw a line between two points with Bresenham's algorithm, clipping out
    /// of bounds. Steps in `i64` so extreme coordinates cannot overflow.
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, mark: char) {
        let (x1i, y1i) = (i64::from(x1), i64::from(y1));
        let dx = (x1i - i64::from(x0)).abs();
        let dy = -(y1i - i64::from(y0)).abs();
        let sx: i64 = if x0 < x1 { 1 } else { -1 };
        let sy: i64 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (i64::from(x0), i64::from(y0));
        loop {
            self.plot(x as i32, y as i32, mark);
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
}
