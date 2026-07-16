//! Function Painter: domain coloring of complex maps.
//!
//! Phase paints the symbol (ASCII stand-in for hue); magnitude paints density.
//! A rack of classic maps (z^2, z^2+c, 1/z, sin z, ...) turns the plate into a
//! museum of the complex plane. DRAG: PICK A MAP / TUNE C. See `docs/ROOMS.md`.

use std::f64::consts::PI;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAPS: [&str; 6] = ["z^2", "z^2+c", "1/z", "sin z", "e^z", "z^3-1"];

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

#[derive(Clone, Copy)]
struct C {
    re: f64,
    im: f64,
}

impl C {
    fn mul(self, o: C) -> C {
        C {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
    fn add(self, o: C) -> C {
        C {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }
    fn inv(self) -> C {
        let n = self.re * self.re + self.im * self.im;
        if n < 1e-18 {
            return C { re: 1e9, im: 1e9 };
        }
        C {
            re: self.re / n,
            im: -self.im / n,
        }
    }
    fn exp(self) -> C {
        let e = self.re.exp();
        C {
            re: e * self.im.cos(),
            im: e * self.im.sin(),
        }
    }
    fn sin(self) -> C {
        // sin(x+iy) = sin x cosh y + i cos x sinh y
        C {
            re: self.re.sin() * self.im.cosh(),
            im: self.re.cos() * self.im.sinh(),
        }
    }
    fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}

fn map_index(t: f64, hand: Option<(f64, f64)>, seed: u64) -> usize {
    if let Some((x, _)) = hand {
        ((x * MAPS.len() as f64) as usize).min(MAPS.len() - 1)
    } else {
        let base = (phase_unit(t) * (MAPS.len() - 1) as f64).round() as usize;
        if seed == 0 {
            base
        } else {
            (base + (seed % MAPS.len() as u64) as usize) % MAPS.len()
        }
    }
}

fn c_param(t: f64, hand: Option<(f64, f64)>) -> C {
    if let Some((_, y)) = hand {
        C {
            re: -0.8 + phase_unit(t) * 0.4,
            im: (y - 0.5) * 1.2,
        }
    } else {
        C {
            re: -0.4 + phase_unit(t) * 0.3,
            im: 0.6 * (phase_unit(t) * PI * 2.0).sin(),
        }
    }
}

fn apply(kind: usize, z: C, c: C) -> C {
    match kind {
        0 => z.mul(z),
        1 => z.mul(z).add(c),
        2 => z.inv(),
        3 => z.sin(),
        4 => z.exp(),
        _ => {
            // z^3 - 1
            let z2 = z.mul(z);
            let z3 = z2.mul(z);
            C {
                re: z3.re - 1.0,
                im: z3.im,
            }
        }
    }
}

fn ink(arg: f64, mag: f64) -> char {
    // Phase bands as symbols; magnitude gates emptiness near zero / infinity.
    if !mag.is_finite() || mag > 50.0 {
        return ' ';
    }
    if mag < 0.05 {
        return '#'; // zero neighborhood
    }
    let sector = ((arg + PI) / (2.0 * PI) * 8.0).floor() as i32;
    match sector.rem_euclid(8) {
        0 => '*',
        1 => '+',
        2 => 'o',
        3 => 'x',
        4 => '=',
        5 => '~',
        6 => ':',
        _ => '.',
    }
}

fn draw(canvas: &mut dyn Surface, kind: usize, c: C, zoom: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let zoom = zoom.clamp(0.4, 4.0);
    for py in 0..height {
        for px in 0..width {
            let u = px as f64 / width.saturating_sub(1).max(1) as f64;
            let v = py as f64 / height.saturating_sub(1).max(1) as f64;
            let re = (u - 0.5) * 4.0 / zoom;
            let im = (0.5 - v) * 3.0 / zoom;
            let z = C { re, im };
            let w = apply(kind, z, c);
            let ch = ink(w.arg(), w.abs());
            if ch != ' ' {
                canvas.plot(px as i32, py as i32, ch);
            }
        }
    }
}

/// Function Painter room.
#[derive(Debug, Default)]
pub struct FunctionPainter {
    seed: u64,
}

impl FunctionPainter {
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

impl Room for FunctionPainter {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "function-painter",
            title: "Function Painter",
            wing: "Fractals",
            blurb: "Domain coloring of complex maps: phase is symbol, magnitude is density. z^2, \
                    z^2+c, 1/z, sin z, e^z, z^3-1. t and DRAG pick the map and tune c.",
            accent: [255, 120, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let kind = map_index(t, None, self.seed);
        let c = c_param(t, None);
        draw(canvas, kind, c, 1.0 + phase_unit(t) * 0.5);
    }

    fn postcard_t(&self) -> f64 {
        0.25
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "domain color",
            root: 415.3,
            tempo: 108,
            line: &[0, 4, 7, 11, 14, 11, 7, 2],
            encodes: "phase and magnitude painting the complex plane",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: PICK A MAP")
    }

    fn status(&self, t: f64) -> Option<String> {
        let kind = map_index(t, None, self.seed);
        let c = c_param(t, None);
        Some(format!(
            "f={}  c=({:.1},{:.1})  DRAG:MAP",
            MAPS[kind], c.re, c.im
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let kind = map_index(t, hands.last().copied(), self.seed);
        let c = c_param(t, hands.last().copied());
        let zoom = hands.last().map(|&(_, y)| 0.6 + y * 2.0).unwrap_or(1.0);
        draw(canvas, kind, c, zoom);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '@');
                canvas.line(px, py - 2, px, py + 2, '@');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let kind = map_index(t, hands.last().copied(), self.seed);
        let c = c_param(t, hands.last().copied());
        // Sample f at the hand as a plate point.
        if let Some(&(x, y)) = hands.last() {
            let re = (x - 0.5) * 4.0;
            let im = (0.5 - y) * 3.0;
            let w = apply(kind, C { re, im }, c);
            return Some(format!(
                "f={}  |w|={:.2}  arg={:.0}",
                MAPS[kind],
                w.abs(),
                w.arg().to_degrees()
            ));
        }
        self.status(t)
    }

    fn reveal(&self) -> &'static str {
        "Domain coloring paints each complex value as a color (here a symbol): \
         phase is the hue, magnitude the brightness. Zeros are pinwheels you can \
         count; poles are the dark infinities. One surface holds every map."
    }
}

#[cfg(test)]
mod tests {
    use super::{C, FunctionPainter, MAPS, apply};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FunctionPainter::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("MAP"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn pick_changes() {
        let r = FunctionPainter::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
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
    fn z_squared_at_i() {
        let w = apply(0, C { re: 0.0, im: 1.0 }, C { re: 0.0, im: 0.0 });
        assert!((w.re + 1.0).abs() < 1e-9);
        assert!(w.im.abs() < 1e-9);
    }

    #[test]
    fn maps_named() {
        assert_eq!(MAPS.len(), 6);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(36, 24);
        FunctionPainter::new().render(&mut c, 0.2);
        assert!(c.ink_count() > 30);
    }

    #[test]
    fn motif_ok() {
        assert!(FunctionPainter::new().motif().unwrap().line.len() >= 6);
    }
}
