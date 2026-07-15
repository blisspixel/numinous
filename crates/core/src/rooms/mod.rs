//! Built-in rooms. Each module implements the [`crate::room::Room`] contract.

/// Shared escape-time budget for the CPU and accelerated Mandelbrot and Julia
/// renderers. Keeping one budget prevents interaction from changing detail
/// merely because a machine has a compatible GPU.
pub const FRACTAL_MAX_ITER: u32 = 160;

pub mod aizawa;
pub mod apollonian;
pub mod arecibo;
pub mod attention;
pub mod audioactive;
pub mod baker;
pub mod barnsley_fern;
pub mod bedhead;
pub mod bifurcation;
pub mod blancmange;
pub mod bogdanov;
pub mod braess;
pub mod brusselator;
pub mod buddhabrot;
pub mod buffon_needle;
pub mod burning_ship;
pub mod busy_beaver;
pub mod calkin_wilf;
pub mod cantor_set;
pub mod cat_map;
pub mod causal_doors;
pub mod cellular_automata;
pub mod cesaro;
pub mod chaos_game;
pub mod chladni;
pub mod chord_game;
pub mod chua;
pub mod circle_map;
pub mod clifford;
pub mod coffee_cup;
pub mod collatz;
pub mod collatz_tree;
pub mod concentration;
pub mod continued_frac;
pub mod coupled_tent;
pub mod cult_of_pi;
pub mod curse_dimension;
pub mod cycloid;
pub mod degree720;
pub mod dejong;
pub mod dla_frost;
pub mod double_pendulum;
pub mod doubling_map;
pub mod dragon_curve;
pub mod duality;
pub mod duffing;
pub mod elliptical_billiard;
pub mod epicycles;
pub mod farey;
pub mod fastest_fall;
pub mod feigenbaum;
pub mod fibonacci_word;
pub mod first_rain;
pub mod ford_circles;
pub mod fourier_square;
pub mod fourteen_beacons;
pub mod function_painter;
pub mod galton_board;
pub mod game_of_life;
pub mod gauss_map;
pub mod gingerbread;
pub mod goldbach;
pub mod golden_angle;
pub mod gosper;
pub mod gradient_valley;
pub mod gray_scott;
pub mod gumowski_mira;
pub mod h_tree;
pub mod halvorsen;
pub mod harmonics;
pub mod harmonograph;
pub mod henon;
pub mod henon_heiles;
pub mod hilbert;
pub mod hilbert_hotel;
pub mod hopalong;
pub mod hopf;
pub mod horseshoe;
pub mod ikeda;
pub mod inversion;
pub mod ising;
pub mod josephus;
pub mod julia;
pub mod kaplan_yorke;
pub mod kaprekar;
pub mod kepler_loom;
pub mod koch;
pub mod koch_snowflake;
pub mod kuramoto;
pub mod lambda_map;
pub mod landauer;
pub mod langtons_ant;
pub mod laplace_clock;
pub mod learning_clock;
pub mod levy_c;
pub mod lissajous;
pub mod logistic_cobweb;
pub mod logistic_map;
pub mod logistic_orbit;
pub mod loneliness;
pub mod lorenz;
pub mod lotka_volterra;
pub mod lozi;
pub mod lsystem;
pub mod lyapunov;
pub mod magnet;
pub mod mandelbrot;
pub mod mandelbulb_slice;
pub mod manneville;
pub mod menagerie;
pub mod menger;
pub mod menger_slice;
pub mod message_heals;
pub mod minkowski;
pub mod mirror_forms;
pub mod mobius;
pub mod morley;
pub mod multibrot;
pub mod murmuration;
pub mod newton;
pub mod newton_basins_cubic;
pub mod nontransitive;
pub mod nova;
pub mod parrondo;
pub mod pascal_mod;
pub mod peano_curve;
pub mod penrose;
pub mod percolation;
pub mod phantom_jam;
pub mod phoenix;
pub mod pickover;
pub mod poincare_disc;
pub mod prime_gaps;
pub mod prime_spirals;
pub mod pursuit;
pub mod pythagoras_tree;
pub mod quadratic_map;
pub mod quine;
pub mod rabinovich_fabrikant;
pub mod random_walk;
pub mod recaman;
pub mod ricker;
pub mod ripple;
pub mod rose;
pub mod rossler;
pub mod rules30;
pub mod sandpile;
pub mod sierpinski_arrowhead;
pub mod sierpinski_carpet;
pub mod sierpinski_tri;
pub mod sieve;
pub mod sinai_billiard;
pub mod slingshot;
pub mod slope_rider;
pub mod soap_film;
pub mod soft_proof;
pub mod sphere_eversion;
pub mod standard_map;
pub mod starbow;
pub mod steiner;
pub mod stern_brocot;
pub mod strange_loop;
pub mod svensson;
pub mod tent_map;
pub mod tetractys;
pub mod the_lens;
pub mod the_magnet;
pub mod the_pour;
pub mod the_stretch;
pub mod thomas;
pub mod three_gap;
pub mod three_scroll;
pub mod thue_morse;
pub mod tilt_cone;
pub mod times_tables;
pub mod tinkerbell;
pub mod tricorn;
pub mod truchet;
pub mod ulam_spiral;
pub mod uncertainty;
pub mod unlit_room;
pub mod upside_ruler;
pub mod van_der_pol;
pub mod vicsek;
pub mod voronoi;
pub mod weierstrass;
pub mod wet_oracle;
pub mod whispering_table;
pub mod wireworld;
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
