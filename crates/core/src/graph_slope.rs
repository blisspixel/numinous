//! A symbolic slope beside one open Studio graph.
//!
//! `sin(a*x)` rewrites to `a*cos(a*x)`. A sum or difference of that form,
//! a line (`x`, a numeric or `a` multiple of `x`, or `x` divided by a
//! numeric constant), and a non-negative integer power of `x` rewrites to
//! the sum of those derivatives. The opening formula `sin(a*x) + x/3`
//! becomes `a*cos(a*x)+1/3`. A constant term drops out, and `1/3` stays a
//! quotient. The capsule keeps the player's source. Samples of the
//! rewritten source are checked, in tests, against an independent slope
//! where both are defined. `floor`, `mod`, `min`, `max`, `euclid`, `pat`,
//! and `note` are refused. Every other graph is left without a reading.

use std::f64::consts::{E, PI};

use crate::studio::{
    Expr, Func, MAX_STUDIO_SOURCE_CHARS, Op, PairFunc, StudioCreation, StudioKind, eval, parse,
};

/// The largest integer a Studio number can hold exactly.
const MAX_EXACT_INT: i128 = 1 << 53;

/// The rewritten source and the expression parsed from it.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphDerivative {
    /// Canonical derivative source, in Studio expression text.
    pub source: String,
    /// Parsed form of [`Self::source`].
    pub expression: Expr,
}

/// A slope reading for one graph expression.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphSlope {
    /// An exact derivative this slice can name.
    Derivative(GraphDerivative),
    /// The grammar has a construct this slice refuses to differentiate.
    Refused,
}

impl GraphSlope {
    /// Read one parsed graph. Refusal wins over a matching sum.
    ///
    /// A recognized sum rewrites to its derivative. A refused construct
    /// anywhere in the tree returns [`Self::Refused`]. A constant graph, and
    /// every shape this slice does not rewrite exactly, returns none:
    /// absence is not a secant and not a guess.
    #[must_use]
    pub fn of_expression(expr: &Expr) -> Option<Self> {
        if refuses_slope(expr) {
            return Some(Self::Refused);
        }
        let terms = differentiate(expr)?;
        if terms.is_empty() {
            return None;
        }
        let source = print_terms(&terms);
        if source.chars().count() > MAX_STUDIO_SOURCE_CHARS {
            return None;
        }
        let expression = parse(&source).ok()?;
        Some(Self::Derivative(GraphDerivative { source, expression }))
    }

    /// Read a Studio creation. Only a graph is eligible.
    ///
    /// A field, a parametric path, and an overlay return none even when a
    /// graph inside them would match. The capsule is not modified.
    #[must_use]
    pub fn of_creation(creation: &StudioCreation) -> Option<Self> {
        if creation.kind() != StudioKind::Graph {
            return None;
        }
        let program = creation.program().ok()?;
        Self::of_expression(program.voice_expression())
    }

    /// Terminal lines for the reading.
    ///
    /// A derivative names its source. A refusal is `slope=refused`. Neither
    /// line is a period claim.
    #[must_use]
    pub fn report_lines(&self) -> Vec<String> {
        match self {
            Self::Derivative(derivative) => {
                vec![format!("slope basis=symbolic source={}", derivative.source)]
            }
            Self::Refused => vec!["slope=refused".to_string()],
        }
    }
}

/// Independent slope `(f(x+h) - f(x-h)) / (2h)`.
///
/// Tests use this to check the symbolic source. The App, the CLI, and MCP
/// do not call it. Named sliders are unbound, because the recognized form
/// in this slice has none. Returns none when `x`, `parameter`, or `step`
/// is non-finite, when `step` is zero, or when the quotient is non-finite.
#[must_use]
pub fn central_slope(expr: &Expr, x: f64, parameter: f64, step: f64) -> Option<f64> {
    if !x.is_finite() || !parameter.is_finite() || !step.is_finite() || step == 0.0 {
        return None;
    }
    let ahead = eval(expr, x + step, parameter);
    let behind = eval(expr, x - step, parameter);
    let slope = (ahead - behind) / (2.0 * step);
    slope.is_finite().then_some(slope)
}

