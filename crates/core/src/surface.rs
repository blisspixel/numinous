//! A drawing-surface abstraction.
//!
//! A room draws in integer coordinates through [`Surface`], so the same room can
//! render to ASCII (`Canvas`), to an RGBA pixel image (`Raster`), and later to
//! the GPU, all from one `render` method. The line drawing lives here once
//! (Bresenham), so every surface inherits it. See `docs/ARCHITECTURE.md`.

/// The largest surface dimension, in cells or pixels. Shared cap so an absurd
/// size request cannot attempt a process-aborting allocation.
pub(crate) const MAX_DIM: usize = 4096;

/// Above this coordinate span a line is clipped to the surface before it is
/// rasterized (see [`Surface::line`]). It is far larger than any real line
/// (which spans at most a surface plus a small poke offset, so a few thousand
/// cells), so every genuine line is left exactly as before; only a pathological,
/// saturated segment is clipped, and there a one-cell rounding at the clip edge
/// is invisible and unreachable.
const LINE_CLIP_SPAN: i64 = 4 * MAX_DIM as i64;

/// The region-outside bits for [`clip_segment`] (Cohen-Sutherland): left, right,
/// below, above.
fn outcode(x: i64, y: i64, w: i64, h: i64) -> u8 {
    let mut code = 0;
    if x < 0 {
        code |= 1;
    } else if x >= w {
        code |= 2;
    }
    if y < 0 {
        code |= 4;
    } else if y >= h {
        code |= 8;
    }
    code
}

