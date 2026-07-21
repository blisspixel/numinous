//! Lambert W: inverse of f(w)=w e^w, principal branch.
//!
//! DRAG: TUNE X. See `docs/ROOMS.md`.

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

fn arg(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.05
    };
    // domain of W0: [-1/e, +inf)
    if let Some((x, _)) = hand {
        -0.35 + x * 3.5 + s
    } else {
        -0.3 + phase_unit(t) * 3.0 + s
    }
}

/// Newton for principal Lambert W.
fn lambert_w0(x: f64) -> f64 {
    let xmin = -1.0 / std::f64::consts::E;
    if x < xmin {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if (x - xmin).abs() < 1e-12 {
        return -1.0;
    }
    // initial guess for Newton
    let mut w = if x < 0.0 {
        -0.5 + (x - xmin).sqrt()
    } else if x < 2.0 {
        x * 0.5
    } else {
        x.ln() - x.ln().ln().max(0.0)
    };
    for _ in 0..20 {
        let ew = w.exp();
        let f = w * ew - x;
        let fp = ew * (w + 1.0);
        if fp.abs() < 1e-14 {
            break;
        }
        let nw = w - f / fp;
        if (nw - w).abs() < 1e-12 {
            w = nw;
            break;
        }
        w = nw;
    }
    w
}

fn draw(canvas: &mut dyn Surface, m: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let xmin = -1.0 / std::f64::consts::E;
    let x0 = xmin - 0.2;
    let x1 = 4.0;
    // plot y = x e^x
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let w = -3.0 + 4.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let x = w * w.exp();
        if x < x0 || x > x1 || !x.is_finite() {
            prev = None;
            continue;
        }
        let px = ((x - x0) / (x1 - x0) * width.saturating_sub(1) as f64).round() as i32;
        let py = ((0.5 - w / 4.0) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // W0 branch
    prev = None;
    for col in 0..width {
        let x = x0 + (x1 - x0) * (col as f64) / width.saturating_sub(1).max(1) as f64;
        if x < xmin {
            prev = None;
            continue;
        }
        let w = lambert_w0(x);
        if !w.is_finite() {
            prev = None;
            continue;
        }
        let py = ((0.5 - w / 4.0) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    let mx = ((m - x0) / (x1 - x0) * width.saturating_sub(1) as f64)
        .round()
        .clamp(0.0, width.saturating_sub(1) as f64) as i32;
    canvas.line(mx, 0, mx, height as i32 - 1, '|');
    let _ = seed;
}

/// Lambert W room.
#[derive(Debug, Default)]
pub struct LambertW {
    seed: u64,
}

impl LambertW {
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

impl Room for LambertW {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lambert-w",
            title: "Lambert W",
            wing: "Analysis",
            blurb: "Inverse of w e^w, principal branch. t and DRAG: TUNE X.",
            accent: [90, 80, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, arg(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lambert-w",
            root: 277.18,
            tempo: 70,
            line: &[0, 4, 7, 9, 7, 4, 0, 12],
            encodes: "Lambert W0: inverse of w exp(w), branch at -1/e",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE X")
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = arg(t, None, self.seed);
        let w = lambert_w0(x);
        if w.is_finite() {
            Some(format!("x={x:.2}  W={w:.2}  DRAG:X"))
        } else {
            Some(format!("x={x:.2}  off  DRAG:X"))
        }
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let x = arg(t, hands.last().copied(), self.seed);
        draw(canvas, x, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let x = arg(t, hands.last().copied(), self.seed);
        let w = lambert_w0(x);
        if w.is_finite() {
            Some(format!("X={x:.3}  W={w:.3}"))
        } else {
            Some(format!("X={x:.3}  off"))
        }
    }

    fn reveal(&self) -> &'static str {
        "The Lambert W function is the inverse of f(w)=w e^w. The principal branch \
         W0 is real on [-1/e, +inf). It appears in delay equations, tree enumeration, \
         and any place an exponential and a linear term tangle."
    }
}

#[cfg(test)]
mod tests {
    use super::LambertW;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LambertW::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains('W'));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn x_changes() {
        let r = LambertW::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        LambertW::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
