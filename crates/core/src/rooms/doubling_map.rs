//! Angle-doubling map on the circle: the Bernoulli shift.
//!
//! theta -> 2 theta mod 1. Expanding chaos, simple symbolics.
//! DRAG: SET THE SEED AND STEPS. See `docs/ROOMS.md` and
//! `docs/MATHEMATICS.md`.

use crate::numerics::MapOrbit;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

/// The fewest steps the room draws.
const MIN_STEPS: usize = 8;
/// The most steps the room draws: past the 53 to 56 binary digits a double
/// in the room's seed range holds, so the end of the machine's digits is on
/// the dial rather than out of reach.
const MAX_STEPS: usize = 64;
/// The graph of the map occupies this top fraction of the height.
const GRAPH_BAND: f64 = 0.45;
/// The digit strip sits between the graph and the circle, at these fractions.
const STRIP: (f64, f64) = (0.48, 0.52);

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

fn steps_from(u: f64, low: usize) -> usize {
    (low + (u * (MAX_STEPS - low) as f64) as usize).clamp(MIN_STEPS, MAX_STEPS)
}

fn params(t: f64, hand: Option<(f64, f64)>) -> (f64, usize) {
    if let Some((x, y)) = hand {
        (x, steps_from(y, MIN_STEPS))
    } else {
        let u = phase_unit(t);
        (0.1 + u * 0.3, steps_from(u, 12))
    }
}

fn start_angle(theta0: f64, seed: u64) -> f64 {
    if seed == 0 {
        theta0
    } else {
        (theta0 + (seed % 20) as f64 * 0.01).fract()
    }
}

fn doubling([theta]: [f64; 1]) -> [f64; 1] {
    [(2.0 * theta).fract()]
}

/// The first steps of the machine's own orbit, and what it read.
///
/// Doubling and taking the fractional part are both exact in binary floating
/// point, so this is the exact orbit of the number the machine holds. That
/// number is `odd / 2^k`: each step reads its next binary digit, and after
/// `k` steps there are none left and the orbit is 0 for good. `k` is 53 for a
/// seed in `[1/2, 1)` and one more for each halving below that.
struct Itinerary {
    /// `theta(0)` through `theta(n - 1)`.
    angles: Vec<f64>,
    /// Binary digit `i + 1` of the seed, which is whether `theta(i) >= 1/2`.
    digits: Vec<bool>,
    /// The step after which the stored digits were exhausted, if it came
    /// within the steps drawn.
    ran_out: Option<usize>,
}

fn itinerary(theta0: f64, steps: usize) -> Itinerary {
    let path: Vec<f64> = std::iter::once(theta0)
        .chain(MapOrbit::exact([theta0], doubling).map(|[theta]| theta))
        .take(steps + 1)
        .collect();
    let angles = path[..steps].to_vec();
    Itinerary {
        digits: angles.iter().map(|&theta| theta >= 0.5).collect(),
        angles,
        ran_out: path.iter().position(|&theta| theta == 0.0),
    }
}

fn draw(canvas: &mut dyn Surface, theta0: f64, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width < 2 || height < 2 {
        return;
    }
    let right = (width - 1) as i32;
    let band = (height - 1) as f64 * GRAPH_BAND;
    let floor = band.round() as i32;
    // Graph of y = 2x mod 1 as two lines, and the diagonal, in the top band.
    let mid = (0.5 * right as f64).round() as i32;
    canvas.line(0, floor, mid, 0, '#');
    canvas.line(mid, floor, right, 0, '#');
    canvas.line(0, floor, right, 0, '.');
    // Orbit on circle at bottom.
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.72;
    let r = height as f64 * 0.18;
    for i in 0..48 {
        let a = std::f64::consts::TAU * i as f64 / 48.0;
        canvas.plot(
            (cx + r * a.cos()).round() as i32,
            (cy + r * a.sin()).round() as i32,
            ':',
        );
    }
    let path = itinerary(start_angle(theta0, seed), n);
    for (i, &theta) in path.angles.iter().enumerate() {
        let a = theta * std::f64::consts::TAU;
        let px = (cx + r * a.cos()).round() as i32;
        let py = (cy + r * a.sin()).round() as i32;
        canvas.plot(px, py, if i + 8 > n { '@' } else { '*' });
        // The same step on the graph: the point (theta, 2 theta mod 1).
        let gx = (theta * right as f64).round() as i32;
        let gy = ((1.0 - (2.0 * theta).fract()) * band).round() as i32;
        canvas.plot(gx, gy, '+');
    }
    // The digits read so far, one cell each on a scale of MAX_STEPS: a one is
    // a tall tick and a zero a short one.
    let (top, base) = (
        ((height - 1) as f64 * STRIP.0).round() as i32,
        ((height - 1) as f64 * STRIP.1).round() as i32,
    );
    let cell = |i: usize| (i * width / MAX_STEPS) as i32;
    for (i, &one) in path.digits.iter().enumerate() {
        let x = cell(i);
        canvas.line(x, if one { top } else { base }, x, base, '*');
    }
    // Where the machine's digits ran out: a bar across the strip at that
    // place, and a ring on the circle around 0, where the orbit now stays.
    if let Some(end) = path.ran_out.filter(|&end| end < n) {
        let x = cell(end);
        canvas.line(x, top - 1, x, base + 1, '#');
        let (zx, zy) = (cx + r, cy);
        for i in 0..12 {
            let a = std::f64::consts::TAU * i as f64 / 12.0;
            canvas.plot(
                (zx + 2.0 * a.cos()).round() as i32,
                (zy + 2.0 * a.sin()).round() as i32,
                '#',
            );
        }
    }
}