/// A coefficient that stays exact in source text: a reduced rational, times
/// optional powers of `a`, `pi`, and `e`. `a` may not appear in a denominator,
/// because the knob can be zero. A decimal literal is not a rational here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Factor {
    num: i128,
    den: i128,
    a_power: i32,
    pi_power: i32,
    e_power: i32,
}

impl Factor {
    fn zero() -> Self {
        Self {
            num: 0,
            den: 1,
            a_power: 0,
            pi_power: 0,
            e_power: 0,
        }
    }

    fn one() -> Self {
        Self::from_int(1)
    }

    fn from_int(num: i128) -> Self {
        Self {
            num,
            den: 1,
            a_power: 0,
            pi_power: 0,
            e_power: 0,
        }
    }

    fn named_a() -> Self {
        Self {
            num: 1,
            den: 1,
            a_power: 1,
            pi_power: 0,
            e_power: 0,
        }
    }

    fn pi() -> Self {
        Self {
            num: 1,
            den: 1,
            a_power: 0,
            pi_power: 1,
            e_power: 0,
        }
    }

    fn e() -> Self {
        Self {
            num: 1,
            den: 1,
            a_power: 0,
            pi_power: 0,
            e_power: 1,
        }
    }

    fn is_zero(self) -> bool {
        self.num == 0
    }

    fn from_number(value: f64) -> Option<Self> {
        if value == 0.0 {
            return Some(Self::zero());
        }
        if value == PI {
            return Some(Self::pi());
        }
        if value == E {
            return Some(Self::e());
        }
        if value < 0.0 {
            return Self::from_number(-value)?.mul_int(-1);
        }
        Self::from_int(exact_int(value)?).reduced()
    }

    fn mul(self, other: Self) -> Option<Self> {
        if self.is_zero() || other.is_zero() {
            return Some(Self::zero());
        }
        Self {
            num: self.num.checked_mul(other.num)?,
            den: self.den.checked_mul(other.den)?,
            a_power: self.a_power.checked_add(other.a_power)?,
            pi_power: self.pi_power.checked_add(other.pi_power)?,
            e_power: self.e_power.checked_add(other.e_power)?,
        }
        .reduced()
    }

    fn mul_int(self, value: i128) -> Option<Self> {
        self.mul(Self::from_int(value))
    }

    /// Divide by a constant. `a` in the divisor is outside this slice.
    fn div(self, other: Self) -> Option<Self> {
        if other.is_zero() || other.a_power != 0 {
            return None;
        }
        self.mul(Self {
            num: other.den,
            den: other.num,
            a_power: 0,
            pi_power: other.pi_power.checked_neg()?,
            e_power: other.e_power.checked_neg()?,
        })
    }

    fn pow(self, exp: u32) -> Option<Self> {
        if exp == 0 {
            return Some(Self::one());
        }
        let mut acc = Self::one();
        let mut base = self;
        let mut remaining = exp;
        while remaining > 0 {
            if remaining % 2 == 1 {
                acc = acc.mul(base)?;
            }
            remaining /= 2;
            if remaining > 0 {
                base = base.mul(base)?;
            }
        }
        Some(acc)
    }

    fn reduced(self) -> Option<Self> {
        if self.num == 0 {
            return Some(Self::zero());
        }
        let mut num = self.num;
        let mut den = self.den;
        if den < 0 {
            num = num.checked_neg()?;
            den = den.checked_neg()?;
        }
        if den == 0 {
            return None;
        }
        let g = gcd(unsigned_mag(num)?, unsigned_mag(den)?);
        let g = i128::try_from(g).ok()?;
        num /= g;
        den /= g;
        if !fits_source_int(num) || !fits_source_int(den) {
            return None;
        }
        Some(Self {
            num,
            den,
            a_power: self.a_power,
            pi_power: self.pi_power,
            e_power: self.e_power,
        })
    }

