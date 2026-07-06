//! Goldbach's Comet: an open problem you can watch shimmer.
//!
//! Every even number from 4 up, tested: in how many ways is it the sum of two
//! primes? Plot the counts and a comet appears, dense, banded, climbing. The
//! conjecture, every even number has at least one way, has been checked past
//! four quintillion and proven never. `t` grows the comet. Nobody knows. You
//! could be first. See the Full Map in `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// The largest even number the comet reaches.
const N_MAX: u64 = 600;

/// Primality by trial division; small numbers, honest method.
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

/// The Goldbach count: ways to write even `n` as p + q with p <= q, both prime.
fn ways(n: u64) -> u64 {
    (2..=n / 2)
        .filter(|&p| is_prime(p) && is_prime(n - p))
        .count() as u64
}

/// Goldbach's Comet.
#[derive(Debug, Default)]
pub struct Goldbach;

impl Goldbach {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for Goldbach {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "goldbach",
            title: "Goldbach's Comet",
            wing: "Open Problems",
            blurb: "Every even number, tested: how many ways is it two primes? The counts plot \
                    into a comet. That it never touches zero is unproven. Nobody knows. Go on.",
            accent: [255, 220, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let reach = 4 + ((t.clamp(0.0, 1.0) * (N_MAX - 4) as f64) as u64) / 2 * 2;
        let y_max = (ways(N_MAX).max(ways(N_MAX - 2)) + 4) as f64;
        let mut n = 4;
        while n <= reach {
            let count = ways(n) as f64;
            let px = ((n - 4) as f64 / (N_MAX - 4) as f64 * (width as f64 - 1.0)) as i32;
            let py = ((1.0 - count / y_max) * (height as f64 - 3.0)) as i32 + 1;
            canvas.plot(px, py, '*');
            // The floor it must never touch: marked faintly along the bottom.
            if n % 12 == 0 {
                canvas.plot(px, height as i32 - 1, '-');
            }
            n += 2;
        }
    }

    fn reveal(&self) -> &'static str {
        "Goldbach wrote to Euler in 1742: every even number past two seems to \
         be the sum of two primes. Every point in this comet is one even number \
         and its count of ways. The conjecture only needs the comet to never \
         touch the floor, and it has been checked past four quintillion without \
         a single miss. Proven: never. This is an open problem; you are looking \
         at the actual frontier of human knowledge, and it shimmers."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "The comet's bands are real structure: even numbers divisible by \
             three have systematically more representations, because their prime \
             pairs dodge fewer collisions. The Hardy-Littlewood circle method \
             predicts the bands' exact heights, still without proving a single \
             even number must have any pair at all.",
            "The best result stands since 1973: Chen Jingrun proved every large \
             even number is a prime plus a number with at most two prime \
             factors. One factor short, for fifty years and counting. That is \
             how hard the last step of an easy question can be.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Goldbach, ways};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_conjecture_holds_as_far_as_the_room_can_see() {
        let mut n = 4;
        while n <= super::N_MAX {
            assert!(ways(n) >= 1, "Goldbach fails at {n}?! Publish immediately.");
            n += 2;
        }
    }

    #[test]
    fn the_counts_are_right_where_hand_checking_is_easy() {
        assert_eq!(ways(4), 1, "2+2");
        assert_eq!(ways(10), 2, "3+7 and 5+5");
        assert_eq!(ways(12), 1, "5+7");
    }

    #[test]
    fn render_is_deterministic_and_grows() {
        let room = Goldbach::new();
        let mut early = Canvas::new(60, 30);
        let mut late = Canvas::new(60, 30);
        room.render(&mut early, 0.2);
        room.render(&mut late, 1.0);
        assert!(late.ink_count() > early.ink_count());
        let mut again = Canvas::new(60, 30);
        room.render(&mut again, 1.0);
        assert_eq!(late.to_text(), again.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Goldbach::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_admits_nobody_knows() {
        assert!(Goldbach::new().reveal().contains("open problem"));
    }
}
