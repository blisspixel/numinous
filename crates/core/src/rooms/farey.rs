//! Farey sequence diagram: all reduced fractions up to denominator Q.
//!
//! Distinct from Ford circles: points on a strip, mediants as neighbors.
//! DRAG: SET Q. See `docs/ROOMS.md`.

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

fn max_q(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (3 + (x * 40.0) as u32).clamp(3, 48)
    } else {
        (5 + (phase_unit(t) * 30.0) as u32).clamp(3, 40)
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn farey(q: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for b in 1..=q {
        for a in 0..=b {
            if gcd(a, b) == 1 {
                out.push((a, b));
            }
        }
    }
    out.sort_by(|p, q| (p.0 * q.1).cmp(&(q.0 * p.1)).then(p.1.cmp(&q.1)));
    out
}

fn draw(canvas: &mut dyn Surface, q: u32, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let fr = farey(q);
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.01
    };
    // Base line
    let yb = (0.85 * height as f64).round() as i32;
    canvas.line(0, yb, width.saturating_sub(1) as i32, yb, '.');
    let mut prev: Option<(i32, i32)> = None;
    for &(a, b) in &fr {
        let x = a as f64 / b as f64;
        let u = ((x + shift).fract()) * 0.9 + 0.05;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = yb - (b as f64 / q as f64 * height as f64 * 0.7).round() as i32;
        canvas.plot(
            px,
            py,
            if b <= 3 {
                '#'
            } else if b <= 8 {
                '*'
            } else {
                '+'
            },
        );
        canvas.line(px, yb, px, py, '.');
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, ':');
        }
        prev = Some((px, py));
    }
}

/// Farey sequence room.
#[derive(Debug, Default)]
pub struct Farey {
    seed: u64,
}

impl Farey {
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

impl Room for Farey {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "farey",
            title: "Farey Sequence",
            wing: "Number & Pattern",
            blurb: "All reduced fractions up to denominator Q as a comb. t and DRAG: SET Q.",
            accent: [100, 120, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, max_q(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "farey",
            root: 196.0,
            tempo: 100,
            line: &[0, 3, 5, 7, 10, 12, 7, 3],
            encodes: "reduced fractions ordered by size on the unit interval",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET Q")
    }

    fn status(&self, t: f64) -> Option<String> {
        let q = max_q(t, None);
        let n = farey(q).len();
        Some(format!("Q={q}  fracs={n}  DRAG:Q"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let q = max_q(t, hands.last().copied());
        draw(canvas, q, self.seed ^ hands.len() as u64);
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
        let q = max_q(t, hands.last().copied());
        let n = farey(q).len();
        Some(format!("Q={q}  fracs={n}"))
    }

    fn reveal(&self) -> &'static str {
        "The Farey sequence of order Q lists every reduced fraction a/b with \
         0 <= a <= b <= Q, sorted by value. Neighbors a/b, c/d always satisfy \
         |ad-bc|=1: best rational approximations sit next door."
    }
}

#[cfg(test)]
mod tests {
    use super::{Farey, farey};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Farey::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("Q="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn q_changes() {
        let r = Farey::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn farey3() {
        // 0/1, 1/3, 1/2, 2/3, 1/1
        assert_eq!(farey(3).len(), 5);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Farey::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Farey::new().motif().unwrap().line.len() >= 6);
    }
}
