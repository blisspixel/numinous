//! Sinai billiard: square table with a circular scatterer (toy).
//!
//! Hard-disk billiard chaos in a box. DRAG: SET THE LAUNCH. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const BOUNCES: usize = 100;

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

fn launch(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.05
    };
    if let Some((x, y)) = hand {
        (0.15 + x * 0.7, 0.15 + y * 0.7, s + x * std::f64::consts::PI)
    } else {
        let u = phase_unit(t);
        (0.2, 0.3 + u * 0.2, 0.7 + u + s)
    }
}

fn draw(canvas: &mut dyn Surface, x0: f64, y0: f64, ang: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Unit square walls.
    let x1 = width.saturating_sub(1) as i32;
    let y1 = height.saturating_sub(1) as i32;
    canvas.line(0, 0, x1, 0, '.');
    canvas.line(0, y1, x1, y1, '.');
    canvas.line(0, 0, 0, y1, '.');
    canvas.line(x1, 0, x1, y1, '.');
    // Central disk.
    let r = 0.22
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.01
        };
    let cx = 0.5f64;
    let cy = 0.5f64;
    for i in 0..64 {
        let a = std::f64::consts::TAU * i as f64 / 64.0;
        let px = ((cx + r * a.cos()) * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - (cy + r * a.sin())) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, '#');
    }
    // Trajectory with wall and circle reflections.
    let mut x = x0.clamp(0.05, 0.95);
    let mut y = y0.clamp(0.05, 0.95);
    // Keep start outside disk.
    let d0 = ((x - cx).hypot(y - cy)).max(1e-6);
    if d0 < r + 0.02 {
        x = cx + (x - cx) / d0 * (r + 0.05);
        y = cy + (y - cy) / d0 * (r + 0.05);
    }
    let mut dx = ang.cos();
    let mut dy = ang.sin();
    let mut last = (
        (x * width.saturating_sub(1) as f64).round() as i32,
        ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32,
    );
    let mut steps = 0usize;
    while steps < BOUNCES * 40 {
        // Small step integration with event detection.
        let step = 0.01;
        let mut nx = x + step * dx;
        let mut ny = y + step * dy;
        // Walls.
        if nx < 0.0 {
            nx = -nx;
            dx = -dx;
            steps += 1;
        } else if nx > 1.0 {
            nx = 2.0 - nx;
            dx = -dx;
            steps += 1;
        }
        if ny < 0.0 {
            ny = -ny;
            dy = -dy;
            steps += 1;
        } else if ny > 1.0 {
            ny = 2.0 - ny;
            dy = -dy;
            steps += 1;
        }
        // Disk.
        let d = (nx - cx).hypot(ny - cy);
        if d < r {
            let nxn = (nx - cx) / d.max(1e-9);
            let nyn = (ny - cy) / d.max(1e-9);
            let dot = dx * nxn + dy * nyn;
            dx -= 2.0 * dot * nxn;
            dy -= 2.0 * dot * nyn;
            nx = cx + nxn * (r + 0.001);
            ny = cy + nyn * (r + 0.001);
            steps += 1;
        }
        x = nx;
        y = ny;
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(
            last.0,
            last.1,
            px,
            py,
            if steps.is_multiple_of(7) { '*' } else { '+' },
        );
        last = (px, py);
        if steps >= BOUNCES {
            break;
        }
    }
}

/// Sinai billiard room.
#[derive(Debug, Default)]
pub struct SinaiBilliard {
    seed: u64,
}

impl SinaiBilliard {
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

impl Room for SinaiBilliard {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sinai-billiard",
            title: "Sinai Billiard",
            wing: "Shape & Space",
            blurb: "Square table with a circular scatterer: hard chaos. t and DRAG: SET THE \
                    LAUNCH.",
            accent: [100, 60, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (x, y, a) = launch(t, None, self.seed);
        draw(canvas, x, y, a, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sinai",
            root: 73.42,
            tempo: 136,
            line: &[0, 8, 3, 11, 6, 14, 9, 17],
            encodes: "dispersing collisions of a hard disk scatterer",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE LAUNCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (x, y, _a) = launch(t, None, self.seed);
        Some(format!("start=({x:.2},{y:.2})  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (x, y, a) = launch(t, hands.last().copied(), self.seed);
        draw(canvas, x, y, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y, ang) = launch(t, hands.last().copied(), self.seed);
        // Sinai: dispersing billiard; positive Lyapunov from curved scatterer.
        let deg =
            (ang.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * 360.0).floor() as i32;
        let r2 = x * x + y * y;
        Some(format!("({x:.2},{y:.2}) a={deg}deg  r2={r2:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Sinai's billiard places a hard disk in a square. Dispersing collisions \
         make the dynamics ergodic and chaotic: the scatterer is a geometric \
         source of mixing without soft forces."
    }
}

#[cfg(test)]
mod tests {
    use super::SinaiBilliard;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SinaiBilliard::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("start"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn launch_changes() {
        let r = SinaiBilliard::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
        SinaiBilliard::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(SinaiBilliard::new().motif().unwrap().line.len() >= 6);
    }
}
