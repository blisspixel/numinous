//! Bifolium: double-leaf algebraic curve (x^2+y^2)^2 = a x^2 y.
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

fn param_a(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.04
    };
    if let Some((x, _)) = hand {
        0.7 + x * 1.0 + s
    } else {
        1.1 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = a.clamp(0.85, 2.1) * (width.min(height) as f64) * 0.72;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.06
    };
    let show = show.clamp(0.0, 1.0);
    let steps = 560;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Ghost full leaves.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = std::f64::consts::TAU * (i as f64 / steps as f64);
        let r = a * th.sin() * th.cos().powi(2);
        if r.abs() < 1e-6 {
            prev = None;
            continue;
        }
        let ang = th + rot;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin() * 0.85).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    for i in 0..=drawn {
        let th = std::f64::consts::TAU * (i as f64 / steps as f64);
        let r = a * th.sin() * th.cos().powi(2);
        if r.abs() < 1e-6 {
            prev = None;
            continue;
        }
        let ang = th + rot;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin() * 0.85).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    let pen_th = show * std::f64::consts::TAU;
    let pen_r = a * pen_th.sin() * pen_th.cos().powi(2);
    let ang = pen_th + rot;
    let pen_x = (cx + pen_r * ang.cos()).round() as i32;
    let pen_y = (cy - pen_r * ang.sin() * 0.85).round() as i32;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(pen_x + dx, pen_y + dy, 'o');
            }
        }
    }
}

/// Bifolium room.
#[derive(Debug, Default)]
pub struct Bifolium {
    seed: u64,
}

impl Bifolium {
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

impl Room for Bifolium {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bifolium",
            title: "Bifolium",
            wing: "Shape & Space",
            blurb: "Two-leaf curve r = a sin th cos^2 th. t and DRAG: TUNE A.",
            accent: [90, 140, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(
            canvas,
            param_a(t, None, self.seed),
            phase_unit(t),
            self.seed,
        );
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bifolium",
            root: 10.91,
            tempo: 92,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "bifolium: two leaves from a quartic algebraic petal",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = param_a(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("a={a:.2}  draw={p}%  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = param_a(t, hands.last().copied(), self.seed);
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
        let a = param_a(t, hands.last().copied(), self.seed);
        // Classical bifolium: total area of both leaves is a^2 / 4.
        let area = a * a / 4.0;
        Some(format!("a={a:.2}  area={area:.3}  leaves"))
    }

    fn reveal(&self) -> &'static str {
        "The bifolium is the double-leaf curve (x^2+y^2)^2 = a x^2 y, or in polar \
         form r = a sin(theta) cos^2(theta). Two petals meet at the origin, a \
         classic algebraic flower of the seventeenth century."
    }
}

#[cfg(test)]
mod tests {
    use super::Bifolium;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Bifolium::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("2leaf"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Bifolium::new();
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
        Bifolium::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
