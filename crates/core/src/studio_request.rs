//! Face-neutral Studio plot and melody requests.
//!
//! Faces parse command-line flags or protocol values, then hand the resulting
//! choices to these types. Defaults, validation, expression parsing, discovery
//! semantics, and execution live here so a formula means the same thing in
//! every interface.

use std::fmt;

use crate::sound::SoundSpec;
use crate::studio::{
    Expr, MAX_MELODY_NOTES, MAX_STUDIO_SOURCE_CHARS, PlotTextError, parse, plot_parsed_text,
    studio_auto_recipe, studio_recipe, studio_recipe_count, to_melody,
};

/// Default left edge of a Studio expression window.
pub const DEFAULT_STUDIO_XMIN: f64 = -std::f64::consts::TAU;

/// Default right edge of a Studio expression window.
pub const DEFAULT_STUDIO_XMAX: f64 = std::f64::consts::TAU;

/// Default value of the Studio parameter `a`.
pub const DEFAULT_STUDIO_PARAMETER: f64 = 1.0;

/// Default number of notes in a Studio melody.
///
/// Thirty-two retains enough samples to make curvature and inflection audible
/// without making a default protocol response or terminal export needlessly
/// long.
pub const DEFAULT_MELODY_NOTES: usize = 32;

/// Default width of a text Studio plot.
pub const DEFAULT_PLOT_WIDTH: usize = 72;

/// Default height of a text Studio plot.
pub const DEFAULT_PLOT_HEIGHT: usize = 26;

/// Maximum width or height accepted by a core Studio plot request.
pub const MAX_PLOT_EXTENT: usize = 4096;

/// Maximum cell allocation accepted by a core Studio plot request.
pub const MAX_PLOT_CELLS: usize = 16 * 1024 * 1024;

/// How a Studio plot expression was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlotDiscovery {
    /// The caller supplied the expression directly.
    Manual,
    /// The caller selected an entry by recipe index.
    Recipe,
    /// The caller supplied a seed without an explicit walk step.
    Random,
    /// The caller supplied a seed and an explicit walk step.
    Auto,
}

impl PlotDiscovery {
    /// Stable lowercase name used by structured interfaces.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Recipe => "recipe",
            Self::Random => "random",
            Self::Auto => "auto",
        }
    }
}

/// One mutually exclusive way to choose a Studio plot expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlotSource {
    /// A caller-authored expression.
    Manual(String),
    /// A curated recipe index, wrapped to the current bank.
    Recipe(u64),
    /// A deterministic walk through the curated recipe bank.
    Seeded {
        /// Starting seed.
        seed: u64,
        /// Optional explicit walk step. `None` is random discovery and `Some`
        /// is Auto discovery, including an explicitly supplied zero.
        auto_step: Option<u64>,
    },
}

/// A validated face-neutral Studio plot request.
#[derive(Debug, Clone, PartialEq)]
pub struct PlotRequest {
    source: String,
    expression: Expr,
    discovery: PlotDiscovery,
    recipe_index: Option<u64>,
    xmin: f64,
    xmax: f64,
    parameter: f64,
    width: usize,
    height: usize,
}

impl PlotRequest {
    /// Resolve and validate one plot request.
    ///
    /// Omitted numeric values use the shared Studio defaults.
    ///
    /// # Errors
    /// Returns a typed refusal for invalid source, window, parameter, or plot
    /// dimensions.
    pub fn new(
        source: PlotSource,
        xmin: Option<f64>,
        xmax: Option<f64>,
        parameter: Option<f64>,
        width: Option<usize>,
        height: Option<usize>,
    ) -> Result<Self, StudioRequestError> {
        let (source, discovery, recipe_index) = resolve_source(source);
        let expression = parse_source(&source)?;
        let (xmin, xmax, parameter) = resolve_window(xmin, xmax, parameter)?;
        let width = width.unwrap_or(DEFAULT_PLOT_WIDTH);
        let height = height.unwrap_or(DEFAULT_PLOT_HEIGHT);
        validate_plot_size(width, height)?;
        Ok(Self {
            source,
            expression,
            discovery,
            recipe_index,
            xmin,
            xmax,
            parameter,
            width,
            height,
        })
    }

