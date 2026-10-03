//! Tent map: the simplest piecewise-linear chaos map on `[0, 1]`.
//!
//! T_mu(x) = mu min(x, 1-x). Orbit cobweb and density.
//! See `docs/ROOMS.md`.

use crate::numerics::MapOrbit;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

const ORBIT: usize = 120;
const DENSITY: usize = 2_000;
const DENSITY_BURN_IN: usize = 40;
/// Keeps the density orbit's dither stream apart from the cobweb's.
const DENSITY_KEY: u64 = 0x7e47_d3a5_17c0_0b1e;

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

fn mu(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let m = if let Some((x, _)) = hand {
        1.0 + x * 1.0 + s
    } else {
        1.5 + phase_unit(t) * 0.5 + s
    };
    m.clamp(0.5, 2.0)
}

fn tent(x: f64, mu_v: f64) -> f64 {
    mu_v * x.min(1.0 - x)
}

/// The orbit the room draws, dithered so it cannot collapse.
///
/// At `mu = 2` every tent step is exact in binary floating point, so a plain
/// orbit loses one binary digit per step and sits at `0` from about step 55
/// on. That put 1,986 of 2,000 density samples at exactly 0 where the true
/// invariant density is uniform. The shared dithered iterator keeps the orbit
/// the shadow of a real one. See `docs/MATHEMATICS.md`.
fn orbit(x0: f64, mu_v: f64, key: u64) -> impl Iterator<Item = f64> {
    MapOrbit::dithered([x0], move |[x]| [tent(x, mu_v)], key).map(|[x]| x)
}

/// The density strip's samples: one long orbit after a burn-in.
fn density_samples(mu_v: f64, key: u64) -> impl Iterator<Item = f64> {
    orbit(0.3, mu_v, key ^ DENSITY_KEY)
        .skip(DENSITY_BURN_IN)
        .take(DENSITY)
}

fn draw(canvas: &mut dyn Surface, mu_v: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Tent graph.
    let mid = (0.5 * width.saturating_sub(1) as f64).round() as i32;
    let peak = (mu_v * 0.5).clamp(0.0, 1.0);
    let y_peak = ((1.0 - peak) * height.saturating_sub(1) as f64).round() as i32;
    let y0 = height.saturating_sub(1) as i32;
    canvas.line(0, y0, mid, y_peak, '#');
    canvas.line(mid, y_peak, width.saturating_sub(1) as i32, y0, '#');
    // Diagonal y=x.
    canvas.line(0, y0, width.saturating_sub(1) as i32, 0, '.');
    // Cobweb orbit.
    let mut x = if seed == 0 {
        0.2
    } else {
        0.1 + (seed % 70) as f64 * 0.01
    };
    let mut prev_px = (x * width.saturating_sub(1) as f64).round() as i32;
    let mut prev_py = y0;
    for (i, y) in orbit(x, mu_v, seed).take(ORBIT).enumerate() {
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(
            prev_px,
            prev_py,
            px,
            prev_py,
            if i % 2 == 0 { '*' } else { '+' },
        );
        canvas.line(px, prev_py, px, py, if i % 2 == 0 { '*' } else { '+' });
        // Move along diagonal toward (y,y) for next vertical.
        let dx = (y * width.saturating_sub(1) as f64).round() as i32;
        let dy = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, py, dx, dy, '.');
        prev_px = dx;
        prev_py = dy;
        x = y;
    }
    // Bottom density strip from a long orbit.
    let mut bins = vec![0u32; width.max(1)];
    for x in density_samples(mu_v, seed) {
        let b = (x.clamp(0.0, 0.999) * width as f64) as usize;
        if b < bins.len() {
            bins[b] = bins[b].saturating_add(1);
        }
    }
    let max_b = bins.iter().copied().max().unwrap_or(1).max(1);
    let yb = height.saturating_sub(2) as i32;
    for (i, &c) in bins.iter().enumerate() {
        if c * 4 > max_b {
            canvas.plot(i as i32, yb, '|');
        } else if c > 0 {
            canvas.plot(i as i32, yb, '.');
        }
    }
}

/// Tent map room.
#[derive(Debug, Default)]
pub struct TentMap {
    seed: u64,
}

