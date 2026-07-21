//! Levy C curve: right-angle dragon cousin via L-system F+F--F+F style.
//!
//! Each segment becomes two at 45 degrees. DRAG: SET THE ORDER.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

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

fn order(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (3 + (x * 9.0) as usize).clamp(3, 12)
    } else {
        (4 + (phase_unit(t) * 7.0) as usize).clamp(3, 11)
    }
}

/// Turns as +1 right, -1 left for Levy C (45-degree steps encoded as 8 dirs).
fn levy_path(n: usize, seed: u64) -> Vec<(f64, f64)> {
    // Build string of moves: recursive rewrite F -> +F--F+
    // Represent as list of turn deltas before each unit step.
    let mut turns: Vec<i8> = Vec::new();
    // Start with one forward (no leading turn).
    for _ in 0..n {
        let mut next = Vec::with_capacity(turns.len() * 2 + 3);
        // + F -- F +
        next.push(1); // +
        next.extend_from_slice(&turns);
        next.push(-1);
        next.push(-1); // --
        next.extend_from_slice(&turns);
        next.push(1); // +
        turns = next;
        if turns.len() > 16_000 {
            break;
        }
    }
    let mut pts = Vec::with_capacity(turns.len() + 2);
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    // 8 compass steps for 45 degrees.
    let mut dir = if seed == 0 { 0i32 } else { (seed % 8) as i32 };
    pts.push((x, y));
    // Initial step.
    step(&mut x, &mut y, dir);
    pts.push((x, y));
    for &t in &turns {
        dir = (dir + i32::from(t)).rem_euclid(8);
        step(&mut x, &mut y, dir);
        pts.push((x, y));
    }
    pts
}

fn step(x: &mut f64, y: &mut f64, dir: i32) {
    // 45-degree unit steps (normalized so diagonals match visual length).
    let s = std::f64::consts::FRAC_1_SQRT_2;
    match dir.rem_euclid(8) {
        0 => *x += 1.0,
        1 => {
            *x += s;
            *y += s;
        }
        2 => *y += 1.0,
        3 => {
            *x -= s;
            *y += s;
        }
        4 => *x -= 1.0,
        5 => {
            *x -= s;
            *y -= s;
        }
        6 => *y -= 1.0,
        _ => {
            *x += s;
            *y -= s;
        }
    }
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for (i, &p) in pts.iter().enumerate() {
        let u = 0.08 + 0.84 * (p.0 - min_x) / dx;
        let v = 0.08 + 0.84 * (p.1 - min_y) / dy;
        let q = (
            (u * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32,
        );
        if let Some(o) = prev {
            let ch = if i % 6 == 0 { '#' } else { '*' };
            canvas.line(o.0, o.1, q.0, q.1, ch);
        }
        prev = Some(q);
    }
}

/// Levy C curve room.
#[derive(Debug, Default)]
pub struct LevyC {
    seed: u64,
}

impl LevyC {
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

impl Room for LevyC {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "levy-c",
            title: "The Levy C Curve",
            wing: "Fractals",
            blurb: "Self-similar C from the rewrite F -> +F--F+. t and DRAG: SET THE ORDER.",
            accent: [40, 160, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let o = order(t, None);
        draw(canvas, &levy_path(o, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "levy c",
            root: 329.63,
            tempo: 114,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "right-angle rewrite becoming a dense C",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ORDER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let o = order(t, None);
        let n = levy_path(o, self.seed).len();
        Some(format!("order={o}  pts={n}  DRAG:ORDER"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let o = order(t, hands.last().copied());
        draw(canvas, &levy_path(o, self.seed ^ hands.len() as u64));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let o = order(t, hands.last().copied());
        let n = levy_path(o, self.seed).len();
        // Levy C: Hausdorff dim = 2; segment count 2^o.
        let segs = 1u64 << o.min(20);
        Some(format!("o={o}  pts={n}  segs={segs}  d=2"))
    }

    fn reveal(&self) -> &'static str {
        "The Levy C curve is the limit of a simple L-system: replace each \
         segment by two at right angles. Unlike the dragon, it is not plane-filling \
         of a region in the same way, but its Hausdorff dimension is greater than one."
    }
}

#[cfg(test)]
mod tests {
    use super::{LevyC, levy_path};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LevyC::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ORDER"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn order_changes() {
        let r = LevyC::new();
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
    fn path_grows() {
        assert!(levy_path(2, 0).len() < levy_path(4, 0).len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        LevyC::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(LevyC::new().motif().unwrap().line.len() >= 6);
    }
}
