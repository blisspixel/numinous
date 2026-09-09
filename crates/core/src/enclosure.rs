//! A set of values guaranteed to contain the true one.
//!
//! Sampling a function at points cannot say what it does between them, and for
//! a curve drawn from `f(x, y) = 0` that gap is not a detail. Tupper proved in
//! 2001 that the obvious two-colour picture, this cell has curve in it and that
//! one does not, is **not computable**: no algorithm decides it for an
//! arbitrary relation with any amount of time and memory. A plotter that only
//! answers yes and no is therefore not a simpler honest plotter, it is one that
//! cannot be honest. The way out is a third answer, and the third answer needs
//! arithmetic over sets rather than over points.
//!
//! What point sampling gets wrong is not theoretical. `(x^2+y^2-1)^2 = 0` is a
//! circle, and a sign test draws nothing at any resolution, because the value
//! touches zero without crossing it. `y = tan(x)` grows a curve at every
//! asymptote, where there is none. This module is what those two failures cost.
//!
//! Every operation here returns a set that **contains** the true value, never
//! one that approximates it. Where that cannot be promised, the result says so
//! by carrying [`Certainty::Doubtful`] rather than by narrowing anyway.
//!
//! # What the soundness rests on
//!
//! Results are computed in the ordinary rounding mode and then widened, because
//! Rust offers no portable way to change the mode. Widening by one unit in the
//! last place is enough for the operations IEEE 754 requires to be correctly
//! rounded: addition, subtraction, multiplication, division, and the square
//! root, each of which lands within half a unit of the true answer. The
//! standard only *recommends* correct rounding for `exp`, `ln`, and the
//! trigonometric functions, and platform libraries differ, so those widen by
//! four units in the last place instead. That is a declared assumption about
//! the host library, not a proof, and it is the one place this module trusts
//! something it did not compute.

/// Units in the last place added on each side of an elementary function.
///
/// IEEE 754-2019 requires correct rounding for the four operations and the
/// square root, and only recommends it for `exp`, `ln`, and the trigonometric
/// functions. Real libraries land within an ulp or two; four is chosen to sit
/// clear of that without widening a picture into uselessness. A host library
/// worse than this would make the enclosures wrong rather than merely loose,
/// which is why the number is written down here instead of assumed.
const ELEMENTARY_ULPS: u32 = 4;

/// Whether an enclosure may be trusted to bound a defined, continuous function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Certainty {
    /// The function is defined and continuous everywhere in the input box, and
    /// the bounds contain every value it takes there.
    Sound,
    /// Something in the box is undefined, discontinuous, or past what this
    /// arithmetic can bound. The bounds still contain every defined value, but
    /// nothing may be concluded from their missing zero.
    Doubtful,
}

impl Certainty {
    /// Sound only when both halves are.
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Sound, Self::Sound) => Self::Sound,
            _ => Self::Doubtful,
        }
    }
}

/// A closed set of real values, with what may be concluded from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Enclosure {
    lo: f64,
    hi: f64,
    certainty: Certainty,
}

fn step_down(value: f64, steps: u32) -> f64 {
    let mut value = value;
    for _ in 0..steps {
        value = value.next_down();
    }
    value
}

fn step_up(value: f64, steps: u32) -> f64 {
    let mut value = value;
    for _ in 0..steps {
        value = value.next_up();
    }
    value
}

impl Enclosure {
    /// The whole line, which contains everything and concludes nothing.
    pub const WHOLE: Self = Self {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
        certainty: Certainty::Doubtful,
    };

    /// A single exact value.
    #[must_use]
    pub fn point(value: f64) -> Self {
        if value.is_nan() {
            return Self::WHOLE;
        }
        Self {
            lo: value,
            hi: value,
            certainty: Certainty::Sound,
        }
    }

    /// A closed span. Reversed or nonfinite ends give the whole line.
    #[must_use]
    pub fn span(lo: f64, hi: f64) -> Self {
        if lo.is_nan() || hi.is_nan() || lo > hi {
            return Self::WHOLE;
        }
        Self {
            lo,
            hi,
            certainty: Certainty::Sound,
        }
    }

