//! Archimedean spiral: arithmetic growth, r = a + b theta.
//!
//! Ambient phase unfurls the arm. DRAG: TUNE PITCH. See `docs/ROOMS.md`.

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

fn pitch(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    if let Some((x, _)) = hand {
        0.04 + x * 0.14 + s
    } else {
        // Ambient pitch holds even grooves; motion lives in the unfurl.
        0.12 + s
    }
}

fn draw(canvas: &mut dyn Surface, b: f64, unfurl: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let b = b.clamp(0.03, 0.22);
    let scale = (width.min(height) as f64) * 0.08;
    let turns = 5.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.3
        };
    let unfurl = unfurl.clamp(0.0, 1.0);
    let steps = 520;
    let drawn = (((0.15 + 0.85 * unfurl) * steps as f64).round() as usize).min(steps);
    let max_r = (width.min(height) as f64) * 0.48;
    // Soft ghost of the full arm.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let th = u * turns * 2.0 * std::f64::consts::PI;
        let r = scale * b * th;
        if r > max_r {
            prev = None;
            continue;
        }
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright arm so far.
    prev = None;
    let mut tip = (cx.round() as i32, cy.round() as i32);
    for i in 0..=drawn {
        let u = i as f64 / steps as f64;
        let th = u * turns * 2.0 * std::f64::consts::PI;
        let r = scale * b * th;
        if r > max_r {
            prev = None;
            continue;
        }
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        tip = (px, py);
        prev = Some((px, py));
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(tip.0 + dx, tip.1 + dy, 'o');
            }
        }
    }
}

/// Archimedean spiral room.
#[derive(Debug, Default)]
pub struct Archimedean {
    seed: u64,
}

impl Archimedean {
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

impl Room for Archimedean {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "archimedean",
            title: "Archimedean Spiral",
            wing: "Shape & Space",
            blurb: "Arithmetic arm unfurls at constant gap. Watch the tip; DRAG: TUNE PITCH.",
            accent: [90, 120, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, pitch(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "archimedean",
            root: 369.99,
            tempo: 86,
            line: &[0, 4, 7, 11, 7, 4, 0, 7],
            encodes: "constant spacing between successive turns of the arm",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PITCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = pitch(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("b={b:.2}  unfurl={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = pitch(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, b, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = pitch(t, hands.last().copied(), self.seed).clamp(0.03, 0.22);
        // Constant radial gap per full turn: Delta r = 2 pi b.
        let gap = std::f64::consts::TAU * b;
        Some(format!("b={b:.3}  gap/turn={gap:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Archimedes studied the spiral r = a + b theta: each turn adds the same \
         radial gap. Unlike the log spiral it is not self-similar; vinyl grooves \
         and coiled ropes follow this arithmetic growth."
    }
}

#[cfg(test)]
mod tests {
    use super::Archimedean;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Archimedean::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("unfurl"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn pitch_changes() {
        let r = Archimedean::new();
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
    fn ambient_unfurl_moves_the_plate() {
        let r = Archimedean::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "arm must unfurl");
        assert!(a.ink_count() > 30);
        assert!(b.ink_count() > 30);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Archimedean::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
