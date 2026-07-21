//! Conchoid of Nicomedes: classical curve for trisecting angles.
//!
//! DRAG: TUNE K. See `docs/ROOMS.md`.

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

fn k_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.6 + s
    } else {
        0.6 + phase_unit(t) * 1.2 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = 0.8;
    let cx = width as f64 * 0.2;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let scale = (width.min(height) as f64) * 0.28;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    // polar form relative to focus: r = a / cos theta +/- k
    for sign in [1.0_f64, -1.0] {
        let mut prev: Option<(i32, i32)> = None;
        let steps = 240;
        for i in 0..=steps {
            let th = -1.3 + 2.6 * (i as f64 / steps as f64) + j * 0.1;
            let c = th.cos();
            if c.abs() < 0.08 {
                prev = None;
                continue;
            }
            let r = a / c + sign * k;
            if !r.is_finite() || r.abs() > 8.0 {
                prev = None;
                continue;
            }
            let px = (cx + scale * r * th.cos()).round() as i32;
            let py = (cy - scale * r * th.sin()).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, if sign > 0.0 { '#' } else { '*' });
            }
            prev = Some((px, py));
        }
    }
    // directrix
    let dx = (cx + scale * a).round() as i32;
    canvas.line(dx, 0, dx, height.saturating_sub(1) as i32, '.');
}

/// Conchoid room.
#[derive(Debug, Default)]
pub struct Conchoid {
    seed: u64,
}

impl Conchoid {
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

impl Room for Conchoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "conchoid",
            title: "Conchoid",
            wing: "Shape & Space",
            blurb: "Nicomedes' shell curve for angle trisection. t and DRAG: TUNE K.",
            accent: [40, 120, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, k_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "conchoid",
            root: 130.8,
            tempo: 88,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "fixed offset from a line through a focus",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = k_param(t, None, self.seed);
        Some(format!("k={k:.2}  conch  DRAG:K"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = k_param(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let k = k_param(t, hands.last().copied(), self.seed);
        // Nicomedes conchoid: r = a/cos(th) +/- k with a=0.8 fixed in draw.
        let a = 0.8_f64;
        let gap = 2.0 * k;
        Some(format!("k={k:.2}  a={a:.1}  gap={gap:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The conchoid of Nicomedes is the locus of points a fixed distance k \
         along lines through a focus meeting a directrix. Greeks used it to \
         trisect angles and double cubes."
    }
}

#[cfg(test)]
mod tests {
    use super::Conchoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Conchoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("K"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn k_changes() {
        let r = Conchoid::new();
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
        Conchoid::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