    /// Execute the validated plot request.
    ///
    /// # Errors
    /// Returns [`StudioRequestError::Undefined`] when no finite sample exists
    /// across the requested window.
    pub fn execute(&self) -> Result<PlotResult, StudioRequestError> {
        plot_parsed_text(
            &self.expression,
            self.xmin,
            self.xmax,
            self.parameter,
            self.width,
            self.height,
        )
        .map(|(text, ymin, ymax)| PlotResult { text, ymin, ymax })
        .map_err(|error| match error {
            PlotTextError::InvalidGeometry => StudioRequestError::InvalidPlotSize {
                width: self.width,
                height: self.height,
            },
            PlotTextError::Undefined => StudioRequestError::Undefined,
        })
    }

    /// Resolved expression source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Discovery mode used to choose the expression.
    #[must_use]
    pub const fn discovery(&self) -> PlotDiscovery {
        self.discovery
    }

    /// Wrapped curated recipe index, when discovery used the recipe bank.
    #[must_use]
    pub const fn recipe_index(&self) -> Option<u64> {
        self.recipe_index
    }

    /// Left edge of the resolved expression window.
    #[must_use]
    pub const fn xmin(&self) -> f64 {
        self.xmin
    }

    /// Right edge of the resolved expression window.
    #[must_use]
    pub const fn xmax(&self) -> f64 {
        self.xmax
    }

    /// Resolved Studio parameter `a`.
    #[must_use]
    pub const fn parameter(&self) -> f64 {
        self.parameter
    }

    /// Resolved plot width.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Resolved plot height.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }
}

/// Result of executing a [`PlotRequest`].
#[derive(Debug, Clone, PartialEq)]
pub struct PlotResult {
    /// Rendered character plot without face-specific framing.
    pub text: String,
    /// Lowest finite value drawn.
    pub ymin: f64,
    /// Highest finite value drawn.
    pub ymax: f64,
}

/// A validated face-neutral Studio melody request.
#[derive(Debug, Clone, PartialEq)]
pub struct SingRequest {
    source: String,
    expression: Expr,
    xmin: f64,
    xmax: f64,
    parameter: f64,
    notes: usize,
}

impl SingRequest {
    /// Resolve and validate one melody request.
    ///
    /// Omitted numeric values use the shared Studio defaults.
    ///
    /// # Errors
    /// Returns a typed refusal for invalid source, window, parameter, or note
    /// count.
    pub fn new(
        source: impl Into<String>,
        xmin: Option<f64>,
        xmax: Option<f64>,
        parameter: Option<f64>,
        notes: Option<usize>,
    ) -> Result<Self, StudioRequestError> {
        let source = source.into();
        let expression = parse_source(&source)?;
        let (xmin, xmax, parameter) = resolve_window(xmin, xmax, parameter)?;
        let notes = notes.unwrap_or(DEFAULT_MELODY_NOTES);
        if !(1..=MAX_MELODY_NOTES).contains(&notes) {
            return Err(StudioRequestError::InvalidNoteCount {
                count: notes,
                maximum: MAX_MELODY_NOTES,
            });
        }
        Ok(Self {
            source,
            expression,
            xmin,
            xmax,
            parameter,
            notes,
        })
    }

    /// Execute the validated melody request.
    ///
    /// # Errors
    /// Returns [`StudioRequestError::Undefined`] when no finite sample exists
    /// across the requested window.
    pub fn execute(&self) -> Result<SoundSpec, StudioRequestError> {
        let spec = to_melody(
            &self.expression,
            self.xmin,
            self.xmax,
            self.notes,
            self.parameter,
        );
        if spec.notes.is_empty() {
            Err(StudioRequestError::Undefined)
        } else {
            Ok(spec)
        }
    }

    /// Resolved expression source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Left edge of the resolved expression window.
    #[must_use]
    pub const fn xmin(&self) -> f64 {
        self.xmin
    }

    /// Right edge of the resolved expression window.
    #[must_use]
    pub const fn xmax(&self) -> f64 {
        self.xmax
    }

    /// Resolved Studio parameter `a`.
    #[must_use]
    pub const fn parameter(&self) -> f64 {
        self.parameter
    }

    /// Resolved melody note count.
    #[must_use]
    pub const fn notes(&self) -> usize {
        self.notes
    }
}

