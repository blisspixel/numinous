//! The Upside-Down Ruler: p-adic ...999999 + 1 = 0.
//!
//! In the 10-adics, an infinite string of 9s to the left behaves like -1:
//! ...999 + 1 = ...000. The room stacks digit columns (least significant on the
//! right) and shows the carry that never ends until you accept the limit.
//! CLICK: ADD ONE. See `docs/ROOMS.md`.

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

/// How many completed plants (pointer downs) sit in the trail.
fn plant_count(pokes: &[(f64, f64)]) -> u32 {
    // Rooms receive down+move samples as pokes. Count spaced plants so a
    // jittery click is one add, not a flood of +1s from micro-moves.
    let hands = finite_pokes(pokes);
    if hands.is_empty() {
        return 0;
    }
    let mut n = 1u32;
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

fn draw_tower(canvas: &mut dyn Surface, digits: &[u8], add_ones: u32) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let w = width as i32;
    let h = height as i32;
    // Floor line: the upside-down "edge" of the ruler.
    let floor = h.saturating_sub(2);
    canvas.line(0, floor, w.saturating_sub(1), floor, '.');

    // Ellipsis on the left: digits continue forever toward higher powers.
    let mid = (h / 2).max(1);
    for i in 0i32..3 {
        canvas.plot(1 + i, mid, '.');
    }

    // Columns grow left from the right edge. Cell width scales to the tower.
    let cols = digits.len().max(1);
    let cell = (width.saturating_sub(6) / cols).clamp(2, 5) as i32;
    let right = w.saturating_sub(2);

    for (i, &dig) in digits.iter().enumerate() {
        let x = right - (i as i32) * cell;
        if x < 4 {
            break;
        }
        let bar_h = (2 + dig as i32).min(floor.saturating_sub(2));
        let glyph = char::from(b'0' + dig);
        let fill = match dig {
            9 => '#',
            0 => {
                if add_ones > 0 {
                    '*'
                } else {
                    '.'
                }
            }
            _ => '+',
        };
        // Vertical tick (ruler mark) growing upward from the floor.
        for dy in 0..bar_h {
            let y = floor - 1 - dy;
            if y >= 0 {
                canvas.plot(x, y, fill);
            }
        }
        // Digit label sits on the tick so the carry is readable.
        let label_y = (floor - 1 - bar_h).max(0);
        canvas.plot(x, label_y, glyph);
        // Small baseline tick under the floor for ruler feel.
        canvas.plot(x, floor, '|');
    }

    // After +1, flash the carry story on the right edge.
    if add_ones > 0 {
        let tag_x = w.saturating_sub(1);
        canvas.plot(tag_x, 1, '+');
        canvas.plot(tag_x, 2, '1');
        let zeros = digits.iter().filter(|&&c| c == 0).count();
        if zeros == digits.len() {
            canvas.plot(tag_x.saturating_sub(2), mid, '=');
            canvas.plot(tag_x.saturating_sub(1), mid, '0');
        }
    }
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
        draw_tower(canvas, &d, 0);
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
        let nines = 4 + (phase_unit(t) * 16.0) as usize;
        let add = plant_count(pokes);
        let d = digits(nines, add, self.seed);
        draw_tower(canvas, &d, add);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let add = plant_count(&pokes);
        if add == 0 {
            return self.status(t);
        }
        let nines = 4 + (phase_unit(t) * 16.0) as usize;
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
    use super::{UpsideRuler, digits, plant_count};
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
    fn click_changes_the_picture() {
        let room = UpsideRuler::new();
        let mut base = Canvas::new(56, 28);
        let mut poked = Canvas::new(56, 28);
        room.render(&mut base, 0.4);
        room.render_poked(&mut poked, 0.4, &[(0.5, 0.5)]);
        assert_ne!(base.to_text(), poked.to_text());
        // Digits must be labeled, not only abstract bars.
        assert!(
            poked.to_text().chars().any(|c| c.is_ascii_digit()),
            "upside ruler must show digit glyphs after +1"
        );
    }

    #[test]
    fn jittery_trail_counts_as_one_plant() {
        let trail = [(0.50, 0.50), (0.51, 0.50), (0.52, 0.51)];
        assert_eq!(plant_count(&trail), 1);
        let two = [(0.2, 0.2), (0.8, 0.8)];
        assert_eq!(plant_count(&two), 2);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 24);
        UpsideRuler::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
        assert!(
            c.to_text().chars().any(|ch| ch.is_ascii_digit()),
            "ambient tower must show digit labels"
        );
    }

    #[test]
    fn motif_ok() {
        assert!(UpsideRuler::new().motif().unwrap().pattern().seconds() > 0.0);
    }
}
