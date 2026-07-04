//! Preview the bitmap font by drawing a label to the ASCII canvas.
//!
//! Run with `cargo run -p numinous-core --example font_preview`.

use numinous_core::{Canvas, draw_text};

fn main() {
    let lines = [
        "MATH IS COOL",
        "ABCDEFGHIJKLM",
        "NOPQRSTUVWXYZ",
        "0123456789 .,'-!?:()",
    ];
    for text in lines {
        let mut canvas = Canvas::new(text.chars().count() * 6 + 2, 7);
        draw_text(&mut canvas, text, 0, 0, 1, '#');
        print!("{}", canvas.to_text());
        println!();
    }
}
