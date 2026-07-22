//! Nephroid: two-cusped epicycloid (kidney-shaped roulette).
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

fn scale(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.55 + x * 0.55 + s
    } else {
        0.85 + s * 0.5
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.32 * a.clamp(0.5, 1.15);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.03
    };
    let show = show.clamp(0.0, 1.0);
    let steps = 480;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Ghost full kidney.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = rad * (3.0 * th.cos() - (3.0 * th).cos());
        let y = rad * (3.0 * th.sin() - (3.0 * th).sin());
        let px = (cx + x * 0.55).round() as i32;
        let py = (cy - y * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    for i in 0..=drawn {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = rad * (3.0 * th.cos() - (3.0 * th).cos());
        let y = rad * (3.0 * th.sin() - (3.0 * th).sin());
        let px = (cx + x * 0.55).round() as i32;
        let py = (cy - y * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Pen bead.
    let pen_th = rot + show * std::f64::consts::TAU;
    let pen_x = (cx + rad * (3.0 * pen_th.cos() - (3.0 * pen_th).cos()) * 0.55).round() as i32;
    let pen_y = (cy - rad * (3.0 * pen_th.sin() - (3.0 * pen_th).sin()) * 0.55).round() as i32;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(pen_x + dx, pen_y + dy, 'o');
            }
        }
    }
}

/// Nephroid room.
#[derive(Debug, Default)]
pub struct Nephroid {
    seed: u64,
}

impl Nephroid {
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

impl Room for Nephroid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "nephroid",
            title: "Nephroid",
            wing: "Shape & Space",
            blurb: "Two-cusped kidney curve from a rolling circle. t and DRAG: TUNE SCALE.",
            accent: [180, 100, 60],
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
            key: "nephroid",
            root: 392.0,
            tempo: 86,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "epicycloid with two cusps like a kidney",
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
        // Nephroid: 2-cusped epicycloid; perimeter 24a, area 12 pi a^2.
        let perim = 24.0 * a;
        let area = 12.0 * std::f64::consts::PI * a * a;
        Some(format!("a={a:.2}  P={perim:.1}  A={area:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "A nephroid is an epicycloid with two cusps: a circle rolls outside a \
         fixed circle of equal radius. It appears as a catacaustic of a circle \
         under parallel light: a kidney of reflected rays."
    }
}

#[cfg(test)]
mod tests {
    use super::Nephroid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Nephroid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("nephroid"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Nephroid::new();
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
        Nephroid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
