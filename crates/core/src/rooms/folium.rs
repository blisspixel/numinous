//! Folium of Descartes: the classical cubic x^3 + y^3 = 3 a x y.
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
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.3 + x * 1.0 + s
    } else {
        0.5 + phase_unit(t) * 0.7 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = a.clamp(0.2, 1.5);
    let cx = width as f64 * 0.35;
    let cy = height as f64 * 0.65;
    let scale = (width.min(height) as f64) * 0.4 / a.max(0.3);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    // parametric: x = 3 a t / (1+t^3), y = 3 a t^2 / (1+t^3)
    let mut prev: Option<(i32, i32)> = None;
    let steps = 300;
    for i in 0..=steps {
        // skip near t = -1 pole
        let t = -2.5 + 5.0 * (i as f64 / steps as f64) + j * 0.05;
        if (t + 1.0).abs() < 0.08 {
            prev = None;
            continue;
        }
        let d = 1.0 + t * t * t;
        if d.abs() < 1e-6 {
            prev = None;
            continue;
        }
        let x = 3.0 * a * t / d;
        let y = 3.0 * a * t * t / d;
        if !x.is_finite() || !y.is_finite() || x.abs() > 4.0 * a || y.abs() > 4.0 * a {
            prev = None;
            continue;
        }
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // asymptote x + y + a = 0
    // y = -x - a
    let x0 = -2.0 * a;
    let x1 = 2.0 * a;
    let y0 = -x0 - a;
    let y1 = -x1 - a;
    let ax0 = (cx + scale * x0).round() as i32;
    let ay0 = (cy - scale * y0).round() as i32;
    let ax1 = (cx + scale * x1).round() as i32;
    let ay1 = (cy - scale * y1).round() as i32;
    canvas.line(ax0, ay0, ax1, ay1, '.');
}

/// Folium of Descartes room.
#[derive(Debug, Default)]
pub struct Folium {
    seed: u64,
}

impl Folium {
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

impl Room for Folium {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "folium",
            title: "Folium",
            wing: "Shape & Space",
            blurb: "Descartes' leaf cubic with a node and asymptote. t and DRAG: TUNE A.",
            accent: [60, 140, 80],
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
            key: "folium",
            root: 164.8,
            tempo: 84,
            line: &[0, 5, 7, 12, 7, 5, 0, 9],
            encodes: "a cubic leaf that taught Fermat about tangents",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = a_param(t, None, self.seed);
        Some(format!("a={a:.2}  leaf  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = a_param(t, hands.last().copied(), self.seed);
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
        let a = a_param(t, hands.last().copied(), self.seed);
        Some(format!("A={a:.3}  folium"))
    }

    fn reveal(&self) -> &'static str {
        "The folium of Descartes is the cubic x^3 + y^3 = 3 a x y. Descartes \
         proposed it; Fermat found its tangents. The loop is a node; the far \
         branch approaches the line x + y + a = 0."
    }
}

#[cfg(test)]
mod tests {
    use super::Folium;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Folium::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("leaf"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Folium::new();
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
        Folium::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
