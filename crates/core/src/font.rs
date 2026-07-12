//! A tiny 5x7 bitmap font for drawing labels into a [`Surface`].
//!
//! Enough of one to draw room names and reveals in the windowed app (the HUD),
//! with no external font dependency. Text is drawn uppercase; unknown characters
//! render blank. Each glyph is seven rows; the low five bits of each row are the
//! pixels, most-significant bit on the left. See `docs/VISUALS.md`.

use crate::surface::Surface;

/// Glyph cell width in font pixels.
const GLYPH_W: i32 = 5;
/// Columns advanced per character (glyph plus one-pixel gap).
const ADVANCE: i32 = 6;
/// Glyph cell height in font pixels.
const GLYPH_H: usize = 7;

/// The seven-row bitmap for a character (uppercased); blank if unsupported.
#[rustfmt::skip]
fn glyph(c: char) -> [u8; GLYPH_H] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],
        'B' => [0b11110,0b10001,0b10001,0b11110,0b10001,0b10001,0b11110],
        'C' => [0b01110,0b10001,0b10000,0b10000,0b10000,0b10001,0b01110],
        'D' => [0b11110,0b10001,0b10001,0b10001,0b10001,0b10001,0b11110],
        'E' => [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b11111],
        'F' => [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b10000],
        'G' => [0b01110,0b10001,0b10000,0b10111,0b10001,0b10001,0b01111],
        'H' => [0b10001,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],
        'I' => [0b01110,0b00100,0b00100,0b00100,0b00100,0b00100,0b01110],
        'J' => [0b00111,0b00010,0b00010,0b00010,0b00010,0b10010,0b01100],
        'K' => [0b10001,0b10010,0b10100,0b11000,0b10100,0b10010,0b10001],
        'L' => [0b10000,0b10000,0b10000,0b10000,0b10000,0b10000,0b11111],
        'M' => [0b10001,0b11011,0b10101,0b10101,0b10001,0b10001,0b10001],
        'N' => [0b10001,0b10001,0b11001,0b10101,0b10011,0b10001,0b10001],
        'O' => [0b01110,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],
        'P' => [0b11110,0b10001,0b10001,0b11110,0b10000,0b10000,0b10000],
        'Q' => [0b01110,0b10001,0b10001,0b10001,0b10101,0b10010,0b01101],
        'R' => [0b11110,0b10001,0b10001,0b11110,0b10100,0b10010,0b10001],
        'S' => [0b01111,0b10000,0b10000,0b01110,0b00001,0b00001,0b11110],
        'T' => [0b11111,0b00100,0b00100,0b00100,0b00100,0b00100,0b00100],
        'U' => [0b10001,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],
        'V' => [0b10001,0b10001,0b10001,0b10001,0b10001,0b01010,0b00100],
        'W' => [0b10001,0b10001,0b10001,0b10101,0b10101,0b11011,0b10001],
        'X' => [0b10001,0b10001,0b01010,0b00100,0b01010,0b10001,0b10001],
        'Y' => [0b10001,0b10001,0b01010,0b00100,0b00100,0b00100,0b00100],
        'Z' => [0b11111,0b00001,0b00010,0b00100,0b01000,0b10000,0b11111],
        '0' => [0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110],
        '1' => [0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110],
        '2' => [0b01110,0b10001,0b00001,0b00110,0b01000,0b10000,0b11111],
        '3' => [0b11111,0b00010,0b00100,0b00010,0b00001,0b10001,0b01110],
        '4' => [0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010],
        '5' => [0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110],
        '6' => [0b00110,0b01000,0b10000,0b11110,0b10001,0b10001,0b01110],
        '7' => [0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000],
        '8' => [0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110],
        '9' => [0b01110,0b10001,0b10001,0b01111,0b00001,0b00010,0b01100],
        '.' => [0b00000,0b00000,0b00000,0b00000,0b00000,0b01100,0b01100],
        ',' => [0b00000,0b00000,0b00000,0b00000,0b01100,0b00100,0b01000],
        '\'' => [0b01100,0b00100,0b01000,0b00000,0b00000,0b00000,0b00000],
        '-' => [0b00000,0b00000,0b00000,0b11111,0b00000,0b00000,0b00000],
        '+' => [0b00000,0b00100,0b00100,0b11111,0b00100,0b00100,0b00000],
        '*' => [0b00000,0b10101,0b01110,0b11111,0b01110,0b10101,0b00000],
        '=' => [0b00000,0b00000,0b11111,0b00000,0b11111,0b00000,0b00000],
        '^' => [0b00100,0b01010,0b10001,0b00000,0b00000,0b00000,0b00000],
        '<' => [0b00010,0b00100,0b01000,0b10000,0b01000,0b00100,0b00010],
        '>' => [0b01000,0b00100,0b00010,0b00001,0b00010,0b00100,0b01000],
        '[' => [0b01110,0b01000,0b01000,0b01000,0b01000,0b01000,0b01110],
        ']' => [0b01110,0b00010,0b00010,0b00010,0b00010,0b00010,0b01110],
        '%' => [0b11001,0b11010,0b00010,0b00100,0b01000,0b01011,0b10011],
        'π' => [0b11111,0b10101,0b10101,0b10101,0b10101,0b10101,0b10001],
        '·' => [0b00000,0b00000,0b00000,0b00100,0b00000,0b00000,0b00000],
        '!' => [0b00100,0b00100,0b00100,0b00100,0b00100,0b00000,0b00100],
        '?' => [0b01110,0b10001,0b00001,0b00110,0b00100,0b00000,0b00100],
        ':' => [0b00000,0b01100,0b01100,0b00000,0b01100,0b01100,0b00000],
        '(' => [0b00010,0b00100,0b01000,0b01000,0b01000,0b00100,0b00010],
        ')' => [0b01000,0b00100,0b00010,0b00010,0b00010,0b00100,0b01000],
        '/' => [0b00001,0b00010,0b00100,0b01000,0b10000,0b00000,0b00000],
        _ => [0; GLYPH_H],
    }
}

