//! Wythoff array: Beatty rows from golden powers, cold/hot mex game.
//!
//! DRAG: TUNE ROWS. See `docs/ROOMS.md`.

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

fn rows(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 4) as f64 };
    if let Some((x, _)) = hand {
        4.0 + x * 16.0 + s
    } else {
        6.0 + phase_unit(t) * 12.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(3.0, 24.0) as usize;
    let phi = (1.0 + 5.0_f64.sqrt()) * 0.5;
    // Wythoff pairs: A_k = floor(k phi), B_k = floor(k phi^2) = A_k + k
    let mut prev_a: Option<(i32, i32)> = None;
    let mut prev_b: Option<(i32, i32)> = None;
    let max_val = ((n as f64) * phi * phi).ceil();
    for k in 1..=n {
        let a = (k as f64 * phi).floor();
        let b = a + k as f64;
        let xa = ((a / max_val) * width.saturating_sub(1) as f64).round() as i32;
        let xb = ((b / max_val) * width.saturating_sub(1) as f64).round() as i32;
        let ya = ((k as f64 / n as f64) * height.saturating_sub(1) as f64 * 0.45).round() as i32;
        let yb = (height as f64 * 0.55
            + (k as f64 / n as f64) * height.saturating_sub(1) as f64 * 0.4)
            .round() as i32;
        if let Some((ox, oy)) = prev_a {
            canvas.line(ox, oy, xa, ya, '#');
        }
        if let Some((ox, oy)) = prev_b {
            canvas.line(ox, oy, xb, yb, '*');
        }
        canvas.line(xa, ya - 1, xa, ya + 1, 'o');
        canvas.line(xb, yb - 1, xb, yb + 1, '+');
        prev_a = Some((xa, ya));
        prev_b = Some((xb, yb));
    }
    let _ = seed;
}

/// Wythoff array room.
#[derive(Debug, Default)]
pub struct Wythoff {
    seed: u64,
}

impl Wythoff {
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

impl Room for Wythoff {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "wythoff",
            title: "Wythoff Array",
            wing: "Number & Pattern",
            blurb: "Golden Beatty pairs A_k, B_k. t and DRAG: TUNE ROWS.",
            accent: [160, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, rows(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "wythoff",
            root: 46.25,
            tempo: 94,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "Wythoff: cold and hot positions from golden Beatty sequences",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ROWS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = rows(t, None, self.seed).round();
        Some(format!("n={n:.0}  gold  DRAG:ROWS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = rows(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = rows(t, hands.last().copied(), self.seed).round() as i32;
        // Beatty pair floor(k phi), floor(k phi^2) partition.
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        Some(format!("n={n}  phi={phi:.3}  Beatty"))
    }

    fn reveal(&self) -> &'static str {
        "The Wythoff array lists pairs (floor(k phi), floor(k phi^2)) for the \
         golden ratio phi. They are the cold positions of Wythoff's game and a \
         Beatty partition of the naturals into two complementary sequences."
    }
}

#[cfg(test)]
mod tests {
    use super::Wythoff;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Wythoff::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("gold"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn rows_change() {
        let r = Wythoff::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Wythoff::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
