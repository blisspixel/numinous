//! The Message That Heals: Hamming codes repair wounds mid-flight.
//!
//! A binary string is sent through noise; Hamming(7,4) parity bits heal single
//! flips until the noise cliff overwhelms them. DRAG: RAISE THE NOISE. See
//! `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0x4A11_5EED_0000_0001;

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

/// Encode 4 data bits to 7 Hamming bits (positions 1..7 with parity at powers of two).
fn hamming74_encode(nibble: u8) -> [u8; 7] {
    let d = [
        (nibble >> 3) & 1,
        (nibble >> 2) & 1,
        (nibble >> 1) & 1,
        nibble & 1,
    ];
    // positions: 1p 2p 3d 4p 5d 6d 7d
    let mut c = [0u8; 7];
    c[2] = d[0];
    c[4] = d[1];
    c[5] = d[2];
    c[6] = d[3];
    c[0] = c[2] ^ c[4] ^ c[6];
    c[1] = c[2] ^ c[5] ^ c[6];
    c[3] = c[4] ^ c[5] ^ c[6];
    c
}

fn syndrome(c: &[u8; 7]) -> u8 {
    let s1 = c[0] ^ c[2] ^ c[4] ^ c[6];
    let s2 = c[1] ^ c[2] ^ c[5] ^ c[6];
    let s4 = c[3] ^ c[4] ^ c[5] ^ c[6];
    s1 | (s2 << 1) | (s4 << 2)
}

fn heal(mut c: [u8; 7]) -> ([u8; 7], u8) {
    let s = syndrome(&c);
    if s != 0 && (s as usize) <= 7 {
        let i = (s as usize) - 1;
        c[i] ^= 1;
    }
    (c, s)
}

fn channel(code: [u8; 7], p_flip: f64, seed: u64, msg: u64) -> [u8; 7] {
    // Fold p into the seed so dial/phase cannot land on the same flip mask.
    let p_bits = (p_flip.clamp(0.0, 1.0) * 10_000.0) as u64;
    let mut rng = SplitMix64::new(SEED ^ seed ^ msg.wrapping_mul(0x9E37_79B9) ^ p_bits);
    let mut out = code;
    for b in &mut out {
        if rng.next_f64() < p_flip {
            *b ^= 1;
        }
    }
    // Guarantee at least one wound once noise is past the soft cliff so the
    // mid row never freezes as a silent copy of the sent code.
    if p_flip >= 0.12 {
        let mut any = false;
        for i in 0..7 {
            if out[i] != code[i] {
                any = true;
                break;
            }
        }
        if !any {
            let i = (p_bits as usize) % 7;
            out[i] ^= 1;
        }
    }
    out
}

fn noise_p(t: f64, pokes: &[(f64, f64)]) -> f64 {
    let hands = finite_pokes(pokes);
    if let Some(&(x, _)) = hands.last() {
        (0.02 + x * 0.55).clamp(0.02, 0.55)
    } else {
        (0.04 + phase_unit(t) * 0.48).clamp(0.04, 0.52)
    }
}

fn draw_bit_block(canvas: &mut dyn Surface, px: i32, py: i32, on: bool, wound: bool) {
    let ch = if on { '#' } else { '.' };
    // Fat bit cell so a large window is a row of chips, not seven freckles.
    for dy in 0..4 {
        for dx in 0..4 {
            canvas.plot(px + dx, py + dy, ch);
        }
    }
    if wound {
        canvas.plot(px + 1, py + 4, 'x');
        canvas.plot(px + 2, py + 4, 'x');
        canvas.plot(px + 1, py - 1, 'x');
        canvas.plot(px + 2, py - 1, 'x');
    }
}

