//! Terminal rendering: the terminal as a framebuffer, with or without color.
//!
//! Modern terminals (Windows Terminal, iTerm2, kitty, most Linux emulators)
//! support 24-bit color. Pairing that with the upper-half-block character, whose
//! foreground paints the top half of a cell and background paints the bottom,
//! gives two full-color pixels per character cell. A [`Raster`] becomes a real
//! color image in the terminal, no window required. See `docs/INTERFACES.md`.
//!
//! [`to_mono`] renders the same geometry without color, for `NO_COLOR` and for
//! any surface that must not depend on hue. [`to_terminal`] picks between them
//! so a caller cannot honor a player's preference on one screen and forget it
//! on the next.

use crate::raster::Raster;
use crate::surface::Surface;

/// The upper-half-block character: foreground on top, background below.
const HALF_BLOCK: char = '\u{2580}';

/// An RGB color.
type Rgb = (u8, u8, u8);

/// Encode a raster for a terminal, in color or not.
///
/// The single place that decides between [`to_ansi`] and [`to_mono`], so a
/// caller cannot honor a player's preference on one screen and forget it on
/// the next.
#[must_use]
pub fn to_terminal(raster: &Raster, color: bool) -> String {
    if color {
        to_ansi(raster)
    } else {
        to_mono(raster)
    }
}

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

/// The four block characters that encode which halves of a cell are lit:
/// neither, upper only, lower only, both.
const BLOCKS: [char; 4] = [' ', '\u{2580}', '\u{2584}', '\u{2588}'];

/// Shades for a cell whose halves are both lit, darkest first.
///
/// A cell with only one half lit already says something with its shape. A cell
/// with both lit used to say only "full", which is what made dense plates
/// answer nothing: a fractal that shifted its whole field from dim to bright
/// still came out as the same slab of solid blocks. These four give that case
/// four levels instead of one, in the same character cell.
const SHADES: [char; 4] = ['\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}'];

/// Boundaries between the shades, on the same 0 to 255 luminance scale as
/// [`LIT_FLOOR`].
///
/// These are the quartiles of what the catalog actually draws: the median
/// both-lit cell across all 354 rooms sits at 152, with quarters at 128 and
/// 203. Splitting there gives each shade about a quarter of the ink, measured
/// at 24.3, 24.8, 25.0 and 26.0 percent.
///
/// Round numbers were tried first and were much worse. Even thirds of the
/// range put the lightest shade below 64, where almost nothing is drawn: it
/// carried 1.4 percent of the ink, so one of four available characters was
/// spent on almost nothing while 45 percent crowded into another. A room
/// whose touch moves the picture by less than a band cannot show it, and two
/// rooms were failing for exactly that reason.
///
/// Fixed rather than derived from the frame: an adaptive threshold would make
/// a still picture change as its neighbours changed, and two frames apart
/// could then differ for no reason the player caused.
const SHADE_STEPS: [u32; 3] = [128, 152, 203];

/// A pixel counts as lit above this luminance. The stage is near-black and
/// strokes glow, so the floor only has to clear the unlit background.
const LIT_FLOOR: u32 = 24;

