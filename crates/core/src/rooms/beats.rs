//! Acoustic beats: sum of two close pure tones as a pulsing envelope.
//!
//! DRAG: TUNE DETUNE. See `docs/ROOMS.md`.

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

fn detune(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 8.0 + s
    } else {
        1.0 + phase_unit(t) * 6.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, df: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.42;
    let f0 = 12.0 + if seed == 0 { 0.0 } else { (seed % 4) as f64 };
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        // 2 cos(2pi f_avg x) cos(2pi (df/2) x)
        let carrier = (std::f64::consts::TAU * f0 * x).cos();
        let env = (std::f64::consts::TAU * (df * 0.5) * x).cos();
        let y = amp * carrier * env;
        let px = col as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // slow envelope
    prev = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let env = (std::f64::consts::TAU * (df * 0.5) * x).cos().abs();
        let py = (cy - amp * env).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
}

/// Beats room.
#[derive(Debug, Default)]
pub struct Beats {
    seed: u64,
}

impl Beats {
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

impl Room for Beats {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "beats",
            title: "Beats",
            wing: "Waves & Sound",
            blurb: "Two close tones pulse as one slow envelope. t and DRAG: TUNE DETUNE.",
            accent: [200, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, detune(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "beats",
            root: 369.99,
            tempo: 96,
            line: &[0, 0, 7, 7, 0, 0, 12, 12],
            encodes: "difference frequency as a hearable throb",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DETUNE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = detune(t, None, self.seed);
        Some(format!("df={d:.1}  beats  DRAG:DETUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = detune(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
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
        let d = detune(t, hands.last().copied(), self.seed);
        // Beat period T = 1/|df| for unit-Hz scale display
        let period = if d.abs() > 1e-6 {
            1.0 / d.abs()
        } else {
            f64::INFINITY
        };
        if period.is_finite() {
            Some(format!("df={d:.2}  beat T={period:.2}"))
        } else {
            Some(format!("df={d:.2}  no beats"))
        }
    }

    fn reveal(&self) -> &'static str {
        "Acoustic beats are the slow amplitude pulse when two pure tones of \
         nearby frequency sound together. The beat rate is the frequency \
         difference: piano tuners live by it."
    }
}

#[cfg(test)]
mod tests {
    use super::Beats;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Beats::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DETUNE") || s.contains("beats"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn detune_changes() {
        let r = Beats::new();
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
        Beats::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
