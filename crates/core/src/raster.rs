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

    /// Dim every pixel to `keep` percent of its brightness, a backdrop for
    /// overlay text so menus stay legible over busy rooms.
    pub fn dim(&mut self, keep: u32) {
        let keep = keep.min(100);
        for pixel in &mut self.pixels {
            for channel in pixel.iter_mut() {
                *channel = ((u32::from(*channel) * keep) / 100) as u8;
            }
        }
    }

    /// Dim only the rows from `y0` to `y1` (clamped): a legibility band
    /// behind HUD text, so words stay readable over bright rooms.
    pub fn dim_rows(&mut self, y0: i32, y1: i32, keep: u32) {
        let keep = keep.min(100);
        let from = y0.max(0) as usize;
        let to = (y1.max(0) as usize).min(self.height);
        for y in from..to {
            for x in 0..self.width {
                let pixel = &mut self.pixels[y * self.width + x];
                for channel in pixel.iter_mut() {
                    *channel = ((u32::from(*channel) * keep) / 100) as u8;
                }
            }
        }
    }

    /// Reset a horizontal band to the stage background.
    ///
    /// This gives dense interface copy a quiet surface instead of asking it to
    /// compete with a bright room. Bounds are clamped to the raster.
    pub fn clear_rows(&mut self, y0: i32, y1: i32) {
        let from = y0.max(0) as usize;
        let to = (y1.max(0) as usize).min(self.height);
        for y in from..to {
            for x in 0..self.width {
                self.pixels[y * self.width + x] = BACKGROUND;
            }
        }
    }

    /// Replace this raster's pixels from an RGBA byte buffer (alpha ignored;
    /// extra or missing bytes are tolerated). Brings a post-processed frame,
    /// for example a visual era, back onto a raster.
    pub fn set_rgba(&mut self, rgba: &[u8]) {
        for (pixel, bytes) in self.pixels.iter_mut().zip(rgba.chunks_exact(4)) {
            *pixel = [bytes[0], bytes[1], bytes[2]];
        }
    }

    /// A `width` x `height` copy of this raster where each source pixel
    /// covers a `factor` x `factor` block (nearest neighbor, never blended).
    ///
    /// The live app view renders heavy rooms below window resolution and
    /// expands them with this before the HUD draws, so interface text stays
    /// window-crisp while only room pixels trade sharpness for motion. The
    /// accent carries over so chrome drawn on the result matches chrome drawn
    /// on a full-resolution render.
    ///
    /// The output is exactly the requested size, whatever the source size:
    /// blocks at the right and bottom edges are partial when the dimensions
    /// are not factor multiples, output beyond the scaled source repeats the
    /// nearest edge pixel, and output smaller than the scaled source is a
    /// top-left crop. Requested dimensions are clamped to the same safe
    /// maximum as every raster; a zero-size source stays background.
    #[must_use]
    pub fn upscaled(&self, factor: usize, width: usize, height: usize) -> Raster {
        let factor = factor.max(1);
        let mut out = Raster::with_accent(width, height, self.accent);
        if self.width == 0 || self.height == 0 {
            return out;
        }
        for y in 0..out.height {
            let sy = (y / factor).min(self.height - 1);
            let row = y * out.width;
            let same_source_row = y > 0 && sy == ((y - 1) / factor).min(self.height - 1);
            if same_source_row {
                out.pixels.copy_within(row - out.width..row, row);
                continue;
            }
            for x in 0..out.width {
                let sx = (x / factor).min(self.width - 1);
                out.pixels[row + x] = self.pixels[sy * self.width + sx];
            }
        }
        out
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
    fn dim_darkens_and_clamps() {
        let mut raster = Raster::new(2, 2);
        raster.plot(0, 0, '#');
        let bright = raster.to_rgba()[0];
        raster.dim(25);
        let dimmed = raster.to_rgba()[0];
        assert!(
            dimmed < bright / 2,
            "should darken hard: {bright} -> {dimmed}"
        );
        raster.dim(150); // clamped to 100: no brightening, no overflow
    }

    #[test]
    fn dim_rows_darkens_only_the_band() {
        let mut raster = Raster::new(2, 4);
        for y in 0..4 {
            raster.plot(0, y, '#');
        }
        let before = raster.to_rgba();
        raster.dim_rows(1, 3, 20);
        let after = raster.to_rgba();
        let px = |buf: &Vec<u8>, y: usize| buf[y * 2 * 4];
        assert_eq!(px(&before, 0), px(&after, 0), "above the band untouched");
        assert!(px(&after, 1) < px(&before, 1), "inside the band darker");
        assert!(px(&after, 2) < px(&before, 2));
        assert_eq!(px(&before, 3), px(&after, 3), "below the band untouched");
        raster.dim_rows(-5, 99, 50); // clamps, never panics
    }

    #[test]
    fn clear_rows_restores_only_the_requested_band() {
        let mut raster = Raster::with_accent(3, 4, [100, 80, 60]);
        for y in 0..4 {
            for x in 0..3 {
                raster.plot(x, y, '#');
            }
        }
        let before = raster.to_rgba();

        raster.clear_rows(1, 3);
        let after = raster.to_rgba();

        assert_eq!(&after[0..12], &before[0..12]);
        assert_eq!(&after[36..48], &before[36..48]);
        for pixel in after[12..36].chunks_exact(4) {
            assert_eq!(pixel, [10, 11, 15, 255]);
        }
    }

    #[test]
    fn upscaled_expands_each_source_pixel_into_a_block() {
        let mut small = Raster::with_accent(2, 2, [200, 40, 40]);
        small.plot(0, 0, '*'); // only the top-left source pixel is lit
        let big = small.upscaled(3, 7, 5);
        assert_eq!(big.width(), 7);
        assert_eq!(big.height(), 5);
        let rgba = big.to_rgba();
        let lit = |x: usize, y: usize| rgba[(y * 7 + x) * 4] > BACKGROUND[0];
        // The lit source pixel covers exactly the 3x3 block at the origin.
        assert!(lit(0, 0) && lit(2, 2), "block interior is lit");
        assert!(!lit(3, 0) && !lit(0, 3), "neighboring blocks stay dark");
        // The partial right/bottom edge repeats the nearest source pixel
        // (source x=1 dark, so the edge is dark) instead of reading out of
        // bounds.
        assert!(!lit(6, 0) && !lit(0, 4));
    }

    #[test]
    fn upscaled_smaller_than_the_scaled_source_is_a_top_left_crop() {
        let mut small = Raster::new(3, 3);
        small.plot(0, 0, '*');
        small.plot(2, 2, '*'); // outside the crop below
        let cropped = small.upscaled(2, 4, 4); // scaled source is 6x6
        assert_eq!(cropped.width(), 4);
        assert_eq!(cropped.height(), 4);
        let rgba = cropped.to_rgba();
        let lit = |x: usize, y: usize| rgba[(y * 4 + x) * 4] > BACKGROUND[0];
        assert!(lit(0, 0) && lit(1, 1), "top-left block survives the crop");
        assert_eq!(
            cropped.lit_count(),
            4,
            "the (2,2) source pixel's block falls wholly outside"
        );
    }

    #[test]
    fn upscaled_factor_one_matches_the_source() {
        let mut small = Raster::new(3, 2);
        small.plot(1, 1, '#');
        let copy = small.upscaled(1, 3, 2);
        assert_eq!(copy.to_rgba(), small.to_rgba());
    }

    #[test]
    fn upscaled_keeps_the_accent_and_survives_degenerate_input() {
        let small = Raster::with_accent(2, 2, [10, 200, 10]);
        let mut big = small.upscaled(0, 4, 4); // factor 0 behaves as 1
        big.plot(0, 0, '*');
        let rgba = big.to_rgba();
        assert!(rgba[1] > 100, "accent green carried to the upscaled raster");
        let empty = Raster::new(0, 3);
        let out = empty.upscaled(2, 4, 4);
        assert_eq!(
            out.lit_count(),
            0,
            "zero-size source upscales to background"
        );
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