impl TentMap {
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

impl Room for TentMap {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, mu(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tent",
            root: 174.61,
            tempo: 120,
            line: &[0, 12, 0, 7, 12, 0, 5, 12],
            encodes: "mu times the min of x and one minus x",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE MU")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mu(t, None, self.seed);
        Some(format!("mu={m:.2}  tent  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = mu(t, hands.last().copied(), self.seed);
        draw(canvas, m, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let m = mu(t, hands.last().copied(), self.seed);
        // Lyapunov of tent: ln(mu) for mu in (1,2]; density support [0, mu/2].
        let lyap = if m > 0.0 { m.ln() } else { 0.0 };
        let peak = (m * 0.5).clamp(0.0, 1.0);
        let chaos = if m > 1.0 { "chaos" } else { "order" };
        Some(format!("mu={m:.2}  L={lyap:.2}  peak={peak:.2}  {chaos}"))
    }

    fn reveal(&self) -> &'static str {
        "The tent map is conjugate to the shift map for mu=2 and is the simplest \
         piecewise-linear engine of chaos. For mu>1 orbits densify; below the \
         critical value they die to a fixed point."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Set mu to two and this fold is the Logistic Map at its wildest, \
             under the substitution x = sin^2(pi y / 2), and both are the \
             Doubling Map in disguise. One is a parabola, one is a corner, one \
             is a shift of binary digits, and they are the same system. The \
             unpredictability is not in the rule, which is as simple as a rule \
             can be. It is in how much of the starting number you never knew.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{DENSITY, TentMap, density_samples, mu, tent};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    /// Kolmogorov-Smirnov distance from the uniform distribution on `[lo, hi]`.
    fn uniform_distance(mut samples: Vec<f64>, lo: f64, hi: f64) -> f64 {
        samples.sort_by(f64::total_cmp);
        let n = samples.len() as f64;
        samples
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                let cdf = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
                (cdf - i as f64 / n).abs().max((cdf - (i + 1) as f64 / n).abs())
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn the_density_strip_at_mu_two_is_uniform() {
        // The ambient sweep ends on mu = 2 and a full drag reaches it, so this
        // is a state players see, not a hypothetical one.
        assert_eq!(mu(1.0, None, 0), 2.0);
        assert_eq!(mu(0.0, Some((1.0, 0.5)), 0), 2.0);
        // At mu = 2 the tent map preserves Lebesgue measure, so the invariant
        // density is uniform on [0, 1]. The plain floating-point orbit put
        // 1,986 of these 2,000 samples at exactly 0.
        for key in [0, 1, 7, 42] {
            let samples: Vec<f64> = density_samples(2.0, key).collect();
            assert_eq!(samples.len(), DENSITY);
            let at_zero = samples.iter().filter(|&&x| x == 0.0).count();
            assert_eq!(at_zero, 0, "key {key}: the orbit collapsed onto 0");
            // The 1 percent critical value for 2,000 independent uniform
            // samples is 1.63 / sqrt(2000), about 0.036.
            let distance = uniform_distance(samples.clone(), 0.0, 1.0);
            assert!(distance < 0.036, "key {key}: KS distance {distance:.4}");
            let mut tenths = [0usize; 10];
            for x in samples {
                tenths[((x * 10.0) as usize).min(9)] += 1;
            }
            for (tenth, &count) in tenths.iter().enumerate() {
                assert!(
                    (150..=250).contains(&count),
                    "key {key}: tenth {tenth} holds {count} of an expected 200"
                );
            }
        }
    }

    #[test]
    fn below_mu_two_the_density_fills_exactly_the_core_interval() {
        // For 1 < mu <= 2 the attractor is [T(T(1/2)), T(1/2)], that is
        // [mu (1 - mu / 2), mu / 2]. An independent consequence of the map, so
        // it checks that dithering neither leaks out of the core nor starves
        // its ends.
        for m in [1.5, 1.7, 1.9, 2.0] {
            let (lo, hi) = (m * (1.0 - m / 2.0), m / 2.0);
            let samples: Vec<f64> = density_samples(m, 3).collect();
            let low = samples.iter().copied().fold(f64::MAX, f64::min);
            let high = samples.iter().copied().fold(f64::MIN, f64::max);
            assert!(low >= lo - 1e-12 && high <= hi + 1e-12, "mu {m}: [{low}, {high}]");
            assert!(low - lo < 0.01 && hi - high < 0.01, "mu {m}: [{low}, {high}]");
        }
    }

    #[test]
    fn status_invites() {
        let s = TentMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = TentMap::new();
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
    fn tent_peak() {
        assert!((tent(0.5, 2.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        TentMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(TentMap::new().motif().unwrap().line.len() >= 6);
    }
}
