//! Named sliders: extra parameters a Studio formula may bind.
//!
//! The parameter `a` is the original knob and stays a leaf of its own. A
//! formula may also name a few more identifiers. Each is a slider: a finite
//! value inside a declared closed range. Capsules write those bindings on
//! `NUMINOUS_STUDIO 6`. Older headers cannot smuggle a named knob by putting
//! it only in the expression.

use crate::studio::Expr;

/// The most extra sliders one formula may bind, not counting `a`.
pub const MAX_NAMED_SLIDERS: usize = 8;

/// The most characters a slider name may hold.
pub const MAX_SLIDER_NAME_CHARS: usize = 8;

/// Default lower edge of a named slider that did not declare a range.
pub const DEFAULT_SLIDER_MIN: f64 = -2.0;

/// Default upper edge of a named slider that did not declare a range.
pub const DEFAULT_SLIDER_MAX: f64 = 2.0;

/// Default value of a named slider, matching the default for `a`.
pub const DEFAULT_SLIDER_VALUE: f64 = 1.0;

const MAX_SLIDER_MAGNITUDE: f64 = 1e12;

/// One named slider: a value inside a declared closed range.
#[derive(Debug, Clone, PartialEq)]
pub struct StudioSlider {
    name: String,
    value: f64,
    min: f64,
    max: f64,
}

impl StudioSlider {
    /// Build a validated slider.
    ///
    /// # Errors
    /// Returns a message when the name is reserved or ill-formed, when any
    /// number is non-finite or too large, or when the value sits outside
    /// `min..=max`.
    pub fn new(name: impl Into<String>, value: f64, min: f64, max: f64) -> Result<Self, String> {
        let name = name.into();
        validate_slider_name(&name)?;
        validate_slider_numbers(&name, value, min, max)?;
        Ok(Self {
            name,
            value,
            min,
            max,
        })
    }

    /// A slider at the shared default value and range.
    ///
    /// # Errors
    /// Returns a message when the name is not a slider name.
    pub fn default_named(name: impl Into<String>) -> Result<Self, String> {
        Self::new(
            name,
            DEFAULT_SLIDER_VALUE,
            DEFAULT_SLIDER_MIN,
            DEFAULT_SLIDER_MAX,
        )
    }

    /// Identifier the formula uses.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current value.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Inclusive lower bound.
    #[must_use]
    pub const fn min(&self) -> f64 {
        self.min
    }

    /// Inclusive upper bound.
    #[must_use]
    pub const fn max(&self) -> f64 {
        self.max
    }

    /// Replace the value, keeping the range.
    ///
    /// # Errors
    /// Returns a message when the value is not finite, too large, or outside
    /// the declared range.
    pub fn with_value(self, value: f64) -> Result<Self, String> {
        Self::new(self.name, value, self.min, self.max)
    }

    /// File-field body: `name:value:min:max`.
    #[must_use]
    pub fn to_file_value(&self) -> String {
        format!("{}:{}:{}:{}", self.name, self.value, self.min, self.max)
    }

    /// CLI or protocol spec: `name=value:min:max`.
    #[must_use]
    pub fn to_spec(&self) -> String {
        format!("{}={}:{}:{}", self.name, self.value, self.min, self.max)
    }

    /// Move the value by `steps` quarter-units, clamped to the declared range.
    ///
    /// # Errors
    /// Returns a message when the stepped value is not a valid slider number.
    pub fn stepped(self, steps: i32) -> Result<Self, String> {
        let next = (self.value + f64::from(steps) * 0.25).clamp(self.min, self.max);
        self.with_value(next)
    }

    /// Parse a file-field body `name:value:min:max`.
    ///
    /// # Errors
    /// Returns a message when the line is malformed or the slider is invalid.
    pub fn from_file_value(value: &str) -> Result<Self, String> {
        let mut parts = value.split(':');
        let name = parts
            .next()
            .ok_or_else(|| "slider field is missing a name".to_string())?;
        let slider_value = parts
            .next()
            .ok_or_else(|| format!("slider '{name}' is missing a value"))?;
        let min = parts
            .next()
            .ok_or_else(|| format!("slider '{name}' is missing a minimum"))?;
        let max = parts
            .next()
            .ok_or_else(|| format!("slider '{name}' is missing a maximum"))?;
        if parts.next().is_some() {
            return Err(format!(
                "slider '{name}' needs name:value:min:max, not extra fields"
            ));
        }
        let slider_value = parse_number("value", slider_value)?;
        let min = parse_number("min", min)?;
        let max = parse_number("max", max)?;
        Self::new(name, slider_value, min, max)
    }

