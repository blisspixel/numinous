//! Real quadratic map family x -> x^2 + c on the line (Mandelbrot's 1D cousin).
//!
//! Orbit cobweb for real c. DRAG: TUNE C AND X0. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBIT: usize = 160;

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
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, y)) = hand {
        (-2.0 + x * 2.25 + s, -1.5 + y * 3.0)
    } else {
        let u = phase_unit(t);
        (-1.2 + u * 0.8 + s, 0.1)
    }
}

fn draw(canvas: &mut dyn Surface, c: f64, x0: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // View window [-2,2] x [-2,2]
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        let u = ((x + 2.0) / 4.0).clamp(0.0, 1.0);
        let v = ((y + 2.0) / 4.0).clamp(0.0, 1.0);
        (
            (u * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    // Graph y = x^2 + c
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=width {
        let x = -2.0 + 4.0 * i as f64 / width.saturating_sub(1).max(1) as f64;
        let y = x * x + c;
        let p = to_px(x, y);
        if let Some(o) = prev {
            canvas.line(o.0, o.1, p.0, p.1, '#');
        }
        prev = Some(p);
    }
    // Diagonal
    let a = to_px(-2.0, -2.0);
    let b = to_px(2.0, 2.0);
    canvas.line(a.0, a.1, b.0, b.1, '.');
    // Cobweb
    let mut x = x0.clamp(-1.9, 1.9);
    let mut p = to_px(x, x);
    for i in 0..ORBIT {
        let y = x * x + c;
        if !y.is_finite() || y.abs() > 4.0 {
            break;
        }
        let q = to_px(x, y);
        canvas.line(p.0, p.1, q.0, p.1, if i % 2 == 0 { '*' } else { '+' });
        canvas.line(q.0, p.1, q.0, q.1, if i % 2 == 0 { '*' } else { '+' });
        let d = to_px(y, y);
        canvas.line(q.0, q.1, d.0, d.1, '.');
        p = d;
        x = y;
    }
}

/// Real quadratic map room.
#[derive(Debug, Default)]
pub struct QuadraticMap {
    seed: u64,
}

impl QuadraticMap {
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

impl Room for QuadraticMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "quadratic-map",
            title: "Quadratic Map",
            wing: "Motion & Dynamics",
            blurb: "Real map x -> x^2 + c: Mandelbrot's one-dimensional cousin. t and DRAG: TUNE \
                    C AND X0.",
            accent: [60, 100, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (c, x0) = params(t, None, self.seed);
        draw(canvas, c, x0);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "quadratic",
            root: 466.16,
            tempo: 94,
            line: &[0, 1, 5, 6, 10, 11, 15, 5],
            encodes: "real parabola iteration conjugate to the logistic map",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE C AND X0")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (c, x0) = params(t, None, self.seed);
        Some(format!("c={c:.2}  x0={x0:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (c, x0) = params(t, hands.last().copied(), self.seed);
        draw(canvas, c, x0);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (c, x0) = params(t, hands.last().copied(), self.seed);
        // Real quadratic x -> x^2 + c; escape |x|>2.
        let mut x = x0;
        let mut esc = 0u32;
        for i in 0..64u32 {
            if x.abs() > 2.0 {
                esc = i;
                break;
            }
            x = x * x + c;
            if !x.is_finite() {
                esc = i;
                break;
            }
            esc = i + 1;
        }
        let band = if c > 0.25 {
            "escape"
        } else if c > -0.75 {
            "period"
        } else {
            "chaos?"
        };
        Some(format!("c={c:.2}  esc={esc}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "The real quadratic family x -> x^2 + c is conjugate to the logistic \
         map. Mandelbrot's set is the complex cousin of the same parameter c: \
         the real line of c is a spine of period-doubling into chaos."
    }
}

#[cfg(test)]
mod tests {
    use super::QuadraticMap;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = QuadraticMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = QuadraticMap::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        QuadraticMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(QuadraticMap::new().motif().unwrap().line.len() >= 6);
    }
}