fn draw(canvas: &mut dyn Surface, sent: &[u8; 7], recv: &[u8; 7], fixed: &[u8; 7], p: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cell = (width.saturating_sub(8) / 7).clamp(5, 12) as i32;
    let left = ((width as i32) - cell * 7) / 2;
    let rows = [
        (sent, 0.2_f64, false),
        (recv, 0.48, true),
        (fixed, 0.76, false),
    ];
    for (bits, yf, mark_bad) in rows {
        let py = (yf * height.saturating_sub(5) as f64).round() as i32;
        for (i, &b) in bits.iter().enumerate() {
            let px = left + i as i32 * cell;
            let wound = mark_bad && b != sent[i];
            draw_bit_block(canvas, px, py, b == 1, wound);
            // Wire between chips.
            if i + 1 < bits.len() {
                let wx0 = px + 4;
                let wx1 = left + (i as i32 + 1) * cell - 1;
                canvas.line(wx0, py + 1, wx1, py + 1, '-');
                canvas.line(wx0, py + 2, wx1, py + 2, '-');
            }
        }
        // Bus from previous row.
        if (yf - 0.2).abs() > 0.05 {
            let mid = left + cell * 3 + 1;
            let y0 = ((0.2 + 0.08) * height.saturating_sub(5) as f64).round() as i32;
            canvas.line(mid, y0, mid, py, if mark_bad { '~' } else { '|' });
        }
    }
    // Noise meter: the dial must move something even when flips are sparse.
    let meter_y = height.saturating_sub(1) as i32;
    let filled = (p.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(0, meter_y, width.saturating_sub(1) as i32, meter_y, '-');
    if filled > 0 {
        canvas.line(0, meter_y, filled, meter_y, '=');
    }
}

/// Message That Heals room.
#[derive(Debug, Default)]
pub struct MessageHeals {
    seed: u64,
}

impl MessageHeals {
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

impl Room for MessageHeals {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "message-heals",
            title: "The Message That Heals",
            wing: "Number & Pattern",
            blurb: "Hamming(7,4) parity bits heal single flips mid-flight until noise wins. t and \
                    DRAG: RAISE THE NOISE walk the cliff.",
            accent: [100, 200, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let nibble = 0b1011u8;
        let sent = hamming74_encode(nibble);
        let p = noise_p(t, &[]);
        let recv = channel(sent, p, self.seed, 0);
        let (fixed, _) = heal(recv);
        draw(canvas, &sent, &recv, &fixed, p);
    }

    fn postcard_t(&self) -> f64 {
        0.25
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "parity heal",
            root: 196.0,
            tempo: 110,
            line: &[0, 4, 7, 4, 0, 7, 12, 0],
            encodes: "a flipped bit found and restored by syndrome",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: RAISE THE NOISE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = noise_p(t, &[]);
        let sent = hamming74_encode(0b1011);
        let recv = channel(sent, p, self.seed, 0);
        let (fixed, s) = heal(recv);
        let ok = fixed == sent;
        Some(format!(
            "p={p:.2}  syn={s}  {}  DRAG:NOISE",
            if ok { "HEALED" } else { "BROKEN" }
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let p = noise_p(t, pokes);
        let sent = hamming74_encode(0b1011);
        let recv = channel(sent, p, self.seed, 1);
        let (fixed, _) = heal(recv);
        draw(canvas, &sent, &recv, &fixed, p);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        if finite_pokes(&pokes).is_empty() {
            return self.status(t);
        }
        let p = noise_p(t, &pokes);
        let sent = hamming74_encode(0b1011);
        let recv = channel(sent, p, self.seed, 1);
        let flips = sent.iter().zip(recv.iter()).filter(|(a, b)| a != b).count();
        let (fixed, s) = heal(recv);
        let ok = fixed == sent;
        Some(format!(
            "NOISE p={p:.2}  flips={flips}  syn={s}  {}",
            if ok { "OK" } else { "FAIL" }
        ))
    }

    fn reveal(&self) -> &'static str {
        "Hamming's code adds parity bits so any single flip points to its own \
         address: the syndrome. Raise the noise and double flips defeat it. Error \
         correction is geometry in Hamming space until the cliff."
    }
}

#[cfg(test)]
mod tests {
    use super::{MessageHeals, hamming74_encode, heal};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn heals_single_flip() {
        let mut c = hamming74_encode(0b1001);
        let clean = c;
        c[3] ^= 1;
        let (fixed, s) = heal(c);
        assert_ne!(s, 0);
        assert_eq!(fixed, clean);
    }

    #[test]
    fn status_invites() {
        let s = MessageHeals::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("NOISE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn noise_changes() {
        let r = MessageHeals::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.4,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 24);
        MessageHeals::new().render(&mut c, 0.2);
        assert!(
            c.ink_count() > 40,
            "bit rows must be fat cells, not freckles"
        );
    }

    #[test]
    fn motif_ok() {
        assert!(MessageHeals::new().motif().unwrap().line.len() >= 6);
    }
}
