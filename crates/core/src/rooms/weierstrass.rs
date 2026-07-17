//! Weierstrass function: continuous everywhere, differentiable nowhere.
//!
//! W(x) = sum a^n cos(b^n pi x) with 0<a<1, ab>1+3pi/2 classically.
//! DRAG: TUNE A AND B. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const TERMS: usize = 18;

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, y)) = hand {
        (0.3 + x * 0.5 + s, 3.0 + y * 10.0)
    } else {
        let u = phase_unit(t);
        (0.5 + s, 5.0 + u * 6.0 + s * 5.0)
    }
}

fn weier(x: f64, a: f64, b: f64) -> f64 {
    let mut s = 0.0;
    let mut an = 1.0;
    let mut bn = 1.0;
    for _ in 0..TERMS {
        s += an * (bn * std::f64::consts::PI * x).cos();
        an *= a;
        bn *= b;
        if !an.is_finite() || !bn.is_finite() {
            break;
        }
    }
    s
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let samples = width.saturating_mul(3).max(80);
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 13) as f64 * 0.02
    };
    let mut vals = Vec::with_capacity(samples + 1);
    let mut min_v = f64::MAX;
    let mut max_v = f64::MIN;
    for i in 0..=samples {
        let x = -1.0 + 2.0 * (i as f64 / samples as f64) + shift;
        let v = weier(x, a, b);
        if v.is_finite() {
            min_v = min_v.min(v);
            max_v = max_v.max(v);
        }
        vals.push(v);
    }
    let dv = (max_v - min_v).max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for (i, &v) in vals.iter().enumerate() {
        if !v.is_finite() {
            prev = None;
            continue;
        }
        let u = i as f64 / samples as f64;
        let nv = (v - min_v) / dv;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - (0.08 + 0.84 * nv)) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            let ch = if i % 7 == 0 { '#' } else { '*' };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Weierstrass function room.
#[derive(Debug, Default)]
pub struct Weierstrass {
    seed: u64,
}

impl Weierstrass {
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

impl Room for Weierstrass {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "weierstrass",
            title: "Nowhere Smooth",
            wing: "Fractals",
            blurb: "Weierstrass sum: continuous everywhere, differentiable nowhere. t and DRAG: \
                    TUNE A AND B.",
            accent: [40, 120, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = params(t, None, self.seed);
        draw(canvas, a, b, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "weier",
            root: 246.94,
            tempo: 108,
            line: &[0, 7, 2, 9, 4, 11, 6, 13],
            encodes: "a^n cos(b^n pi x) refusing a tangent",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = params(t, None, self.seed);
        Some(format!("a={a:.2}  b={b:.1}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b, self.seed ^ hands.len() as u64);
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
        let (a, b) = params(t, hands.last().copied(), self.seed);
        let prod = a * b;
        // Continuous nowhere-differentiable when 0<a<1, b odd integer, ab>1+3pi/2...
        // Report ab product against classic threshold 1.
        let rough = if prod > 1.0 {
            "nowhere-diff?"
        } else {
            "milder"
        };
        Some(format!("a={a:.2} b={b:.1}  ab={prod:.2}  {rough}"))
    }

    fn reveal(&self) -> &'static str {
        "Weierstrass (1872) exhibited a function continuous at every point yet \
         differentiable at none: an infinite sum of ever-faster, ever-smaller \
         cosines. Roughness is not noise; it is a precise construction."
    }
}

#[cfg(test)]
mod tests {
    use super::Weierstrass;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Weierstrass::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Weierstrass::new();
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
        Weierstrass::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Weierstrass::new().motif().unwrap().line.len() >= 6);
    }
}
