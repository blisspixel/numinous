//! Unknot: a circle that is not knotted, deformed toward a round form.
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

fn kink(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        x * 1.8 + s
    } else {
        phase_unit(t) * 1.6 + s
    }
    .clamp(0.0, 2.0)
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.32;
    let phase = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.3
    };
    // a planar closed curve that looks knotted when k is high but is unknot
    // (stereotyped "ugly" unknot projection that can be untangled)
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=160 {
        let th = std::f64::consts::TAU * (i as f64) / 160.0;
        let wiggle = k * 0.35 * (3.0 * th + phase).sin() + k * 0.2 * (5.0 * th).cos();
        let x = r * (th.cos() + 0.25 * k * (2.0 * th).sin());
        let y = r * (th.sin() + wiggle * 0.5) * 0.7;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // hint of Reidemeister simplification: dashed round goal
    prev = None;
    for i in 0..=60 {
        let th = std::f64::consts::TAU * (i as f64) / 60.0;
        let x = r * 0.85 * th.cos();
        let y = r * 0.85 * th.sin() * 0.7;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if i % 2 == 0 {
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, '.');
            }
        }
        prev = Some((px, py));
    }
}

/// Unknot room.
#[derive(Debug, Default)]
pub struct Unknot {
    seed: u64,
}

impl Unknot {
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

impl Room for Unknot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "unknot",
            title: "Unknot",
            wing: "Shape & Space",
            blurb: "A tangled circle that is still the unknot. t and DRAG: TUNE K.",
            accent: [50, 80, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, kink(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "unknot",
            root: 130.81,
            tempo: 72,
            line: &[0, 2, 4, 5, 7, 5, 4, 2],
            encodes: "unknot: any embedding of S1 that bounds a disk, messy or round",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = kink(t, None, self.seed);
        Some(format!("k={k:.2}  unknot  DRAG:K"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = kink(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
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
        let k = kink(t, hands.last().copied(), self.seed);
        // Unknot: trivial knot type; kink is cosmetic, crossings stay 0.
        Some(format!("kink={k:.2}  cr=0  unknot"))
    }

    fn reveal(&self) -> &'static str {
        "The unknot is any embedding of a circle that can be deformed to a round \
         circle without cutting: it bounds a disk. Ugly projections can look \
         knotted; Reidemeister moves and knot polynomials are the tools that \
         prove they are not."
    }
}

#[cfg(test)]
mod tests {
    use super::Unknot;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Unknot::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("unknot"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn k_changes() {
        let r = Unknot::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Unknot::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
