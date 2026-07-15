//! Steiner Chains: a ring of circles that always closes.
//!
//! Once a chain of circles fits between two boundaries and closes, it closes
//! from every angle (after inversion the outer circles become parallel lines).
//! DRAG: SPIN THE CHAIN. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

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

fn draw_circle(
    canvas: &mut dyn Surface,
    cx: f64,
    cy: f64,
    r: f64,
    ch: char,
    width: usize,
    height: usize,
) {
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (y.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let steps = ((r * 90.0) as usize).clamp(16, 72);
    let mut prev: Option<(i32, i32)> = None;
    for s in 0..=steps {
        let a = TAU * s as f64 / steps as f64;
        let p = to_px(cx + r * a.cos(), cy + r * a.sin());
        if let Some(o) = prev {
            canvas.line(o.0, o.1, p.0, p.1, ch);
        }
        prev = Some(p);
    }
}

fn draw(canvas: &mut dyn Surface, n: usize, rot: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n.clamp(4, 16);
    let ox = 0.5
        + if seed == 0 {
            0.0
        } else {
            ((seed % 5) as f64 - 2.0) * 0.01
        };
    let oy = 0.5;
    let r_out = 0.42;
    let r_in = 0.14;
    draw_circle(canvas, ox, oy, r_out, '#', width, height);
    draw_circle(canvas, ox, oy, r_in, '#', width, height);
    // Steiner circles between concentric pair: radius and orbit from packing.
    let gap = r_out - r_in;
    let r = gap / 2.0 * 0.92;
    let orbit = r_in + r;
    for i in 0..n {
        let a = rot + TAU * i as f64 / n as f64;
        let cx = ox + orbit * a.cos();
        let cy = oy + orbit * a.sin();
        draw_circle(canvas, cx, cy, r, '*', width, height);
    }
}

/// Steiner Chains room.
#[derive(Debug, Default)]
pub struct Steiner {
    seed: u64,
}

impl Steiner {
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

impl Room for Steiner {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "steiner",
            title: "The Ring That Always Closes",
            wing: "Shape & Space",
            blurb: "A Steiner chain of circles fits between two boundaries and closes from every \
                    angle. t sets count; DRAG: SPIN THE CHAIN.",
            accent: [160, 200, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = 6 + (phase_unit(t) * 6.0) as usize;
        let rot = phase_unit(t) * TAU * 0.25;
        draw(canvas, n, rot, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "steiner ring",
            root: 277.18,
            tempo: 97,
            line: &[0, 4, 5, 9, 12, 9, 5, 4],
            encodes: "a closed necklace of circles that spins without breaking",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SPIN THE CHAIN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = 6 + (phase_unit(t) * 6.0) as usize;
        Some(format!("n={n}  closed  DRAG:SPIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = 6 + (phase_unit(t) * 6.0) as usize;
        let rot = hands
            .last()
            .map(|&(x, y)| (y - 0.5).atan2(x - 0.5))
            .unwrap_or(0.0);
        draw(canvas, n, rot, self.seed);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = 6 + (phase_unit(t) * 6.0) as usize;
        let rot = hands
            .last()
            .map(|&(x, y)| (y - 0.5).atan2(x - 0.5))
            .unwrap_or(0.0);
        Some(format!(
            "SPIN rot={:.0}deg  n={n}  CLOSED",
            rot.to_degrees()
        ))
    }

    fn reveal(&self) -> &'static str {
        "A Steiner chain is a closed ring of circles packed between two others. \
         Invert so the boundaries become parallel lines, and the mystery becomes \
         obvious: a line of equal coins. Spin the original and it still closes."
    }
}

#[cfg(test)]
mod tests {
    use super::Steiner;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Steiner::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SPIN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn spin_changes() {
        let r = Steiner::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Steiner::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 30);
    }

    #[test]
    fn motif_ok() {
        assert!(Steiner::new().motif().unwrap().line.len() >= 6);
    }
}
