//! Ikeda map: complex dissipative chaos from laser optics.
//!
//! z_{n+1} = a + b z_n exp(i (c - d/(1+|z|^2))). DRAG: TUNE A AND B.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 7_000;

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    // Classic a=1.0, b=0.9, c=0.4, d=6
    if let Some((x, y)) = hand {
        (0.6 + x * 0.6 + s, 0.7 + y * 0.25)
    } else {
        let u = phase_unit(t);
        (0.9 + u * 0.2 + s, 0.85 + (1.0 - u) * 0.1)
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let c = 0.4;
    let d = 6.0;
    let mut x: f64 = 0.1;
    let mut y: f64 = 0.1;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    for _ in 0..40 {
        let r2 = x * x + y * y;
        let phase = c - d / (1.0 + r2);
        let (st, ct) = phase.sin_cos();
        let nx = a + b * (x * ct - y * st);
        let ny = b * (x * st + y * ct);
        x = nx;
        y = ny;
    }
    for _ in 0..ITERS {
        let r2 = x * x + y * y;
        let phase = c - d / (1.0 + r2);
        let (st, ct) = phase.sin_cos();
        let nx = a + b * (x * ct - y * st);
        let ny = b * (x * st + y * ct);
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
        let ch = if i % 11 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Ikeda map room.
#[derive(Debug, Default)]
pub struct Ikeda {
    seed: u64,
}

impl Ikeda {
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

impl Room for Ikeda {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ikeda",
            title: "The Ikeda Map",
            wing: "Motion & Dynamics",
            blurb: "Dissipative complex map from laser cavities: a curly strange attractor. t and \
                    DRAG: TUNE A AND B.",
            accent: [60, 180, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = params(t, None, self.seed);
        draw(canvas, a, b);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ikeda",
            root: 293.66,
            tempo: 102,
            line: &[0, 4, 8, 11, 8, 4, 14, 8],
            encodes: "phase twist from intensity in a complex map",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = params(t, None, self.seed);
        Some(format!("a={a:.2}  b={b:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b);
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
        let (a, b) = params(t, hands.last().copied(), self.seed);
        Some(format!("TUNE a={a:.3}  b={b:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The Ikeda map models a laser pulse in a ring cavity: each pass multiplies \
         by a complex phase that depends on intensity. At high b it folds into a \
         curly strange attractor; low b collapses toward a fixed point."
    }
}

#[cfg(test)]
mod tests {
    use super::Ikeda;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Ikeda::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Ikeda::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.85,
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
        Ikeda::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Ikeda::new().motif().unwrap().line.len() >= 6);
    }
}
