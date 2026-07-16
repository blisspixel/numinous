//! Landauer's Price: erase a bit, pay heat.
//!
//! A toy meter: each irreversible forget costs kT ln 2. FORGET: ONE BIT.
//! See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0x1A4D_A0E4_0000_0001;
const KT_LN2: f64 = 1.0; // natural units for the toy meter

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

fn bits(t: f64, seed: u64) -> Vec<u8> {
    let n = 8 + (phase_unit(t) * 16.0) as usize;
    let mut rng = SplitMix64::new(SEED ^ seed);
    (0..n).map(|_| (rng.next_u64() % 2) as u8).collect()
}

fn draw(canvas: &mut dyn Surface, bits: &[u8], erased: usize, heat: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for (i, &b) in bits.iter().enumerate() {
        let x = ((0.1 + i as f64 * 0.08) * width as f64).round() as i32;
        let y = (0.4 * height as f64).round() as i32;
        let ch = if i < erased {
            '.'
        } else if b == 1 {
            '#'
        } else {
            'o'
        };
        canvas.plot(x, y, ch);
        canvas.plot(x, y + 1, if i < erased { 'x' } else { ' ' });
    }
    // Heat bar.
    let bar = (heat / (bits.len() as f64 * KT_LN2).max(1.0)).clamp(0.0, 1.0);
    let y = (0.75 * height as f64).round() as i32;
    let x0 = (0.1 * width as f64).round() as i32;
    let x1 = ((0.1 + bar * 0.8) * width as f64).round() as i32;
    canvas.line(x0, y, x1, y, '=');
    canvas.plot(x0, y - 1, 'H');
}

/// Landauer room.
#[derive(Debug, Default)]
pub struct Landauer {
    seed: u64,
}

impl Landauer {
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

impl Room for Landauer {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "landauer",
            title: "Landauer's Price",
            wing: "Number & Pattern",
            blurb: "Erase a bit, pay heat: kT ln 2 per irreversible forget. t grows the register; \
                    CLICK: FORGET ONE BIT.",
            accent: [255, 120, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let b = bits(t, self.seed);
        let erased = (phase_unit(t) * b.len() as f64 * 0.3) as usize;
        let heat = erased as f64 * KT_LN2;
        draw(canvas, &b, erased, heat);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "landauer heat",
            root: 110.0,
            tempo: 72,
            line: &[0, 0, 5, 0, 7, 0, 12, 0],
            encodes: "each forgotten bit meters a quantum of heat",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: FORGET ONE BIT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = bits(t, self.seed);
        let erased = (phase_unit(t) * b.len() as f64 * 0.3) as usize;
        let heat = erased as f64 * KT_LN2;
        Some(format!("bits={}  heat={heat:.1}  CLICK:FORGET", b.len()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = bits(t, self.seed);
        let erased = hands.len().min(b.len());
        let heat = erased as f64 * KT_LN2;
        draw(canvas, &b, erased, heat);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = bits(t, self.seed);
        let erased = hands.len().min(b.len());
        let heat = erased as f64 * KT_LN2;
        Some(format!("FORGET n={erased}  Q={heat:.2} kTln2"))
    }

    fn reveal(&self) -> &'static str {
        "Landauer's principle: erasing one bit of information in a system at \
         temperature T dissipates at least kT ln 2 of heat. Computation has a \
         physical price; reversible steps can dodge the bill."
    }
}

#[cfg(test)]
mod tests {
    use super::Landauer;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Landauer::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("FORGET"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn forget_changes() {
        let r = Landauer::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        Landauer::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(Landauer::new().motif().unwrap().line.len() >= 6);
    }
}