    /// Lower bound.
    #[must_use]
    pub const fn lo(self) -> f64 {
        self.lo
    }

    /// Upper bound.
    #[must_use]
    pub const fn hi(self) -> f64 {
        self.hi
    }

    /// What may be concluded from the bounds.
    #[must_use]
    pub const fn certainty(self) -> Certainty {
        self.certainty
    }

    /// The same set, with nothing concluded from it.
    #[must_use]
    pub const fn doubted(mut self) -> Self {
        self.certainty = Certainty::Doubtful;
        self
    }

    /// Zero lies inside the bounds.
    #[must_use]
    pub fn holds_zero(self) -> bool {
        self.lo <= 0.0 && self.hi >= 0.0
    }

    /// The bounds are entirely above zero.
    #[must_use]
    pub fn is_positive(self) -> bool {
        self.lo > 0.0
    }

    /// The bounds are entirely below zero.
    #[must_use]
    pub fn is_negative(self) -> bool {
        self.hi < 0.0
    }

    /// Widest distance between two values in the set.
    #[must_use]
    pub fn width(self) -> f64 {
        self.hi - self.lo
    }

    fn rounded(lo: f64, hi: f64, certainty: Certainty, ulps: u32) -> Self {
        if lo.is_nan() || hi.is_nan() {
            return Self::WHOLE;
        }
        Self {
            lo: step_down(lo, ulps),
            hi: step_up(hi, ulps),
            certainty,
        }
    }

    fn exact(lo: f64, hi: f64, certainty: Certainty) -> Self {
        Self::rounded(lo, hi, certainty, 1)
    }

