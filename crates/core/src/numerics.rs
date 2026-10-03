//! Bounded-step numerical methods shared by the room models.

use crate::rng::SplitMix64;

/// The largest nudge a dithered [`MapOrbit`] gives a coordinate in one step:
/// `2^-44`, about `5.7e-14`.
///
/// Doubles just below 1 are `2^-53` apart, so a nudge of up to this size
/// spans about a thousand representable values there and refills about nine
/// low binary digits a step, where an exact slope-2 step shifts one out. It is
/// still about ten orders of magnitude finer than a pixel on the largest
/// surface.
pub(crate) const MAP_DITHER: f64 = 1.0 / (1u64 << 44) as f64;

/// The orbit of a map of the unit cube `[0, 1]^N`, one step per item.
///
/// A map whose slopes are powers of two, such as angle doubling or the tent
/// map at `mu = 2`, is computed exactly in binary floating point. Every step
/// shifts one significant binary digit out of a coordinate and none in, so the
/// machine's orbit is the exact orbit of the dyadic rational the seed rounded
/// to. That orbit reaches `0` within about 55 steps and stays there. A picture
/// or a statistic built on it shows a 53-digit number running out of digits,
/// not the behavior of the map.
///
/// [`MapOrbit::exact`] keeps that arithmetic, for a room whose subject is the
/// digits themselves. [`MapOrbit::dithered`] adds a deterministic nudge of at
/// most [`MAP_DITHER`] to each coordinate after every step and reflects it back
/// into the cube. The result is a pseudo-orbit with error at most
/// `MAP_DITHER` per step. For angle doubling, a uniformly expanding circle map
/// with expansion 2, a true orbit lies within `MAP_DITHER / (2 - 1)` of it at
/// every step, and Coven, Kan and Yorke prove the same shadowing property for
/// the tent map at slope 2. So there the picture is the orbit of a real seed
/// next to the one asked for. Elsewhere the claim is statistical, and the room
/// using it tests its density or locking against an exact invariant. See
/// `docs/MATHEMATICS.md`.
pub(crate) struct MapOrbit<const N: usize, F> {
    point: [f64; N],
    map: F,
    dither: Option<SplitMix64>,
}

impl<const N: usize, F: Fn([f64; N]) -> [f64; N]> MapOrbit<N, F> {
    /// The machine's own orbit: each step is `map` and nothing else.
    ///
    /// Exact only when `map` is exact in binary floating point; the caller
    /// owns that claim and its test.
    pub(crate) fn exact(point: [f64; N], map: F) -> Self {
        Self {
            point,
            map,
            dither: None,
        }
    }

    /// A pseudo-orbit nudged by at most [`MAP_DITHER`] per coordinate per
    /// step, replayable from `key`.
    ///
    /// `map` must send the unit cube into itself; the nudge is reflected at
    /// the faces so the orbit never leaves it.
    pub(crate) fn dithered(point: [f64; N], map: F, key: u64) -> Self {
        Self {
            point,
            map,
            dither: Some(SplitMix64::new(key)),
        }
    }
}

impl<const N: usize, F: Fn([f64; N]) -> [f64; N]> Iterator for MapOrbit<N, F> {
    type Item = [f64; N];

    fn next(&mut self) -> Option<[f64; N]> {
        let mapped = (self.map)(self.point);
        self.point = match &mut self.dither {
            None => mapped,
            Some(noise) => mapped.map(|coordinate| {
                reflect_into_unit(coordinate + MAP_DITHER * (2.0 * noise.next_f64() - 1.0))
            }),
        };
        Some(self.point)
    }
}

/// Fold a value that a nudge pushed just past `0` or `1` back inside.
///
/// Reflection rather than clamping, because a clamp would pile every nudge
/// that crossed a face onto the face itself, and for the tent map the face
/// `0` is the repelling fixed point the dither exists to keep orbits off.
fn reflect_into_unit(value: f64) -> f64 {
    let folded = if value < 0.0 {
        -value
    } else if value > 1.0 {
        2.0 - value
    } else {
        value
    };
    folded.clamp(0.0, 1.0)
}

/// A double strictly between 0 and 1 written exactly as `odd / 2^shift`.
///
/// Every double is a dyadic rational, and this is the one the machine actually
/// holds. It is what an exact-arithmetic room has to reason about: the angle
/// doubling map sends it to `0` in exactly `shift` steps, and its continued
/// fraction ends. Returns `None` outside `(0, 1)`, and for the few values
/// below `2^-11` whose `2^shift` would not fit in a `u64`.
pub(crate) fn dyadic_parts(value: f64) -> Option<(u64, u32)> {
    if !(value > 0.0 && value < 1.0) {
        return None;
    }
    let bits = value.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let fraction = bits & ((1u64 << 52) - 1);
    // value = mantissa * 2^-shift exactly, for normal and subnormal doubles.
    // Below 1 the biased exponent is at most 1022, so shift is at least 53.
    let (mantissa, shift) = if biased == 0 {
        (fraction, 1074)
    } else {
        (fraction | (1u64 << 52), 1075 - biased)
    };
    let zeros = mantissa.trailing_zeros();
    u32::try_from(shift - u64::from(zeros))
        .ok()
        .filter(|&shift| shift <= 63)
        .map(|shift| (mantissa >> zeros, shift))
}

