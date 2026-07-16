//! Borromean rings: three mutually linked circles, no two linked.
//!
//! DRAG: TUNE ANGLE. See `docs/ROOMS.md`.

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

fn angle(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::PI + s
    } else {
        phase_unit(t) * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, ang: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.28;
    let offs = r * 0.55;
    let rings: [((f64, f64), char); 3] = [
        ((0.0, -offs * 0.4), '#'),
        ((-offs * 0.7, offs * 0.5), '*'),
        ((offs * 0.7, offs * 0.5), '+'),
    ];
    let tilt = ang
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.05
        };
    for (i, &((ox, oy), ch)) in rings.iter().enumerate() {
        let mut prev: Option<(i32, i32)> = None;
        let plane = i as f64 * 0.4 + tilt * 0.3;
        for j in 0..=48 {
            let th = 2.0 * std::f64::consts::PI * (j as f64 / 48.0);
            let x = ox + r * th.cos();
            let y = oy + r * th.sin() * plane.cos() * 0.55;
            let px = (cx + x).round() as i32;
            let py = (cy - y).round() as i32;
            if let Some((qx, qy)) = prev {
                canvas.line(qx, qy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
}

/// Borromean rings room.
#[derive(Debug, Default)]
pub struct Borromean {
    seed: u64,
}

impl Borromean {
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

impl Room for Borromean {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "borromean",
            title: "Borromean Rings",
            wing: "Shape & Space",
            blurb: "Three linked as one; no pair linked. t and DRAG: TUNE ANGLE.",
            accent: [160, 120, 30],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, angle(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "borromean",
            root: 77.78,
            tempo: 92,
            line: &[0, 5, 8, 12, 8, 5, 0, 12],
            encodes: "Borromean rings: Brunnian link of three unknots",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = angle(t, None, self.seed);
        Some(format!("a={a:.2}  3link  DRAG:ANG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = angle(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = angle(t, hands.last().copied(), self.seed);
        Some(format!("A={a:.3}  borro"))
    }

    fn reveal(&self) -> &'static str {
        "Borromean rings are a Brunnian link: three circles locked so that \
         removing any one frees the other two. No pair is linked by itself; the \
         linking is purely ternary."
    }
}

#[cfg(test)]
mod tests {
    use super::Borromean;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Borromean::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("3link"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = Borromean::new();
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
        Borromean::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
