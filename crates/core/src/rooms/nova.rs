//! Nova fractal: Newton-like rational map portrait (z - R (z^p-1)/(p z^{p-1}))^2 + c toy.
//!
//! DRAG: TUNE POWER. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 28;

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

fn power(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((x, _)) = hand {
        2.0 + x * 5.0
    } else {
        3.0 + phase_unit(t) * 3.0
    }
}

fn escape(cx: f64, cy: f64, p: f64) -> u32 {
    let mut zx = cx;
    let mut zy = cy;
    let r = 1.0;
    for i in 0..MAX_ITER {
        let r2 = zx * zx + zy * zy;
        if !(1e-12..=1e6).contains(&r2) {
            return i;
        }
        // z^p via polar
        let rad = r2.sqrt();
        let th = zy.atan2(zx);
        let zp_r = rad.powf(p);
        let zp_t = th * p;
        let zpr = zp_r * zp_t.cos();
        let zpi = zp_r * zp_t.sin();
        // z^{p-1}
        let zm1_r = rad.powf(p - 1.0);
        let zm1_t = th * (p - 1.0);
        let zm1r = zm1_r * zm1_t.cos();
        let zm1i = zm1_r * zm1_t.sin();
        // (z^p - 1) / (p z^{p-1})
        let num_r = zpr - 1.0;
        let num_i = zpi;
        let den_r = p * zm1r;
        let den_i = p * zm1i;
        let den2 = den_r * den_r + den_i * den_i;
        if den2 < 1e-18 {
            return i;
        }
        let qr = (num_r * den_r + num_i * den_i) / den2;
        let qi = (num_i * den_r - num_r * den_i) / den2;
        let wr = zx - r * qr;
        let wi = zy - r * qi;
        // square + c (c = start point for nova parameter space portrait)
        let nx = wr * wr - wi * wi + cx;
        let ny = 2.0 * wr * wi + cy;
        zx = nx;
        zy = ny;
    }
    MAX_ITER
}

fn ink(iter: u32) -> char {
    if iter >= MAX_ITER {
        '#'
    } else if iter > 14 {
        '*'
    } else if iter > 6 {
        '+'
    } else if iter > 1 {
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
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.002
    };
    let scale = 1.2;
    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width.saturating_sub(1).max(1) as f64;
            let v = y as f64 / height.saturating_sub(1).max(1) as f64;
            let re = -scale + 2.0 * scale * u + j;
            let im = scale - 2.0 * scale * v;
            canvas.plot(x as i32, y as i32, ink(escape(re, im, p)));
        }
    }
}

/// Nova fractal room.
#[derive(Debug, Default)]
pub struct Nova {
    seed: u64,
}

impl Nova {
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

impl Room for Nova {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "nova",
            title: "Nova Fractal",
            wing: "Fractals",
            blurb: "Newton-style rational map as an escape portrait. t and DRAG: TUNE POWER.",
            accent: [200, 80, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, power(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "nova",
            root: 349.23,
            tempo: 98,
            line: &[0, 5, 10, 15, 10, 5, 12, 0],
            encodes: "rational Newton step squared into escape time",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE POWER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = power(t, None);
        Some(format!("p={p:.1}  nova  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = power(t, hands.last().copied());
        draw(canvas, p, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = power(t, hands.last().copied());
        // Newton-style nova for z^p - 1; sample escape near a p-th root of unity.
        let th = std::f64::consts::TAU / p.max(1.0);
        let iter = escape(0.7 * th.cos(), 0.7 * th.sin(), p);
        Some(format!("p={p:.1}  esc={iter}  Newton"))
    }

    fn reveal(&self) -> &'static str {
        "Nova fractals blend Newton iteration toward roots of unity with a \
         Mandelbrot-style parameter portrait. Power p changes the petal count \
         of the rational map."
    }
}

#[cfg(test)]
mod tests {
    use super::Nova;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Nova::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Nova::new();
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
        Nova::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Nova::new().motif().unwrap().line.len() >= 6);
    }
}