    fn elementary(lo: f64, hi: f64, certainty: Certainty) -> Self {
        Self::rounded(lo, hi, certainty, ELEMENTARY_ULPS)
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(self) -> Self {
        if self.lo >= 0.0 {
            return self;
        }
        if self.hi <= 0.0 {
            return -self;
        }
        Self {
            lo: 0.0,
            hi: (-self.lo).max(self.hi),
            certainty: self.certainty,
        }
    }

    /// Whole power.
    ///
    /// An even power over a span that straddles zero has its least value at
    /// zero rather than at an end, which is the case a corner-only rule gets
    /// wrong.
    #[must_use]
    pub fn powi(self, exponent: i32) -> Self {
        if exponent == 0 {
            return Self::point(1.0);
        }
        if exponent < 0 {
            return Self::point(1.0) / self.powi(-exponent);
        }
        let (a, b) = (self.lo.powi(exponent), self.hi.powi(exponent));
        if exponent % 2 == 1 {
            return Self::exact(a, b, self.certainty);
        }
        if self.holds_zero() {
            return Self::exact(0.0, a.max(b), self.certainty);
        }
        Self::exact(a.min(b), a.max(b), self.certainty)
    }

    /// Natural exponential, which is increasing everywhere.
    #[must_use]
    pub fn exp(self) -> Self {
        Self::elementary(self.lo.exp(), self.hi.exp(), self.certainty)
    }

    /// Natural logarithm, which the nonpositive part of the line has no real
    /// value for.
    #[must_use]
    pub fn ln(self) -> Self {
        if self.lo <= 0.0 {
            return Self::WHOLE;
        }
        Self::elementary(self.lo.ln(), self.hi.ln(), self.certainty)
    }

    /// Square root, which the negative part of the line has no real value for.
    #[must_use]
    pub fn sqrt(self) -> Self {
        if self.lo < 0.0 {
            return Self::WHOLE;
        }
        Self::exact(self.lo.sqrt(), self.hi.sqrt(), self.certainty)
    }

    /// Greatest integer at or below each value.
    ///
    /// A span crossing an integer crosses a jump, so the bounds still contain
    /// every value while nothing may be concluded from them.
    #[must_use]
    pub fn floor(self) -> Self {
        let (lo, hi) = (self.lo.floor(), self.hi.floor());
        let certainty = if lo == hi {
            self.certainty
        } else {
            Certainty::Doubtful
        };
        Self { lo, hi, certainty }
    }

    /// Least of two sets, taken value by value.
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.min(other.hi),
            certainty: self.certainty.and(other.certainty),
        }
    }

    /// Greatest of two sets, taken value by value.
    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self {
            lo: self.lo.max(other.lo),
            hi: self.hi.max(other.hi),
            certainty: self.certainty.and(other.certainty),
        }
    }

    /// Euclidean remainder.
    ///
    /// The result always lies in `[0, |divisor|)`, which bounds it without any
    /// analysis. It is continuous only where the quotient's whole part does not
    /// change, so anything else is reported as doubtful rather than narrowed.
    #[must_use]
    pub fn rem_euclid(self, divisor: Self) -> Self {
        if divisor.holds_zero() {
            return Self::WHOLE;
        }
        let bound = divisor.abs().hi;
        let quotient = (self / divisor).floor();
        if quotient.certainty == Certainty::Sound && quotient.lo == quotient.hi {
            // One whole quotient over the whole box, so the remainder is the
            // dividend shifted by a constant and the exact span survives.
            let exact = self - divisor * Self::point(quotient.lo);
            return Self {
                lo: exact.lo.max(0.0),
                hi: exact.hi.min(bound),
                certainty: self.certainty.and(divisor.certainty),
            };
        }
        Self {
            lo: 0.0,
            hi: bound,
            certainty: Certainty::Doubtful,
        }
    }

    /// Sine.
    ///
    /// A span wider than a full turn takes every value, and so does one holding
    /// a peak or a trough. Otherwise the ends are the extremes. Argument
    /// reduction loses accuracy far from the origin, so a distant span falls
    /// back to the full range, which is looser and still contains the truth.
    #[must_use]
    pub fn sin(self) -> Self {
        self.wave(0.0)
    }

    /// Cosine, which is the sine a quarter turn ahead.
    #[must_use]
    pub fn cos(self) -> Self {
        self.wave(std::f64::consts::FRAC_PI_2)
    }

    fn wave(self, shift: f64) -> Self {
        const FAR: f64 = 1e9;
        let full = Self {
            lo: -1.0,
            hi: 1.0,
            certainty: self.certainty,
        };
        if !self.lo.is_finite()
            || !self.hi.is_finite()
            || self.lo.abs() > FAR
            || self.hi.abs() > FAR
            || self.width() >= std::f64::consts::TAU
        {
            return full;
        }
        let at = |value: f64| (value + shift).sin();
        let (mut lo, mut hi) = (at(self.lo).min(at(self.hi)), at(self.lo).max(at(self.hi)));
        // A peak sits a quarter turn past every whole turn; a trough sits three
        // quarters past. Walk the few turns the span can reach and take any
        // that land inside it.
        let base = (self.lo + shift) / std::f64::consts::TAU;
        let first = base.floor() as i64 - 1;
        let last = ((self.hi + shift) / std::f64::consts::TAU).ceil() as i64 + 1;
        for turn in first..=last {
            let turn = turn as f64 * std::f64::consts::TAU;
            let peak = turn + std::f64::consts::FRAC_PI_2 - shift;
            let trough = turn + 3.0 * std::f64::consts::FRAC_PI_2 - shift;
            if peak >= self.lo && peak <= self.hi {
                hi = 1.0;
            }
            if trough >= self.lo && trough <= self.hi {
                lo = -1.0;
            }
        }
        Self::elementary(lo.max(-1.0), hi.min(1.0), self.certainty)
    }

    /// Tangent.
    ///
    /// Reported as the whole line whenever the span can reach a pole, because
    /// there the function is neither bounded nor continuous, and a plotter that
    /// treats the jump across a pole as a crossing draws a curve that is not
    /// there.
    #[must_use]
    pub fn tan(self) -> Self {
        let cos = self.cos();
        if cos.holds_zero() {
            return Self::WHOLE;
        }
        self.sin() / cos
    }
}

impl std::ops::Neg for Enclosure {
    type Output = Self;

    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
            certainty: self.certainty,
        }
    }
}

