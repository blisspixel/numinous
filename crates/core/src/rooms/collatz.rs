//! Collatz: a five-year-old's rule no one can crack.
//!
//! Pick a number. If it is even, halve it; if it is odd, triple it and add one.
//! Repeat. Every number ever tested falls to 1, yet no one can prove they all
//! do. This room plots the (log-scaled) trajectory of a starting number as it
//! falls. `t` picks the number. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// The starting number at `t = 0` (27 is famous for its long, wild orbit).
const START_MIN: u64 = 27;
/// How far `t` sweeps the starting number.
const START_SWEEP: u64 = 100;
/// Safety cap on orbit length, so the loop is always bounded even if a value
/// saturates (Collatz is unproven, so we never assume termination).
const MAX_STEPS: usize = 1000;

/// The Collatz room.
#[derive(Debug, Default)]
pub struct Collatz;

impl Collatz {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The starting number for phase `t`.
    fn start_for(t: f64) -> u64 {
        START_MIN + (t.clamp(0.0, 1.0) * START_SWEEP as f64).round() as u64
    }
}

impl Room for Collatz {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "collatz",
            title: "Collatz",
            wing: "Emergence",
            blurb: "Halve it if even, triple it and add one if odd, and repeat; every number \
                    always crashes to 1, eventually. t picks the number; watch its wild journey.",
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let orbit = collatz_orbit(Self::start_for(t));
        if orbit.len() < 2 {
            return;
        }
        let max = orbit.iter().copied().max().unwrap_or(1);
        let log_max = (max as f64).log2().max(1e-9);
        let (fw, fh) = (width as f64, height as f64);
        let last = orbit.len() - 1;

        let to_pixel = |i: usize, value: u64| -> (i32, i32) {
            let x = (i as f64 / last as f64) * (fw - 1.0);
            let normalized = (value as f64).log2() / log_max;
            let y = (fh - 1.0) - normalized * (fh - 1.0);
            (x.round() as i32, y.round() as i32)
        };

        let mut prev = to_pixel(0, orbit[0]);
        for (i, &value) in orbit.iter().enumerate().skip(1) {
            let current = to_pixel(i, value);
            canvas.line(prev.0, prev.1, current.0, current.1, '*');
            prev = current;
        }
    }

    fn reveal(&self) -> &'static str {
        "Every number ever tested falls to 1. Nobody on Earth can prove they all \
         do. It looks like a rule a child could follow, and it has defeated every \
         mathematician for 90 years. You are playing with an open mystery."
    }
}

/// The Collatz sequence from `start` down to 1 (bounded by `MAX_STEPS`).
///
/// Uses saturating arithmetic so an extreme start cannot overflow; the safety
/// cap guarantees termination regardless.
fn collatz_orbit(start: u64) -> Vec<u64> {
    let mut n = start.max(1);
    let mut sequence = vec![n];
    while n != 1 && sequence.len() <= MAX_STEPS {
        n = if n % 2 == 0 {
            n / 2
        } else {
            n.saturating_mul(3).saturating_add(1)
        };
        sequence.push(n);
    }
    sequence
}

#[cfg(test)]
mod tests {
    use super::{Collatz, collatz_orbit};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn small_orbit_is_correct() {
        assert_eq!(collatz_orbit(6), vec![6, 3, 10, 5, 16, 8, 4, 2, 1]);
        assert_eq!(collatz_orbit(1), vec![1]);
    }

    #[test]
    fn famous_orbit_peaks_at_9232_and_reaches_one() {
        let orbit = collatz_orbit(27);
        assert_eq!(orbit.iter().copied().max(), Some(9232));
        assert_eq!(orbit.last(), Some(&1));
    }

    #[test]
    fn start_defaults_to_27() {
        assert_eq!(Collatz::start_for(0.0), 27);
    }

    #[test]
    fn render_is_deterministic() {
        let room = Collatz::new();
        let mut a = Canvas::new(60, 20);
        let mut b = Canvas::new(60, 20);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = Collatz::new();
        let mut canvas = Canvas::new(60, 20);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = Collatz::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(5, 5);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_mystery() {
        assert!(Collatz::new().reveal().contains("prove they all"));
    }
}
