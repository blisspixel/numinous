//! Two Descriptions, One Truth: duality as a play verb.
//!
//! One object, two languages: a polygon and its dual, or a function and its
//! transform. TOGGLE: DUAL VIEW. See `docs/ROOMS.md`.

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

fn n_sides(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (3 + (x * 6.0) as usize).clamp(3, 8)
    } else {
        (3 + (phase_unit(t) * 5.0) as usize).clamp(3, 7)
    }
}

fn dual_view(t: f64, hand: Option<(f64, f64)>) -> bool {
    if let Some((_, y)) = hand {
        y > 0.5
    } else {
        phase_unit(t) > 0.5
    }
}

fn polygon(n: usize, r: f64, rot: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = rot + TAU * i as f64 / n as f64;
            (0.5 + r * a.cos(), 0.5 + r * a.sin())
        })
        .collect()
}

fn dual_of(verts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    // Dual vertices = edge midpoints, scaled out slightly (polar dual toy).
    let n = verts.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        let mx = (verts[i].0 + verts[j].0) * 0.5;
        let my = (verts[i].1 + verts[j].1) * 0.5;
        let dx = mx - 0.5;
        let dy = my - 0.5;
        let s = 1.15;
        out.push((0.5 + dx * s, 0.5 + dy * s));
    }
    out
}

fn draw(canvas: &mut dyn Surface, verts: &[(f64, f64)], ch: char) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || verts.len() < 2 {
        return;
    }
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        (
            (p.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (p.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    for i in 0..verts.len() {
        let a = to_px(verts[i]);
        let b = to_px(verts[(i + 1) % verts.len()]);
        canvas.line(a.0, a.1, b.0, b.1, ch);
        canvas.plot(a.0, a.1, '+');
    }
}

/// Duality room.
#[derive(Debug, Default)]
pub struct Duality {
    seed: u64,
}

impl Duality {
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

impl Room for Duality {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "duality",
            title: "Two Descriptions, One Truth",
            wing: "Shape & Space",
            blurb: "One polygon, two languages: faces become vertices in the dual. t and DRAG: \
                    TOGGLE DUAL VIEW.",
            accent: [180, 140, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = n_sides(t, None);
        let rot = if self.seed == 0 {
            0.0
        } else {
            (self.seed % 8) as f64 * 0.05
        };
        let prim = polygon(n, 0.32, rot);
        let dual = dual_of(&prim);
        if dual_view(t, None) {
            draw(canvas, &dual, '#');
            draw(canvas, &prim, '.');
        } else {
            draw(canvas, &prim, '#');
            draw(canvas, &dual, '.');
        }
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dual view",
            root: 277.18,
            tempo: 100,
            line: &[0, 7, 12, 7, 0, 5, 12, 7],
            encodes: "one object spoken in two geometries",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TOGGLE DUAL VIEW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_sides(t, None);
        let dual = dual_view(t, None);
        Some(format!(
            "n={n}  view={}  DRAG:DUAL",
            if dual { "DUAL" } else { "PRIM" }
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_sides(t, hands.last().copied());
        let prim = polygon(n, 0.32, 0.0);
        let dual = dual_of(&prim);
        if dual_view(t, hands.last().copied()) {
            draw(canvas, &dual, '#');
            draw(canvas, &prim, '.');
        } else {
            draw(canvas, &prim, '#');
            draw(canvas, &dual, '.');
        }
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
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
        let n = n_sides(t, hands.last().copied());
        let dual = dual_view(t, hands.last().copied());
        Some(format!(
            "TOGGLE n={n}  {}  same truth",
            if dual { "DUAL" } else { "PRIM" }
        ))
    }

    fn reveal(&self) -> &'static str {
        "Duality is one truth in two languages: faces become vertices, a \
         function becomes its transform. Switching views does not change the \
         object; it changes which edges of the truth you can hold."
    }
}

#[cfg(test)]
mod tests {
    use super::{Duality, dual_of, polygon};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Duality::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("DUAL"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn toggle_changes() {
        let r = Duality::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.9,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn dual_same_count() {
        let p = polygon(5, 0.3, 0.0);
        assert_eq!(dual_of(&p).len(), 5);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Duality::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(Duality::new().motif().unwrap().line.len() >= 6);
    }
}