impl std::ops::Add for Enclosure {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::exact(
            self.lo + other.lo,
            self.hi + other.hi,
            self.certainty.and(other.certainty),
        )
    }
}

impl std::ops::Sub for Enclosure {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::exact(
            self.lo - other.hi,
            self.hi - other.lo,
            self.certainty.and(other.certainty),
        )
    }
}

impl std::ops::Mul for Enclosure {
    type Output = Self;

    /// The extremes of a product of two spans are among the four corner
    /// products, so taking the least and greatest of those bounds it.
    fn mul(self, other: Self) -> Self {
        let corners = [
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ];
        // A zero times an infinity is the one corner with no value. It arises
        // only when one span touches zero and the other runs to infinity, and
        // the product then really can be anything.
        if corners.iter().any(|value| value.is_nan()) {
            return Self::WHOLE;
        }
        let lo = corners.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::exact(lo, hi, self.certainty.and(other.certainty))
    }
}

impl std::ops::Div for Enclosure {
    type Output = Self;

    /// A divisor that can be zero makes the true function unbounded and
    /// discontinuous somewhere in the box, so the answer is the whole line and
    /// nothing may be concluded from it. That is exactly the case a sign test
    /// gets wrong: a pole is not a zero, and `y = tan(x)` is drawn at every
    /// asymptote by a plotter that cannot tell them apart.
    fn div(self, other: Self) -> Self {
        if other.holds_zero() {
            return Self::WHOLE;
        }
        let corners = [
            self.lo / other.lo,
            self.lo / other.hi,
            self.hi / other.lo,
            self.hi / other.hi,
        ];
        if corners.iter().any(|value| value.is_nan()) {
            return Self::WHOLE;
        }
        let lo = corners.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = corners.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::exact(lo, hi, self.certainty.and(other.certainty))
    }
}

#[cfg(test)]
mod tests {
    use super::{Certainty, ELEMENTARY_ULPS, Enclosure};
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    fn holds(set: Enclosure, value: f64) -> bool {
        value >= set.lo() && value <= set.hi()
    }

    /// Walk a span and require the enclosure to contain every value taken.
    fn contains_every_sample(set: Enclosure, span: (f64, f64), f: impl Fn(f64) -> f64) {
        for step in 0..=200 {
            let at = span.0 + (span.1 - span.0) * f64::from(step) / 200.0;
            let value = f(at);
            if value.is_finite() {
                assert!(
                    holds(set, value),
                    "{value} at {at} escaped [{}, {}]",
                    set.lo(),
                    set.hi()
                );
            }
        }
    }

    #[test]
    fn a_point_is_its_own_enclosure_and_a_reversed_span_concludes_nothing() {
        let point = Enclosure::point(2.5);
        assert_eq!((point.lo(), point.hi()), (2.5, 2.5));
        assert_eq!(point.certainty(), Certainty::Sound);
        assert_eq!(Enclosure::span(1.0, 0.0), Enclosure::WHOLE);
        assert_eq!(Enclosure::point(f64::NAN), Enclosure::WHOLE);
        assert_eq!(Enclosure::WHOLE.certainty(), Certainty::Doubtful);
    }

    #[test]
    fn arithmetic_contains_every_value_the_true_function_takes() {
        let x = Enclosure::span(-2.0, 3.0);
        let y = Enclosure::span(1.0, 4.0);
        let sum = x + y;
        let difference = x - y;
        let product = x * y;
        for step in 0..=50 {
            let a = -2.0 + 5.0 * f64::from(step) / 50.0;
            for inner in 0..=50 {
                let b = 1.0 + 3.0 * f64::from(inner) / 50.0;
                assert!(holds(sum, a + b));
                assert!(holds(difference, a - b));
                assert!(holds(product, a * b));
            }
        }
    }