/// Why a face-neutral Studio request was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioRequestError {
    /// Expression text was empty, oversized, or failed to parse.
    InvalidSource(String),
    /// An expression-window endpoint was not finite.
    NonFiniteWindow,
    /// The right endpoint did not exceed the left endpoint.
    InvalidWindow,
    /// Parameter `a` was not finite.
    NonFiniteParameter,
    /// Plot dimensions were below two cells on either axis.
    InvalidPlotSize {
        /// Requested width.
        width: usize,
        /// Requested height.
        height: usize,
    },
    /// A plot extent or total cell allocation exceeded the core limit.
    PlotTooLarge {
        /// Requested width.
        width: usize,
        /// Requested height.
        height: usize,
    },
    /// A melody note count was outside the core domain bound.
    InvalidNoteCount {
        /// Requested number of notes.
        count: usize,
        /// Largest accepted number of notes.
        maximum: usize,
    },
    /// No finite function sample existed in the requested window.
    Undefined,
}

impl fmt::Display for StudioRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSource(message) => formatter.write_str(message),
            Self::NonFiniteWindow => formatter.write_str("need finite xmin and xmax"),
            Self::InvalidWindow => formatter.write_str("need xmax > xmin"),
            Self::NonFiniteParameter => formatter.write_str("need finite a"),
            Self::InvalidPlotSize { .. } => formatter.write_str("need width >= 2 and height >= 2"),
            Self::PlotTooLarge { width, height } => write!(
                formatter,
                "plot size {width}x{height} exceeds the core allocation limit"
            ),
            Self::InvalidNoteCount { count, maximum } => write!(
                formatter,
                "note count {count} must be between 1 and {maximum}"
            ),
            Self::Undefined => formatter.write_str("the function is undefined across this range"),
        }
    }
}

impl std::error::Error for StudioRequestError {}

fn resolve_source(source: PlotSource) -> (String, PlotDiscovery, Option<u64>) {
    match source {
        PlotSource::Manual(source) => (source, PlotDiscovery::Manual, None),
        PlotSource::Recipe(index) => {
            let wrapped = index % studio_recipe_count() as u64;
            (
                studio_recipe(index).to_string(),
                PlotDiscovery::Recipe,
                Some(wrapped),
            )
        }
        PlotSource::Seeded { seed, auto_step } => {
            let step = auto_step.unwrap_or(0);
            let wrapped = seed.wrapping_add(step) % studio_recipe_count() as u64;
            (
                studio_auto_recipe(seed, step).to_string(),
                if auto_step.is_some() {
                    PlotDiscovery::Auto
                } else {
                    PlotDiscovery::Random
                },
                Some(wrapped),
            )
        }
    }
}

fn parse_source(source: &str) -> Result<Expr, StudioRequestError> {
    if source.is_empty() {
        return Err(StudioRequestError::InvalidSource(
            "Studio expression is empty".to_string(),
        ));
    }
    if source.chars().count() > MAX_STUDIO_SOURCE_CHARS {
        return Err(StudioRequestError::InvalidSource(format!(
            "Studio expression is too long; limit is {MAX_STUDIO_SOURCE_CHARS} characters"
        )));
    }
    parse(source).map_err(StudioRequestError::InvalidSource)
}

fn resolve_window(
    xmin: Option<f64>,
    xmax: Option<f64>,
    parameter: Option<f64>,
) -> Result<(f64, f64, f64), StudioRequestError> {
    let xmin = xmin.unwrap_or(DEFAULT_STUDIO_XMIN);
    let xmax = xmax.unwrap_or(DEFAULT_STUDIO_XMAX);
    let parameter = parameter.unwrap_or(DEFAULT_STUDIO_PARAMETER);
    if !xmin.is_finite() || !xmax.is_finite() {
        return Err(StudioRequestError::NonFiniteWindow);
    }
    if xmax <= xmin {
        return Err(StudioRequestError::InvalidWindow);
    }
    if !parameter.is_finite() {
        return Err(StudioRequestError::NonFiniteParameter);
    }
    Ok((xmin, xmax, parameter))
}

