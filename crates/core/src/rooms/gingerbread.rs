//! Gingerbreadman map: piecewise linear chaos with a cookie silhouette.
//!
//! x' = 1 - y + |x|; y' = x. DRAG: SET THE START. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 6_000;

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

fn start(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.05
    };
    if let Some((x, y)) = hand {
        ((x - 0.5) * 4.0, (0.5 - y) * 4.0)
    } else {
        let u = phase_unit(t);
        (-0.1 + u * 0.2 + s, 0.1 * (u * std::f64::consts::TAU).sin())
    }
}

fn orbit(x0: f64, y0: f64) -> Vec<(f64, f64)> {
    let mut x = x0;
    let mut y = y0;
    let mut pts = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let nx = 1.0 - y + x.abs();
        let ny = x;
        x = nx;
        y = ny;
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        if x.abs() > 30.0 || y.abs() > 30.0 {
            break;
        }
        pts.push((x, y));
    }
    pts
}

fn draw(canvas: &mut dyn Surface, x0: f64, y0: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Several starts so the cookie silhouette fills even when one orbit is short.
    let mut pts = orbit(x0, y0);
    for (dx, dy) in [(-0.5, 0.0), (0.5, 0.0), (0.0, 0.5), (-0.2, -0.3)] {
        pts.extend(orbit(x0 + dx, y0 + dy));
    }
    if pts.is_empty() {
        // Fallback fixed frame so the room is never silent.
        pts = orbit(-0.1, 0.0);
    }
    if pts.is_empty() {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in &pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    // Pin a reasonable view box so sparse orbits still paint.
    min_x = min_x.min(-5.0);
    max_x = max_x.max(10.0);
    min_y = min_y.min(-5.0);
    max_y = max_y.max(10.0);
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        let ch = if i % 8 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Gingerbreadman map room.
#[derive(Debug, Default)]
pub struct Gingerbread {
    seed: u64,
}

impl Gingerbread {
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

impl Room for Gingerbread {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gingerbread",
            title: "Gingerbreadman Map",
            wing: "Motion & Dynamics",
            blurb: "Piecewise-linear map whose orbit sketches a cookie silhouette. t and DRAG: SET \
                    THE START.",
            accent: [180, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (x, y) = start(t, None, self.seed);
        draw(canvas, x, y);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gingerbread",
            root: 146.83,
            tempo: 94,
            line: &[0, 3, 7, 5, 10, 7, 12, 3],
            encodes: "absolute-value fold into a cookie orbit",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE START")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (x, y) = start(t, None, self.seed);
        Some(format!("start=({x:.2},{y:.2})  DRAG:START"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (x, y) = start(t, hands.last().copied(), self.seed);
        draw(canvas, x, y);
        if let Some(&(hx, hy)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (hx * width.saturating_sub(1) as f64).round() as i32;
                let py = (hy * height.saturating_sub(1) as f64).round() as i32;
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
        let (x, y) = start(t, hands.last().copied(), self.seed);
        Some(format!("START ({x:.2},{y:.2})"))
    }

    fn reveal(&self) -> &'static str {
        "The Gingerbreadman map is a piecewise-linear planar iteration. Absolute \
         value folds the plane; orbits of many starts paint a silhouette that \
         looks like a cookie with arms. Simple rule, rich picture."
    }
}

#[cfg(test)]
mod tests {
    use super::Gingerbread;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Gingerbread::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("START"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn start_changes() {
        let r = Gingerbread::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.8,
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
        Gingerbread::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Gingerbread::new().motif().unwrap().line.len() >= 6);
    }
}
