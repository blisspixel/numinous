//! Tinkerbell map: a planar quadratic with a butterfly-shaped attractor.
//!
//! x' = x^2 - y^2 + a x + b y; y' = 2 x y + c x + d y.
//! DRAG: TUNE A AND C. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 8_000;

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64, f64, f64) {
    // Classic a=0.9, b=-0.6013, c=2.0, d=0.5. Keep a,c inside the butterfly basin;
    // wider sweeps explode to a few divergent pixels and look blank.
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.008
    };
    if let Some((x, y)) = hand {
        (
            (0.82 + x * 0.16 + s).clamp(0.78, 0.98),
            -0.6013,
            (1.7 + y * 0.55).clamp(1.55, 2.35),
            0.5,
        )
    } else {
        let u = phase_unit(t);
        (
            (0.88 + u * 0.06 + s).clamp(0.85, 0.96),
            -0.6013,
            (1.85 + u * 0.25).clamp(1.75, 2.2),
            0.5,
        )
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64, c: f64, d: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    // Burn-in, then sample. Restart from classic seed if the orbit escapes.
    for attempt in 0..3 {
        let mut x = -0.72 + attempt as f64 * 0.02;
        let mut y = -0.64 - attempt as f64 * 0.01;
        pts.clear();
        min_x = f64::MAX;
        max_x = f64::MIN;
        min_y = f64::MAX;
        max_y = f64::MIN;
        let mut ok = true;
        for _ in 0..80 {
            let nx = x * x - y * y + a * x + b * y;
            let ny = 2.0 * x * y + c * x + d * y;
            if !nx.is_finite() || !ny.is_finite() || nx.abs() > 20.0 || ny.abs() > 20.0 {
                ok = false;
                break;
            }
            x = nx;
            y = ny;
        }
        if !ok {
            continue;
        }
        for _ in 0..ITERS {
            let nx = x * x - y * y + a * x + b * y;
            let ny = 2.0 * x * y + c * x + d * y;
            if !nx.is_finite() || !ny.is_finite() || nx.abs() > 20.0 || ny.abs() > 20.0 {
                break;
            }
            x = nx;
            y = ny;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            pts.push((x, y));
        }
        if pts.len() > 200 {
            break;
        }
    }
    if pts.is_empty() {
        // Fallback silhouette so the room never paints blank.
        for i in 0..120 {
            let u = i as f64 / 119.0;
            let th = u * std::f64::consts::TAU;
            let px = (0.5 + 0.28 * th.cos()) * width.saturating_sub(1) as f64;
            let py = (0.5 + 0.18 * th.sin()) * height.saturating_sub(1) as f64;
            canvas.plot(px.round() as i32, py.round() as i32, '*');
        }
        return;
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    // Keep a margin so wings do not collapse into the corners.
    let mx = width as f64 * 0.06;
    let my = height as f64 * 0.06;
    let iw = (width as f64 - 2.0 * mx).max(1.0);
    let ih = (height as f64 - 2.0 * my).max(1.0);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (mx + u * iw).round() as i32;
        let iy = (my + (1.0 - v) * ih).round() as i32;
        let ch = if i % 9 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Tinkerbell map room.
#[derive(Debug, Default)]
pub struct Tinkerbell {
    seed: u64,
}

impl Tinkerbell {
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

impl Room for Tinkerbell {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tinkerbell",
            title: "Tinkerbell Map",
            wing: "Motion & Dynamics",
            blurb: "Quadratic planar map with a butterfly-shaped attractor. t and DRAG: TUNE A AND \
                    C.",
            accent: [220, 120, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b, c, d) = params(t, None, self.seed);
        draw(canvas, a, b, c, d);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tinkerbell",
            root: 196.00,
            tempo: 118,
            line: &[0, 7, 3, 10, 5, 12, 7, 14],
            encodes: "quadratic wings of a planar attractor",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND C")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, _b, c, _d) = params(t, None, self.seed);
        Some(format!("a={a:.2}  c={c:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b, c, d) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b, c, d);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (a, b, c, d) = params(t, hands.last().copied(), self.seed);
        let mut x = -0.72_f64;
        let mut y = -0.64_f64;
        for _ in 0..50 {
            let nx = x * x - y * y + a * x + b * y;
            let ny = 2.0 * x * y + c * x + d * y;
            x = nx;
            y = ny;
            if !x.is_finite() || !y.is_finite() || x.abs() > 50.0 || y.abs() > 50.0 {
                return Some(format!("a={a:.2} c={c:.2}  span=0  div"));
            }
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..500 {
            let nx = x * x - y * y + a * x + b * y;
            let ny = 2.0 * x * y + c * x + d * y;
            if !nx.is_finite() || !ny.is_finite() || nx.abs() > 50.0 || ny.abs() > 50.0 {
                break;
            }
            x = nx;
            y = ny;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        Some(format!("a={a:.2} c={c:.2}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Tinkerbell map is a planar quadratic iteration whose classic \
         parameters yield a butterfly-shaped strange attractor. Small parameter \
         moves shred or thicken the wings."
    }
}

#[cfg(test)]
mod tests {
    use super::Tinkerbell;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Tinkerbell::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Tinkerbell::new();
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
        Tinkerbell::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 80, "butterfly must fill the plate");
        for t in [0.0, 0.35, 0.55, 0.9] {
            let mut large = Canvas::new(120, 70);
            Tinkerbell::new().render(&mut large, t);
            assert!(
                large.ink_count() > 100,
                "t={t} must not collapse to blank: {}",
                large.ink_count()
            );
        }
    }

    #[test]
    fn motif_ok() {
        assert!(Tinkerbell::new().motif().unwrap().line.len() >= 6);
    }
}
