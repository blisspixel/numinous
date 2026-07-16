//! Baker's map: stretch, stack, and fold the unit square.
//!
//! The classic chaotic map on [0,1]^2. DRAG: SET THE STEPS. See `docs/ROOMS.md`.

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

fn steps(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (2 + (x * 14.0) as usize).clamp(2, 16)
    } else {
        (3 + (phase_unit(t) * 10.0) as usize).clamp(2, 14)
    }
}

fn baker_step(x: f64, y: f64) -> (f64, f64) {
    if x < 0.5 {
        (2.0 * x, y * 0.5)
    } else {
        (2.0 * x - 1.0, 0.5 + y * 0.5)
    }
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Grid of sample points, each iterated n times, paints the image of the square.
    let grid = 48usize;
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.01
    };
    for iy in 0..grid {
        for ix in 0..grid {
            let mut x = (ix as f64 + 0.5) / grid as f64;
            let mut y = (iy as f64 + 0.5) / grid as f64;
            x = (x + shift).fract();
            for _ in 0..n {
                let (nx, ny) = baker_step(x, y);
                x = nx;
                y = ny;
            }
            let px = (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
            let py = ((1.0 - y.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
            let ch = if n > 8 {
                '#'
            } else if n > 4 {
                '*'
            } else {
                '+'
            };
            canvas.plot(px, py, ch);
        }
    }
    // Outline the unit square for orientation.
    let x0 = 0;
    let x1 = width.saturating_sub(1) as i32;
    let y0 = 0;
    let y1 = height.saturating_sub(1) as i32;
    canvas.line(x0, y0, x1, y0, '.');
    canvas.line(x0, y1, x1, y1, '.');
    canvas.line(x0, y0, x0, y1, '.');
    canvas.line(x1, y0, x1, y1, '.');
}

/// Baker's map room.
#[derive(Debug, Default)]
pub struct Baker {
    seed: u64,
}

impl Baker {
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

impl Room for Baker {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "baker",
            title: "Baker's Map",
            wing: "Motion & Dynamics",
            blurb: "Stretch the square, cut, and stack: classic chaos on [0,1]^2. t and DRAG: SET \
                    THE STEPS.",
            accent: [180, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, steps(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "baker",
            root: 138.59,
            tempo: 112,
            line: &[0, 7, 0, 12, 5, 0, 7, 12],
            encodes: "stretch cut stack fold on the unit square",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE STEPS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = steps(t, None);
        Some(format!("steps={n}  baker  DRAG:STEPS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = steps(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'o');
                canvas.line(px, py - 2, px, py + 2, 'o');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = steps(t, hands.last().copied());
        Some(format!("STEPS={n}  layers~{}", 1usize << n.min(12)))
    }

    fn reveal(&self) -> &'static str {
        "The baker's map models kneading dough: stretch horizontally, cut, and \
         stack. It is conjugate to a shift on binary sequences and is a standard \
         textbook of deterministic chaos on the square."
    }
}

#[cfg(test)]
mod tests {
    use super::{Baker, baker_step};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Baker::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("STEPS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn steps_change() {
        let r = Baker::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn step_splits() {
        let (x, y) = baker_step(0.25, 0.5);
        assert!((x - 0.5).abs() < 1e-9);
        assert!((y - 0.25).abs() < 1e-9);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Baker::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Baker::new().motif().unwrap().line.len() >= 6);
    }
}
