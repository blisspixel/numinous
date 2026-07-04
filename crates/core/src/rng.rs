//! A small, deterministic pseudo-random generator (SplitMix64).
//!
//! Rooms that need randomness (for example the Chaos Game) use this so a render
//! always reproduces exactly from its seed, which is what makes shared
//! configurations and tests deterministic. This is not cryptographic; it is a
//! fast, well-distributed generator for visuals.

/// A [SplitMix64](https://prng.di.unimi.it/splitmix64.c) generator: one `u64` of
/// state, fast, deterministic, and well-distributed for visual use.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Create a generator seeded with `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Return the next 64-bit value and advance the state.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Return the next value uniformly in `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        // Use the top 53 bits: the result is one of the 2^53 evenly spaced
        // multiples of 2^-53 in [0, 1), which is uniform to full double precision.
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Return the next value in `0..n` (with negligible modulo bias for small `n`).
    ///
    /// Returns `0` when `n == 0`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
}

#[cfg(test)]
mod tests {
    use super::SplitMix64;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn next_f64_is_in_unit_interval() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..10_000 {
            let x = rng.next_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn next_f64_mean_is_near_one_half() {
        let mut rng = SplitMix64::new(123_456_789);
        let n = 50_000;
        let mean = (0..n).map(|_| rng.next_f64()).sum::<f64>() / f64::from(n);
        assert!((mean - 0.5).abs() < 0.01, "mean was {mean}");
    }

    #[test]
    fn below_is_bounded_and_handles_zero() {
        let mut rng = SplitMix64::new(9);
        assert_eq!(rng.below(0), 0);
        for _ in 0..1000 {
            assert!(rng.below(3) < 3);
        }
    }
}
