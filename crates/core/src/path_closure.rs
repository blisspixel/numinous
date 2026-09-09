//! Independently checked closure of a parametric Studio path.
//!
//! This is the first slice of the Returning home capability quest: a player
//! who can construct two oscillators should be able to ask whether the motion
//! repeats, without taking the picture as proof. It is a trial of the path,
//! not a grade of the player, and it does not gate rooms, study, or creation.

use crate::studio::{Expr, Func, Op, StudioCreation, StudioKind, StudioProgram};

/// What a Studio creation does in time, when that question is well posed.
#[derive(Debug, Clone, PartialEq)]
pub enum PathClosure {
    /// A graph is a height over x, not a planar path that can come home.
    Graph,
    /// The pair is not two harmonic oscillators of the form
    /// `A*sin(w*t+p)` or `A*cos(w*t+p)` with a constant scale and phase.
    Unsupported,
    /// A common period exists in the ideal model.
    Periodic(PeriodicClosure),
    /// The two frequencies are incommensurate, so the ideal motion never
    /// repeats.
    Aperiodic(AperiodicClosure),
}

/// A repeating two-oscillator path.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicClosure {
    /// Least positive common period as exact prose, for example `12` or `2*pi`.
    pub period_text: String,
    /// The same period as a finite float, for window comparison only.
    pub period: f64,
    /// Cycles of the x oscillator per unit t, for example `1` or `2`.
    pub x_frequency_text: String,
    /// Cycles of the y oscillator per unit t, for example `17/12`.
    pub y_frequency_text: String,
    /// How many x oscillations fit in one common period, when that count is
    /// an integer.
    pub x_cycles: Option<i64>,
    /// How many y oscillations fit in one common period, when that count is
    /// an integer.
    pub y_cycles: Option<i64>,
    /// How many common periods the saved window covers, when it covers a
    /// whole number of them.
    pub window_periods: Option<u32>,
    /// State at half a period after the window start. Position can return
    /// here while velocity reverses.
    pub half_period: ClosureCheckpoint,
    /// State at the saved window end.
    pub window_end: ClosureCheckpoint,
}

/// An ideal path with no positive common period.
#[derive(Debug, Clone, PartialEq)]
pub struct AperiodicClosure {
    /// Cycles of the x oscillator per unit t.
    pub x_frequency_text: String,
    /// Cycles of the y oscillator per unit t.
    pub y_frequency_text: String,
    /// State at the saved window end. A near return is still not a period.
    pub window_end: ClosureCheckpoint,
}

/// Position and full-state comparison against the window start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosureCheckpoint {
    /// The time that was compared to the window start.
    pub t: f64,
    /// Both coordinates match the start.
    pub position_returns: bool,
    /// Position and both velocities match the start.
    pub state_returns: bool,
}

impl PathClosure {
    /// Analyze one Studio creation. Graphs and unrecognized pairs stay
    /// explicit rather than guessed.
    #[must_use]
    pub fn of(creation: &StudioCreation) -> Self {
        match creation.kind() {
            StudioKind::Graph => Self::Graph,
            StudioKind::Parametric => analyze_parametric(creation),
        }
    }

    /// Terminal lines for a report. Empty when the creation has no planar
    /// path to discuss.
    #[must_use]
    pub fn report_lines(&self) -> Vec<String> {
        match self {
            Self::Graph | Self::Unsupported => Vec::new(),
            Self::Periodic(periodic) => {
                let mut lines = vec![format!(
                    "closure=periodic period={} x_freq={} y_freq={}",
                    periodic.period_text, periodic.x_frequency_text, periodic.y_frequency_text
                )];
                if let (Some(x_cycles), Some(y_cycles)) = (periodic.x_cycles, periodic.y_cycles) {
                    lines.push(format!("cycles_in_period x={x_cycles} y={y_cycles}"));
                }
                if let Some(count) = periodic.window_periods {
                    lines.push(format!("window covers {count} period(s)"));
                }
                lines.push(checkpoint_line("half-period", &periodic.half_period));
                lines.push(checkpoint_line("window-end", &periodic.window_end));
                lines
            }
            Self::Aperiodic(aperiodic) => vec![
                format!(
                    "closure=aperiodic x_freq={} y_freq={}",
                    aperiodic.x_frequency_text, aperiodic.y_frequency_text
                ),
                "ideal motion has no positive common period".to_string(),
                checkpoint_line("window-end", &aperiodic.window_end),
            ],
        }
    }

