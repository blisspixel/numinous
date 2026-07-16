//! The Menagerie: a zoo of 2D strange attractors (Clifford / de Jong).
//!
//! Four constants drive a million iterations into a luminous creature.
//! DRAG THE FOUR CONSTANTS (plate x/y sets a pair; phase the rest). See
//! `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 12_000;

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

/// Clifford attractor parameters (a,b,c,d).
fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64, f64, f64) {
    let u = phase_unit(t);
    let (a, b, c, d) = if let Some((x, y)) = hand {
        (
            -2.0 + x * 4.0,
            -2.0 + y * 4.0,
            -1.5 + u * 3.0,
            -1.5 + (1.0 - u) * 3.0,
        )
    } else {
        // Famous-ish Clifford look.
        (
            -1.4 + u * 0.3,
            1.6 - u * 0.2,
            1.0,
            0.7 + if seed == 0 {
                0.0
            } else {
                (seed % 5) as f64 * 0.05
            },
        )
    };
    (a, b, c, d)
}

fn clifford_step(x: f64, y: f64, a: f64, b: f64, c: f64, d: f64) -> (f64, f64) {
    let nx = (a * y).sin() + c * (a * x).cos();
    let ny = (b * x).sin() + d * (b * y).cos();
    (nx, ny)
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64, c: f64, d: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut x = 0.1;
    let mut y = 0.1;
    // Burn-in.
    for _ in 0..40 {
        let n = clifford_step(x, y, a, b, c, d);
        x = n.0;
        y = n.1;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let n = clifford_step(x, y, a, b, c, d);
        x = n.0;
        y = n.1;
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
        let ch = if i % 17 == 0 { '#' } else { '*' };
        canvas.plot(ix, iy, ch);
    }
}

/// Menagerie (Clifford attractor) room.
#[derive(Debug, Default)]
pub struct Menagerie {
    seed: u64,
}

impl Menagerie {
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

impl Room for Menagerie {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "menagerie",
            title: "The Menagerie",
            wing: "Fractals",
            blurb: "Clifford attractor: four numbers and a long orbit condense a luminous alien. \
                    t drifts constants; DRAG: TUNE THE FOUR.",
            accent: [180, 90, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b, c, d) = params(t, None, self.seed);
        draw(canvas, a, b, c, d);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "clifford zoo",
            root: 155.56,
            tempo: 88,
            line: &[0, 3, 7, 10, 12, 7, 3, 0],
            encodes: "four constants folding a plane orbit into a creature",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE THE FOUR")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b, c, d) = params(t, None, self.seed);
        Some(format!("a={a:.1} b={b:.1} c={c:.1} d={d:.1}  DRAG"))
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
        Some(format!("TUNE a={a:.2} b={b:.2} c={c:.1} d={d:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "Strange attractors are deterministic chaos with a stable silhouette. \
         Clifford's map needs only four reals; iterate long enough and a creature \
         appears that no closed form draws. The zoo is infinite; Lorenz is one cage."
    }
}

#[cfg(test)]
mod tests {
    use super::Menagerie;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Menagerie::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("a="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Menagerie::new();
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
        Menagerie::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 50);
    }

    #[test]
    fn motif_ok() {
        assert!(Menagerie::new().motif().unwrap().line.len() >= 6);
    }
}
