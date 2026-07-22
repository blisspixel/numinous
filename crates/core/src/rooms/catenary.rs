//! Catenary: the hanging chain curve y = a cosh(x/a).
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

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

fn a_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.25 + x * 1.5 + s
    } else {
        0.4 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Fixed abutment span: a only changes sag depth (self-similar rescales are dead).
    let a = a.clamp(0.35, 2.2);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let x_span = 2.2_f64;
    let mut prev: Option<(i32, i32)> = None;
    // Map this chain's own [bottom, posts] into the plate. Small a hangs deep;
    // large a hangs shallow, so left and right dials are never self-similar.
    let y_bottom = a;
    let y_posts = a * (x_span / a).cosh();
    let rel_sag = ((y_posts - y_bottom) / a).clamp(0.2, 12.0);
    let fill = (0.22 + 0.55 * (rel_sag / 8.0).clamp(0.0, 1.0)).clamp(0.22, 0.78);
    let plate_posts = height as f64 * 0.1;
    let plate_bottom = plate_posts + fill * (height as f64 * 0.8);
    for col in 0..width {
        let x = -x_span + 2.0 * x_span * (col as f64 / width.saturating_sub(1).max(1) as f64) + j;
        let y = a * (x / a).cosh();
        let u = col as f64 / width.saturating_sub(1).max(1) as f64;
        // 0 at the low point of the chain, 1 at the posts.
        let rel = ((y - y_bottom) / (y_posts - y_bottom).max(1e-6)).clamp(0.0, 1.0);
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = (plate_bottom - rel * (plate_bottom - plate_posts)).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // support posts
    let post_y1 = plate_posts.round() as i32;
    let post_y0 = plate_bottom.round() as i32;
    canvas.line(2, post_y1, 2, post_y0, '|');
    canvas.line(
        width.saturating_sub(3) as i32,
        post_y1,
        width.saturating_sub(3) as i32,
        post_y0,
        '|',
    );
}

/// Catenary room.
#[derive(Debug, Default)]
pub struct Catenary {
    seed: u64,
}

impl Catenary {
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

impl Room for Catenary {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "catenary",
            title: "Catenary",
            wing: "Shape & Space",
            blurb: "The hanging chain: a cosh curve under gravity. t and DRAG: TUNE A.",
            accent: [140, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, a_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "catenary",
            root: 783.99,
            tempo: 72,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "gravity hangs a chain as a hyperbolic cosine",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = a_param(t, None, self.seed);
        Some(format!("a={a:.2}  chain  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = a_param(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = a_param(t, hands.last().copied(), self.seed).clamp(0.2, 2.5);
        // Hang span x in [-2.5a, 2.5a]; sag = y(edge) - a.
        let x_edge = 2.5 * a;
        let y_edge = a * (x_edge / a).cosh();
        let sag = y_edge - a;
        Some(format!("a={a:.2}  sag={sag:.2}  span={:.1}", 2.0 * x_edge))
    }

    fn reveal(&self) -> &'static str {
        "A catenary is the shape of a hanging flexible chain under gravity: \
         y = a cosh(x/a). It is not a parabola. Archimedes knew the problem; \
         Huygens named the curve; Bernoulli and Leibniz found the cosh form."
    }
}

#[cfg(test)]
mod tests {
    use super::Catenary;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Catenary::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("chain"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Catenary::new();
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
        Catenary::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn dial_a_moves_the_chain() {
        let r = Catenary::new();
        let mut left = Canvas::new(100, 60);
        let mut right = Canvas::new(100, 60);
        r.render_poked(&mut left, 0.5, &[(0.15, 0.5)]);
        r.render_poked(&mut right, 0.5, &[(0.85, 0.5)]);
        assert_ne!(left.to_text(), right.to_text(), "a must change sag");
    }
}