    /// CLI or protocol spec: `name=value` or `name=value:min:max`.
    ///
    /// An omitted range uses the shared default window. An omitted value uses
    /// the shared default value.
    ///
    /// # Errors
    /// Returns a message when the spec is malformed or the slider is invalid.
    pub fn from_spec(spec: &str) -> Result<Self, String> {
        let (name, rest) = spec
            .split_once('=')
            .ok_or_else(|| format!("slider spec '{spec}' needs name=value"))?;
        if rest.is_empty() {
            return Self::default_named(name);
        }
        if rest.contains(':') {
            let mut parts = rest.split(':');
            let value = parts
                .next()
                .ok_or_else(|| format!("slider '{name}' is missing a value"))?;
            let min = parts
                .next()
                .ok_or_else(|| format!("slider '{name}' is missing a minimum"))?;
            let max = parts
                .next()
                .ok_or_else(|| format!("slider '{name}' is missing a maximum"))?;
            if parts.next().is_some() {
                return Err(format!(
                    "slider '{name}' needs name=value:min:max, not extra fields"
                ));
            }
            Self::new(
                name,
                parse_number("value", value)?,
                parse_number("min", min)?,
                parse_number("max", max)?,
            )
        } else {
            Self::new(
                name,
                parse_number("value", rest)?,
                DEFAULT_SLIDER_MIN,
                DEFAULT_SLIDER_MAX,
            )
        }
    }
}

/// Whether `name` may be bound as a slider.
#[must_use]
pub fn is_slider_name(name: &str) -> bool {
    validate_slider_name(name).is_ok()
}

/// Names that already mean something in the Studio grammar.
#[must_use]
pub fn is_reserved_name(name: &str) -> bool {
    matches!(
        name,
        "x" | "t"
            | "a"
            | "pi"
            | "e"
            | "y"
            | "z"
            | "i"
            | "sin"
            | "cos"
            | "tan"
            | "exp"
            | "ln"
            | "log"
            | "abs"
            | "sqrt"
            | "floor"
            | "re"
            | "im"
            | "arg"
            | "conj"
            | "mod"
            | "min"
            | "max"
            | "euclid"
    )
}

/// Unique slider names in an expression, in first-seen order.
#[must_use]
pub fn collect_slider_names(expression: &Expr) -> Vec<String> {
    let mut names = Vec::new();
    walk_slider_names(expression, &mut names);
    names
}

/// The value bound to `name`, or the shared default when the table is silent.
#[must_use]
pub fn slider_value(name: &str, sliders: &[StudioSlider]) -> f64 {
    sliders
        .iter()
        .find(|slider| slider.name == name)
        .map_or(DEFAULT_SLIDER_VALUE, StudioSlider::value)
}

/// Merge caller-supplied sliders onto the names a formula actually uses.
///
/// Every name in the formula must appear at most once in `supplied`. Names
/// the formula uses but the caller omitted take the default value and range.
/// A supplied name the formula does not use is refused.
///
/// # Errors
/// Returns a message for an unknown name, a duplicate, or too many sliders.
pub fn bind_sliders(
    names: &[String],
    supplied: &[StudioSlider],
) -> Result<Vec<StudioSlider>, String> {
    if names.len() > MAX_NAMED_SLIDERS {
        return Err(format!(
            "a Studio formula may name at most {MAX_NAMED_SLIDERS} sliders"
        ));
    }
    let mut seen_supplied: Vec<&str> = Vec::new();
    for slider in supplied {
        if !names.iter().any(|name| name == slider.name()) {
            return Err(format!(
                "slider '{}' is not used in the formula",
                slider.name()
            ));
        }
        if seen_supplied.contains(&slider.name()) {
            return Err(format!("duplicate slider '{}'", slider.name()));
        }
        seen_supplied.push(slider.name());
    }
    names
        .iter()
        .map(|name| {
            supplied
                .iter()
                .find(|slider| slider.name() == name)
                .cloned()
                .map_or_else(|| StudioSlider::default_named(name), Ok)
        })
        .collect()
}

/// Keep existing sliders whose names the formula still uses, and fill defaults
/// for any new names.
///
/// # Errors
/// Returns a message when the formula names too many sliders.
pub fn retain_sliders(
    names: &[String],
    existing: &[StudioSlider],
) -> Result<Vec<StudioSlider>, String> {
    let kept: Vec<StudioSlider> = existing
        .iter()
        .filter(|slider| names.iter().any(|name| name == slider.name()))
        .cloned()
        .collect();
    bind_sliders(names, &kept)
}

/// Parse a list of CLI or protocol specs.
///
/// # Errors
/// Returns a message when any spec is malformed or invalid.
pub fn sliders_from_specs<I, S>(specs: I) -> Result<Vec<StudioSlider>, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    specs
        .into_iter()
        .map(|spec| StudioSlider::from_spec(spec.as_ref()))
        .collect()
}

fn walk_slider_names(expression: &Expr, names: &mut Vec<String>) {
    match expression {
        Expr::Slider(name) => {
            if !names.iter().any(|existing| existing == name) {
                names.push(name.clone());
            }
        }
        Expr::Neg(inner) | Expr::Call(_, inner) => walk_slider_names(inner, names),
        Expr::Bin(_, lhs, rhs) | Expr::PairCall(_, lhs, rhs) => {
            walk_slider_names(lhs, names);
            walk_slider_names(rhs, names);
        }
        Expr::Num(_) | Expr::Var | Expr::VarIm | Expr::Point | Expr::ImagUnit | Expr::Param => {}
    }
}

