//! Gauss map (mouse map): iterated modular inverse-like chaos on the line.
//!
//! x' = frac(1/x) for x!=0, with a soft escape. DRAG: SET THE SEED.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBIT: usize = 180;

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

fn seed_x(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 17) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        (0.05 + x * 0.9 + s).fract().max(0.02)
    } else {
        (0.2 + phase_unit(t) * 0.5 + s).fract().max(0.02)
    }
}

fn gauss_step(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        0.5
    } else {
        (1.0 / x).fract().abs()
    }
}

fn draw(canvas: &mut dyn Surface, x0: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Graph y = frac(1/x) on (0,1]
    let mut prev: Option<(i32, i32)> = None;
    for i in 1..=width {
        let x = i as f64 / width as f64;
        let y = gauss_step(x).clamp(0.0, 1.0);
        let px = i as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, '#');
        }
        prev = Some((px, py));
    }
    canvas.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        0,
        '.',
    );
    // Cobweb orbit
    let mut x = x0;
    let mut px = (x * width.saturating_sub(1) as f64).round() as i32;
    let mut py = height.saturating_sub(1) as i32;
    for i in 0..ORBIT {
        let y = gauss_step(x);
        let qx = (x * width.saturating_sub(1) as f64).round() as i32;
        let qy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, py, qx, py, if i % 2 == 0 { '*' } else { '+' });
        canvas.line(qx, py, qx, qy, if i % 2 == 0 { '*' } else { '+' });
        let dx = (y * width.saturating_sub(1) as f64).round() as i32;
        let dy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(qx, qy, dx, dy, '.');
        px = dx;
        py = dy;
        x = y;
    }
}

/// Gauss map room.
#[derive(Debug, Default)]
pub struct GaussMap {
    seed: u64,
}

impl GaussMap {
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

impl Room for GaussMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gauss-map",
            title: "Gauss Map",
            wing: "Number & Pattern",
            blurb: "Continued-fraction engine: x -> frac(1/x). t and DRAG: SET THE SEED.",
            accent: [120, 80, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, seed_x(t, None, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gauss map",
            root: 277.18,
            tempo: 118,
            line: &[0, 12, 5, 0, 7, 12, 0, 5],
            encodes: "fractional parts of reciprocal iterates",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE SEED")
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = seed_x(t, None, self.seed);
        Some(format!("x0={x:.3}  gauss  DRAG:SEED"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let x = seed_x(t, hands.last().copied(), self.seed);
        draw(canvas, x);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let x = seed_x(t, hands.last().copied(), self.seed);
        let mut y = x;
        for _ in 0..8 {
            y = gauss_step(y);
        }
        Some(format!("SEED x0={x:.3}  x8={y:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The Gauss map x -> frac(1/x) generates the continued-fraction digits of \
         x. It is expanding on (0,1] and is a classical engine of Diophantine \
         approximation and ergodic theory."
    }
}

#[cfg(test)]
mod tests {
    use super::GaussMap;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GaussMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SEED"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn seed_changes() {
        let r = GaussMap::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
        GaussMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(GaussMap::new().motif().unwrap().line.len() >= 6);
    }
}