    /// One instrument caption for a status line. Graphs and unrecognized
    /// pairs stay quiet so ordinary Formula Jam is not a trial.
    #[must_use]
    pub fn status_caption(&self) -> Option<String> {
        match self {
            Self::Graph | Self::Unsupported => None,
            Self::Periodic(periodic) => {
                let mut line = format!("PERIOD {}", periodic.period_text);
                if periodic.half_period.position_returns && !periodic.half_period.state_returns {
                    line.push_str("  HALF: PLACE NOT STATE");
                }
                Some(line)
            }
            Self::Aperiodic(_) => Some("NO PERIOD".to_string()),
        }
    }
}

fn checkpoint_line(name: &str, checkpoint: &ClosureCheckpoint) -> String {
    let position = if checkpoint.position_returns {
        "position returns"
    } else {
        "position does not return"
    };
    let state = if checkpoint.state_returns {
        "state returns"
    } else {
        "state does not return"
    };
    format!("{name} t={} {position}; {state}", checkpoint.t)
}

fn analyze_parametric(creation: &StudioCreation) -> PathClosure {
    let Ok(StudioProgram::Parametric {
        x_expression,
        y_expression,
        ..
    }) = creation.program()
    else {
        return PathClosure::Unsupported;
    };
    let parameter = match Exact::from_f64(creation.a()) {
        Some(value) => value,
        None => return PathClosure::Unsupported,
    };
    let Some(x_osc) = oscillator(&x_expression, parameter) else {
        return PathClosure::Unsupported;
    };
    let Some(y_osc) = oscillator(&y_expression, parameter) else {
        return PathClosure::Unsupported;
    };
    let Some(x_freq) = frequency_cycles(&x_osc.omega) else {
        return PathClosure::Unsupported;
    };
    let Some(y_freq) = frequency_cycles(&y_osc.omega) else {
        return PathClosure::Unsupported;
    };
    let tmin = creation.xmin();
    let tmax = creation.xmax();
    if !tmin.is_finite() || !tmax.is_finite() || tmax <= tmin {
        return PathClosure::Unsupported;
    }
    let start = state(&x_osc, &y_osc, tmin);
    let window_end = checkpoint(&x_osc, &y_osc, start, tmax);
    if let Some(period) = common_period(&x_freq, &y_freq) {
        let half_t = tmin + period.value / 2.0;
        PathClosure::Periodic(PeriodicClosure {
            period_text: period.text.clone(),
            period: period.value,
            x_frequency_text: x_freq.text(),
            y_frequency_text: y_freq.text(),
            x_cycles: integer_cycles(&x_freq, period.value),
            y_cycles: integer_cycles(&y_freq, period.value),
            window_periods: whole_periods_in(tmax - tmin, period.value),
            half_period: checkpoint(&x_osc, &y_osc, start, half_t),
            window_end,
        })
    } else {
        PathClosure::Aperiodic(AperiodicClosure {
            x_frequency_text: x_freq.text(),
            y_frequency_text: y_freq.text(),
            window_end,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct Oscillator {
    kind: Func,
    amp: Exact,
    omega: Exact,
    phase: Exact,
}

#[derive(Debug, Clone, Copy)]
struct State {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn oscillator(expr: &Expr, parameter: Exact) -> Option<Oscillator> {
    match expr {
        Expr::Call(kind @ (Func::Sin | Func::Cos), arg) => {
            let (phase, omega) = affine(arg, parameter)?;
            if omega.is_zero() {
                return None;
            }
            Some(Oscillator {
                kind: *kind,
                amp: Exact::one(),
                omega,
                phase,
            })
        }
        Expr::Neg(inner) => {
            let mut osc = oscillator(inner, parameter)?;
            osc.amp = osc.amp.checked_neg()?;
            Some(osc)
        }
        Expr::Bin(Op::Mul, left, right) => {
            if let Some(scale) = constant(left, parameter) {
                let mut osc = oscillator(right, parameter)?;
                osc.amp = osc.amp.checked_mul(scale)?;
                return Some(osc);
            }
            if let Some(scale) = constant(right, parameter) {
                let mut osc = oscillator(left, parameter)?;
                osc.amp = osc.amp.checked_mul(scale)?;
                return Some(osc);
            }
            None
        }
        Expr::Bin(Op::Div, left, right) => {
            let scale = constant(right, parameter)?;
            let mut osc = oscillator(left, parameter)?;
            osc.amp = osc.amp.checked_div(scale)?;
            Some(osc)
        }
        _ => None,
    }
}

fn affine(expr: &Expr, parameter: Exact) -> Option<(Exact, Exact)> {
    match expr {
        Expr::Var => Some((Exact::zero(), Exact::one())),
        Expr::Neg(inner) => {
            let (phase, omega) = affine(inner, parameter)?;
            Some((phase.checked_neg()?, omega.checked_neg()?))
        }
        Expr::Bin(Op::Add, left, right) => {
            let (p0, w0) = affine(left, parameter)?;
            let (p1, w1) = affine(right, parameter)?;
            Some((p0.checked_add(p1)?, w0.checked_add(w1)?))
        }
        Expr::Bin(Op::Sub, left, right) => {
            let (p0, w0) = affine(left, parameter)?;
            let (p1, w1) = affine(right, parameter)?;
            Some((p0.checked_sub(p1)?, w0.checked_sub(w1)?))
        }
        Expr::Bin(Op::Mul, left, right) => {
            if let Some(scale) = constant(left, parameter) {
                let (phase, omega) = affine(right, parameter)?;
                return Some((phase.checked_mul(scale)?, omega.checked_mul(scale)?));
            }
            if let Some(scale) = constant(right, parameter) {
                let (phase, omega) = affine(left, parameter)?;
                return Some((phase.checked_mul(scale)?, omega.checked_mul(scale)?));
            }
            None
        }
        Expr::Bin(Op::Div, left, right) => {
            let scale = constant(right, parameter)?;
            let (phase, omega) = affine(left, parameter)?;
            Some((phase.checked_div(scale)?, omega.checked_div(scale)?))
        }
        _ => {
            let value = constant(expr, parameter)?;
            Some((value, Exact::zero()))
        }
    }
}

fn constant(expr: &Expr, parameter: Exact) -> Option<Exact> {
    match expr {
        Expr::Num(value) => Exact::from_f64(*value),
        Expr::Param => Some(parameter),
        Expr::Var => None,
        Expr::Neg(inner) => constant(inner, parameter)?.checked_neg(),
        Expr::Bin(Op::Add, left, right) => {
            constant(left, parameter)?.checked_add(constant(right, parameter)?)
        }
        Expr::Bin(Op::Sub, left, right) => {
            constant(left, parameter)?.checked_sub(constant(right, parameter)?)
        }
        Expr::Bin(Op::Mul, left, right) => {
            constant(left, parameter)?.checked_mul(constant(right, parameter)?)
        }
        Expr::Bin(Op::Div, left, right) => {
            constant(left, parameter)?.checked_div(constant(right, parameter)?)
        }
        Expr::Call(Func::Sqrt, arg) => constant(arg, parameter)?.checked_sqrt(),
        Expr::Call(_, _) | Expr::Bin(Op::Pow, _, _) | Expr::PairCall(_, _, _) => None,
    }
}

fn state(x_osc: &Oscillator, y_osc: &Oscillator, t: f64) -> State {
    let (x, vx) = sample(x_osc, t);
    let (y, vy) = sample(y_osc, t);
    State { x, y, vx, vy }
}

fn sample(osc: &Oscillator, t: f64) -> (f64, f64) {
    let omega = osc.omega.to_f64();
    let phase = osc.phase.to_f64() + omega * t;
    let amp = osc.amp.to_f64();
    match osc.kind {
        Func::Cos => (amp * phase.cos(), -amp * omega * phase.sin()),
        Func::Sin => (amp * phase.sin(), amp * omega * phase.cos()),
        _ => (f64::NAN, f64::NAN),
    }
}

fn checkpoint(x_osc: &Oscillator, y_osc: &Oscillator, start: State, t: f64) -> ClosureCheckpoint {
    let here = state(x_osc, y_osc, t);
    let position = near(here.x, start.x) && near(here.y, start.y);
    let velocity = near(here.vx, start.vx) && near(here.vy, start.vy);
    ClosureCheckpoint {
        t,
        position_returns: position,
        state_returns: position && velocity,
    }
}

fn near(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1e-8 * scale
}

fn whole_periods_in(span: f64, period: f64) -> Option<u32> {
    if !(span.is_finite() && period.is_finite()) || span <= 0.0 || period <= 0.0 {
        return None;
    }
    let ratio = span / period;
    let rounded = ratio.round();
    if rounded < 1.0 || (ratio - rounded).abs() > 1e-8 {
        return None;
    }
    let count = rounded as u32;
    (count as f64 == rounded).then_some(count)
}

fn integer_cycles(frequency: &Frequency, period: f64) -> Option<i64> {
    let cycles = frequency.to_f64() * period;
    let rounded = cycles.round();
    if (cycles - rounded).abs() > 1e-8 {
        return None;
    }
    if rounded < i64::MIN as f64 || rounded > i64::MAX as f64 {
        return None;
    }
    Some(rounded as i64)
}

/// Cycles per unit t: `(num/den) * sqrt(rad)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frequency {
    num: i64,
    den: u64,
    rad: u64,
}

impl Frequency {
    fn text(self) -> String {
        let body = format_ratio(self.num, self.den);
        if self.rad == 1 {
            body
        } else if self.num == 1 && self.den == 1 {
            format!("sqrt({})", self.rad)
        } else {
            format!("{body}*sqrt({})", self.rad)
        }
    }

    fn to_f64(self) -> f64 {
        (self.num as f64 / self.den as f64) * (self.rad as f64).sqrt()
    }
}

struct Period {
    text: String,
    value: f64,
}

fn frequency_cycles(omega: &Exact) -> Option<Frequency> {
    // f = omega / (2*pi). Requires omega to carry exactly one pi so the
    // quotient is a rational times a square root.
    if omega.pi != 1 || omega.is_zero() {
        return None;
    }
    let (num, den) = reduce(omega.num, omega.den.checked_mul(2)?);
    Some(Frequency {
        num,
        den,
        rad: omega.rad,
    })
}

fn common_period(x: &Frequency, y: &Frequency) -> Option<Period> {
    if x.rad != y.rad {
        return None;
    }
    if x.rad != 1 {
        return None;
    }
    let px = x.num.unsigned_abs();
    let py = y.num.unsigned_abs();
    if px == 0 || py == 0 {
        return None;
    }
    let den = lcm_u64(x.den, y.den)?;
    let gcd_nums = gcd_u64(px, py);
    let period_num = den / gcd_nums;
    let text = format_ratio(period_num as i64, 1);
    let value = period_num as f64;
    Some(Period { text, value })
}

fn format_ratio(num: i64, den: u64) -> String {
    if den == 1 {
        num.to_string()
    } else {
        format!("{num}/{den}")
    }
}

/// Exact constant: `(num/den) * pi^pi * sqrt(rad)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Exact {
    num: i64,
    den: u64,
    pi: i8,
    rad: u64,
}

impl Exact {
    const fn zero() -> Self {
        Self {
            num: 0,
            den: 1,
            pi: 0,
            rad: 1,
        }
    }

    const fn one() -> Self {
        Self {
            num: 1,
            den: 1,
            pi: 0,
            rad: 1,
        }
    }

    fn is_zero(self) -> bool {
        self.num == 0
    }

    fn from_int(value: i64) -> Self {
        Self {
            num: value,
            den: 1,
            pi: 0,
            rad: 1,
        }
        .normalized()
    }

    fn from_f64(value: f64) -> Option<Self> {
        if value.to_bits() == std::f64::consts::PI.to_bits() {
            return Some(Self {
                num: 1,
                den: 1,
                pi: 1,
                rad: 1,
            });
        }
        if !value.is_finite() {
            return None;
        }
        if value.fract() == 0.0 && (-1e12..=1e12).contains(&value) {
            return Some(Self::from_int(value as i64));
        }
        // Saved knob steps are quarters.
        let quarters = value * 4.0;
        if quarters.fract() == 0.0 && (-1e12..=1e12).contains(&quarters) {
            return Some(
                Self {
                    num: quarters as i64,
                    den: 4,
                    pi: 0,
                    rad: 1,
                }
                .normalized(),
            );
        }
        None
    }

    fn to_f64(self) -> f64 {
        let mut value = self.num as f64 / self.den as f64;
        match self.pi {
            0 => {}
            1 => value *= std::f64::consts::PI,
            -1 => value /= std::f64::consts::PI,
            _ => value *= std::f64::consts::PI.powi(i32::from(self.pi)),
        }
        if self.rad != 1 {
            value *= (self.rad as f64).sqrt();
        }
        value
    }

    fn normalized(self) -> Self {
        if self.num == 0 {
            return Self::zero();
        }
        let (square, rest) = split_square(self.rad);
        let mut num = self.num;
        let den = self.den;
        if square > 1
            && let Ok(factor) = i64::try_from(square)
        {
            num = num.saturating_mul(factor);
        }
        let (num, den) = reduce(num, den);
        Self {
            num,
            den,
            pi: self.pi,
            rad: rest.max(1),
        }
    }

    fn checked_neg(self) -> Option<Self> {
        Some(
            Self {
                num: self.num.checked_neg()?,
                ..self
            }
            .normalized(),
        )
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        if self.is_zero() {
            return Some(other);
        }
        if other.is_zero() {
            return Some(self);
        }
        if self.pi != other.pi || self.rad != other.rad {
            return None;
        }
        let den = lcm_u64(self.den, other.den)?;
        let left = (den / self.den) as i64;
        let right = (den / other.den) as i64;
        let num = self
            .num
            .checked_mul(left)?
            .checked_add(other.num.checked_mul(right)?)?;
        Some(
            Self {
                num,
                den,
                pi: self.pi,
                rad: self.rad,
            }
            .normalized(),
        )
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_add(other.checked_neg()?)
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        if self.is_zero() || other.is_zero() {
            return Some(Self::zero());
        }
        let num = self.num.checked_mul(other.num)?;
        let den = self.den.checked_mul(other.den)?;
        let rad = self.rad.checked_mul(other.rad)?;
        let pi = self.pi.checked_add(other.pi)?;
        Some(Self { num, den, pi, rad }.normalized())
    }

    fn checked_div(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Self::zero());
        }
        let num = self.num.checked_mul(other.den as i64)?;
        let den = self.den.checked_mul(other.num.unsigned_abs())?;
        let num = if other.num < 0 {
            num.checked_neg()?
        } else {
            num
        };
        let rad = self.rad.checked_mul(other.rad)?;
        // sqrt in the denominator: multiply num and den by sqrt(rad) via rad
        // in both, then the extra rad in den is a square and normalizes out
        // of the radicand after we put other.rad into den as an integer by
        // writing 1/sqrt(r) = sqrt(r)/r.
        let num = num.checked_mul(i64::try_from(other.rad).ok()?)?;
        let den = den.checked_mul(other.rad)?;
        let pi = self.pi.checked_sub(other.pi)?;
        Some(Self { num, den, pi, rad }.normalized())
    }

    fn checked_sqrt(self) -> Option<Self> {
        if self.pi != 0 || self.num < 0 {
            return None;
        }
        if self.is_zero() {
            return Some(Self::zero());
        }
        let n = (self.num as u64).checked_mul(self.rad)?;
        let (num_square, num_rest) = split_square(n);
        let (den_square, den_rest) = split_square(self.den);
        if den_rest != 1 {
            // sqrt(1/d) with d not square: write as sqrt(d)/d.
            let den = self.den.checked_mul(den_rest)?;
            let rad = num_rest.checked_mul(den_rest)?;
            return Some(
                Self {
                    num: i64::try_from(num_square).ok()?,
                    den,
                    pi: 0,
                    rad,
                }
                .normalized(),
            );
        }
        Some(
            Self {
                num: i64::try_from(num_square).ok()?,
                den: den_square,
                pi: 0,
                rad: num_rest,
            }
            .normalized(),
        )
    }
}

fn reduce(num: i64, den: u64) -> (i64, u64) {
    if num == 0 {
        return (0, 1);
    }
    if den == 0 {
        return (num, 1);
    }
    let g = gcd_u64(num.unsigned_abs(), den);
    (num / g as i64, den / g)
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let rest = left % right;
        left = right;
        right = rest;
    }
    left
}

