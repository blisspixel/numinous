//! The Sieve of Eratosthenes: primes fall out of a grid of naturals.
//!
//! Cross out multiples; what remains is prime. DRAG: SET THE CEILING.
//! See `docs/ROOMS.md`.

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

fn ceiling(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (30 + (x * 170.0) as usize).clamp(20, 200)
    } else {
        (40 + (phase_unit(t) * 120.0) as usize).clamp(20, 160)
    }
}

fn sieve(n: usize) -> Vec<bool> {
    let n = n.max(2);
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;
    let mut p = 2usize;
    while p * p <= n {
        if is_prime[p] {
            let mut m = p * p;
            while m <= n {
                is_prime[m] = false;
                m += p;
            }
        }
        p += 1;
    }
    is_prime
}

fn draw(canvas: &mut dyn Surface, is_prime: &[bool], strike_upto: usize) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || is_prime.len() < 3 {
        return;
    }
    let n = is_prime.len() - 1;
    let cols = ((n as f64).sqrt().ceil() as usize).max(4);
    let rows = n.div_ceil(cols);
    for (i, &prime) in is_prime.iter().enumerate().take(n + 1).skip(1) {
        let col = (i - 1) % cols;
        let row = (i - 1) / cols;
        let x = ((col as f64 + 0.5) / cols as f64 * width as f64).round() as i32;
        let y = ((row as f64 + 0.5) / rows as f64 * height as f64).round() as i32;
        let ch = if prime {
            '#'
        } else if i <= strike_upto {
            '.'
        } else {
            '+'
        };
        canvas.plot(x, y, ch);
    }
}

/// Sieve of Eratosthenes room.
#[derive(Debug, Default)]
pub struct Sieve {
    seed: u64,
}

impl Sieve {
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

impl Room for Sieve {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sieve",
            title: "The Sieve",
            wing: "Number & Pattern",
            blurb: "Eratosthenes: cross out multiples, primes remain. t and DRAG: SET THE CEILING. \
                    Variation shifts the strike animation seed.",
            accent: [220, 180, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = ceiling(t, None);
        let primes = sieve(n);
        let strike = 2
            + ((phase_unit(t) * (n as f64).sqrt()) as usize)
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 5) as usize
            };
        draw(canvas, &primes, strike);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "eratosthenes",
            root: 261.63,
            tempo: 112,
            line: &[0, 0, 5, 7, 0, 7, 12, 0],
            encodes: "multiples fall away until only primes stand",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE CEILING")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = ceiling(t, None);
        let primes = sieve(n);
        let count = primes.iter().filter(|&&p| p).count();
        Some(format!("N={n}  primes={count}  DRAG:CEIL"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = ceiling(t, hands.last().copied());
        let primes = sieve(n);
        let strike = hands
            .last()
            .map(|&(_, y)| 2 + (y * (n as f64).sqrt()) as usize)
            .unwrap_or(n);
        draw(canvas, &primes, strike);
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
        let n = ceiling(t, hands.last().copied());
        let primes = sieve(n);
        let count = primes.iter().filter(|&&p| p).count();
        let density = count as f64 / n as f64;
        Some(format!("CEIL N={n}  pi~{count}  dens={density:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Eratosthenes' sieve finds every prime up to N by crossing out \
         multiples of each prime p starting at p^2. What remains is the set \
         that has no proper factors: the atoms of multiplication."
    }
}

#[cfg(test)]
mod tests {
    use super::{Sieve, sieve};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Sieve::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("CEIL"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ceil_changes() {
        let r = Sieve::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
    fn primes_under_20() {
        let p = sieve(20);
        assert!(p[2] && p[3] && p[5] && p[7] && p[11] && p[13] && p[17] && p[19]);
        assert!(!p[1] && !p[4] && !p[9] && !p[15]);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Sieve::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Sieve::new().motif().unwrap().line.len() >= 6);
    }
}
