//! Julia quadratic family map: z -> z^2 + c, filled silhouette.
//!
//! DRAG: TUNE C. See `docs/ROOMS.md`.
//!
//! Distinct from the flagship Julia room: a compact filled set toy for the catalog
//! invent loop (id `julia-filled`).

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

fn c_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, y)) = hand {
        (-1.0 + x * 1.2 + s * 0.1, -0.5 + y + s * 0.05)
    } else {
        let u = phase_unit(t);
        (-0.8 + u * 0.5 + s * 0.1, 0.156 + (u - 0.5) * 0.4 + s * 0.05)
    }
}

fn draw(canvas: &mut dyn Surface, c_re: f64, c_im: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let max_iter = 40 + if seed == 0 { 0 } else { (seed % 10) as u32 };
    for row in 0..height {
        for col in 0..width {
            let x0 = -1.6 + 3.2 * (col as f64) / width.saturating_sub(1).max(1) as f64;
            let y0 = -1.2 + 2.4 * (row as f64) / height.saturating_sub(1).max(1) as f64;
            let mut zx = x0;
            let mut zy = y0;
            let mut i = 0u32;
            while i < max_iter && zx * zx + zy * zy < 4.0 {
                let zx2 = zx * zx - zy * zy + c_re;
                zy = 2.0 * zx * zy + c_im;
                zx = zx2;
                i += 1;
            }
            if i >= max_iter {
                canvas.line(col as i32, row as i32, col as i32, row as i32, '#');
            } else if i > max_iter / 3 {
                canvas.line(col as i32, row as i32, col as i32, row as i32, '.');
            }
        }
    }
}

/// Filled Julia set room.
#[derive(Debug, Default)]
pub struct JuliaFilled {
    seed: u64,
}

impl JuliaFilled {
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

impl Room for JuliaFilled {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "julia-filled",
            title: "Filled Julia",
            wing: "Fractals",
            blurb: "Filled set for z^2+c. t and DRAG: TUNE C.",
            accent: [20, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (cr, ci) = c_param(t, None, self.seed);
        draw(canvas, cr, ci, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "julia-filled",
            root: 87.31,
            tempo: 88,
            line: &[0, 2, 7, 9, 12, 9, 7, 2],
            encodes: "filled Julia: points that never escape under z^2+c",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE C")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (cr, ci) = c_param(t, None, self.seed);
        Some(format!("c={cr:.2}{ci:+.2}i  DRAG:C"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (cr, ci) = c_param(t, hands.last().copied(), self.seed);
        draw(canvas, cr, ci, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (cr, ci) = c_param(t, hands.last().copied(), self.seed);
        // Probe grid: fraction of points that stay bounded (filled measure).
        let max_iter = 40u32;
        let mut filled = 0u32;
        let mut total = 0u32;
        for row in 0..12 {
            for col in 0..16 {
                let x0 = -1.6 + 3.2 * (col as f64) / 15.0;
                let y0 = -1.2 + 2.4 * (row as f64) / 11.0;
                let mut zx = x0;
                let mut zy = y0;
                let mut i = 0u32;
                while i < max_iter && zx * zx + zy * zy < 4.0 {
                    let zx2 = zx * zx - zy * zy + cr;
                    zy = 2.0 * zx * zy + ci;
                    zx = zx2;
                    i += 1;
                }
                if i >= max_iter {
                    filled += 1;
                }
                total += 1;
            }
        }
        let pct = if total > 0 {
            (100.0 * filled as f64 / total as f64).round() as i32
        } else {
            0
        };
        Some(format!("c=({cr:.2},{ci:.2})  fill~{pct}%"))
    }

    fn reveal(&self) -> &'static str {
        "For each complex c the filled Julia set is the points that stay bounded \
         under iteration of z |-> z^2 + c. Connected Julia sets sit over the \
         Mandelbrot set; dust-like ones sit outside it."
    }
}

#[cfg(test)]
mod tests {
    use super::JuliaFilled;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = JuliaFilled::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("c="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn c_changes() {
        let r = JuliaFilled::new();
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
        JuliaFilled::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}
