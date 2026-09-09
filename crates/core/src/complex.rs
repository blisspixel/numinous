//! A complex value and the functions a Studio field is allowed to ask of it.
//!
//! The core carries no dependencies, so the plane is built here rather than
//! borrowed. Only what a field expression can reach is implemented, and each
//! multivalued function takes its principal branch and says so, because a
//! picture that silently picked a sheet is a picture that lies about where its
//! seam is.
//!
//! Undefined stays undefined. Every operation that has no answer produces a
//! value with a NaN part rather than a plausible substitute, so a sample the
//! mathematics does not define reaches the renderer as a gap instead of as a
//! colour.

use std::f64::consts::PI;

/// A point of the complex plane.
///
/// Kept as two `f64` parts rather than a magnitude and an angle: the parts are
/// what the arithmetic is written in, and converting on every operation would
/// lose the exactness that makes `z*z - 1` land on zero at `z = 1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    /// Real part.
    pub re: f64,
    /// Imaginary part.
    pub im: f64,
}

/// Largest whole exponent that is raised by repeated multiplication.
///
/// Repeated multiplication is exact at the origin and on the negative real
/// axis, where `exp(w * ln z)` is not. Past this the expression is doing
/// something the branch cut is entitled to be part of, and the general form
/// takes over. The bound also caps the work one power can ask for.
const MAX_EXACT_POWER: i32 = 64;

impl std::ops::Neg for Complex {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.re, -self.im)
    }
}

impl std::ops::Add for Complex {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }
}

impl std::ops::Sub for Complex {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }
}

impl std::ops::Mul for Complex {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }
}

/// Quotient, scaled so ordinary values do not overflow on the way.
///
/// Dividing by exactly zero is a pole when the numerator is not zero and
/// undefined when it is. Both are reported rather than smoothed: the pole is a
/// real feature of the field and the renderer marks it.
impl std::ops::Div for Complex {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        if other.re == 0.0 && other.im == 0.0 {
            if self.re == 0.0 && self.im == 0.0 {
                return Self::UNDEFINED;
            }
            return Self::new(f64::INFINITY, f64::INFINITY);
        }
        // Smith's method: divide through by the larger part so the squares
        // that follow cannot overflow for values a direct formula would lose.
        if other.re.abs() >= other.im.abs() {
            let ratio = other.im / other.re;
            let denom = other.re + other.im * ratio;
            Self::new(
                (self.re + self.im * ratio) / denom,
                (self.im - self.re * ratio) / denom,
            )
        } else {
            let ratio = other.re / other.im;
            let denom = other.re * ratio + other.im;
            Self::new(
                (self.re * ratio + self.im) / denom,
                (self.im * ratio - self.re) / denom,
            )
        }
    }
}

