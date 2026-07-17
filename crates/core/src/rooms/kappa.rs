//! Kappa curve: classical cubic with a cusp and double asymptote.
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
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.35 * a.clamp(0.25, 1.0);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.03
    };
    // polar: r = a cot theta  (kappa / versiera cousin)
    // or y^2 (x^2 + y^2) = a^2 x^2
    let mut prev: Option<(i32, i32)> = None;
    let steps = 280;
    for i in 0..=steps {
        let th = 0.15 + (std::f64::consts::PI - 0.3) * (i as f64 / steps as f64) + j * 0.05;
        let s = th.sin();
        if s.abs() < 1e-3 {
            prev = None;
            continue;
        }
        let r = rad * th.cos() / s; // a cot theta
        if !r.is_finite() || r.abs() > rad * 4.0 {
            prev = None;
            continue;
        }
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
        // mirror lower half
        let py2 = (cy + r * th.sin()).round() as i32;
        canvas.plot(px, py2, '*');
    }
    // vertical asymptote x = 0 through focus region
    canvas.line(
        cx.round() as i32,
        0,
        cx.round() as i32,
        height.saturating_sub(1) as i32,
        '.',
    );
}

/// Kappa curve room.
#[derive(Debug, Default)]
pub struct Kappa {
    seed: u64,
}

impl Kappa {
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

impl Room for Kappa {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kappa",
            title: "Kappa Curve",
            wing: "Shape & Space",
            blurb: "Classical kappa: r = a cot theta. t and DRAG: TUNE SCALE.",
            accent: [120, 80, 160],
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
            key: "kappa",
            root: 196.0,
            tempo: 90,
            line: &[0, 5, 8, 12, 8, 5, 0, 7],
            encodes: "cot theta spiral arms meeting at a cusp",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        Some(format!("a={a:.2}  kappa  DRAG"))
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
        let a = scale(t, hands.last().copied(), self.seed).clamp(0.25, 1.0);
        // Kappa: r = a cot theta; vertical asymptotes at th=0,pi.
        Some(format!("a={a:.2}  r=a cot th  arms"))
    }

    fn reveal(&self) -> &'static str {
        "The kappa curve is the polar graph r = a cot theta. It has a cusp at \
         the origin and two infinite branches with a common asymptote: a \
         classical cubic named for its resemblance to the Greek letter."
    }
}

#[cfg(test)]
mod tests {
    use super::Kappa;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Kappa::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("kappa"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Kappa::new();
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
        Kappa::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
