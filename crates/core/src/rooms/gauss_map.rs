//! Gauss map: `x -> frac(1/x)`, the engine of continued fractions.
//!
//! Each step reads off one continued-fraction digit, `a = floor(1/x)`, and
//! keeps the remainder, so the orbit of `[0; a1, a2, a3, ...]` is the shift of
//! its digits. A rational seed runs out of digits and its orbit ends at 0; an
//! irrational one never does. See `docs/ROOMS.md` and `docs/MATHEMATICS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

/// The most cobweb legs the picture draws.
const MAX_LEGS: usize = 15;
/// Digits kept past the last drawn leg. Each drawn point is evaluated from at
/// least this many following digits, which pins it to double precision even
/// for the slowest-converging expansion, all ones, where forty digits leave an
/// error near `1 / F(41)^2`, below `1e-16`.
const TAIL: usize = 40;
/// Digits an expansion stores: everything a drawn point can depend on.
const DIGITS: usize = MAX_LEGS + TAIL;

/// Continued-fraction digits of `pi - 3`, OEIS A001203 without its leading 3.
///
/// No pattern is known, so exactly the `DIGITS` an expansion stores are
/// written out; a test checks they evaluate to `PI - 3`.
const PI_DIGITS: [u64; DIGITS] = [
    7, 15, 1, 292, 1, 1, 1, 2, 1, 3, 1, 14, 2, 1, 1, 2, 2, 2, 2, 1, 84, 2, 1, 1, 15, 3, 13, 1, 4,
    2, 6, 6, 99, 1, 2, 2, 6, 3, 5, 1, 1, 6, 8, 1, 7, 1, 2, 3, 7, 1, 2, 1, 1, 12, 1,
];

/// The `i`-th digit (from 1) of `pi - 3`, for `i` up to `DIGITS`.
fn pi_digit(i: usize) -> u64 {
    PI_DIGITS[i - 1]
}

/// The `i`-th digit of `e - 2 = [0; 1, 2, 1, 1, 4, 1, 1, 6, ...]` (Euler).
fn e_digit(i: usize) -> u64 {
    if i % 3 == 2 { 2 * (i as u64 + 1) / 3 } else { 1 }
}

/// The golden ratio's conjugate, `[0; 1, 1, 1, ...]`: a fixed point.
fn golden_digit(_: usize) -> u64 {
    1
}

/// `sqrt 2 - 1 = [0; 2, 2, 2, ...]`: also a fixed point.
fn root_two_digit(_: usize) -> u64 {
    2
}

/// `sqrt 3 - 1 = [0; 1, 2, 1, 2, ...]`: a two-cycle.
fn root_three_digit(i: usize) -> u64 {
    if i % 2 == 1 { 1 } else { 2 }
}

/// `tanh(1/2) = [0; 2, 6, 10, 14, ...]` (Lambert): digits that grow forever.
fn tanh_half_digit(i: usize) -> u64 {
    4 * i as u64 - 2
}

/// The irrational seeds a visit can open on, by variation seed.
///
/// Two transcendental numbers that wander and drift, three quadratic
/// irrationals whose orbits close into loops (Lagrange's theorem: exactly
/// the quadratic irrationals have eventually periodic digits), and one whose
/// digits grow without end.
const VISITS: [fn(usize) -> u64; 6] = [
    pi_digit,
    e_digit,
    golden_digit,
    root_two_digit,
    root_three_digit,
    tanh_half_digit,
];

/// A seed's continued-fraction digits `a1, a2, ...`, as many as can matter.
#[derive(Debug, Clone, PartialEq)]
struct Expansion {
    digits: Vec<u64>,
    /// The digits are all of them: the seed is rational and its orbit ends.
    complete: bool,
}

impl Expansion {
    /// An irrational seed from its digit rule.
    fn irrational(digit: fn(usize) -> u64) -> Self {
        Self {
            digits: (1..=DIGITS).map(digit).collect(),
            complete: false,
        }
    }

