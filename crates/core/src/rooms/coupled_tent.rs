//! Coupled tent maps: two tents with symmetric coupling (toy synchronization).
//!
//! `x' = (1 - eps) T(x) + eps T(y)` and `y' = (1 - eps) T(y) + eps T(x)`, with
//! `T(u) = 2 min(u, 1 - u)`. A small gap across the diagonal is multiplied by
//! `2 (1 - 2 eps)` each step, so the pair locks exactly when `eps > 1/4`.
//! DRAG: TUNE COUPLING. See `docs/ROOMS.md` and `docs/MATHEMATICS.md`.

use crate::numerics::MapOrbit;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

/// Steps discarded before the cloud is drawn, so it shows the attractor
/// rather than the approach to it.
const BURN_IN: usize = 200;
/// Points binned into the density cloud each frame.
const SAMPLES: usize = 4_000;
/// The ambient coupling's peak: high enough to cross the threshold, so every
/// cycle shows both sides of it.
const AMBIENT_PEAK: f64 = 0.35;
/// Density marks from faintest to brightest.
const RAMP: [char; 4] = ['.', '+', '*', '#'];

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

/// The coupling the room draws.
///
/// Ambient coupling breathes from 0 up to [`AMBIENT_PEAK`] and back once per
/// cycle, crossing the threshold twice and never jumping at the wrap. A hand
/// sets it directly. The variation seed offsets both.
fn coupling(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        x * 0.5 + s
    } else {
        let breath = (1.0 - (std::f64::consts::TAU * phase_unit(t)).cos()) / 2.0;
        breath * AMBIENT_PEAK + s
    }
}

fn tent(x: f64) -> f64 {
    2.0 * x.min(1.0 - x)
}

/// One step of the coupled pair. A convex mix of two values in `[0, 1]`, so
/// the unit square maps into itself for every admitted coupling.
fn step([x, y]: [f64; 2], eps: f64) -> [f64; 2] {
    let (tx, ty) = (tent(x), tent(y));
    [(1.0 - eps) * tx + eps * ty, (1.0 - eps) * ty + eps * tx]
}

/// The transverse Lyapunov exponent of the synchronized state,
/// `ln 2 + ln |1 - 2 eps|`.
///
/// Exact rather than estimated: `|T'| = 2` everywhere, so every synchronized
/// orbit multiplies a small gap `x - y` by the same `2 (1 - 2 eps)` each step.
/// Negative means the diagonal attracts. It crosses zero at `eps = 1/4`.
fn transverse_exponent(eps: f64) -> f64 {
    std::f64::consts::LN_2 + (1.0 - 2.0 * eps).abs().ln()
}

/// Whether the pair locks, from the sign of the exact exponent.
fn lock_label(eps: f64) -> &'static str {
    let lambda = transverse_exponent(eps);
    if lambda < 0.0 {
        "LOCK"
    } else if lambda > 0.0 {
        "FREE"
    } else {
        "EDGE"
    }
}

/// The attractor's points: a dithered orbit after its burn-in.
///
/// In plain binary floating point a locked pair is the pure slope-2 tent map,
/// which runs out of binary digits and sits at the origin within about 55
/// steps; the room used to draw that collapse at couplings 0, 0.24, 0.30, and
/// 0.35 and call it synchrony. The shared dithered iterator nudges each
/// coordinate independently, so it can never manufacture a lock: a gap made
/// by the nudges shrinks only when the exponent above is negative.
fn cloud(eps: f64, seed: u64) -> impl Iterator<Item = [f64; 2]> {
    let x0 = if seed == 0 {
        0.2
    } else {
        0.1 + (seed % 30) as f64 * 0.01
    };
    MapOrbit::dithered([x0, 0.7], move |point| step(point, eps), seed)
        .skip(BURN_IN)
        .take(SAMPLES)
}

fn draw(canvas: &mut dyn Surface, eps: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let (right, bottom) = (width.saturating_sub(1), height.saturating_sub(1));
    // The diagonal of perfect lock, as a faint guide under the cloud.
    canvas.line(0, bottom as i32, right as i32, 0, '-');
    // Each occupied cell is plotted once, at a brightness set by its share of
    // the busiest cell. Plotting every point instead would make the total
    // light grow with how tightly the cloud is packed, so the frame would
    // brighten each time the pair locked.
    let mut counts = vec![0u32; width * height];
    for [x, y] in cloud(eps, seed) {
        let column = (x * right as f64).round() as usize;
        let row = ((1.0 - y) * bottom as f64).round() as usize;
        let cell = row.min(bottom) * width + column.min(right);
        counts[cell] = counts[cell].saturating_add(1);
    }
    let busiest = counts.iter().copied().max().unwrap_or(0);
    if busiest == 0 {
        return;
    }
    let top = f64::from(busiest).ln_1p();
    for (cell, &count) in counts.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let level = f64::from(count).ln_1p() / top;
        let mark = RAMP[((level * RAMP.len() as f64) as usize).min(RAMP.len() - 1)];
        canvas.plot((cell % width) as i32, (cell / width) as i32, mark);
    }
}

