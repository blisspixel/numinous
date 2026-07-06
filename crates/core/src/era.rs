//! Visual Eras: the same math, rendered as its own history.
//!
//! The lore says the transmission arrived as phosphor text, then pixels, then
//! vectors, and only now renders in full color (see `docs/LORE.md`); the design
//! bible makes that a real progression (see `docs/DESIGN.md`). Each era is a
//! pure transform over an RGBA frame, so every face (window, terminal color,
//! PNG) can show any era, deterministically.

/// A rendering era. `Modern` is the identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Era {
    /// Green phosphor terminal: luminance on P1 glass.
    Phosphor,
    /// 8-bit: a fixed 16-color palette and chunky 2x2 pixels.
    EightBit,
    /// Vector scope: bright lines on pure black, dim light culled.
    Vector,
    /// Today: full color, untouched.
    #[default]
    Modern,
}

/// The classic 16-color palette the 8-bit era snaps to.
const PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [0, 0, 170],
    [0, 170, 0],
    [0, 170, 170],
    [170, 0, 0],
    [170, 0, 170],
    [170, 85, 0],
    [170, 170, 170],
    [85, 85, 85],
    [85, 85, 255],
    [85, 255, 85],
    [85, 255, 255],
    [255, 85, 85],
    [255, 85, 255],
    [255, 255, 85],
    [255, 255, 255],
];

impl Era {
    /// Every era, in historical order.
    pub const ALL: [Era; 4] = [Era::Phosphor, Era::EightBit, Era::Vector, Era::Modern];

    /// The era's display name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Era::Phosphor => "Phosphor",
            Era::EightBit => "8-bit",
            Era::Vector => "Vector",
            Era::Modern => "Modern",
        }
    }

    /// Parse an era name (as typed on a command line).
    #[must_use]
    pub fn parse(name: &str) -> Option<Era> {
        match name.trim().to_ascii_lowercase().as_str() {
            "phosphor" | "terminal" => Some(Era::Phosphor),
            "8bit" | "8-bit" | "eightbit" => Some(Era::EightBit),
            "vector" | "scope" => Some(Era::Vector),
            "modern" | "now" => Some(Era::Modern),
            _ => None,
        }
    }

    /// The next era in the cycle (wrapping), for a toggle key.
    #[must_use]
    pub fn next(self) -> Era {
        match self {
            Era::Phosphor => Era::EightBit,
            Era::EightBit => Era::Vector,
            Era::Vector => Era::Modern,
            Era::Modern => Era::Phosphor,
        }
    }

    /// Apply the era to an RGBA frame in place (`width * height * 4` bytes).
    pub fn apply(self, rgba: &mut [u8], width: usize, height: usize) {
        match self {
            Era::Modern => {}
            Era::Phosphor => {
                for pixel in rgba.chunks_exact_mut(4) {
                    let lum = luminance(pixel);
                    pixel[0] = (lum / 6).min(255) as u8;
                    pixel[1] = lum.min(255) as u8;
                    pixel[2] = (lum / 4).min(255) as u8;
                }
                // The glass: every third scanline sits darker, like the tube.
                for y in (0..height).step_by(3) {
                    for x in 0..width {
                        let o = (y * width + x) * 4;
                        if o + 2 < rgba.len() {
                            rgba[o] = (u16::from(rgba[o]) * 6 / 10) as u8;
                            rgba[o + 1] = (u16::from(rgba[o + 1]) * 6 / 10) as u8;
                            rgba[o + 2] = (u16::from(rgba[o + 2]) * 6 / 10) as u8;
                        }
                    }
                }
            }
            Era::EightBit => {
                pixelate(rgba, width, height, 2);
                for pixel in rgba.chunks_exact_mut(4) {
                    let snapped = nearest_palette(pixel[0], pixel[1], pixel[2]);
                    pixel[0] = snapped[0];
                    pixel[1] = snapped[1];
                    pixel[2] = snapped[2];
                }
            }
            Era::Vector => {
                for pixel in rgba.chunks_exact_mut(4) {
                    let lum = luminance(pixel);
                    if lum < 40 {
                        // The scope shows nothing but the beam.
                        pixel[0] = 0;
                        pixel[1] = 0;
                        pixel[2] = 0;
                    } else {
                        // The beam burns bright.
                        pixel[0] = boost(pixel[0]);
                        pixel[1] = boost(pixel[1]);
                        pixel[2] = boost(pixel[2]);
                    }
                }
            }
        }
    }
}

