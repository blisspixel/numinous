//! Halvorsen attractor: another continuous 3D chaos system.
//!
//! x' = -a x - 4 y - 4 z - y^2 (cyclic). DRAG: TUNE A. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 6_500;
const DT: f64 = 0.005;

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

fn a_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        1.0 + x * 0.8 + s
    } else {
        1.4 + phase_unit(t) * 0.4 + s
    }
}

fn integrate(a: f64) -> Vec<(f64, f64, f64)> {
    let mut x = -1.48;
    let mut y = -1.51;
    let mut z = 2.04;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let dx = -a * x - 4.0 * y - 4.0 * z - y * y;
        let dy = -a * y - 4.0 * z - 4.0 * x - z * z;
        let dz = -a * z - 4.0 * x - 4.0 * y - x * x;
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
            let ch = if z > 0.0 {
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

/// Halvorsen attractor room.
#[derive(Debug, Default)]
pub struct Halvorsen {
    seed: u64,
}

impl Halvorsen {
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

impl Room for Halvorsen {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "halvorsen",
            title: "Halvorsen Attractor",
            wing: "Motion & Dynamics",
            blurb: "Continuous cyclic chaos with quadratic folds. t and DRAG: TUNE A.",
            accent: [200, 80, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &integrate(a_param(t, None, self.seed)));
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "halvorsen",
            root: 220.0,
            tempo: 98,
            line: &[0, 4, 7, 11, 7, 4, 12, 4],
            encodes: "cyclic quadratic damping into a strange cloud",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = a_param(t, None, self.seed);
        Some(format!("a={a:.2}  cyclic  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = a_param(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(a));
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
        let a = a_param(t, hands.last().copied(), self.seed);
        // Lightweight span after burn (not full render integration).
        let mut x = -1.48_f64;
        let mut y = -1.51_f64;
        let mut z = 2.04_f64;
        for _ in 0..200 {
            let dx = -a * x - 4.0 * y - 4.0 * z - y * y;
            let dy = -a * y - 4.0 * z - 4.0 * x - z * z;
            let dz = -a * z - 4.0 * x - 4.0 * y - x * x;
            x += DT * dx;
            y += DT * dy;
            z += DT * dz;
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return Some(format!("a={a:.2}  span=div"));
            }
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..800 {
            let dx = -a * x - 4.0 * y - 4.0 * z - y * y;
            let dy = -a * y - 4.0 * z - 4.0 * x - z * z;
            let dz = -a * z - 4.0 * x - 4.0 * y - x * x;
            x += DT * dx;
            y += DT * dy;
            z += DT * dz;
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                break;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        Some(format!("a={a:.2}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Halvorsen's attractor is a continuous three-dimensional system with \
         cyclic symmetry and quadratic terms. The single parameter a steers how \
         tightly the orbit folds."
    }
}

#[cfg(test)]
mod tests {
    use super::Halvorsen;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Halvorsen::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Halvorsen::new();
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
        Halvorsen::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Halvorsen::new().motif().unwrap().line.len() >= 6);
    }
}