/// Clip a segment to the surface rectangle `[0, w) x [0, h)` with the
/// Cohen-Sutherland algorithm, returning the clipped integer endpoints or `None`
/// when the segment misses the surface entirely. Used only for pathological
/// spans, so integer truncation at the clip edge is acceptable. The loop is
/// capped so truncation can never spin it.
fn clip_segment(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    w: usize,
    h: usize,
) -> Option<(i32, i32, i32, i32)> {
    if w == 0 || h == 0 {
        return None;
    }
    let (w, h) = (w as i64, h as i64);
    let (mut x0, mut y0, mut x1, mut y1) =
        (i64::from(x0), i64::from(y0), i64::from(x1), i64::from(y1));
    let mut c0 = outcode(x0, y0, w, h);
    let mut c1 = outcode(x1, y1, w, h);
    // Four edges, two endpoints: the classic bound is four clips; a couple more
    // absorbs any integer-truncation wobble before giving up.
    for _ in 0..8 {
        if c0 & c1 != 0 {
            return None; // both endpoints share an outside half-plane
        }
        if c0 | c1 == 0 {
            return Some((x0 as i32, y0 as i32, x1 as i32, y1 as i32));
        }
        let outside = if c0 != 0 { c0 } else { c1 };
        let (x, y) = if outside & 8 != 0 {
            let edge = h - 1;
            (x0 + (x1 - x0) * (edge - y0) / (y1 - y0), edge)
        } else if outside & 4 != 0 {
            (x0 + (x1 - x0) * (0 - y0) / (y1 - y0), 0)
        } else if outside & 2 != 0 {
            let edge = w - 1;
            (edge, y0 + (y1 - y0) * (edge - x0) / (x1 - x0))
        } else {
            (0, y0 + (y1 - y0) * (0 - x0) / (x1 - x0))
        };
        if outside == c0 {
            x0 = x;
            y0 = y;
            c0 = outcode(x0, y0, w, h);
        } else {
            x1 = x;
            y1 = y;
            c1 = outcode(x1, y1, w, h);
        }
    }
    None
}

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

    /// The vertical scale a room should apply to keep a round shape round.
    ///
    /// Pixel surfaces return `1.0`; the ASCII `Canvas` returns `0.5` because a
    /// text character is about twice as tall as it is wide. A room drawing a
    /// circle multiplies its y extent by this instead of hardcoding a factor.
    fn char_aspect(&self) -> f64 {
        1.0
    }

    /// [`Surface::char_aspect`], guarded for hostile implementations: a
    /// non-finite or non-positive aspect falls back to the terminal's 0.5,
    /// so no room can be driven into degenerate geometry by its surface.
    fn safe_char_aspect(&self) -> f64 {
        let aspect = self.char_aspect();
        if aspect.is_finite() && aspect > 0.0 {
            aspect
        } else {
            0.5
        }
    }

    /// Mark a single point, clipping if out of bounds.
    fn plot(&mut self, x: i32, y: i32, mark: char);

    /// Draw a line between two points with Bresenham's algorithm, clipping out
    /// of bounds. Steps in `i64` so extreme coordinates cannot overflow.
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, mark: char) {
        // Bound the work. A Bresenham loop runs for the coordinate span, not the
        // surface size, so a saturated segment (billions of cells, almost none of
        // it visible) would stall for minutes. When the span dwarfs any real
        // line, clip to the surface first; genuine lines are below the bound and
        // are rasterized exactly as before.
        let (x0, y0, x1, y1) = {
            let span = (i64::from(x1) - i64::from(x0))
                .abs()
                .max((i64::from(y1) - i64::from(y0)).abs());
            if span > LINE_CLIP_SPAN {
                match clip_segment(x0, y0, x1, y1, self.width(), self.height()) {
                    Some(clipped) => clipped,
                    None => return,
                }
            } else {
                (x0, y0, x1, y1)
            }
        };
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

#[cfg(test)]
mod tests {
    use super::{Surface, clip_segment};

    /// A surface that only counts how many points a draw touched, so a line's
    /// work can be measured without a real canvas.
    struct Counter {
        w: usize,
        h: usize,
        plots: usize,
    }
    impl Surface for Counter {
        fn width(&self) -> usize {
            self.w
        }
        fn height(&self) -> usize {
            self.h
        }
        fn plot(&mut self, _x: i32, _y: i32, _mark: char) {
            self.plots += 1;
        }
    }

    #[test]
    fn a_saturated_line_does_bounded_work_not_billions_of_steps() {
        // The whole point of the clip: a coordinate-span Bresenham loop over
        // i32::MAX would run ~2 billion plots; clipped to a 10x10 surface it is a
        // handful. This must return effectively instantly.
        let mut s = Counter {
            w: 10,
            h: 10,
            plots: 0,
        };
        s.line(i32::MAX, i32::MAX, 0, 0, '*');
        assert!(
            s.plots <= 64,
            "expected bounded work, got {} plots",
            s.plots
        );
        assert!(s.plots > 0, "the visible corner should still be drawn");
    }

    #[test]
    fn a_normal_line_is_rasterized_exactly_as_before() {
        // A line whose span is below the clip gate is untouched: a 20-cell
        // diagonal draws 20 points, the same Bresenham output as without the gate.
        let mut s = Counter {
            w: 20,
            h: 20,
            plots: 0,
        };
        s.line(0, 0, 19, 19, '*');
        assert_eq!(s.plots, 20);
    }

    #[test]
    fn clip_segment_rejects_a_fully_outside_segment() {
        assert_eq!(clip_segment(-100, -100, -50, -50, 10, 10), None);
    }

    #[test]
    fn clip_segment_keeps_a_fully_inside_segment_unchanged() {
        assert_eq!(clip_segment(1, 1, 8, 8, 10, 10), Some((1, 1, 8, 8)));
    }

    #[test]
    fn clip_segment_clips_one_end_to_the_surface_edge() {
        // From inside (5,5) straight down to far outside: clips to the last row.
        let (_, _, _, y1) = clip_segment(5, 5, 5, 10_000, 10, 10).expect("crosses the surface");
        assert_eq!(y1, 9, "clipped to the last valid row");
    }

    #[test]
    fn clip_segment_is_empty_for_a_zero_size_surface() {
        assert_eq!(clip_segment(0, 0, 5, 5, 0, 0), None);
    }
}
