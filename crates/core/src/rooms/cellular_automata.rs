//! Cellular Automata: elementary (Wolfram) rules on a line.
//!
//! A single row of cells evolves generation by generation, each cell's next
//! state decided only by itself and its two neighbors via an 8-bit rule number.
//! The space-time history is drawn top to bottom. Rule 90 draws a Sierpinski
//! triangle; Rule 30 pours out chaos; Rule 110 is Turing-complete. See
//! `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// A curated tour of notable elementary rules, indexed by phase `t`.
///
/// The full 0..=255 dial belongs to the interactive GPU version; the headless
/// face sweeps these standouts so every frame is worth looking at.
const NOTABLE_RULES: [u8; 8] = [90, 30, 110, 54, 150, 18, 60, 105];

/// The Cellular Automata room.
#[derive(Debug, Default)]
pub struct CellularAutomata;

impl CellularAutomata {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The elementary rule selected by phase `t` (clamped into range).
    fn rule_for(t: f64) -> u8 {
        let span = NOTABLE_RULES.len();
        let index = (t.clamp(0.0, 0.999) * span as f64) as usize;
        NOTABLE_RULES[index.min(span - 1)]
    }
}

impl Room for CellularAutomata {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cellular-automata",
            title: "Cellular Automata",
            wing: "Emergence",
            blurb: "One line of cells and one tiny rule per cell; sweep t across notable rules, \
                    where Rule 90 draws a Sierpinski triangle and Rule 30 pours out chaos.",
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let rule = Self::rule_for(t);

        let mut cells = vec![false; width];
        cells[width / 2] = true;
        for y in 0..height {
            for (x, &alive) in cells.iter().enumerate() {
                if alive {
                    canvas.plot(x as i32, y as i32, '*');
                }
            }
            cells = next_generation(&cells, rule);
        }
    }

    fn reveal(&self) -> &'static str {
        "Rule 110 is as powerful as any computer ever built, and Rule 30's chaos \
         was good enough to ship as a random number generator. A one-line rule \
         with the power of a universe."
    }
}

/// Advance an elementary cellular automaton by one generation.
///
/// Cells outside the row are treated as dead. Each cell's next state is bit
/// `(left, center, right)` of the 8-bit `rule`.
fn next_generation(cells: &[bool], rule: u8) -> Vec<bool> {
    let n = cells.len();
    (0..n)
        .map(|i| {
            let left = i > 0 && cells[i - 1];
            let center = cells[i];
            let right = i + 1 < n && cells[i + 1];
            let pattern =
                (usize::from(left) << 2) | (usize::from(center) << 1) | usize::from(right);
            (rule >> pattern) & 1 == 1
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{CellularAutomata, next_generation};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn rule_for_starts_at_rule_90() {
        assert_eq!(CellularAutomata::rule_for(0.0), 90);
    }

    #[test]
    fn rule_90_is_left_xor_right() {
        let start = [false, false, true, false, false];
        let next = next_generation(&start, 90);
        assert_eq!(next, vec![false, true, false, true, false]);
    }

    #[test]
    fn render_is_deterministic() {
        let room = CellularAutomata::new();
        let mut a = Canvas::new(41, 21);
        let mut b = Canvas::new(41, 21);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = CellularAutomata::new();
        let mut canvas = Canvas::new(41, 21);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 1);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = CellularAutomata::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(10, 10);
        for t in [-1.0, 0.0, 0.999, 2.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_rule_110() {
        assert!(CellularAutomata::new().reveal().contains("Rule 110"));
    }
}
