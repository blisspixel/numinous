//! Ricker map: discrete population model with boom and bust.
//!
//! `x' = x exp(r (1 - x))`. Taking logs, `ln x(n+1) - ln x(n) = r (1 - x(n))`,
//! and the sum telescopes: along any orbit that stays bounded and away from 0
//! the long-run mean population is exactly the carrying capacity 1. See
//! `docs/ROOMS.md` and `docs/MATHEMATICS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

/// Generations discarded before anything is drawn, so the picture is the
/// attractor and not the road to it.
const BURN_IN: usize = 300;
/// Cobweb legs drawn, newest brightest.
const LEGS: usize = 16;
/// Generations in the population strip.
const GENERATIONS: usize = 48;
/// The ambient growth rate breathes from here...
const AMBIENT_LOW: f64 = 1.5;
/// ...up by this much and back, once per cycle.
const AMBIENT_SPAN: f64 = 2.0;
/// A hand sets the growth rate from here...
const HAND_LOW: f64 = 0.5;
/// ...across this span.
const HAND_SPAN: f64 = 3.5;
/// The cobweb plot's bottom edge, as a fraction of the height.
const PLOT_FLOOR: f64 = 0.72;
/// The population strip's top edge, as a fraction of the height.
const STRIP_TOP: f64 = 0.80;

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

fn seed_offset(seed: u64) -> f64 {
    if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    }
}

/// The growth rate. Ambient play breathes it up and back down once per cycle,
/// so the period doubling is crossed both ways and never jumps at the wrap.
fn r_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = seed_offset(seed);
    if let Some((x, _)) = hand {
        HAND_LOW + x * HAND_SPAN + s
    } else {
        let breath = (1.0 - (std::f64::consts::TAU * phase_unit(t)).cos()) / 2.0;
        AMBIENT_LOW + breath * AMBIENT_SPAN + s
    }
}

fn ricker(x: f64, r: f64) -> f64 {
    x * (r * (1.0 - x)).exp()
}

/// The hump's peak, `f(1/r) = e^(r-1) / r`: no population ever exceeds it
/// after one generation.
fn peak(r: f64) -> f64 {
    (r - 1.0).exp() / r
}

/// The extent of both axes, fixed for a whole mode of play.
///
/// Five percent above the highest peak any rate in the mode's range can
/// reach. The peak falls and then rises in `r`, so over an interval it is
/// largest at an end. One extent for both axes keeps the drawn diagonal the
/// true line `y = x`, and fixing it per mode means nothing is ever clamped to
/// the frame edge and the axes never breathe with the rate.
fn extent(hand: bool, seed: u64) -> f64 {
    let (low, span) = if hand {
        (HAND_LOW, HAND_SPAN)
    } else {
        (AMBIENT_LOW, AMBIENT_SPAN)
    };
    let low = low + seed_offset(seed);
    1.05 * peak(low).max(peak(low + span))
}

/// The attractor sample: the generations after the burn-in, oldest first.
fn attractor(r: f64, seed: u64) -> Vec<f64> {
    let mut x = if seed == 0 {
        0.3
    } else {
        0.1 + (seed % 20) as f64 * 0.02
    };
    for _ in 0..BURN_IN {
        x = ricker(x, r);
    }
    (0..GENERATIONS)
        .map(|_| {
            x = ricker(x, r);
            x
        })
        .collect()
}

fn draw(canvas: &mut dyn Surface, r: f64, extent: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width < 2 || height < 4 {
        return;
    }
    let (right, bottom) = (width - 1, height - 1);
    let floor = (bottom as f64 * PLOT_FLOOR).round();
    let px = |x: f64| (x / extent * right as f64).round() as i32;
    let py = |y: f64| (floor - y / extent * floor).round() as i32;

    // The hump and the diagonal y = x, on equal axes.
    let mut previous: Option<(i32, i32)> = None;
    for column in 0..=right {
        let x = column as f64 / right as f64 * extent;
        let point = (column as i32, py(ricker(x, r)));
        if let Some(last) = previous {
            canvas.line(last.0, last.1, point.0, point.1, '#');
        }
        previous = Some(point);
    }
    canvas.line(0, floor as i32, right as i32, 0, '.');

    // The last legs of the cobweb, faded by age, newest brightest. Oldest
    // first, so on a character surface the newest mark is the one that stays.
    let history = attractor(r, seed);
    let legs = &history[GENERATIONS - LEGS - 1..];
    for (index, pair) in legs.windows(2).enumerate() {
        let (x, next) = (pair[0], pair[1]);
        let age = LEGS - 1 - index;
        let mark = ['#', '*', '+', '.'][(age * 4 / LEGS).min(3)];
        canvas.line(px(x), py(x), px(x), py(next), mark);
        canvas.line(px(x), py(next), px(next), py(next), mark);
    }

    // The population strip: one bar per generation, on the same scale. The
    // bars' total area is proportional to the mean population, which the
    // telescoping sum pins near 1 at every rate, so the strip's light holds
    // steady while the bars boom and bust.
    let strip_top = (bottom as f64 * STRIP_TOP).round();
    let strip_height = bottom as f64 - strip_top;
    let capacity = (bottom as f64 - strip_height / extent).round() as i32;
    canvas.line(0, capacity, right as i32, capacity, '-');
    for (generation, &x) in history.iter().enumerate() {
        let left = generation * width / GENERATIONS;
        let next = (generation + 1) * width / GENERATIONS;
        let bar = (x / extent * strip_height).round() as i32;
        if bar == 0 {
            continue;
        }
        for column in left..next.saturating_sub(1).max(left + 1) {
            canvas.line(
                column as i32,
                bottom as i32,
                column as i32,
                bottom as i32 - bar + 1,
                '+',
            );
        }
    }
}

