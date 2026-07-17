//! Logistic map cobweb gallery variant: two r values side by side.
//!
//! Distinct from logistic-cobweb. DRAG: TUNE R. See `docs/ROOMS.md`.

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

fn r_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        2.5 + x * 1.5 + s
    } else {
        2.8 + phase_unit(t) * 1.1 + s
    }
}

fn cobweb(canvas: &mut dyn Surface, r: f64, x0: f64, x_off: i32, w: i32, h: i32) {
    // parabola y = r x (1-x) and diagonal in a square panel
    let steps = 40;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let x = i as f64 / steps as f64;
        let y = r * x * (1.0 - x) / 4.0; // scale into [0,1] roughly for r<=4
        let y = y.clamp(0.0, 1.0);
        let px = x_off + (x * (w - 1) as f64).round() as i32;
        let py = ((1.0 - y) * (h - 1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '*');
        }
        prev = Some((px, py));
    }
    // diagonal
    canvas.line(x_off, h - 1, x_off + w - 1, 0, '.');
    // orbit
    let mut x = x0;
    let mut px = x_off + (x * (w - 1) as f64).round() as i32;
    let mut py = h - 1;
    for _ in 0..24 {
        let y = (r * x * (1.0 - x)).clamp(0.0, 1.0);
        let qx = x_off + (x * (w - 1) as f64).round() as i32;
        let qy = ((1.0 - y) * (h - 1) as f64).round() as i32;
        canvas.line(px, py, qx, qy, '#');
        let rx = x_off + (y * (w - 1) as f64).round() as i32;
        canvas.line(qx, qy, rx, qy, '#');
        px = rx;
        py = ((1.0 - y) * (h - 1) as f64).round() as i32;
        x = y;
    }
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mid = width as i32 / 2;
    let h = height as i32;
    let x0 = 0.2
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.05
        };
    cobweb(canvas, r, x0, 1, mid - 2, h);
    cobweb(canvas, (r + 0.3).min(4.0), 1.0 - x0, mid + 1, mid - 2, h);
}

/// Dual cobweb room.
#[derive(Debug, Default)]
pub struct DualCobweb {
    seed: u64,
}

impl DualCobweb {
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

impl Room for DualCobweb {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "dual-cobweb",
            title: "Dual Cobweb",
            wing: "Motion & Dynamics",
            blurb: "Two logistic cobwebs at neighboring r. t and DRAG: TUNE R.",
            accent: [180, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, r_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dual cobweb",
            root: 622.25,
            tempo: 96,
            line: &[0, 5, 7, 12, 7, 5, 0, 10],
            encodes: "neighbor r values show order beside chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r_param(t, None, self.seed);
        Some(format!("r={r:.2}  dual  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r_param(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
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
        let r = r_param(t, hands.last().copied(), self.seed);
        let r2 = (r + 0.3).min(4.0);
        // Logistic fixed-point stability for |r(1-2x*)| with x*=1-1/r when r>1.
        let band = if r < 3.0 {
            "period1"
        } else if r < 3.45 {
            "period2"
        } else if r < 3.57 {
            "cascade"
        } else {
            "chaos"
        };
        Some(format!("r={r:.2}/{r2:.2}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "A cobweb plot walks the logistic map against the diagonal. Two panels \
         at neighboring r show how a small parameter shift can turn a settled \
         fixed point into a period-doubling cascade."
    }
}

#[cfg(test)]
mod tests {
    use super::DualCobweb;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DualCobweb::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("R"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = DualCobweb::new();
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
        DualCobweb::new().render(&mut c, 0.7);
        assert!(c.ink_count() > 0);
    }
}
