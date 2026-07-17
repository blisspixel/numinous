//! Semicubical parabola (cuspidal cubic): y^2 = x^3.
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

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

fn scale(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.3 + x * 0.7 + s
    } else {
        0.4 + phase_unit(t) * 0.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 * 0.2;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.4 * a.clamp(0.25, 1.0);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    // parametric: x = t^2, y = t^3  (semicubical)
    let mut prev_u: Option<(i32, i32)> = None;
    let mut prev_l: Option<(i32, i32)> = None;
    let steps = 200;
    for i in 0..=steps {
        let t = (i as f64 / steps as f64) * 1.4 + j * 0.1;
        let x = rad * t * t;
        let y = rad * t * t * t;
        let px = (cx + x).round() as i32;
        let py_u = (cy - y).round() as i32;
        let py_l = (cy + y).round() as i32;
        if let Some((ox, oy)) = prev_u {
            canvas.line(ox, oy, px, py_u, '#');
        }
        if let Some((ox, oy)) = prev_l {
            canvas.line(ox, oy, px, py_l, '#');
        }
        prev_u = Some((px, py_u));
        prev_l = Some((px, py_l));
    }
    // cusp mark
    canvas.plot(cx.round() as i32, cy.round() as i32, 'o');
}

/// Semicubical parabola room.
#[derive(Debug, Default)]
pub struct Semicubical {
    seed: u64,
}

impl Semicubical {
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

impl Room for Semicubical {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "semicubical",
            title: "Semicubical",
            wing: "Shape & Space",
            blurb: "Cuspidal cubic y squared equals x cubed. t and DRAG: TUNE SCALE.",
            accent: [180, 120, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "semicubical",
            root: 185.0,
            tempo: 86,
            line: &[0, 3, 7, 12, 7, 3, 0, 5],
            encodes: "a cusp where tangents agree and curvature blows",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        Some(format!("a={a:.2}  cusp  DRAG:SCALE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = scale(t, hands.last().copied(), self.seed);
        // Semicubical parabola y^2 = a x^3; cusp at 0.
        let y1 = a.max(0.0).sqrt();
        Some(format!("a={a:.2}  cusp  y(1)~{y1:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The semicubical parabola y^2 = x^3 is the simplest curve with a cusp. \
         It is also an evolute of a parabola and a model singularity in \
         catastrophe theory: two branches meet with a shared tangent."
    }
}

#[cfg(test)]
mod tests {
    use super::Semicubical;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Semicubical::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("cusp"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Semicubical::new();
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
        Semicubical::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
