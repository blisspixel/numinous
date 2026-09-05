//! The Studio's expression engine: type a function, get a curve.
//!
//! A small, safe evaluator for single-variable expressions in `x`, the seed of
//! the creative graphing calculator (Tier 1 of the extensibility model in
//! `docs/ARCHITECTURE.md`: no arbitrary code, just math). It parses `sin(3*x) +
//! x^2/2` into an AST and evaluates it, so a plotter, a quiz, or an authored room
//! can all share one safe language. See `docs/PLAYFUL.md`.

use std::f64::consts::{E, PI};

use crate::sound::{Note, SoundSpec};

/// Maximum accepted Studio source length for share files and links.
pub const MAX_STUDIO_SOURCE_CHARS: usize = 512;

/// Maximum editable text for one scalar formula or one labeled parametric
/// pair. Each expression keeps the per-source cap above; this larger bound
/// only accounts for the second expression and the `x(t)` / `y(t)` labels.
pub const MAX_STUDIO_EDITOR_CHARS: usize = MAX_STUDIO_SOURCE_CHARS * 2 + 16;

/// Curated Formula Jam recipes shared by App Random/Auto, CLI, and MCP.
/// Random discovery draws only from this bank, never free assembly.
pub const STUDIO_RECIPES: &[&str] = &[
    "sin(a*x) + x/3",
    "sin(x) + sin(2*x)/2",
    "cos(x)*sin(a*x)",
    "abs(sin(x))",
    "x^2/12 - 1",
    "sin(x) + cos(a*x)/2",
    "sin(3*x)/3 + sin(x)",
    "cos(x + a) + x/8",
    "abs(x)/3 - cos(x)",
    "sin(a*x) * cos(x)",
    "x/4 + sin(2*x)",
    "cos(x)^2 - sin(x)^2",
    "floor(3*sin(x))/3",
    "mod(x + pi, 2*pi) - pi",
    "min(max(x, -2), 2)",
    "max(abs(x) - a, 0)",
];

/// How many curated recipes the bank holds.
#[must_use]
pub fn studio_recipe_count() -> usize {
    STUDIO_RECIPES.len()
}

/// Recipe at a wrapped index (App Random cursor and MCP/CLI discovery).
#[must_use]
pub fn studio_recipe(index: u64) -> &'static str {
    let count = STUDIO_RECIPES.len() as u64;
    debug_assert!(count > 0, "recipe bank is never empty");
    STUDIO_RECIPES[(index % count) as usize]
}

/// Deterministic Auto walk: bank entry after `step` advances from `seed`.
/// Stateless stand-in for the App's dwell-and-phrase Auto set.
#[must_use]
pub fn studio_auto_recipe(seed: u64, step: u64) -> &'static str {
    studio_recipe(seed.wrapping_add(step))
}

/// The most characters a capsule title or author may hold.
pub const MAX_META_TEXT_CHARS: usize = 64;
/// The most characters a capsule prose credit may hold.
///
/// Wider than a title so a fork's default sentence `After {title} by {author}`
/// always fits when both identity fields are at their own cap.
pub const MAX_CREDIT_CHARS: usize = 160;
/// The most bytes a recorded parent link may hold. Well under the whole-file
/// cap so a capsule with lineage still has room for its own expression.
const MAX_DESCENDS_BYTES: usize = 4096;

/// The mathematical form carried by a Studio capsule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioKind {
    /// One graph, `y = f(x)`.
    Graph,
    /// One planar path, `x = f(t)` and `y = g(t)`.
    Parametric,
}

impl StudioKind {
    /// Stable lowercase capsule and protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::Parametric => "parametric",
        }
    }

    #[must_use]
    fn parse(value: &str) -> Option<Self> {
        match value {
            "graph" => Some(Self::Graph),
            "parametric" => Some(Self::Parametric),
            _ => None,
        }
    }
}

/// A bounded pitch map stored with a Studio creation.
///
/// `Continuous` preserves every capsule and melody from versions 1 and 2.
/// The named scales quantize the same two-octave voice to semitone classes
/// relative to A, making the musical choice portable rather than face-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StudioScale {
    /// Preserve the continuous pitch curve.
    #[default]
    Continuous,
    /// Twelve equal-tempered pitch classes.
    Chromatic,
    /// Major scale: 0, 2, 4, 5, 7, 9, 11.
    Major,
    /// Natural minor scale: 0, 2, 3, 5, 7, 8, 10.
    Minor,
    /// Major pentatonic scale: 0, 2, 4, 7, 9.
    Pentatonic,
}

impl StudioScale {
    /// Stable lowercase capsule and protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Continuous => "continuous",
            Self::Chromatic => "chromatic",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Pentatonic => "pentatonic",
        }
    }

    /// Parse a stable scale name.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "continuous" => Some(Self::Continuous),
            "chromatic" => Some(Self::Chromatic),
            "major" => Some(Self::Major),
            "minor" => Some(Self::Minor),
            "pentatonic" => Some(Self::Pentatonic),
            _ => None,
        }
    }

    /// Next scale in the App's bounded performance cycle.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Continuous => Self::Chromatic,
            Self::Chromatic => Self::Major,
            Self::Major => Self::Minor,
            Self::Minor => Self::Pentatonic,
            Self::Pentatonic => Self::Continuous,
        }
    }
}

/// A shareable Studio program plus its viewing parameters. The second capsule
/// version adds identity: an optional title, author, Visual Era, and parent
/// link. The third adds one paired parametric form and a stored pitch map.
/// The fourth adds editable prose credit, the sentence a forker writes so
/// honor is not only a machine `descends` link.
///
/// The metadata is data-only, per `docs/EXTENSIBILITY.md` Tier 1: every field
/// is capped, character-whitelisted, and interpreted by trusted engine code.
/// Serialization writes the lowest header version that carries the content,
/// so a capsule without metadata stays a `NUMINOUS_STUDIO 1` file that older
/// builds keep opening.
#[derive(Debug, Clone, PartialEq)]
pub struct StudioCreation {
    source: String,
    second_source: Option<String>,
    xmin: f64,
    xmax: f64,
    a: f64,
    scale: StudioScale,
    title: Option<String>,
    author: Option<String>,
    credit: Option<String>,
    era: Option<crate::era::Era>,
    descends: Option<String>,
}

impl StudioCreation {
    /// Build a validated Studio creation.
    ///
    /// # Errors
    /// Returns a message if the source is empty, too large, contains control
    /// characters, does not parse, or if the range/parameter are not finite.
    pub fn new(source: impl Into<String>, xmin: f64, xmax: f64, a: f64) -> Result<Self, String> {
        let source = source.into().trim().to_string();
        validate_share_source(&source)?;
        parse(&source)?;
        validate_share_numbers(xmin, xmax, a)?;
        Ok(Self {
            source,
            second_source: None,
            xmin,
            xmax,
            a,
            scale: StudioScale::Continuous,
            title: None,
            author: None,
            credit: None,
            era: None,
            descends: None,
        })
    }

    /// Build a validated parametric creation over one bounded parameter
    /// interval. `t` is an alias for the expression engine's single input
    /// variable, so this adds a second expression without adding a second
    /// simultaneously varying variable or any executable surface.
    ///
    /// # Errors
    /// Returns a message if either source or any shared number is invalid.
    pub fn new_parametric(
        x_source: impl Into<String>,
        y_source: impl Into<String>,
        tmin: f64,
        tmax: f64,
        a: f64,
    ) -> Result<Self, String> {
        let x_source = x_source.into().trim().to_string();
        let y_source = y_source.into().trim().to_string();
        validate_share_source(&x_source)?;
        validate_share_source(&y_source)?;
        parse(&x_source)?;
        parse(&y_source)?;
        validate_share_numbers(tmin, tmax, a)?;
        Ok(Self {
            source: x_source,
            second_source: Some(y_source),
            xmin: tmin,
            xmax: tmax,
            a,
            scale: StudioScale::Continuous,
            title: None,
            author: None,
            credit: None,
            era: None,
            descends: None,
        })
    }

    /// Name the creation.
    ///
    /// # Errors
    /// Returns a message when the title is empty, longer than
    /// [`MAX_META_TEXT_CHARS`], or holds anything outside printable ASCII.
    pub fn with_title(mut self, title: &str) -> Result<Self, String> {
        let title = title.trim();
        validate_meta_text("title", title)?;
        self.title = Some(title.to_string());
        Ok(self)
    }

    /// Credit the creation.
    ///
    /// # Errors
    /// The same bounds as [`Self::with_title`].
    pub fn with_author(mut self, author: &str) -> Result<Self, String> {
        let author = author.trim();
        validate_meta_text("author", author)?;
        self.author = Some(author.to_string());
        Ok(self)
    }

    /// Take the name off, leaving the creation unnamed.
    ///
    /// Deleting a name is a decision, not an error, and it needs its own
    /// verb: [`Self::with_title`] refuses an empty string, so a face that
    /// mapped "the player cleared the field" onto "no title given" would
    /// silently keep the old name on a creation whose form showed none.
    #[must_use]
    pub fn without_title(mut self) -> Self {
        self.title = None;
        self
    }

    /// Take the signature off, leaving the creation unsigned.
    #[must_use]
    pub fn without_author(mut self) -> Self {
        self.author = None;
        self
    }

    /// Record the prose credit a forker writes for the parent.
    ///
    /// # Errors
    /// Returns a message when the credit is empty, longer than
    /// [`MAX_CREDIT_CHARS`], or holds anything outside printable ASCII.
    pub fn with_credit(mut self, credit: &str) -> Result<Self, String> {
        let credit = credit.trim();
        validate_credit_text(credit)?;
        self.credit = Some(credit.to_string());
        Ok(self)
    }

    /// Take the prose credit off, leaving the machine lineage to speak alone.
    #[must_use]
    pub fn without_credit(mut self) -> Self {
        self.credit = None;
        self
    }

    /// Apply an optional prose-credit edit without storing an empty field.
    ///
    /// An omitted edit keeps the current credit, including a fork suggestion.
    /// An explicitly empty or whitespace-only edit removes it. Other text is
    /// trimmed and validated by [`Self::with_credit`]. Machine lineage remains.
    ///
    /// # Errors
    /// Returns a message when nonempty credit is longer than
    /// [`MAX_CREDIT_CHARS`] or holds anything outside printable ASCII.
    pub fn with_credit_override(self, credit: Option<&str>) -> Result<Self, String> {
        match credit {
            None => Ok(self),
            Some(credit) if credit.trim().is_empty() => Ok(self.without_credit()),
            Some(credit) => self.with_credit(credit),
        }
    }

    /// The default sentence a fork offers: `After {title} by {author}`,
    /// omitting whichever identity the parent never recorded. Empty when the
    /// parent has neither, so a nameless parent is not invented.
    #[must_use]
    pub fn fork_credit_suggestion(&self) -> Option<String> {
        match (self.title(), self.author()) {
            (Some(title), Some(author)) => Some(format!("After {title} by {author}")),
            (Some(title), None) => Some(format!("After {title}")),
            (None, Some(author)) => Some(format!("After a creation by {author}")),
            (None, None) => None,
        }
    }

    fn with_suggested_fork_credit(self, parent: &Self) -> Self {
        match parent.fork_credit_suggestion() {
            Some(credit) => self.clone().with_credit(&credit).unwrap_or(self),
            None => self,
        }
    }