/// Doubling map room.
#[derive(Debug, Default)]
pub struct DoublingMap {
    seed: u64,
}

impl DoublingMap {
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

impl Room for DoublingMap {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (th, n) = params(t, None);
        draw(canvas, th, n, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "doubling",
            root: 523.25,
            tempo: 152,
            line: &[0, 0, 0, 7, 7, 7, 12, 12],
            encodes: "binary expansion revealed by doubling",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE SEED AND STEPS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (th, n) = params(t, None);
        let path = itinerary(start_angle(th, self.seed), n);
        Some(match path.ran_out.filter(|&end| end < n) {
            Some(end) => format!("th={th:.2}  n={n}  digits ran out at {end}  DRAG:SEED"),
            None => format!("th={th:.2}  n={n}  DRAG:SEED"),
        })
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (th, n) = params(t, hands.last().copied());
        draw(canvas, th, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (th, n) = params(t, hands.last().copied());
        let path = itinerary(start_angle(th, self.seed ^ hands.len() as u64), n);
        if let Some(end) = path.ran_out.filter(|&end| end < n) {
            return Some(format!("n={n}  binary digits ran out at {end}"));
        }
        // Bernoulli map: Lyapunov is ln 2; bits are the itinerary.
        let lyap = std::f64::consts::LN_2;
        let ones = path.digits.iter().filter(|&&one| one).count();
        let dens = ones as f64 / n.max(1) as f64;
        Some(format!("n={n}  lyap={lyap:.2}  1s={dens:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The angle-doubling map is the Bernoulli shift: each iterate reveals the \
         next binary digit of the starting angle. It is expanding, ergodic, and \
         conjugate to the full shift on two symbols."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "The Tent Map and the Logistic Map at its wildest are this map in \
             other clothes. A change of variable carries each onto the others, \
             so a smooth parabola, a folded line and a shift of binary digits \
             are one system written three ways. Chaos here is not intricacy: it \
             is reading out digits that were always in the starting number.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{DoublingMap, MAX_STEPS, itinerary, params, start_angle};
    use crate::canvas::Canvas;
    use crate::numerics::dyadic_parts;
    use crate::room::{Room, RoomInput};

    fn hand(x: f64, y: f64) -> [RoomInput; 1] {
        [RoomInput::PointerDown { x, y, t: 0.0 }]
    }

    #[test]
    fn status_invites() {
        for t in [0.0, 0.3, 0.9, 1.0] {
            let s = DoublingMap::new().status(t).unwrap();
            assert!(s.contains("DRAG") || s.contains("SEED"));
            assert!(s.chars().count() <= 56, "{s}");
        }
    }

    #[test]
    fn seed_changes() {
        let r = DoublingMap::new();
        let o = r.status(0.2).unwrap();
        let a = r.status_input(0.2, &hand(0.9, 0.8)).unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        DoublingMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(DoublingMap::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn the_dial_reaches_past_the_machines_digits_and_says_so() {
        // The hand's full reach and the end of the ambient sweep both ask for
        // more steps than a double holds digits.
        assert_eq!(params(0.0, Some((0.3, 1.0))).1, MAX_STEPS);
        assert_eq!(params(1.0, None).1, MAX_STEPS);
        for (x, seed) in [(0.3, 0), (0.7, 0), (0.123_456_789, 0), (0.3, 7)] {
            let room = DoublingMap::new_with(seed);
            let inputs = hand(x, 1.0);
            let status = room.status_input(0.0, &inputs).unwrap();
            // The oracle is the stored value's own bits: x is odd / 2^k, and
            // k is where the digits end.
            let theta = start_angle(x, seed ^ 1);
            let (_, k) = dyadic_parts(theta).expect("in range");
            assert!((k as usize) < MAX_STEPS);
            assert_eq!(status, format!("n={MAX_STEPS}  binary digits ran out at {k}"));
        }
        let ambient = DoublingMap::new().status(1.0).unwrap();
        let (_, k) = dyadic_parts(0.4).expect("in range");
        assert!(ambient.contains(&format!("digits ran out at {k}")), "{ambient}");
    }

    #[test]
    fn the_strip_reads_the_seeds_binary_digits() {
        // 0.8125 = 0.1101 in binary: four digits, then nothing.
        let path = itinerary(0.8125, 8);
        assert_eq!(
            path.digits,
            [true, true, false, true, false, false, false, false]
        );
        assert_eq!(path.ran_out, Some(4));
        assert!(path.angles[4..].iter().all(|&theta| theta == 0.0));
        // Within the stored digits nothing is lost early.
        let (odd, k) = dyadic_parts(0.1).expect("in range");
        let path = itinerary(0.1, MAX_STEPS);
        assert_eq!(path.ran_out, Some(k as usize));
        for (i, &one) in path.digits.iter().enumerate().take(k as usize) {
            assert_eq!(one, (odd >> (k as usize - 1 - i)) & 1 == 1, "digit {}", i + 1);
        }
    }
}
