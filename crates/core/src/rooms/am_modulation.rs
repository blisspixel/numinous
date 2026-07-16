//! Amplitude modulation: carrier times (1 + m cos omega_m t).
//!
//! DRAG: TUNE MOD INDEX. See `docs/ROOMS.md`.

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

fn mod_index(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * 1.4 + s
    } else {
        0.2 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, m: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.38;
    let fc = 16.0 + if seed == 0 { 0.0 } else { (seed % 4) as f64 };
    let fm = 2.0;
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let env = 1.0 + m * (std::f64::consts::TAU * fm * x).cos();
        let y = amp * 0.5 * env * (std::f64::consts::TAU * fc * x).cos();
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // envelope
    prev = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let env = (1.0 + m * (std::f64::consts::TAU * fm * x).cos()).abs();
        let py = (cy - amp * 0.5 * env).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
}

/// AM modulation room.
#[derive(Debug, Default)]
pub struct AmModulation {
    seed: u64,
}

impl AmModulation {
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

impl Room for AmModulation {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "am-modulation",
            title: "AM Modulation",
            wing: "Waves & Sound",
            blurb: "Carrier times slow envelope: radio AM. t and DRAG: TUNE MOD INDEX.",
            accent: [80, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, mod_index(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "am modulation",
            root: 466.16,
            tempo: 92,
            line: &[0, 5, 7, 12, 7, 5, 0, 10],
            encodes: "sidebands at carrier plus and minus the message",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE MOD INDEX")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mod_index(t, None, self.seed);
        Some(format!("m={m:.2}  AM  DRAG:M"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = mod_index(t, hands.last().copied(), self.seed);
        draw(canvas, m, self.seed ^ hands.len() as u64);
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
        let m = mod_index(t, hands.last().copied(), self.seed);
        let depth = (m * 100.0).round() as i32;
        // Carrier power share for tone AM: 2/(2+m^2) of total sideband+carrier.
        let carrier_pct = ((2.0 / (2.0 + m * m)) * 100.0).round() as i32;
        let mode = if m > 1.0 {
            "OVER"
        } else if m > 0.5 {
            "deep"
        } else {
            "light"
        };
        Some(format!("m={m:.2}  {depth}%  car={carrier_pct}%  {mode}"))
    }

    fn reveal(&self) -> &'static str {
        "Amplitude modulation multiplies a fast carrier by a slow message \
         envelope. Sidebands appear at fc ± fm; modulation index m > 1 \
         overmodulates and distorts."
    }
}

#[cfg(test)]
mod tests {
    use super::AmModulation;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = AmModulation::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("AM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn m_changes() {
        let r = AmModulation::new();
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
        AmModulation::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}
