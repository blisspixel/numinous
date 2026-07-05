//! An RGBA pixel raster: the image (PNG) surface.
//!
//! Rooms draw into a [`Raster`] through the same [`Surface`] trait they use for
//! ASCII, so one `render` method produces both the terminal view and a real
//! image. Rendering is on the CPU, deterministic, and needs no GPU, so it is
//! fully testable. Marks are drawn additively on a near-black stage, so
//! overlapping strokes glow (see `docs/VISUALS.md`).

use crate::surface::{MAX_DIM, Surface};

/// The near-black background (the Numinous stage).
const BACKGROUND: [u8; 3] = [10, 11, 15];

/// The accent used when a room does not specify one.
const DEFAULT_ACCENT: [u8; 3] = [36, 120, 180];

/// Scale a color by `factor`, clamping each channel to 255.
fn scale(color: [u8; 3], factor: f32) -> [u8; 3] {
    let ch = |c: u8| (f32::from(c) * factor).round().clamp(0.0, 255.0) as u8;
    [ch(color[0]), ch(color[1]), ch(color[2])]
}

/// A fixed-size RGB pixel buffer that rooms draw into, in a room's accent color.
#[derive(Debug, Clone)]
pub struct Raster {
    width: usize,
    height: usize,
    accent: [u8; 3],
    pixels: Vec<[u8; 3]>,
}

impl Raster {
    /// Create a raster filled with the background color, using the default accent.
    ///
    /// Each dimension is clamped to a safe maximum so any request is safe.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_accent(width, height, DEFAULT_ACCENT)
    }

    /// Create a raster that draws in the given accent color.
    #[must_use]
    pub fn with_accent(width: usize, height: usize, accent: [u8; 3]) -> Self {
        let width = width.min(MAX_DIM);
        let height = height.min(MAX_DIM);
        Self {
            width,
            height,
            accent,
            pixels: vec![BACKGROUND; width * height],
        }
    }

    /// The color added for a mark: the accent, a brighter accent for `'#'`, or a
    /// faint structural gray for `'-'`.
    fn ink(&self, mark: char) -> [u8; 3] {
        match mark {
            '#' => scale(self.accent, 1.7),
            '-' => [16, 20, 34],
            _ => self.accent,
        }
    }

    /// The pixels as a tightly packed RGBA byte buffer (`width * height * 4`),
    /// suitable for PNG encoding. Alpha is always fully opaque.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            out.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        out
    }

    /// The number of pixels brighter than the background. Useful for tests.
    #[must_use]
    pub fn lit_count(&self) -> usize {
        self.pixels.iter().filter(|&&p| p != BACKGROUND).count()
    }

    /// Replace this raster's pixels from an RGBA byte buffer (alpha ignored;
    /// extra or missing bytes are tolerated). Brings a post-processed frame,
    /// for example a visual era, back onto a raster.
    pub fn set_rgba(&mut self, rgba: &[u8]) {
        for (pixel, bytes) in self.pixels.iter_mut().zip(rgba.chunks_exact(4)) {
            *pixel = [bytes[0], bytes[1], bytes[2]];
        }
    }

    /// Copy another raster's pixels into this one with its top-left at `(x, y)`,
    /// clipping anything that falls outside. Used to tile rooms into a sheet.
    pub fn blit(&mut self, other: &Raster, x: usize, y: usize) {
        for oy in 0..other.height {
            let ty = y + oy;
            if ty >= self.height {
                break;
            }
            for ox in 0..other.width {
                let tx = x + ox;
                if tx >= self.width {
                    break;
                }
                self.pixels[ty * self.width + tx] = other.pixels[oy * other.width + ox];
            }
        }
    }
}

impl Surface for Raster {
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
            let add = self.ink(mark);
            let pixel = &mut self.pixels[y * self.width + x];
            for i in 0..3 {
                pixel[i] = pixel[i].saturating_add(add[i]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BACKGROUND, Raster};
    use crate::surface::Surface;

    #[test]
    fn new_raster_is_background() {
        let r = Raster::new(4, 4);
        assert_eq!(r.width(), 4);
        assert_eq!(r.lit_count(), 0);
    }

    #[test]
    fn plot_brightens_a_pixel_additively() {
        let mut r = Raster::new(4, 4);
        r.plot(1, 1, '*');
        assert_eq!(r.lit_count(), 1);
        r.plot(1, 1, '*'); // additive: brighter, still one lit pixel
        assert_eq!(r.lit_count(), 1);
    }

    #[test]
    fn plot_clips_out_of_bounds() {
        let mut r = Raster::new(4, 4);
        r.plot(-1, 0, '*');
        r.plot(0, 99, '*');
        assert_eq!(r.lit_count(), 0);
    }

    #[test]
    fn to_rgba_has_four_bytes_per_pixel_and_opaque_alpha() {
        let r = Raster::new(3, 2);
        let bytes = r.to_rgba();
        assert_eq!(bytes.len(), 3 * 2 * 4);
        assert_eq!(bytes[0..3], BACKGROUND);
        assert_eq!(bytes[3], 255);
    }

    #[test]
    fn line_lights_pixels_via_the_shared_bresenham() {
        let mut r = Raster::new(10, 10);
        r.line(0, 0, 9, 9, '#');
        assert!(r.lit_count() >= 10);
    }

    #[test]
    fn with_accent_draws_in_the_given_color() {
        let mut r = Raster::with_accent(2, 2, [200, 0, 0]);
        r.plot(0, 0, '*');
        let bytes = r.to_rgba();
        assert!(bytes[0] > BACKGROUND[0] + 100, "red channel should be lit");
        assert!(
            bytes[2] <= BACKGROUND[2] + 1,
            "blue channel should stay dark"
        );
    }

    #[test]
    fn pixels_have_square_aspect() {
        assert!((Raster::new(4, 4).char_aspect() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn blit_copies_a_tile_and_clips() {
        let mut tile = Raster::new(2, 2);
        tile.plot(0, 0, '*');
        tile.plot(1, 1, '*');
        let mut sheet = Raster::new(4, 4);
        sheet.blit(&tile, 1, 1); // places the two lit pixels at (1,1) and (2,2)
        assert_eq!(sheet.lit_count(), 2);
        sheet.blit(&tile, 3, 3); // partly off the edge: only (3,3) lands
        assert_eq!(sheet.lit_count(), 3);
    }
}
