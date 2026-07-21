//! Minkowski question-mark function: maps rationals to dyadics, quells jumps.
//!
//! DRAG: TUNE X. See `docs/ROOMS.md`.

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

fn x_mark(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        (x + s * 0.5).clamp(0.0, 1.0)
    } else {
        (phase_unit(t) + s * 0.5).clamp(0.0, 1.0)
    }
}

/// Continued-fraction based Minkowski ?(x) for x in [0,1].
fn minkowski_question(x: f64) -> f64 {
    if !(0.0..=1.0).contains(&x) {
        return 0.0;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    // CF expansion
    let mut a = Vec::new();
    let mut y = x;
    for _ in 0..24 {
        if y < 1e-12 {
            break;
        }
        let ai = y.floor() as i64;
        a.push(ai.max(0) as u32);
        let f = y - ai as f64;
        if f < 1e-12 {
            break;
        }
        y = 1.0 / f;
        if !y.is_finite() || y > 1e12 {
            break;
        }
    }
    if a.is_empty() {
        return 0.0;
    }
    // ?(x) = 2 sum_{k>=1} (-1)^{k-1} 2^{-(a1+...+ak)} for x in (0,1).
    let mut sum = 0.0_f64;
    let mut exp_sum = 0u32;
    let mut sign = 1.0_f64;
    let start = if a.first() == Some(&0) { 1 } else { 0 };
    for &ak in a.iter().skip(start) {
        exp_sum = exp_sum.saturating_add(ak.max(1));
        sum += sign * (-(exp_sum as f64)).exp2();
        sign = -sign;
    }
    (2.0 * sum).clamp(0.0, 1.0)
}

fn draw(canvas: &mut dyn Surface, x0: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let y = minkowski_question(x);
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Diagonal y=x for comparison.
    canvas.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        0,
        '.',
    );
    // Marker at x0.
    let mx = (x0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    let my = ((1.0 - minkowski_question(x0)) * height.saturating_sub(1) as f64).round() as i32;
    canvas.line(mx - 2, my, mx + 2, my, 'o');
    canvas.line(mx, my - 2, mx, my + 2, 'o');
    let _ = seed;
}

/// Minkowski question mark room.
#[derive(Debug, Default)]
pub struct MinkowskiQm {
    seed: u64,
}

impl MinkowskiQm {
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

impl Room for MinkowskiQm {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "minkowski-qm",
            title: "Minkowski Question Mark",
            wing: "Number & Pattern",
            blurb: "?(x) maps CF to dyadics, flattens jumps. t and DRAG: TUNE X.",
            accent: [60, 40, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, x_mark(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "minkowski-qm",
            root: 43.65,
            tempo: 92,
            line: &[0, 4, 5, 9, 12, 9, 5, 4],
            encodes: "Minkowski ?: continued fractions to dyadic rationals",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE X")
    }

    fn status(&self, t: f64) -> Option<String> {
        let x = x_mark(t, None, self.seed);
        let y = minkowski_question(x);
        Some(format!("x={x:.2}  ?={y:.2}  DRAG:X"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let x = x_mark(t, hands.last().copied(), self.seed);
        draw(canvas, x, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let x = x_mark(t, hands.last().copied(), self.seed);
        let y = minkowski_question(x);
        Some(format!("X={x:.3}  ?={y:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Minkowski's question-mark function ?(x) sends quadratic irrationals to \
         rational dyadics and turns the Devil's staircase of Farey mediants into \
         a singular continuous increasing map: continuous, strictly rising, and \
         flat almost nowhere."
    }
}

#[cfg(test)]
mod tests {
    use super::MinkowskiQm;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MinkowskiQm::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("?="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn x_changes() {
        let r = MinkowskiQm::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        MinkowskiQm::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}
