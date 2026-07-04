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

/// The color added per hit for each mark glyph.
fn ink(mark: char) -> [u8; 3] {
    match mark {
        '#' => [80, 200, 255], // highlighted strokes, brighter
        '-' => [18, 22, 38],   // faint structure (for example floor lines)
        _ => [36, 120, 180],   // the default accent
    }
}

/// A fixed-size RGB pixel buffer that rooms draw into.
#[derive(Debug, Clone)]
pub struct Raster {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 3]>,
}

impl Raster {
    /// Create a raster filled with the background color.
    ///
    /// Each dimension is clamped to a safe maximum so any request is safe.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        let width = width.min(MAX_DIM);
        let height = height.min(MAX_DIM);
        Self {
            width,
            height,
            pixels: vec![BACKGROUND; width * height],
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
            let pixel = &mut self.pixels[y * self.width + x];
            let add = ink(mark);
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
}
