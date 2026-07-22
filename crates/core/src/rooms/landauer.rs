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

fn plant_count(pokes: &[(f64, f64)]) -> usize {
    let hands = finite_pokes(pokes);
    if hands.is_empty() {
        return 0;
    }
    let mut n = 1usize;
    let mut prev = hands[0];
    for &p in hands.iter().skip(1) {
        let dx = p.0 - prev.0;
        let dy = p.1 - prev.1;
        if dx * dx + dy * dy > 0.01 {
            n = n.saturating_add(1);
            prev = p;
        }
    }
    n
}

fn bits(t: f64, seed: u64) -> Vec<u8> {
    let n = (10 + (phase_unit(t) * 14.0) as usize).clamp(8, 24);
    let mut rng = SplitMix64::new(SEED ^ seed);
    (0..n).map(|_| (rng.next_u64() % 2) as u8).collect()
}

fn draw_chip(canvas: &mut dyn Surface, px: i32, py: i32, on: bool, erased: bool) {
    let ch = if erased {
        '.'
    } else if on {
        '#'
    } else {
        'o'
    };
    for dy in 0..4 {
        for dx in 0..3 {
            canvas.plot(px + dx, py + dy, ch);
        }
    }
    if erased {
        canvas.plot(px + 1, py + 4, 'x');
    }
}

fn draw(canvas: &mut dyn Surface, bits: &[u8], erased: usize, heat: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = bits.len().max(1);
    let cell = (width.saturating_sub(6) / n).clamp(4, 8) as i32;
    let left = ((width as i32) - cell * n as i32).max(0) / 2;
    let py = (height as f64 * 0.28).round() as i32;
    // Register rail.
    canvas.line(left, py + 5, left + cell * n as i32, py + 5, '-');
    for (i, &b) in bits.iter().enumerate() {
        let px = left + i as i32 * cell;
        draw_chip(canvas, px, py, b == 1, i < erased);
    }
    // Heat thermometer grows with forgotten bits.
    let bar = (heat / (bits.len() as f64 * KT_LN2).max(1.0)).clamp(0.0, 1.0);
    let y = (0.72 * height as f64).round() as i32;
    let x0 = (0.12 * width as f64).round() as i32;
    let x1 = ((0.12 + bar * 0.76) * width as f64).round() as i32;
    let track = ((0.12 + 0.76) * width as f64).round() as i32;
    canvas.line(x0, y, track, y, '.');
    if x1 > x0 {
        canvas.line(x0, y, x1, y, '=');
        canvas.line(x0, y - 1, x1, y - 1, '=');
        canvas.line(x0, y + 1, x1, y + 1, '=');
    }
    canvas.plot(x0 - 2, y, 'H');
    // Price label ticks.
    for k in 0..5 {
        let tx = x0 + ((track - x0) * k) / 4;
        canvas.plot(tx, y + 2, '|');
    }
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
        let b = bits(t, self.seed);
        let ambient = (phase_unit(t) * b.len() as f64 * 0.3) as usize;
        let erased = ambient.saturating_add(plant_count(pokes)).min(b.len());
        let heat = erased as f64 * KT_LN2;
        // No hand reticle: the erased chips and heat bar are the consequence.
        draw(canvas, &b, erased, heat);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let plants = plant_count(&pokes);
        if plants == 0 {
            return self.status(t);
        }
        let b = bits(t, self.seed);
        let ambient = (phase_unit(t) * b.len() as f64 * 0.3) as usize;
        let erased = ambient.saturating_add(plants).min(b.len());
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
    fn render_is_a_readable_register() {
        let mut c = Canvas::new(80, 40);
        Landauer::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 80, "register chips must fill the plate");
        let mut large = Canvas::new(160, 90);
        Landauer::new().render(&mut large, 0.0);
        assert!(
            large.ink_count() > 60,
            "even t=0 must not look blank: {}",
            large.ink_count()
        );
    }

    #[test]
    fn forget_changes_the_picture_without_a_reticle() {
        let room = Landauer::new();
        let mut base = Canvas::new(72, 40);
        let mut poked = Canvas::new(72, 40);
        // t=0 starts with no ambient erase so a plant is unmistakable.
        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &[(0.5, 0.5)]);
        assert_ne!(base.to_text(), poked.to_text());
        assert!(
            poked.to_text().contains('x'),
            "forgotten chip should mark an x"
        );
    }

    #[test]
    fn motif_ok() {
        assert!(Landauer::new().motif().unwrap().line.len() >= 6);
    }
}
