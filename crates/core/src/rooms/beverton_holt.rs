//! Beverton-Holt map: discrete population with saturating recruitment.
//!
//! x -> r x / (1 + x). DRAG: TUNE R. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 80;

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

fn r_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        0.5 + x * 4.0 + s
    } else {
        1.0 + phase_unit(t) * 3.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // bifurcation-style: columns are r, vertical is attractor
    let r0 = 0.5;
    let r1 = 5.0;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    for col in 0..width {
        let rr = r0 + (r1 - r0) * (col as f64 / width.saturating_sub(1).max(1) as f64) + j;
        let mut x = 0.5;
        for _ in 0..40 {
            x = rr * x / (1.0 + x);
        }
        for _ in 0..30 {
            x = rr * x / (1.0 + x);
            let y = (x / (rr.max(1.0))).clamp(0.0, 1.0);
            let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
            canvas.plot(col as i32, py, '.');
        }
    }
    // highlight current r orbit
    let u = ((r - r0) / (r1 - r0)).clamp(0.0, 1.0);
    let px = (u * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(px, 0, px, height.saturating_sub(1) as i32, '|');
    let mut x = 0.3;
    let mut prev_y: Option<i32> = None;
    for i in 0..STEPS {
        x = r * x / (1.0 + x);
        let y = (x / r.max(1.0)).clamp(0.0, 1.0);
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        let qx = ((i as f64 / STEPS as f64) * width.saturating_sub(1) as f64).round() as i32;
        if let Some(oy) = prev_y {
            canvas.line(qx.saturating_sub(1), oy, qx, py, '#');
        }
        prev_y = Some(py);
    }
}

/// Beverton-Holt room.
#[derive(Debug, Default)]
pub struct BevertonHolt {
    seed: u64,
}

impl BevertonHolt {
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

impl Room for BevertonHolt {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "beverton-holt",
            title: "Beverton-Holt",
            wing: "Motion & Dynamics",
            blurb: "Saturating recruitment map for a fishery. t and DRAG: TUNE R.",
            accent: [40, 140, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, r_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "beverton holt",
            root: 659.25,
            tempo: 88,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "population that saturates instead of exploding",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r_param(t, None, self.seed);
        Some(format!("r={r:.2}  BH  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r_param(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let r = r_param(t, hands.last().copied(), self.seed);
        // equilibrium x* = r-1 for r>1
        let eq = (r - 1.0).max(0.0);
        Some(format!("R={r:.3}  eq~{eq:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Beverton-Holt map models a population with limited recruitment: \
         x -> r x/(1+x). Unlike the logistic map it never chaos-doubles; for \
         r>1 it settles to a stable equilibrium r-1."
    }
}

#[cfg(test)]
mod tests {
    use super::BevertonHolt;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = BevertonHolt::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("R"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = BevertonHolt::new();
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
        BevertonHolt::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
