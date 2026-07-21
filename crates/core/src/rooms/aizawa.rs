//! Aizawa attractor: a 3D continuous system with a ring-like strange attractor.
//!
//! Projected to xy. DRAG: TUNE EPSILON. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 7_000;
const DT: f64 = 0.01;

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

fn epsilon(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        0.1 + x * 0.4 + s
    } else {
        0.25 + phase_unit(t) * 0.15 + s
    }
}

fn integrate(eps: f64) -> Vec<(f64, f64, f64)> {
    // Classic Aizawa parameters with tunable epsilon.
    let a = 0.95;
    let b = 0.7;
    let c = 0.6;
    let d = 3.5;
    let e = 0.25;
    let f = 0.1;
    let mut x = 0.1;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let dx = (z - b) * x - d * y;
        let dy = d * x + (z - b) * y;
        let dz = c + a * z - z * z * z / 3.0 - (x * x + y * y) * (1.0 + e * z)
            + f * z * x * x * x
            + eps * x;
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
            let ch = if z > 0.5 {
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

/// Aizawa attractor room.
#[derive(Debug, Default)]
pub struct Aizawa {
    seed: u64,
}

impl Aizawa {
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

impl Room for Aizawa {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "aizawa",
            title: "Aizawa Ring",
            wing: "Motion & Dynamics",
            blurb: "Continuous 3D chaos with a ring-like attractor, projected. t and DRAG: TUNE \
                    EPSILON.",
            accent: [60, 140, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let e = epsilon(t, None, self.seed);
        draw(canvas, &integrate(e));
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "aizawa",
            root: 196.0,
            tempo: 92,
            line: &[0, 3, 7, 12, 7, 3, 10, 5],
            encodes: "a ring of continuous chaos in three dimensions",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE EPSILON")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = epsilon(t, None, self.seed);
        Some(format!("eps={e:.2}  ring  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = epsilon(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(e));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let e = epsilon(t, hands.last().copied(), self.seed);
        // Lightweight span sample (not the full render integration).
        let a = 0.95;
        let b = 0.7;
        let c = 0.6;
        let d = 3.5;
        let ee = 0.25;
        let f = 0.1;
        let mut x = 0.1_f64;
        let mut y = 0.0_f64;
        let mut z = 0.0_f64;
        for _ in 0..200 {
            let dx = (z - b) * x - d * y;
            let dy = d * x + (z - b) * y;
            let dz = c + a * z - z * z * z / 3.0 - (x * x + y * y) * (1.0 + ee * z)
                + f * z * x * x * x
                + e * x;
            x += DT * dx;
            y += DT * dy;
            z += DT * dz;
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_z = z;
        let mut max_z = z;
        for _ in 0..800 {
            let dx = (z - b) * x - d * y;
            let dy = d * x + (z - b) * y;
            let dz = c + a * z - z * z * z / 3.0 - (x * x + y * y) * (1.0 + ee * z)
                + f * z * x * x * x
                + e * x;
            x += DT * dx;
            y += DT * dy;
            z += DT * dz;
            if !x.is_finite() || !z.is_finite() {
                break;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_z = min_z.min(z);
            max_z = max_z.max(z);
        }
        let span = ((max_x - min_x) * (max_z - min_z)).max(0.0).sqrt();
        Some(format!("eps={e:.2}  span={span:.2}  Aizawa"))
    }

    fn reveal(&self) -> &'static str {
        "Aizawa's system is a continuous three-dimensional flow whose attractor \
         often looks like a thickened ring. Small changes in the epsilon term \
         reshape the orbit without losing the swirl."
    }
}

#[cfg(test)]
mod tests {
    use super::Aizawa;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Aizawa::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Aizawa::new();
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
        Aizawa::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Aizawa::new().motif().unwrap().line.len() >= 6);
    }
}
