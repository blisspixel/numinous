//! Rule 30: elementary CA chaos from one black cell.
//!
//! Wolfram Rule 30 from a single seed: structured randomness. DRAG: SET THE
//! RULE BYTE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
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

fn rule_byte(t: f64, hand: Option<(f64, f64)>, seed: u64) -> u8 {
    if let Some((x, _)) = hand {
        (x * 255.0).round() as u8
    } else {
        // Ambient stays on rules known to leave ink from a single seed cell.
        const GALLERY: [u8; 8] = [30, 90, 110, 150, 54, 60, 105, 126];
        let idx = if seed == 0 {
            (phase_unit(t) * (GALLERY.len() - 1) as f64).round() as usize
        } else {
            ((seed as usize) + (phase_unit(t) * 3.0) as usize) % GALLERY.len()
        };
        GALLERY[idx.min(GALLERY.len() - 1)]
    }
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
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "rule-30",
            title: "Rule 30",
            wing: "Emergence",
            blurb: "Elementary cellular automaton Rule 30: one black cell becomes structured \
                    chaos. t and DRAG: SET THE RULE BYTE.",
            accent: [40, 40, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let rule = rule_byte(t, None, self.seed);
        let cols = 72usize;
        let rows = 36 + (phase_unit(t) * 20.0) as usize;
        let grid = evolve(cols, rows, rule, cols / 2);
        draw(canvas, &grid);
    }

    fn postcard_t(&self) -> f64 {
        0.5
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

    fn status(&self, t: f64) -> Option<String> {
        let rule = rule_byte(t, None, self.seed);
        Some(format!("rule={rule}  CA  DRAG:RULE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let rule = rule_byte(t, hands.last().copied(), self.seed);
        let cols = 72usize;
        let rows = 40;
        let seed_bit = hands
            .last()
            .map(|&(x, _)| (x * (cols - 1) as f64) as usize)
            .unwrap_or(cols / 2);
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
        let rule = rule_byte(t, hands.last().copied(), self.seed);
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
    use super::{Rules30, evolve};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

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
