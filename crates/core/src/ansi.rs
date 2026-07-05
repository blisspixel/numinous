//! Truecolor terminal rendering: the terminal as a framebuffer.
//!
//! Modern terminals (Windows Terminal, iTerm2, kitty, most Linux emulators)
//! support 24-bit color. Pairing that with the upper-half-block character, whose
//! foreground paints the top half of a cell and background paints the bottom,
//! gives two full-color pixels per character cell. A [`Raster`] becomes a real
//! color image in the terminal, no window required. See `docs/INTERFACES.md`.

use crate::raster::Raster;
use crate::surface::Surface;

/// The upper-half-block character: foreground on top, background below.
const HALF_BLOCK: char = '\u{2580}';

/// An RGB color.
type Rgb = (u8, u8, u8);

/// Encode a raster as truecolor ANSI, two pixels per character cell.
///
/// Each output row covers two pixel rows (the last row pairs with black when the
/// height is odd). Every line ends with a reset so the terminal state is clean.
#[must_use]
pub fn to_ansi(raster: &Raster) -> String {
    let width = raster.width();
    let height = raster.height();
    let rgba = raster.to_rgba();
    let pixel = |x: usize, y: usize| -> Rgb {
        if y >= height {
            return (0, 0, 0);
        }
        let o = (y * width + x) * 4;
        (rgba[o], rgba[o + 1], rgba[o + 2])
    };

    let mut out = String::with_capacity(width * height * 20);
    for row in 0..height.div_ceil(2) {
        let (top_y, bottom_y) = (row * 2, row * 2 + 1);
        // Track the last colors to skip redundant escape codes (smaller frames).
        let mut last: Option<(Rgb, Rgb)> = None;
        for x in 0..width {
            let top = pixel(x, top_y);
            let bottom = pixel(x, bottom_y);
            if last != Some((top, bottom)) {
                out.push_str(&format!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
                    top.0, top.1, top.2, bottom.0, bottom.1, bottom.2
                ));
                last = Some((top, bottom));
            }
            out.push(HALF_BLOCK);
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_ansi;
    use crate::raster::Raster;
    use crate::surface::Surface;

    #[test]
    fn output_has_one_line_per_two_pixel_rows() {
        let raster = Raster::new(8, 6);
        assert_eq!(to_ansi(&raster).lines().count(), 3);
        let odd = Raster::new(8, 5);
        assert_eq!(to_ansi(&odd).lines().count(), 3); // last row pads with black
    }

    #[test]
    fn lit_pixels_change_the_colors() {
        let mut raster = Raster::new(4, 4);
        let plain = to_ansi(&raster);
        raster.plot(1, 1, '#');
        let lit = to_ansi(&raster);
        assert_ne!(plain, lit);
        assert!(lit.contains("\x1b[38;2;"), "has truecolor escapes");
    }

    #[test]
    fn every_line_resets_the_terminal() {
        let raster = Raster::new(6, 4);
        for line in to_ansi(&raster).lines() {
            assert!(line.ends_with("\x1b[0m"));
        }
    }

    #[test]
    fn encoding_is_deterministic() {
        let mut a = Raster::new(10, 10);
        let mut b = Raster::new(10, 10);
        a.plot(3, 3, '#');
        b.plot(3, 3, '#');
        assert_eq!(to_ansi(&a), to_ansi(&b));
    }
}
