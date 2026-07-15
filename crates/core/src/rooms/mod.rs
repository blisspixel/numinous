//! Built-in rooms. Each module implements the [`crate::room::Room`] contract.

/// Shared escape-time budget for the CPU and accelerated Mandelbrot and Julia
/// renderers. Keeping one budget prevents interaction from changing detail
/// merely because a machine has a compatible GPU.
pub const FRACTAL_MAX_ITER: u32 = 160;

pub mod arecibo;
pub mod barnsley_fern;
pub mod buffon_needle;
pub mod cellular_automata;
pub mod chaos_game;
pub mod chladni;
pub mod coffee_cup;
pub mod collatz;
pub mod cult_of_pi;
pub mod double_pendulum;
pub mod epicycles;
pub mod ford_circles;
pub mod galton_board;
pub mod game_of_life;
pub mod goldbach;
pub mod golden_angle;
pub mod harmonograph;
pub mod julia;
pub mod langtons_ant;
pub mod lissajous;
pub mod logistic_map;
pub mod lorenz;
pub mod lsystem;
pub mod mandelbrot;
pub mod mobius;
pub mod prime_spirals;
pub mod quine;
pub mod random_walk;
pub mod ripple;
pub mod sandpile;
pub mod slope_rider;
pub mod strange_loop;
pub mod tetractys;
pub mod the_pour;
pub mod times_tables;
pub mod voronoi;
pub mod zeno;
pub mod zeta_walk;

pub(super) fn variation_unit(seed: u64, salt: u64) -> f64 {
    if seed == 0 {
        0.0
    } else {
        let mut rng = crate::rng::SplitMix64::new(seed ^ salt);
        rng.next_f64()
    }
}

pub(super) fn variation_signed(seed: u64, salt: u64) -> f64 {
    if seed == 0 {
        0.0
    } else {
        variation_unit(seed, salt) * 2.0 - 1.0
    }
}
