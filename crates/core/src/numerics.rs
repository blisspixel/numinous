//! Bounded-step numerical methods shared by the room models.

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
    use super::rk4;

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
