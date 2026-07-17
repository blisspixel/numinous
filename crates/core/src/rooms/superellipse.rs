//! Superellipse (Lamé curve): |x/a|^n + |y/b|^n = 1.
//!
//! DRAG: TUNE N. See `docs/ROOMS.md`.

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

fn exponent(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        0.5 + x * 5.5 + s
    } else {
        0.8 + phase_unit(t) * 4.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, n: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n.clamp(0.4, 8.0);
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.4;
    let b = a
        * (0.85
            + if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.04
            });
    let steps = 360;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let c = th.cos().abs().powf(2.0 / n) * th.cos().signum();
        let s = th.sin().abs().powf(2.0 / n) * th.sin().signum();
        // polar form of superellipse: x = a |cos|^{2/n} sgn cos, y = b |sin|^{2/n} sgn sin
        let x = a * c;
        let y = b * s;
        let px = (cx + x).round() as i32;
        let py = (cy - y * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Superellipse room.
#[derive(Debug, Default)]
pub struct Superellipse {
    seed: u64,
}

impl Superellipse {
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

impl Room for Superellipse {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "superellipse",
            title: "Superellipse",
            wing: "Shape & Space",
            blurb: "Lame curve |x|^n+|y|^n=1 from diamond to square. t and DRAG: TUNE N.",
            accent: [70, 110, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, exponent(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "superellipse",
            root: 12.99,
            tempo: 80,
            line: &[0, 4, 7, 11, 7, 4, 0, 12],
            encodes: "superellipse: n morphs diamond through circle into squircle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = exponent(t, None, self.seed);
        Some(format!("n={n:.2}  lame  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = exponent(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let n = exponent(t, hands.last().copied(), self.seed).clamp(0.4, 8.0);
        // Lamé superellipse |x/a|^n + |y/b|^n = 1; special cases n=2 circle, n->inf square.
        let shape = if (n - 2.0).abs() < 0.12 {
            "ellipse"
        } else if n < 1.0 {
            "astro"
        } else if n > 4.0 {
            "boxy"
        } else {
            "super"
        };
        Some(format!("n={n:.2}  {shape}"))
    }

    fn reveal(&self) -> &'static str {
        "A superellipse is |x/a|^n + |y/b|^n = 1. At n=2 it is an ordinary ellipse; \
         as n grows it fattens toward a rectangle (the squircle at n=4); as n drops \
         toward 1 it pinches into a diamond. Piet Hein used it for city planning."
    }
}

#[cfg(test)]
mod tests {
    use super::Superellipse;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Superellipse::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lame"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Superellipse::new();
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
        Superellipse::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