/// Integer luminance of an RGBA pixel (0..=255 scale).
fn luminance(pixel: &[u8]) -> u32 {
    (u32::from(pixel[0]) * 299 + u32::from(pixel[1]) * 587 + u32::from(pixel[2]) * 114) / 1000
}

/// Brighten a channel toward full, preserving hue ratios roughly.
fn boost(value: u8) -> u8 {
    let boosted = u32::from(value) * 3 / 2 + 40;
    boosted.min(255) as u8
}

/// Snap to the nearest palette color by squared distance.
fn nearest_palette(r: u8, g: u8, b: u8) -> [u8; 3] {
    let mut best = PALETTE[0];
    let mut best_d = u32::MAX;
    for color in PALETTE {
        let dr = i32::from(r) - i32::from(color[0]);
        let dg = i32::from(g) - i32::from(color[1]);
        let db = i32::from(b) - i32::from(color[2]);
        let d = (dr * dr + dg * dg + db * db) as u32;
        if d < best_d {
            best_d = d;
            best = color;
        }
    }
    best
}

/// Average `block`-sized cells so pixels get chunky.
fn pixelate(rgba: &mut [u8], width: usize, height: usize, block: usize) {
    if block < 2 || width == 0 || height == 0 || rgba.len() < width * height * 4 {
        return;
    }
    for by in (0..height).step_by(block) {
        for bx in (0..width).step_by(block) {
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for y in by..(by + block).min(height) {
                for x in bx..(bx + block).min(width) {
                    let o = (y * width + x) * 4;
                    r += u32::from(rgba[o]);
                    g += u32::from(rgba[o + 1]);
                    b += u32::from(rgba[o + 2]);
                    n += 1;
                }
            }
            let (r, g, b) = ((r / n) as u8, (g / n) as u8, (b / n) as u8);
            for y in by..(by + block).min(height) {
                for x in bx..(bx + block).min(width) {
                    let o = (y * width + x) * 4;
                    rgba[o] = r;
                    rgba[o + 1] = g;
                    rgba[o + 2] = b;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Era, PALETTE, nearest_palette};

    fn frame(width: usize, height: usize) -> Vec<u8> {
        // A gradient test frame with alpha 255.
        let mut rgba = Vec::with_capacity(width * height * 4);
        for i in 0..width * height {
            rgba.extend_from_slice(&[(i * 7 % 256) as u8, (i * 13 % 256) as u8, 200, 255]);
        }
        rgba
    }

    #[test]
    fn phosphor_is_green_dominant() {
        let mut rgba = frame(8, 8);
        Era::Phosphor.apply(&mut rgba, 8, 8);
        for pixel in rgba.chunks_exact(4) {
            assert!(pixel[1] >= pixel[0] && pixel[1] >= pixel[2]);
        }
    }

    #[test]
    fn eight_bit_snaps_every_pixel_to_the_palette() {
        let mut rgba = frame(8, 8);
        Era::EightBit.apply(&mut rgba, 8, 8);
        for pixel in rgba.chunks_exact(4) {
            assert!(PALETTE.contains(&[pixel[0], pixel[1], pixel[2]]));
        }
        assert_eq!(nearest_palette(250, 250, 250), [255, 255, 255]);
        assert_eq!(nearest_palette(5, 5, 5), [0, 0, 0]);
    }

    #[test]
    fn vector_culls_the_dim_and_boosts_the_bright() {
        let mut rgba = vec![10, 11, 15, 255, 200, 100, 50, 255];
        Era::Vector.apply(&mut rgba, 2, 1);
        assert_eq!(&rgba[0..3], &[0, 0, 0], "dim background goes pure black");
        assert!(rgba[4] > 200, "the beam brightens");
    }

    #[test]
    fn modern_is_the_identity_and_the_cycle_wraps() {
        let mut rgba = frame(4, 4);
        let before = rgba.clone();
        Era::Modern.apply(&mut rgba, 4, 4);
        assert_eq!(rgba, before);
        assert_eq!(Era::Modern.next(), Era::Phosphor);
        assert_eq!(Era::Phosphor.next().next().next(), Era::Modern);
    }

    #[test]
    fn names_parse_back() {
        for era in Era::ALL {
            assert_eq!(Era::parse(era.name()), Some(era));
        }
        assert_eq!(Era::parse("scope"), Some(Era::Vector));
        assert!(Era::parse("betamax").is_none());
    }
}