    #[test]
    fn a_product_spanning_zero_takes_its_least_value_off_the_corners() {
        // The corner products of [-2,3] by [-2,3] are 4, -6, -6, 9.
        let span = Enclosure::span(-2.0, 3.0);
        let product = span * span;
        assert!(product.lo() <= -6.0 && product.hi() >= 9.0);
        // And an even power does not: it is least at zero, not at an end.
        let squared = span.powi(2);
        assert_eq!(squared.lo().max(0.0), 0.0);
        assert!(squared.hi() >= 9.0);
        contains_every_sample(squared, (-2.0, 3.0), |v| v * v);
    }

    #[test]
    fn dividing_by_a_span_that_can_be_zero_concludes_nothing() {
        let divided = Enclosure::point(1.0) / Enclosure::span(-1.0, 1.0);
        assert_eq!(divided, Enclosure::WHOLE);
        let safe = Enclosure::span(1.0, 2.0) / Enclosure::span(2.0, 4.0);
        assert_eq!(safe.certainty(), Certainty::Sound);
        contains_every_sample(safe, (1.0, 2.0), |v| v / 2.0);
        contains_every_sample(safe, (1.0, 2.0), |v| v / 4.0);
    }

    #[test]
    fn a_tangent_that_can_reach_a_pole_is_not_narrowed_into_a_crossing() {
        // This is the failure that draws a curve at every asymptote.
        let across = Enclosure::span(FRAC_PI_2 - 0.1, FRAC_PI_2 + 0.1);
        assert_eq!(across.tan(), Enclosure::WHOLE);
        assert_eq!(across.tan().certainty(), Certainty::Doubtful);
        let clear = Enclosure::span(0.1, 0.4);
        assert_eq!(clear.tan().certainty(), Certainty::Sound);
        contains_every_sample(clear.tan(), (0.1, 0.4), f64::tan);
    }

    #[test]
    fn a_wave_holding_a_peak_reaches_it_even_though_no_end_does() {
        let over_peak = Enclosure::span(FRAC_PI_2 - 0.3, FRAC_PI_2 + 0.3);
        assert_eq!(over_peak.sin().hi().min(1.0), 1.0);
        contains_every_sample(
            over_peak.sin(),
            (FRAC_PI_2 - 0.3, FRAC_PI_2 + 0.3),
            f64::sin,
        );
        let over_trough = Enclosure::span(-FRAC_PI_2 - 0.3, -FRAC_PI_2 + 0.3);
        assert!(over_trough.sin().lo() <= -1.0);
        let quiet = Enclosure::span(0.0, 0.5);
        contains_every_sample(quiet.sin(), (0.0, 0.5), f64::sin);
        assert!(quiet.sin().width() < 0.6, "a narrow span stays narrow");
        let whole_turn = Enclosure::span(0.0, TAU + 1.0);
        assert!(whole_turn.sin().lo() <= -1.0 && whole_turn.sin().hi() >= 1.0);
    }

    #[test]
    fn cosine_is_the_sine_a_quarter_turn_ahead() {
        for (lo, hi) in [(0.0, 0.4), (-3.0, -2.5), (PI - 0.2, PI + 0.2), (1.0, 5.0)] {
            let span = Enclosure::span(lo, hi);
            contains_every_sample(span.cos(), (lo, hi), f64::cos);
            contains_every_sample(span.sin(), (lo, hi), f64::sin);
        }
    }

    #[test]
    fn a_wave_far_from_the_origin_widens_instead_of_trusting_reduction() {
        let far = Enclosure::span(1e12, 1e12 + 0.1);
        assert_eq!((far.sin().lo(), far.sin().hi()), (-1.0, 1.0));
    }

    #[test]
    fn the_functions_the_positive_line_owns_refuse_the_rest_of_it() {
        assert_eq!(Enclosure::span(-1.0, 1.0).ln(), Enclosure::WHOLE);
        assert_eq!(Enclosure::span(-1.0, 1.0).sqrt(), Enclosure::WHOLE);
        let safe = Enclosure::span(1.0, 4.0);
        assert_eq!(safe.sqrt().certainty(), Certainty::Sound);
        contains_every_sample(safe.sqrt(), (1.0, 4.0), f64::sqrt);
        contains_every_sample(safe.ln(), (1.0, 4.0), f64::ln);
        contains_every_sample(safe.exp(), (1.0, 4.0), f64::exp);
    }