    /// Make a new creation that records this one as its parent.
    ///
    /// A fork keeps the parent's window, parameter, and recorded Visual Era,
    /// but it does not inherit the parent's title or author. Those fields
    /// identify the child and are present only when the caller supplies them.
    /// It offers prose credit from the parent's identity, which the caller
    /// may replace or clear. The source is copied unless the caller provides
    /// the remix expression.
    ///
    /// # Errors
    /// Returns a message when the replacement expression, child identity, or
    /// generated parent link does not satisfy the capsule bounds.
    pub fn fork(
        &self,
        source: Option<&str>,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Result<Self, String> {
        let mut child = match (&self.second_source, source) {
            (None, source) => {
                Self::new(source.unwrap_or(&self.source), self.xmin, self.xmax, self.a)?
            }
            (Some(y_source), None) => {
                Self::new_parametric(&self.source, y_source, self.xmin, self.xmax, self.a)?
            }
            (Some(_), Some(_)) => {
                return Err("a parametric fork needs both x(t) and y(t) replacements".to_string());
            }
        };
        child = child.with_scale(self.scale);
        if let Some(era) = self.era {
            child = child.with_era(era);
        }
        if let Some(title) = title {
            child = child.with_title(title)?;
        }
        if let Some(author) = author {
            child = child.with_author(author)?;
        }
        child = child.with_suggested_fork_credit(self);
        child.with_descends(&self.to_link())
    }

    /// Fork a parametric creation, optionally replacing both coordinate
    /// expressions. A pair is atomic: providing only one replacement is
    /// refused rather than silently mixing a new coordinate with an old one.
    ///
    /// # Errors
    /// Returns a message for a scalar parent, a partial replacement, or any
    /// invalid child field.
    pub fn fork_parametric(
        &self,
        x_source: Option<&str>,
        y_source: Option<&str>,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Result<Self, String> {
        let Some(parent_y) = self.second_source.as_deref() else {
            return Err("the parent is a graph, not a parametric pair".to_string());
        };
        let (x_source, y_source) = match (x_source, y_source) {
            (None, None) => (self.source.as_str(), parent_y),
            (Some(x_source), Some(y_source)) => (x_source, y_source),
            _ => return Err("replace both x(t) and y(t), or neither".to_string()),
        };
        let mut child = Self::new_parametric(x_source, y_source, self.xmin, self.xmax, self.a)?
            .with_scale(self.scale);
        if let Some(era) = self.era {
            child = child.with_era(era);
        }
        if let Some(title) = title {
            child = child.with_title(title)?;
        }
        if let Some(author) = author {
            child = child.with_author(author)?;
        }
        child = child.with_suggested_fork_credit(self);
        child.with_descends(&self.to_link())
    }

    /// Record the Visual Era the creation was made in.
    #[must_use]
    pub fn with_era(mut self, era: crate::era::Era) -> Self {
        self.era = Some(era);
        self
    }

    /// Store the pitch map used when the creation sings.
    #[must_use]
    pub fn with_scale(mut self, scale: StudioScale) -> Self {
        self.scale = scale;
        self
    }

    /// Record the creation this one descends from, as its native link.
    ///
    /// The link is validated by opening it: a parent that cannot be reopened
    /// is not lineage, it is decoration. Links themselves never carry
    /// `descends` (see [`Self::to_link`]), so validation cannot recurse.
    ///
    /// # Errors
    /// Returns a message when the link is oversized or does not describe a
    /// valid creation.
    pub fn with_descends(mut self, link: &str) -> Result<Self, String> {
        let link = link.trim();
        if link.len() > MAX_DESCENDS_BYTES {
            return Err(format!(
                "Studio descends link is too large; limit is {MAX_DESCENDS_BYTES} bytes"
            ));
        }
        // The link text itself must be line-safe: the file format is one
        // field per line, and a control byte the link parser happens to
        // tolerate would split the written line and make the saved capsule
        // unreadable by every reopen path.
        if link.chars().any(char::is_control) {
            return Err("Studio descends link cannot contain control characters".to_string());
        }
        Self::from_link(link)
            .map_err(|error| format!("Studio descends link does not reopen: {error}"))?;
        self.descends = Some(link.to_string());
        Ok(self)
    }

    /// The graph expression, or the parametric x-coordinate expression.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The parametric y-coordinate expression, when this is a pair.
    #[must_use]
    pub fn second_source(&self) -> Option<&str> {
        self.second_source.as_deref()
    }

    /// Whether this capsule is one graph or one parametric path.
    #[must_use]
    pub const fn kind(&self) -> StudioKind {
        if self.second_source.is_some() {
            StudioKind::Parametric
        } else {
            StudioKind::Graph
        }
    }

    /// One canonical editor-facing formula label.
    #[must_use]
    pub fn editor_source(&self) -> String {
        match &self.second_source {
            Some(y_source) => format!("x(t)={}; y(t)={y_source}", self.source),
            None => self.source.clone(),
        }
    }

    /// Left edge of the graph x range or parametric t range.
    #[must_use]
    pub fn xmin(&self) -> f64 {
        self.xmin
    }

    /// Right edge of the graph x range or parametric t range.
    #[must_use]
    pub fn xmax(&self) -> f64 {
        self.xmax
    }

    /// Shared value for the parameter `a`.
    #[must_use]
    pub fn a(&self) -> f64 {
        self.a
    }

    /// Stored musical pitch map.
    #[must_use]
    pub const fn scale(&self) -> StudioScale {
        self.scale
    }

    /// Parse this creation into its reusable graph or parametric program.
    ///
    /// # Errors
    /// Returns a parser diagnostic if a constructor invariant has regressed.
    pub fn program(&self) -> Result<StudioProgram, String> {
        StudioProgram::from_creation(self)
    }

    /// Render this exact creation as text, including a parametric path when
    /// the capsule carries two coordinate expressions.
    /// Graphs auto-scale y; parametric paths fit both coordinates with equal
    /// physical units, including the terminal character aspect.
    ///
    /// # Errors
    /// Returns a parser, geometry, or all-undefined diagnostic.
    pub fn plot_text(&self, width: usize, height: usize) -> Result<StudioPlot, String> {
        let program = self.program()?;
        plot_program_text(&program, self.xmin, self.xmax, self.a, width, height)
            .map_err(|error| error.message().to_string())
    }

    /// Render this exact creation's voice. A parametric creation sings its
    /// y-coordinate over `t`; the x-coordinate remains the visible path.
    #[must_use]
    pub fn to_melody(&self, notes: usize) -> SoundSpec {
        match self.program() {
            Ok(program) => to_melody_with_scale(
                program.voice_expression(),
                self.xmin,
                self.xmax,
                notes,
                self.a,
                self.scale,
            ),
            Err(_) => SoundSpec {
                duration: 0.12,
                notes: Vec::new(),
            },
        }
    }

    /// The creation's name, when it has one.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The creation's author, when recorded.
    #[must_use]
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// The prose credit recorded for this creation, when present.
    #[must_use]
    pub fn credit(&self) -> Option<&str> {
        self.credit.as_deref()
    }

    /// The Visual Era the creation was made in, when recorded.
    #[must_use]
    pub fn era(&self) -> Option<crate::era::Era> {
        self.era
    }

    /// The link of the creation this one descends from, when recorded.
    #[must_use]
    pub fn descends(&self) -> Option<&str> {
        self.descends.as_deref()
    }

    fn has_meta(&self) -> bool {
        self.title.is_some()
            || self.author.is_some()
            || self.era.is_some()
            || self.descends.is_some()
    }

    /// Serialize to a `.num` Studio file, in the lowest format version that
    /// carries the content. Plain graphs stay version 1, identity makes them
    /// version 2, a parametric pair or stored scale uses version 3, and
    /// prose credit uses version 4.
    #[must_use]
    pub fn to_num_file(&self) -> String {
        let version = if self.credit.is_some() {
            4
        } else if self.kind() == StudioKind::Parametric || self.scale != StudioScale::Continuous {
            3
        } else if self.has_meta() {
            2
        } else {
            1
        };
        let mut out = format!("NUMINOUS_STUDIO {version}\n");
        if version < 3 {
            out.push_str(&format!(
                "expr={}\nxmin={}\nxmax={}\na={}\n",
                self.source,
                format_share_number(self.xmin),
                format_share_number(self.xmax),
                format_share_number(self.a)
            ));
        } else {
            out.push_str(&format!("kind={}\n", self.kind().name()));
            match &self.second_source {
                Some(y_source) => out.push_str(&format!(
                    "xexpr={}\nyexpr={y_source}\ntmin={}\ntmax={}\na={}\n",
                    self.source,
                    format_share_number(self.xmin),
                    format_share_number(self.xmax),
                    format_share_number(self.a)
                )),
                None => out.push_str(&format!(
                    "expr={}\nxmin={}\nxmax={}\na={}\n",
                    self.source,
                    format_share_number(self.xmin),
                    format_share_number(self.xmax),
                    format_share_number(self.a)
                )),
            }
            out.push_str(&format!("scale={}\n", self.scale.name()));
        }
        if let Some(title) = &self.title {
            out.push_str(&format!("title={title}\n"));
        }
        if let Some(author) = &self.author {
            out.push_str(&format!("author={author}\n"));
        }
        if let Some(era) = self.era {
            out.push_str(&format!("era={}\n", era.name()));
        }
        if let Some(descends) = &self.descends {
            out.push_str(&format!("descends={descends}\n"));
        }
        if let Some(credit) = &self.credit {
            out.push_str(&format!("credit={credit}\n"));
        }
        out
    }

    /// Parse a `.num` Studio file, version 1 through 4.
    ///
    /// Version 1 rejects the metadata fields rather than ignoring them, so a
    /// file cannot claim the old header while smuggling new content. A header
    /// past version 4 is refused by name: a future capsule is a fact to
    /// report, not a guess to parse.
    ///
    /// # Errors
    /// Returns a message if the file is malformed or describes an invalid
    /// Studio expression.
    pub fn from_num_file(text: &str) -> Result<Self, String> {
        reject_oversized_share(text)?;
        let mut lines = text.lines();
        let version = match lines.next() {
            Some("NUMINOUS_STUDIO 1") => 1,
            Some("NUMINOUS_STUDIO 2") => 2,
            Some("NUMINOUS_STUDIO 3") => 3,
            Some("NUMINOUS_STUDIO 4") => 4,
            Some(header) if header.starts_with("NUMINOUS_STUDIO ") => {
                return Err(
                    "this Studio .num file is from a newer Numinous; update to open it".to_string(),
                );
            }
            _ => return Err("not a Numinous Studio .num file".to_string()),
        };
        let mut kind: Option<StudioKind> = None;
        let mut source: Option<String> = None;
        let mut x_source: Option<String> = None;
        let mut y_source: Option<String> = None;
        let mut xmin: Option<f64> = None;
        let mut xmax: Option<f64> = None;
        let mut tmin: Option<f64> = None;
        let mut tmax: Option<f64> = None;
        let mut a: Option<f64> = None;
        let mut scale: Option<StudioScale> = None;
        let mut title: Option<String> = None;
        let mut author: Option<String> = None;
        let mut credit: Option<String> = None;
        let mut era: Option<crate::era::Era> = None;
        let mut descends: Option<String> = None;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("bad Studio .num line '{line}'"))?;
            match key {
                "kind" | "xexpr" | "yexpr" | "tmin" | "tmax" | "scale" if version < 3 => {
                    return Err(format!(
                        "Studio .num field '{key}' needs a NUMINOUS_STUDIO 3 header"
                    ));
                }
                "kind" if kind.is_none() => {
                    kind = Some(
                        StudioKind::parse(value)
                            .ok_or_else(|| format!("unknown Studio kind '{value}'"))?,
                    );
                }
                "expr" if source.is_none() => source = Some(value.to_string()),
                "xexpr" if x_source.is_none() => x_source = Some(value.to_string()),
                "yexpr" if y_source.is_none() => y_source = Some(value.to_string()),
                "xmin" if xmin.is_none() => xmin = Some(parse_share_number("xmin", value)?),
                "xmax" if xmax.is_none() => xmax = Some(parse_share_number("xmax", value)?),
                "tmin" if tmin.is_none() => tmin = Some(parse_share_number("tmin", value)?),
                "tmax" if tmax.is_none() => tmax = Some(parse_share_number("tmax", value)?),
                "a" if a.is_none() => a = Some(parse_share_number("a", value)?),
                "scale" if scale.is_none() => {
                    scale = Some(
                        StudioScale::parse(value)
                            .ok_or_else(|| format!("unknown Studio scale '{value}'"))?,
                    );
                }
                "title" | "author" | "era" | "descends" if version < 2 => {
                    return Err(format!(
                        "Studio .num field '{key}' needs a NUMINOUS_STUDIO 2 header"
                    ));
                }
                "credit" if version < 4 => {
                    return Err(format!(
                        "Studio .num field '{key}' needs a NUMINOUS_STUDIO 4 header"
                    ));
                }
                "title" if title.is_none() => title = Some(value.to_string()),
                "author" if author.is_none() => author = Some(value.to_string()),
                "credit" if credit.is_none() => credit = Some(value.to_string()),
                "era" if era.is_none() => {
                    era = Some(
                        crate::era::Era::parse(value)
                            .ok_or_else(|| format!("unknown Studio era '{value}'"))?,
                    );
                }
                "descends" if descends.is_none() => descends = Some(value.to_string()),
                "kind" | "expr" | "xexpr" | "yexpr" | "xmin" | "xmax" | "tmin" | "tmax" | "a"
                | "scale" | "title" | "author" | "era" | "descends" | "credit" => {
                    return Err(format!("duplicate Studio .num field '{key}'"));
                }
                other => return Err(format!("unknown Studio .num field '{other}'")),
            }
        }
        let a = a.ok_or_else(|| "missing a".to_string())?;
        let mut creation = if version < 3 {
            Self::new(
                source.ok_or_else(|| "missing Studio expression".to_string())?,
                xmin.ok_or_else(|| "missing xmin".to_string())?,
                xmax.ok_or_else(|| "missing xmax".to_string())?,
                a,
            )?
        } else {
            let kind = kind.ok_or_else(|| "missing Studio kind".to_string())?;
            let scale = scale.ok_or_else(|| "missing Studio scale".to_string())?;
            let creation = match kind {
                StudioKind::Graph => {
                    if x_source.is_some() || y_source.is_some() || tmin.is_some() || tmax.is_some()
                    {
                        return Err("graph Studio capsule mixes parametric fields".to_string());
                    }
                    Self::new(
                        source.ok_or_else(|| "missing Studio expression".to_string())?,
                        xmin.ok_or_else(|| "missing xmin".to_string())?,
                        xmax.ok_or_else(|| "missing xmax".to_string())?,
                        a,
                    )?
                }
                StudioKind::Parametric => {
                    if source.is_some() || xmin.is_some() || xmax.is_some() {
                        return Err("parametric Studio capsule mixes graph fields".to_string());
                    }
                    Self::new_parametric(
                        x_source.ok_or_else(|| "missing parametric x expression".to_string())?,
                        y_source.ok_or_else(|| "missing parametric y expression".to_string())?,
                        tmin.ok_or_else(|| "missing tmin".to_string())?,
                        tmax.ok_or_else(|| "missing tmax".to_string())?,
                        a,
                    )?
                }
            };
            creation.with_scale(scale)
        };
        if let Some(title) = title {
            creation = creation.with_title(&title)?;
        }
        if let Some(author) = author {
            creation = creation.with_author(&author)?;
        }
        if let Some(credit) = credit {
            creation = creation.with_credit(&credit)?;
        }
        if let Some(era) = era {
            creation = creation.with_era(era);
        }
        if let Some(descends) = descends {
            creation = creation.with_descends(&descends)?;
        }
        Ok(creation)
    }

    /// Open portable capsule data supplied directly by a caller.
    ///
    /// Native links and `.num` text share one bounded input door. Filesystem
    /// paths are deliberately not accepted here: a face that owns path access
    /// must use [`Self::from_num_path`] and make that capability explicit.
    ///
    /// # Errors
    /// Returns a message when the input is neither a valid native link nor a
    /// valid `.num` document.
    pub fn from_capsule(input: &str) -> Result<Self, String> {
        if input.starts_with("numinous://") {
            Self::from_link(input)
        } else {
            Self::from_num_file(input)
        }
    }

