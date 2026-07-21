//! Magnet fractal (type I toy): rational map escape portrait.
//!
//! z -> ((z^2+c-1)/(2z+c-2))^2. DRAG: AIM C. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 30;

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

fn center(t: f64, hand: Option<(f64, f64)>) -> (f64, f64) {
    if let Some((x, y)) = hand {
        ((x - 0.5) * 3.0, (0.5 - y) * 3.0)
    } else {
        let u = phase_unit(t);
        (0.0 + u * 0.3, 0.0)
    }
}

fn escape(cx: f64, cy: f64) -> u32 {
    let mut zx = 0.0f64;
    let mut zy = 0.0f64;
    for i in 0..MAX_ITER {
        // ((z^2 + c - 1) / (2z + c - 2))^2
        let z2r = zx * zx - zy * zy;
        let z2i = 2.0 * zx * zy;
        let num_r = z2r + cx - 1.0;
        let num_i = z2i + cy;
        let den_r = 2.0 * zx + cx - 2.0;
        let den_i = 2.0 * zy + cy;
        let den2 = den_r * den_r + den_i * den_i;
        if den2 < 1e-18 {
            return i;
        }
        let qr = (num_r * den_r + num_i * den_i) / den2;
        let qi = (num_i * den_r - num_r * den_i) / den2;
        let nx = qr * qr - qi * qi;
        let ny = 2.0 * qr * qi;
        zx = nx;
        zy = ny;
        if zx * zx + zy * zy > 100.0 {
            return i;
        }
    }
    MAX_ITER
}

fn ink(iter: u32) -> char {
    if iter >= MAX_ITER {
        '#'
    } else if iter > 15 {
        '*'
    } else if iter > 7 {
        '+'
    } else if iter > 2 {
        '.'
    } else {
        ' '
    }
}

fn draw(canvas: &mut dyn Surface, cx: f64, cy: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.002
    };
    let scale = 1.8;
    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width.saturating_sub(1).max(1) as f64;
            let v = y as f64 / height.saturating_sub(1).max(1) as f64;
            let re = cx - scale + 2.0 * scale * u + j;
            let im = cy + scale - 2.0 * scale * v;
            canvas.plot(x as i32, y as i32, ink(escape(re, im)));
        }
    }
}

/// Magnet fractal room.
#[derive(Debug, Default)]
pub struct MagnetFractal {
    seed: u64,
}

impl MagnetFractal {
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

impl Room for MagnetFractal {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "magnet-fractal",
            title: "Magnet Fractal",
            wing: "Fractals",
            blurb: "Type-I magnet set: rational map escape portrait. t and DRAG: AIM C.",
            accent: [80, 40, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (cx, cy) = center(t, None);
        draw(canvas, cx, cy, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "magnet fractal",
            root: 196.0,
            tempo: 88,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "rational magnet map as an escape set",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: AIM C")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (cx, cy) = center(t, None);
        Some(format!("c=({cx:.2},{cy:.2})  DRAG:AIM"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (cx, cy) = center(t, hands.last().copied());
        draw(canvas, cx, cy, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (cx, cy) = center(t, hands.last().copied());
        let iter = escape(cx, cy);
        Some(format!("c=({cx:.2},{cy:.2}) esc={iter}"))
    }

    fn reveal(&self) -> &'static str {
        "Magnet fractals come from renormalization of Ising models. The type-I \
         map is a rational function whose parameter escape set looks like a \
         Mandelbrot cousin with different bulbs."
    }
}

#[cfg(test)]
mod tests {
    use super::MagnetFractal;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MagnetFractal::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("AIM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn aim_changes() {
        let r = MagnetFractal::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.1,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        MagnetFractal::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(MagnetFractal::new().motif().unwrap().line.len() >= 6);
    }
}
