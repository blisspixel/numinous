//! Piriform (pear) curve: y^2 = x^3 (a-x)/b^2.
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

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

fn param_a(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.04
    };
    if let Some((x, _)) = hand {
        0.5 + x * 1.3 + s
    } else {
        0.7 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = a.clamp(0.4, 2.0);
    let b = 1.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.1
        };
    let ox = width as f64 * 0.15;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sx = (width as f64 * 0.7) / a.max(0.4);
    let sy = (height as f64 * 0.35) / b.max(0.5);
    let steps = 200;
    // upper and lower branches
    for sign in [1.0_f64, -1.0] {
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let u = i as f64 / steps as f64;
            let x = a * u;
            let inside = x * x * x * (a - x);
            if inside < 0.0 {
                prev = None;
                continue;
            }
            let y = sign * (inside.sqrt()) / b;
            let px = (ox + x * sx).round() as i32;
            let py = (cy - y * sy).round() as i32;
            if let Some((qx, qy)) = prev {
                canvas.line(qx, qy, px, py, if sign > 0.0 { '#' } else { '*' });
            }
            prev = Some((px, py));
        }
    }
}

/// Piriform room.
#[derive(Debug, Default)]
pub struct Piriform {
    seed: u64,
}

impl Piriform {
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

impl Room for Piriform {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "piriform",
            title: "Piriform Curve",
            wing: "Shape & Space",
            blurb: "Pear curve y^2 = x^3(a-x)/b^2. t and DRAG: TUNE A.",
            accent: [120, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, param_a(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "piriform",
            root: 9.72,
            tempo: 78,
            line: &[0, 2, 7, 9, 12, 9, 7, 2],
            encodes: "piriform pear: cubic algebraic fruit closed at the stem",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = param_a(t, None, self.seed);
        Some(format!("a={a:.2}  pear  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = param_a(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = param_a(t, hands.last().copied(), self.seed).clamp(0.4, 2.0);
        // Pear curve y^2 = x^3 (a-x)/b^2; length along x is a; max height near x=0.75a.
        let xmax = a;
        let ymax = ((0.75_f64.powi(3) * 0.25).sqrt()) * a; // b~1 in draw
        Some(format!("a={a:.2}  L={xmax:.2}  h~{ymax:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The piriform (pear-shaped) curve satisfies y^2 = x^3 (a - x) / b^2. It is \
         a quartic with a cusp-like stem at the origin and a rounded body: an old \
         algebraic fruit studied alongside the cissoid and conchoid."
    }
}

#[cfg(test)]
mod tests {
    use super::Piriform;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Piriform::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("pear"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Piriform::new();
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
        Piriform::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
