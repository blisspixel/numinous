//! Newton fractal: basins of attraction for root-finding.
//!
//! Newton iteration on p(z)=z^3-1 (or z^n-1) paints which root each seed
//! finds. The boundaries are fractal. DRAG: RAISE THE DEGREE. See
//! `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 24;

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

fn degree(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (2 + (x * 4.0) as u32).clamp(2, 6)
    } else {
        (3 + (phase_unit(t) * 2.0) as u32).clamp(2, 5)
    }
}

/// Newton step for z^n - 1: z - (z^n-1)/(n z^{n-1}) = ((n-1)z + z^{1-n})/n ... use complex.
fn newton_basin(re: f64, im: f64, n: u32) -> (u32, u32) {
    let mut zr = re;
    let mut zi = im;
    for k in 0..MAX_ITER {
        // z^n via polar.
        let r = zr.hypot(zi);
        if r < 1e-12 {
            return (0, k);
        }
        let th = zi.atan2(zr);
        let rn = r.powi(n as i32);
        let rn1 = r.powi(n as i32 - 1);
        // p = z^n - 1, p' = n z^{n-1}
        let pr = rn * (n as f64 * th).cos() - 1.0;
        let pi = rn * (n as f64 * th).sin();
        let dpr = n as f64 * rn1 * ((n as f64 - 1.0) * th).cos();
        let dpi = n as f64 * rn1 * ((n as f64 - 1.0) * th).sin();
        let den = dpr * dpr + dpi * dpi;
        if den < 1e-18 {
            break;
        }
        // (p/p')
        let qr = (pr * dpr + pi * dpi) / den;
        let qi = (pi * dpr - pr * dpi) / den;
        zr -= qr;
        zi -= qi;
        // Near a root of unity?
        for root in 0..n {
            let a = std::f64::consts::TAU * root as f64 / n as f64;
            let rr = a.cos();
            let ri = a.sin();
            if (zr - rr).hypot(zi - ri) < 0.05 {
                return (root + 1, k);
            }
        }
    }
    (0, MAX_ITER)
}

fn ink(root: u32, steps: u32) -> char {
    if root == 0 {
        return '.';
    }
    let bright = steps < 6;
    match (root % 4, bright) {
        (1, true) => '#',
        (1, false) => '*',
        (2, true) => '+',
        (2, false) => 'x',
        (3, true) => 'o',
        (3, false) => '=',
        (_, true) => '@',
        (_, false) => '~',
    }
}

fn draw(canvas: &mut dyn Surface, n: u32, zoom: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let zoom = zoom.clamp(0.5, 3.0);
    for py in 0..height {
        for px in 0..width {
            let u = px as f64 / width.saturating_sub(1).max(1) as f64;
            let v = py as f64 / height.saturating_sub(1).max(1) as f64;
            let re = (u - 0.5) * 3.0 / zoom;
            let im = (0.5 - v) * 2.5 / zoom;
            let (root, steps) = newton_basin(re, im, n);
            canvas.plot(px as i32, py as i32, ink(root, steps));
        }
    }
}

/// Newton fractal room.
#[derive(Debug, Default)]
pub struct Newton {
    seed: u64,
}

impl Newton {
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

impl Room for Newton {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "newton",
            title: "Newton's Basins",
            wing: "Fractals",
            blurb: "Newton's method on z^n-1 paints which root each seed finds. Basin boundaries \
                    are fractal. t and DRAG: RAISE THE DEGREE.",
            accent: [255, 100, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = degree(t, None);
        let zoom = 1.0
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 3) as f64 * 0.1
            };
        draw(canvas, n, zoom);
    }

    fn postcard_t(&self) -> f64 {
        0.2
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "newton basin",
            root: 466.16,
            tempo: 124,
            line: &[0, 3, 7, 10, 14, 10, 7, 3],
            encodes: "each seed falls to a root; the borders never settle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: RAISE THE DEGREE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = degree(t, None);
        Some(format!("n={n}  z^n-1  DRAG:DEGREE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = degree(t, hands.last().copied());
        let zoom = hands.last().map(|&(_, y)| 0.6 + y * 2.0).unwrap_or(1.0);
        draw(canvas, n, zoom);
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
        let n = degree(t, hands.last().copied());
        if let Some(&(x, y)) = hands.last() {
            let re = (x - 0.5) * 3.0;
            let im = (0.5 - y) * 2.5;
            let (root, steps) = newton_basin(re, im, n);
            return Some(format!("n={n}  root={root}  steps={steps}"));
        }
        self.status(t)
    }

    fn reveal(&self) -> &'static str {
        "Newton's method finds roots by iteration. On the complex plane, the \
         set of seeds that fall to each root of z^n-1 forms basins whose \
         shared boundary is a fractal of infinite detail."
    }
}

#[cfg(test)]
mod tests {
    use super::{Newton, newton_basin};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Newton::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEGREE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn degree_changes() {
        let r = Newton::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
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
    fn root_of_unity_found() {
        let (root, steps) = newton_basin(1.0, 0.0, 3);
        assert_eq!(root, 1);
        assert!(steps < 5);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(28, 20);
        Newton::new().render(&mut c, 0.15);
        assert!(c.ink_count() > 50);
    }

    #[test]
    fn motif_ok() {
        assert!(Newton::new().motif().unwrap().line.len() >= 6);
    }
}