fn lcm_u64(left: u64, right: u64) -> Option<u64> {
    if left == 0 || right == 0 {
        return Some(0);
    }
    let g = gcd_u64(left, right);
    left.checked_div(g)?.checked_mul(right)
}

fn split_square(mut value: u64) -> (u64, u64) {
    if value == 0 {
        return (0, 1);
    }
    let mut square = 1u64;
    let mut rest = 1u64;
    let mut p = 2u64;
    while p.saturating_mul(p) <= value {
        let mut exp = 0u32;
        while value.is_multiple_of(p) {
            value /= p;
            exp += 1;
        }
        for _ in 0..(exp / 2) {
            square = square.saturating_mul(p);
        }
        if exp % 2 == 1 {
            rest = rest.saturating_mul(p);
        }
        p += 1;
    }
    if value > 1 {
        rest = rest.saturating_mul(value);
    }
    (square, rest)
}

#[cfg(test)]
mod tests {
    use super::{PathClosure, PeriodicClosure};
    use crate::studio::StudioCreation;

    fn bundled(id: &str) -> StudioCreation {
        StudioCreation::from_capsule(id).expect("bundled experiment")
    }

    fn periodic(id: &str) -> PeriodicClosure {
        match PathClosure::of(&bundled(id)) {
            PathClosure::Periodic(periodic) => periodic,
            other => panic!("{id} should be periodic, got {other:?}"),
        }
    }

