//! Euclidean algorithm: subtractive/modular dance of two integers.
//!
//! DRAG: SET THE PAIR. See `docs/ROOMS.md`.

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

fn pair(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (u32, u32) {
    let s = if seed == 0 { 0 } else { (seed % 17) as u32 };
    if let Some((x, y)) = hand {
        let a = 5 + (x * 90.0) as u32 + s;
        let b = 3 + (y * 70.0) as u32 + s / 2;
        (a.max(2), b.max(1))
    } else {
        let u = phase_unit(t);
        let a = 20 + (u * 80.0) as u32 + s;
        let b = 12 + ((1.0 - u) * 50.0) as u32 + s / 3;
        (a, b.max(1))
    }
}

fn gcd_steps(mut a: u32, mut b: u32) -> Vec<(u32, u32)> {
    let mut steps = vec![(a, b)];
    while b != 0 && steps.len() < 40 {
        let r = a % b;
        a = b;
        b = r;
        steps.push((a, b));
    }
    steps
}

fn draw(canvas: &mut dyn Surface, a0: u32, b0: u32, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let steps = gcd_steps(a0, b0);
    let max_v = steps
        .iter()
        .map(|(a, b)| (*a).max(*b))
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    // rectangle subtraction view: start with a x b rectangle, cut squares
    let mut a = a0 as f64;
    let mut b = b0 as f64;
    let scale = (width.min(height) as f64) * 0.85 / max_v;
    let ox = width as f64 * 0.08 + if seed == 0 { 0.0 } else { (seed % 5) as f64 };
    let oy = height as f64 * 0.1;
    let mut x = ox;
    let mut y = oy;
    let mut toggle = true;
    while a > 0.5 && b > 0.5 {
        if a >= b {
            let side = b * scale;
            let w = a * scale;
            // draw square of side b on the left of remaining
            canvas.line(
                x.round() as i32,
                y.round() as i32,
                (x + side).round() as i32,
                y.round() as i32,
                if toggle { '#' } else { '*' },
            );
            canvas.line(
                (x + side).round() as i32,
                y.round() as i32,
                (x + side).round() as i32,
                (y + side).round() as i32,
                if toggle { '#' } else { '*' },
            );
            canvas.line(
                (x + side).round() as i32,
                (y + side).round() as i32,
                x.round() as i32,
                (y + side).round() as i32,
                if toggle { '#' } else { '*' },
            );
            canvas.line(
                x.round() as i32,
                (y + side).round() as i32,
                x.round() as i32,
                y.round() as i32,
                if toggle { '#' } else { '*' },
            );
            x += side;
            a -= b;
            let _ = w;
        } else {
            let side = a * scale;
            canvas.line(
                x.round() as i32,
                y.round() as i32,
                (x + side).round() as i32,
                y.round() as i32,
                if toggle { '#' } else { '*' },
            );
            canvas.line(
                (x + side).round() as i32,
                y.round() as i32,
                (x + side).round() as i32,
                (y + side).round() as i32,
                if toggle { '#' } else { '*' },
            );
            canvas.line(
                (x + side).round() as i32,
                (y + side).round() as i32,
                x.round() as i32,
                (y + side).round() as i32,
                if toggle { '#' } else { '*' },
            );
            canvas.line(
                x.round() as i32,
                (y + side).round() as i32,
                x.round() as i32,
                y.round() as i32,
                if toggle { '#' } else { '*' },
            );
            y += side;
            b -= a;
        }
        toggle = !toggle;
    }
    // step count marks along bottom
    for (i, _) in steps.iter().enumerate() {
        let px = ((i as f64 / steps.len().max(1) as f64) * width.saturating_sub(1) as f64).round()
            as i32;
        canvas.plot(px, height.saturating_sub(1) as i32, '+');
    }
}

/// Euclidean algorithm room.
#[derive(Debug, Default)]
pub struct EuclidAlgorithm {
    seed: u64,
}

impl EuclidAlgorithm {
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

impl Room for EuclidAlgorithm {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "euclid",
            title: "Euclid Algorithm",
            wing: "Number & Pattern",
            blurb: "Square-cutting dance that finds gcd. t and DRAG: SET THE PAIR.",
            accent: [80, 80, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = pair(t, None, self.seed);
        draw(canvas, a, b, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "euclid",
            root: 73.42,
            tempo: 70,
            line: &[0, 5, 7, 0, 5, 7, 12, 0],
            encodes: "remainders falling until the common measure",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE PAIR")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = pair(t, None, self.seed);
        Some(format!("({a},{b})  euclid  DRAG:PAIR"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = pair(t, hands.last().copied(), self.seed);
        draw(canvas, a, b, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (a, b) = pair(t, hands.last().copied(), self.seed);
        let steps = gcd_steps(a, b);
        let g = steps.last().map(|(x, _)| *x).unwrap_or(1);
        Some(format!("gcd({a},{b})={g}  n={}", steps.len()))
    }

    fn reveal(&self) -> &'static str {
        "Euclid's algorithm replaces a pair (a,b) by (b, a mod b) until the \
         remainder vanishes. The last nonzero remainder is gcd(a,b). Drawn as \
         square-cutting, it is the oldest algorithm still in daily use."
    }
}

#[cfg(test)]
mod tests {
    use super::EuclidAlgorithm;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = EuclidAlgorithm::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("PAIR"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn pair_changes() {
        let r = EuclidAlgorithm::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        EuclidAlgorithm::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