/// One classical fourth-order Runge-Kutta step for an autonomous system.
///
/// Callers own the physical model, finite state domain, step size, and horizon.
/// This method does not conserve energy exactly or certify long chaotic paths.
pub(crate) fn rk4<const N: usize>(
    state: [f64; N],
    dt: f64,
    derivative: impl Fn([f64; N]) -> [f64; N],
) -> [f64; N] {
    let shifted =
        |delta: [f64; N], scale: f64| std::array::from_fn(|i| state[i] + delta[i] * scale);
    let k1 = derivative(state);
    let k2 = derivative(shifted(k1, dt * 0.5));
    let k3 = derivative(shifted(k2, dt * 0.5));
    let k4 = derivative(shifted(k3, dt));
    std::array::from_fn(|i| state[i] + dt * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0)
}

#[cfg(test)]
mod tests {
    use super::{MAP_DITHER, MapOrbit, dyadic_parts, rk4};

    fn doubling([x]: [f64; 1]) -> [f64; 1] {
        [(2.0 * x).fract()]
    }

    fn tent([x]: [f64; 1]) -> [f64; 1] {
        [2.0 * x.min(1.0 - x)]
    }

    #[test]
    fn dyadic_parts_match_known_exact_values() {
        assert_eq!(dyadic_parts(0.5), Some((1, 1)));
        assert_eq!(dyadic_parts(0.75), Some((3, 2)));
        // 0.1 is stored as 3602879701896397 / 2^55, the standard example.
        assert_eq!(dyadic_parts(0.1), Some((3_602_879_701_896_397, 55)));
        for outside in [0.0, 1.0, -0.25, 1.5, f64::NAN, f64::INFINITY] {
            assert_eq!(dyadic_parts(outside), None, "{outside}");
        }
        // Reconstructing from the parts gives the double back exactly.
        for value in [0.1, 0.3, 0.7, 1.0 / 3.0, 0.618_033_988_749_895, 0.002] {
            let (odd, shift) = dyadic_parts(value).expect("in range");
            assert_eq!(odd % 2, 1);
            assert_eq!(odd as f64 / (1u64 << shift) as f64, value);
        }
    }

    #[test]
    fn exact_doubling_reads_out_the_stored_digits_then_stops_at_zero() {
        // The integer bits of the stored value are an independent oracle for
        // what the floating-point orbit must do: digit k is bit k after the
        // point, and the orbit is 0 from step `shift` on and never before.
        for seed in [0.1, 0.3, 0.7, 1.0 / 3.0, 0.618_033_988_749_895, 0.5, 0.75] {
            let (odd, shift) = dyadic_parts(seed).expect("in range");
            let mut previous = seed;
            for (step, [x]) in MapOrbit::exact([seed], doubling).take(70).enumerate() {
                let k = step as u32 + 1;
                let digit = u64::from(previous >= 0.5);
                let expected = if k <= shift {
                    (odd >> (shift - k)) & 1
                } else {
                    0
                };
                assert_eq!(digit, expected, "seed {seed}: digit {k}");
                assert_eq!(x == 0.0, k >= shift, "seed {seed}: step {k} gave {x}");
                previous = x;
            }
        }
    }

    #[test]
    fn a_plain_slope_two_orbit_collapses_and_a_dithered_one_does_not() {
        let plain: Vec<[f64; 1]> = MapOrbit::exact([0.3], tent).take(200).collect();
        assert!(plain[60..].iter().all(|&[x]| x == 0.0));
        let dithered: Vec<[f64; 1]> = MapOrbit::dithered([0.3], tent, 9).take(200).collect();
        assert!(dithered.iter().all(|&[x]| x > 0.0 && x <= 1.0));
        let mean = dithered[100..].iter().map(|&[x]| x).sum::<f64>() / 100.0;
        assert!((mean - 0.5).abs() < 0.15, "mean {mean}");
    }

    #[test]
    fn a_dithered_orbit_stays_within_the_propagated_dither_of_the_exact_one() {
        // The tent map is 2-Lipschitz and reflection is 1-Lipschitz, so after k
        // steps a nudge of at most d per step has moved the orbit by at most
        // d (2^k - 1). This is the error bound the shadowing argument starts
        // from, checked against the exact orbit while both are still meaningful.
        let exact = MapOrbit::exact([0.3], tent);
        let dithered = MapOrbit::dithered([0.3], tent, 5);
        for (k, ([a], [b])) in exact.zip(dithered).take(40).enumerate() {
            let bound = MAP_DITHER * (2f64.powi(k as i32 + 1) - 1.0);
            assert!((a - b).abs() <= bound, "step {}: {a} vs {b}", k + 1);
        }
    }

    #[test]
    fn refinement_has_fourth_order_error_against_exact_growth_and_decay() {
        let integrate = |steps| {
            let mut state = [1.0, 0.5];
            for _ in 0..steps {
                state = rk4(state, 1.0 / steps as f64, |[x, y]| [2.0 * x, -3.0 * y]);
            }
            state
        };
        let exact = [2.0_f64.exp(), 0.5 * (-3.0_f64).exp()];
        let coarse = integrate(20);
        let fine = integrate(40);
        for i in 0..2 {
            let coarse_error = (coarse[i] - exact[i]).abs();
            let fine_error = (fine[i] - exact[i]).abs();
            let improvement = coarse_error / fine_error;
            assert!(
                (14.0..18.0).contains(&improvement),
                "component {i}: {improvement}"
            );
            assert!(fine_error < 1e-6);
        }
    }

    #[test]
    fn harmonic_motion_returns_to_its_start_with_small_energy_error() {
        let mut state = [1.0, 0.0];
        for _ in 0..1024 {
            state = rk4(state, std::f64::consts::TAU / 1024.0, |[x, y]| [-y, x]);
        }
        assert!((state[0] - 1.0).hypot(state[1]) < 1e-10);
        assert!((state[0] * state[0] + state[1] * state[1] - 1.0).abs() < 1e-12);
    }
}