impl Complex {
    /// The additive identity.
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };

    /// The multiplicative identity.
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };

    /// The imaginary unit.
    pub const I: Self = Self { re: 0.0, im: 1.0 };

    /// An explicitly undefined value.
    pub const UNDEFINED: Self = Self {
        re: f64::NAN,
        im: f64::NAN,
    };

    /// Build a value from its two parts.
    #[must_use]
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Build a value on the real axis.
    #[must_use]
    pub const fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }

    /// Both parts are finite.
    ///
    /// A value that fails this is a sample the field has no colour for: it is
    /// either undefined or has escaped past what a double can hold.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// Either part is NaN.
    #[must_use]
    pub fn is_nan(self) -> bool {
        self.re.is_nan() || self.im.is_nan()
    }

    /// The value lies exactly on the real axis.
    ///
    /// Exact rather than tolerant on purpose. The functions that consult this
    /// are the ones with no complex meaning at all, and widening the test would
    /// have them answer questions they were not asked.
    #[must_use]
    pub fn is_real(self) -> bool {
        self.im == 0.0
    }

    /// Modulus, computed without overflowing on large parts.
    #[must_use]
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// Squared modulus.
    ///
    /// Cheaper than [`Self::abs`] and enough whenever only an ordering or a
    /// comparison against a squared bound is wanted.
    #[must_use]
    pub fn norm_sqr(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    /// Principal argument in `(-pi, pi]`.
    ///
    /// The origin has no argument; it reports NaN rather than zero, so a
    /// renderer cannot mistake the one point with no phase for the phase zero.
    #[must_use]
    pub fn arg(self) -> f64 {
        if self.re == 0.0 && self.im == 0.0 {
            return f64::NAN;
        }
        self.im.atan2(self.re)
    }

    /// Complex conjugate.
    #[must_use]
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }

    /// Natural exponential.
    #[must_use]
    pub fn exp(self) -> Self {
        let magnitude = self.re.exp();
        Self::new(magnitude * self.im.cos(), magnitude * self.im.sin())
    }

    /// Principal natural logarithm.
    ///
    /// The branch cut runs along the negative real axis, where the imaginary
    /// part jumps by `2*pi`. That seam is drawn rather than hidden: a field
    /// built from a logarithm really does have an edge, and pretending
    /// otherwise would be the one dishonest thing this function could do.
    #[must_use]
    pub fn ln(self) -> Self {
        if self.re == 0.0 && self.im == 0.0 {
            return Self::UNDEFINED;
        }
        Self::new(self.abs().ln(), self.arg())
    }

    /// Principal square root, sharing the logarithm's branch cut.
    #[must_use]
    pub fn sqrt(self) -> Self {
        if self.re == 0.0 && self.im == 0.0 {
            return Self::ZERO;
        }
        if !self.is_finite() {
            return Self::UNDEFINED;
        }
        // Halving the modulus under a square root and the argument by two is
        // the definition; computing it this way keeps the real axis exact.
        let modulus = self.abs();
        let re = ((modulus + self.re) * 0.5).sqrt();
        let im = ((modulus - self.re) * 0.5).sqrt();
        Self::new(re, if self.im < 0.0 { -im } else { im })
    }

    /// Sine.
    #[must_use]
    pub fn sin(self) -> Self {
        Self::new(
            self.re.sin() * self.im.cosh(),
            self.re.cos() * self.im.sinh(),
        )
    }

    /// Cosine.
    #[must_use]
    pub fn cos(self) -> Self {
        Self::new(
            self.re.cos() * self.im.cosh(),
            -(self.re.sin() * self.im.sinh()),
        )
    }

    /// Tangent, as sine over cosine.
    #[must_use]
    pub fn tan(self) -> Self {
        self.sin() / self.cos()
    }

    /// Power.
    ///
    /// A whole exponent within the exact-power bound is repeated multiplication,
    /// which is exact at the origin and along the negative real axis where the
    /// general form would meet the logarithm's seam. Everything else is
    /// `exp(w * ln z)` on the principal branch.
    #[must_use]
    pub fn powc(self, exponent: Self) -> Self {
        if exponent.is_real() {
            let whole = exponent.re;
            if whole.fract() == 0.0 && whole.abs() <= f64::from(MAX_EXACT_POWER) {
                // `as` is exact here: the value is a whole number already
                // bounded well inside i32.
                return self.powi(whole as i32);
            }
        }
        if self.re == 0.0 && self.im == 0.0 {
            if exponent.is_real() && exponent.re > 0.0 {
                return Self::ZERO;
            }
            return Self::UNDEFINED;
        }
        (exponent * self.ln()).exp()
    }

    /// Whole power by repeated multiplication.
    #[must_use]
    pub fn powi(self, exponent: i32) -> Self {
        if exponent == 0 {
            return Self::ONE;
        }
        let mut result = Self::ONE;
        let mut base = self;
        let mut remaining = exponent.unsigned_abs();
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = result * base;
            }
            base = base * base;
            remaining >>= 1;
        }
        if exponent < 0 {
            return Self::ONE / result;
        }
        result
    }

    /// Signed turn from this value's argument to another's, in `(-pi, pi]`.
    ///
    /// The step a walk takes, measured the only way a walk can measure it:
    /// as the shortest turn between two directions. Summing these along a
    /// closed contour is what counts the zeros inside it. Returns NaN when
    /// either value has no argument.
    #[must_use]
    pub fn turn_to(self, other: Self) -> f64 {
        let (from, to) = (self.arg(), other.arg());
        if !from.is_finite() || !to.is_finite() {
            return f64::NAN;
        }
        let mut delta = to - from;
        while delta > PI {
            delta -= 2.0 * PI;
        }
        while delta <= -PI {
            delta += 2.0 * PI;
        }
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::{Complex, MAX_EXACT_POWER};
    use std::f64::consts::PI;

    fn close(left: Complex, right: Complex) -> bool {
        (left.re - right.re).abs() < 1e-12 && (left.im - right.im).abs() < 1e-12
    }

    #[test]
    fn arithmetic_matches_the_definitions() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        assert_eq!(a + b, Complex::new(4.0, 2.0));
        assert_eq!(a - b, Complex::new(2.0, 6.0));
        assert_eq!(a * b, Complex::new(11.0, -2.0));
        assert_eq!(a.abs(), 5.0);
        assert_eq!(a.norm_sqr(), 25.0);
        assert_eq!(a.conj(), Complex::new(3.0, -4.0));
        assert_eq!(-a, Complex::new(-3.0, -4.0));
    }

    #[test]
    fn division_inverts_multiplication() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        assert!(close(a * b / b, a));
        assert!(close(Complex::ONE / Complex::I, -Complex::I));
    }

    #[test]
    fn division_scales_so_large_parts_do_not_overflow() {
        // A direct formula squares the denominator and loses this to infinity.
        let big = Complex::new(1e200, 1e200);
        let quotient = big / big;
        assert!(close(quotient, Complex::ONE));
        let small = Complex::new(1e-200, 1e-200);
        assert!(close(small / small, Complex::ONE));
    }

    #[test]
    fn dividing_by_zero_separates_a_pole_from_an_undefined_sample() {
        let pole = Complex::ONE / Complex::ZERO;
        assert!(!pole.is_finite());
        assert!(!pole.is_nan());
        assert!((Complex::ZERO / Complex::ZERO).is_nan());
    }

    #[test]
    fn the_origin_has_no_argument_rather_than_the_argument_zero() {
        assert!(Complex::ZERO.arg().is_nan());
        assert_eq!(Complex::ONE.arg(), 0.0);
        assert!((Complex::I.arg() - PI / 2.0).abs() < 1e-15);
        assert!((Complex::real(-1.0).arg() - PI).abs() < 1e-15);
    }

    #[test]
    fn exp_and_ln_undo_each_other_off_the_branch_cut() {
        for value in [
            Complex::new(0.5, 0.25),
            Complex::new(-0.5, 2.0),
            Complex::new(3.0, -1.0),
        ] {
            assert!(close(value.ln().exp(), value));
        }
    }

    #[test]
    fn ln_takes_the_principal_branch_and_the_origin_is_undefined() {
        assert!(close(Complex::ONE.ln(), Complex::ZERO));
        let cut = Complex::real(-1.0).ln();
        assert!((cut.im - PI).abs() < 1e-15);
        assert!(Complex::ZERO.ln().is_nan());
    }

    #[test]
    fn eulers_identity_holds() {
        assert!(close(Complex::new(0.0, PI).exp(), Complex::real(-1.0)));
    }

    #[test]
    fn sqrt_is_the_principal_root_and_squares_back() {
        for value in [
            Complex::new(4.0, 0.0),
            Complex::new(-4.0, 0.0),
            Complex::new(3.0, 4.0),
            Complex::new(3.0, -4.0),
        ] {
            let root = value.sqrt();
            assert!(root.re >= 0.0, "principal root has a nonnegative real part");
            assert!(close(root * root, value));
        }
        assert_eq!(Complex::ZERO.sqrt(), Complex::ZERO);
        assert!(close(Complex::real(-1.0).sqrt(), Complex::I));
    }

    #[test]
    fn trigonometry_matches_the_real_axis_and_the_pythagorean_identity() {
        let angle = Complex::real(0.7);
        assert!((angle.sin().re - 0.7_f64.sin()).abs() < 1e-15);
        assert!(angle.sin().is_real());
        for value in [Complex::new(0.3, 0.4), Complex::new(-1.0, 2.0)] {
            let identity = value.sin() * value.sin() + value.cos() * value.cos();
            assert!(close(identity, Complex::ONE));
            assert!(close(value.tan(), value.sin() / value.cos()));
        }
    }

    #[test]
    fn sin_has_its_zeros_exactly_at_the_multiples_of_pi() {
        assert_eq!(Complex::ZERO.sin(), Complex::ZERO);
        assert!(Complex::real(PI).sin().abs() < 1e-15);
    }

    #[test]
    fn a_whole_power_is_exact_where_the_general_form_is_not() {
        // exp(2 * ln 0) is undefined; repeated multiplication is zero.
        assert_eq!(Complex::ZERO.powc(Complex::real(2.0)), Complex::ZERO);
        // On the negative real axis the general form drifts off the axis.
        let squared = Complex::real(-3.0).powc(Complex::real(2.0));
        assert_eq!(squared, Complex::real(9.0));
        assert_eq!(Complex::I.powi(2), Complex::real(-1.0));
        assert_eq!(Complex::I.powi(4), Complex::ONE);
        assert!(close(Complex::real(2.0).powi(-2), Complex::real(0.25)));
        assert_eq!(Complex::new(5.0, 7.0).powi(0), Complex::ONE);
    }

    #[test]
    fn a_power_past_the_exact_bound_uses_the_general_form() {
        let exponent = f64::from(MAX_EXACT_POWER) + 1.0;
        let value = Complex::real(1.0).powc(Complex::real(exponent));
        assert!(close(value, Complex::ONE));
    }

    #[test]
    fn a_fractional_power_is_the_principal_branch() {
        assert!(close(
            Complex::real(4.0).powc(Complex::real(0.5)),
            Complex::real(2.0)
        ));
        assert!(close(
            Complex::real(-1.0).powc(Complex::real(0.5)),
            Complex::I
        ));
    }

    #[test]
    fn zero_to_a_nonpositive_power_is_undefined() {
        assert!(Complex::ZERO.powc(Complex::real(-0.5)).is_nan());
        assert!(Complex::ZERO.powc(Complex::I).is_nan());
        assert_eq!(Complex::ZERO.powc(Complex::real(0.5)), Complex::ZERO);
    }

    #[test]
    fn a_turn_is_the_shortest_one_and_is_refused_at_the_origin() {
        let quarter = Complex::ONE.turn_to(Complex::I);
        assert!((quarter - PI / 2.0).abs() < 1e-15);
        // Crossing the seam turns a little, not almost all the way round.
        let across = Complex::new(-1.0, -0.001).turn_to(Complex::new(-1.0, 0.001));
        assert!(across.abs() < 0.01, "turn across the seam was {across}");
        assert!(Complex::ZERO.turn_to(Complex::ONE).is_nan());
        assert!(Complex::ONE.turn_to(Complex::ZERO).is_nan());
    }

    #[test]
    fn a_half_turn_is_signed_positive_rather_than_negative() {
        let turn = Complex::ONE.turn_to(Complex::real(-1.0));
        assert!((turn - PI).abs() < 1e-15, "half turn was {turn}");
    }

    #[test]
    fn finiteness_and_realness_are_reported_exactly() {
        assert!(Complex::new(1.0, 2.0).is_finite());
        assert!(!Complex::new(f64::INFINITY, 0.0).is_finite());
        assert!(!Complex::UNDEFINED.is_finite());
        assert!(Complex::UNDEFINED.is_nan());
        assert!(Complex::real(2.0).is_real());
        assert!(!Complex::new(2.0, 1e-300).is_real());
    }
}
