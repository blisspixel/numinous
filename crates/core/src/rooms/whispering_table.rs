//! The Whispering Table: elliptic billiards; chaos is impossible here.
//!
//! A ball on an elliptical table reflects with equal angles. Every trajectory
//! is integrable: caustics are smaller confocal ellipses (or hyperbolas). PULL
//! AND RELEASE: SHOOT. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const BOUNCES: usize = 48;
const A: f64 = 1.0;
const B: f64 = 0.62;

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

/// Reflect velocity off ellipse normal at point (x,y) on the rim.
fn reflect(x: f64, y: f64, vx: f64, vy: f64) -> (f64, f64) {
    // Gradient of x^2/A^2 + y^2/B^2 = 1 is (2x/A^2, 2y/B^2).
    let nx = x / (A * A);
    let ny = y / (B * B);
    let nlen = nx.hypot(ny).max(1e-9);
    let nx = nx / nlen;
    let ny = ny / nlen;
    let dot = vx * nx + vy * ny;
    (vx - 2.0 * dot * nx, vy - 2.0 * dot * ny)
}

fn on_ellipse(theta: f64) -> (f64, f64) {
    (A * theta.cos(), B * theta.sin())
}

/// Integrate billiard from rim angle and direction; return polyline in plate coords.
fn trajectory(theta0: f64, aim: f64, bounces: usize) -> Vec<(f64, f64)> {
    let (mut x, mut y) = on_ellipse(theta0);
    // Inward direction mixed with tangent aim.
    let tx = -A * theta0.sin();
    let ty = B * theta0.cos();
    let tlen = tx.hypot(ty).max(1e-9);
    let tx = tx / tlen;
    let ty = ty / tlen;
    let nx = x / (A * A);
    let ny = y / (B * B);
    let nlen = nx.hypot(ny).max(1e-9);
    let nx = -nx / nlen; // inward
    let ny = -ny / nlen;
    let mut vx = nx * aim.cos() + tx * aim.sin();
    let mut vy = ny * aim.cos() + ty * aim.sin();
    let vlen = vx.hypot(vy).max(1e-9);
    vx /= vlen;
    vy /= vlen;

    let mut path = vec![(x, y)];
    let dt = 0.02;
    let mut b = 0usize;
    let mut guard = 0usize;
    while b < bounces && guard < 20_000 {
        guard += 1;
        x += vx * dt;
        y += vy * dt;
        // Outside ellipse?
        if (x * x) / (A * A) + (y * y) / (B * B) >= 1.0 {
            // Pull back onto rim approximately.
            let s = ((x * x) / (A * A) + (y * y) / (B * B)).sqrt().max(1e-9);
            x /= s;
            y /= s;
            // Scale to ellipse: already roughly on unit after that? Better project.
            let th = (y / B).atan2(x / A);
            let p = on_ellipse(th);
            x = p.0;
            y = p.1;
            let r = reflect(x, y, vx, vy);
            vx = r.0;
            vy = r.1;
            let vlen = vx.hypot(vy).max(1e-9);
            vx /= vlen;
            vy /= vlen;
            path.push((x, y));
            b += 1;
        }
    }
    path
}

fn to_plate(x: f64, y: f64) -> (f64, f64) {
    (0.5 + x * 0.42, 0.5 + y * 0.42)
}

fn draw(canvas: &mut dyn Surface, path: &[(f64, f64)], seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        let (px, py) = to_plate(x, y);
        (
            (px.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (py.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    // Ellipse rim.
    let mut prev = to_px(A, 0.0);
    for i in 1..=120 {
        let th = TAU * i as f64 / 120.0;
        let p = to_px(A * th.cos(), B * th.sin());
        canvas.line(prev.0, prev.1, p.0, p.1, '.');
        prev = p;
    }
    if path.len() >= 2 {
        let mut prev = to_px(path[0].0, path[0].1);
        for &(x, y) in &path[1..] {
            let cur = to_px(x, y);
            canvas.line(prev.0, prev.1, cur.0, cur.1, '*');
            prev = cur;
        }
    }
    let _ = seed;
}

/// Whispering Table room.
#[derive(Debug, Default)]
pub struct WhisperingTable {
    seed: u64,
}

impl WhisperingTable {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for WhisperingTable {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "whispering-table",
            title: "The Whispering Table",
            wing: "Shape & Space",
            blurb: "Elliptic billiards: every shot is integrable, caustics are confocal curves, \
                    chaos never starts. t turns the ambient aim; PULL AND RELEASE: SHOOT.",
            accent: [180, 140, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let th = phase_unit(t) * TAU
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 7) as f64 * 0.3
            };
        let aim = 0.4 + phase_unit(t) * 0.5;
        let path = trajectory(th, aim, BOUNCES);
        draw(canvas, &path, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "confocal caustic",
            root: 196.0,
            tempo: 104,
            line: &[0, 5, 7, 12, 7, 5, 0, 5],
            encodes: "bounces weaving a quiet caustic ellipse",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("PULL AND RELEASE: SHOOT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let th = phase_unit(t) * TAU;
        let path = trajectory(th, 0.55, BOUNCES);
        Some(format!(
            "BOUNCE {}  a/b={:.2}  PULL:SHOOT",
            path.len().saturating_sub(1),
            A / B
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (x, y) = *hands.last().unwrap();
        let th = ((y - 0.5).atan2(x - 0.5)).rem_euclid(TAU);
        let aim = 0.2 + x * 0.9;
        let path = trajectory(th, aim, BOUNCES);
        draw(canvas, &path, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let th = ((y - 0.5).atan2(x - 0.5)).rem_euclid(TAU);
        let aim = 0.2 + x * 0.9;
        let path = trajectory(th, aim, BOUNCES);
        Some(format!(
            "SHOOT th={:.0}  aim={aim:.2}  n={}",
            th.to_degrees(),
            path.len().saturating_sub(1)
        ))
    }

    fn reveal(&self) -> &'static str {
        "On an elliptical billiard table every trajectory is regular: the \
         reflection law preserves a caustic confocal with the boundary. There is \
         no chaos here; the whispering gallery is geometry, not magic."
    }
}

#[cfg(test)]
mod tests {
    use super::{WhisperingTable, trajectory};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn trajectory_bounces() {
        let p = trajectory(0.3, 0.5, 10);
        assert!(p.len() > 5);
    }

    #[test]
    fn status_invites() {
        let s = WhisperingTable::new().status(0.0).unwrap();
        assert!(s.contains("PULL") || s.contains("SHOOT"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn shoot_changes() {
        let r = WhisperingTable::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 32);
        WhisperingTable::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(WhisperingTable::new().motif().unwrap().line.len() >= 6);
    }
}
