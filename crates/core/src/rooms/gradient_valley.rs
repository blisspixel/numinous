//! The Gradient Valley: descent finds a basin; ridges lie.
//!
//! A 2D loss landscape; drop a seeker and watch gradient steps. Local minima
//! and ridges teach that the landscape lies. DROP: A SEEKER. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 40;

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

/// Map plate [0,1]^2 to world [-2,2]^2.
fn to_world(px: f64, py: f64) -> (f64, f64) {
    ((px - 0.5) * 4.0, (0.5 - py) * 4.0)
}

fn to_plate(x: f64, y: f64) -> (f64, f64) {
    (x / 4.0 + 0.5, 0.5 - y / 4.0)
}

/// Multimodal loss: two basins and a ridge.
fn loss(x: f64, y: f64, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    let b1 = (x - 0.8 - s).powi(2) + (y - 0.6).powi(2);
    let b2 = 0.6 * ((x + 1.0 + s).powi(2) + (y + 0.8).powi(2));
    let ridge = 0.15 * (-((x + y) * 0.8).powi(2)).exp() * (x * y).abs();
    b1.min(b2) + ridge + 0.05 * (x * 3.0 + phase_unit(0.0)).sin().abs()
}

fn grad(x: f64, y: f64, seed: u64) -> (f64, f64) {
    let h = 1e-3;
    let l = loss(x, y, seed);
    let dx = (loss(x + h, y, seed) - l) / h;
    let dy = (loss(x, y + h, seed) - l) / h;
    (dx, dy)
}

fn descend(start: (f64, f64), seed: u64, lr: f64) -> Vec<(f64, f64)> {
    let mut path = Vec::with_capacity(STEPS + 1);
    let (mut x, mut y) = start;
    path.push((x, y));
    for _ in 0..STEPS {
        let (gx, gy) = grad(x, y, seed);
        x -= lr * gx;
        y -= lr * gy;
        x = x.clamp(-2.0, 2.0);
        y = y.clamp(-2.0, 2.0);
        path.push((x, y));
    }
    path
}

fn draw(canvas: &mut dyn Surface, seed: u64, path: &[(f64, f64)], t: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Contour-ish field.
    for py in 0..height {
        for px in 0..width {
            let u = px as f64 / width.saturating_sub(1).max(1) as f64;
            let v = py as f64 / height.saturating_sub(1).max(1) as f64;
            let (x, y) = to_world(u, v);
            let l = loss(x, y, seed);
            let ch = if l < 0.3 {
                '#'
            } else if l < 0.8 {
                '*'
            } else if l < 1.5 {
                '+'
            } else if l < 2.5 {
                '.'
            } else {
                ' '
            };
            if ch != ' ' {
                canvas.plot(px as i32, py as i32, ch);
            }
        }
    }
    let _ = t;
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        let (u, v) = to_plate(p.0, p.1);
        (
            (u.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (v.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let mut prev: Option<(i32, i32)> = None;
    for &p in path {
        let q = to_px(p);
        if let Some(o) = prev {
            canvas.line(o.0, o.1, q.0, q.1, 'o');
        }
        prev = Some(q);
    }
    if let Some(last) = path.last() {
        let q = to_px(*last);
        canvas.plot(q.0, q.1, '@');
    }
}

/// Gradient Valley room.
#[derive(Debug, Default)]
pub struct GradientValley {
    seed: u64,
}

impl GradientValley {
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

impl Room for GradientValley {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gradient-valley",
            title: "The Gradient Valley",
            wing: "Number & Pattern",
            blurb: "Descent finds a basin; a ridge blocks another. The landscape lies to the \
                    seeker. t drifts start; CLICK: DROP A SEEKER.",
            accent: [80, 160, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let a = phase_unit(t) * std::f64::consts::TAU;
        let start = (1.5 * a.cos(), 1.2 * a.sin());
        let path = descend(start, self.seed, 0.15);
        draw(canvas, self.seed, &path, t);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "descent",
            root: 146.83,
            tempo: 100,
            line: &[0, 5, 3, 7, 12, 7, 3, 0],
            encodes: "a seeker sliding into a basin the ridge hid",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DROP A SEEKER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = phase_unit(t) * std::f64::consts::TAU;
        let start = (1.5 * a.cos(), 1.2 * a.sin());
        let path = descend(start, self.seed, 0.15);
        let end = *path.last().unwrap_or(&start);
        let l = loss(end.0, end.1, self.seed);
        Some(format!("loss={l:.2}  CLICK:SEEKER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let start = hands
            .last()
            .map(|&(x, y)| to_world(x, y))
            .unwrap_or((1.0, 1.0));
        let path = descend(start, self.seed, 0.12);
        draw(canvas, self.seed, &path, t);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let start = hands
            .last()
            .map(|&(x, y)| to_world(x, y))
            .unwrap_or((0.0, 0.0));
        let path = descend(start, self.seed, 0.12);
        let end = *path.last().unwrap_or(&start);
        let l0 = loss(start.0, start.1, self.seed);
        let l1 = loss(end.0, end.1, self.seed);
        Some(format!("DROP L0={l0:.2} -> L1={l1:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Gradient descent follows the local slope. Basins trap seekers; ridges \
         hide better valleys. The landscape of learning is honest only in \
         hindsight: the same feeling every optimizer, silicon or not, meets."
    }
}

#[cfg(test)]
mod tests {
    use super::{GradientValley, descend, loss};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GradientValley::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("SEEKER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn drop_changes() {
        let r = GradientValley::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn descent_lowers_loss() {
        let start = (1.5, 1.2);
        let path = descend(start, 0, 0.15);
        let end = *path.last().unwrap();
        assert!(loss(end.0, end.1, 0) <= loss(start.0, start.1, 0) + 1e-6);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(36, 28);
        GradientValley::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 30);
    }

    #[test]
    fn motif_ok() {
        assert!(GradientValley::new().motif().unwrap().line.len() >= 6);
    }
}