    #[test]
    fn a_graph_is_not_a_path_that_comes_home() {
        let graph = StudioCreation::new("sin(x)", -1.0, 1.0, 1.0).expect("graph");
        assert_eq!(PathClosure::of(&graph), PathClosure::Graph);
        assert!(PathClosure::of(&graph).report_lines().is_empty());
        assert_eq!(PathClosure::of(&graph).status_caption(), None);
    }

    #[test]
    fn full_return_has_period_twelve_with_seventeen_y_cycles() {
        let closure = periodic("full-return");
        assert_eq!(closure.period_text, "12");
        assert_eq!(closure.x_frequency_text, "1");
        assert_eq!(closure.y_frequency_text, "17/12");
        assert_eq!(closure.x_cycles, Some(12));
        assert_eq!(closure.y_cycles, Some(17));
        assert_eq!(closure.window_periods, Some(1));
        assert!(closure.window_end.state_returns);
        assert!(closure.window_end.position_returns);
    }

    #[test]
    fn same_place_half_period_is_the_deceptive_return() {
        let closure = periodic("same-place");
        assert_eq!(closure.period_text, "1");
        assert_eq!(closure.x_frequency_text, "2");
        assert_eq!(closure.y_frequency_text, "3");
        assert_eq!(closure.x_cycles, Some(2));
        assert_eq!(closure.y_cycles, Some(3));
        assert_eq!(closure.half_period.t, 0.5);
        assert!(
            closure.half_period.position_returns,
            "position at t=0.5 matches t=0"
        );
        assert!(
            !closure.half_period.state_returns,
            "velocity reverses at t=0.5"
        );
        assert!(closure.window_end.state_returns);
    }