fn validate_plot_size(width: usize, height: usize) -> Result<(), StudioRequestError> {
    if width < 2 || height < 2 {
        return Err(StudioRequestError::InvalidPlotSize { width, height });
    }
    if width > MAX_PLOT_EXTENT
        || height > MAX_PLOT_EXTENT
        || width.saturating_mul(height) > MAX_PLOT_CELLS
    {
        return Err(StudioRequestError::PlotTooLarge { width, height });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_defaults_are_one_shared_contract() {
        let request = PlotRequest::new(
            PlotSource::Manual("sin(a*x)".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("default request");
        assert_eq!(request.xmin(), DEFAULT_STUDIO_XMIN);
        assert_eq!(request.xmax(), DEFAULT_STUDIO_XMAX);
        assert_eq!(request.parameter(), DEFAULT_STUDIO_PARAMETER);
        assert_eq!(request.width(), DEFAULT_PLOT_WIDTH);
        assert_eq!(request.height(), DEFAULT_PLOT_HEIGHT);
        assert_eq!(request.discovery(), PlotDiscovery::Manual);
        assert!(request.execute().expect("plot").text.contains('#'));
    }

    #[test]
    fn recipe_and_seed_discovery_are_exact_and_wrapped() {
        let count = studio_recipe_count() as u64;
        let recipe = PlotRequest::new(
            PlotSource::Recipe(count + 2),
            None,
            None,
            None,
            Some(24),
            Some(8),
        )
        .expect("recipe");
        assert_eq!(recipe.discovery(), PlotDiscovery::Recipe);
        assert_eq!(recipe.recipe_index(), Some(2));
        assert_eq!(recipe.source(), studio_recipe(2));

        let random = PlotRequest::new(
            PlotSource::Seeded {
                seed: 7,
                auto_step: None,
            },
            None,
            None,
            None,
            Some(24),
            Some(8),
        )
        .expect("random");
        let auto = PlotRequest::new(
            PlotSource::Seeded {
                seed: 7,
                auto_step: Some(0),
            },
            None,
            None,
            None,
            Some(24),
            Some(8),
        )
        .expect("auto");
        assert_eq!(random.source(), auto.source());
        assert_eq!(random.discovery(), PlotDiscovery::Random);
        assert_eq!(auto.discovery(), PlotDiscovery::Auto);
    }

    #[test]
    fn sing_defaults_are_one_shared_contract() {
        let request = SingRequest::new("sin(a*x)", None, None, None, None).expect("request");
        assert_eq!(request.xmin(), DEFAULT_STUDIO_XMIN);
        assert_eq!(request.xmax(), DEFAULT_STUDIO_XMAX);
        assert_eq!(request.parameter(), DEFAULT_STUDIO_PARAMETER);
        assert_eq!(request.notes(), DEFAULT_MELODY_NOTES);
        assert_eq!(request.execute().expect("melody").notes.len(), 32);
    }

    #[test]
    fn invalid_requests_fail_before_execution() {
        assert!(matches!(
            SingRequest::new("x", Some(f64::NAN), None, None, None),
            Err(StudioRequestError::NonFiniteWindow)
        ));
        assert!(matches!(
            SingRequest::new("x", Some(1.0), Some(1.0), None, None),
            Err(StudioRequestError::InvalidWindow)
        ));
        assert!(matches!(
            SingRequest::new("x", None, None, Some(f64::INFINITY), None),
            Err(StudioRequestError::NonFiniteParameter)
        ));
        assert!(matches!(
            SingRequest::new("x", None, None, None, Some(0)),
            Err(StudioRequestError::InvalidNoteCount { .. })
        ));
        assert!(matches!(
            PlotRequest::new(
                PlotSource::Manual("x".to_string()),
                None,
                None,
                None,
                Some(1),
                Some(8)
            ),
            Err(StudioRequestError::InvalidPlotSize { .. })
        ));
        assert!(matches!(
            PlotRequest::new(
                PlotSource::Manual("x".to_string()),
                None,
                None,
                None,
                Some(MAX_PLOT_EXTENT),
                Some(MAX_PLOT_EXTENT + 1)
            ),
            Err(StudioRequestError::PlotTooLarge { .. })
        ));
    }

    #[test]
    fn undefined_functions_fail_consistently() {
        let plot = PlotRequest::new(
            PlotSource::Manual("sqrt(0-1)".to_string()),
            None,
            None,
            None,
            Some(24),
            Some(8),
        )
        .expect("valid expression");
        assert_eq!(plot.execute(), Err(StudioRequestError::Undefined));

        let sing =
            SingRequest::new("sqrt(0-1)", None, None, None, Some(8)).expect("valid expression");
        assert_eq!(sing.execute(), Err(StudioRequestError::Undefined));
    }

    #[test]
    fn source_limits_apply_before_parser_work_and_whitespace_remains_valid() {
        let oversized = "x".repeat(MAX_STUDIO_SOURCE_CHARS + 1);
        assert!(matches!(
            SingRequest::new(oversized, None, None, None, None),
            Err(StudioRequestError::InvalidSource(_))
        ));
        assert!(SingRequest::new("x\n", None, None, None, None).is_ok());
    }
}