    /// The exact expansion of the number the machine holds for `x0`.
    ///
    /// A double is a dyadic rational, `odd / 2^shift`, so Euclid's algorithm
    /// on the two integers gives its digits with no rounding at all, and it
    /// stops. Seeds the room admits are at least 0.02, so the denominator
    /// always fits; anything else has no digits and ends at once.
    fn machine(x0: f64) -> Self {
        let Some((mut p, shift)) = crate::numerics::dyadic_parts(x0) else {
            return Self {
                digits: Vec::new(),
                complete: true,
            };
        };
        let mut q = 1u64 << shift;
        let mut digits = Vec::new();
        while p != 0 && digits.len() < DIGITS {
            digits.push(q / p);
            (q, p) = (p, q % p);
        }
        Self {
            digits,
            complete: p == 0,
        }
    }

    /// The `n`-th point of the orbit, `[0; a(n+1), a(n+2), ...]`.
    ///
    /// Evaluated from the tail backwards, which is the stable direction: no
    /// error is amplified, so every point is as good as the digits behind it.
    /// Past the end of a complete expansion this is the exact 0 the orbit
    /// stops on.
    fn point(&self, n: usize) -> f64 {
        self.digits
            .get(n..)
            .unwrap_or_default()
            .iter()
            .rev()
            .fold(0.0, |tail, &a| 1.0 / (a as f64 + tail))
    }

    /// How many legs exist: one per digit, and no more once a rational runs
    /// out of them.
    fn legs(&self, wanted: usize) -> usize {
        if self.complete {
            wanted.min(self.digits.len())
        } else {
            wanted
        }
    }

    /// Whether drawing `legs` legs reaches the end of a rational seed.
    fn ends_within(&self, legs: usize) -> bool {
        self.complete && legs >= self.digits.len()
    }

    /// `[0; a1, ..., ak]` for the first `count` digits, keeping the newest
    /// ones and eliding the oldest when it would run past `budget` characters.
    ///
    /// A digit of a million or more is written in scientific form. Those come
    /// from a hand seed that sits extraordinarily close to a simple fraction,
    /// and nineteen figures of one would crowd every other digit out.
    fn readout(&self, count: usize, budget: usize) -> String {
        let shown: Vec<String> = self.digits[..count.min(self.digits.len())]
            .iter()
            .map(|&digit| {
                if digit < 1_000_000 {
                    digit.to_string()
                } else {
                    format!("{:.1e}", digit as f64)
                }
            })
            .collect();
        let full = format!("[0; {}]", shown.join(", "));
        if full.len() <= budget {
            return full;
        }
        let elided = |from: usize| format!("[0; ..., {}]", shown[from..].join(", "));
        let mut from = shown.len().saturating_sub(1);
        while from > 0 && elided(from - 1).len() <= budget {
            from -= 1;
        }
        elided(from)
    }
}

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

/// The seed a hand sets, offset by the variation seed.
fn hand_x0(x: f64, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 17) as f64 * 0.01
    };
    (0.05 + x * 0.9 + s).fract().max(0.02)
}

/// The visit's seed and how many legs the phase has revealed.
///
/// Ambient play keeps one irrational seed for the whole visit and lets the
/// phase add one leg at a time, so a frame differs from the last by at most
/// one leg. A hand picks its own seed and sees every leg at once.
fn experiment(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, Expansion, usize) {
    match hand {
        Some((x, _)) => {
            let x0 = hand_x0(x, seed);
            (x0, Expansion::machine(x0), MAX_LEGS)
        }
        None => {
            let expansion = Expansion::irrational(VISITS[(seed % VISITS.len() as u64) as usize]);
            let legs = 1 + (phase_unit(t) * (MAX_LEGS - 1) as f64) as usize;
            (expansion.point(0), expansion, legs)
        }
    }
}

/// The Gauss invariant density, `1 / ((1 + x) ln 2)`.
fn gauss_density(x: f64) -> f64 {
    1.0 / ((1.0 + x) * std::f64::consts::LN_2)
}

