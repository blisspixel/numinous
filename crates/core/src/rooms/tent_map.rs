//! Tent map: the simplest piecewise-linear chaos map on [0,1].
//!
//! T_mu(x) = mu min(x, 1-x). Orbit cobweb and density. DRAG: TUNE MU.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBIT: usize = 120;
const DENSITY: usize = 2_000;

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

fn mu(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let m = if let Some((x, _)) = hand {
        1.0 + x * 1.0 + s
    } else {
        1.5 + phase_unit(t) * 0.5 + s
    };
    m.clamp(0.5, 2.0)
}

fn tent(x: f64, mu_v: f64) -> f64 {
    mu_v * x.min(1.0 - x)
}

fn draw(canvas: &mut dyn Surface, mu_v: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Tent graph.
    let mid = (0.5 * width.saturating_sub(1) as f64).round() as i32;
    let peak = (mu_v * 0.5).clamp(0.0, 1.0);
    let y_peak = ((1.0 - peak) * height.saturating_sub(1) as f64).round() as i32;
    let y0 = height.saturating_sub(1) as i32;
    canvas.line(0, y0, mid, y_peak, '#');
    canvas.line(mid, y_peak, width.saturating_sub(1) as i32, y0, '#');
    // Diagonal y=x.
    canvas.line(0, y0, width.saturating_sub(1) as i32, 0, '.');
    // Cobweb orbit.
    let mut x = if seed == 0 {
        0.2
    } else {
        0.1 + (seed % 70) as f64 * 0.01
    };
    let mut prev_px = (x * width.saturating_sub(1) as f64).round() as i32;
    let mut prev_py = y0;
    for i in 0..ORBIT {
        let y = tent(x, mu_v);
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - y.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(
            prev_px,
            prev_py,
            px,
            prev_py,
            if i % 2 == 0 { '*' } else { '+' },
        );
        canvas.line(px, prev_py, px, py, if i % 2 == 0 { '*' } else { '+' });
        // Move along diagonal toward (y,y) for next vertical.
        let dx = (y * width.saturating_sub(1) as f64).round() as i32;
        let dy = ((1.0 - y.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, py, dx, dy, '.');
        prev_px = dx;
        prev_py = dy;
        x = y;
        if !x.is_finite() {
            break;
        }
    }
    // Bottom density strip from a long orbit.
    x = 0.3;
    let mut bins = vec![0u32; width.max(1)];
    for _ in 0..40 {
        x = tent(x, mu_v);
    }
    for _ in 0..DENSITY {
        x = tent(x, mu_v);
        if !x.is_finite() {
            break;
        }
        let b = (x.clamp(0.0, 0.999) * width as f64) as usize;
        if b < bins.len() {
            bins[b] = bins[b].saturating_add(1);
        }
    }
    let max_b = bins.iter().copied().max().unwrap_or(1).max(1);
    let yb = height.saturating_sub(2) as i32;
    for (i, &c) in bins.iter().enumerate() {
        if c * 4 > max_b {
            canvas.plot(i as i32, yb, '|');
        } else if c > 0 {
            canvas.plot(i as i32, yb, '.');
        }
    }
}

/// Tent map room.
#[derive(Debug, Default)]
pub struct TentMap {
    seed: u64,
}

impl TentMap {
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

impl Room for TentMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tent-map",
            title: "The Tent Map",
            wing: "Motion & Dynamics",
            blurb: "Piecewise-linear map on [0,1]: cobweb and density. t and DRAG: TUNE MU.",
            accent: [40, 160, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, mu(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tent",
            root: 174.61,
            tempo: 120,
            line: &[0, 12, 0, 7, 12, 0, 5, 12],
            encodes: "mu times the min of x and one minus x",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE MU")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mu(t, None, self.seed);
        Some(format!("mu={m:.2}  tent  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = mu(t, hands.last().copied(), self.seed);
        draw(canvas, m, self.seed ^ hands.len() as u64);
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
        let m = mu(t, hands.last().copied(), self.seed);
        let chaos = if m > 1.0 { "CHAOS" } else { "ORDER" };
        Some(format!("mu={m:.3}  {chaos}"))
    }

    fn reveal(&self) -> &'static str {
        "The tent map is conjugate to the shift map for mu=2 and is the simplest \
         piecewise-linear engine of chaos. For mu>1 orbits densify; below the \
         critical value they die to a fixed point."
    }
}

#[cfg(test)]
mod tests {
    use super::{TentMap, tent};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TentMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = TentMap::new();
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
    fn tent_peak() {
        assert!((tent(0.5, 2.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        TentMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(TentMap::new().motif().unwrap().line.len() >= 6);
    }
}
