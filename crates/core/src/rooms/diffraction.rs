//! Single-slit diffraction intensity: sinc squared envelope.
//!
//! DRAG: TUNE WIDTH. See `docs/ROOMS.md`.

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

fn width(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 4.0 + s
    } else {
        1.0 + phase_unit(t) * 3.0 + s
    }
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-8 { 1.0 } else { x.sin() / x }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width_px, height) = canvas.draw_bounds();
    if width_px == 0 || height == 0 {
        return;
    }
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    // intensity I = I0 [sinc(beta)]^2 with beta ~ a * theta
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width_px {
        let u = col as f64 / width_px.saturating_sub(1).max(1) as f64;
        let theta = (u - 0.5) * 2.0 + j * 0.05; // -1..1
        let beta = a * std::f64::consts::PI * theta;
        let i = sinc(beta).powi(2);
        let py = ((1.0 - i.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        // fill under curve
        for y in py..=height.saturating_sub(1) as i32 {
            if y % 2 == 0 {
                canvas.plot(col as i32, y, if i > 0.3 { '*' } else { '.' });
            }
        }
        prev = Some((col as i32, py));
    }
    // central mark
    let mid = (width_px.saturating_sub(1) / 2) as i32;
    canvas.line(mid, 0, mid, height.saturating_sub(1) as i32, '|');
}

/// Single-slit diffraction room.
#[derive(Debug, Default)]
pub struct Diffraction {
    seed: u64,
}

impl Diffraction {
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

impl Room for Diffraction {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "diffraction",
            title: "Diffraction",
            wing: "Waves & Sound",
            blurb: "Single-slit sinc squared intensity pattern. t and DRAG: TUNE WIDTH.",
            accent: [100, 60, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, width(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "diffraction",
            root: 622.25,
            tempo: 74,
            line: &[0, 4, 7, 12, 7, 4, 0, 12],
            encodes: "narrower slit spreads light wider",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE WIDTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = width(t, None, self.seed);
        Some(format!("a={a:.2}  slit  DRAG:WIDTH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = width(t, hands.last().copied(), self.seed);
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
        let a = width(t, hands.last().copied(), self.seed);
        // First zeros of sinc(a pi theta) at theta = +/- 1/a (unit screen coords).
        let zero = 1.0 / a.max(1e-6);
        // Half-width of central lobe (half of first-zero spacing).
        let half = zero;
        Some(format!("a={a:.2}  zero={zero:.2}  half={half:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Single-slit diffraction intensity follows a sinc squared envelope. \
         Wider slits squeeze the central peak; narrower ones spread light \
         farther: wave optics in one plot."
    }
}

#[cfg(test)]
mod tests {
    use super::Diffraction;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Diffraction::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("slit") || s.contains("WIDTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn width_changes() {
        let r = Diffraction::new();
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
        Diffraction::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
