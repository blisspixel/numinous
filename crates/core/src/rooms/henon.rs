//! Henon Map: the classic 2D strange attractor.
//!
//! (x,y) -> (1 - a x^2 + y, b x). DRAG: TUNE A AND B. See `docs/ROOMS.md`.

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    // Classic a=1.4, b=0.3
    if let Some((x, y)) = hand {
        (1.0 + x * 0.6, 0.1 + y * 0.35)
    } else {
        let u = phase_unit(t);
        let s = if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.02
        };
        (1.2 + u * 0.3 + s, 0.25 + (1.0 - u) * 0.1)
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut x = 0.1;
    let mut y = 0.1;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    for _ in 0..40 {
        let nx = 1.0 - a * x * x + y;
        let ny = b * x;
        x = nx;
        y = ny;
    }
    for _ in 0..ITERS {
        let nx = 1.0 - a * x * x + y;
        let ny = b * x;
        x = nx;
        y = ny;
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        pts.push((x, y));
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, if i % 11 == 0 { '#' } else { '*' });
    }
}

/// Henon Map room.
#[derive(Debug, Default)]
pub struct Henon {
    seed: u64,
}

impl Henon {
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

impl Room for Henon {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "henon",
            title: "The Henon Map",
            wing: "Fractals",
            blurb: "Henon attractor: one quadratic map, a folded horseshoe of chaos. t and DRAG: \
                    TUNE A AND B.",
            accent: [180, 100, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = params(t, None, self.seed);
        draw(canvas, a, b);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "henon",
            root: 415.3,
            tempo: 88,
            line: &[0, 5, 9, 12, 16, 12, 9, 5],
            encodes: "a quadratic horseshoe folding the plane",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = params(t, None, self.seed);
        Some(format!("a={a:.2}  b={b:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b);
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
        let (a, b) = params(t, hands.last().copied(), self.seed);
        // Jacobian det is -b for every step: area contraction |b| each map.
        let area_contract = b.abs();
        let mut x = 0.1_f64;
        let mut y = 0.1_f64;
        for _ in 0..40 {
            let nx = 1.0 - a * x * x + y;
            let ny = b * x;
            x = nx;
            y = ny;
        }
        // Bounds after burn-in only, so span is the settled attractor window.
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..400 {
            let nx = 1.0 - a * x * x + y;
            let ny = b * x;
            if !nx.is_finite() || !ny.is_finite() {
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
        Some(format!("a={a:.2} |det|={area_contract:.2} span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Henon map is a planar quadratic iteration with a strange attractor \
         for classic parameters a=1.4, b=0.3. Stretch, fold, and reconverge: \
         chaos with a thin fractal silhouette."
    }
}

#[cfg(test)]
mod tests {
    use super::Henon;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Henon::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("a="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Henon::new();
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
        Henon::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 50);
    }

    #[test]
    fn motif_ok() {
        assert!(Henon::new().motif().unwrap().line.len() >= 6);
    }
}
