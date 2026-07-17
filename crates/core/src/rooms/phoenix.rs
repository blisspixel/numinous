//! Phoenix fractal: Mandelbrot with a memory term of the previous z.
//!
//! z_{n+1} = z_n^2 + c + p * z_{n-1}. DRAG: TUNE P. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 36;

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

fn p_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        -0.8 + x * 1.2 + s
    } else {
        -0.5 + phase_unit(t) * 0.4 + s
    }
}

fn escape(cx: f64, cy: f64, p: f64) -> u32 {
    let mut zx: f64 = 0.0;
    let mut zy: f64 = 0.0;
    let mut px: f64 = 0.0;
    let mut py: f64 = 0.0;
    for i in 0..MAX_ITER {
        if zx * zx + zy * zy > 4.0 {
            return i;
        }
        let nx = zx * zx - zy * zy + cx + p * px;
        let ny = 2.0 * zx * zy + cy + p * py;
        px = zx;
        py = zy;
        zx = nx;
        zy = ny;
    }
    MAX_ITER
}

fn ink(iter: u32) -> char {
    if iter >= MAX_ITER {
        '#'
    } else if iter > 18 {
        '*'
    } else if iter > 8 {
        '+'
    } else if iter > 2 {
        '.'
    } else {
        ' '
    }
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let jx = if seed == 0 {
        0.0
    } else {
        (seed % 8) as f64 * 0.002
    };
    let scale = 1.6;
    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width.saturating_sub(1).max(1) as f64;
            let v = y as f64 / height.saturating_sub(1).max(1) as f64;
            let re = -scale + 2.0 * scale * u + jx;
            let im = scale - 2.0 * scale * v;
            canvas.plot(x as i32, y as i32, ink(escape(re, im, p)));
        }
    }
}

/// Phoenix fractal room.
#[derive(Debug, Default)]
pub struct Phoenix {
    seed: u64,
}

impl Phoenix {
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

impl Room for Phoenix {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "phoenix",
            title: "Phoenix Fractal",
            wing: "Fractals",
            blurb: "Escape set with a one-step memory of z. t and DRAG: TUNE P.",
            accent: [220, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, p_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "phoenix",
            root: 164.81,
            tempo: 106,
            line: &[0, 5, 8, 12, 8, 5, 14, 8],
            encodes: "memory of the previous z reshaping escape",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = p_param(t, None, self.seed);
        Some(format!("p={p:.2}  phoenix  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = p_param(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = p_param(t, hands.last().copied(), self.seed);
        // Phoenix memory term p; sample escape on the negative real probe.
        let iter = escape(-0.5, 0.0, p);
        let band = if p.abs() < 0.2 {
            "weak"
        } else if p > 0.0 {
            "push"
        } else {
            "pull"
        };
        Some(format!("p={p:.2}  esc={iter}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "The Phoenix fractal adds a linear memory of the previous iterate to \
         the Mandelbrot recurrence. The parameter p reshapes filaments and \
         bulbs while keeping escape-time rendering."
    }
}

#[cfg(test)]
mod tests {
    use super::Phoenix;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Phoenix::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Phoenix::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Phoenix::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Phoenix::new().motif().unwrap().line.len() >= 6);
    }
}
