//! Prime Spirals (Ulam): order hiding in the most patternless numbers.
//!
//! Write the whole numbers in a square spiral out from the center and light up
//! the primes. The primes, famously unpredictable, snap into unmistakable
//! diagonal streaks. `t` shifts the starting number. See `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// How far `t` shifts the starting number (41 gives Euler's long prime diagonal).
const MAX_START_OFFSET: u64 = 40;
/// Cap on cells walked, so a huge canvas stays bounded (see `.agent` skill S7).
const MAX_CELLS: usize = 200_000;

/// The Prime Spirals room.
#[derive(Debug, Default)]
pub struct PrimeSpirals;

impl PrimeSpirals {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The number at the center of the spiral for phase `t`.
    fn start_for(t: f64) -> u64 {
        1 + (t.clamp(0.0, 1.0) * MAX_START_OFFSET as f64).round() as u64
    }
}

impl Room for PrimeSpirals {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "prime-spirals",
            title: "Prime Spirals",
            wing: "Number & Pattern",
            blurb: "Write the whole numbers in a spiral and light up the primes; the most \
                    patternless numbers we know snap into diagonal streaks. t shifts the start.",
            accent: [190, 70, 170],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let cap = width
            .max(height)
            .saturating_mul(width.max(height))
            .min(MAX_CELLS);
        let mut n = Self::start_for(t);
        let (mut x, mut y) = ((width / 2) as i32, (height / 2) as i32);
        if is_prime(n) {
            canvas.plot(x, y, '*');
        }

        // Ulam spiral: step right, up, left, down, with segment lengths
        // 1, 1, 2, 2, 3, 3, ... walking one cell at a time.
        let dirs = [(1, 0), (0, -1), (-1, 0), (0, 1)];
        let mut dir = 0usize;
        let mut segment = 1u64;
        let mut visited = 1usize;
        'walk: loop {
            for _ in 0..2 {
                for _ in 0..segment {
                    if visited >= cap {
                        break 'walk;
                    }
                    let (dx, dy) = dirs[dir];
                    x += dx;
                    y += dy;
                    n += 1;
                    visited += 1;
                    if is_prime(n) {
                        canvas.plot(x, y, '*');
                    }
                }
                dir = (dir + 1) % 4;
            }
            segment += 1;
        }
    }

    fn reveal(&self) -> &'static str {
        "Primes are famously unpredictable, and a million-dollar prize (the \
         Riemann Hypothesis) rides on how they are spread out. Yet arrange them \
         in a spiral and they line up in diagonal streaks nobody has fully \
         explained. There is a pattern in the most patternless thing we know."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Stanislaw Ulam found this in 1963 by doodling numbers in a grid during a \
             boring conference talk. Some of the best mathematics starts as not \
             paying attention.",
            "Euler's polynomial n squared plus n plus 41 produces primes for every n \
             from 0 to 39, and quadratics like it are exactly why the primes fall \
             into diagonal streaks here. Nobody has fully explained the streaks.",
        ]
    }
}

/// Return whether `n` is prime, by trial division.
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3u64;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{PrimeSpirals, is_prime};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn primality_is_correct() {
        for p in [2, 3, 5, 7, 11, 13, 41, 97, 7919] {
            assert!(is_prime(p), "{p} should be prime");
        }
        for c in [0, 1, 4, 6, 9, 100, 7917] {
            assert!(!is_prime(c), "{c} should not be prime");
        }
    }

    #[test]
    fn start_defaults_to_one() {
        assert_eq!(PrimeSpirals::start_for(0.0), 1);
    }

    #[test]
    fn render_is_deterministic() {
        let room = PrimeSpirals::new();
        let mut a = Canvas::new(41, 25);
        let mut b = Canvas::new(41, 25);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = PrimeSpirals::new();
        let mut canvas = Canvas::new(41, 25);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = PrimeSpirals::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(7, 7);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_hypothesis() {
        assert!(PrimeSpirals::new().reveal().contains("Riemann Hypothesis"));
    }
}
