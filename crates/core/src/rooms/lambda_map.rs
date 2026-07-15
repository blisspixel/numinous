//! Lambda map / logistic in multiplicative form: z -> lambda z (1-z).
//!
//! Complex lambda Julia-style escape for fixed lambda. DRAG: TUNE LAMBDA.
//! See `docs/ROOMS.md`.

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

fn lambda(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, y)) = hand {
        ((x - 0.5) * 4.0 + s, (0.5 - y) * 4.0)
    } else {
        let u = phase_unit(t);
        (0.5 + u * 0.5 + s, 0.5 * (u * std::f64::consts::TAU).sin())
    }
}

fn escape(zx0: f64, zy0: f64, lr: f64, li: f64) -> u32 {
    let mut zx = zx0;
    let mut zy = zy0;
    for i in 0..MAX_ITER {
        if zx * zx + zy * zy > 16.0 {
            return i;
        }
        // z (1-z) = z - z^2
        let z2r = zx * zx - zy * zy;
        let z2i = 2.0 * zx * zy;
        let wr = zx - z2r;
        let wi = zy - z2i;
        // lambda * that
        let nx = lr * wr - li * wi;
        let ny = lr * wi + li * wr;
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

fn draw(canvas: &mut dyn Surface, lr: f64, li: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.002
    };
    let scale = 1.5;
    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width.saturating_sub(1).max(1) as f64;
            let v = y as f64 / height.saturating_sub(1).max(1) as f64;
            let re = -scale + 2.0 * scale * u + j;
            let im = scale - 2.0 * scale * v;
            canvas.plot(x as i32, y as i32, ink(escape(re, im, lr, li)));
        }
    }
}

/// Lambda map room.
#[derive(Debug, Default)]
pub struct LambdaMap {
    seed: u64,
}

impl LambdaMap {
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

impl Room for LambdaMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lambda-map",
            title: "Lambda Map",
            wing: "Fractals",
            blurb: "Complex logistic z -> lambda z(1-z) as Julia portrait. t and DRAG: TUNE \
                    LAMBDA.",
            accent: [40, 140, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (lr, li) = lambda(t, None, self.seed);
        draw(canvas, lr, li, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lambda map",
            root: 261.63,
            tempo: 104,
            line: &[0, 3, 7, 12, 7, 3, 10, 5],
            encodes: "complex logistic parameter as a Julia field",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LAMBDA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (lr, li) = lambda(t, None, self.seed);
        Some(format!("L=({lr:.2},{li:.2})  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (lr, li) = lambda(t, hands.last().copied(), self.seed);
        draw(canvas, lr, li, self.seed ^ hands.len() as u64);
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
        let (lr, li) = lambda(t, hands.last().copied(), self.seed);
        Some(format!("L=({lr:.2},{li:.2})"))
    }

    fn reveal(&self) -> &'static str {
        "The lambda map is the complex logistic family. It is conjugate to the \
         quadratic map z^2+c, so its Julia sets are Mandelbrot cousins under a \
         change of coordinates."
    }
}

#[cfg(test)]
mod tests {
    use super::LambdaMap;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LambdaMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = LambdaMap::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        LambdaMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(LambdaMap::new().motif().unwrap().line.len() >= 6);
    }
}
