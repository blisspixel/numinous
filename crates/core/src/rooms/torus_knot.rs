//! Torus knot T(p,q): winds p times the long way, q the short.
//!
//! DRAG: TUNE P. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

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

fn p_winds(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 3) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 5.0 + s * 0.1
    } else {
        2.0 + phase_unit(t) * 4.0 + s * 0.1
    }
}

fn draw(canvas: &mut dyn Surface, p: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let p = p.round().clamp(2.0, 7.0);
    let q = 3.0 + if seed == 0 { 0.0 } else { (seed % 2) as f64 };
    let r0 = (width.min(height) as f64) * 0.22;
    let r1 = r0 * 0.45;
    let steps = 320;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let t = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r = r0 + r1 * (q * t).cos();
        let x = r * (p * t).cos();
        let y = r * (p * t).sin();
        let z = r1 * (q * t).sin();
        let d = 1.0 / (2.8 + z * 0.02);
        let px = (cx + x * d).round() as i32;
        let py = (cy - y * d * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            let ch = if z > 0.0 { '#' } else { '*' };
            canvas.line(ox, oy, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Torus knot room.
#[derive(Debug, Default)]
pub struct TorusKnot {
    seed: u64,
}

impl TorusKnot {
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

impl Room for TorusKnot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "torus-knot",
            title: "Torus Knot",
            wing: "Shape & Space",
            blurb: "T(p,q) winds the torus both ways. t and DRAG: TUNE P.",
            accent: [120, 50, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, p_winds(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "torus-knot",
            root: 69.3,
            tempo: 82,
            line: &[0, 4, 7, 11, 12, 11, 7, 4],
            encodes: "torus knot T(p,q): p meridional, q longitudinal winds",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = p_winds(t, None, self.seed).round();
        Some(format!("p={p:.0}  T(p,q)  DRAG:P"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = p_winds(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let p = p_winds(t, hands.last().copied(), self.seed)
            .round()
            .clamp(2.0, 7.0);
        let q = 3.0
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 2) as f64
            };
        let pi = p as i32;
        let qi = q as i32;
        // T(p,q) is a single knot iff gcd(p,q)=1; otherwise a link of gcd components.
        let g = {
            let mut a = pi.unsigned_abs();
            let mut b = qi.unsigned_abs();
            while b != 0 {
                let t = b;
                b = a % b;
                a = t;
            }
            a
        };
        let kind = if g == 1 { "knot" } else { "link" };
        Some(format!("T({pi},{qi})  gcd={g}  {kind}"))
    }

    fn reveal(&self) -> &'static str {
        "A torus knot T(p,q) wraps p times around the long way of a torus and q \
         times the short way. When p and q are coprime it is a single knot; the \
         trefoil is T(2,3)."
    }
}

#[cfg(test)]
mod tests {
    use super::TorusKnot;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TorusKnot::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("T(p"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = TorusKnot::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        TorusKnot::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
