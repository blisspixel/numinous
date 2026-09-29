//! A symbolic slope beside one open Studio graph.
//!
//! The first reading rewrites `sin(a*x)` to `a*cos(a*x)`. The capsule keeps
//! the player's source. Samples of the rewritten source are checked, in
//! tests, against an independent slope of `sin(a*x)`. `floor`, `mod`,
//! `min`, `max`, `euclid`, `pat`, and `note` are refused. Every other
//! graph in this slice is left without a reading.

use crate::studio::{Expr, Func, Op, PairFunc, StudioCreation, StudioKind, eval, parse};

/// Source text of the derivative this slice knows.
const DERIVATIVE_SOURCE: &str = "a*cos(a*x)";

/// The rewritten source and the expression parsed from it.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphDerivative {
    /// Canonical derivative source. This slice always uses `a*cos(a*x)`.
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
    /// Read one parsed graph. Refusal wins over the sine match.
    ///
    /// `sin(a*x)` and `sin(x*a)` return the derivative. A refused construct
    /// anywhere in the tree returns [`Self::Refused`]. Every other shape
    /// returns none: absence is not a secant and not a guess.
    #[must_use]
    pub fn of_expression(expr: &Expr) -> Option<Self> {
        if refuses_slope(expr) {
            return Some(Self::Refused);
        }
        if is_sin_parameter_times_variable(expr) {
            return derivative();
        }
        None
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

fn derivative() -> Option<GraphSlope> {
    let expression = parse(DERIVATIVE_SOURCE).ok()?;
    Some(GraphSlope::Derivative(GraphDerivative {
        source: DERIVATIVE_SOURCE.to_string(),
        expression,
    }))
}

fn is_sin_parameter_times_variable(expr: &Expr) -> bool {
    let Expr::Call(Func::Sin, argument) = expr else {
        return false;
    };
    let Expr::Bin(Op::Mul, left, right) = argument.as_ref() else {
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

#[cfg(test)]
mod tests {
    use super::{DERIVATIVE_SOURCE, GraphSlope, central_slope};
    use crate::field::FieldReading;
    use crate::studio::{
        StudioCreation, StudioScale, eval, graph_and_slope_melody, parse, to_melody,
    };

    fn reading(source: &str) -> Option<GraphSlope> {
        GraphSlope::of_expression(&parse(source).expect(source))
    }

    fn derivative(source: &str) -> GraphSlope {
        let slope = reading(source).expect("reading");
        assert!(
            matches!(slope, GraphSlope::Derivative(_)),
            "{source} should be the derivative"
        );
        let line = slope.report_lines().join("\n");
        assert_eq!(
            line,
            format!("slope basis=symbolic source={DERIVATIVE_SOURCE}")
        );
        slope
    }

    #[test]
    fn sin_a_x_rewrites_to_a_cos_a_x() {
        for source in ["sin(a*x)", "sin(x*a)", "sin( a * x )", "sin((a)*(x))"] {
            let GraphSlope::Derivative(found) = derivative(source) else {
                unreachable!("asserted above");
            };
            assert_eq!(found.source, DERIVATIVE_SOURCE);
            let again = parse(&found.source).expect("derivative source parses");
            assert_eq!(found.expression, again);
            assert_eq!(eval(&found.expression, 0.0, 2.0), 2.0);
        }
    }

    #[test]
    fn the_independent_slope_agrees_where_both_are_defined() {
        let graph = parse("sin(a*x)").expect("graph");
        let slope = parse(DERIVATIVE_SOURCE).expect("slope");
        for parameter in [1.0, 2.5, -1.5] {
            for x in [-1.0, -0.3, 0.0, 0.4, 1.0] {
                let numerical = central_slope(&graph, x, parameter, 1e-4).expect("finite slope");
                let symbolic = eval(&slope, x, parameter);
                assert!(
                    (numerical - symbolic).abs() < 1e-6,
                    "x={x} a={parameter}: numerical {numerical}, symbolic {symbolic}"
                );
            }
        }
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
        assert_eq!(found.source, DERIVATIVE_SOURCE);
    }

    #[test]
    fn unrecognized_graphs_are_absent_and_rhythm_graphs_are_refused() {
        for source in [
            "sin(x)",
            "sin(a*x)+0",
            "sin(a*x)+x/3",
            "x^2",
            "tan(x)",
            "ln(x)",
            "sqrt(x)",
            "abs(x)",
            "sin(b*x)",
            "cos(a*x)",
            "a*x",
        ] {
            assert!(reading(source).is_none(), "{source} is outside this slice");
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
        ] {
            let slope = reading(source).expect("refusal");
            assert!(matches!(slope, GraphSlope::Refused), "{source}");
            let line = slope.report_lines().join("\n");
            assert_eq!(line, "slope=refused");
            assert!(!line.contains(DERIVATIVE_SOURCE), "{line}");
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
        let slope = parse(DERIVATIVE_SOURCE).expect("slope");
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
