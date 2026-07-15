//! Mandelbulb Slice: a 2D cut through the classic 3D bulb (power 8 toy).
//!
//! Iterate z^n + c in cylindrical form on a plane. DRAG: AIM THE SLICE.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 24;
const POWER: f64 = 8.0;

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

fn slice_z(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((_, y)) = hand {
        (y - 0.5) * 2.0
    } else {
        (phase_unit(t) - 0.5) * 1.2
    }
}

fn escape(cx: f64, cy: f64, cz: f64) -> u32 {
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut z: f64 = 0.0;
    for i in 0..MAX_ITER {
        let r = (x * x + y * y + z * z).sqrt();
        if r > 2.0 {
            return i;
        }
        // Spherical: r^n, theta*n, phi*n
        let theta = (y.hypot(x)).atan2(z);
        let phi = y.atan2(x);
        let rn = r.powf(POWER);
        let nt = theta * POWER;
        let np = phi * POWER;
        x = rn * nt.sin() * np.cos() + cx;
        y = rn * nt.sin() * np.sin() + cy;
        z = rn * nt.cos() + cz;
    }
    MAX_ITER
}

fn ink(iter: u32) -> char {
    if iter >= MAX_ITER {
        '#'
    } else if iter > 12 {
        '*'
    } else if iter > 6 {
        '+'
    } else if iter > 2 {
        '.'
    } else {
        ' '
    }
}

fn draw(canvas: &mut dyn Surface, cz: f64, zoom: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let zoom = zoom.clamp(0.5, 3.0);
    let off = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    for py in 0..height {
        for px in 0..width {
            let u = px as f64 / width.saturating_sub(1).max(1) as f64;
            let v = py as f64 / height.saturating_sub(1).max(1) as f64;
            let cx = (u - 0.5) * 2.5 / zoom + off;
            let cy = (0.5 - v) * 2.5 / zoom;
            let it = escape(cx, cy, cz);
            let ch = ink(it);
            if ch != ' ' {
                canvas.plot(px as i32, py as i32, ch);
            }
        }
    }
}

/// Mandelbulb slice room.
#[derive(Debug, Default)]
pub struct MandelbulbSlice {
    seed: u64,
}

impl MandelbulbSlice {
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

impl Room for MandelbulbSlice {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mandelbulb-slice",
            title: "Mandelbulb Slice",
            wing: "Fractals",
            blurb: "A plane cut through the power-8 Mandelbulb. t and DRAG: AIM THE SLICE through \
                    the bulb.",
            accent: [160, 80, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let cz = slice_z(t, None);
        let zoom = 0.9 + phase_unit(t) * 0.4;
        draw(canvas, cz, zoom, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bulb slice",
            root: 61.74,
            tempo: 68,
            line: &[0, 4, 7, 11, 16, 11, 7, 4],
            encodes: "a planar cut of a three-dimensional bulb",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: AIM THE SLICE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let cz = slice_z(t, None);
        Some(format!("z={cz:.2}  p=8  DRAG:SLICE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let cz = slice_z(t, hands.last().copied());
        let zoom = hands.last().map(|&(x, _)| 0.6 + x * 2.0).unwrap_or(1.0);
        draw(canvas, cz, zoom, self.seed);
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
        let cz = slice_z(t, hands.last().copied());
        let zoom = hands.last().map(|&(x, _)| 0.6 + x * 2.0).unwrap_or(1.0);
        Some(format!("SLICE z={cz:.2}  zoom={zoom:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "The Mandelbulb lifts Mandelbrot iteration into 3D with a spherical \
         power map. A plane slice is an honest toy of that bulb: same escape \
         time idea, one fixed height through the figure."
    }
}

#[cfg(test)]
mod tests {
    use super::MandelbulbSlice;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MandelbulbSlice::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SLICE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn aim_changes() {
        let r = MandelbulbSlice::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.9,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(32, 24);
        MandelbulbSlice::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(MandelbulbSlice::new().motif().unwrap().line.len() >= 6);
    }
}
