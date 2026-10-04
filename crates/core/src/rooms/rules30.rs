//! Rule 30: elementary CA chaos from one black cell.
//!
//! Wolfram Rule 30 from a single seed: structured randomness. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn rule_byte(hand: Option<(f64, f64)>) -> u8 {
    if let Some((x, _)) = hand {
        (x * 255.0).round() as u8
    } else {
        // The named room grows one rule. Comparing other rules belongs to
        // the player's dial or the Cellular Automata gallery. Switching rules
        // on its own both misnamed the picture and strobed between densities.
        30
    }
}

fn seed_column(width: usize, seed: u64) -> usize {
    let width = width.max(1);
    (width / 2 + (seed % width as u64) as usize) % width
}

fn step(row: &[u8], rule: u8) -> Vec<u8> {
    let n = row.len();
    let mut next = vec![0u8; n];
    for i in 0..n {
        let l = row[(i + n - 1) % n];
        let c = row[i];
        let r = row[(i + 1) % n];
        let idx = (l << 2) | (c << 1) | r;
        next[i] = (rule >> idx) & 1;
    }
    next
}

fn evolve(width: usize, rows: usize, rule: u8, seed_bit: usize) -> Vec<Vec<u8>> {
    let mut row = vec![0u8; width];
    row[seed_bit.min(width.saturating_sub(1))] = 1;
    let mut out = Vec::with_capacity(rows);
    out.push(row.clone());
    for _ in 1..rows {
        row = step(&row, rule);
        out.push(row.clone());
    }
    out
}

fn draw(canvas: &mut dyn Surface, grid: &[Vec<u8>]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || grid.is_empty() {
        return;
    }
    let rows = grid.len();
    let cols = grid[0].len();
    for (ry, row) in grid.iter().enumerate() {
        let y0 = (ry as f64 / rows as f64 * height as f64).round() as i32;
        let y1 = (((ry + 1) as f64 / rows as f64) * height as f64).round() as i32;
        for (cx, &bit) in row.iter().enumerate() {
            if bit == 0 {
                continue;
            }
            let x0 = (cx as f64 / cols as f64 * width as f64).round() as i32;
            let x1 = (((cx + 1) as f64 / cols as f64) * width as f64).round() as i32;
            for yy in y0..y1.max(y0 + 1) {
                for xx in x0..x1.max(x0 + 1) {
                    canvas.plot(xx, yy, '#');
                }
            }
        }
    }
}

/// Rule 30 / elementary CA room.
#[derive(Debug, Default)]
pub struct Rules30 {
    seed: u64,
}

impl Rules30 {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }
    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for Rules30 {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let rule = rule_byte(None);
        let cols = 72usize;
        let rows = 36 + (phase_unit(t) * 20.0) as usize;
        let grid = evolve(cols, rows, rule, seed_column(cols, self.seed));
        draw(canvas, &grid);
    }

    /// The start of Rule 30's growing space-time history.
    fn postcard_t(&self) -> f64 {
        0.0
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "rule30",
            root: 82.41,
            tempo: 144,
            line: &[0, 7, 0, 12, 0, 5, 0, 14],
            encodes: "one seed bit becoming aperiodic weather",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE RULE BYTE")
    }

    fn status(&self, _t: f64) -> Option<String> {
        let rule = rule_byte(None);
        Some(format!("rule={rule}  CA  DRAG:RULE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, _t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let rule = rule_byte(hands.last().copied());
        let cols = 72usize;
        let rows = 40;
        let seed_bit = hands
            .last()
            .map(|&(x, _)| (x * (cols - 1) as f64) as usize)
            .unwrap_or_else(|| seed_column(cols, self.seed));
        let grid = evolve(cols, rows, rule, seed_bit);
        draw(canvas, &grid);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let rule = rule_byte(hands.last().copied());
        let name = if rule == 30 {
            "classic"
        } else if rule == 90 {
            "sierp"
        } else if rule == 110 {
            "univ"
        } else {
            "ECA"
        };
        // Elementary CA rule number in 0..255; class hint for famous ones.
        Some(format!("rule={rule}  {name}"))
    }

    fn reveal(&self) -> &'static str {
        "Rule 30 is an elementary cellular automaton: each cell looks at itself \
         and its two neighbors, then applies an 8-bit lookup. From one black \
         cell it produces aperiodic patterns used as a randomness generator."
    }
}

#[cfg(test)]
mod tests {
    use super::{Rules30, evolve, rule_byte};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn the_postcard_is_rule_30() {
        let room = Rules30::new();
        let t = room.postcard_t();
        assert_eq!(rule_byte(None), 30);
        assert!(room.status(t).unwrap().starts_with("rule=30 "));
        // The center column from one cell, OEIS A051023, computed separately
        // from this file. Rule 54, which the postcard used to show, repeats
        // 1, 1, 0, 0 instead.
        const CENTER: [u8; 36] = [
            1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 1, 0,
            1, 1, 1, 0, 0, 1, 1, 1,
        ];
        let grid = evolve(72, CENTER.len(), rule_byte(None), 36);
        let column: Vec<u8> = grid.iter().map(|row| row[36]).collect();
        assert_eq!(column, CENTER);
    }

    #[test]
    fn ambient_growth_keeps_rule_30_and_variation_moves_only_the_seed() {
        for seed in [0, 1, 17, u64::MAX] {
            for t in [0.0, 0.25, 0.5, 0.75, 1.0, f64::NAN] {
                assert!(Rules30::new_with(seed).status(t).unwrap().starts_with("rule=30 "));
            }
        }
        let mut original = Canvas::new(72, 40);
        let mut moved = Canvas::new(72, 40);
        Rules30::new().render(&mut original, 0.0);
        Rules30::new_with(17).render(&mut moved, 0.0);
        assert_ne!(original.to_text(), moved.to_text());
        assert_eq!(super::seed_column(72, 0), 36);
        assert_eq!(super::seed_column(72, u64::MAX), 51);
        assert_eq!(rule_byte(Some((90.0 / 255.0, 0.5))), 90);
    }

    #[test]
    fn status_invites() {
        let s = Rules30::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("rule="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn rule_changes() {
        let r = Rules30::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn rule30_grows() {
        let g = evolve(21, 5, 30, 10);
        assert_eq!(g[0][10], 1);
        assert!(g[4].contains(&1));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        Rules30::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Rules30::new().motif().unwrap().line.len() >= 6);
    }
}
