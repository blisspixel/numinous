//! Kepler's second law: equal areas in equal times on an ellipse.
//!
//! DRAG: TUNE ECC. See `docs/ROOMS.md`.

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

fn ecc(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        (x * 0.85 + s).clamp(0.0, 0.9)
    } else {
        (phase_unit(t) * 0.75 + s).clamp(0.0, 0.9)
    }
}

fn draw(canvas: &mut dyn Surface, e: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let e = e.clamp(0.0, 0.9);
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.4;
    let b = a * (1.0 - e * e).sqrt();
    // focus
    let c = a * e;
    let fx = cx - c;
    // ellipse
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=120 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 120.0);
        let px = (cx + a * th.cos()).round() as i32;
        let py = (cy - b * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // sun at focus
    let fxi = fx.round() as i32;
    let fyi = cy.round() as i32;
    canvas.line(fxi - 1, fyi, fxi + 1, fyi, 'o');
    canvas.line(fxi, fyi - 1, fxi, fyi + 1, 'o');
    // equal-area sectors: equal true-anomaly steps are wrong; equal mean anomaly
    // Approximate with equal eccentric anomaly chunks for area feeling
    let n_sec = 6 + if seed == 0 { 0 } else { (seed % 2) as i32 };
    for s in 0..n_sec {
        let e1 = 2.0 * std::f64::consts::PI * (s as f64) / n_sec as f64;
        let e2 = 2.0 * std::f64::consts::PI * ((s + 1) as f64) / n_sec as f64;
        let x1 = cx + a * e1.cos();
        let y1 = cy - b * e1.sin() * 0.55;
        let x2 = cx + a * e2.cos();
        let y2 = cy - b * e2.sin() * 0.55;
        canvas.line(fxi, fyi, x1.round() as i32, y1.round() as i32, '.');
        canvas.line(fxi, fyi, x2.round() as i32, y2.round() as i32, '.');
        // chord of sector
        canvas.line(
            x1.round() as i32,
            y1.round() as i32,
            x2.round() as i32,
            y2.round() as i32,
            if s % 2 == 0 { '*' } else { '+' },
        );
    }
}

/// Kepler equal-area room.
#[derive(Debug, Default)]
pub struct KeplerLaws {
    seed: u64,
}

impl KeplerLaws {
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

impl Room for KeplerLaws {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kepler-laws",
            title: "Kepler Areas",
            wing: "Motion & Dynamics",
            blurb: "Equal areas in equal times on an ellipse. t and DRAG: TUNE ECC.",
            accent: [100, 70, 30],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ecc(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kepler-laws",
            root: 8.18,
            tempo: 80,
            line: &[0, 3, 7, 12, 7, 3, 0, 12],
            encodes: "Kepler II: radius to the sun sweeps equal areas per time",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ECC")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = ecc(t, None, self.seed);
        Some(format!("e={e:.2}  areas  DRAG:ECC"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = ecc(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
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
        let e = ecc(t, hands.last().copied(), self.seed);
        // peri/aphelion distance ratio (1-e)/(1+e) for a=1
        let ra_rp = if e < 0.99 {
            (1.0 + e) / (1.0 - e)
        } else {
            f64::INFINITY
        };
        if ra_rp.is_finite() {
            Some(format!("e={e:.3}  ra/rp={ra_rp:.2}  areas"))
        } else {
            Some(format!("e={e:.3}  parabolic"))
        }
    }

    fn reveal(&self) -> &'static str {
        "Kepler's second law: the line from the sun to a planet sweeps equal areas \
         in equal times. Near perihelion the planet races; near aphelion it crawls. \
         Angular momentum conservation is the modern reason."
    }
}

#[cfg(test)]
mod tests {
    use super::KeplerLaws;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = KeplerLaws::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("areas"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ecc_changes() {
        let r = KeplerLaws::new();
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
        KeplerLaws::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