/// Coupled tent maps room.
#[derive(Debug, Default)]
pub struct CoupledTent {
    seed: u64,
}

impl CoupledTent {
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

impl Room for CoupledTent {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, coupling(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "coupled tent",
            root: 196.0,
            tempo: 112,
            line: &[0, 5, 0, 7, 0, 12, 5, 7],
            encodes: "two expanding maps learning to lock",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE COUPLING")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = coupling(t, None, self.seed);
        let lambda = transverse_exponent(e);
        Some(format!(
            "eps={e:.2}  lam={lambda:+.2}  {}  DRAG:TUNE",
            lock_label(e)
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = coupling(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let e = coupling(t, hands.last().copied(), self.seed);
        let lambda = transverse_exponent(e);
        Some(format!(
            "SET eps={e:.2}  lam_perp={lambda:+.2}  {}",
            lock_label(e)
        ))
    }

    fn reveal(&self) -> &'static str {
        "Coupled chaotic maps can synchronize when the coupling is strong enough. \
         Two tent maps share a diagonal of perfect lock: above a coupling of one \
         quarter every gap between them shrinks and the cloud falls onto it, and \
         below that the gap grows and the pair wanders the square."
    }
}

#[cfg(test)]
mod tests {
    use super::{CoupledTent, cloud, coupling, step, transverse_exponent};
    use crate::canvas::Canvas;
    use crate::numerics::MapOrbit;
    use crate::raster::Raster;
    use crate::room::{Room, RoomInput};

    /// The mean gap below which a "free" pair would look locked: a third of
    /// the uncoupled value, `E|x - y| = 1/3` for independent uniform points.
    const FREE_MEAN_GAP: f64 = 1.0 / 9.0;

    #[test]
    fn status_invites() {
        let s = CoupledTent::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = CoupledTent::new();
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
        CoupledTent::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(CoupledTent::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn the_exponent_crosses_zero_at_one_quarter() {
        assert_eq!(transverse_exponent(0.25), 0.0);
        assert!((transverse_exponent(0.0) - std::f64::consts::LN_2).abs() < 1e-15);
        for eps in [0.0, 0.1, 0.2, 0.24, 0.249] {
            assert!(transverse_exponent(eps) > 0.0, "eps {eps}");
        }
        for eps in [0.251, 0.26, 0.3, 0.35, 0.5, 0.58] {
            assert!(transverse_exponent(eps) < 0.0, "eps {eps}");
        }
        // The status reports the label the exponent earns, at both sides.
        let status_at = |x: f64| {
            CoupledTent::new()
                .status_input(0.0, &[RoomInput::PointerDown { x, y: 0.5, t: 0.0 }])
                .unwrap()
        };
        assert!(status_at(0.48).ends_with("FREE"), "{}", status_at(0.48));
        assert!(status_at(0.5).ends_with("EDGE"), "{}", status_at(0.5));
        assert!(status_at(0.52).ends_with("LOCK"), "{}", status_at(0.52));
    }

    #[test]
    fn a_measured_gap_grows_at_the_rate_the_exponent_predicts() {
        // An independent check of the formula: follow a synchronized orbit and
        // a copy displaced across the diagonal by 1e-6, and read the growth
        // rate off the gap itself rather than off the derivative. Ten steps
        // keep the gap far from the fold and far above rounding.
        const STEPS: i32 = 10;
        for eps in [0.0, 0.1, 0.2, 0.3, 0.4] {
            let mut rates = Vec::new();
            for start in [0.1234, 0.3456, 0.6789, 0.8765] {
                let mut base = [start, start];
                let mut moved = [start + 0.5e-6, start - 0.5e-6];
                let gap = |p: [f64; 2], q: [f64; 2]| (p[0] - p[1]) - (q[0] - q[1]);
                let first = gap(moved, base).abs();
                for _ in 0..STEPS {
                    base = step(base, eps);
                    moved = step(moved, eps);
                }
                rates.push((gap(moved, base).abs() / first).ln() / f64::from(STEPS));
            }
            let mean = rates.iter().sum::<f64>() / rates.len() as f64;
            let predicted = transverse_exponent(eps);
            assert!(
                (mean - predicted).abs() < 1e-3,
                "eps {eps}: measured {mean:.5}, predicted {predicted:.5}"
            );
        }
    }

    #[test]
    fn above_one_quarter_every_start_locks_and_below_it_none_does() {
        // |T(a) - T(b)| <= 2 |a - b| for every pair, so the gap obeys
        // |gap'| <= 2 |1 - 2 eps| |gap|: for 1/4 < eps < 3/4 any start locks.
        // Below 1/4 the dithered cloud must stay off the diagonal on average.
        let gaps = |eps: f64, seed: u64| -> Vec<f64> {
            cloud(eps, seed).map(|[x, y]| (x - y).abs()).collect()
        };
        for seed in [0, 1, 2, 3] {
            for eps in [0.27, 0.3, 0.35, 0.5] {
                let settled = gaps(eps, seed)[1_000..].iter().copied().fold(0.0, f64::max);
                assert!(settled < 1e-11, "eps {eps} seed {seed}: gap {settled:e}");
            }
            let mean = |eps: f64| {
                let all = gaps(eps, seed);
                all.iter().sum::<f64>() / all.len() as f64
            };
            for eps in [0.0, 0.1, 0.15] {
                assert!(mean(eps) > FREE_MEAN_GAP, "eps {eps} seed {seed}: {}", mean(eps));
            }
            // Just below the threshold the gap grows only 4 percent a step and
            // the fold keeps knocking it down, so the cloud hugs the diagonal
            // without settling on it. The old status called this synchrony.
            assert!(mean(0.24) > 0.01, "eps 0.24 seed {seed}: {}", mean(0.24));
        }
    }

    #[test]
    fn the_old_plain_orbit_called_a_collapse_synchrony() {
        // The defect this room used to draw, kept as a regression oracle: the
        // undithered pair falls to the exact origin below the threshold.
        let plain = MapOrbit::exact([0.2, 0.7], |point| step(point, 0.24));
        let last = plain.take(300).last().unwrap();
        assert_eq!(last, [0.0, 0.0]);
        // The dithered cloud at the same coupling is not at the origin.
        assert!(cloud(0.24, 0).all(|[x, y]| x + y > 0.0));
    }

    #[test]
    fn ambient_coupling_breathes_without_a_jump_at_the_wrap() {
        let start = coupling(0.0, None, 0);
        let end = coupling(0.999_999, None, 0);
        assert!(start.abs() < 1e-9 && end.abs() < 1e-9, "{start} {end}");
        assert!((coupling(0.5, None, 0) - super::AMBIENT_PEAK).abs() < 1e-12);
        let crossings = (0..1000)
            .map(|i| coupling(f64::from(i) / 1000.0, None, 0) > 0.25)
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|pair| pair[0] != pair[1])
            .count();
        assert_eq!(crossings, 2, "the cycle crosses the threshold up and down once each");
    }

    #[test]
    fn locking_collapses_the_cloud_onto_the_diagonal_without_a_light_swell() {
        // The picture of the aha is a loss of dimension. Measured on pixels:
        // the locked cloud occupies far fewer cells than the free one, and the
        // whole frame's light stays within a narrow band either way.
        let render = |eps: f64| {
            let mut raster = Raster::with_accent(240, 140, [40, 160, 120]);
            super::draw(&mut raster, eps, 0);
            raster
        };
        let free = render(0.0);
        let locked = render(0.35);
        assert!(free.lit_count() > 5 * locked.lit_count());
        let light = |raster: &Raster| crate::photosensitivity::frame_luminance(&raster.to_rgba());
        let swell = (light(&free) - light(&locked)).abs();
        assert!(
            swell < crate::photosensitivity::GENERAL_FLASH_DELTA / 2.0,
            "locking moved the frame's mean luminance by {swell:.4}"
        );
        // The locked cloud lies on the drawn diagonal.
        let mut canvas = Canvas::new(60, 40);
        super::draw(&mut canvas, 0.35, 0);
        let text = canvas.to_text();
        let rows: Vec<&str> = text.lines().collect();
        for (row, line) in rows.iter().enumerate() {
            for (column, mark) in line.chars().enumerate() {
                if matches!(mark, '.' | '+' | '*' | '#') {
                    let on_diagonal = 39.0 * (1.0 - column as f64 / 59.0);
                    assert!(
                        (row as f64 - on_diagonal).abs() <= 1.0,
                        "a locked point at ({column}, {row}) is off the diagonal"
                    );
                }
            }
        }
    }
}