fn draw(canvas: &mut dyn Surface, expansion: &Expansion, wanted: usize) {
    let (width, height) = canvas.draw_bounds();
    if width < 2 || height < 2 {
        return;
    }
    let (right, bottom) = (width - 1, height - 1);
    let px = |x: f64| (x.clamp(0.0, 1.0) * right as f64).round() as i32;
    let py = |y: f64| ((1.0 - y.clamp(0.0, 1.0)) * bottom as f64).round() as i32;

    // The Gauss density as a faint guide along the bottom fifth: where a
    // typical orbit spends its time, twice as often near 0 as near 1.
    let band = bottom as f64 * 0.2;
    let guide = |column: usize| {
        let x = column as f64 / right as f64;
        let share = gauss_density(x) / gauss_density(0.0);
        (bottom as f64 - band * share).round() as i32
    };
    for column in 1..=right {
        canvas.line(
            column as i32 - 1,
            guide(column - 1),
            column as i32,
            guide(column),
            '-',
        );
    }

    // The graph, one branch y = 1/x - k per digit k. Columns on different
    // branches are not joined: the map jumps there, and a stroke across the
    // jump would draw points that are not on the graph.
    let mut previous: Option<(u64, i32, i32)> = None;
    for column in 1..=right {
        let x = column as f64 / right as f64;
        let reciprocal = 1.0 / x;
        let branch = reciprocal.floor() as u64;
        let (cx, cy) = (column as i32, py(reciprocal.fract()));
        match previous {
            Some((last, lx, ly)) if last == branch => canvas.line(lx, ly, cx, cy, '#'),
            _ => canvas.plot(cx, cy, '#'),
        }
        previous = Some((branch, cx, cy));
    }
    canvas.line(0, bottom as i32, right as i32, 0, '.');

    // The cobweb: up from the axis to the graph, then across to the diagonal,
    // one leg per digit. The newest leg is the brightest.
    let legs = expansion.legs(wanted);
    for n in 0..legs {
        let (x, next) = (expansion.point(n), expansion.point(n + 1));
        let mark = if n + 1 == legs { '#' } else { '*' };
        let from = if n == 0 { 0.0 } else { x };
        canvas.line(px(x), py(from), px(x), py(next), mark);
        canvas.line(px(x), py(next), px(next), py(next), mark);
    }

    // A rational seed's orbit stops where its last digit lands exactly on the
    // axis, at x = 1 / a_last. Marked so the stop reads as an end, not a pause.
    if expansion.ends_within(legs) && legs > 0 {
        let (ex, ey) = (px(expansion.point(legs - 1)), py(0.0));
        canvas.line(ex - 2, ey, ex + 2, ey, '#');
        canvas.line(ex, ey - 2, ex, ey, '#');
    }
}

/// Gauss map room.
#[derive(Debug, Default)]
pub struct GaussMap {
    seed: u64,
}

impl GaussMap {
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

impl Room for GaussMap {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (_, expansion, legs) = experiment(t, None, self.seed);
        draw(canvas, &expansion, legs);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gauss map",
            root: 277.18,
            tempo: 118,
            line: &[0, 12, 5, 0, 7, 12, 0, 5],
            encodes: "fractional parts of reciprocal iterates",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE SEED")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (x0, expansion, legs) = experiment(t, None, self.seed);
        Some(format!(
            "x0={x0:.3}  {}  DRAG:SEED",
            expansion.readout(legs, 35)
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (_, expansion, legs) = experiment(t, hands.last().copied(), self.seed);
        draw(canvas, &expansion, legs);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x0, expansion, legs) = experiment(t, hands.last().copied(), self.seed);
        let legs = expansion.legs(legs);
        Some(if expansion.ends_within(legs) {
            format!(
                "SEED x0={x0:.3}  {}  ENDS: RATIONAL",
                expansion.readout(legs, 25)
            )
        } else {
            format!("SEED x0={x0:.3}  {}", expansion.readout(legs, 41))
        })
    }

