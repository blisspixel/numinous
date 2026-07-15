//! Tricorn (Mandelbar): Mandelbrot with conjugate squaring.
//!
//! z -> conjugate(z)^2 + c. DRAG: AIM THE WINDOW. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 40;

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

fn window(t: f64, hand: Option<(f64, f64)>) -> (f64, f64, f64) {
    if let Some((x, y)) = hand {
        (-1.0 + x * 2.0, (0.5 - y) * 2.0, 0.5 + y * 1.5)
    } else {
        let u = phase_unit(t);
        (-0.5 + u * 0.3, 0.0, 1.5 - u * 0.4)
    }
}

fn escape(cx: f64, cy: f64) -> u32 {
    let mut zx: f64 = 0.0;
    let mut zy: f64 = 0.0;
    for i in 0..MAX_ITER {
        if zx * zx + zy * zy > 4.0 {
            return i;
        }
        // conjugate square: (x - i y)^2 = x^2 - y^2 - 2 i x y
        let nx = zx * zx - zy * zy + cx;
        let ny = -2.0 * zx * zy + cy;
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

fn draw(canvas: &mut dyn Surface, cx: f64, cy: f64, scale: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let scale = scale.clamp(0.2, 3.0);
    let jx = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.002
    };
    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width.saturating_sub(1).max(1) as f64;
            let v = y as f64 / height.saturating_sub(1).max(1) as f64;
            let re = cx - scale + 2.0 * scale * u + jx;
            let im = cy + scale - 2.0 * scale * v;
            canvas.plot(x as i32, y as i32, ink(escape(re, im)));
        }
    }
}

/// Tricorn room.
#[derive(Debug, Default)]
pub struct Tricorn {
    seed: u64,
}

impl Tricorn {
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

impl Room for Tricorn {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tricorn",
            title: "Tricorn",
            wing: "Fractals",
            blurb: "Mandelbar set: conjugate squaring, three-lobed body. t and DRAG: AIM THE \
                    WINDOW.",
            accent: [120, 60, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (cx, cy, s) = window(t, None);
        draw(canvas, cx, cy, s, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tricorn",
            root: 138.59,
            tempo: 100,
            line: &[0, 4, 8, 12, 8, 4, 0, 12],
            encodes: "conjugate squaring with three main lobes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: AIM THE WINDOW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (cx, cy, _s) = window(t, None);
        Some(format!("c=({cx:.2},{cy:.2})  DRAG:AIM"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (cx, cy, s) = window(t, hands.last().copied());
        draw(canvas, cx, cy, s, self.seed ^ hands.len() as u64);
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
        let (cx, cy, s) = window(t, hands.last().copied());
        Some(format!("AIM ({cx:.2},{cy:.2}) sc={s:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The tricorn (Mandelbar) set uses the conjugate of z before squaring. \
         The main body has three lobes instead of Mandelbrot's cardioid-and-disk \
         structure: same escape idea, flipped conjugation."
    }
}

#[cfg(test)]
mod tests {
    use super::Tricorn;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Tricorn::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("AIM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn aim_changes() {
        let r = Tricorn::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.1,
                    y: 0.9,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Tricorn::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Tricorn::new().motif().unwrap().line.len() >= 6);
    }
}