/// Ricker map room.
#[derive(Debug, Default)]
pub struct Ricker {
    seed: u64,
}

impl Ricker {
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

impl Room for Ricker {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let r = r_param(t, None, self.seed);
        draw(canvas, r, extent(false, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ricker",
            root: 174.61,
            tempo: 96,
            line: &[0, 5, 12, 5, 0, 7, 12, 0],
            encodes: "growth rate driving boom and crash cycles",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r_param(t, None, self.seed);
        Some(format!("r={r:.2}  pop  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r_param(t, hands.last().copied(), self.seed);
        let seed = self.seed ^ hands.len() as u64;
        draw(canvas, r, extent(!hands.is_empty(), self.seed), seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let r = r_param(t, hands.last().copied(), self.seed);
        // The range the strip actually shows.
        let history = attractor(r, self.seed ^ hands.len() as u64);
        let mn = history.iter().copied().fold(f64::INFINITY, f64::min);
        let mx = history.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let band = if r < 2.0 {
            "stable"
        } else if r < 2.7 {
            "period"
        } else {
            "chaos"
        };
        Some(format!("r={r:.2}  x=[{mn:.2},{mx:.2}]  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "The Ricker map is a classic discrete population model. As r rises, the \
         fixed point loses stability through period doubling into chaos: boom \
         and bust written as a one-dimensional map. However violent the swings, \
         the booms and busts average out to the carrying capacity exactly."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GENERATIONS, Ricker, attractor, extent, peak, r_param, ricker,
    };
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Ricker::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Ricker::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Ricker::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Ricker::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn the_mean_population_is_the_carrying_capacity() {
        // ln x(N) - ln x(0) = r * sum (1 - x(n)), so the mean over N
        // generations differs from 1 by exactly (ln x(0) - ln x(N)) / (r N).
        // Checked as an identity, then as the bound the strip relies on.
        for r in [1.5, 2.2, 2.6, 2.9, 3.3, 3.7, 4.2] {
            let mut x: f64 = 0.3;
            let start = x;
            let mut sum = 0.0;
            let n = 100_000;
            for _ in 0..n {
                sum += x;
                x = ricker(x, r);
            }
            let mean = sum / f64::from(n);
            let predicted = 1.0 + (start.ln() - x.ln()) / (r * f64::from(n));
            assert!((mean - predicted).abs() < 1e-9, "r {r}: {mean} vs {predicted}");
            assert!((mean - 1.0).abs() < 1e-3, "r {r}: mean {mean}");
        }
        // Over the strip's own 48 generations the bound is
        // ln(peak / smallest) / (48 r), a few percent even in deep chaos.
        for r in [2.9, 3.3, 3.7, 4.2] {
            let history = attractor(r, 0);
            let mean = history.iter().sum::<f64>() / GENERATIONS as f64;
            let low = ricker(peak(r), r);
            let bound = (peak(r) / low).ln() / (GENERATIONS as f64 * r) + 1e-12;
            assert!((mean - 1.0).abs() <= bound, "r {r}: mean {mean}, bound {bound}");
        }
    }

    #[test]
    fn no_population_ever_leaves_the_fixed_axes() {
        // The hump's maximum is at x = 1/r, an exact calculus fact checked here
        // by its neighbors, and the axes clear it for every reachable rate.
        for r in [0.5, 1.5, 2.5, 3.5, 4.2] {
            let top = ricker(1.0 / r, r);
            assert!((top - peak(r)).abs() < 1e-12);
            assert!(ricker(1.0 / r - 1e-4, r) < top && ricker(1.0 / r + 1e-4, r) < top);
        }
        for seed in 0..5 {
            for i in 0..=100 {
                let u = f64::from(i) / 100.0;
                let ambient = r_param(u, None, seed);
                let hand = r_param(0.0, Some((u, 0.5)), seed);
                for (r, limit) in [(ambient, extent(false, seed)), (hand, extent(true, seed))] {
                    assert!(peak(r) * 1.05 <= limit + 1e-12, "seed {seed} r {r}");
                    for x in attractor(r, seed) {
                        assert!(x > 0.0 && x < limit, "seed {seed} r {r}: {x}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_strip_holds_its_light_while_the_rate_sweeps_into_chaos() {
        // The strip's lit area tracks the mean population, which the telescoping
        // sum pins near 1, so it stays within a narrow band from the stable
        // regime into deep chaos even though every bar moves.
        let strip_light = |r: f64| {
            let mut raster = Raster::with_accent(240, 140, [40, 160, 80]);
            super::draw(&mut raster, r, extent(false, 0), 0);
            // Below the plot's floor at row 100 only the strip is drawn.
            let rgba = raster.to_rgba();
            crate::photosensitivity::frame_luminance(&rgba[105 * 240 * 4..])
        };
        let readings: Vec<f64> = [1.6, 2.2, 2.6, 2.9, 3.2, 3.5].map(strip_light).to_vec();
        let low = readings.iter().copied().fold(f64::MAX, f64::min);
        let high = readings.iter().copied().fold(f64::MIN, f64::max);
        assert!(high - low < 0.15 * high, "strip light {readings:?}");
    }

    #[test]
    fn ambient_rate_breathes_without_a_jump_at_the_wrap() {
        assert!((r_param(0.0, None, 0) - 1.5).abs() < 1e-12);
        assert!((r_param(0.999_999, None, 0) - 1.5).abs() < 1e-9);
        assert!((r_param(0.5, None, 0) - 3.5).abs() < 1e-12);
    }
}
