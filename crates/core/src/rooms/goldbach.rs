//! Goldbach's Comet: an open problem you can watch shimmer.
//!
//! Every even number from 4 up, tested: in how many ways is it the sum of two
//! primes? Plot the counts and a comet appears, dense, banded, climbing. The
//! conjecture, every even number has at least one way, has been checked past
//! four quintillion and proven never. `t` grows the comet. Nobody knows. You
//! could be first. See the Full Map in `docs/ROOMS.md`.

use crate::room::{Room, RoomInput, RoomMeta};
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

/// The actual Goldbach witnesses for `n`, ordered by the smaller prime.
fn prime_pairs(n: u64) -> Vec<(u64, u64)> {
    (2..=n / 2)
        .filter(|&p| is_prime(p) && is_prime(n - p))
        .map(|p| (p, n - p))
        .collect()
}

fn witness_for(n: u64, y: f64) -> Option<(u64, u64)> {
    let pairs = prime_pairs(n);
    if pairs.is_empty() {
        return None;
    }
    let unit = if y.is_finite() {
        y.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let index = (unit * (pairs.len() - 1) as f64).round() as usize;
    pairs.get(index).copied()
}

fn even_at_hand(x: f64) -> u64 {
    4 + ((x.clamp(0.0, 1.0) * (N_MAX - 4) as f64) as u64 / 2 * 2)
}

fn even_x(n: u64, width: usize) -> i32 {
    ((n - 4) as f64 / (N_MAX - 4) as f64 * (width as f64 - 1.0)) as i32
}

fn prime_x(n: u64, width: usize) -> i32 {
    ((n.saturating_sub(2)) as f64 / (N_MAX - 2) as f64 * (width as f64 - 1.0)) as i32
}

fn witness_row(height: usize) -> i32 {
    ((height.saturating_sub(1) as f64) * 0.9) as i32
}

/// Goldbach's Comet.
#[derive(Debug, Default)]
pub struct Goldbach {
    seed: u64,
}

impl Goldbach {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn reach_for(&self, t: f64) -> u64 {
        let base_reach = 4 + ((t.clamp(0.0, 1.0) * (N_MAX - 4) as f64) as u64) / 2 * 2;
        if self.seed == 0 {
            base_reach
        } else {
            base_reach.saturating_add((self.seed % 5) * 2).min(N_MAX)
        }
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
        let reach = self.reach_for(t);
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

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "G major pairs",
            root: 196.0,
            tempo: 92,
            line: &[0, 7, 2, 5, 4, 3, 5, 2, 7, 0],
            encodes: "prime pairs orbiting the same even target",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: TEST THIS EVEN")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // base
        self.render(canvas, t);
        // The hand chooses the even number with x; y chooses one concrete
        // Goldbach witness pair for that number. The comet is the count, and
        // the bottom bracket is the proof pair: p + q = n.
        let y_max = (ways(N_MAX).max(ways(N_MAX - 2)) + 4) as f64;
        for &(px, py) in pokes {
            if !px.is_finite() {
                continue;
            }
            let n = even_at_hand(px);
            let count = ways(n) as f64;
            let x = even_x(n, width);
            let y = ((1.0 - count / y_max) * (height as f64 - 3.0)) as i32 + 1;
            if let Some((p, q)) = witness_for(n, py) {
                let row = witness_row(height);
                let px = prime_x(p, width);
                let qx = prime_x(q, width);
                canvas.line(x, y, x, row, '|');
                canvas.line(px, row, qx, row, '=');
                if px == qx {
                    canvas.plot(px, row, '2');
                } else {
                    canvas.plot(px, row, 'p');
                    canvas.plot(qx, row, 'q');
                }
            }
            canvas.plot(x, y, '+');
            // mark the floor test
            canvas.plot(x, height as i32 - 1, 'o');
        }
    }

    fn status_input(&self, _t: f64, inputs: &[RoomInput]) -> Option<String> {
        let (x, y) = inputs.iter().rev().find_map(|input| match *input {
            RoomInput::PointerDown { x, y, .. } if x.is_finite() => Some((x, y)),
            _ => None,
        })?;
        let n = even_at_hand(x);
        let count = ways(n);
        let (p, q) = witness_for(n, y)?;
        Some(format!("{n} = {p} + {q}   {count} PRIME PAIR(S)"))
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
    use super::{Goldbach, prime_pairs, prime_x, ways, witness_for, witness_row};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    fn text_char_at(text: &str, x: i32, y: i32) -> char {
        text.lines()
            .nth(y as usize)
            .and_then(|line| line.chars().nth(x as usize))
            .unwrap_or(' ')
    }

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
    fn the_witnesses_are_the_actual_prime_pairs() {
        assert_eq!(prime_pairs(10), vec![(3, 7), (5, 5)]);
        assert_eq!(witness_for(10, 0.0), Some((3, 7)));
        assert_eq!(witness_for(10, 1.0), Some((5, 5)));
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
        room.render_poked(&mut canvas, f64::INFINITY, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn reveal_admits_nobody_knows() {
        assert!(Goldbach::new().reveal().contains("open problem"));
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = Goldbach::new_with(0);
        let r_def = Goldbach::new();
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        r0.render(&mut a, 0.5);
        r_def.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(60, 30);
        r0.render_poked(&mut cp, 0.5, &[(0.5, 0.5)]);
        assert_ne!(cp.to_text(), a.to_text());
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let r0 = Goldbach::new_with(0);
        let r42 = Goldbach::new_with(42);
        let mut a = Canvas::new(60, 30);
        let mut c = Canvas::new(60, 30);
        r0.render(&mut a, 0.5);
        r42.render(&mut c, 0.5);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn variation_never_reaches_past_the_room_domain() {
        let room = Goldbach::new_with(99);
        assert_eq!(room.reach_for(1.0), super::N_MAX);
    }

    #[test]
    fn entry_click_tests_an_even_beyond_the_growing_comet() {
        let varied = Goldbach::new_with(42);
        let mut base = Canvas::new(60, 30);
        let mut poked = Canvas::new(60, 30);
        varied.render(&mut base, 0.0);
        varied.render_poked(&mut poked, 0.0, &[(0.5, 0.5)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn entry_click_status_names_the_even_and_its_witness() {
        let room = Goldbach::new();
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.0,
            t: 0.0,
        }];
        let status = room.status_input(0.0, &input).expect("Goldbach witness");

        assert!(status.starts_with("302 = "));
        assert!(status.ends_with("PRIME PAIR(S)"));
        assert_eq!(room.status_input(0.0, &[]), None);
    }

    #[test]
    fn vertical_hand_position_selects_the_witness_pair() {
        let room = Goldbach::new();
        let width = 180;
        let height = 30;
        let witness_row = witness_row(height);
        let x_for_100 = (100.0 - 4.0) / (super::N_MAX - 4) as f64;
        let mut low = Canvas::new(width, height);
        let mut high = Canvas::new(width, height);
        room.render_poked(&mut low, 1.0, &[(x_for_100, 0.0)]);
        room.render_poked(&mut high, 1.0, &[(x_for_100, 1.0)]);
        let low_text = low.to_text();
        let high_text = high.to_text();
        assert_eq!(text_char_at(&low_text, prime_x(3, width), witness_row), 'p');
        assert_eq!(
            text_char_at(&low_text, prime_x(97, width), witness_row),
            'q'
        );
        assert_eq!(
            text_char_at(&high_text, prime_x(47, width), witness_row),
            'p'
        );
        assert_eq!(
            text_char_at(&high_text, prime_x(53, width), witness_row),
            'q'
        );
        assert_ne!(
            low_text, high_text,
            "the hand's y coordinate should choose a different Goldbach proof pair"
        );
    }

    #[test]
    fn same_prime_witnesses_get_a_distinct_marker() {
        let room = Goldbach::new();
        let width = 180;
        let height = 30;
        let x_for_10 = (10.0 - 4.0) / (super::N_MAX - 4) as f64;
        let mut canvas = Canvas::new(width, height);
        room.render_poked(&mut canvas, 1.0, &[(x_for_10, 1.0)]);
        let text = canvas.to_text();
        assert_eq!(
            text_char_at(&text, prime_x(5, width), witness_row(height)),
            '2'
        );
    }
}
