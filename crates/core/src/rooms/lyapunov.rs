//! Logistic Lyapunov exponent vs r: chaos meter of the logistic map.
//!
//! Distinct from orbit/cobweb rooms: plots lambda(r). DRAG: SET R WINDOW.
//! See `docs/ROOMS.md`.

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

fn r_window(t: f64, hand: Option<(f64, f64)>) -> (f64, f64) {
    if let Some((x, y)) = hand {
        let mid = 2.8 + x * 1.2;
        let half = 0.1 + y * 0.5;
        ((mid - half).max(2.5), (mid + half).min(4.0))
    } else {
        let u = phase_unit(t);
        (2.8 + u * 0.4, 3.6 + u * 0.4)
    }
}

fn lyap(r: f64) -> f64 {
    let mut x = 0.5;
    let mut sum = 0.0;
    for _ in 0..80 {
        x = r * x * (1.0 - x);
    }
    for _ in 0..200 {
        x = r * x * (1.0 - x);
        let der = (r * (1.0 - 2.0 * x)).abs().max(1e-12);
        sum += der.ln();
    }
    sum / 200.0
}

fn draw(canvas: &mut dyn Surface, r0: f64, r1: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r0 = r0.min(r1 - 1e-6);
    let r1 = r1.max(r0 + 1e-6);
    // Zero line
    let y0 = (0.5 * height as f64).round() as i32;
    canvas.line(0, y0, width.saturating_sub(1) as i32, y0, '.');
    let mut prev: Option<(i32, i32)> = None;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.002
    };
    for col in 0..width {
        let r = r0 + (r1 - r0) * (col as f64 / width.saturating_sub(1).max(1) as f64) + j;
        let l = lyap(r).clamp(-2.0, 1.0);
        // Map lambda from [-2,1] to vertical
        let v = (l + 2.0) / 3.0;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        let px = col as i32;
        if let Some(o) = prev {
            let ch = if l > 0.0 { '#' } else { '*' };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Lyapunov logistic room.
#[derive(Debug, Default)]
pub struct Lyapunov {
    seed: u64,
}

impl Lyapunov {
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

impl Room for Lyapunov {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lyapunov",
            title: "Lyapunov Weather",
            wing: "Motion & Dynamics",
            blurb: "Logistic Lyapunov exponent lambda(r): chaos when positive. t and DRAG: SET R \
                    WINDOW.",
            accent: [200, 40, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = r_window(t, None);
        draw(canvas, a, b, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lyapunov",
            root: 277.18,
            tempo: 120,
            line: &[0, 0, 5, 7, 12, 7, 5, 12],
            encodes: "sign of average log derivative of the logistic map",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET R WINDOW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = r_window(t, None);
        Some(format!("r=[{a:.2},{b:.2}]  DRAG:WIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = r_window(t, hands.last().copied());
        draw(canvas, a, b, self.seed ^ hands.len() as u64);
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
        let (a, b) = r_window(t, hands.last().copied());
        let mid = 0.5 * (a + b);
        let l = lyap(mid);
        Some(format!("r~{mid:.2}  lam={l:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Lyapunov exponent measures average exponential divergence of \
         nearby orbits. For the logistic map, lambda(r) > 0 marks chaos and \
         lambda < 0 marks attracting cycles: a weather report of r."
    }
}

#[cfg(test)]
mod tests {
    use super::Lyapunov;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Lyapunov::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("WIN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn window_changes() {
        let r = Lyapunov::new();
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
        Lyapunov::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Lyapunov::new().motif().unwrap().line.len() >= 6);
    }
}