    #[test]
    fn a_step_across_a_whole_number_concludes_nothing_but_still_contains_it() {
        let across = Enclosure::span(1.5, 2.5).floor();
        assert_eq!((across.lo(), across.hi()), (1.0, 2.0));
        assert_eq!(across.certainty(), Certainty::Doubtful);
        let inside = Enclosure::span(1.2, 1.8).floor();
        assert_eq!((inside.lo(), inside.hi()), (1.0, 1.0));
        assert_eq!(inside.certainty(), Certainty::Sound);
    }

    #[test]
    fn a_remainder_stays_inside_its_divisor_and_keeps_one_whole_quotient_exact() {
        let exact = Enclosure::span(3.2, 3.8).rem_euclid(Enclosure::point(3.0));
        assert_eq!(exact.certainty(), Certainty::Sound);
        contains_every_sample(exact, (3.2, 3.8), |v| v.rem_euclid(3.0));
        let across = Enclosure::span(2.5, 3.5).rem_euclid(Enclosure::point(3.0));
        assert_eq!(across.certainty(), Certainty::Doubtful);
        contains_every_sample(across, (2.5, 3.5), |v| v.rem_euclid(3.0));
        assert_eq!(
            Enclosure::point(1.0).rem_euclid(Enclosure::span(-1.0, 1.0)),
            Enclosure::WHOLE
        );
    }

    #[test]
    fn absolute_value_folds_a_span_without_losing_its_reach() {
        assert_eq!(
            (
                Enclosure::span(-3.0, 2.0).abs().lo(),
                Enclosure::span(-3.0, 2.0).abs().hi()
            ),
            (0.0, 3.0)
        );
        let positive = Enclosure::span(1.0, 2.0);
        assert_eq!(positive.abs(), positive);
        let negative = Enclosure::span(-2.0, -1.0);
        assert_eq!((negative.abs().lo(), negative.abs().hi()), (1.0, 2.0));
    }

    #[test]
    fn min_and_max_take_the_bounds_value_by_value() {
        let a = Enclosure::span(0.0, 3.0);
        let b = Enclosure::span(1.0, 2.0);
        assert_eq!((a.min(b).lo(), a.min(b).hi()), (0.0, 2.0));
        assert_eq!((a.max(b).lo(), a.max(b).hi()), (1.0, 3.0));
    }

    #[test]
    fn doubt_spreads_through_every_operation_that_combines_two_sets() {
        let doubtful = Enclosure::span(1.0, 2.0).doubted();
        let sound = Enclosure::span(1.0, 2.0);
        for combined in [
            doubtful + sound,
            doubtful - sound,
            doubtful * sound,
            doubtful / sound,
            doubtful.min(sound),
            doubtful.max(sound),
            sound + doubtful,
        ] {
            assert_eq!(combined.certainty(), Certainty::Doubtful);
        }
        assert_eq!((sound + sound).certainty(), Certainty::Sound);
    }

    #[test]
    fn a_negative_power_refuses_a_base_that_can_be_zero() {
        assert_eq!(Enclosure::span(-1.0, 1.0).powi(-1), Enclosure::WHOLE);
        let safe = Enclosure::span(1.0, 2.0).powi(-2);
        contains_every_sample(safe, (1.0, 2.0), |v| v.powi(-2));
        assert_eq!(Enclosure::span(3.0, 4.0).powi(0), Enclosure::point(1.0));
    }

    #[test]
    fn the_declared_widening_is_the_only_trust_this_module_places_anywhere() {
        // A regression on the constant itself: soundness for the elementary
        // functions rests on it, so a change to it is a change to a claim.
        assert_eq!(ELEMENTARY_ULPS, 4);
        let exact = Enclosure::point(1.0) + Enclosure::point(1.0);
        assert!(exact.lo() < 2.0 && exact.hi() > 2.0, "one ulp each side");
        assert!(exact.width() < 1e-15);
    }
}