/// The pixel width of `text` at `scale`.
#[must_use]
pub fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().count() as i32 * ADVANCE * scale
}

/// Word-wrap `text` into lines of at most `max_chars` characters.
#[must_use]
pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let max = max_chars.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line = word.to_string();
        } else if line.len() + 1 + word.len() <= max {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Draw `text` into `surface`, top-left at `(x, y)`, `scale` pixels per font
/// pixel, marking lit pixels with `mark`.
pub fn draw_text(surface: &mut dyn Surface, text: &str, x: i32, y: i32, scale: i32, mark: char) {
    let scale = scale.max(1);
    let mut cursor_x = x;
    for ch in text.chars() {
        let rows = glyph(ch);
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..GLYPH_W {
                if bits & (1 << (GLYPH_W - 1 - col)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            surface.plot(
                                cursor_x + col * scale + sx,
                                y + row as i32 * scale + sy,
                                mark,
                            );
                        }
                    }
                }
            }
        }
        cursor_x += ADVANCE * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::{draw_text, text_width};
    use crate::canvas::Canvas;

    #[test]
    fn width_scales_with_length_and_scale() {
        assert_eq!(text_width("AB", 1), 12);
        assert_eq!(text_width("AB", 2), 24);
        assert_eq!(text_width("", 3), 0);
    }

    #[test]
    fn draw_text_puts_ink_on_the_surface() {
        let mut c = Canvas::new(60, 10);
        draw_text(&mut c, "HI", 0, 0, 1, '*');
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn different_letters_render_differently() {
        let mut a = Canvas::new(10, 8);
        let mut b = Canvas::new(10, 8);
        draw_text(&mut a, "A", 0, 0, 1, '*');
        draw_text(&mut b, "Z", 0, 0, 1, '*');
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn a_space_draws_nothing_but_advances() {
        let mut c = Canvas::new(20, 8);
        draw_text(&mut c, " ", 0, 0, 1, '*');
        assert_eq!(c.ink_count(), 0);
    }

    #[test]
    fn out_of_bounds_text_does_not_panic() {
        let mut c = Canvas::new(4, 4);
        draw_text(&mut c, "HELLO WORLD", -3, -2, 2, '#');
    }

    #[test]
    fn wrap_text_respects_the_width() {
        let lines = super::wrap_text("the quick brown fox jumps", 10);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| l.len() <= 10));
        assert_eq!(lines.join(" "), "the quick brown fox jumps");
    }

    #[test]
    fn wrap_text_handles_empty() {
        assert!(super::wrap_text("", 10).is_empty());
    }

    #[test]
    fn every_supported_glyph_draws_something() {
        let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,'-!?:()/+*=^<>[]%π·";
        let mut c = Canvas::new(charset.chars().count() * 6, 8);
        draw_text(&mut c, charset, 0, 0, 1, '*');
        assert!(
            c.ink_count() > 100,
            "the full charset should draw many pixels"
        );
    }
}
