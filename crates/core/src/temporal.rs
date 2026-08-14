//! Exact phase pairs for deterministic temporal comparison.

/// Two explicit normalized phases, ordered from observation to destination.
///
/// A pair says only which two room states to compare. It does not assert an
/// elapsed duration, frame rate, interpolated path, or wall-clock ordering.
/// Equal and decreasing pairs are valid because a caller may need to verify a
/// fixed point or compare across the room's normalized loop boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemporalPair {
    from_t: f64,
    to_t: f64,
}

impl TemporalPair {
    /// Validates two finite phases in `[0.0, 1.0)`.
    #[must_use]
    pub fn new(from_t: f64, to_t: f64) -> Option<Self> {
        (from_t.is_finite()
            && to_t.is_finite()
            && (0.0..1.0).contains(&from_t)
            && (0.0..1.0).contains(&to_t))
        .then_some(Self { from_t, to_t })
    }

    /// The first observation phase.
    #[must_use]
    pub const fn from_t(self) -> f64 {
        self.from_t
    }

    /// The destination phase.
    #[must_use]
    pub const fn to_t(self) -> f64 {
        self.to_t
    }
}

#[cfg(test)]
mod tests {
    use super::TemporalPair;

    #[test]
    fn accepts_forward_equal_and_decreasing_exact_phases() {
        for (from_t, to_t) in [(0.2, 0.35), (0.5, 0.5), (0.95, 0.05)] {
            let pair = TemporalPair::new(from_t, to_t).expect("valid pair");
            assert_eq!(pair.from_t(), from_t);
            assert_eq!(pair.to_t(), to_t);
        }
    }

    #[test]
    fn rejects_nonfinite_and_out_of_range_phases() {
        for (from_t, to_t) in [
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
            (-f64::EPSILON, 0.0),
            (0.0, -f64::EPSILON),
            (1.0, 0.0),
            (0.0, 1.0),
        ] {
            assert!(TemporalPair::new(from_t, to_t).is_none());
        }
    }
}
