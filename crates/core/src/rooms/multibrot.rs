//! Multibrot set: z^d + c for tunable power d.
//!
//! DRAG: TUNE POWER AND WINDOW. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ITER: u32 = 32;

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

fn params(t: f64, hand: Option<(f64, f64)>) -> (f64, f64) {
    // power, zoom
    if let Some((x, y)) = hand {
        (2.0 + x * 6.0, 0.6 + y * 1.8)
    } else {
        let u = phase_unit(t);
        (2.0 + u * 4.0, 1.4 - u * 0.3)
    }
}

fn escape(cx: f64, cy: f64, power: f64) -> u32 {
    let mut zx: f64 = 0.0;
    let mut zy: f64 = 0.0;
    for i in 0..MAX_ITER {
        let r2 = zx * zx + zy * zy;
        if r2 > 4.0 {
            return i;
        }
        let r = r2.sqrt();
        let theta = zy.atan2(zx);
        let rn = r.powf(power);
        let nt = theta * power;
        zx = rn * nt.cos() + cx;
        zy = rn * nt.sin() + cy;
    }
    MAX_ITER
}

fn ink(iter: u32) -> char {
    if iter >= MAX_ITER {
        '#'
    } else if iter > 16 {
        '*'
    } else if iter > 8 {
        '+'
    } else if iter > 2 {
        '.'
    } else {
        ' '
    }
}

fn draw(canvas: &mut dyn Surface, power: f64, zoom: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let zoom = zoom.clamp(0.3, 3.0);
    let jx = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.002
    };
    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width.saturating_sub(1).max(1) as f64;
            let v = y as f64 / height.saturating_sub(1).max(1) as f64;
            let re = -zoom + 2.0 * zoom * u + jx;
            let im = zoom - 2.0 * zoom * v;
            canvas.plot(x as i32, y as i32, ink(escape(re, im, power)));
        }
    }
}

/// Multibrot room.
#[derive(Debug, Default)]
pub struct Multibrot {
    seed: u64,
}

impl Multibrot {
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

impl Room for Multibrot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "multibrot",
            title: "Multibrot",
            wing: "Fractals",
            blurb: "z^d + c: Mandelbrot power raised. t and DRAG: TUNE POWER AND WINDOW.",
            accent: [160, 40, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (p, z) = params(t, None);
        draw(canvas, p, z, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "multibrot",
            root: 220.0,
            tempo: 114,
            line: &[0, 3, 6, 9, 12, 15, 12, 6],
            encodes: "integer power of z before adding c",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE POWER AND WINDOW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (p, _z) = params(t, None);
        Some(format!("d={p:.1}  multibrot  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (p, z) = params(t, hands.last().copied());
        draw(canvas, p, z, self.seed ^ hands.len() as u64);
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
        let (p, z) = params(t, hands.last().copied());
        let z = z.clamp(0.3, 3.0);
        let kind = if (p - 2.0).abs() < 0.08 {
            "Mandel"
        } else if p > 4.0 {
            "star"
        } else {
            "multi"
        };
        // Sample escape at a probe near the main cardioid-ish region.
        let iter = escape(-0.75 / z, 0.0, p);
        Some(format!("d={p:.1}  z={z:.1}  esc={iter}  {kind}"))
    }

    fn reveal(&self) -> &'static str {
        "Multibrot sets replace z^2 with z^d. Higher powers add more bulbs and \
         change the connectedness loci while keeping the escape-time portrait \
         language of Mandelbrot."
    }
}

#[cfg(test)]
mod tests {
    use super::Multibrot;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Multibrot::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Multibrot::new();
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
        Multibrot::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Multibrot::new().motif().unwrap().line.len() >= 6);
    }
}
