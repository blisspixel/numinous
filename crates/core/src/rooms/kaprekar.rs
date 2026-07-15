//! Kaprekar 6174: the number that eats numbers.
//!
//! Every four-digit number with not all digits equal reaches 6174 in at most
//! seven Kaprekar steps (largest rearrangement minus smallest). CLICK: FEED
//! A NUMBER. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const TARGET: u32 = 6174;

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

fn digits4(n: u32) -> [u8; 4] {
    let n = n % 10000;
    [
        ((n / 1000) % 10) as u8,
        ((n / 100) % 10) as u8,
        ((n / 10) % 10) as u8,
        (n % 10) as u8,
    ]
}

fn from_digits(d: [u8; 4]) -> u32 {
    d[0] as u32 * 1000 + d[1] as u32 * 100 + d[2] as u32 * 10 + d[3] as u32
}

fn kap_step(n: u32) -> u32 {
    let mut d = digits4(n);
    d.sort_unstable();
    let small = from_digits(d);
    d.reverse();
    let large = from_digits(d);
    large.saturating_sub(small)
}

fn kap_chain(start: u32, max_steps: usize) -> Vec<u32> {
    let mut chain = vec![start % 10000];
    let mut n = start % 10000;
    for _ in 0..max_steps {
        if n == TARGET || digits4(n).windows(2).all(|w| w[0] == w[1]) {
            break;
        }
        n = kap_step(n);
        chain.push(n);
        if n == TARGET {
            break;
        }
    }
    chain
}

fn start_from(t: f64, seed: u64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, y)) = hand {
        let n = (x * 9000.0 + y * 999.0) as u32 + 1000;
        n % 10000
    } else {
        let base = 1000 + (phase_unit(t) * 8999.0) as u32;
        if seed == 0 {
            base
        } else {
            (base + (seed % 997) as u32) % 10000
        }
    }
}

fn draw(canvas: &mut dyn Surface, chain: &[u32]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || chain.is_empty() {
        return;
    }
    // Plot each step as a bar height from the number.
    for (i, &n) in chain.iter().enumerate() {
        let x = ((0.1 + i as f64 * 0.1) * width as f64).round() as i32;
        let h = (n as f64 / 9999.0) * height as f64 * 0.8;
        let y1 = height.saturating_sub(1) as i32;
        let y0 = (y1 as f64 - h).round() as i32;
        let ch = if n == TARGET { '#' } else { '*' };
        canvas.line(x, y1, x, y0, ch);
        // Digit glyphs as simple plots.
        let d = digits4(n);
        for (k, &dig) in d.iter().enumerate() {
            let px = x - 1 + k as i32;
            let py = (y0 - 2).max(0);
            if dig > 0 {
                canvas.plot(px, py, char::from(b'0' + dig));
            } else {
                canvas.plot(px, py, '0');
            }
        }
    }
    // Target line.
    let ty = (height as f64 * (1.0 - TARGET as f64 / 9999.0 * 0.8)).round() as i32;
    canvas.line(0, ty, width.saturating_sub(1) as i32, ty, '.');
}

/// Kaprekar room.
#[derive(Debug, Default)]
pub struct Kaprekar {
    seed: u64,
}

impl Kaprekar {
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

impl Room for Kaprekar {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kaprekar",
            title: "The Number That Eats Numbers",
            wing: "Number & Pattern",
            blurb: "Kaprekar's routine: rearrange digits large minus small. Every mixed 4-digit \
                    number falls to 6174 in at most seven steps. t picks a start; CLICK: FEED.",
            accent: [220, 160, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let start = start_from(t, self.seed, None);
        let chain = kap_chain(start, 8);
        draw(canvas, &chain);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kaprekar fall",
            root: 311.13,
            tempo: 117,
            line: &[0, 6, 7, 11, 14, 11, 7, 6],
            encodes: "large minus small digits cascade to 6174",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: FEED A NUMBER")
    }

    fn status(&self, t: f64) -> Option<String> {
        let start = start_from(t, self.seed, None);
        let chain = kap_chain(start, 8);
        let steps = chain.len().saturating_sub(1);
        let last = *chain.last().unwrap_or(&start);
        Some(format!("n={start:04}  steps={steps}  ->{last}  CLICK"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let start = start_from(t, self.seed, hands.last().copied());
        let chain = kap_chain(start, 8);
        draw(canvas, &chain);
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
        let start = start_from(t, self.seed, hands.last().copied());
        let chain = kap_chain(start, 8);
        let steps = chain.len().saturating_sub(1);
        let last = *chain.last().unwrap_or(&start);
        let ok = last == TARGET;
        Some(format!(
            "FEED {start:04}  steps={steps}  {}",
            if ok { "6174!" } else { "..." }
        ))
    }

    fn reveal(&self) -> &'static str {
        "Kaprekar's constant 6174 is the unique attractor of the four-digit \
         rearrange-and-subtract process (excluding repdigits). Every mixed \
         four-digit number reaches it in at most seven steps: the tidy twin of Collatz."
    }
}

#[cfg(test)]
mod tests {
    use super::{Kaprekar, TARGET, kap_chain, kap_step};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Kaprekar::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("n="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn feed_changes() {
        let r = Kaprekar::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.7,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn reaches_6174() {
        let chain = kap_chain(3524, 8);
        assert_eq!(*chain.last().unwrap(), TARGET);
        assert!(chain.len() <= 8);
        assert_eq!(kap_step(6174), 6174);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        Kaprekar::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(Kaprekar::new().motif().unwrap().line.len() >= 6);
    }
}
