//! Thomas cyclically symmetric attractor.
//!
//! x' = sin(y) - b x; y' = sin(z) - b y; z' = sin(x) - b z.
//! DRAG: TUNE B. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 7_000;
const DT: f64 = 0.05;

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

fn b_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.005
    };
    if let Some((x, _)) = hand {
        0.05 + x * 0.3 + s
    } else {
        0.18 + phase_unit(t) * 0.1 + s
    }
}

fn integrate(b: f64) -> Vec<(f64, f64, f64)> {
    let mut x: f64 = 0.1;
    let mut y: f64 = 0.0;
    let mut z: f64 = 0.0;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let dx = y.sin() - b * x;
        let dy = z.sin() - b * y;
        let dz = x.sin() - b * z;
        x += DT * dx;
        y += DT * dy;
        z += DT * dz;
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
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
            let ch = if z.abs() > 1.0 {
                '#'
            } else if i % 5 == 0 {
                '+'
            } else {
                '*'
            };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Thomas attractor room.
#[derive(Debug, Default)]
pub struct Thomas {
    seed: u64,
}

impl Thomas {
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

impl Room for Thomas {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "thomas",
            title: "Thomas Attractor",
            wing: "Motion & Dynamics",
            blurb: "Cyclically symmetric continuous chaos. t and DRAG: TUNE B.",
            accent: [80, 200, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &integrate(b_param(t, None, self.seed)));
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "thomas",
            root: 146.83,
            tempo: 88,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "cyclic sin damping into a symmetric cloud",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = b_param(t, None, self.seed);
        Some(format!("b={b:.3}  cyclic  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = b_param(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(b));
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
        let b = b_param(t, hands.last().copied(), self.seed);
        Some(format!("TUNE b={b:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Thomas' cyclically symmetric attractor treats x, y, and z the same way: \
         each is driven by the sine of the next and damped by b. The symmetry \
         produces a lace of chaos that is neither Lorenz nor Rossler."
    }
}

#[cfg(test)]
mod tests {
    use super::Thomas;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Thomas::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Thomas::new();
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
        Thomas::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Thomas::new().motif().unwrap().line.len() >= 6);
    }
}
