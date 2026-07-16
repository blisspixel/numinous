//! Van der Pol oscillator: nonlinear damping, stable limit cycle.
//!
//! x'' - mu (1 - x^2) x' + x = 0. DRAG: TUNE MU. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 5_000;
const DT: f64 = 0.02;

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
        (seed % 6) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        0.2 + x * 4.5 + s
    } else {
        0.5 + phase_unit(t) * 3.0 + s
    }
}

fn integrate(mu_v: f64, x0: f64, y0: f64) -> Vec<(f64, f64)> {
    let mut x = x0;
    let mut y = y0;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        // y = x', y' = mu(1-x^2)y - x
        let dy = mu_v * (1.0 - x * x) * y - x;
        let dx = y;
        x += DT * dx;
        y += DT * dy;
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        out.push((x, y));
    }
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)], seed_path: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in pts.iter().chain(seed_path.iter()) {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        let u = 0.08 + 0.84 * (x - min_x) / dx;
        let v = 0.08 + 0.84 * (y - min_y) / dy;
        (
            (u * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let mut prev: Option<(i32, i32)> = None;
    for (i, &p) in pts.iter().enumerate() {
        let q = to_px(p.0, p.1);
        if let Some(o) = prev {
            let ch = if i + 200 > pts.len() { '#' } else { '*' };
            canvas.line(o.0, o.1, q.0, q.1, ch);
        }
        prev = Some(q);
    }
    // Secondary orbit from a different start (dashed lighter).
    prev = None;
    for &p in seed_path {
        let q = to_px(p.0, p.1);
        if let Some(o) = prev {
            canvas.line(o.0, o.1, q.0, q.1, '.');
        }
        prev = Some(q);
    }
}

/// Van der Pol room.
#[derive(Debug, Default)]
pub struct VanDerPol {
    seed: u64,
}

impl VanDerPol {
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

impl Room for VanDerPol {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "van-der-pol",
            title: "Van der Pol Cycle",
            wing: "Motion & Dynamics",
            blurb: "Nonlinear damping births a stable limit cycle. t and DRAG: TUNE MU.",
            accent: [220, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let m = mu(t, None, self.seed);
        let main = integrate(m, 0.1, 0.0);
        let other = integrate(m, 2.5, 0.0);
        draw(canvas, &main, &other);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "vdp",
            root: 130.81,
            tempo: 100,
            line: &[0, 3, 5, 7, 10, 7, 5, 3],
            encodes: "soft start, hard relaxation on a limit cycle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE MU")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mu(t, None, self.seed);
        Some(format!("mu={m:.2}  cycle  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = mu(t, hands.last().copied(), self.seed);
        let (x0, y0) = hands
            .last()
            .map(|&(x, y)| ((x - 0.5) * 4.0, (0.5 - y) * 4.0))
            .unwrap_or((0.1, 0.0));
        let main = integrate(m, x0, y0);
        let other = integrate(m, 0.1, 0.0);
        draw(canvas, &main, &other);
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
        let (x0, y0) = hands
            .last()
            .map(|&(x, y)| ((x - 0.5) * 4.0, (0.5 - y) * 4.0))
            .unwrap_or((0.1, 0.0));
        Some(format!("mu={m:.2}  start=({x0:.1},{y0:.1})"))
    }

    fn reveal(&self) -> &'static str {
        "Van der Pol's equation models a triode circuit with nonlinear damping: \
         energy is pumped in near the origin and dissipated at large amplitude. \
         Trajectories forget their start and lock onto a stable limit cycle."
    }
}

#[cfg(test)]
mod tests {
    use super::VanDerPol;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = VanDerPol::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = VanDerPol::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        VanDerPol::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(VanDerPol::new().motif().unwrap().line.len() >= 6);
    }
}
