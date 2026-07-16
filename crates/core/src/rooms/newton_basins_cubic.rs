//! Newton on z^3-1 is already in newton.rs; this room is the cubic map
//! as a pure attractor portrait with hand-set c for z^3 + c.
//!
//! DRAG: TUNE C. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 20;

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
    if let Some((x, y)) = hand {
        ((x - 0.5) * 2.0, (0.5 - y) * 2.0)
    } else {
        let u = phase_unit(t);
        let s = if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.05
        };
        (-0.5 + u * 0.4 + s, 0.3 * (u * std::f64::consts::TAU).sin())
    }
}

/// Newton for p(z)=z^3+c: z - p/p' = (2z + c/z^2)/3 when z!=0.
fn basin(re: f64, im: f64, cr: f64, ci: f64) -> (u32, u32) {
    let mut zr = re;
    let mut zi = im;
    for k in 0..MAX_ITER {
        let r2 = zr * zr + zi * zi;
        if r2 < 1e-14 {
            return (0, k);
        }
        // z^2
        let z2r = zr * zr - zi * zi;
        let z2i = 2.0 * zr * zi;
        // z^3
        let z3r = z2r * zr - z2i * zi;
        let z3i = z2r * zi + z2i * zr;
        // p = z^3 + c
        let pr = z3r + cr;
        let pi = z3i + ci;
        // p' = 3 z^2
        let dpr = 3.0 * z2r;
        let dpi = 3.0 * z2i;
        let den = dpr * dpr + dpi * dpi;
        if den < 1e-18 {
            break;
        }
        let qr = (pr * dpr + pi * dpi) / den;
        let qi = (pi * dpr - pr * dpi) / den;
        zr -= qr;
        zi -= qi;
        // classify by arg of z near a cube root attractor of fixed point of Newton
        if zr * zr + zi * zi > 100.0 {
            return (0, k);
        }
        // three sectors by argument
        let arg = zi.atan2(zr);
        let sector = ((arg + std::f64::consts::PI) / (std::f64::consts::TAU / 3.0)).floor() as i32;
        if k > 4 && (zr * zr + zi * zi - 1.0).abs() < 0.15 {
            return ((sector.rem_euclid(3) as u32) + 1, k);
        }
    }
    let arg = zi.atan2(zr);
    let sector = ((arg + std::f64::consts::PI) / (std::f64::consts::TAU / 3.0)).floor() as i32;
    ((sector.rem_euclid(3) as u32) + 1, MAX_ITER)
}

fn ink(root: u32, steps: u32) -> char {
    if root == 0 {
        return '.';
    }
    let bright = steps < 8;
    match (root, bright) {
        (1, true) => '#',
        (1, false) => '*',
        (2, true) => '+',
        (2, false) => 'x',
        (_, true) => 'o',
        (_, false) => '=',
    }
}

fn draw(canvas: &mut dyn Surface, cr: f64, ci: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for py in 0..height {
        for px in 0..width {
            let u = px as f64 / width.saturating_sub(1).max(1) as f64;
            let v = py as f64 / height.saturating_sub(1).max(1) as f64;
            let re = (u - 0.5) * 3.0;
            let im = (0.5 - v) * 2.5;
            let (root, steps) = basin(re, im, cr, ci);
            canvas.plot(px as i32, py as i32, ink(root, steps));
        }
    }
}

/// Cubic Newton portrait room.
#[derive(Debug, Default)]
pub struct NewtonCubic {
    seed: u64,
}

impl NewtonCubic {
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

impl Room for NewtonCubic {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "newton-cubic",
            title: "Cubic Newton",
            wing: "Fractals",
            blurb: "Newton basins for z^3+c: three attractors paint a cubic portrait. t and DRAG: \
                    TUNE C.",
            accent: [255, 90, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (cr, ci) = c_param(t, None, self.seed);
        draw(canvas, cr, ci);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cubic newton",
            root: 349.23,
            tempo: 118,
            line: &[0, 3, 6, 9, 12, 15, 12, 6],
            encodes: "three basins of a cubic Newton map",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE C")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (cr, ci) = c_param(t, None, self.seed);
        Some(format!("c=({cr:.2},{ci:.2})  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (cr, ci) = c_param(t, hands.last().copied(), self.seed);
        draw(canvas, cr, ci);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'O');
                canvas.line(px, py - 2, px, py + 2, 'O');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (cr, ci) = c_param(t, hands.last().copied(), self.seed);
        let (root, steps) = basin(0.5, 0.3, cr, ci);
        Some(format!("c=({cr:.2},{ci:.2})  root={root} it={steps}"))
    }

    fn reveal(&self) -> &'static str {
        "Newton's method on a cubic polynomial has three roots (counting \
         multiplicity). Basins of attraction meet in fractal boundaries: the \
         portrait of which seed finds which root."
    }
}

#[cfg(test)]
mod tests {
    use super::NewtonCubic;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = NewtonCubic::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = NewtonCubic::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(28, 20);
        NewtonCubic::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 50);
    }

    #[test]
    fn motif_ok() {
        assert!(NewtonCubic::new().motif().unwrap().line.len() >= 6);
    }
}