    fn reveal(&self) -> &'static str {
        "The Gauss map x -> frac(1/x) reads off the continued-fraction digits of \
         x, one per step. A rational seed runs out of digits and stops at 0; an \
         irrational one never does, and almost every orbit spends its time by \
         the Gauss density 1/((1+x) ln 2). It is a classical engine of \
         Diophantine approximation and ergodic theory."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DIGITS, Expansion, GaussMap, MAX_LEGS, VISITS, experiment, gauss_density,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    fn hand(x: f64) -> [RoomInput; 1] {
        [RoomInput::PointerDown { x, y: 0.5, t: 0.0 }]
    }

    #[test]
    fn status_invites() {
        for t in [0.0, 0.3, 0.99] {
            for seed in 0..6 {
                let s = GaussMap::new_with(seed).status(t).unwrap();
                assert!(s.contains("DRAG") || s.contains("SEED"));
                assert!(s.chars().count() <= 56, "{s}");
            }
        }
    }

    #[test]
    fn seed_changes() {
        let r = GaussMap::new();
        let o = r.status(0.2).unwrap();
        let a = r.status_input(0.2, &hand(0.9)).unwrap();
        assert_ne!(o, a);
        for x in [0.0, 0.18, 0.37, 0.5, 0.9, 1.0] {
            let s = r.status_input(0.2, &hand(x)).unwrap();
            assert!(s.chars().count() <= 56, "{s}");
        }
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        GaussMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(GaussMap::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn every_visit_seed_is_the_irrational_its_digits_name() {
        // An independent check of each digit rule: the expansion it generates
        // must evaluate to the number computed directly.
        let named = [
            std::f64::consts::PI - 3.0,
            std::f64::consts::E - 2.0,
            (5f64.sqrt() - 1.0) / 2.0,
            2f64.sqrt() - 1.0,
            3f64.sqrt() - 1.0,
            0.5f64.tanh(),
        ];
        for (digit, value) in VISITS.iter().zip(named) {
            let x0 = Expansion::irrational(*digit).point(0);
            assert!((x0 - value).abs() < 1e-15, "{x0} vs {value}");
        }
    }

    #[test]
    fn the_orbit_is_the_digit_shift_and_agrees_with_the_map() {
        // frac(1/x_n) must be x_(n+1), and floor(1/x_n) the next digit, for
        // every drawn leg of every visit seed.
        for digit in VISITS {
            let expansion = Expansion::irrational(digit);
            for n in 0..MAX_LEGS {
                let x = expansion.point(n);
                let reciprocal = 1.0 / x;
                assert_eq!(reciprocal.floor() as u64, expansion.digits[n]);
                let next = expansion.point(n + 1);
                assert!((reciprocal.fract() - next).abs() < 1e-9 * reciprocal, "leg {n}");
            }
        }
        // The golden conjugate and sqrt 2 - 1 are fixed points; sqrt 3 - 1 is
        // a two-cycle. Exact facts about the numbers, not the code.
        let golden = Expansion::irrational(VISITS[2]);
        let root_three = Expansion::irrational(VISITS[4]);
        for n in 0..MAX_LEGS {
            assert!((golden.point(n) - (5f64.sqrt() - 1.0) / 2.0).abs() < 1e-15);
            assert!((root_three.point(n) - root_three.point(n + 2)).abs() < 1e-15);
        }
    }

    #[test]
    fn a_rational_seed_ends_where_euclid_says() {
        // 0.5 = [0; 2] stops after one leg; 0.375 = 3/8 = [0; 2, 1, 2] after
        // three. Both are exact doubles, so Euclid on the stored value is the
        // oracle, worked by hand.
        let half = Expansion::machine(0.5);
        assert_eq!(half, Expansion { digits: vec![2], complete: true });
        assert_eq!(half.point(1), 0.0);
        let three_eighths = Expansion::machine(0.375);
        assert_eq!(three_eighths.digits, vec![2, 1, 2]);
        assert!(three_eighths.complete);
        assert_eq!(three_eighths.point(3), 0.0);
        // Every double is rational, so every hand seed's expansion is finite.
        // 0.37 is stored as a nearby dyadic; its digits begin like 37/100's.
        let near = Expansion::machine(0.37);
        assert!(near.complete && near.digits.len() < DIGITS);
        assert_eq!(&near.digits[..5], &[2, 1, 2, 2, 1]);
        // The room: a hand at the middle lands on 0.5 and says it ended.
        let room = GaussMap::new();
        let status = room.status_input(0.4, &hand(0.5)).unwrap();
        assert!(status.ends_with("ENDS: RATIONAL"), "{status}");
        assert!(status.contains("[0; 2]"), "{status}");
    }

    #[test]
    fn zero_is_an_end_and_never_steps_to_one_half() {
        // The old step sent 0 to 0.5, which invented a 0 <-> 0.5 cycle that
        // the postcard seed 0.4 = 2/5 fell into. Now nothing follows 0.
        let two_fifths = Expansion::machine(0.4);
        assert_eq!(&two_fifths.digits[..2], &[2, 2]);
        for n in 0..two_fifths.digits.len() {
            assert!(two_fifths.point(n) > 0.0);
        }
        assert_eq!(two_fifths.point(two_fifths.digits.len()), 0.0);
        // The ambient postcard is an irrational seed and never ends.
        let (_, postcard, legs) = experiment(GaussMap::new().postcard_t(), None, 0);
        assert!(!postcard.ends_within(legs));
        assert!((0..=legs).all(|n| postcard.point(n) > 0.0));
    }

    #[test]
    fn the_readout_keeps_the_newest_digits_inside_its_budget() {
        let pi = Expansion::irrational(VISITS[0]);
        assert_eq!(pi.readout(4, 35), "[0; 7, 15, 1, 292]");
        let long = pi.readout(MAX_LEGS, 35);
        assert!(long.len() <= 35 && long.starts_with("[0; ..., "), "{long}");
        assert!(long.ends_with(", 1, 1]"), "{long}");
        let giant = Expansion {
            digits: vec![2, 4_611_686_018_427_387_904, 3],
            complete: true,
        };
        assert_eq!(giant.readout(3, 25), "[0; 2, 4.6e18, 3]");
    }

    #[test]
    fn ambient_phase_adds_one_leg_at_a_time_on_one_seed() {
        let (x_start, _, first) = experiment(0.0, None, 0);
        let (x_end, _, last) = experiment(1.0, None, 0);
        assert_eq!((first, last), (1, MAX_LEGS));
        assert_eq!(x_start, x_end, "the seed holds for the whole visit");
        let mut previous = first;
        for i in 1..=1000 {
            let (_, _, legs) = experiment(f64::from(i) / 1000.0, None, 0);
            assert!(legs == previous || legs == previous + 1);
            previous = legs;
        }
    }

    #[test]
    fn the_guide_is_the_exact_invariant_density() {
        // Normalized: the integral over [0, 1] is 1, by midpoint quadrature.
        let n = 100_000;
        let integral: f64 = (0..n)
            .map(|i| gauss_density((f64::from(i) + 0.5) / f64::from(n)))
            .sum::<f64>()
            / f64::from(n);
        assert!((integral - 1.0).abs() < 1e-9, "{integral}");
        // Invariant: the transfer operator of x -> frac(1/x) fixes it,
        // sum over k of rho(1/(k+x)) / (k+x)^2 = rho(x). The terms decay like
        // 1/k^2, so the tail beyond K is bounded by rho(0)/K and is added as
        // the matching integral.
        let k_max = 200_000u32;
        for x in [0.0, 0.1, 0.37, 0.5, 0.9] {
            let sum: f64 = (1..=k_max)
                .map(|k| {
                    let y = f64::from(k) + x;
                    gauss_density(1.0 / y) / (y * y)
                })
                .sum();
            let tail = gauss_density(0.0) / (f64::from(k_max) + x + 0.5);
            assert!((sum + tail - gauss_density(x)).abs() < 1e-9, "x {x}");
        }
    }
}
