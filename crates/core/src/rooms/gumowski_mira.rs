//! Gumowski-Mira map: accelerator beam dynamics as a strange attractor.
//!
//! x' = y + a(1 - b y^2)y + f(x); y' = -x + f(x'); f(x)=mu x + 2(1-mu)x^2/(1+x^2).
//! DRAG: TUNE MU. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 8_000;

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
        (seed % 7) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        -0.8 + x * 1.0 + s
    } else {
        -0.5 + phase_unit(t) * 0.4 + s
    }
}

fn f(x: f64, mu_v: f64) -> f64 {
    mu_v * x + 2.0 * (1.0 - mu_v) * x * x / (1.0 + x * x)
}

fn draw(canvas: &mut dyn Surface, mu_v: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = 0.008;
    let b = 0.05;
    let mut x: f64 = 0.1;
    let mut y: f64 = 0.1;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let nx = y + a * (1.0 - b * y * y) * y + f(x, mu_v);
        let ny = -x + f(nx, mu_v);
        x = nx;
        y = ny;
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        pts.push((x, y));
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        let ch = if i % 9 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Gumowski-Mira map room.
#[derive(Debug, Default)]
pub struct GumowskiMira {
    seed: u64,
}

impl GumowskiMira {
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

impl Room for GumowskiMira {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gumowski-mira",
            title: "Gumowski-Mira",
            wing: "Motion & Dynamics",
            blurb: "Accelerator beam map that paints butterfly-like attractors. t and DRAG: TUNE \
                    MU.",
            accent: [100, 60, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, mu(t, None, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gumowski",
            root: 164.81,
            tempo: 96,
            line: &[0, 5, 7, 12, 9, 5, 2, 7],
            encodes: "beam nonlinearity becoming a butterfly",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE MU")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mu(t, None, self.seed);
        Some(format!("mu={m:.2}  beam  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = mu(t, hands.last().copied(), self.seed);
        draw(canvas, m);
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
        let a = 0.008;
        let b = 0.05;
        let mut x = 0.1_f64;
        let mut y = 0.1_f64;
        for _ in 0..80 {
            let nx = y + a * (1.0 - b * y * y) * y + f(x, m);
            let ny = -x + f(nx, m);
            x = nx;
            y = ny;
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..500 {
            let nx = y + a * (1.0 - b * y * y) * y + f(x, m);
            let ny = -x + f(nx, m);
            if !nx.is_finite() || !ny.is_finite() {
                break;
            }
            x = nx;
            y = ny;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        Some(format!("mu={m:.2}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Gumowski and Mira studied particle beams in accelerators and wrote a \
         discrete map whose orbits can form butterfly-like strange attractors. \
         Mu steers the nonlinearity of the beam."
    }
}

#[cfg(test)]
mod tests {
    use super::GumowskiMira;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GumowskiMira::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = GumowskiMira::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        GumowskiMira::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(GumowskiMira::new().motif().unwrap().line.len() >= 6);
    }
}
