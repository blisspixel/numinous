//! Butterfly curve: polar r = e^{cos th} - 2 cos(4 th) + sin^5(th/12).
//!
//! Ambient phase draws the Temple-Fay wings with a pen. DRAG: TUNE SPIN.
//! See `docs/ROOMS.md`.

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

fn spin(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * 12.0 * std::f64::consts::PI + s
    } else {
        // Ambient spin holds; motion lives in the wing draw.
        s
    }
}

fn draw(canvas: &mut dyn Surface, ph: f64, show: f64, seed: u64) {
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
    let show = show.clamp(0.0, 1.0);
    // Temple-Fay butterfly over th in 0..12 pi
    let steps = 900;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    let max_jump = (width as i32 / 2).max(8);
    let max_jumph = (height as i32 / 2).max(8);
    // Soft ghost of the full wings.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = ph + 12.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r = th.cos().exp() - 2.0 * (4.0 * th).cos() + (th / 12.0).sin().powi(5);
        let px = (cx + scale * r * th.sin()).round() as i32;
        let py = (cy - scale * r * th.cos() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev
            && (px - ox).abs() < max_jump
            && (py - oy).abs() < max_jumph
        {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright wings so far.
    prev = None;
    let mut tip = (cx.round() as i32, cy.round() as i32);
    for i in 0..=drawn {
        let th = ph + 12.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r = th.cos().exp() - 2.0 * (4.0 * th).cos() + (th / 12.0).sin().powi(5);
        let px = (cx + scale * r * th.sin()).round() as i32;
        let py = (cy - scale * r * th.cos() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev
            && (px - ox).abs() < max_jump
            && (py - oy).abs() < max_jumph
        {
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
            blurb: "Temple-Fay wings draw themselves. Watch the pen; DRAG: TUNE SPIN.",
            accent: [160, 50, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, spin(t, None, self.seed), phase_unit(t), self.seed);
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
        Some("DRAG: TUNE SPIN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("wings draw={p}%  DRAG:SPIN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let ph = spin(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, ph, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = spin(t, hands.last().copied(), self.seed);
        let wing = ((p.rem_euclid(12.0 * std::f64::consts::PI) / (std::f64::consts::PI)).floor()
            as i32)
            .rem_euclid(12)
            + 1;
        Some(format!("spin={p:.2}  lobe~{wing}/12"))
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
        assert!(s.contains("DRAG") || s.contains("draw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn spin_changes() {
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
    fn ambient_pen_moves_the_plate() {
        let r = ButterflyCurve::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the wings");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        ButterflyCurve::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}
