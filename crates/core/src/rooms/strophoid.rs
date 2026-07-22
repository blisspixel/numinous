//! Right strophoid: classical cubic with a loop and asymptote.
//!
//! Ambient phase walks a pen along the loop and arms. DRAG: TUNE SCALE.
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

fn scale(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.55 + x * 0.55 + s
    } else {
        // Ambient scale holds a readable loop; motion lives in the pen.
        0.88 + s * 0.5
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 * 0.45;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.42 * a.clamp(0.5, 1.15);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.02
    };
    let show = show.clamp(0.0, 1.0);
    // polar: r = a cos(2 theta) / cos theta
    let steps = 420;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Soft ghost of the full strophoid.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = -1.2 + 2.4 * (i as f64 / steps as f64) + j * 0.1;
        let c = th.cos();
        if c.abs() < 1e-3 {
            prev = None;
            continue;
        }
        let r = rad * (2.0 * th).cos() / c;
        if !r.is_finite() || r.abs() > rad * 3.0 {
            prev = None;
            continue;
        }
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin() * 0.9).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    let mut tip = (cx.round() as i32, cy.round() as i32);
    for i in 0..=drawn {
        let th = -1.2 + 2.4 * (i as f64 / steps as f64) + j * 0.1;
        let c = th.cos();
        if c.abs() < 1e-3 {
            prev = None;
            continue;
        }
        let r = rad * (2.0 * th).cos() / c;
        if !r.is_finite() || r.abs() > rad * 3.0 {
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
    // asymptote x = -a
    let ax = (cx - rad).round() as i32;
    canvas.line(ax, 0, ax, height.saturating_sub(1) as i32, '.');
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(tip.0 + dx, tip.1 + dy, 'o');
            }
        }
    }
}

/// Strophoid room.
#[derive(Debug, Default)]
pub struct Strophoid {
    seed: u64,
}

impl Strophoid {
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

impl Room for Strophoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "strophoid",
            title: "Strophoid",
            wing: "Shape & Space",
            blurb: "Twisted belt draws its loop and asymptote. Watch the pen; DRAG: TUNE SCALE.",
            accent: [160, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "strophoid",
            root: 116.5,
            tempo: 86,
            line: &[0, 3, 7, 10, 14, 10, 7, 3],
            encodes: "a twisted loop that runs to infinity",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("a={a:.2}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, a, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = scale(t, hands.last().copied(), self.seed);
        // Right strophoid loop area 2a^2 - pi a^2 / 2.
        let loop_a = 2.0 * a * a - 0.5 * std::f64::consts::PI * a * a;
        Some(format!("a={a:.2}  loopA={loop_a:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "A strophoid is a cubic with a node and an asymptote. The right \
         strophoid r = a cos(2 theta)/cos theta looks like a twisted belt: \
         one of the classical plane curves of the 17th century."
    }
}

#[cfg(test)]
mod tests {
    use super::Strophoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Strophoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("draw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Strophoid::new();
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
        let r = Strophoid::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the strophoid");
        assert!(a.ink_count() > 30);
        assert!(b.ink_count() > 30);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Strophoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
