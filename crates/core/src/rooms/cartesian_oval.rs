//! Cartesian oval: weighted sum of distances to two foci is constant.
//!
//! DRAG: TUNE WEIGHT. See `docs/ROOMS.md`.

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

fn weight(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.4 + s
    } else {
        0.6 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, m: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let m = m.clamp(0.35, 2.0);
    let f = 0.55
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.05
        };
    let scale = (width.min(height) as f64) * 0.32;
    // m r1 + r2 = k. Sample polar around midpoint.
    let k = m * f + 1.4;
    let steps = 280;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        // Solve for r from focus1 at (-f,0): m |p-F1| + |p-F2| = k
        // p = (r cos, r sin) from origin between foci.
        // Try r via quadratic-ish search.
        let mut found = None;
        for j in 1..80 {
            let r = j as f64 * 0.04;
            let x = r * th.cos();
            let y = r * th.sin();
            let d1 = ((x + f).powi(2) + y * y).sqrt();
            let d2 = ((x - f).powi(2) + y * y).sqrt();
            if (m * d1 + d2 - k).abs() < 0.06 {
                found = Some((x, y));
                break;
            }
        }
        if let Some((x, y)) = found {
            let px = (cx + x * scale).round() as i32;
            let py = (cy - y * scale).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, '#');
            }
            prev = Some((px, py));
        } else {
            prev = None;
        }
    }
    // Foci.
    for sx in [-f, f] {
        let px = (cx + sx * scale).round() as i32;
        let py = cy.round() as i32;
        canvas.line(px - 1, py, px + 1, py, 'o');
        canvas.line(px, py - 1, px, py + 1, 'o');
    }
}

/// Cartesian oval room.
#[derive(Debug, Default)]
pub struct CartesianOval {
    seed: u64,
}

impl CartesianOval {
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

impl Room for CartesianOval {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cartesian-oval",
            title: "Cartesian Oval",
            wing: "Shape & Space",
            blurb: "Weighted sum of distances to two foci. t and DRAG: TUNE WEIGHT.",
            accent: [50, 110, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, weight(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cartesian-oval",
            root: 155.56,
            tempo: 86,
            line: &[0, 3, 5, 8, 10, 12, 8, 3],
            encodes: "m d1 + d2 = const: refraction ovals of Descartes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE WEIGHT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = weight(t, None, self.seed);
        Some(format!("m={m:.2}  oval  DRAG:WT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = weight(t, hands.last().copied(), self.seed);
        draw(canvas, m, self.seed ^ hands.len() as u64);
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
        let m = weight(t, hands.last().copied(), self.seed);
        // m = n1/n2 refractive ratio for Cartesian oval.
        Some(format!("m={m:.2}  n1/n2  oval"))
    }

    fn reveal(&self) -> &'static str {
        "A Cartesian oval is the locus where m d1 + d2 is constant. When m is 1 it \
         is an ordinary ellipse; other m model refraction: rays from one focus \
         refract toward the other through a surface of this shape."
    }
}

#[cfg(test)]
mod tests {
    use super::CartesianOval;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CartesianOval::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("oval"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn weight_changes() {
        let r = CartesianOval::new();
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
        CartesianOval::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
