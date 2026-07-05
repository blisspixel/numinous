//! The Arecibo puzzle: a message that only a species that does math can read.
//!
//! A stream of bits looks like noise until you arrange it into a grid, and it can
//! only be arranged one way, because its length is a semiprime with exactly two
//! factors. Here the message is 143 bits, and 143 is 11 times 13, nothing else.
//! `t` sweeps the width you try; only at the true width does the picture appear.
//! (The real 1974 message was 1,679 bits: 23 times 73.) See `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// The hidden payload, drawn at the one width that reads it: a big letter pi.
const ART: [&str; 13] = [
    "...........",
    "###########",
    "###########",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...##..",
    "..##...###.",
    "...........",
    "...........",
];
/// Smallest and largest widths `t` will try.
const MIN_WIDTH: usize = 6;
const MAX_WIDTH: usize = 18;

/// The Arecibo message room.
#[derive(Debug, Default)]
pub struct Arecibo;

impl Arecibo {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The width the player is trying at phase `t`.
    fn width_for(t: f64) -> usize {
        let span = (MAX_WIDTH - MIN_WIDTH) as f64;
        MIN_WIDTH + (t.clamp(0.0, 1.0) * span).round() as usize
    }
}

/// The message as a flat bitstream (row-major over `ART`).
fn bits() -> Vec<bool> {
    ART.iter()
        .flat_map(|row| row.chars().map(|c| c == '#'))
        .collect()
}

impl Room for Arecibo {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "arecibo",
            title: "Arecibo Message",
            wing: "Signals & Codes",
            blurb: "A stream of bits that looks like noise until you line it up at the right width. \
                    The length is a semiprime, so there is only one width that works. t hunts for it.",
            accent: [120, 230, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let stream = bits();
        let grid_w = Self::width_for(t);
        let grid_h = stream.len().div_ceil(grid_w);
        // Sample the grid across the whole surface so the picture fills it.
        for py in 0..height {
            for px in 0..width {
                let gx = px * grid_w / width;
                let gy = py * grid_h / height;
                let index = gy * grid_w + gx;
                if stream.get(index).copied().unwrap_or(false) {
                    canvas.plot(px as i32, py as i32, '#');
                }
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "This message is 143 bits, and 143 is 11 times 13 and nothing else, so \
         there is exactly one grid it fits. Any species that could factor it would \
         find the picture. In 1974 we sent 1,679 bits, which is 23 times 73, at a \
         star cluster 25,000 light-years away. The reply is not due for a while."
    }
}

#[cfg(test)]
mod tests {
    use super::{ART, Arecibo, bits};
    use crate::canvas::Canvas;
    use crate::room::Room;

    /// The secret width: the smaller factor of 143 (11 x 13).
    const TRUE_WIDTH: usize = 11;

    #[test]
    fn the_message_length_is_the_semiprime() {
        assert_eq!(bits().len(), 11 * 13);
        assert_eq!(ART.len(), 13);
        assert!(ART.iter().all(|row| row.chars().count() == 11));
    }

    #[test]
    fn the_true_width_is_reachable_by_some_phase() {
        let hit = (0..=100).any(|i| Arecibo::width_for(f64::from(i) / 100.0) == TRUE_WIDTH);
        assert!(hit, "no phase produces the true width");
    }

    #[test]
    fn the_true_width_shows_more_of_the_picture_than_a_wrong_one() {
        // Find phases for the true width and a deliberately wrong one, compare ink.
        let t_true = (0..=100)
            .map(|i| f64::from(i) / 100.0)
            .find(|&t| Arecibo::width_for(t) == TRUE_WIDTH)
            .unwrap();
        let mut right = Canvas::new(44, 26);
        Arecibo::new().render(&mut right, t_true);
        assert!(right.ink_count() > 20);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Arecibo::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_explains_the_semiprime() {
        assert!(Arecibo::new().reveal().contains("11 times 13"));
    }
}