fn validate_slider_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("slider name is empty".to_string());
    }
    if name.chars().count() > MAX_SLIDER_NAME_CHARS {
        return Err(format!(
            "slider name '{name}' is too long; limit is {MAX_SLIDER_NAME_CHARS} characters"
        ));
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err("slider name is empty".to_string());
    };
    if !first.is_ascii_lowercase() || !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err(format!(
            "slider name '{name}' must be a lowercase letter followed by lowercase letters or digits"
        ));
    }
    if is_reserved_name(name) {
        return Err(format!(
            "'{name}' is not a slider; it already means something"
        ));
    }
    Ok(())
}

fn validate_slider_numbers(name: &str, value: f64, min: f64, max: f64) -> Result<(), String> {
    if !value.is_finite() || !min.is_finite() || !max.is_finite() {
        return Err(format!("slider '{name}' numbers must be finite"));
    }
    if value.abs() > MAX_SLIDER_MAGNITUDE
        || min.abs() > MAX_SLIDER_MAGNITUDE
        || max.abs() > MAX_SLIDER_MAGNITUDE
    {
        return Err(format!(
            "slider '{name}' numbers must be within {MAX_SLIDER_MAGNITUDE:e} in magnitude"
        ));
    }
    if max <= min {
        return Err(format!("slider '{name}' needs max > min"));
    }
    if value < min || value > max {
        return Err(format!(
            "slider '{name}' value {value} sits outside {min}..={max}"
        ));
    }
    Ok(())
}

fn parse_number(label: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("bad slider {label} '{value}'"))
}

#[cfg(test)]
mod tests {
    use super::{StudioSlider, bind_sliders, collect_slider_names, is_slider_name, slider_value};
    use crate::studio::parse;

    #[test]
    fn a_fresh_name_is_a_slider_and_a_is_not() {
        assert!(is_slider_name("b"));
        assert!(is_slider_name("freq"));
        assert!(is_slider_name("p2"));
        assert!(!is_slider_name("a"));
        assert!(!is_slider_name("x"));
        assert!(!is_slider_name("sin"));
        assert!(!is_slider_name("euclid"));
        assert!(!is_slider_name("B"));
        assert!(!is_slider_name(""));
    }

    #[test]
    fn parse_collects_extra_names_in_source_order() {
        let expr = parse("sin(q*x) + p").expect("parses");
        assert_eq!(collect_slider_names(&expr), ["q", "p"]);
        let again = parse("p*p + q").expect("parses");
        assert_eq!(collect_slider_names(&again), ["p", "q"]);
        assert!(collect_slider_names(&parse("sin(a*x)").expect("parses")).is_empty());
    }

    #[test]
    fn a_file_line_round_trips() {
        let slider = StudioSlider::new("b", 1.5, -4.0, 4.0).expect("slider");
        let text = slider.to_file_value();
        assert_eq!(text, "b:1.5:-4:4");
        assert_eq!(StudioSlider::from_file_value(&text).expect("parse"), slider);
    }

    #[test]
    fn a_spec_can_omit_the_range() {
        let slider = StudioSlider::from_spec("b=0.5").expect("spec");
        assert_eq!(slider.name(), "b");
        assert_eq!(slider.value(), 0.5);
        assert_eq!(slider.min(), -2.0);
        assert_eq!(slider.max(), 2.0);
        let ranged = StudioSlider::from_spec("p=3:1:8").expect("ranged");
        assert_eq!(ranged.value(), 3.0);
        assert_eq!(ranged.min(), 1.0);
        assert_eq!(ranged.max(), 8.0);
        assert_eq!(ranged.to_spec(), "p=3:1:8");
        let stepped = ranged.clone().stepped(1).expect("step");
        assert_eq!(stepped.value(), 3.25);
        let clamped = ranged.stepped(40).expect("clamp");
        assert_eq!(clamped.value(), 8.0);
    }

    #[test]
    fn bind_fills_defaults_and_refuses_strangers() {
        let names = vec!["p".to_string(), "q".to_string()];
        let bound = bind_sliders(&names, &[StudioSlider::new("q", 2.0, 1.0, 8.0).expect("q")])
            .expect("bind");
        assert_eq!(bound[0].name(), "p");
        assert_eq!(bound[0].value(), 1.0);
        assert_eq!(bound[1].name(), "q");
        assert_eq!(bound[1].value(), 2.0);
        let err = bind_sliders(&names, &[StudioSlider::default_named("r").expect("r")])
            .expect_err("stranger");
        assert!(err.contains("not used"), "{err}");
    }

    #[test]
    fn a_missing_table_entry_evaluates_at_one() {
        assert_eq!(slider_value("b", &[]), 1.0);
        let sliders = [StudioSlider::new("b", 0.25, -2.0, 2.0).expect("b")];
        assert_eq!(slider_value("b", &sliders), 0.25);
        assert_eq!(slider_value("q", &sliders), 1.0);
    }

    #[test]
    fn a_value_outside_the_range_is_refused() {
        let err = StudioSlider::new("b", 3.0, -2.0, 2.0).expect_err("outside");
        assert!(err.contains("outside"), "{err}");
    }
}