    /// Load a `.num` file from disk without trusting its size.
    ///
    /// Reads at most one byte past [`MAX_SHARE_INPUT_BYTES`] so a huge file
    /// cannot drive a huge allocation, then hands the text to
    /// [`Self::from_num_file`], which re-checks the same bound at its own
    /// door; a parser must not trust its caller.
    ///
    /// # Errors
    /// Returns a [`NumFileError`] naming which door refused: the read, the
    /// byte cap, or the format.
    pub fn from_num_path(path: &std::path::Path) -> Result<Self, NumFileError> {
        use std::io::Read;
        let file = std::fs::File::open(path).map_err(NumFileError::Io)?;
        let mut text = String::new();
        file.take(MAX_SHARE_INPUT_BYTES as u64 + 1)
            .read_to_string(&mut text)
            .map_err(NumFileError::Io)?;
        if text.len() > MAX_SHARE_INPUT_BYTES {
            return Err(NumFileError::TooLarge);
        }
        Self::from_num_file(&text).map_err(NumFileError::Invalid)
    }

    /// Produce a native `numinous://` Studio link for this creation.
    ///
    /// A link carries the creation and its identity (title, author, era,
    /// credit) but never `descends`: lineage nests links inside links, and a
    /// handoff format that can nest itself is a growth format. Lineage lives
    /// in `.num` files, where the byte cap bounds it flat.
    #[must_use]
    pub fn to_link(&self) -> String {
        let mut link = if self.kind() == StudioKind::Graph && self.scale == StudioScale::Continuous
        {
            format!(
                "numinous://studio?expr={}&xmin={}&xmax={}&a={}",
                percent_encode(&self.source),
                format_share_number(self.xmin),
                format_share_number(self.xmax),
                format_share_number(self.a)
            )
        } else {
            let mut link = format!("numinous://studio?kind={}", self.kind().name());
            match &self.second_source {
                Some(y_source) => link.push_str(&format!(
                    "&xexpr={}&yexpr={}&tmin={}&tmax={}&a={}",
                    percent_encode(&self.source),
                    percent_encode(y_source),
                    format_share_number(self.xmin),
                    format_share_number(self.xmax),
                    format_share_number(self.a)
                )),
                None => link.push_str(&format!(
                    "&expr={}&xmin={}&xmax={}&a={}",
                    percent_encode(&self.source),
                    format_share_number(self.xmin),
                    format_share_number(self.xmax),
                    format_share_number(self.a)
                )),
            }
            link.push_str(&format!("&scale={}", self.scale.name()));
            link
        };
        if let Some(title) = &self.title {
            link.push_str(&format!("&title={}", percent_encode(title)));
        }
        if let Some(author) = &self.author {
            link.push_str(&format!("&author={}", percent_encode(author)));
        }
        if let Some(credit) = &self.credit {
            link.push_str(&format!("&credit={}", percent_encode(credit)));
        }
        if let Some(era) = self.era {
            link.push_str(&format!("&era={}", percent_encode(era.name())));
        }
        link
    }

    /// Parse a native `numinous://` Studio link.
    ///
    /// # Errors
    /// Returns a message if the link is malformed or describes an invalid
    /// Studio expression. A `descends` parameter is refused as unknown:
    /// links never carry lineage (see [`Self::to_link`]).
    pub fn from_link(link: &str) -> Result<Self, String> {
        reject_oversized_share(link)?;
        let query = link
            .strip_prefix("numinous://studio?")
            .or_else(|| link.strip_prefix("numinous://studio/?"))
            .ok_or_else(|| "not a Numinous Studio link".to_string())?;
        let mut kind: Option<StudioKind> = None;
        let mut source: Option<String> = None;
        let mut x_source: Option<String> = None;
        let mut y_source: Option<String> = None;
        let mut xmin: Option<f64> = None;
        let mut xmax: Option<f64> = None;
        let mut tmin: Option<f64> = None;
        let mut tmax: Option<f64> = None;
        let mut a: Option<f64> = None;
        let mut scale: Option<StudioScale> = None;
        let mut title: Option<String> = None;
        let mut author: Option<String> = None;
        let mut credit: Option<String> = None;
        let mut era: Option<crate::era::Era> = None;
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("bad Studio link parameter '{pair}'"))?;
            match key {
                "kind" if kind.is_none() => {
                    let decoded = percent_decode(value)?;
                    kind = Some(
                        StudioKind::parse(&decoded)
                            .ok_or_else(|| format!("unknown Studio kind '{decoded}'"))?,
                    );
                }
                "expr" if source.is_none() => source = Some(percent_decode(value)?),
                "xexpr" if x_source.is_none() => x_source = Some(percent_decode(value)?),
                "yexpr" if y_source.is_none() => y_source = Some(percent_decode(value)?),
                "xmin" if xmin.is_none() => xmin = Some(parse_share_number("xmin", value)?),
                "xmax" if xmax.is_none() => xmax = Some(parse_share_number("xmax", value)?),
                "tmin" if tmin.is_none() => tmin = Some(parse_share_number("tmin", value)?),
                "tmax" if tmax.is_none() => tmax = Some(parse_share_number("tmax", value)?),
                "a" if a.is_none() => a = Some(parse_share_number("a", value)?),
                "scale" if scale.is_none() => {
                    let decoded = percent_decode(value)?;
                    scale = Some(
                        StudioScale::parse(&decoded)
                            .ok_or_else(|| format!("unknown Studio scale '{decoded}'"))?,
                    );
                }
                "title" if title.is_none() => title = Some(percent_decode(value)?),
                "author" if author.is_none() => author = Some(percent_decode(value)?),
                "credit" if credit.is_none() => credit = Some(percent_decode(value)?),
                "era" if era.is_none() => {
                    let decoded = percent_decode(value)?;
                    era = Some(
                        crate::era::Era::parse(&decoded)
                            .ok_or_else(|| format!("unknown Studio era '{decoded}'"))?,
                    );
                }
                "kind" | "expr" | "xexpr" | "yexpr" | "xmin" | "xmax" | "tmin" | "tmax" | "a"
                | "scale" | "title" | "author" | "era" | "credit" => {
                    return Err(format!("duplicate Studio link field '{key}'"));
                }
                other => return Err(format!("unknown Studio link field '{other}'")),
            }
        }
        let a = a.ok_or_else(|| "missing a".to_string())?;
        let mut creation = match kind {
            None => {
                if x_source.is_some()
                    || y_source.is_some()
                    || tmin.is_some()
                    || tmax.is_some()
                    || scale.is_some()
                {
                    return Err("Studio link needs kind for version 3 fields".to_string());
                }
                Self::new(
                    source.ok_or_else(|| "missing Studio expression".to_string())?,
                    xmin.ok_or_else(|| "missing xmin".to_string())?,
                    xmax.ok_or_else(|| "missing xmax".to_string())?,
                    a,
                )?
            }
            Some(StudioKind::Graph) => {
                if x_source.is_some() || y_source.is_some() || tmin.is_some() || tmax.is_some() {
                    return Err("graph Studio link mixes parametric fields".to_string());
                }
                Self::new(
                    source.ok_or_else(|| "missing Studio expression".to_string())?,
                    xmin.ok_or_else(|| "missing xmin".to_string())?,
                    xmax.ok_or_else(|| "missing xmax".to_string())?,
                    a,
                )?
                .with_scale(scale.ok_or_else(|| "missing Studio scale".to_string())?)
            }
            Some(StudioKind::Parametric) => {
                if source.is_some() || xmin.is_some() || xmax.is_some() {
                    return Err("parametric Studio link mixes graph fields".to_string());
                }
                Self::new_parametric(
                    x_source.ok_or_else(|| "missing parametric x expression".to_string())?,
                    y_source.ok_or_else(|| "missing parametric y expression".to_string())?,
                    tmin.ok_or_else(|| "missing tmin".to_string())?,
                    tmax.ok_or_else(|| "missing tmax".to_string())?,
                    a,
                )?
                .with_scale(scale.ok_or_else(|| "missing Studio scale".to_string())?)
            }
        };
        if let Some(title) = title {
            creation = creation.with_title(&title)?;
        }
        if let Some(author) = author {
            creation = creation.with_author(&author)?;
        }
        if let Some(credit) = credit {
            creation = creation.with_credit(&credit)?;
        }
        if let Some(era) = era {
            creation = creation.with_era(era);
        }
        Ok(creation)
    }
}

/// The most bytes a shared `.num` file or `numinous://` link may hold. Four
/// fields, one a 512-char expression, need only a few hundred bytes; this cap
/// is generous headroom. A hostile input parser must bound its own byte count
/// rather than trust its caller, so this check lives at the door of both
/// import paths, not only in the faces that happen to read files. Public so a
/// face that reads a `.num` from disk can name the same number in its own
/// error copy instead of keeping a twin constant that drifts.
pub const MAX_SHARE_INPUT_BYTES: usize = 8 * 1024;

/// Why a `.num` file failed to load from disk.
///
/// Typed rather than a message because the faces speak differently about the
/// same refusal: the terminal face prints the path and the io error, the App
/// shows one short footer line. One loader with named reasons keeps the byte
/// cap and the read order in one place instead of one bounded reader per face,
/// which is the kind of second copy that drifts.
#[derive(Debug)]
pub enum NumFileError {
    /// The file could not be opened or read as UTF-8 text.
    Io(std::io::Error),
    /// The file holds more than [`MAX_SHARE_INPUT_BYTES`] bytes.
    TooLarge,
    /// The bytes read do not describe a valid Studio creation.
    Invalid(String),
}

/// The widest a share's x-range or parameter may reach. Well past any real
/// plot, and far below the point where the f64-to-pixel casts would matter,
/// so this is defense in depth, not a correctness fix.
const MAX_SHARE_MAGNITUDE: f64 = 1e12;

fn reject_oversized_share(text: &str) -> Result<(), String> {
    if text.len() > MAX_SHARE_INPUT_BYTES {
        return Err(format!(
            "Studio share is too large; limit is {MAX_SHARE_INPUT_BYTES} bytes"
        ));
    }
    Ok(())
}

fn validate_share_source(source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("Studio expression is empty".to_string());
    }
    if source.chars().count() > MAX_STUDIO_SOURCE_CHARS {
        return Err(format!(
            "Studio expression is too long; limit is {MAX_STUDIO_SOURCE_CHARS} characters"
        ));
    }
    if source.chars().any(char::is_control) {
        return Err("Studio expression cannot contain control characters".to_string());
    }
    Ok(())
}

/// Bound a capsule title or author: short, printable ASCII only, so a name
/// cannot steer a terminal, hide a control byte, or smuggle a line break
/// past the line-oriented file format.
fn validate_meta_text(name: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("Studio {name} is empty"));
    }
    if value.chars().count() > MAX_META_TEXT_CHARS {
        return Err(format!(
            "Studio {name} is too long; limit is {MAX_META_TEXT_CHARS} characters"
        ));
    }
    if !value.chars().all(|c| (' '..='~').contains(&c)) {
        return Err(format!("Studio {name} may hold only printable ASCII"));
    }
    Ok(())
}

fn validate_credit_text(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("Studio credit is empty".to_string());
    }
    if value.chars().count() > MAX_CREDIT_CHARS {
        return Err(format!(
            "Studio credit is too long; limit is {MAX_CREDIT_CHARS} characters"
        ));
    }
    if !value.chars().all(|c| (' '..='~').contains(&c)) {
        return Err("Studio credit may hold only printable ASCII".to_string());
    }
    Ok(())
}

fn validate_share_numbers(xmin: f64, xmax: f64, a: f64) -> Result<(), String> {
    if !xmin.is_finite() || !xmax.is_finite() || !a.is_finite() {
        return Err("Studio share numbers must be finite".to_string());
    }
    if xmin.abs() > MAX_SHARE_MAGNITUDE
        || xmax.abs() > MAX_SHARE_MAGNITUDE
        || a.abs() > MAX_SHARE_MAGNITUDE
    {
        return Err(format!(
            "Studio share numbers must be within {MAX_SHARE_MAGNITUDE:e} in magnitude"
        ));
    }
    if xmax <= xmin {
        return Err("Studio share needs xmax > xmin".to_string());
    }
    Ok(())
}

fn format_share_number(value: f64) -> String {
    value.to_string()
}

fn parse_share_number(name: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("bad Studio number for {name}: '{value}'"))
}

