//! The Upside-Down Ruler: p-adic ...999999 + 1 = 0.
//!
//! In the 10-adics, an infinite string of 9s to the left behaves like -1:
//! ...999 + 1 = ...000. The room stacks digit towers and shows the carry that
//! never ends until you accept the limit. CLICK: ADD ONE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_DIGITS: usize = 24;

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

/// Digits least-significant first: ambient fills with 9s; clicks add one.
fn digits(nines: usize, add_ones: u32, seed: u64) -> Vec<u8> {
    let nines = nines.clamp(1, MAX_DIGITS);
    let mut d = vec![9u8; nines];
    if seed != 0 {
        d[0] = (8 + (seed % 2) as u8).min(9);
    }
    for _ in 0..add_ones {
        let mut carry = 1u8;
        for cell in &mut d {
            let sum = *cell + carry;
            *cell = sum % 10;
            carry = sum / 10;
            if carry == 0 {
                break;
            }
        }
        if carry > 0 && d.len() < MAX_DIGITS {
            d.push(carry);
        }
    }
    d
}

fn draw_tower(canvas: &mut dyn Surface, digits: &[u8]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Draw ... d2 d1 d0 with d0 on the right (upside-down ruler growing left).
    for (i, &dig) in digits.iter().enumerate() {
        let col_from_right = i;
        let x = width.saturating_sub(2 + col_from_right * 3);
        if x >= width {
            continue;
        }
        let ch = char::from(b'0' + dig);
        let mark = match dig {
            9 => '#',
            0 => '.',
            _ => '+',
        };
        for dy in 0..(2 + dig as usize).min(height.saturating_sub(1)) {
            let y = height.saturating_sub(1 + dy) as i32;
            canvas.plot(x as i32, y, mark);
        }
        let _ = ch;
    }
    // Ellipsis on the left: infinite continuation.
    canvas.plot(1, height as i32 / 2, '.');
    canvas.plot(2, height as i32 / 2, '.');
    canvas.plot(3, height as i32 / 2, '.');
}

/// Upside-Down Ruler room.
#[derive(Debug, Default)]
pub struct UpsideRuler {
    seed: u64,
}

impl UpsideRuler {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for UpsideRuler {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "upside-ruler",
            title: "The Upside-Down Ruler",
            wing: "Number & Pattern",
            blurb: "In the 10-adics, ...999999 + 1 = 0, so ...999999 = -1. A tower of nines waits \
                    for the carry that only resolves at infinity. t grows the tower; CLICK: ADD ONE.",
            accent: [200, 160, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let nines = 4 + (phase_unit(t) * 16.0) as usize;
        let d = digits(nines, 0, self.seed);
        draw_tower(canvas, &d);
    }

    fn postcard_t(&self) -> f64 {
        0.75
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "infinite nines",
            root: 164.81,
            tempo: 88,
            line: &[0, 0, 0, 7, 0, 0, 12, 0],
            encodes: "carry cascading through an endless string of nines",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: ADD ONE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let nines = 4 + (phase_unit(t) * 16.0) as usize;
        let d = digits(nines, 0, self.seed);
        Some(format!(
            "...{} 9s  +0  CLICK:ADD ONE",
            d.iter().filter(|&&c| c == 9).count()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let nines = 4 + (phase_unit(t) * 16.0) as usize;
        let add = hands.len() as u32;
        let d = digits(nines, add, self.seed);
        draw_tower(canvas, &d);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let nines = 4 + (phase_unit(t) * 16.0) as usize;
        let add = hands.len() as u32;
        let d = digits(nines, add, self.seed);
        let zeros = d.iter().filter(|&&c| c == 0).count();
        let nines_left = d.iter().filter(|&&c| c == 9).count();
        let eq = if zeros == d.len() { "=-1+1=0" } else { "CARRY" };
        Some(format!("ADD{add}  0s={zeros}  9s={nines_left}  {eq}"))
    }

    fn reveal(&self) -> &'static str {
        "In ordinary integers, 999+1=1000. Keep the nines going forever to the \
         left and the carry never finds a place to stop: ...999+1=...000. So in \
         the 10-adic numbers, ...999 = -1. Completeness for a different absolute \
         value: |10|_10 = 1/10."
    }
}

#[cfg(test)]
mod tests {
    use super::{UpsideRuler, digits};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn nines_plus_one_carries_to_zero_prefix() {
        let d = digits(4, 1, 0);
        // 9999+1 = 10000 in finite digits -> [0,0,0,0,1]
        assert_eq!(d[0], 0);
    }

    #[test]
    fn status_invites_add() {
        let s = UpsideRuler::new().status(0.0).unwrap();
        assert!(s.contains("CLICK") || s.contains("ADD"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn add_changes_status() {
        let r = UpsideRuler::new();
        let open = r.status(0.0).unwrap();
        let after = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(open, after);
        assert!(after.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 24);
        UpsideRuler::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(UpsideRuler::new().motif().unwrap().pattern().seconds() > 0.0);
    }
}