    #[test]
    fn almost_home_has_no_period() {
        match PathClosure::of(&bundled("almost-home")) {
            PathClosure::Aperiodic(aperiodic) => {
                assert_eq!(aperiodic.x_frequency_text, "1");
                assert_eq!(aperiodic.y_frequency_text, "sqrt(2)");
                assert!(!aperiodic.window_end.state_returns);
            }
            other => panic!("almost-home should be aperiodic, got {other:?}"),
        }
    }

    #[test]
    fn an_unseen_eight_fifths_ratio_has_period_five() {
        let creation =
            StudioCreation::new_parametric("cos(2*pi*t)", "sin(2*pi*(8/5)*t)", 0.0, 5.0, 1.0)
                .expect("unseen ratio");
        match PathClosure::of(&creation) {
            PathClosure::Periodic(periodic) => {
                assert_eq!(periodic.period_text, "5");
                assert_eq!(periodic.x_frequency_text, "1");
                assert_eq!(periodic.y_frequency_text, "8/5");
                assert_eq!(periodic.x_cycles, Some(5));
                assert_eq!(periodic.y_cycles, Some(8));
                assert_eq!(periodic.window_periods, Some(1));
                assert!(periodic.window_end.state_returns);
            }
            other => panic!("8/5 should be periodic with T=5, got {other:?}"),
        }
    }

