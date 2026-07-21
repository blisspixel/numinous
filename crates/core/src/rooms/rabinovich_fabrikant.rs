//! Rabinovich-Fabrikant equations: continuous chaos with cubic terms.
//!
//! Projected to xy. DRAG: TUNE GAMMA. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 6_000;
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

fn gamma(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        0.1 + x * 1.0 + s
    } else {
        0.87 + phase_unit(t) * 0.2 + s
    }
}

fn integrate(g: f64) -> Vec<(f64, f64, f64)> {
    let alpha = 1.1;
    let mut x = -1.0;
    let mut y = 0.0;
    let mut z = 0.5;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let dx = y * (z - 1.0 + x * x) + g * x;
        let dy = x * (3.0 * z + 1.0 - x * x) + g * y;
        let dz = -2.0 * z * (alpha + x * y);
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

/// Rabinovich-Fabrikant room.
#[derive(Debug, Default)]
pub struct RabinovichFabrikant {
    seed: u64,
}

impl RabinovichFabrikant {
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

impl Room for RabinovichFabrikant {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "rabinovich-fabrikant",
            title: "Rabinovich-Fabrikant",
            wing: "Motion & Dynamics",
            blurb: "Cubic continuous chaos from plasma physics. t and DRAG: TUNE GAMMA.",
            accent: [180, 60, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &integrate(gamma(t, None, self.seed)));
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "rab fab",
            root: 164.81,
            tempo: 102,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "cubic plasma chaos folding a continuous flow",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE GAMMA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = gamma(t, None, self.seed);
        Some(format!("gamma={g:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = gamma(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(g));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let g = gamma(t, hands.last().copied(), self.seed);
        let alpha = 1.1;
        let mut x = -1.0_f64;
        let mut y = 0.0_f64;
        let mut z = 0.5_f64;
        for _ in 0..200 {
            let dx = y * (z - 1.0 + x * x) + g * x;
            let dy = x * (3.0 * z + 1.0 - x * x) + g * y;
            let dz = -2.0 * z * (alpha + x * y);
            x += DT * dx;
            y += DT * dy;
            z += DT * dz;
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return Some(format!("g={g:.2}  span=0  div"));
            }
            if x.abs() > 100.0 || y.abs() > 100.0 || z.abs() > 100.0 {
                return Some(format!("g={g:.2}  span=0  div"));
            }
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..800 {
            let dx = y * (z - 1.0 + x * x) + g * x;
            let dy = x * (3.0 * z + 1.0 - x * x) + g * y;
            let dz = -2.0 * z * (alpha + x * y);
            x += DT * dx;
            y += DT * dy;
            z += DT * dz;
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                break;
            }
            if x.abs() > 100.0 || y.abs() > 100.0 || z.abs() > 100.0 {
                break;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        Some(format!("g={g:.2}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Rabinovich and Fabrikant proposed a cubic system modeling modulational \
         instability. For classic parameters the flow produces a strange \
         attractor with rich lobes; gamma steers the dissipation."
    }
}

#[cfg(test)]
mod tests {
    use super::RabinovichFabrikant;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = RabinovichFabrikant::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = RabinovichFabrikant::new();
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
        RabinovichFabrikant::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(RabinovichFabrikant::new().motif().unwrap().line.len() >= 6);
    }
}