fn percent_encode(source: &str) -> String {
    let mut out = String::new();
    for byte in source.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn percent_decode(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hi = bytes
                    .get(i + 1)
                    .copied()
                    .ok_or_else(|| "truncated percent escape".to_string())?;
                let lo = bytes
                    .get(i + 2)
                    .copied()
                    .ok_or_else(|| "truncated percent escape".to_string())?;
                let value = hex_value(hi)
                    .and_then(|h| hex_value(lo).map(|l| h * 16 + l))
                    .ok_or_else(|| "bad percent escape".to_string())?;
                out.push(value);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| "Studio link is not valid UTF-8".to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// A parsed expression tree over one input variable, written as `x` for a
/// graph or `t` for a parametric coordinate.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal number (also holds folded constants like pi).
    Num(f64),
    /// The variable `x`.
    Var,
    /// The adjustable parameter `a`.
    Param,
    /// Unary negation.
    Neg(Box<Expr>),
    /// A binary operation.
    Bin(Op, Box<Expr>, Box<Expr>),
    /// A function call, e.g. `sin(...)`.
    Call(Func, Box<Expr>),
    /// A two-argument function call, e.g. `min(..., ...)`.
    PairCall(PairFunc, Box<Expr>, Box<Expr>),
}

/// A parsed Studio program ready for repeated drawing or sound generation.
#[derive(Debug, Clone, PartialEq)]
pub enum StudioProgram {
    /// One graph, `y = f(x)`.
    Graph {
        /// Canonical source without a `y =` label.
        source: String,
        /// Parsed expression.
        expression: Expr,
    },
    /// One planar path, `x = f(t)` and `y = g(t)`.
    Parametric {
        /// Canonical x-coordinate source without its label.
        x_source: String,
        /// Parsed x-coordinate expression.
        x_expression: Expr,
        /// Canonical y-coordinate source without its label.
        y_source: String,
        /// Parsed y-coordinate expression.
        y_expression: Expr,
    },
}

impl StudioProgram {
    /// Parse the App's one-line editor form. A scalar is ordinary expression
    /// text. A parametric pair is exactly `x(t)=...; y(t)=...`, with flexible
    /// ASCII whitespace around labels and the separator.
    ///
    /// # Errors
    /// Returns the same bounded expression diagnostics as a capsule import,
    /// plus a short pair-form diagnostic when only half a pair is present.
    pub fn from_editor(source: &str) -> Result<Self, String> {
        let source = source.trim();
        if source.chars().count() > MAX_STUDIO_EDITOR_CHARS {
            return Err(format!(
                "Studio editor text is too long; limit is {MAX_STUDIO_EDITOR_CHARS} characters"
            ));
        }
        if !source.contains(';') {
            validate_share_source(source)?;
            return Ok(Self::Graph {
                source: source.to_string(),
                expression: parse(source)?,
            });
        }
        let mut parts = source.split(';');
        let first = parts.next().unwrap_or_default().trim();
        let second = parts.next().unwrap_or_default().trim();
        if parts.next().is_some() {
            return Err("parametric Studio text needs exactly one ';' separator".to_string());
        }
        let x_source = strip_coordinate_label(first, 'x')?;
        let y_source = strip_coordinate_label(second, 'y')?;
        Self::parametric(&x_source, &y_source)
    }

    /// Parse a graph program.
    ///
    /// # Errors
    /// Returns an expression validation or parser diagnostic.
    pub fn graph(source: &str) -> Result<Self, String> {
        validate_share_source(source)?;
        Ok(Self::Graph {
            source: source.to_string(),
            expression: parse(source)?,
        })
    }

    /// Parse a parametric pair.
    ///
    /// # Errors
    /// Returns an expression validation or parser diagnostic for either
    /// coordinate.
    pub fn parametric(x_source: &str, y_source: &str) -> Result<Self, String> {
        validate_share_source(x_source)?;
        validate_share_source(y_source)?;
        Ok(Self::Parametric {
            x_source: x_source.to_string(),
            x_expression: parse(x_source)?,
            y_source: y_source.to_string(),
            y_expression: parse(y_source)?,
        })
    }

    /// Parse the program stored in one already-validated creation.
    ///
    /// # Errors
    /// Returns a parser diagnostic if an invariant has regressed.
    pub fn from_creation(creation: &StudioCreation) -> Result<Self, String> {
        match creation.second_source() {
            Some(y_source) => Self::parametric(creation.source(), y_source),
            None => Self::graph(creation.source()),
        }
    }

    /// Mathematical form of this program.
    #[must_use]
    pub const fn kind(&self) -> StudioKind {
        match self {
            Self::Graph { .. } => StudioKind::Graph,
            Self::Parametric { .. } => StudioKind::Parametric,
        }
    }

    /// Canonical editor text.
    #[must_use]
    pub fn editor_source(&self) -> String {
        match self {
            Self::Graph { source, .. } => source.clone(),
            Self::Parametric {
                x_source, y_source, ..
            } => format!("x(t)={x_source}; y(t)={y_source}"),
        }
    }

    /// Source fields without editor labels.
    #[must_use]
    pub fn sources(&self) -> (&str, Option<&str>) {
        match self {
            Self::Graph { source, .. } => (source, None),
            Self::Parametric {
                x_source, y_source, ..
            } => (x_source, Some(y_source)),
        }
    }

    /// Evaluate one graph input or parametric time into a planar point.
    #[must_use]
    pub fn point(&self, input: f64, a: f64) -> Option<(f64, f64)> {
        let point = match self {
            Self::Graph { expression, .. } => (input, eval(expression, input, a)),
            Self::Parametric {
                x_expression,
                y_expression,
                ..
            } => (eval(x_expression, input, a), eval(y_expression, input, a)),
        };
        (point.0.is_finite() && point.1.is_finite()).then_some(point)
    }

    /// Expression that carries pitch when this program sings. Graphs sing
    /// their y value; parametric paths sing their y coordinate over `t`.
    #[must_use]
    pub fn voice_expression(&self) -> &Expr {
        match self {
            Self::Graph { expression, .. } => expression,
            Self::Parametric { y_expression, .. } => y_expression,
        }
    }
}

fn strip_coordinate_label(source: &str, coordinate: char) -> Result<String, String> {
    let compact: String = source
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    let prefix = format!("{coordinate}(t)=");
    let Some(value) = compact.strip_prefix(&prefix) else {
        return Err(format!("parametric Studio text needs '{prefix}...'"));
    };
    if value.is_empty() {
        return Err(format!("parametric {coordinate}(t) expression is empty"));
    }
    Ok(value.to_string())
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Exponentiation.
    Pow,
}

/// A supported single-argument function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    /// Sine.
    Sin,
    /// Cosine.
    Cos,
    /// Tangent.
    Tan,
    /// Natural exponential.
    Exp,
    /// Natural logarithm.
    Ln,
    /// Absolute value.
    Abs,
    /// Square root.
    Sqrt,
    /// Greatest integer less than or equal to the argument.
    Floor,
}

/// A supported two-argument function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairFunc {
    /// Euclidean remainder. A finite result is nonnegative.
    Mod,
    /// The lesser of two defined values.
    Min,
    /// The greater of two defined values.
    Max,
}

/// Evaluate a parsed expression at variable `x` and parameter `a`.
#[must_use]
pub fn eval(expr: &Expr, x: f64, a: f64) -> f64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Var => x,
        Expr::Param => a,
        Expr::Neg(inner) => -eval(inner, x, a),
        Expr::Bin(op, lhs, rhs) => {
            let (lhs, rhs) = (eval(lhs, x, a), eval(rhs, x, a));
            match op {
                Op::Add => lhs + rhs,
                Op::Sub => lhs - rhs,
                Op::Mul => lhs * rhs,
                Op::Div => lhs / rhs,
                Op::Pow => lhs.powf(rhs),
            }
        }
        Expr::Call(func, arg) => {
            let arg = eval(arg, x, a);
            match func {
                Func::Sin => arg.sin(),
                Func::Cos => arg.cos(),
                Func::Tan => arg.tan(),
                Func::Exp => arg.exp(),
                Func::Ln => arg.ln(),
                Func::Abs => arg.abs(),
                Func::Sqrt => arg.sqrt(),
                Func::Floor => arg.floor(),
            }
        }
        Expr::PairCall(func, lhs, rhs) => {
            let (lhs, rhs) = (eval(lhs, x, a), eval(rhs, x, a));
            if lhs.is_nan() || rhs.is_nan() {
                return f64::NAN;
            }
            match func {
                PairFunc::Mod => lhs.rem_euclid(rhs),
                PairFunc::Min => lhs.min(rhs),
                PairFunc::Max => lhs.max(rhs),
            }
        }
    }
}

/// The most notes a melody may hold. Each note is a fixed slice of time and a
/// sample buffer, so an unbounded count (a hostile `--notes`) would drive an
/// unbounded allocation; this bounds it to a couple of minutes of audio while
/// staying far above any real curve's detail.
pub const MAX_MELODY_NOTES: usize = 512;

/// Turn an expression into a melody: sample `y = f(x)` across `[xmin, xmax]` and
/// map each value to a pitch, stepping through time. You hear the curve.
#[must_use]
pub fn to_melody(expr: &Expr, xmin: f64, xmax: f64, notes: usize, a: f64) -> SoundSpec {
    to_melody_with_scale(expr, xmin, xmax, notes, a, StudioScale::Continuous)
}

/// Turn one expression into a melody through a named, portable pitch map.
#[must_use]
pub fn to_melody_with_scale(
    expr: &Expr,
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    scale: StudioScale,
) -> SoundSpec {
    let notes = notes.clamp(1, MAX_MELODY_NOTES);
    let step = 0.12_f32;
    let denom = (notes as f64 - 1.0).max(1.0);
    let samples: Vec<f64> = (0..notes)
        .map(|i| eval(expr, xmin + (xmax - xmin) * i as f64 / denom, a))
        .filter(|y| y.is_finite())
        .collect();
    if samples.is_empty() {
        return SoundSpec {
            duration: step,
            notes: Vec::new(),
        };
    }
    let ymin = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let ymax = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let amplitude_scale = ymin.abs().max(ymax.abs()).max(1.0);
    let scaled_min = ymin / amplitude_scale;
    let span = (ymax / amplitude_scale - scaled_min).max(f64::EPSILON);
    let note_vec: Vec<Note> = samples
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            let norm = ((y / amplitude_scale - scaled_min) / span).clamp(0.0, 1.0) as f32;
            let semitones = quantized_semitones(norm * 24.0, scale);
            Note {
                freq: 220.0 * 2.0_f32.powf(semitones / 12.0),
                start: i as f32 * step,
                dur: step * 1.4,
                amp: 0.3,
            }
        })
        .collect();
    SoundSpec {
        duration: note_vec.len() as f32 * step + 0.3,
        notes: note_vec,
    }
}

fn quantized_semitones(value: f32, scale: StudioScale) -> f32 {
    if scale == StudioScale::Continuous {
        return value;
    }
    let classes: &[i32] = match scale {
        StudioScale::Continuous => unreachable!("returned above"),
        StudioScale::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        StudioScale::Major => &[0, 2, 4, 5, 7, 9, 11],
        StudioScale::Minor => &[0, 2, 3, 5, 7, 8, 10],
        StudioScale::Pentatonic => &[0, 2, 4, 7, 9],
    };
    let value = value.clamp(0.0, 24.0);
    (0..=2)
        .flat_map(|octave| {
            classes
                .iter()
                .map(move |class| (octave * 12 + class) as f32)
        })
        .filter(|candidate| *candidate <= 24.0)
        .min_by(|left, right| {
            (left - value)
                .abs()
                .total_cmp(&(right - value).abs())
                .then_with(|| left.total_cmp(right))
        })
        .unwrap_or(0.0)
}

/// Plot `source` as ASCII over `[xmin, xmax]` at parameter `a`, auto-scaling y.
/// Returns the picture and the y range it covered.
///
/// # Errors
/// Returns a message if the expression does not parse, the ranges are invalid,
/// or the function is undefined across the whole range.
pub fn plot_text(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    width: usize,
    height: usize,
) -> Result<(String, f64, f64), String> {
    let expr = parse(source)?;
    plot_parsed_text(&expr, xmin, xmax, a, width, height)
        .map_err(|error| error.message().to_string())
}

/// Bounded text rendering and its planar bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct StudioPlot {
    /// Rendered character path.
    pub text: String,
    /// Lowest finite x-coordinate drawn.
    pub xmin: f64,
    /// Highest finite x-coordinate drawn.
    pub xmax: f64,
    /// Lowest finite y-coordinate drawn.
    pub ymin: f64,
    /// Highest finite y-coordinate drawn.
    pub ymax: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlotTextError {
    InvalidGeometry,
    Undefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProgramPlotError {
    Sampling(PlotTextError),
    UnrepresentablePlanarRange,
}

impl ProgramPlotError {
    const fn message(self) -> &'static str {
        match self {
            Self::Sampling(error) => error.message(),
            Self::UnrepresentablePlanarRange => {
                "the planar coordinate range cannot be represented faithfully"
            }
        }
    }
}

fn plot_program_text(
    program: &StudioProgram,
    input_min: f64,
    input_max: f64,
    a: f64,
    width: usize,
    height: usize,
) -> Result<StudioPlot, ProgramPlotError> {
    if width < 2 || height < 2 || input_max <= input_min {
        return Err(ProgramPlotError::Sampling(PlotTextError::InvalidGeometry));
    }
    match program {
        StudioProgram::Graph { expression, .. } => {
            plot_parsed_text(expression, input_min, input_max, a, width, height)
                .map(|(text, ymin, ymax)| StudioPlot {
                    text,
                    xmin: input_min,
                    xmax: input_max,
                    ymin,
                    ymax,
                })
                .map_err(ProgramPlotError::Sampling)
        }
        StudioProgram::Parametric { .. } => {
            let sample_count = width.saturating_mul(4).clamp(64, 16_384);
            let denom = (sample_count - 1) as f64;
            let points: Vec<Option<(f64, f64)>> = (0..sample_count)
                .map(|index| {
                    let input = input_min + (input_max - input_min) * index as f64 / denom;
                    program.point(input, a)
                })
                .collect();
            let finite: Vec<(f64, f64)> = points.iter().flatten().copied().collect();
            if finite.is_empty() {
                return Err(ProgramPlotError::Sampling(PlotTextError::Undefined));
            }
            let xmin = finite
                .iter()
                .map(|point| point.0)
                .fold(f64::INFINITY, f64::min);
            let xmax = finite
                .iter()
                .map(|point| point.0)
                .fold(f64::NEG_INFINITY, f64::max);
            let ymin = finite
                .iter()
                .map(|point| point.1)
                .fold(f64::INFINITY, f64::min);
            let ymax = finite
                .iter()
                .map(|point| point.1)
                .fold(f64::NEG_INFINITY, f64::max);
            let mut canvas = crate::canvas::Canvas::new(width, height);
            let projection = crate::PlanarProjection::fit(
                &canvas,
                (0, 0, width, height),
                (xmin, xmax),
                (ymin, ymax),
            )
            .ok_or(ProgramPlotError::UnrepresentablePlanarRange)?;
            let mut previous: Option<(i32, i32)> = None;
            for point in points {
                let Some((sx, sy)) = point.and_then(|(x, y)| projection.point(x, y)) else {
                    previous = None;
                    continue;
                };
                use crate::surface::Surface;
                if let Some((px, py)) = previous {
                    canvas.line(px, py, sx, sy, '#');
                } else {
                    canvas.plot(sx, sy, '#');
                }
                previous = Some((sx, sy));
            }
            Ok(StudioPlot {
                text: canvas.to_text(),
                xmin,
                xmax,
                ymin,
                ymax,
            })
        }
    }
}

impl PlotTextError {
    const fn message(self) -> &'static str {
        match self {
            Self::InvalidGeometry => "need width >= 2, height >= 2, and xmax > xmin",
            Self::Undefined => "nothing to plot: the function is undefined across this range",
        }
    }
}