    fn add_rational(self, other: Self) -> Option<Self> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        let den = self.den.checked_mul(other.den)?;
        Self {
            num,
            den,
            a_power: self.a_power,
            pi_power: self.pi_power,
            e_power: self.e_power,
        }
        .reduced()
    }
}

/// What a recognized term is, before it is differentiated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Body {
    /// No `x`. Its derivative is zero.
    Const,
    /// `x` raised to a positive integer.
    Power(u32),
    /// `sin(a*x)` or `sin(x*a)`.
    SinAx,
}

/// What remains after differentiation, besides the coefficient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rest {
    Const,
    /// `x` raised to a positive integer. `1` prints as `x`.
    Power(u32),
    /// `cos(a*x)`.
    Cos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Input {
    factor: Factor,
    body: Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OutTerm {
    factor: Factor,
    rest: Rest,
}

impl OutTerm {
    fn same_shape(self, other: Self) -> bool {
        self.rest == other.rest
            && self.factor.a_power == other.factor.a_power
            && self.factor.pi_power == other.factor.pi_power
            && self.factor.e_power == other.factor.e_power
    }

    fn add_same(self, other: Self) -> Option<Self> {
        Some(Self {
            factor: self.factor.add_rational(other.factor)?,
            rest: self.rest,
        })
    }
}

fn differentiate(expr: &Expr) -> Option<Vec<OutTerm>> {
    match expr {
        Expr::Bin(Op::Add, left, right) => merge(differentiate(left)?, differentiate(right)?),
        Expr::Bin(Op::Sub, left, right) => {
            merge(differentiate(left)?, negate(differentiate(right)?)?)
        }
        Expr::Neg(inner) => negate(differentiate(inner)?),
        other => match derive_one(peel(other)?)? {
            Some(term) => Some(vec![term]),
            None => Some(Vec::new()),
        },
    }
}

fn derive_one(input: Input) -> Option<Option<OutTerm>> {
    let term = match input.body {
        Body::Const | Body::Power(0) => return Some(None),
        Body::Power(1) => OutTerm {
            factor: input.factor,
            rest: Rest::Const,
        },
        Body::Power(n) => OutTerm {
            factor: input.factor.mul_int(i128::from(n))?,
            rest: Rest::Power(n - 1),
        },
        Body::SinAx => OutTerm {
            factor: input.factor.mul(Factor::named_a())?,
            rest: Rest::Cos,
        },
    };
    if term.factor.is_zero() {
        Some(None)
    } else {
        Some(Some(term))
    }
}

fn peel(expr: &Expr) -> Option<Input> {
    match expr {
        Expr::Bin(Op::Mul, left, right) => {
            if let Some(factor) = as_factor(left) {
                let mut input = peel(right)?;
                input.factor = input.factor.mul(factor)?;
                Some(input)
            } else if let Some(factor) = as_factor(right) {
                let mut input = peel(left)?;
                input.factor = input.factor.mul(factor)?;
                Some(input)
            } else {
                None
            }
        }
        Expr::Bin(Op::Div, left, right) => {
            let mut input = peel(left)?;
            input.factor = input.factor.div(as_factor(right)?)?;
            Some(input)
        }
        Expr::Var => Some(Input {
            factor: Factor::one(),
            body: Body::Power(1),
        }),
        Expr::Bin(Op::Pow, base, exp) => {
            if !matches!(base.as_ref(), Expr::Var) {
                return Some(Input {
                    factor: as_factor(expr)?,
                    body: Body::Const,
                });
            }
            let &Expr::Num(value) = exp.as_ref() else {
                return None;
            };
            let n = exact_int(value)?;
            if n < 0 {
                return None;
            }
            if n == 0 {
                return Some(Input {
                    factor: Factor::one(),
                    body: Body::Const,
                });
            }
            Some(Input {
                factor: Factor::one(),
                body: Body::Power(u32::try_from(n).ok()?),
            })
        }
        Expr::Call(Func::Sin, argument) if is_parameter_times_variable(argument) => Some(Input {
            factor: Factor::one(),
            body: Body::SinAx,
        }),
        other => Some(Input {
            factor: as_factor(other)?,
            body: Body::Const,
        }),
    }
}

fn as_factor(expr: &Expr) -> Option<Factor> {
    match expr {
        Expr::Num(value) => Factor::from_number(*value),
        Expr::Param => Some(Factor::named_a()),
        Expr::Neg(inner) => as_factor(inner)?.mul_int(-1),
        Expr::Bin(Op::Mul, left, right) => as_factor(left)?.mul(as_factor(right)?),
        Expr::Bin(Op::Div, left, right) => as_factor(left)?.div(as_factor(right)?),
        Expr::Bin(Op::Pow, base, exp) => {
            let &Expr::Num(value) = exp.as_ref() else {
                return None;
            };
            let n = exact_int(value)?;
            if n < 0 {
                return None;
            }
            as_factor(base)?.pow(u32::try_from(n).ok()?)
        }
        _ => None,
    }
}

fn merge(mut left: Vec<OutTerm>, right: Vec<OutTerm>) -> Option<Vec<OutTerm>> {
    for term in right {
        if term.factor.is_zero() {
            continue;
        }
        if let Some(existing) = left.iter_mut().find(|item| item.same_shape(term)) {
            *existing = existing.add_same(term)?;
        } else {
            left.push(term);
        }
    }
    left.retain(|term| !term.factor.is_zero());
    Some(left)
}

fn negate(terms: Vec<OutTerm>) -> Option<Vec<OutTerm>> {
    terms
        .into_iter()
        .map(|mut term| {
            term.factor = term.factor.mul_int(-1)?;
            Some(term)
        })
        .collect()
}

fn print_terms(terms: &[OutTerm]) -> String {
    let mut out = String::new();
    for (index, term) in terms.iter().enumerate() {
        let negative = term.factor.num < 0;
        let magnitude = magnitude_source(term);
        if index > 0 && !negative {
            out.push('+');
        }
        if negative {
            out.push('-');
        }
        out.push_str(&magnitude);
    }
    out
}

fn magnitude_source(term: &OutTerm) -> String {
    let mut nums = Vec::new();
    let mut dens = Vec::new();
    if term.factor.a_power > 0 {
        nums.push(powered("a", term.factor.a_power));
    }
    if term.factor.pi_power > 0 {
        nums.push(powered("pi", term.factor.pi_power));
    }
    if term.factor.e_power > 0 {
        nums.push(powered("e", term.factor.e_power));
    }
    match term.rest {
        Rest::Const => {}
        Rest::Power(1) => nums.push("x".to_string()),
        Rest::Power(power) => nums.push(format!("x^{power}")),
        Rest::Cos => nums.push("cos(a*x)".to_string()),
    }
    let mag = term.factor.num.unsigned_abs();
    if mag != 1 || nums.is_empty() {
        nums.insert(0, mag.to_string());
    }
    if term.factor.den != 1 {
        dens.push(term.factor.den.to_string());
    }
    if term.factor.pi_power < 0 {
        dens.push(powered("pi", -term.factor.pi_power));
    }
    if term.factor.e_power < 0 {
        dens.push(powered("e", -term.factor.e_power));
    }
    let numerator = nums.join("*");
    if dens.is_empty() {
        numerator
    } else if dens.len() == 1 {
        format!("{numerator}/{}", dens[0])
    } else {
        format!("{numerator}/({})", dens.join("*"))
    }
}

fn powered(name: &str, power: i32) -> String {
    if power == 1 {
        name.to_string()
    } else {
        format!("{name}^{power}")
    }
}

fn is_parameter_times_variable(expr: &Expr) -> bool {
    let Expr::Bin(Op::Mul, left, right) = expr else {
        return false;
    };
    matches!(
        (left.as_ref(), right.as_ref()),
        (Expr::Param, Expr::Var) | (Expr::Var, Expr::Param)
    )
}

fn refuses_slope(expr: &Expr) -> bool {
    match expr {
        Expr::Call(Func::Floor, _)
        | Expr::PairCall(PairFunc::Mod | PairFunc::Min | PairFunc::Max | PairFunc::Euclid, _, _)
        | Expr::Pattern(_)
        | Expr::Notes(_) => true,
        Expr::Neg(inner) | Expr::Call(_, inner) => refuses_slope(inner),
        Expr::Bin(_, left, right) => refuses_slope(left) || refuses_slope(right),
        Expr::Num(_)
        | Expr::Var
        | Expr::VarIm
        | Expr::Point
        | Expr::ImagUnit
        | Expr::Param
        | Expr::Slider(_) => false,
    }
}

fn exact_int(value: f64) -> Option<i128> {
    if !value.is_finite() || value.fract() != 0.0 || value.abs() > MAX_EXACT_INT as f64 {
        return None;
    }
    let n = value as i128;
    (n as f64 == value).then_some(n)
}

fn fits_source_int(value: i128) -> bool {
    (-MAX_EXACT_INT..=MAX_EXACT_INT).contains(&value) && (value as f64) as i128 == value
}

fn unsigned_mag(value: i128) -> Option<u128> {
    u128::try_from(value.checked_abs()?).ok()
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::{GraphSlope, central_slope};
    use crate::field::FieldReading;
    use crate::studio::{
        StudioCreation, StudioScale, eval, graph_and_slope_melody, parse, to_melody,
    };

    fn reading(source: &str) -> Option<GraphSlope> {
        GraphSlope::of_expression(&parse(source).expect(source))
    }

    fn expect_slope(source: &str, expected: &str) {
        let slope = reading(source).unwrap_or_else(|| panic!("{source} should have a slope"));
        let GraphSlope::Derivative(found) = slope else {
            panic!("{source} was refused");
        };
        assert_eq!(found.source, expected, "{source}");
        assert_eq!(
            found.expression,
            parse(expected).unwrap_or_else(|_| panic!("{expected} should parse")),
            "{source}"
        );
        let line = GraphSlope::Derivative(found).report_lines().join("\n");
        assert_eq!(line, format!("slope basis=symbolic source={expected}"));
    }

    fn assert_absent(source: &str) {
        assert!(reading(source).is_none(), "{source} is outside this slice");
    }

    fn assert_agrees(source: &str, expected: &str) {
        expect_slope(source, expected);
        let graph = parse(source).expect(source);
        let slope = parse(expected).expect(expected);
        for parameter in [1.0, 2.5, -1.5] {
            for x in [-1.0, -0.3, 0.0, 0.4, 1.0] {
                let Some(numerical) = central_slope(&graph, x, parameter, 1e-4) else {
                    continue;
                };
                let symbolic = eval(&slope, x, parameter);
                if !symbolic.is_finite() {
                    continue;
                }
                assert!(
                    (numerical - symbolic).abs() < 1e-6,
                    "{source} at x={x} a={parameter}: numerical {numerical}, symbolic {symbolic}"
                );
            }
        }
    }

    #[test]
    fn sin_a_x_rewrites_to_a_cos_a_x() {
        for source in ["sin(a*x)", "sin(x*a)", "sin( a * x )", "sin((a)*(x))"] {
            expect_slope(source, "a*cos(a*x)");
            let graph = parse(source).expect(source);
            let slope = parse("a*cos(a*x)").expect("slope");
            assert_eq!(eval(&slope, 0.0, 2.0), 2.0);
            assert!((eval(&graph, 0.0, 2.0) - 0.0).abs() < 1e-12);
        }
    }

    #[test]
    fn the_opening_sum_grows_a_cos_a_x_plus_one_third() {
        for source in [
            "sin(a*x) + x/3",
            "sin(a*x)+x/3",
            "sin(a*x) + x/3 + 0",
            "sin(a*x)+0+x/3",
        ] {
            assert_agrees(source, "a*cos(a*x)+1/3");
        }
        assert_agrees("x/3+sin(a*x)", "1/3+a*cos(a*x)");
        assert_agrees("sin(a*x)-x/3", "a*cos(a*x)-1/3");
        assert_agrees("sin(a*x)+0", "a*cos(a*x)");
        assert_agrees("sin(a*x)+x-x", "a*cos(a*x)");
        assert_agrees("2*sin(a*x)", "2*a*cos(a*x)");
        assert_agrees("sin(a*x)+sin(x*a)", "2*a*cos(a*x)");
        assert_agrees("-sin(a*x)", "-a*cos(a*x)");

        let creation = StudioCreation::new("sin(a*x) + x/3", -2.0, 2.0, 1.0).expect("creation");
        let file = creation.to_num_file();
        let GraphSlope::Derivative(found) =
            GraphSlope::of_creation(&creation).expect("opening slope")
        else {
            panic!("the opening formula is a derivative");
        };
        assert_eq!(found.source, "a*cos(a*x)+1/3");
        assert_eq!(creation.source(), "sin(a*x) + x/3");
        assert!(!file.contains("cos"), "{file}");
    }

    #[test]
    fn lines_and_integer_powers_rewrite_exactly() {
        assert_agrees("x", "1");
        assert_agrees("2*x", "2");
        assert_agrees("x*2", "2");
        assert_agrees("x/3", "1/3");
        assert_agrees("x/4", "1/4");
        assert_agrees("a*x", "a");
        assert_agrees("x*a", "a");
        assert_agrees("2*a*x", "2*a");
        assert_agrees("x+x", "2");
        assert_agrees("x^2", "2*x");
        assert_agrees("x^2.0", "2*x");
        assert_agrees("x^3", "3*x^2");
        assert_agrees("3*x^2", "6*x");
        assert_agrees("x^2*3", "6*x");
        assert_agrees("x^2+x^2", "4*x");
        assert_agrees("x^2/12 - 1", "x/6");
        assert_agrees("x^2+x", "2*x+1");
        assert_agrees("sin(a*x)+x", "a*cos(a*x)+1");
        assert_agrees("x/(-2)", "-1/2");
        assert_agrees("x^2/2", "x");
        assert_agrees("-x^2", "-2*x");
        assert_agrees("x^3/3", "x^2");
        assert_agrees("2*pi*x", "2*pi");
        assert_agrees("x/pi", "1/pi");
        assert_agrees("x/(2*pi)", "1/(2*pi)");
        assert_agrees("e*x", "e");
        assert_agrees("a^2*x", "a^2");
        assert_agrees("-(x/3)", "-1/3");
    }

    #[test]
    fn the_independent_slope_rejects_a_bad_step() {
        let graph = parse("sin(a*x)").expect("graph");
        assert!(central_slope(&graph, 0.0, 1.0, 0.0).is_none());
        assert!(central_slope(&graph, f64::NAN, 1.0, 1e-4).is_none());
        assert!(central_slope(&graph, 0.0, f64::INFINITY, 1e-4).is_none());
        assert!(central_slope(&graph, 0.0, 1.0, f64::NAN).is_none());

        let creation = StudioCreation::new("sin(a*x)", -1.0, 1.0, -4.0).expect("creation");
        let GraphSlope::Derivative(found) =
            GraphSlope::of_creation(&creation).expect("creation slope")
        else {
            panic!("negative a still names the same source");
        };
        assert_eq!(found.source, "a*cos(a*x)");
    }

    #[test]
    fn unrecognized_graphs_are_absent_and_rhythm_graphs_are_refused() {
        for source in [
            "sin(x)",
            "sin(b*x)",
            "cos(a*x)",
            "tan(x)",
            "ln(x)",
            "sqrt(x)",
            "abs(x)",
            "x*x",
            "0.5*x",
            "sin(a*x)+0.1",
            "x^2.5",
            "x^(-1)",
            "x^a",
            "sin(a*x)*x",
            "sin(a*x+x)",
            "sin(2*a*x)",
            "2*(x+1)",
            "(a+1)*x",
            "x/a",
            "a*x/a",
            "sin(a*x)/a",
            "2^x",
            "5",
            "a",
            "x-x",
            "0*x",
            "x/0",
            "x^0",
        ] {
            assert_absent(source);
        }
        for source in [
            "floor(x)",
            "mod(x,2)",
            "min(x,1)",
            "max(x,0)",
            "euclid(3,8)",
            "pat(x..x..x.)",
            "note(\"c e g\")",
            "sin(floor(x))",
            "floor(x)+sin(a*x)",
            "floor(x)+x",
        ] {
            let slope = reading(source).expect("refusal");
            assert!(matches!(slope, GraphSlope::Refused), "{source}");
            let line = slope.report_lines().join("\n");
            assert_eq!(line, "slope=refused");
            assert!(!line.contains("cos"), "{line}");
        }
    }

    #[test]
    fn only_a_graph_creation_carries_the_reading() {
        let graph = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5).expect("graph");
        let file = graph.to_num_file();
        assert!(matches!(
            GraphSlope::of_creation(&graph),
            Some(GraphSlope::Derivative(_))
        ));
        assert_eq!(graph.source(), "sin(a*x)");
        assert_eq!(graph.to_num_file(), file);
        assert!(!file.contains("cos"));

        let parametric = StudioCreation::new_parametric("cos(t)", "sin(a*t)", 0.0, 1.0, 1.0)
            .expect("parametric");
        assert!(GraphSlope::of_creation(&parametric).is_none());
        let field = StudioCreation::new_field("z", -1.0, 1.0, -1.0, 1.0, 1.0, FieldReading::Phase)
            .expect("field");
        assert!(GraphSlope::of_creation(&field).is_none());
        let overlay =
            StudioCreation::new_program(["sin(a*x)", "cos(x)"], -1.0, 1.0, 1.0).expect("overlay");
        assert!(GraphSlope::of_creation(&overlay).is_none());
        let refused = StudioCreation::new("floor(x)", 0.0, 4.0, 1.0).expect("floor");
        assert!(matches!(
            GraphSlope::of_creation(&refused),
            Some(GraphSlope::Refused)
        ));
    }

    #[test]
    fn the_beside_voice_shares_one_axis_and_keeps_grid_time() {
        let graph = parse("sin(a*x)").expect("graph");
        let slope = parse("a*cos(a*x)").expect("slope");
        let beside = graph_and_slope_melody(
            &graph,
            &slope,
            -2.0,
            2.0,
            32,
            2.0,
            &[],
            StudioScale::Continuous,
        );
        let solo = to_melody(&graph, -2.0, 2.0, 32, 2.0);
        assert_eq!(solo.notes.len(), 32);
        assert_eq!(beside.notes.len(), 64);
        assert_eq!(beside.duration, solo.duration);
        let mut graph_peak = 0.0_f32;
        let mut slope_peak = 0.0_f32;
        for index in 0..32 {
            let left = &beside.notes[index * 2];
            let right = &beside.notes[index * 2 + 1];
            let start = index as f32 * 0.12;
            assert_eq!(left.start, start);
            assert_eq!(right.start, start);
            graph_peak = graph_peak.max(left.freq);
            slope_peak = slope_peak.max(right.freq);
        }
        assert!(
            slope_peak > graph_peak,
            "at a=2 the slope reaches about 2 and the graph about 1: slope {slope_peak}, graph {graph_peak}"
        );
        assert!(
            beside.notes[33].freq > beside.notes[32].freq,
            "pair 16 is near the slope peak"
        );

        let singular = parse("1/x").expect("singular");
        let gapped = graph_and_slope_melody(
            &singular,
            &singular,
            -1.0,
            1.0,
            3,
            1.0,
            &[],
            StudioScale::Continuous,
        );
        assert_eq!(gapped.notes.len(), 4);
        assert_eq!(gapped.notes[0].start, 0.0_f32);
        assert_eq!(gapped.notes[2].start, 2.0_f32 * 0.12);
        assert_eq!(gapped.duration, 3.0_f32 * 0.12 + 0.3);

        let infinite = parse("1/0").expect("infinite");
        let silent = graph_and_slope_melody(
            &infinite,
            &infinite,
            -1.0,
            1.0,
            4,
            1.0,
            &[],
            StudioScale::Continuous,
        );
        assert!(silent.notes.is_empty());
    }
}