    #[test]
    fn save_and_reopen_preserves_full_return_closure() {
        let original = bundled("full-return");
        let reopened = StudioCreation::from_capsule(&original.to_num_file()).expect("reopen");
        assert_eq!(PathClosure::of(&original), PathClosure::of(&reopened));
        let from_link = StudioCreation::from_capsule(&original.to_link()).expect("link");
        assert_eq!(PathClosure::of(&original), PathClosure::of(&from_link));
    }

    #[test]
    fn status_captions_name_the_trial_without_a_lecture() {
        assert_eq!(
            PathClosure::of(&bundled("full-return")).status_caption(),
            Some("PERIOD 12  HALF: PLACE NOT STATE".to_string())
        );
        assert_eq!(
            PathClosure::of(&bundled("same-place")).status_caption(),
            Some("PERIOD 1  HALF: PLACE NOT STATE".to_string())
        );
        assert_eq!(
            PathClosure::of(&bundled("almost-home")).status_caption(),
            Some("NO PERIOD".to_string())
        );
        assert_eq!(
            PathClosure::of(&bundled("another-ratio")).status_caption(),
            Some("PERIOD 1".to_string())
        );
        let unseen =
            StudioCreation::new_parametric("cos(2*pi*t)", "sin(2*pi*(8/5)*t)", 0.0, 5.0, 1.0)
                .expect("unseen ratio");
        assert_eq!(
            PathClosure::of(&unseen).status_caption(),
            Some("PERIOD 5".to_string())
        );
    }
}
