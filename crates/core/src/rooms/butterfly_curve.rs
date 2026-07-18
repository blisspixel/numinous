//! Butterfly curve: polar r = e^{cos th} - 2 cos(4 th) + sin^5(th/12).
//!
//! DRAG: TUNE PHASE. See `docs/ROOMS.md`.

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

fn phase(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * 12.0 * std::f64::consts::PI + s
    } else {
        phase_unit(t) * 12.0 * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, ph: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let scale = (width.min(height) as f64)
        * 0.12
        * (1.0
            + if seed == 0 {
                0.0
            } else {
                (seed % 3) as f64 * 0.05
            });
    // Temple-Fay butterfly over th in 0..12 pi
    let steps = 900;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = ph + 12.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r = th.cos().exp() - 2.0 * (4.0 * th).cos() + (th / 12.0).sin().powi(5);
        let px = (cx + scale * r * th.sin()).round() as i32;
        let py = (cy - scale * r * th.cos() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev
            && (px - ox).abs() < width as i32 / 2
            && (py - oy).abs() < height as i32 / 2
        {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Butterfly curve room.
#[derive(Debug, Default)]
pub struct ButterflyCurve {
    seed: u64,
}

impl ButterflyCurve {
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

impl Room for ButterflyCurve {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "butterfly-curve",
            title: "Butterfly Curve",
            wing: "Shape & Space",
            blurb: "Temple-Fay polar butterfly. t and DRAG: TUNE PHASE.",
            accent: [160, 50, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phase(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "butterfly-curve",
            root: 10.3,
            tempo: 96,
            line: &[0, 4, 7, 12, 9, 5, 0, 7],
            encodes: "butterfly: exp cos, cos 4th, and sin^5 wings in polar form",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHASE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = phase(t, None, self.seed);
        Some(format!("ph={p:.1}  wing  DRAG:PH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = phase(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = phase(t, hands.last().copied(), self.seed);
        let wing = ((p.rem_euclid(1.0) * 12.0).floor() as i32) + 1;
        Some(format!("ph={p:.2}  wing={wing}/12"))
    }

    fn reveal(&self) -> &'static str {
        "The butterfly curve of Temple Fay is the polar graph r = e^{cos th} - \
         2 cos(4 th) + sin^5(th/12). Over twelve full turns it paints a pair of \
         wings: a modern icon of recreational polar geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::ButterflyCurve;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ButterflyCurve::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("wing"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phase_changes() {
        let r = ButterflyCurve::new();
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
        ButterflyCurve::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}
