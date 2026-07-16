//! Blackbody spectrum: Planck curve vs wavelength, Wien peak.
//!
//! DRAG: TUNE TEMP. See `docs/ROOMS.md`.

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

fn temp(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 100.0
    };
    if let Some((x, _)) = hand {
        2000.0 + x * 6000.0 + s
    } else {
        2500.0 + phase_unit(t) * 5000.0 + s
    }
}

/// Toy spectral radiance ~ 1/x^5 / (exp(c/xT)-1) with scaled constants.
fn planck(x: f64, t: f64) -> f64 {
    // x in (0,1] maps wavelength; c' chosen so peak sits in range
    let c = 1.4;
    let u = c / (x * t / 3000.0).max(1e-6);
    if u > 40.0 {
        return 0.0;
    }
    let denom = u.exp() - 1.0;
    if denom <= 0.0 {
        return 0.0;
    }
    x.powi(-5) / denom
}

fn draw(canvas: &mut dyn Surface, t_kelvin: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let t_kelvin = t_kelvin.clamp(1500.0, 9000.0);
    let mut max_b = 1e-12_f64;
    let mut vals = Vec::with_capacity(width);
    for col in 0..width {
        let x = 0.08 + 0.92 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let b = planck(x, t_kelvin);
        max_b = max_b.max(b);
        vals.push(b);
    }
    let mut prev: Option<(i32, i32)> = None;
    let mut peak_col = 0usize;
    let mut peak_b = 0.0;
    for (col, &b) in vals.iter().enumerate() {
        if b > peak_b {
            peak_b = b;
            peak_col = col;
        }
        let y = ((1.0 - b / max_b) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, y, '#');
        }
        prev = Some((col as i32, y));
    }
    // Wien peak mark
    canvas.line(
        peak_col as i32,
        0,
        peak_col as i32,
        height.saturating_sub(1) as i32,
        '|',
    );
    let _ = seed;
}

/// Blackbody room.
#[derive(Debug, Default)]
pub struct Blackbody {
    seed: u64,
}

impl Blackbody {
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

impl Room for Blackbody {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "blackbody",
            title: "Blackbody Spectrum",
            wing: "Waves & Sound",
            blurb: "Planck curve and Wien peak shift with T. t and DRAG: TUNE TEMP.",
            accent: [180, 80, 30],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, temp(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "blackbody",
            root: 8.66,
            tempo: 84,
            line: &[0, 4, 7, 12, 9, 4, 0, 7],
            encodes: "Planck blackbody: peak wavelength slides as 1/T (Wien)",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TEMP")
    }

    fn status(&self, t: f64) -> Option<String> {
        let tk = temp(t, None, self.seed);
        Some(format!("T={tk:.0}K  planck  DRAG:T"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let tk = temp(t, hands.last().copied(), self.seed);
        draw(canvas, tk, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let tk = temp(t, hands.last().copied(), self.seed);
        Some(format!("T={tk:.0}K  wien"))
    }

    fn reveal(&self) -> &'static str {
        "A blackbody's spectrum is the Planck curve. Wien's displacement law says \
         the peak wavelength times temperature is constant: hotter stars look \
         bluer because their peak slides toward shorter wavelengths."
    }
}

#[cfg(test)]
mod tests {
    use super::Blackbody;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Blackbody::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("planck"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn temp_changes() {
        let r = Blackbody::new();
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
        Blackbody::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}
