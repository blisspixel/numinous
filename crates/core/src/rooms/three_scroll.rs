//! Three-scroll unified chaotic system (Lü et al. family, toy form).
//!
//! A continuous 3D flow that can show multi-scroll structure.
//! DRAG: TUNE C. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 7_000;
const DT: f64 = 0.002;

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

fn c_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.5
    };
    if let Some((x, _)) = hand {
        10.0 + x * 30.0 + s
    } else {
        20.0 + phase_unit(t) * 15.0 + s
    }
}

fn integrate(c: f64) -> Vec<(f64, f64, f64)> {
    // Multi-scroll Chua-like toy: x' = a(y-x), y' = x - y + z, z' = -b y - c z + d x z (simplified)
    let a = 40.0;
    let b = 3.0;
    let d = 1.0;
    let mut x = 0.1;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let dx = a * (y - x);
        let dy = (c - a) * x + c * y + x * z;
        let dz = -b * z + x * y * d;
        x += DT * dx;
        y += DT * dy;
        z += DT * dz;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            break;
        }
        // Soft clamp runaway.
        if x.abs() > 100.0 || y.abs() > 100.0 || z.abs() > 100.0 {
            break;
        }
        out.push((x, y, z));
    }
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y, _) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y, z)) in pts.iter().enumerate() {
        let u = 0.08 + 0.84 * (x - min_x) / dx;
        let v = 0.08 + 0.84 * (y - min_y) / dy;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            let ch = if z.abs() > 10.0 {
                '#'
            } else if i % 4 == 0 {
                '+'
            } else {
                '*'
            };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Three-scroll chaos room.
#[derive(Debug, Default)]
pub struct ThreeScroll {
    seed: u64,
}

impl ThreeScroll {
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

impl Room for ThreeScroll {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "three-scroll",
            title: "Three-Scroll Chaos",
            wing: "Motion & Dynamics",
            blurb: "Continuous multi-scroll chaotic flow, projected. t and DRAG: TUNE C.",
            accent: [100, 80, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &integrate(c_param(t, None, self.seed)));
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "three scroll",
            root: 233.08,
            tempo: 110,
            line: &[0, 7, 3, 10, 5, 12, 7, 14],
            encodes: "multiple scrolls of continuous chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE C")
    }

    fn status(&self, t: f64) -> Option<String> {
        let c = c_param(t, None, self.seed);
        Some(format!("c={c:.1}  scrolls  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let c = c_param(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(c));
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
        let c = c_param(t, hands.last().copied(), self.seed);
        Some(format!("TUNE c={c:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Multi-scroll chaotic systems extend Lorenz-like flows so the orbit \
         visits several lobe centers. This room is a CPU-honest toy of that \
         idea: one parameter steers the scroll structure."
    }
}

#[cfg(test)]
mod tests {
    use super::ThreeScroll;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ThreeScroll::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = ThreeScroll::new();
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
        ThreeScroll::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(ThreeScroll::new().motif().unwrap().line.len() >= 6);
    }
}
