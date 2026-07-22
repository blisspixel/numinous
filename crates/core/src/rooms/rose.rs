//! Rhodonea (rose) curves: r = cos(k theta) polar flowers.
//!
//! Ambient phase draws the petals with a traveling pen. DRAG: TUNE PETALS.
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

fn petals(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        // Hand x sets petal count; y scrubs the pen in render_poked.
        1.5 + x * 7.0 + s
    } else {
        // Ambient k holds a readable flower; motion lives in the pen.
        3.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.42;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.05
    };
    let show = show.clamp(0.0, 1.0);
    // Two full turns so even roses close cleanly.
    let steps = 720;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    let stroke = if k > 5.0 { '#' } else { '*' };
    // Soft ghost of the full rose.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = rot + std::f64::consts::TAU * 2.0 * (i as f64 / steps as f64);
        let r = (k * th).cos().abs();
        let px = (cx + rad * r * th.cos()).round() as i32;
        let py = (cy - rad * r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright petals so far.
    prev = None;
    for i in 0..=drawn {
        let th = rot + std::f64::consts::TAU * 2.0 * (i as f64 / steps as f64);
        let r = (k * th).cos().abs();
        let px = (cx + rad * r * th.cos()).round() as i32;
        let py = (cy - rad * r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, stroke);
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    let pen_th = rot + show * std::f64::consts::TAU * 2.0;
    let pen_r = (k * pen_th).cos().abs();
    let pen_x = (cx + rad * pen_r * pen_th.cos()).round() as i32;
    let pen_y = (cy - rad * pen_r * pen_th.sin()).round() as i32;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(pen_x + dx, pen_y + dy, 'o');
            }
        }
    }
}

/// Rose curve room.
#[derive(Debug, Default)]
pub struct Rose {
    seed: u64,
}

impl Rose {
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

impl Room for Rose {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "rose",
            title: "Rose Curve",
            wing: "Shape & Space",
            blurb: "Rhodonea petals draw themselves. Watch the pen; DRAG: TUNE PETALS.",
            accent: [220, 40, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, petals(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "rose",
            root: 293.66,
            tempo: 100,
            line: &[0, 4, 7, 11, 7, 4, 0, 12],
            encodes: "polar cosine petals counted by k",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PETALS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = petals(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("k={k:.2}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = petals(t, hands.last().copied(), self.seed);
        // When held, x still drives k via petals(); y scrubs the pen.
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, k, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let k = petals(t, hands.last().copied(), self.seed);
        // Integer k: k petals if odd, 2k if even (for cos)
        let ki = k.round() as i32;
        let count = if ki % 2 == 0 { 2 * ki } else { ki };
        Some(format!("PETALS k={k:.2}  ~{count}"))
    }

    fn reveal(&self) -> &'static str {
        "Rhodonea curves are polar roses r = cos(k theta). Integer k yields \
         k petals when odd and 2k when even. Rational k fills denser lace; \
         the family is classical polar geometry made floral."
    }
}

#[cfg(test)]
mod tests {
    use super::Rose;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Rose::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("draw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn petals_change() {
        let r = Rose::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn ambient_pen_moves_the_plate() {
        let r = Rose::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the petals");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Rose::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}