/// Rec. 601 luma, integer arithmetic so the result cannot drift by platform.
fn luminance((r, g, b): Rgb) -> u32 {
    (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000
}

/// Encode a raster using block characters alone, adding no color.
///
/// Same geometry as [`to_ansi`]: one output row per two pixel rows, one
/// character per column, so a layout built for one renders identically under
/// the other. Structure survives because each character still carries both of
/// its half-pixels; only the hue is gone.
///
/// Where exactly one half is lit the shape carries the meaning, so the half
/// block says it. Where both are lit there is no shape left to carry anything,
/// and that is where a dense plate loses its answer: an audit of all 354 rooms
/// found 21 whose response to a touch vanished here, because the whole field
/// was already solid and only its brightness moved. Shading that case restores
/// 15 of them at no cost to the geometry.
///
/// This is what `NO_COLOR` selects, and what any surface that must not depend
/// on color should use. It emits no escape sequences at all.
#[must_use]
pub fn to_mono(raster: &Raster) -> String {
    let width = raster.width();
    let height = raster.height();
    let rgba = raster.to_rgba();
    // Once per half-pixel. Both the lit decision and the shading mean come
    // from the same number, because the shading path is busiest on exactly the
    // dense plates where every cell would otherwise be measured twice over.
    let level = |x: usize, y: usize| -> u32 {
        if y >= height {
            return 0;
        }
        let o = (y * width + x) * 4;
        luminance((rgba[o], rgba[o + 1], rgba[o + 2]))
    };

    let mut out = String::with_capacity(width.saturating_add(1) * height.div_ceil(2));
    for row in 0..height.div_ceil(2) {
        let (top_y, bottom_y) = (row * 2, row * 2 + 1);
        for x in 0..width {
            let (top, bottom) = (level(x, top_y), level(x, bottom_y));
            let (tl, bl) = (top >= LIT_FLOOR, bottom >= LIT_FLOOR);
            if tl && bl {
                let mean = (top + bottom) / 2;
                let step = SHADE_STEPS.iter().filter(|&&edge| mean >= edge).count();
                out.push(SHADES[step]);
            } else {
                out.push(BLOCKS[usize::from(tl) | (usize::from(bl) << 1)]);
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{LIT_FLOOR, SHADE_STEPS, SHADES, to_ansi, to_mono};
    use crate::raster::Raster;
    use crate::surface::Surface;

    #[test]
    fn every_shade_carries_a_real_share_of_the_ink() {
        // The failure this guards against is subtle: thresholds that look
        // reasonable can leave one shade almost unused, which spends a
        // quarter of the available characters on nothing and starves the
        // range where the drawing actually lives. Even thirds of 0 to 255 did
        // exactly that here, giving the lightest shade 1.4 percent.
        //
        // A sample of the catalog rather than all of it, so this stays cheap
        // enough to run on every push.
        use crate::registry::all_rooms;
        let mut counts = [0u64; 4];
        for room in all_rooms().iter().step_by(7) {
            let mut raster = Raster::with_accent(96, 56, room.meta().accent);
            room.render(&mut raster, 0.35);
            for glyph in to_mono(&raster).chars() {
                if let Some(index) = SHADES.iter().position(|shade| *shade == glyph) {
                    counts[index] += 1;
                }
            }
        }
        let total: u64 = counts.iter().sum();
        assert!(total > 1_000, "too little ink sampled to judge: {total}");
        for (index, count) in counts.iter().enumerate() {
            let share = 100.0 * *count as f64 / total as f64;
            assert!(
                share >= 10.0,
                "shade {index} carries only {share:.1} percent of the ink, so it is \n                 nearly wasted; the thresholds no longer match what is drawn"
            );
        }
    }

    #[test]
    fn a_solid_field_still_reports_how_bright_it_is() {
        // The whole point. Two frames that are both entirely lit, differing
        // only in brightness, used to render as the same slab of full blocks.
        let mut dim = Raster::new(4, 4);
        let mut bright = Raster::new(4, 4);
        dim.set_rgba(&[60u8; 4 * 4 * 4]);
        bright.set_rgba(&[220u8; 4 * 4 * 4]);
        assert_ne!(
            to_mono(&dim),
            to_mono(&bright),
            "a change in brightness alone must still show"
        );
    }

    #[test]
    fn shading_only_replaces_the_case_that_had_no_shape_left() {
        // A cell with one half lit keeps its half block, because the shape is
        // already carrying the meaning there.
        let mut top = Raster::new(1, 2);
        top.set_rgba(&[255, 255, 255, 255, 0, 0, 0, 255]);
        assert_eq!(to_mono(&top).trim_end(), "\u{2580}");
        let mut bottom = Raster::new(1, 2);
        bottom.set_rgba(&[0, 0, 0, 255, 255, 255, 255, 255]);
        assert_eq!(to_mono(&bottom).trim_end(), "\u{2584}");
    }

    #[test]
    fn every_shade_is_reachable_and_they_climb_in_order() {
        // Four distinct levels, and brighter input never picks a darker shade.
        //
        // The samples come from the thresholds rather than from constants
        // chosen alongside them. Written out by hand they silently stopped
        // covering all four bands the moment the thresholds moved onto the
        // catalog's measured ink, and this test caught that as a collision.
        let mut probes: Vec<u8> = vec![LIT_FLOOR as u8];
        probes.extend(SHADE_STEPS.iter().map(|edge| *edge as u8));
        let mut seen = Vec::new();
        for level in probes {
            let mut raster = Raster::new(1, 2);
            raster.set_rgba(&[level, level, level, 255, level, level, level, 255]);
            seen.push(to_mono(&raster).trim_end().to_string());
        }
        seen.dedup();
        assert_eq!(
            seen.len(),
            4,
            "each step must pick a different shade: {seen:?}"
        );
    }

    #[test]
    fn shading_adds_no_escape_and_keeps_the_geometry() {
        // The two guarantees NO_COLOR rests on, re-checked against the shades.
        for (w, h) in [(8usize, 6usize), (8, 5), (40, 21)] {
            let mut raster = Raster::new(w, h);
            raster.set_rgba(&vec![140u8; w * h * 4]);
            let mono = to_mono(&raster);
            assert!(!mono.contains('\u{1b}'), "escape in mono output");
            assert_eq!(mono.lines().count(), to_ansi(&raster).lines().count());
            for line in mono.lines() {
                assert_eq!(line.chars().count(), w);
            }
        }
    }

    #[test]
    fn mono_matches_the_color_geometry_exactly() {
        // A layout built for one must render identically under the other, or
        // switching renderers would reflow every screen.
        for (w, h) in [(8, 6), (8, 5), (1, 1), (40, 21)] {
            let mut raster = Raster::new(w, h);
            raster.plot(0, 0, '#');
            let color = to_ansi(&raster);
            let mono = to_mono(&raster);
            assert_eq!(
                mono.lines().count(),
                color.lines().count(),
                "{w}x{h} row count"
            );
            for line in mono.lines() {
                assert_eq!(line.chars().count(), w, "{w}x{h} column count");
            }
        }
    }

    #[test]
    fn mono_adds_no_color_and_no_escapes_at_all() {
        let mut raster = Raster::new(12, 8);
        raster.plot(3, 3, '#');
        raster.plot(4, 4, '#');
        let mono = to_mono(&raster);
        assert!(!mono.contains('\u{1b}'), "escape sequence in {mono:?}");
        assert!(!mono.contains("38;2;"), "color in {mono:?}");
    }

    #[test]
    fn mono_keeps_the_structure_the_color_path_shows() {
        // Both halves of a cell stay independent, so a lit pixel is visible
        // and its vertical position within the cell is preserved.
        let mut top = Raster::new(2, 2);
        top.plot(0, 0, '#');
        let mut bottom = Raster::new(2, 2);
        bottom.plot(0, 1, '#');
        let blank = to_mono(&Raster::new(2, 2));
        assert_ne!(to_mono(&top), blank, "a lit pixel must show");
        assert_ne!(
            to_mono(&top),
            to_mono(&bottom),
            "which half is lit must survive"
        );
    }

    #[test]
    fn mono_is_deterministic() {
        let mut a = Raster::new(10, 10);
        let mut b = Raster::new(10, 10);
        a.plot(3, 3, '#');
        b.plot(3, 3, '#');
        assert_eq!(to_mono(&a), to_mono(&b));
    }

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