pub(crate) fn plot_parsed_text(
    expr: &Expr,
    xmin: f64,
    xmax: f64,
    a: f64,
    width: usize,
    height: usize,
) -> Result<(String, f64, f64), PlotTextError> {
    if width < 2 || height < 2 || xmax <= xmin {
        return Err(PlotTextError::InvalidGeometry);
    }
    let samples: Vec<(f64, f64)> = (0..width)
        .map(|i| {
            let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
            (x, eval(expr, x, a))
        })
        .filter(|(_, y)| y.is_finite())
        .collect();
    if samples.is_empty() {
        return Err(PlotTextError::Undefined);
    }
    let ymin = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let ymax = samples
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let yspan = (ymax - ymin).max(1e-9);

    let mut canvas = crate::canvas::Canvas::new(width, height);
    let mut previous: Option<(i32, i32)> = None;
    for &(x, y) in &samples {
        let sx = ((x - xmin) / (xmax - xmin) * (width as f64 - 1.0)) as i32;
        let sy = ((height as f64 - 1.0) - (y - ymin) / yspan * (height as f64 - 1.0)) as i32;
        if let Some((px, py)) = previous {
            use crate::surface::Surface;
            canvas.line(px, py, sx, sy, '#');
        }
        previous = Some((sx, sy));
    }
    Ok((canvas.to_text(), ymin, ymax))
}

/// The most tokens an expression may hold. A real formula is tiny; this only
/// bites pathological input (a million parentheses), and it bounds both the
/// token vector and the AST that grows from it. Checked before any recursion
/// so a hostile expression is rejected at the door, not mid-descent.
pub const MAX_EXPR_TOKENS: usize = 4096;

/// The deepest the recursive-descent parser may nest. Every `(`, every `^`,
/// and every leading `-` adds a level, so this caps stack growth from
/// crafted input. A stack overflow in Rust aborts the process uncatchably,
/// so this guard is load-bearing on the MCP surface, not a nicety: the
/// `plot_expression` and `sing_expression` tools parse agent-supplied text.
const MAX_PARSE_DEPTH: usize = 64;

/// Parse an expression in `x`, or return a human-readable error.
///
/// # Errors
/// Returns a message describing the first problem (too many tokens, nesting
/// too deep, unexpected token, unknown name, unbalanced parentheses, or
/// trailing input).
pub fn parse(source: &str) -> Result<Expr, String> {
    let tokenized = tokenize(source)?;
    let tokens = tokenized.tokens;
    if tokens.len() > MAX_EXPR_TOKENS {
        return Err(format!(
            "expression is too complex; limit is {MAX_EXPR_TOKENS} tokens"
        ));
    }
    let mut parser = Parser {
        tokens,
        columns: tokenized.columns,
        end_column: tokenized.end_column,
        pos: 0,
    };
    let expr = parser.expr(0)?;
    if parser.pos != parser.tokens.len() {
        return Err(format!(
            "unexpected trailing input at column {}",
            parser.current_column()
        ));
    }
    Ok(expr)
}

/// A token in an expression.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Comma,
    LParen,
    RParen,
}

impl Tok {
    fn diagnostic_name(&self) -> String {
        match self {
            Self::Num(_) => "number".to_string(),
            Self::Ident(name) => format!("name '{name}'"),
            Self::Plus => "'+'".to_string(),
            Self::Minus => "'-'".to_string(),
            Self::Star => "'*'".to_string(),
            Self::Slash => "'/'".to_string(),
            Self::Caret => "'^'".to_string(),
            Self::Comma => "','".to_string(),
            Self::LParen => "'('".to_string(),
            Self::RParen => "')'".to_string(),
        }
    }
}

/// Split `source` into tokens.
struct Tokenized {
    tokens: Vec<Tok>,
    columns: Vec<usize>,
    end_column: usize,
}

fn tokenize(source: &str) -> Result<Tokenized, String> {
    let mut tokens = Vec::new();
    let mut columns = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let value = text
                .parse::<f64>()
                .map_err(|_| format!("bad number '{text}' at column {}", start + 1))?;
            tokens.push(Tok::Num(value));
            columns.push(start + 1);
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            tokens.push(Tok::Ident(chars[start..i].iter().collect()));
            columns.push(start + 1);
        } else {
            let column = i + 1;
            tokens.push(match c {
                '+' => Tok::Plus,
                '-' => Tok::Minus,
                '*' => Tok::Star,
                '/' => Tok::Slash,
                '^' => Tok::Caret,
                ',' => Tok::Comma,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                other => {
                    return Err(format!("unexpected character '{other}' at column {column}"));
                }
            });
            columns.push(column);
            i += 1;
        }
    }
    Ok(Tokenized {
        tokens,
        columns,
        end_column: chars.len() + 1,
    })
}

/// A recursive-descent parser over a token slice.
struct Parser {
    tokens: Vec<Tok>,
    columns: Vec<usize>,
    end_column: usize,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn current_column(&self) -> usize {
        self.columns
            .get(self.pos)
            .copied()
            .unwrap_or(self.end_column)
    }

    fn expect_right_paren(&mut self, context: &str) -> Result<(), String> {
        let column = self.current_column();
        match self.peek() {
            Some(Tok::RParen) => {
                self.pos += 1;
                Ok(())
            }
            Some(token) => Err(format!(
                "expected ')' {context}at column {column}; found {}",
                token.diagnostic_name()
            )),
            None => Err(format!(
                "expression ended at column {column}; expected ')' {context}"
            )),
        }
    }

    fn expect_comma(&mut self, function: &str) -> Result<(), String> {
        let column = self.current_column();
        match self.peek() {
            Some(Tok::Comma) => {
                self.pos += 1;
                Ok(())
            }
            Some(token) => Err(format!(
                "expected ',' between arguments to {function} at column {column}; found {}",
                token.diagnostic_name()
            )),
            None => Err(format!(
                "expression ended at column {column}; expected ',' between arguments to {function}"
            )),
        }
    }

    /// Guard one level of recursion: fail before the stack, not with it. A
    /// crafted expression can nest arbitrarily deep through `(`, `^`, and
    /// unary `-`, and a Rust stack overflow aborts uncatchably, so every
    /// recursive descent checks its depth against [`MAX_PARSE_DEPTH`] first.
    fn deeper(depth: usize) -> Result<usize, String> {
        if depth >= MAX_PARSE_DEPTH {
            return Err(format!(
                "expression nests too deeply; limit is {MAX_PARSE_DEPTH} levels"
            ));
        }
        Ok(depth + 1)
    }

