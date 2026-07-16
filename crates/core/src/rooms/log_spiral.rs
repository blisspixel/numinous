//! Logarithmic spiral: equiangular growth, r = a e^{b theta}.
//!
//! DRAG: TUNE GROWTH. See `docs/ROOMS.md`.

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

fn growth(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        0.08 + x * 0.22 + s
    } else {
        0.1 + phase_unit(t) * 0.16 + s
    }
}

fn draw(canvas: &mut dyn Surface, b: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a0 = (width.min(height) as f64) * 0.02;
    let b = b.clamp(0.06, 0.35);
    let turns = 4.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.25
        };
    let steps = 500;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let th = u * turns * 2.0 * std::f64::consts::PI;
        let r = a0 * (b * th).exp();
        let max_r = (width.min(height) as f64) * 0.48;
        if r > max_r {
            prev = None;
            continue;
        }
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Logarithmic spiral room.
#[derive(Debug, Default)]
pub struct LogSpiral {
    seed: u64,
}

impl LogSpiral {
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

impl Room for LogSpiral {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "log-spiral",
            title: "Logarithmic Spiral",
            wing: "Shape & Space",
            blurb: "Equiangular growth r = a e^{b theta}. t and DRAG: TUNE GROWTH.",
            accent: [40, 140, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, growth(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "log-spiral",
            root: 349.23,
            tempo: 92,
            line: &[0, 2, 5, 7, 12, 7, 5, 2],
            encodes: "self-similar spiral: angle between radius and tangent fixed",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE GROWTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = growth(t, None, self.seed);
        Some(format!("b={b:.2}  equiang  DRAG:GRW"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = growth(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
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
        let b = growth(t, hands.last().copied(), self.seed);
        Some(format!("B={b:.3}  log spiral"))
    }

    fn reveal(&self) -> &'static str {
        "In a logarithmic spiral the angle between the radius and the tangent is \
         constant. Nautilus shells, galaxy arms, and hawk stoops approximate this \
         self-similar curve of Bernoulli's spira mirabilis."
    }
}

#[cfg(test)]
mod tests {
    use super::LogSpiral;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LogSpiral::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("equiang"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn growth_changes() {
        let r = LogSpiral::new();
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
        LogSpiral::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
