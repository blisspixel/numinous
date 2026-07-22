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

fn growth(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        0.08 + x * 0.22 + s
    } else {
        // Ambient unfurls; growth rate holds a readable spiral.
        0.14 + s
    }
}

fn draw(canvas: &mut dyn Surface, b: f64, unfurl: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a0 = (width.min(height) as f64) * 0.025;
    let b = b.clamp(0.08, 0.35);
    let turns_full = 5.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.25
        };
    // Unfurl: ambient phase grows the spiral from the center outward.
    let turns = turns_full * (0.2 + 0.8 * unfurl.clamp(0.0, 1.0));
    let spin = unfurl * std::f64::consts::TAU * 0.5;
    let steps = 720;
    let mut prev: Option<(i32, i32)> = None;
    let mut tip = (cx.round() as i32, cy.round() as i32);
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let th = spin + u * turns * 2.0 * std::f64::consts::PI;
        let r = a0 * (b * (u * turns * 2.0 * std::f64::consts::PI)).exp();
        let max_r = (width.min(height) as f64) * 0.48;
        if r > max_r {
            prev = None;
            continue;
        }
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin() * 0.9).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        tip = (px, py);
        prev = Some((px, py));
    }
    // Tip bead rides the growing arm.
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(tip.0 + dx, tip.1 + dy, 'o');
            }
        }
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
        draw(canvas, growth(t, None, self.seed), phase_unit(t), self.seed);
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
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("b={b:.2}  unfurl={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = growth(t, hands.last().copied(), self.seed);
        let unfurl = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, b, unfurl, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = growth(t, hands.last().copied(), self.seed).clamp(0.06, 0.35);
        // r = a0 e^{b theta}; equiangular pitch satisfies tan(phi) = 1/b.
        let phi = (1.0 / b).atan().to_degrees();
        let grow_turn = (b * std::f64::consts::TAU).exp();
        Some(format!("b={b:.2}  phi={phi:.0}deg  x{grow_turn:.2}/turn"))
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