    /// expr := term (('+' | '-') term)*
    fn expr(&mut self, depth: usize) -> Result<Expr, String> {
        let depth = Self::deeper(depth)?;
        let mut left = self.term(depth)?;
        while let Some(op) = match self.peek() {
            Some(Tok::Plus) => Some(Op::Add),
            Some(Tok::Minus) => Some(Op::Sub),
            _ => None,
        } {
            self.pos += 1;
            let right = self.term(depth)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// term := unary (('*' | '/') unary)*
    fn term(&mut self, depth: usize) -> Result<Expr, String> {
        let mut left = self.unary(depth)?;
        while let Some(op) = match self.peek() {
            Some(Tok::Star) => Some(Op::Mul),
            Some(Tok::Slash) => Some(Op::Div),
            _ => None,
        } {
            self.pos += 1;
            let right = self.unary(depth)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// unary := '-' unary | power
    fn unary(&mut self, depth: usize) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Tok::Minus)) {
            self.pos += 1;
            return Ok(Expr::Neg(Box::new(self.unary(Self::deeper(depth)?)?)));
        }
        self.power(depth)
    }

    /// power := atom ('^' unary)?  (right associative)
    fn power(&mut self, depth: usize) -> Result<Expr, String> {
        let base = self.atom(depth)?;
        if matches!(self.peek(), Some(Tok::Caret)) {
            self.pos += 1;
            let exp = self.unary(Self::deeper(depth)?)?;
            return Ok(Expr::Bin(Op::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    /// atom := number | name | name '(' expr (',' expr)? ')' | '(' expr ')'
    fn atom(&mut self, depth: usize) -> Result<Expr, String> {
        let column = self.current_column();
        match self.bump() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::LParen) => {
                let inner = self.expr(depth)?;
                self.expect_right_paren("")?;
                Ok(inner)
            }
            Some(Tok::Ident(name)) => self.ident(&name, depth, column),
            None => Err(format!(
                "expression ended at column {column}; expected a number, variable, function, or '('",
            )),
            Some(other) => Err(format!(
                "unexpected token {} at column {column}",
                other.diagnostic_name()
            )),
        }
    }

    /// Resolve an identifier: the variable, a constant, or a function call.
    fn ident(&mut self, name: &str, depth: usize, name_column: usize) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Tok::LParen)) {
            let unary = match name {
                "sin" => Some(Func::Sin),
                "cos" => Some(Func::Cos),
                "tan" => Some(Func::Tan),
                "exp" => Some(Func::Exp),
                "ln" | "log" => Some(Func::Ln),
                "abs" => Some(Func::Abs),
                "sqrt" => Some(Func::Sqrt),
                "floor" => Some(Func::Floor),
                _ => None,
            };
            let pair = match name {
                "mod" => Some(PairFunc::Mod),
                "min" => Some(PairFunc::Min),
                "max" => Some(PairFunc::Max),
                _ => None,
            };
            if unary.is_none() && pair.is_none() {
                return Err(format!("unknown function '{name}' at column {name_column}"));
            }
            self.pos += 1; // consume '('
            if let Some(func) = unary {
                let arg = self.expr(depth)?;
                self.expect_right_paren(&format!("after {name}( "))?;
                Ok(Expr::Call(func, Box::new(arg)))
            } else {
                let lhs = self.expr(depth)?;
                self.expect_comma(name)?;
                let rhs = self.expr(depth)?;
                self.expect_right_paren(&format!("after {name}( "))?;
                Ok(Expr::PairCall(
                    pair.expect("validated pair function"),
                    Box::new(lhs),
                    Box::new(rhs),
                ))
            }
        } else {
            match name {
                "x" | "t" => Ok(Expr::Var),
                "a" => Ok(Expr::Param),
                "pi" => Ok(Expr::Num(PI)),
                "e" => Ok(Expr::Num(E)),
                other => Err(format!("unknown name '{other}' at column {name_column}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_CREDIT_CHARS, MAX_EXPR_TOKENS, MAX_MELODY_NOTES, MAX_META_TEXT_CHARS, MAX_PARSE_DEPTH,
        MAX_STUDIO_SOURCE_CHARS, STUDIO_RECIPES, StudioCreation, StudioKind, StudioProgram,
        StudioScale, eval, parse, studio_auto_recipe, studio_recipe, studio_recipe_count,
        to_melody, to_melody_with_scale,
    };

    #[test]
    fn curated_recipes_parse_and_auto_walk_is_deterministic() {
        assert!(studio_recipe_count() >= 16);
        for (index, source) in STUDIO_RECIPES.iter().enumerate() {
            let expr = parse(source).unwrap_or_else(|e| panic!("recipe {index}: {e}"));
            let _ = eval(&expr, 0.5, 1.0);
            assert_eq!(studio_recipe(index as u64), *source);
        }
        assert_eq!(
            studio_recipe(studio_recipe_count() as u64),
            STUDIO_RECIPES[0]
        );
        assert_eq!(studio_auto_recipe(3, 0), studio_recipe(3));
        assert_eq!(studio_auto_recipe(3, 1), studio_recipe(4));
        assert_eq!(
            studio_auto_recipe(3, studio_recipe_count() as u64),
            studio_recipe(3)
        );
    }

    fn at(source: &str, x: f64) -> f64 {
        eval(&parse(source).expect("parse"), x, 0.0)
    }

    #[test]
    fn to_melody_bounds_a_hostile_note_count() {
        // A huge `notes` (from a hostile CLI --notes) would otherwise drive an
        // unbounded sample allocation; it must clamp, while still making music.
        let expr = parse("x").expect("parses");
        let spec = to_melody(&expr, 0.0, 1.0, usize::MAX, 0.0);
        assert!(spec.notes.len() <= MAX_MELODY_NOTES);
        assert!(!spec.notes.is_empty());
    }

    #[test]
    fn to_melody_keeps_extreme_finite_ranges_finite() {
        let source = format!("x*{}", f64::MAX);
        let expr = parse(&source).expect("extreme finite expression parses");
        let spec = to_melody(&expr, -1.0, 1.0, 32, 0.0);

        assert_eq!(spec.notes.len(), 32);
        assert!(spec.notes.iter().all(|note| note.freq.is_finite()));
        assert!(spec.render(16_000).iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn deeply_nested_input_is_rejected_not_overflowed() {
        // A crafted expression must never reach the stack limit: a Rust stack
        // overflow aborts the process uncatchably, and this parser is live on
        // the MCP surface (plot_expression, sing_expression). Each of the
        // three nesting operators is checked.
        for opener in ["(", "-", "0^"] {
            let deep = opener.repeat(MAX_PARSE_DEPTH + 50);
            let source = format!("{deep}1{}", ")".repeat(MAX_PARSE_DEPTH + 50));
            let err = parse(&source).expect_err("deep nesting must error, not crash");
            assert!(
                err.contains("deep") || err.contains("token"),
                "guides the caller: {err}"
            );
        }
    }

    #[test]
    fn a_flood_of_tokens_is_rejected_at_the_door() {
        // A long-but-flat expression cannot overflow the stack, but it can
        // build a giant AST; the token cap bounds it before any descent.
        let flat = "1+".repeat(MAX_EXPR_TOKENS);
        let err = parse(&flat).expect_err("too many tokens must error");
        assert!(err.contains("token"), "names the limit: {err}");
    }

    #[test]
    fn ordinary_nesting_still_parses() {
        // The guard must not bite real formulas: a dozen levels is plenty of
        // headroom for anything a human or agent actually writes.
        assert!((at("sin(cos(((x + 1) * 2) - 3))", 0.0)).is_finite());
        assert!((at("-(-(-(-(x))))", 5.0) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert!((at("2 + 3 * 4", 0.0) - 14.0).abs() < 1e-9);
        assert!((at("(2 + 3) * 4", 0.0) - 20.0).abs() < 1e-9);
        assert!((at("2 - 3 - 4", 0.0) - -5.0).abs() < 1e-9); // left associative
    }

    #[test]
    fn power_is_right_associative() {
        assert!((at("2 ^ 3 ^ 2", 0.0) - 512.0).abs() < 1e-9); // 2^(3^2)
    }

    #[test]
    fn variable_and_unary_minus() {
        assert!((at("x^2", 3.0) - 9.0).abs() < 1e-9);
        assert!((at("-x + 1", 4.0) - -3.0).abs() < 1e-9);
    }

    #[test]
    fn exponentiation_binds_before_conventional_unary_minus() {
        assert!((at("-x^2", 3.0) + 9.0).abs() < 1e-9);
        assert!((at("(-x)^2", 3.0) - 9.0).abs() < 1e-9);
        assert!((at("2^-2", 0.0) - 0.25).abs() < 1e-9);
        assert!((at("-2^2^2", 0.0) + 16.0).abs() < 1e-9);
    }

    #[test]
    fn functions_and_constants() {
        assert!(at("sin(0)", 0.0).abs() < 1e-9);
        assert!((at("cos(0)", 0.0) - 1.0).abs() < 1e-9);
        assert!((at("sqrt(x)", 16.0) - 4.0).abs() < 1e-9);
        assert!((at("pi", 0.0) - std::f64::consts::PI).abs() < 1e-9);
        assert!((at("ln(e)", 0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn floor_and_pair_functions_cover_steps_wraps_and_clamps() {
        assert!((at("floor(x)", -1.2) + 2.0).abs() < 1e-9);
        assert!((at("mod(x, 3)", -1.0) - 2.0).abs() < 1e-9);
        assert!((at("mod(x, 3)", 7.0) - 1.0).abs() < 1e-9);
        assert!((at("min(max(x, -2), 2)", -4.0) + 2.0).abs() < 1e-9);
        assert!((at("min(max(x, -2), 2)", 1.5) - 1.5).abs() < 1e-9);
        assert!((at("min(max(x, -2), 2)", 4.0) - 2.0).abs() < 1e-9);

        let threshold = parse("max(abs(x) - a, 0)").expect("pair function accepts a");
        assert!((eval(&threshold, -3.0, 1.25) - 1.75).abs() < 1e-9);
    }

    #[test]
    fn pair_functions_do_not_hide_undefined_arguments() {
        assert!(at("mod(1, 0)", 0.0).is_nan());
        assert!(at("min(sqrt(-1), 2)", 0.0).is_nan());
        assert!(at("max(2, sqrt(-1))", 0.0).is_nan());
    }

    #[test]
    fn function_arity_errors_name_the_expected_separator() {
        assert_eq!(
            parse("min(1)").expect_err("pair function needs two arguments"),
            "expected ',' between arguments to min at column 6; found ')'"
        );
        assert_eq!(
            parse("min(1 2)").expect_err("pair function needs a comma"),
            "expected ',' between arguments to min at column 7; found number"
        );
        assert_eq!(
            parse("floor(1, 2)").expect_err("unary function rejects a second argument"),
            "expected ')' after floor( at column 8; found ','"
        );
        assert_eq!(
            parse("min(1, 2, 3)").expect_err("pair function rejects a third argument"),
            "expected ')' after min( at column 9; found ','"
        );
    }

    #[test]
    fn the_parameter_a_is_read() {
        let expr = parse("a * x").expect("parse");
        assert!((eval(&expr, 3.0, 2.0) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn a_function_becomes_a_rising_melody() {
        let expr = parse("x").expect("parse");
        let spec = super::to_melody(&expr, -1.0, 1.0, 8, 0.0);
        assert_eq!(spec.notes.len(), 8);
        assert!(spec.duration > 0.0);
        assert!(spec.notes.last().unwrap().freq > spec.notes[0].freq);
    }

    #[test]
    fn plot_text_draws_and_reports_the_range() {
        let (text, ymin, ymax) = super::plot_text("x", -1.0, 1.0, 0.0, 24, 8).expect("plot");
        assert!(text.contains('#'));
        assert!((ymin - -1.0).abs() < 0.1 && (ymax - 1.0).abs() < 0.1);
        assert!(super::plot_text("sin(", -1.0, 1.0, 0.0, 24, 8).is_err());
        assert!(super::plot_text("x", 1.0, -1.0, 0.0, 24, 8).is_err());
    }

    #[test]
    fn errors_are_reported() {
        assert!(parse("2 +").is_err());
        assert!(parse("sin(").is_err());
        assert!(parse("2 3").is_err()); // trailing input
        assert!(parse("nope(x)").is_err());
        assert!(parse("wut").is_err());
        assert!(parse("2 @ 3").is_err());
    }

    #[test]
    fn errors_name_the_source_column_and_expected_expression() {
        assert_eq!(
            parse("sin(").expect_err("incomplete call must fail"),
            "expression ended at column 5; expected a number, variable, function, or '('"
        );
        assert_eq!(
            parse("2 @ 3").expect_err("invalid character must fail"),
            "unexpected character '@' at column 3"
        );
        assert_eq!(
            parse("2 3").expect_err("trailing input must fail"),
            "unexpected trailing input at column 3"
        );
        assert_eq!(
            parse("(1 2)").expect_err("missing right parenthesis must name the token"),
            "expected ')' at column 4; found number"
        );
        assert_eq!(
            parse("+1").expect_err("unexpected token must be readable"),
            "unexpected token '+' at column 1"
        );
    }

    #[test]
    fn studio_creation_round_trips_num_files_and_links() {
        let creation = StudioCreation::new("sin(a*x) + x/2", -3.0, 3.0, 1.25).expect("creation");
        let file = creation.to_num_file();
        assert!(file.starts_with("NUMINOUS_STUDIO 1\n"));
        assert!(file.contains("expr=sin(a*x) + x/2\n"));
        assert_eq!(
            StudioCreation::from_num_file(&file).expect("file round trip"),
            creation
        );

        let link = creation.to_link();
        assert!(link.starts_with("numinous://studio?expr=sin%28a%2Ax%29%20%2B%20x%2F2"));
        assert_eq!(
            StudioCreation::from_link(&link).expect("link round trip"),
            creation
        );
    }

    #[test]
    fn pair_function_capsules_round_trip_their_separator() {
        let creation = StudioCreation::new("min(max(x, -2), 2)", -3.0, 3.0, 1.0)
            .expect("pair function creation");
        assert_eq!(
            StudioCreation::from_num_file(&creation.to_num_file()).expect("file round trip"),
            creation
        );

        let link = creation.to_link();
        assert!(
            link.contains("%2C"),
            "the comma is encoded in the URI: {link}"
        );
        assert_eq!(
            StudioCreation::from_link(&link).expect("link round trip"),
            creation
        );
    }

    #[test]
    fn studio_creation_preserves_tiny_ranges() {
        let creation = StudioCreation::new("x", 0.0, 1e-20, 1e-30).expect("tiny creation");
        let from_file =
            StudioCreation::from_num_file(&creation.to_num_file()).expect("file round trip");
        assert_eq!(from_file.xmin(), 0.0);
        assert_eq!(from_file.xmax(), 1e-20);
        assert_eq!(from_file.a(), 1e-30);
        let from_link = StudioCreation::from_link(&creation.to_link()).expect("link round trip");
        assert_eq!(from_link, creation);
    }

    #[test]
    fn studio_creation_validates_source_and_range() {
        assert!(StudioCreation::new("", -1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("sin(", -1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("x\nx", -1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("x", 1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("x", -1.0, 1.0, f64::NAN).is_err());
        let too_long = "x".repeat(MAX_STUDIO_SOURCE_CHARS + 1);
        assert!(StudioCreation::new(too_long, -1.0, 1.0, 1.0).is_err());
    }

    #[test]
    fn studio_creation_rejects_malformed_artifacts() {
        assert!(StudioCreation::from_num_file("nope").is_err());
        assert!(
            StudioCreation::from_num_file(
                "NUMINOUS_STUDIO 1\nexpr=x\nxmin=-1\nxmax=1\na=1\nunknown=2\n"
            )
            .is_err()
        );
        assert!(StudioCreation::from_link("https://example.com").is_err());
        assert!(StudioCreation::from_link("numinous://studio?expr=x&xmin=-1&xmax=1&a=%").is_err());
        assert!(
            StudioCreation::from_link("numinous://studio?expr=x&expr=x&xmin=-1&xmax=1&a=1")
                .is_err()
        );
    }

    #[test]
    fn oversized_and_out_of_range_shares_are_rejected_at_the_door() {
        // A hostile import bounds its own byte count rather than trusting the
        // caller, so a giant blob is refused before any per-line work.
        let giant = format!(
            "NUMINOUS_STUDIO 1\nexpr={}\nxmin=-1\nxmax=1\na=1\n",
            "x".repeat(super::MAX_SHARE_INPUT_BYTES)
        );
        let err = StudioCreation::from_num_file(&giant).expect_err("too large must error");
        assert!(err.contains("too large"), "names the cap: {err}");
        assert!(
            StudioCreation::from_link(&format!(
                "numinous://studio?expr={}&xmin=-1&xmax=1&a=1",
                "x".repeat(super::MAX_SHARE_INPUT_BYTES)
            ))
            .is_err()
        );
        // Absurd magnitudes are refused even when finite.
        assert!(StudioCreation::new("x".to_string(), -1e300, 1e300, 1.0).is_err());
        assert!(StudioCreation::new("x".to_string(), -1.0, 1.0, 1e300).is_err());
    }

    #[test]
    fn a_capsule_without_metadata_stays_a_version_one_file() {
        // Older builds parse only NUMINOUS_STUDIO 1, so the lowest-version
        // rule is what keeps a plain share openable by the release before
        // this format existed.
        let plain = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5).expect("plain");
        let text = plain.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 1\n"), "{text}");
        assert_eq!(StudioCreation::from_num_file(&text).expect("reopen"), plain);
    }

    #[test]
    fn a_capsule_with_metadata_round_trips_as_version_two() {
        let parent = StudioCreation::new("sin(x)", -1.0, 1.0, 0.0).expect("parent");
        let full = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
            .expect("creation")
            .with_title("Slow Waves")
            .expect("title")
            .with_author("A Curious Mind")
            .expect("author")
            .with_era(crate::era::Era::Phosphor)
            .with_descends(&parent.to_link())
            .expect("descends");
        let text = full.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 2\n"), "{text}");
        let reopened = StudioCreation::from_num_file(&text).expect("reopen");
        assert_eq!(reopened, full);
        assert_eq!(reopened.title(), Some("Slow Waves"));
        assert_eq!(reopened.author(), Some("A Curious Mind"));
        assert_eq!(reopened.era(), Some(crate::era::Era::Phosphor));
        assert_eq!(reopened.descends(), Some(parent.to_link().as_str()));
    }

    #[test]
    fn links_carry_identity_but_never_lineage() {
        let parent = StudioCreation::new("sin(x)", -1.0, 1.0, 0.0).expect("parent");
        let full = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
            .expect("creation")
            .with_title("Slow Waves")
            .expect("title")
            .with_era(crate::era::Era::Vector)
            .with_descends(&parent.to_link())
            .expect("descends");
        let link = full.to_link();
        assert!(link.contains("title=Slow%20Waves"), "{link}");
        assert!(link.contains("era=Vector"), "{link}");
        assert!(
            !link.contains("descends"),
            "a link that nests links is a growth format: {link}"
        );
        let reopened = StudioCreation::from_link(&link).expect("reopen");
        assert_eq!(reopened.title(), Some("Slow Waves"));
        assert_eq!(reopened.era(), Some(crate::era::Era::Vector));
        assert_eq!(reopened.descends(), None);
        assert!(
            StudioCreation::from_link(
                "numinous://studio?expr=x&xmin=-1&xmax=1&a=0&descends=numinous"
            )
            .is_err(),
            "a descends parameter in a link is refused as unknown"
        );
    }

    #[test]
    fn forks_keep_the_canvas_but_take_their_own_identity() {
        let parent = StudioCreation::new("sin(a*x)", -3.0, 4.0, 0.75)
            .expect("parent")
            .with_title("First Wave")
            .expect("title")
            .with_author("First Hand")
            .expect("author")
            .with_era(crate::era::Era::Vector);
        let child = parent
            .fork(Some("sin(a*x)+0.1"), Some("Second Wave"), Some("Next Hand"))
            .expect("fork");

        assert_eq!(child.source(), "sin(a*x)+0.1");
        assert_eq!((child.xmin(), child.xmax(), child.a()), (-3.0, 4.0, 0.75));
        assert_eq!(child.title(), Some("Second Wave"));
        assert_eq!(child.author(), Some("Next Hand"));
        assert_eq!(child.era(), Some(crate::era::Era::Vector));
        assert_eq!(child.descends(), Some(parent.to_link().as_str()));

        let unsigned = parent.fork(None, None, None).expect("plain fork");
        assert_eq!(unsigned.source(), parent.source());
        assert_eq!(unsigned.title(), None);
        assert_eq!(unsigned.author(), None);
        assert_eq!(
            unsigned.credit(),
            Some("After First Wave by First Hand"),
            "a fork offers editable prose credit from the parent's identity"
        );
        assert!(unsigned.to_num_file().starts_with("NUMINOUS_STUDIO 4\n"));
        assert_eq!(unsigned.descends(), Some(parent.to_link().as_str()));
        assert_eq!(
            unsigned.clone().without_credit().credit(),
            None,
            "clearing the suggestion leaves the machine lineage"
        );
    }

    #[test]
    fn credit_overrides_retain_clear_or_replace_without_empty_records() {
        let parent = StudioCreation::new("sin(x)", -2.0, 3.0, 0.5)
            .expect("parent")
            .with_title("First Wave")
            .expect("title");
        let child = parent.fork(None, None, None).expect("child");
        assert_eq!(
            child.clone().with_credit_override(None).expect("no edit"),
            child
        );
        assert_eq!(child.credit(), Some("After First Wave"));
        for edit in ["", "   ", "\t\n"] {
            let cleared = child
                .clone()
                .with_credit_override(Some(edit))
                .expect("clear");
            assert_eq!(cleared, child.clone().without_credit());
            assert_eq!(cleared.descends(), Some(parent.to_link().as_str()));
            assert!(!cleared.to_num_file().contains("credit="));
            assert!(!cleared.to_link().contains("credit="));
            assert_eq!(
                StudioCreation::from_num_file(&cleared.to_num_file()).expect("cleared file"),
                cleared
            );
        }
        let replaced = child
            .clone()
            .with_credit_override(Some("  A different source  "))
            .expect("replace");
        assert_eq!(replaced.credit(), Some("A different source"));
        assert_eq!(replaced.descends(), child.descends());
        for invalid in ["x".repeat(MAX_CREDIT_CHARS + 1), "line\nbreak".to_string()] {
            assert!(child.clone().with_credit_override(Some(&invalid)).is_err());
        }
        // Editing emptiness removes the field; stored emptiness remains invalid.
        let invalid_file = child
            .to_num_file()
            .replace("credit=After First Wave\n", "credit=\n");
        assert!(StudioCreation::from_num_file(&invalid_file).is_err());
        let invalid_link = child
            .to_link()
            .replace("credit=After%20First%20Wave", "credit=");
        assert!(StudioCreation::from_link(&invalid_link).is_err());
    }

    #[test]
    fn prose_credit_round_trips_as_version_four() {
        let parent = StudioCreation::new("sin(x)", -1.0, 1.0, 0.0).expect("parent");
        let credited = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5)
            .expect("creation")
            .with_title("Second Wave")
            .expect("title")
            .with_credit("After Slow Waves by A Curious Mind")
            .expect("credit");
        let text = credited.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 4\n"), "{text}");
        assert!(text.contains("kind=graph\n"), "{text}");
        assert!(
            text.contains("credit=After Slow Waves by A Curious Mind\n"),
            "{text}"
        );
        assert_eq!(
            StudioCreation::from_num_file(&text).expect("file"),
            credited
        );
        let link = credited.to_link();
        assert!(link.contains("credit=After%20Slow%20Waves"), "{link}");
        assert!(!link.contains("descends"), "{link}");
        assert_eq!(StudioCreation::from_link(&link).expect("link"), credited);

        let child = parent
            .clone()
            .with_title("Slow Waves")
            .expect("title")
            .fork(None, Some("Remix"), None)
            .expect("fork");
        assert_eq!(child.credit(), Some("After Slow Waves"));
        assert_eq!(child.title(), Some("Remix"));

        let nameless = parent.fork(None, None, None).expect("nameless parent");
        assert_eq!(
            nameless.credit(),
            None,
            "a nameless parent invents no thanks"
        );
    }

    #[test]
    fn portable_capsule_input_never_interprets_a_path() {
        let creation = StudioCreation::new("cos(x)", -2.0, 2.0, 0.5)
            .expect("creation")
            .with_title("Arc")
            .expect("title");
        assert_eq!(
            StudioCreation::from_capsule(&creation.to_num_file()).expect("num text"),
            creation
        );
        let from_link = StudioCreation::from_capsule(&creation.to_link()).expect("native link");
        assert_eq!(from_link.title(), creation.title());
        assert_eq!(from_link.source(), creation.source());

        let path = std::env::temp_dir().join(format!(
            "numinous_core_portable_capsule_inert_{}.num",
            std::process::id()
        ));
        std::fs::write(&path, creation.to_num_file()).expect("write valid path target");
        assert!(
            StudioCreation::from_capsule(&path.to_string_lossy()).is_err(),
            "portable input must not read even a valid capsule from a path"
        );
        std::fs::remove_file(path).expect("remove valid path target");
    }

    #[test]
    fn the_old_header_rejects_the_new_fields_and_newer_headers_are_named() {
        // Version 1 cannot smuggle version 2 content.
        let smuggled = "NUMINOUS_STUDIO 1\nexpr=x\nxmin=-1\nxmax=1\na=0\ntitle=Sneak\n";
        let err = StudioCreation::from_num_file(smuggled).expect_err("smuggling refused");
        assert!(err.contains("NUMINOUS_STUDIO 2"), "{err}");
        let smuggled_credit = "NUMINOUS_STUDIO 3\nkind=graph\nexpr=x\nxmin=-1\nxmax=1\na=0\nscale=continuous\ncredit=After Waves\n";
        let err = StudioCreation::from_num_file(smuggled_credit).expect_err("credit needs v4");
        assert!(err.contains("NUMINOUS_STUDIO 4"), "{err}");
        // A future version is a fact to report, not a guess to parse.
        let future = "NUMINOUS_STUDIO 5\nexpr=x\nxmin=-1\nxmax=1\na=0\n";
        let err = StudioCreation::from_num_file(future).expect_err("future refused");
        assert!(err.contains("newer Numinous"), "{err}");
    }

    #[test]
    fn parametric_capsules_round_trip_as_version_three() {
        let creation = StudioCreation::new_parametric(
            "cos(3*t + a)",
            "sin(2*t)",
            0.0,
            std::f64::consts::TAU,
            0.25,
        )
        .expect("parametric creation")
        .with_scale(StudioScale::Minor)
        .with_title("Three by Two")
        .expect("title")
        .with_author("Curve Hand")
        .expect("author");

        assert_eq!(creation.kind(), StudioKind::Parametric);
        assert_eq!(creation.second_source(), Some("sin(2*t)"));
        assert_eq!(creation.scale(), StudioScale::Minor);
        assert_eq!(creation.editor_source(), "x(t)=cos(3*t + a); y(t)=sin(2*t)");

        let text = creation.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 3\n"), "{text}");
        assert!(text.contains("kind=parametric\n"), "{text}");
        assert!(text.contains("scale=minor\n"), "{text}");
        assert_eq!(
            StudioCreation::from_num_file(&text).expect("file"),
            creation
        );

        let link = creation.to_link();
        assert!(link.contains("kind=parametric"), "{link}");
        assert!(link.contains("xexpr=cos%283%2At%20%2B%20a%29"), "{link}");
        assert_eq!(StudioCreation::from_link(&link).expect("link"), creation);
    }

    #[test]
    fn stored_scale_grows_a_graph_capsule_without_changing_old_defaults() {
        let plain = StudioCreation::new("sin(x)", -2.0, 2.0, 0.0).expect("plain");
        assert_eq!(plain.scale(), StudioScale::Continuous);
        assert!(plain.to_num_file().starts_with("NUMINOUS_STUDIO 1\n"));

        let scaled = plain.clone().with_scale(StudioScale::Pentatonic);
        let text = scaled.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 3\n"), "{text}");
        assert!(text.contains("kind=graph\n"), "{text}");
        assert_eq!(StudioCreation::from_num_file(&text).expect("file"), scaled);
        assert_eq!(
            StudioCreation::from_link(&scaled.to_link()).expect("link"),
            scaled
        );
    }

    fn parametric_cells(plot: &super::StudioPlot) -> Vec<(usize, usize)> {
        plot.text
            .lines()
            .enumerate()
            .flat_map(|(y, row)| {
                row.chars()
                    .enumerate()
                    .filter(|(_, mark)| *mark != ' ')
                    .map(move |(x, _)| (x, y))
            })
            .collect()
    }

    fn cell_bounds(cells: &[(usize, usize)]) -> ((usize, usize), (usize, usize)) {
        (
            (
                cells.iter().map(|p| p.0).min().unwrap(),
                cells.iter().map(|p| p.1).min().unwrap(),
            ),
            (
                cells.iter().map(|p| p.0).max().unwrap(),
                cells.iter().map(|p| p.1).max().unwrap(),
            ),
        )
    }

    #[test]
    fn parametric_canvas_keeps_circles_round_and_ellipses_distinct() {
        let circle =
            StudioCreation::new_parametric("cos(t)", "sin(t)", 0.0, std::f64::consts::TAU, 0.0)
                .unwrap();
        let ellipse =
            StudioCreation::new_parametric("4*cos(t)", "sin(t)", 0.0, std::f64::consts::TAU, 0.0)
                .unwrap();
        for (width, height) in [(81, 41), (41, 81), (120, 40), (40, 120)] {
            let circle_plot = circle.plot_text(width, height).unwrap();
            let ellipse_plot = ellipse.plot_text(width, height).unwrap();
            assert_ne!(circle_plot.text, ellipse_plot.text);
            for (plot, ratio) in [(&circle_plot, 1.0), (&ellipse_plot, 4.0)] {
                let (lower, upper) = cell_bounds(&parametric_cells(plot));
                let dx = (upper.0 - lower.0) as f64;
                let dy = 2.0 * (upper.1 - lower.1) as f64;
                // A terminal cell is twice as tall as wide. One-cell endpoint
                // rounding permits one column plus one scaled row of error.
                assert!((dx - ratio * dy).abs() <= 1.0 + 2.0 * ratio);
                assert!((lower.0 + upper.0).abs_diff(width - 1) <= 1);
                assert!((lower.1 + upper.1).abs_diff(height - 1) <= 1);
            }
        }
    }

    #[test]
    fn translated_parametric_paths_keep_their_centered_geometry_and_values() {
        let circle =
            StudioCreation::new_parametric("cos(t)", "sin(t)", 0.0, std::f64::consts::TAU, 0.0)
                .unwrap();
        let translated = StudioCreation::new_parametric(
            "cos(t)+17",
            "sin(t)-11",
            0.0,
            std::f64::consts::TAU,
            0.0,
        )
        .unwrap();
        let capsule = translated.to_num_file();
        for (width, height) in [(81, 41), (41, 81)] {
            let base = circle.plot_text(width, height).unwrap();
            let shifted = translated.plot_text(width, height).unwrap();
            assert!((shifted.xmin - base.xmin - 17.0).abs() < 1e-12);
            assert!((shifted.xmax - base.xmax - 17.0).abs() < 1e-12);
            assert!((shifted.ymin - base.ymin + 11.0).abs() < 1e-12);
            assert!((shifted.ymax - base.ymax + 11.0).abs() < 1e-12);
            let base_cells = parametric_cells(&base);
            let shifted_cells = parametric_cells(&shifted);
            for (left, right) in [(&base_cells, &shifted_cells), (&shifted_cells, &base_cells)] {
                for p in left {
                    assert!(
                        right
                            .iter()
                            .any(|q| p.0.abs_diff(q.0) <= 1 && p.1.abs_diff(q.1) <= 1)
                    );
                }
            }
        }
        assert_eq!(translated.to_num_file(), capsule);
        assert_eq!(StudioCreation::from_num_file(&capsule).unwrap(), translated);
    }

    #[test]
    fn parametric_lines_and_points_remain_centered_and_visible() {
        for (width, height) in [(81, 41), (41, 81)] {
            for (x, y, expected) in [
                (
                    "t",
                    "7",
                    ((0, (height - 1) / 2), (width - 1, (height - 1) / 2)),
                ),
                (
                    "2",
                    "t",
                    (((width - 1) / 2, 0), ((width - 1) / 2, height - 1)),
                ),
                (
                    "7",
                    "-3",
                    (
                        ((width - 1) / 2, (height - 1) / 2),
                        ((width - 1) / 2, (height - 1) / 2),
                    ),
                ),
            ] {
                let creation = StudioCreation::new_parametric(x, y, -1.0, 1.0, 0.0).unwrap();
                let cells = parametric_cells(&creation.plot_text(width, height).unwrap());
                assert_eq!(cell_bounds(&cells), expected);
                if x == "7" {
                    assert_eq!(cells.len(), 1);
                }
            }
        }
        // Only t=0 is finite, so no segment exists to make this point visible.
        let isolated = StudioCreation::new_parametric("7", "sqrt(-t)", 0.0, 1.0, 0.0).unwrap();
        assert_eq!(
            parametric_cells(&isolated.plot_text(81, 41).unwrap()),
            [(40, 20)]
        );
    }

    #[test]
    fn parametric_undefined_intervals_remain_gaps() {
        let creation =
            StudioCreation::new_parametric("t", "sqrt(t^2-0.25)", -1.0, 1.0, 0.0).unwrap();
        let cells = parametric_cells(&creation.plot_text(81, 41).unwrap());
        assert!(cells.iter().any(|p| p.0 < 20));
        assert!(cells.iter().any(|p| p.0 > 60));
        // Finite samples next to x=+/-0.5 can round onto the boundary columns.
        assert!(cells.iter().all(|p| p.0 <= 20 || p.0 >= 60));
        let undefined = StudioCreation::new_parametric("sqrt(-1)", "t", -1.0, 1.0, 0.0).unwrap();
        assert!(undefined.plot_text(81, 41).is_err());
    }

    #[test]
    fn parametric_finite_extreme_curves_and_hostile_sizes_stay_bounded() {
        for exponent in [-700, 700] {
            let creation = StudioCreation::new_parametric(
                format!("exp({exponent})*cos(t)"),
                format!("exp({exponent})*sin(t)"),
                0.0,
                std::f64::consts::TAU,
                0.0,
            )
            .unwrap();
            let plot = creation.plot_text(81, 41).unwrap();
            let (lower, upper) = cell_bounds(&parametric_cells(&plot));
            assert!((upper.0 - lower.0).abs_diff(2 * (upper.1 - lower.1)) <= 2);
            assert!(plot.xmin < 0.0 && plot.xmax > 0.0);
        }
        let creation = StudioCreation::new_parametric("t", "t", -1.0, 1.0, 0.0).unwrap();
        let plot = creation.plot_text(usize::MAX, 18).unwrap();
        assert_eq!(plot.text.lines().count(), 18);
        assert!(
            plot.text
                .lines()
                .all(|line| line.len() <= crate::surface::MAX_DIM)
        );
        let (lower, upper) = cell_bounds(&parametric_cells(&plot));
        assert!((upper.0 - lower.0).abs_diff(2 * (upper.1 - lower.1)) <= 2);
        assert!(creation.plot_text(1, 18).is_err());
        let unresolved =
            StudioCreation::new_parametric("exp(-700)*t", "exp(700)*t", -1.0, 1.0, 0.0).unwrap();
        assert_eq!(
            unresolved.plot_text(81, 41).unwrap_err(),
            "the planar coordinate range cannot be represented faithfully"
        );
        let graph = StudioCreation::new("t", -1.0, 1.0, 0.0).unwrap();
        assert_eq!(
            graph.plot_text(1, 41).unwrap_err(),
            "need width >= 2, height >= 2, and xmax > xmin"
        );
    }

    #[test]
    fn parametric_editor_and_plot_are_one_bounded_program() {
        let program =
            StudioProgram::from_editor(" x(t) = cos(t) ; y(t) = sin(t) ").expect("editor pair");
        assert_eq!(program.kind(), StudioKind::Parametric);
        assert_eq!(program.editor_source(), "x(t)=cos(t); y(t)=sin(t)");
        let creation =
            StudioCreation::new_parametric("cos(t)", "sin(t)", 0.0, std::f64::consts::TAU, 0.0)
                .expect("circle");
        let plot = creation.plot_text(40, 18).expect("plot");
        assert!(plot.text.contains('#'));
        assert!((plot.xmin + 1.0).abs() < 0.01, "{}", plot.xmin);
        assert!((plot.xmax - 1.0).abs() < 0.01, "{}", plot.xmax);
        assert!((plot.ymin + 1.0).abs() < 0.01, "{}", plot.ymin);
        assert!((plot.ymax - 1.0).abs() < 0.01, "{}", plot.ymax);

        assert!(StudioProgram::from_editor("x(t)=t").is_err());
        assert!(StudioProgram::from_editor("x(t)=t; y(t)=t; y(t)=0").is_err());
        assert!(StudioCreation::from_num_file(
            "NUMINOUS_STUDIO 3\nkind=parametric\nxexpr=cos(t)\ntmin=0\ntmax=1\na=0\nscale=major\n"
        )
        .is_err());
    }

    #[test]
    fn named_scales_quantize_the_portable_voice() {
        let expression = parse("x").expect("expression");
        let continuous =
            to_melody_with_scale(&expression, 0.0, 1.0, 19, 0.0, StudioScale::Continuous);
        let major = to_melody_with_scale(&expression, 0.0, 1.0, 19, 0.0, StudioScale::Major);
        assert_eq!(continuous.notes.len(), major.notes.len());
        assert!(
            continuous
                .notes
                .iter()
                .zip(&major.notes)
                .any(|(left, right)| (left.freq - right.freq).abs() > 0.1)
        );
        let allowed = [0, 2, 4, 5, 7, 9, 11];
        for note in &major.notes {
            let semitones = (12.0 * (note.freq / 220.0).log2()).round() as i32;
            assert!(allowed.contains(&semitones.rem_euclid(12)) || semitones == 24);
        }
    }

    #[test]
    fn parametric_forks_keep_both_coordinates_and_scale() {
        let parent = StudioCreation::new_parametric("cos(t)", "sin(t)", 0.0, 6.0, 0.5)
            .expect("parent")
            .with_scale(StudioScale::Pentatonic)
            .with_era(crate::era::Era::Vector);
        let child = parent
            .fork_parametric(Some("cos(3*t)"), Some("sin(2*t)"), Some("Lissajous"), None)
            .expect("fork");
        assert_eq!(child.source(), "cos(3*t)");
        assert_eq!(child.second_source(), Some("sin(2*t)"));
        assert_eq!(child.scale(), StudioScale::Pentatonic);
        assert_eq!(child.descends(), Some(parent.to_link().as_str()));
        assert!(parent.fork(Some("cos(2*t)"), None, None).is_err());
        assert!(parent.fork_parametric(Some("t"), None, None, None).is_err());
    }

    #[test]
    fn capsule_metadata_is_capped_and_printable_only() {
        let base = || StudioCreation::new("x", -1.0, 1.0, 0.0).expect("base");
        assert!(base().with_title("").is_err(), "empty is not a name");
        assert!(base().with_title("   ").is_err(), "spaces are not a name");
        assert!(
            base()
                .with_title(&"x".repeat(MAX_META_TEXT_CHARS + 1))
                .is_err(),
            "the cap bites"
        );
        assert!(
            base().with_title("line\u{1b}[31mbreak").is_err(),
            "a control byte cannot ride in a title"
        );
        assert!(
            base().with_author("newline\nauthor").is_err(),
            "a line break cannot fork the line format"
        );
        assert!(
            base().with_title("Prismatic Chord No. 7").is_ok(),
            "an ordinary name passes"
        );
        assert!(base().with_credit("").is_err(), "empty is not credit");
        assert!(
            base()
                .with_credit(&"x".repeat(MAX_CREDIT_CHARS + 1))
                .is_err(),
            "the credit cap bites"
        );
        assert!(
            base().with_credit("thanks\nnope").is_err(),
            "a line break cannot fork the line format"
        );
        let title = "T".repeat(MAX_META_TEXT_CHARS);
        let author = "A".repeat(MAX_META_TEXT_CHARS);
        let maxed = base()
            .with_title(&title)
            .expect("max title")
            .with_author(&author)
            .expect("max author");
        let suggestion = maxed.fork_credit_suggestion().expect("suggestion");
        assert!(
            suggestion.chars().count() <= MAX_CREDIT_CHARS,
            "the default sentence must fit the credit cap"
        );
        assert!(base().with_credit(&suggestion).is_ok());

        // Lineage must reopen or it is not lineage.
        assert!(base().with_descends("not a link").is_err());
        assert!(
            base()
                .with_descends("numinous://studio?expr=x&xmin=-1&xmax=1&a=%")
                .is_err(),
            "a broken parent link is refused"
        );
        let mut giant = String::from("numinous://studio?expr=x&xmin=-1&xmax=1&a=0");
        giant.push_str(&"&".repeat(5000));
        assert!(
            base().with_descends(&giant).is_err(),
            "an oversized parent link is refused"
        );

        // A control byte inside the link text would split the written
        // descends line and make the saved capsule unreadable, so it is
        // refused at the door even when the link parser tolerates it.
        assert!(
            base()
                .with_descends("numinous://studio?expr=x\n&xmin=-1&xmax=1&a=0")
                .is_err(),
            "a line break cannot ride inside a recorded parent link"
        );
    }

    #[test]
    fn num_files_load_from_disk_through_one_bounded_door() {
        // Every face that reads a `.num` from disk goes through this loader,
        // so the byte cap, the read order, and the refusal reasons are proved
        // once here rather than once per face.
        let dir = std::env::temp_dir();

        let good = dir.join("numinous_core_from_num_path_good.num");
        let creation = StudioCreation::new("sin(a*x)", -2.0, 2.0, 0.5).expect("creation");
        std::fs::write(&good, creation.to_num_file()).expect("write good");
        let reopened = StudioCreation::from_num_path(&good).expect("reopen");
        assert_eq!(reopened, creation, "a reopen is exact, not approximate");
        let _ = std::fs::remove_file(&good);

        let missing = dir.join("numinous_core_from_num_path_missing.num");
        let _ = std::fs::remove_file(&missing);
        assert!(matches!(
            StudioCreation::from_num_path(&missing),
            Err(super::NumFileError::Io(_))
        ));

        let huge = dir.join("numinous_core_from_num_path_huge.num");
        std::fs::write(&huge, "x".repeat(super::MAX_SHARE_INPUT_BYTES + 1)).expect("write huge");
        assert!(matches!(
            StudioCreation::from_num_path(&huge),
            Err(super::NumFileError::TooLarge)
        ));
        let _ = std::fs::remove_file(&huge);

        let invalid = dir.join("numinous_core_from_num_path_invalid.num");
        std::fs::write(&invalid, "not a studio file\n").expect("write invalid");
        assert!(matches!(
            StudioCreation::from_num_path(&invalid),
            Err(super::NumFileError::Invalid(_))
        ));
        let _ = std::fs::remove_file(&invalid);

        // Bytes that are not UTF-8 are a read refusal, never a panic.
        let binary = dir.join("numinous_core_from_num_path_binary.num");
        std::fs::write(&binary, [0xFFu8, 0xFE, 0x00, 0x01]).expect("write binary");
        assert!(matches!(
            StudioCreation::from_num_path(&binary),
            Err(super::NumFileError::Io(_))
        ));
        let _ = std::fs::remove_file(&binary);
    }

    // A seeded totality harness for the untrusted-input surface. EXTENSIBILITY.md
    // promises the Studio parser and importers are "fuzzed continuously"; a
    // full cargo-fuzz run needs a nightly toolchain and is the CI-nightly
    // future, but the core totality properties (never panic, never diverge,
    // always terminate, caps always bite) belong in the stable gate where
    // every commit exercises them. This is that guard: deterministic, so a
    // regression names the exact seed that broke it.

    /// A pseudo-random hostile string over an alphabet biased toward the
    /// characters that actually drive the parser, plus junk and multi-byte
    /// UTF-8 to probe byte-boundary slicing.
    fn hostile_string(rng: &mut crate::rng::SplitMix64, max_len: usize) -> String {
        // Weighted so parens and operators dominate: deep nesting and long
        // operator runs are the shapes that stress a recursive-descent parser.
        const ALPHABET: &[char] = &[
            '(', '(', '(', ')', ')', '+', '-', '*', '/', '^', 'x', 'a', '.', '0', '1', '9', 's',
            'i', 'n', 'c', 'o', 'e', ' ', 'z', '%', '=', '&', '\n', '\t', '\u{00e9}', '\u{4e16}',
        ];
        let len = (rng.below(max_len as u64 + 1)) as usize;
        (0..len)
            .map(|_| ALPHABET[rng.below(ALPHABET.len() as u64) as usize])
            .collect()
    }

    #[test]
    fn the_parser_is_total_over_hostile_input() {
        let mut rng = crate::rng::SplitMix64::new(0x00D1_5EA5);
        for _ in 0..20_000 {
            let source = hostile_string(&mut rng, 200);
            // The only contract: parse returns, never panics, never hangs. A
            // panic or a non-terminating input fails this test outright.
            if let Ok(expr) = parse(&source) {
                // A parsed expression must evaluate totally at any x, however
                // hostile: the caller (renderer, melody) relies on a finite
                // world downstream, but eval itself must never panic.
                for &x in &[0.0, 1.0, -1e300, 1e300, f64::MIN_POSITIVE, -0.0] {
                    let y = eval(&expr, x, 0.5);
                    let _ = y.is_finite(); // touch it; NaN and inf are allowed
                }
            }
        }
    }

    #[test]
    fn the_caps_always_bite_pathological_input() {
        // Past the caps, parse must ALWAYS reject, never accept and never
        // crash. Both nesting and breadth are checked, at and beyond the edge.
        let mut rng = crate::rng::SplitMix64::new(0x0000_CA95);
        for _ in 0..500 {
            let over_depth = rng.below(200) as usize + MAX_PARSE_DEPTH + 1;
            let nested = format!("{}1{}", "(".repeat(over_depth), ")".repeat(over_depth));
            assert!(
                parse(&nested).is_err(),
                "depth {over_depth} must be rejected"
            );

            // A well-formed but oversized sum: "1+1+...+1" with a trailing
            // operand, so it would parse cleanly if not for the token cap.
            // (A trailing "+" would error on its own and prove nothing.) The
            // error must name the token limit, so this guards the cap itself,
            // not some incidental syntax failure.
            let pairs = rng.below(500) as usize + MAX_EXPR_TOKENS;
            let flooded = format!("{}1", "1+".repeat(pairs));
            let err = parse(&flooded).expect_err("token flood must be rejected");
            assert!(
                err.contains("token"),
                "the token cap must be the reason: {err}"
            );
        }
    }

    #[test]
    fn the_importers_are_total_and_never_forge_state() {
        let mut rng = crate::rng::SplitMix64::new(0x0000_F11E);
        for _ in 0..20_000 {
            // Feed both importers arbitrary bytes, including well-formed
            // prefixes so the per-field parsing is actually reached.
            let body = hostile_string(&mut rng, 120);
            let file = if rng.below(2) == 0 {
                format!("NUMINOUS_STUDIO 1\n{body}")
            } else {
                body.clone()
            };
            let _ = StudioCreation::from_num_file(&file); // must not panic
            let link = if rng.below(2) == 0 {
                format!("numinous://studio?{body}")
            } else {
                body
            };
            let _ = StudioCreation::from_link(&link); // must not panic
        }
    }

    #[test]
    fn valid_creations_round_trip_under_fuzzed_values() {
        // Any creation the constructor accepts must survive both
        // serializations unchanged: sharing is lossless or it is a bug.
        // Real formulas so the round-trip path is reliably exercised; the
        // fuzzing is in the numeric fields and the occasional hostile source.
        const VALID: &[&str] = &[
            "x",
            "sin(x)",
            "a*x + 1",
            "x^2 - 3",
            "cos(x) / 2",
            "-x",
            "abs(x)",
        ];
        let mut rng = crate::rng::SplitMix64::new(0x0000_5EED);
        let mut round_tripped = 0;
        for _ in 0..5_000 {
            let source = if rng.below(2) == 0 {
                VALID[rng.below(VALID.len() as u64) as usize].to_string()
            } else {
                hostile_string(&mut rng, 40)
            };
            let span = rng.next_f64() * 2000.0 - 1000.0;
            let xmin = rng.next_f64() * 2000.0 - 1000.0;
            let xmax = xmin + span.abs() + 1e-6;
            let a = rng.next_f64() * 20.0 - 10.0;
            if let Ok(creation) = StudioCreation::new(source, xmin, xmax, a) {
                assert_eq!(
                    StudioCreation::from_num_file(&creation.to_num_file()).as_ref(),
                    Ok(&creation),
                    ".num round trip must be lossless"
                );
                assert_eq!(
                    StudioCreation::from_link(&creation.to_link()).as_ref(),
                    Ok(&creation),
                    "link round trip must be lossless"
                );
                round_tripped += 1;
            }
        }
        assert!(
            round_tripped > 500,
            "the generator must actually produce valid creations, got {round_tripped}"
        );
    }
}
