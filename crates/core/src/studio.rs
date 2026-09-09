//! The Studio's expression engine: type a function, get a curve.
//!
//! A small, safe evaluator for single-variable expressions in `x`, the seed of
//! the creative graphing calculator (Tier 1 of the extensibility model in
//! `docs/ARCHITECTURE.md`: no arbitrary code, just math). It parses `sin(3*x) +
//! x^2/2` into an AST and evaluates it, so a plotter, a quiz, or an authored room
//! can all share one safe language. See `docs/PLAYFUL.md`.

use std::f64::consts::{E, PI};

use crate::complex::Complex;
use crate::field::{self, FieldReading, height_measure, is_real_valued, phase_measure};
use crate::sound::{Note, SoundSpec};

/// Maximum accepted Studio source length for share files and links.
pub const MAX_STUDIO_SOURCE_CHARS: usize = 512;

/// The most graph expressions one overlay program may hold.
pub const MAX_PROGRAM_EXPRS: usize = 4;

/// Marks drawn for overlay graphs, in source order.
pub const PROGRAM_MARKS: [char; MAX_PROGRAM_EXPRS] = ['#', '*', '+', 'o'];

/// Largest step count `euclid(hits, steps)` will realize. A larger request
/// is undefined rather than silently truncated.
pub const MAX_EUCLID_STEPS: usize = 64;

/// Hit mark in the pattern-text view of a 0/1 step graph.
pub const PATTERN_HIT: char = 'x';

/// Rest mark in the pattern-text view of a 0/1 step graph.
pub const PATTERN_REST: char = '.';

/// Maximum editable text for one scalar formula, one labeled parametric pair,
/// or one overlay program. Each expression keeps the per-source cap above;
/// this larger bound accounts for extra expressions and their separators.
pub const MAX_STUDIO_EDITOR_CHARS: usize = MAX_STUDIO_SOURCE_CHARS * MAX_PROGRAM_EXPRS + 16;

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
    "euclid(3,8)",
    "pat(x..x..x.)",
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

/// One bundled Studio experiment: a titled, portable capsule with a question.
///
/// These are garage doors after a touch of math, not a lobby. The `.num`
/// documents in `docs/experiments/` remain the source; this table is how a
/// packaged player opens them without a host path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudioExperiment {
    /// Stable id, also accepted by [`StudioCreation::from_capsule`].
    pub id: &'static str,
    /// Family this experiment belongs to, for filtered lists.
    pub family: &'static str,
    /// Capsule title, matching the bundled `.num` document.
    pub title: &'static str,
    /// One nonspoiling question to try with this creation.
    pub invitation: &'static str,
    num_file: &'static str,
}

impl StudioExperiment {
    /// Parse the bundled capsule. The documents are repository invariants.
    #[must_use]
    pub fn creation(self) -> StudioCreation {
        StudioCreation::from_num_file(self.num_file).unwrap_or_else(|error| {
            panic!(
                "bundled Studio experiment '{}' must parse: {error}",
                self.id
            )
        })
    }
}

/// Bundled Studio experiments, family by family, in the order a player should
/// meet them.
pub const STUDIO_EXPERIMENTS: &[StudioExperiment] = &[
    StudioExperiment {
        id: "full-return",
        family: "returning-home",
        title: "A full return",
        invitation: "Count how many oscillations each coordinate makes before the whole motion repeats.",
        num_file: include_str!("../../../docs/experiments/full-return.num"),
    },
    StudioExperiment {
        id: "almost-home",
        family: "returning-home",
        title: "Almost home",
        invitation: "Compare its formula with the first creation. What changed, and what would count as evidence of repetition?",
        num_file: include_str!("../../../docs/experiments/almost-home.num"),
    },
    StudioExperiment {
        id: "same-place",
        family: "returning-home",
        title: "Same place, another direction",
        invitation: "Evaluate the point at t = 0 and t = 0.5. Would you expect its next move to be the same?",
        num_file: include_str!("../../../docs/experiments/same-place.num"),
    },
    StudioExperiment {
        id: "another-ratio",
        family: "returning-home",
        title: "Another ratio",
        invitation: "Change one frequency. What would count as a return now?",
        num_file: include_str!("../../../docs/experiments/another-ratio.num"),
    },
    StudioExperiment {
        id: "circle-to-ellipse",
        family: "shape-and-scale",
        title: "Circle to ellipse",
        invitation: "What changes between a = 1 and a = 4? How wide is the shape compared with its height?",
        num_file: include_str!("../../../docs/experiments/circle-to-ellipse.num"),
    },
    StudioExperiment {
        id: "uniform-circle",
        family: "shape-and-scale",
        title: "Uniform circle",
        invitation: "Change a from 1 to 4 again. Can the mathematical size change while the fitted picture looks the same?",
        num_file: include_str!("../../../docs/experiments/uniform-circle.num"),
    },
    StudioExperiment {
        id: "simple-zero",
        family: "three-readings",
        title: "A simple zero",
        invitation: "Follow the ramp around the origin. How many times does it go around before it meets itself?",
        num_file: include_str!("../../../docs/experiments/simple-zero.num"),
    },
    StudioExperiment {
        id: "a-pole",
        family: "three-readings",
        title: "A pole",
        invitation: "Compare its formula with A simple zero. Which way does the ramp run now?",
        num_file: include_str!("../../../docs/experiments/a-pole.num"),
    },
    StudioExperiment {
        id: "the-circle",
        family: "three-readings",
        title: "The circle",
        invitation: "A marked cell is a proved crossing, not a sample. What is a blank cell entitled to claim?",
        num_file: include_str!("../../../docs/experiments/the-circle.num"),
    },
    StudioExperiment {
        id: "the-bowl",
        family: "three-readings",
        title: "The bowl",
        invitation: "This is the same formula as The circle, read as height. Where does the bowl meet nothing?",
        num_file: include_str!("../../../docs/experiments/the-bowl.num"),
    },
    StudioExperiment {
        id: "extra-knob",
        family: "named-sliders",
        title: "An extra knob",
        invitation: "Change b. What moved, and what stayed the same?",
        num_file: include_str!("../../../docs/experiments/extra-knob.num"),
    },
    StudioExperiment {
        id: "live-ratio",
        family: "named-sliders",
        title: "A live ratio",
        invitation: "Change p or q. When does the path close again?",
        num_file: include_str!("../../../docs/experiments/live-ratio.num"),
    },
    StudioExperiment {
        id: "the-parts",
        family: "overlay",
        title: "The parts",
        invitation: "Two curves share one window. Which one oscillates, and which one is shifted?",
        num_file: include_str!("../../../docs/experiments/the-parts.num"),
    },
    StudioExperiment {
        id: "the-sum",
        family: "overlay",
        title: "The sum",
        invitation: "The third curve is their sum. Where does it sit when they cancel?",
        num_file: include_str!("../../../docs/experiments/the-sum.num"),
    },
    StudioExperiment {
        id: "tresillo",
        family: "euclidean",
        title: "Tresillo",
        invitation: "Three hits among eight steps. Are they equally spaced, or only as even as eight allows?",
        num_file: include_str!("../../../docs/experiments/tresillo.num"),
    },
    StudioExperiment {
        id: "three-against-five",
        family: "euclidean",
        title: "Three against five",
        invitation: "Two patterns share eight steps. Where do three hits and five hits land together?",
        num_file: include_str!("../../../docs/experiments/three-against-five.num"),
    },
    StudioExperiment {
        id: "closing-voices",
        family: "two-voices",
        title: "Closing voices",
        invitation: "These are the two oscillators of A full return. Count the peaks. How many does each make in this window?",
        num_file: include_str!("../../../docs/experiments/closing-voices.num"),
    },
    StudioExperiment {
        id: "wandering-voices",
        family: "two-voices",
        title: "Wandering voices",
        invitation: "Compare with Closing voices. What would a common period require of both counts?",
        num_file: include_str!("../../../docs/experiments/wandering-voices.num"),
    },
];

/// Look up a bundled experiment by id, or by `experiment:<id>`.
#[must_use]
pub fn studio_experiment(id: &str) -> Option<StudioCreation> {
    studio_experiment_meta(id).map(StudioExperiment::creation)
}

/// Metadata for a bundled experiment id, or `experiment:<id>`.
#[must_use]
pub fn studio_experiment_meta(id: &str) -> Option<StudioExperiment> {
    let id = id.strip_prefix("experiment:").unwrap_or(id);
    STUDIO_EXPERIMENTS
        .iter()
        .copied()
        .find(|experiment| experiment.id == id)
}

/// Bundled experiments, optionally restricted to one family.
///
/// # Errors
/// Returns a guiding message when `family` is not a known family name.
pub fn studio_experiments_in(family: Option<&str>) -> Result<Vec<StudioExperiment>, String> {
    match family {
        None => Ok(STUDIO_EXPERIMENTS.to_vec()),
        Some(name) => {
            let listed: Vec<StudioExperiment> = STUDIO_EXPERIMENTS
                .iter()
                .copied()
                .filter(|experiment| experiment.family == name)
                .collect();
            if listed.is_empty() {
                let mut families = Vec::new();
                for experiment in STUDIO_EXPERIMENTS {
                    if !families.contains(&experiment.family) {
                        families.push(experiment.family);
                    }
                }
                Err(format!(
                    "Unknown experiment family '{name}'. Known families: {}.",
                    families.join(", ")
                ))
            } else {
                Ok(listed)
            }
        }
    }
}

/// The Studio family a catalog room offers as an optional construction.
///
/// Lissajous owns Returning home. Other rooms stay quiet rather than
/// inheriting a family they do not ask.
#[must_use]
pub fn studio_construction_family(room_id: &str) -> Option<&'static str> {
    match crate::canonical_room_id(room_id) {
        "lissajous" => Some("returning-home"),
        _ => None,
    }
}

/// First bundled experiment in a room's construction family, if it has one.
#[must_use]
pub fn first_studio_construction(room_id: &str) -> Option<StudioExperiment> {
    let family = studio_construction_family(room_id)?;
    studio_experiments_in(Some(family))
        .ok()
        .and_then(|listed| listed.into_iter().next())
}

/// The bundled experiment whose formula matches this creation, if any.
#[must_use]
pub fn studio_experiment_matching(creation: &StudioCreation) -> Option<StudioExperiment> {
    STUDIO_EXPERIMENTS.iter().copied().find(|experiment| {
        let bundled = experiment.creation();
        bundled.kind() == creation.kind()
            && bundled.source() == creation.source()
            && bundled.second_source() == creation.second_source()
            && bundled.extra_sources() == creation.extra_sources()
            && bundled.reading() == creation.reading()
    })
}

/// Neighbor in the same family, with no wrap. `delta` is typically -1 or 1.
///
/// After the last Returning home capsule, +1 opens the first two-voices
/// overlay. That is a transfer into the next contrast, not a wrap back to
/// A full return. PageUp from Closing voices returns to Another ratio.
#[must_use]
pub fn adjacent_studio_experiment(id: &str, delta: i32) -> Option<StudioExperiment> {
    let current = studio_experiment_meta(id)?;
    let family = studio_experiments_in(Some(current.family)).ok()?;
    let index = family
        .iter()
        .position(|experiment| experiment.id == current.id)?;
    let next = i32::try_from(index).ok()?.checked_add(delta)?;
    if next < 0 {
        return quest_predecessor(current.id);
    }
    family
        .get(usize::try_from(next).ok()?)
        .copied()
        .or_else(|| quest_successor(current.id))
}

fn quest_successor(id: &str) -> Option<StudioExperiment> {
    match id {
        "another-ratio" => studio_experiment_meta("closing-voices"),
        _ => None,
    }
}

fn quest_predecessor(id: &str) -> Option<StudioExperiment> {
    match id {
        "closing-voices" => studio_experiment_meta("another-ratio"),
        _ => None,
    }
}

/// Starter path after the last Returning home contrast.
///
/// Both frequencies are 1, so the trial reads period 1. The y ratio is not
/// a spoiler; changing it is the transfer.
#[must_use]
pub fn returning_home_transfer() -> StudioCreation {
    studio_experiment("another-ratio").expect("bundled another-ratio must parse")
}

/// Whether this creation is still the unedited transfer starter.
#[must_use]
pub fn is_returning_home_transfer(creation: &StudioCreation) -> bool {
    studio_experiment_matching(creation).is_some_and(|experiment| experiment.id == "another-ratio")
}

/// Next or previous creation in a construction family walk. No wrap.
#[must_use]
pub fn adjacent_construction_creation(
    creation: &StudioCreation,
    delta: i32,
) -> Option<StudioCreation> {
    let current = studio_experiment_matching(creation)?;
    adjacent_studio_experiment(current.id, delta).map(StudioExperiment::creation)
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
    /// One field over the plane, `f(x, y)` or `f(z)`.
    Field,
    /// Several graphs over one window, drawn together. The first sings.
    Program,
}

impl StudioKind {
    /// Stable lowercase capsule and protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Graph => "graph",
            Self::Parametric => "parametric",
            Self::Field => "field",
            Self::Program => "program",
        }
    }

    #[must_use]
    fn parse(value: &str) -> Option<Self> {
        match value {
            "graph" => Some(Self::Graph),
            "parametric" => Some(Self::Parametric),
            "field" => Some(Self::Field),
            "program" => Some(Self::Program),
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
/// honor is not only a machine `descends` link. The fifth adds a field over
/// the plane: a second independent variable, a stored 2D window, and which
/// reading of the field the plate asserts.
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
    more_sources: Vec<String>,
    xmin: f64,
    xmax: f64,
    ymin: Option<f64>,
    ymax: Option<f64>,
    a: f64,
    sliders: Vec<crate::slider::StudioSlider>,
    scale: StudioScale,
    reading: Option<FieldReading>,
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
        Self {
            source,
            second_source: None,
            more_sources: Vec::new(),
            xmin,
            xmax,
            ymin: None,
            ymax: None,
            a,
            sliders: Vec::new(),
            scale: StudioScale::Continuous,
            reading: None,
            title: None,
            author: None,
            credit: None,
            era: None,
            descends: None,
        }
        .bind_sliders()
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
        Self {
            source: x_source,
            second_source: Some(y_source),
            more_sources: Vec::new(),
            xmin: tmin,
            xmax: tmax,
            ymin: None,
            ymax: None,
            a,
            sliders: Vec::new(),
            scale: StudioScale::Continuous,
            reading: None,
            title: None,
            author: None,
            credit: None,
            era: None,
            descends: None,
        }
        .bind_sliders()
    }

    /// Build a validated field over a requested rectangle of the plane.
    ///
    /// The stored window is the rectangle the player asked for. Fitting that
    /// rectangle to a plate's cell shape happens at draw time, so a circle
    /// saved from a tall terminal cell reopens as the same mathematics on
    /// square pixels. `reading` is which proposition the plate asserts, and
    /// it is part of the artifact: a proved curve and a phase wheel of the
    /// same formula are not two skins of one creation.
    ///
    /// # Errors
    /// Returns a message if the source is invalid, the window or parameter
    /// are not finite increasing rectangles, or a zero reading is asked of a
    /// field that leaves the real line.
    pub fn new_field(
        source: impl Into<String>,
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
        a: f64,
        reading: FieldReading,
    ) -> Result<Self, String> {
        let source = source.into().trim().to_string();
        validate_share_source(&source)?;
        let expression = parse_field(&source)?;
        validate_share_numbers(xmin, xmax, a)?;
        validate_share_numbers(ymin, ymax, a)?;
        if reading == FieldReading::Zero && !is_real_valued(&expression) {
            return Err(
                "the zero reading needs a real-valued field; this one leaves the real line, \
                 so read its phase instead"
                    .to_string(),
            );
        }
        Self {
            source,
            second_source: None,
            more_sources: Vec::new(),
            xmin,
            xmax,
            ymin: Some(ymin),
            ymax: Some(ymax),
            a,
            sliders: Vec::new(),
            scale: StudioScale::Continuous,
            reading: Some(reading),
            title: None,
            author: None,
            credit: None,
            era: None,
            descends: None,
        }
        .bind_sliders()
    }

    /// Build an overlay program: two to [`MAX_PROGRAM_EXPRS`] graphs that share
    /// one window, knob, sliders, and scale. Every graph sings in WAV; MIDI
    /// keeps the first expression.
    ///
    /// # Errors
    /// Returns a message when the count is out of range, a part is empty or
    /// not a graph, or the window and parameter are invalid.
    pub fn new_program(
        sources: impl IntoIterator<Item = impl Into<String>>,
        xmin: f64,
        xmax: f64,
        a: f64,
    ) -> Result<Self, String> {
        let sources: Vec<String> = sources
            .into_iter()
            .map(|source| source.into().trim().to_string())
            .collect();
        let sources = validate_program_sources(sources)?;
        validate_share_numbers(xmin, xmax, a)?;
        let (source, rest) = sources.split_first().expect("validated program length");
        Self {
            source: source.clone(),
            second_source: None,
            more_sources: rest.to_vec(),
            xmin,
            xmax,
            ymin: None,
            ymax: None,
            a,
            sliders: Vec::new(),
            scale: StudioScale::Continuous,
            reading: None,
            title: None,
            author: None,
            credit: None,
            era: None,
            descends: None,
        }
        .bind_sliders()
    }

    /// Change which truth a field plate asserts, keeping the formula and window.
    ///
    /// # Errors
    /// Returns a message when this is not a field, or when the new reading
    /// cannot be asked of this expression.
    pub fn with_reading(mut self, reading: FieldReading) -> Result<Self, String> {
        if self.kind() != StudioKind::Field {
            return Err("only a field creation has a reading".to_string());
        }
        let expression = parse_field(&self.source)?;
        if reading == FieldReading::Zero && !is_real_valued(&expression) {
            return Err(
                "the zero reading needs a real-valued field; this one leaves the real line, \
                 so read its phase instead"
                    .to_string(),
            );
        }
        self.reading = Some(reading);
        Ok(self)
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
        if self.kind() == StudioKind::Field {
            return self.fork_field(source, None, title, author);
        }
        let mut child = match (self.kind(), source) {
            (StudioKind::Program, None) => {
                Self::new_program(self.graph_sources(), self.xmin, self.xmax, self.a)?
            }
            (StudioKind::Program, Some(source)) => {
                program_or_graph(source, self.xmin, self.xmax, self.a)?
            }
            (StudioKind::Graph, source) => {
                Self::new(source.unwrap_or(&self.source), self.xmin, self.xmax, self.a)?
            }
            (StudioKind::Parametric, None) => Self::new_parametric(
                &self.source,
                self.second_source.as_deref().expect("parametric y"),
                self.xmin,
                self.xmax,
                self.a,
            )?,
            (StudioKind::Parametric, Some(_)) => {
                return Err("a parametric fork needs both x(t) and y(t) replacements".to_string());
            }
            (StudioKind::Field, _) => unreachable!("field fork uses fork_field"),
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
        child = child.inherit_sliders(self)?;
        child.with_descends(&self.to_link())
    }

    /// Fork a field creation, optionally replacing the formula or the reading.
    /// The requested window, parameter, and era stay with the child.
    ///
    /// # Errors
    /// Returns a message for a non-field parent, an invalid replacement, or
    /// any invalid child field.
    pub fn fork_field(
        &self,
        source: Option<&str>,
        reading: Option<FieldReading>,
        title: Option<&str>,
        author: Option<&str>,
    ) -> Result<Self, String> {
        let Some(parent_reading) = self.reading else {
            return Err("the parent is not a field".to_string());
        };
        let ymin = self
            .ymin
            .ok_or_else(|| "the parent field has no ymin".to_string())?;
        let ymax = self
            .ymax
            .ok_or_else(|| "the parent field has no ymax".to_string())?;
        let mut child = Self::new_field(
            source.unwrap_or(&self.source),
            self.xmin,
            self.xmax,
            ymin,
            ymax,
            self.a,
            reading.unwrap_or(parent_reading),
        )?;
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
        child = child.inherit_sliders(self)?;
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
        child = child.inherit_sliders(self)?;
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

    /// Graph expressions in an overlay program, first-seen order. Empty when
    /// this is not a program.
    #[must_use]
    pub fn extra_sources(&self) -> &[String] {
        &self.more_sources
    }

    /// Every graph expression this creation draws, first-seen order.
    #[must_use]
    pub fn graph_sources(&self) -> Vec<String> {
        let mut sources = vec![self.source.clone()];
        sources.extend(self.more_sources.iter().cloned());
        sources
    }

    /// Whether this capsule is one graph, one parametric path, one field, or
    /// an overlay program.
    #[must_use]
    pub fn kind(&self) -> StudioKind {
        if self.reading.is_some() {
            StudioKind::Field
        } else if self.second_source.is_some() {
            StudioKind::Parametric
        } else if !self.more_sources.is_empty() {
            StudioKind::Program
        } else {
            StudioKind::Graph
        }
    }

    /// One canonical editor-facing formula label.
    #[must_use]
    pub fn editor_source(&self) -> String {
        if !self.more_sources.is_empty() {
            return self.graph_sources().join(" & ");
        }
        match &self.second_source {
            Some(y_source) => format!("x(t)={}; y(t)={y_source}", self.source),
            None => self.source.clone(),
        }
    }

    /// Lower edge of a field window, when this is a field.
    #[must_use]
    pub const fn ymin(&self) -> Option<f64> {
        self.ymin
    }

    /// Upper edge of a field window, when this is a field.
    #[must_use]
    pub const fn ymax(&self) -> Option<f64> {
        self.ymax
    }

    /// Which truth a field plate asserts, when this is a field.
    #[must_use]
    pub const fn reading(&self) -> Option<FieldReading> {
        self.reading
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

    /// Named sliders this formula binds, in first-seen order.
    #[must_use]
    pub fn sliders(&self) -> &[crate::slider::StudioSlider] {
        &self.sliders
    }

    /// Replace the named sliders. Every extra identifier in the formula must
    /// be present; a name the formula does not use is refused.
    ///
    /// # Errors
    /// Returns a message when the bindings do not match the formula.
    pub fn with_sliders(
        mut self,
        sliders: Vec<crate::slider::StudioSlider>,
    ) -> Result<Self, String> {
        let names = self.collect_slider_names()?;
        self.sliders = crate::slider::bind_sliders(&names, &sliders)?;
        Ok(self)
    }

    /// Copy matching parent sliders onto this child, keeping defaults for
    /// names the parent never bound.
    fn inherit_sliders(self, parent: &Self) -> Result<Self, String> {
        let names = self.collect_slider_names()?;
        let kept: Vec<crate::slider::StudioSlider> = parent
            .sliders
            .iter()
            .filter(|slider| names.iter().any(|name| name == slider.name()))
            .cloned()
            .collect();
        self.with_sliders(kept)
    }

    fn collect_slider_names(&self) -> Result<Vec<String>, String> {
        let first = if self.kind() == StudioKind::Field {
            parse_field(&self.source)?
        } else {
            parse(&self.source)?
        };
        let mut names = crate::slider::collect_slider_names(&first);
        if let Some(second) = &self.second_source {
            for name in crate::slider::collect_slider_names(&parse(second)?) {
                if !names.iter().any(|existing| existing == &name) {
                    names.push(name);
                }
            }
        }
        for extra in &self.more_sources {
            for name in crate::slider::collect_slider_names(&parse(extra)?) {
                if !names.iter().any(|existing| existing == &name) {
                    names.push(name);
                }
            }
        }
        Ok(names)
    }

    fn bind_sliders(self) -> Result<Self, String> {
        let names = self.collect_slider_names()?;
        let sliders = crate::slider::bind_sliders(&names, &[])?;
        Ok(Self { sliders, ..self })
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
    /// the capsule carries two coordinate expressions, or a field plate when
    /// it carries a reading.
    /// Graphs auto-scale y; parametric paths fit both coordinates with equal
    /// physical units, including the terminal character aspect. Fields draw
    /// the stored reading over the requested window, fitted to tall cells.
    ///
    /// # Errors
    /// Returns a parser, geometry, or all-undefined diagnostic.
    pub fn plot_text(&self, width: usize, height: usize) -> Result<StudioPlot, String> {
        if self.kind() == StudioKind::Field {
            return self.plot_field_text(width, height, 0.5);
        }
        let program = self.program()?;
        plot_program_text(
            &program,
            self.xmin,
            self.xmax,
            self.a,
            &self.sliders,
            width,
            height,
        )
        .map_err(|error| error.message().to_string())
    }

    /// Render this field at a chosen character aspect.
    ///
    /// `char_aspect` is how wide a cell is compared with its height: half for
    /// a terminal, one for square pixels. The stored window is what was asked
    /// for; the returned plot reports the rectangle actually drawn.
    ///
    /// # Errors
    /// Returns a parser or field-drawing diagnostic.
    pub fn plot_field_text(
        &self,
        width: usize,
        height: usize,
        char_aspect: f64,
    ) -> Result<StudioPlot, String> {
        let reading = self
            .reading
            .ok_or_else(|| "only a field creation draws a field plate".to_string())?;
        let ymin = self
            .ymin
            .ok_or_else(|| "a field creation needs ymin".to_string())?;
        let ymax = self
            .ymax
            .ok_or_else(|| "a field creation needs ymax".to_string())?;
        let expression = parse_field(&self.source)?;
        let plate = field::draw_named(
            &expression,
            reading,
            (self.xmin, self.xmax),
            (ymin, ymax),
            self.a,
            &self.sliders,
            (width, height),
            char_aspect,
        )
        .map_err(|error| error.message().to_string())?;
        Ok(StudioPlot {
            text: plate.text,
            xmin: plate.x_bounds.0,
            xmax: plate.x_bounds.1,
            ymin: plate.y_bounds.0,
            ymax: plate.y_bounds.1,
        })
    }

    /// Render this exact creation's voice. A parametric creation sings its
    /// y-coordinate over `t`; the x-coordinate remains the visible path.
    /// Height and phase fields sing the stored reading along the real axis.
    /// The zero reading stays silent: it is a proof, not a sample. Overlay
    /// programs mix every graph; [`Self::to_midi_melody`] keeps the first graph.
    #[must_use]
    pub fn to_melody(&self, notes: usize) -> SoundSpec {
        match self.program() {
            Ok(program) => program.to_melody(
                self.xmin,
                self.xmax,
                notes,
                self.a,
                &self.sliders,
                self.scale,
            ),
            Err(_) => silent_spec(),
        }
    }

    /// MIDI voice of this creation. Overlay programs keep the first graph
    /// because the file is one channel of pitch bend. WAV mixes every graph.
    #[must_use]
    pub fn to_midi_melody(&self, notes: usize) -> SoundSpec {
        match self.program() {
            Ok(program) => program.to_midi_melody(
                self.xmin,
                self.xmax,
                notes,
                self.a,
                &self.sliders,
                self.scale,
            ),
            Err(_) => silent_spec(),
        }
    }

    /// Pattern-text view of integer 0/1 graphs, one row per overlay curve.
    ///
    /// Empty when this is not a step pattern: the window must be an integer
    /// span of 1 to [`MAX_EUCLID_STEPS`] steps, and every graph must stay 0
    /// or 1 across each step. This is a tracker reading of the same formula
    /// that draws, not a second document.
    #[must_use]
    pub fn pattern_rows(&self) -> Vec<String> {
        match self.kind() {
            StudioKind::Graph | StudioKind::Program => {}
            StudioKind::Parametric | StudioKind::Field => return Vec::new(),
        }
        let Ok(program) = self.program() else {
            return Vec::new();
        };
        let expressions: &[Expr] = match program.kind() {
            StudioKind::Program => program.overlay_expressions(),
            StudioKind::Graph => std::slice::from_ref(program.voice_expression()),
            StudioKind::Parametric | StudioKind::Field => return Vec::new(),
        };
        let mut rows = Vec::with_capacity(expressions.len());
        for expression in expressions {
            let Some(row) = pattern_row(expression, self.xmin, self.xmax, self.a, &self.sliders)
            else {
                return Vec::new();
            };
            rows.push(row);
        }
        rows
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
    /// version 2, a parametric pair or stored scale uses version 3, prose
    /// credit uses version 4, a field uses version 5, named sliders use
    /// version 6, and overlay programs use version 7.
    #[must_use]
    pub fn to_num_file(&self) -> String {
        let version = if self.kind() == StudioKind::Program {
            7
        } else if !self.sliders.is_empty() {
            6
        } else if self.kind() == StudioKind::Field {
            5
        } else if self.credit.is_some() {
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
        } else if self.kind() == StudioKind::Field {
            out.push_str(&format!(
                "kind=field\nexpr={}\nxmin={}\nxmax={}\nymin={}\nymax={}\nreading={}\na={}\n",
                self.source,
                format_share_number(self.xmin),
                format_share_number(self.xmax),
                format_share_number(self.ymin.expect("field has ymin")),
                format_share_number(self.ymax.expect("field has ymax")),
                self.reading.expect("field has reading").name(),
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
                None => {
                    out.push_str(&format!("expr={}\n", self.source));
                    for extra in &self.more_sources {
                        out.push_str(&format!("expr={extra}\n"));
                    }
                    out.push_str(&format!(
                        "xmin={}\nxmax={}\na={}\n",
                        format_share_number(self.xmin),
                        format_share_number(self.xmax),
                        format_share_number(self.a)
                    ));
                }
            }
            out.push_str(&format!("scale={}\n", self.scale.name()));
        }
        for slider in &self.sliders {
            out.push_str(&format!("slider={}\n", slider.to_file_value()));
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

    /// Parse a `.num` Studio file, version 1 through 7.
    ///
    /// Version 1 rejects the metadata fields rather than ignoring them, so a
    /// file cannot claim the old header while smuggling new content. A header
    /// past version 7 is refused by name: a future capsule is a fact to
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
            Some("NUMINOUS_STUDIO 5") => 5,
            Some("NUMINOUS_STUDIO 6") => 6,
            Some("NUMINOUS_STUDIO 7") => 7,
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
        let mut ymin: Option<f64> = None;
        let mut ymax: Option<f64> = None;
        let mut tmin: Option<f64> = None;
        let mut tmax: Option<f64> = None;
        let mut a: Option<f64> = None;
        let mut scale: Option<StudioScale> = None;
        let mut reading: Option<FieldReading> = None;
        let mut title: Option<String> = None;
        let mut author: Option<String> = None;
        let mut credit: Option<String> = None;
        let mut era: Option<crate::era::Era> = None;
        let mut descends: Option<String> = None;
        let mut sliders: Vec<crate::slider::StudioSlider> = Vec::new();
        let mut extra_exprs: Vec<String> = Vec::new();
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
                "ymin" | "ymax" | "reading" if version < 5 => {
                    return Err(format!(
                        "Studio .num field '{key}' needs a NUMINOUS_STUDIO 5 header"
                    ));
                }
                "slider" if version < 6 => {
                    return Err(
                        "Studio .num field 'slider' needs a NUMINOUS_STUDIO 6 header".to_string(),
                    );
                }
                "kind" if value == "program" && version < 7 => {
                    return Err(
                        "Studio .num field 'kind=program' needs a NUMINOUS_STUDIO 7 header"
                            .to_string(),
                    );
                }
                "kind" if kind.is_none() => {
                    kind = Some(
                        StudioKind::parse(value)
                            .ok_or_else(|| format!("unknown Studio kind '{value}'"))?,
                    );
                }
                "expr" if source.is_none() => source = Some(value.to_string()),
                "expr" if version >= 7 => extra_exprs.push(value.to_string()),
                "xexpr" if x_source.is_none() => x_source = Some(value.to_string()),
                "yexpr" if y_source.is_none() => y_source = Some(value.to_string()),
                "xmin" if xmin.is_none() => xmin = Some(parse_share_number("xmin", value)?),
                "xmax" if xmax.is_none() => xmax = Some(parse_share_number("xmax", value)?),
                "ymin" if ymin.is_none() => ymin = Some(parse_share_number("ymin", value)?),
                "ymax" if ymax.is_none() => ymax = Some(parse_share_number("ymax", value)?),
                "tmin" if tmin.is_none() => tmin = Some(parse_share_number("tmin", value)?),
                "tmax" if tmax.is_none() => tmax = Some(parse_share_number("tmax", value)?),
                "a" if a.is_none() => a = Some(parse_share_number("a", value)?),
                "scale" if scale.is_none() => {
                    if version == 5 || kind == Some(StudioKind::Field) {
                        return Err("a field Studio capsule has no scale".to_string());
                    }
                    scale = Some(
                        StudioScale::parse(value)
                            .ok_or_else(|| format!("unknown Studio scale '{value}'"))?,
                    );
                }
                "reading" if reading.is_none() => {
                    reading = Some(
                        FieldReading::parse(value)
                            .ok_or_else(|| format!("unknown Studio reading '{value}'"))?,
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
                "slider" => sliders.push(crate::slider::StudioSlider::from_file_value(value)?),
                "kind" | "expr" | "xexpr" | "yexpr" | "xmin" | "xmax" | "ymin" | "ymax"
                | "tmin" | "tmax" | "a" | "scale" | "reading" | "title" | "author" | "era"
                | "descends" | "credit" => {
                    return Err(format!("duplicate Studio .num field '{key}'"));
                }
                other => return Err(format!("unknown Studio .num field '{other}'")),
            }
        }
        let a = a.ok_or_else(|| "missing a".to_string())?;
        let mut creation = if version < 3 {
            if ymin.is_some() || ymax.is_some() || reading.is_some() {
                return Err("graph Studio capsule mixes field fields".to_string());
            }
            Self::new(
                source.ok_or_else(|| "missing Studio expression".to_string())?,
                xmin.ok_or_else(|| "missing xmin".to_string())?,
                xmax.ok_or_else(|| "missing xmax".to_string())?,
                a,
            )?
        } else if version == 5 || kind == Some(StudioKind::Field) {
            let kind = kind.ok_or_else(|| "missing Studio kind".to_string())?;
            if kind != StudioKind::Field {
                return Err("a NUMINOUS_STUDIO 5 capsule needs kind=field".to_string());
            }
            if version < 5 {
                return Err(
                    "Studio .num field 'kind=field' needs a NUMINOUS_STUDIO 5 header".to_string(),
                );
            }
            if x_source.is_some()
                || y_source.is_some()
                || tmin.is_some()
                || tmax.is_some()
                || scale.is_some()
            {
                return Err("field Studio capsule mixes graph or parametric fields".to_string());
            }
            Self::new_field(
                source.ok_or_else(|| "missing Studio expression".to_string())?,
                xmin.ok_or_else(|| "missing xmin".to_string())?,
                xmax.ok_or_else(|| "missing xmax".to_string())?,
                ymin.ok_or_else(|| "missing ymin".to_string())?,
                ymax.ok_or_else(|| "missing ymax".to_string())?,
                a,
                reading.ok_or_else(|| "missing Studio reading".to_string())?,
            )?
        } else {
            let kind = kind.ok_or_else(|| "missing Studio kind".to_string())?;
            if kind == StudioKind::Field {
                return Err(
                    "Studio .num field 'kind=field' needs a NUMINOUS_STUDIO 5 header".to_string(),
                );
            }
            let scale = scale.ok_or_else(|| "missing Studio scale".to_string())?;
            if ymin.is_some() || ymax.is_some() || reading.is_some() {
                return Err("graph Studio capsule mixes field fields".to_string());
            }
            let creation = match kind {
                StudioKind::Graph => {
                    if x_source.is_some() || y_source.is_some() || tmin.is_some() || tmax.is_some()
                    {
                        return Err("graph Studio capsule mixes parametric fields".to_string());
                    }
                    if !extra_exprs.is_empty() {
                        return Err(
                            "extra expr lines need kind=program and a NUMINOUS_STUDIO 7 header"
                                .to_string(),
                        );
                    }
                    Self::new(
                        source.ok_or_else(|| "missing Studio expression".to_string())?,
                        xmin.ok_or_else(|| "missing xmin".to_string())?,
                        xmax.ok_or_else(|| "missing xmax".to_string())?,
                        a,
                    )?
                }
                StudioKind::Program => {
                    if version < 7 {
                        return Err(
                            "Studio .num field 'kind=program' needs a NUMINOUS_STUDIO 7 header"
                                .to_string(),
                        );
                    }
                    if x_source.is_some() || y_source.is_some() || tmin.is_some() || tmax.is_some()
                    {
                        return Err("program Studio capsule mixes parametric fields".to_string());
                    }
                    let mut sources =
                        vec![source.ok_or_else(|| "missing Studio expression".to_string())?];
                    sources.extend(extra_exprs);
                    Self::new_program(
                        sources,
                        xmin.ok_or_else(|| "missing xmin".to_string())?,
                        xmax.ok_or_else(|| "missing xmax".to_string())?,
                        a,
                    )?
                }
                StudioKind::Parametric => {
                    if source.is_some() || xmin.is_some() || xmax.is_some() {
                        return Err("parametric Studio capsule mixes graph fields".to_string());
                    }
                    if !extra_exprs.is_empty() {
                        return Err(
                            "parametric Studio capsule mixes overlay expr lines".to_string()
                        );
                    }
                    Self::new_parametric(
                        x_source.ok_or_else(|| "missing parametric x expression".to_string())?,
                        y_source.ok_or_else(|| "missing parametric y expression".to_string())?,
                        tmin.ok_or_else(|| "missing tmin".to_string())?,
                        tmax.ok_or_else(|| "missing tmax".to_string())?,
                        a,
                    )?
                }
                StudioKind::Field => unreachable!("field kind needs version 5"),
            };
            creation.with_scale(scale)
        };
        if !creation.sliders.is_empty() && version < 6 {
            return Err("named sliders need a NUMINOUS_STUDIO 6 header".to_string());
        }
        if !sliders.is_empty() {
            creation = creation.with_sliders(sliders)?;
        } else if version >= 6 && !creation.sliders.is_empty() {
            return Err(
                "a NUMINOUS_STUDIO 6 capsule that names sliders needs slider lines".to_string(),
            );
        }
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
    /// Native links, `.num` text, and bundled experiment ids share one bounded
    /// input door. Filesystem paths are deliberately not accepted here: a face
    /// that owns path access must use [`Self::from_num_path`] and make that
    /// capability explicit.
    ///
    /// # Errors
    /// Returns a message when the input is neither a valid native link nor a
    /// valid `.num` document.
    pub fn from_capsule(input: &str) -> Result<Self, String> {
        if let Some(creation) = studio_experiment(input) {
            return Ok(creation);
        }
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
        let mut link = if self.kind() == StudioKind::Field {
            format!(
                "numinous://studio?kind=field&expr={}&xmin={}&xmax={}&ymin={}&ymax={}&reading={}&a={}",
                percent_encode(&self.source),
                format_share_number(self.xmin),
                format_share_number(self.xmax),
                format_share_number(self.ymin.expect("field has ymin")),
                format_share_number(self.ymax.expect("field has ymax")),
                self.reading.expect("field has reading").name(),
                format_share_number(self.a)
            )
        } else if self.kind() == StudioKind::Graph && self.scale == StudioScale::Continuous {
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
                None => {
                    link.push_str(&format!(
                        "&expr={}&xmin={}&xmax={}&a={}",
                        percent_encode(&self.source),
                        format_share_number(self.xmin),
                        format_share_number(self.xmax),
                        format_share_number(self.a)
                    ));
                    for extra in &self.more_sources {
                        link.push_str(&format!("&expr={}", percent_encode(extra)));
                    }
                }
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
        for slider in &self.sliders {
            link.push_str(&format!(
                "&slider={}",
                percent_encode(&slider.to_file_value())
            ));
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
        let mut ymin: Option<f64> = None;
        let mut ymax: Option<f64> = None;
        let mut tmin: Option<f64> = None;
        let mut tmax: Option<f64> = None;
        let mut a: Option<f64> = None;
        let mut scale: Option<StudioScale> = None;
        let mut reading: Option<FieldReading> = None;
        let mut title: Option<String> = None;
        let mut author: Option<String> = None;
        let mut credit: Option<String> = None;
        let mut era: Option<crate::era::Era> = None;
        let mut sliders: Vec<crate::slider::StudioSlider> = Vec::new();
        let mut extra_exprs: Vec<String> = Vec::new();
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
                "expr" => extra_exprs.push(percent_decode(value)?),
                "xexpr" if x_source.is_none() => x_source = Some(percent_decode(value)?),
                "yexpr" if y_source.is_none() => y_source = Some(percent_decode(value)?),
                "xmin" if xmin.is_none() => xmin = Some(parse_share_number("xmin", value)?),
                "xmax" if xmax.is_none() => xmax = Some(parse_share_number("xmax", value)?),
                "ymin" if ymin.is_none() => ymin = Some(parse_share_number("ymin", value)?),
                "ymax" if ymax.is_none() => ymax = Some(parse_share_number("ymax", value)?),
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
                "reading" if reading.is_none() => {
                    let decoded = percent_decode(value)?;
                    reading = Some(
                        FieldReading::parse(&decoded)
                            .ok_or_else(|| format!("unknown Studio reading '{decoded}'"))?,
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
                "slider" => {
                    let decoded = percent_decode(value)?;
                    sliders.push(crate::slider::StudioSlider::from_file_value(&decoded)?);
                }
                "kind" | "xexpr" | "yexpr" | "xmin" | "xmax" | "ymin" | "ymax" | "tmin"
                | "tmax" | "a" | "scale" | "reading" | "title" | "author" | "era" | "credit" => {
                    return Err(format!("duplicate Studio link field '{key}'"));
                }
                other => return Err(format!("unknown Studio link field '{other}'")),
            }
        }
        let a = a.ok_or_else(|| "missing a".to_string())?;
        if !extra_exprs.is_empty() && kind != Some(StudioKind::Program) {
            return Err("extra expr parameters need kind=program".to_string());
        }
        let mut creation = match kind {
            None => {
                if x_source.is_some()
                    || y_source.is_some()
                    || tmin.is_some()
                    || tmax.is_some()
                    || scale.is_some()
                    || ymin.is_some()
                    || ymax.is_some()
                    || reading.is_some()
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
                if x_source.is_some()
                    || y_source.is_some()
                    || tmin.is_some()
                    || tmax.is_some()
                    || ymin.is_some()
                    || ymax.is_some()
                    || reading.is_some()
                {
                    return Err("graph Studio link mixes parametric or field fields".to_string());
                }
                Self::new(
                    source.ok_or_else(|| "missing Studio expression".to_string())?,
                    xmin.ok_or_else(|| "missing xmin".to_string())?,
                    xmax.ok_or_else(|| "missing xmax".to_string())?,
                    a,
                )?
                .with_scale(scale.ok_or_else(|| "missing Studio scale".to_string())?)
            }
            Some(StudioKind::Program) => {
                if x_source.is_some()
                    || y_source.is_some()
                    || tmin.is_some()
                    || tmax.is_some()
                    || ymin.is_some()
                    || ymax.is_some()
                    || reading.is_some()
                {
                    return Err("program Studio link mixes parametric or field fields".to_string());
                }
                let mut sources =
                    vec![source.ok_or_else(|| "missing Studio expression".to_string())?];
                sources.extend(extra_exprs);
                Self::new_program(
                    sources,
                    xmin.ok_or_else(|| "missing xmin".to_string())?,
                    xmax.ok_or_else(|| "missing xmax".to_string())?,
                    a,
                )?
                .with_scale(scale.ok_or_else(|| "missing Studio scale".to_string())?)
            }
            Some(StudioKind::Parametric) => {
                if source.is_some()
                    || xmin.is_some()
                    || xmax.is_some()
                    || ymin.is_some()
                    || ymax.is_some()
                    || reading.is_some()
                {
                    return Err("parametric Studio link mixes graph or field fields".to_string());
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
            Some(StudioKind::Field) => {
                if x_source.is_some()
                    || y_source.is_some()
                    || tmin.is_some()
                    || tmax.is_some()
                    || scale.is_some()
                {
                    return Err("field Studio link mixes graph or parametric fields".to_string());
                }
                Self::new_field(
                    source.ok_or_else(|| "missing Studio expression".to_string())?,
                    xmin.ok_or_else(|| "missing xmin".to_string())?,
                    xmax.ok_or_else(|| "missing xmax".to_string())?,
                    ymin.ok_or_else(|| "missing ymin".to_string())?,
                    ymax.ok_or_else(|| "missing ymax".to_string())?,
                    a,
                    reading.ok_or_else(|| "missing Studio reading".to_string())?,
                )?
            }
        };
        if !sliders.is_empty() {
            creation = creation.with_sliders(sliders)?;
        }
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

fn validate_program_sources(sources: Vec<String>) -> Result<Vec<String>, String> {
    if !(2..=MAX_PROGRAM_EXPRS).contains(&sources.len()) {
        return Err(format!(
            "an overlay program needs 2 to {MAX_PROGRAM_EXPRS} graph expressions"
        ));
    }
    for source in &sources {
        validate_share_source(source)?;
        let expression = parse(source)?;
        if uses_field_vocabulary(&expression) {
            return Err(
                "an overlay program is graphs over x, not a field; omit y, z, i, re, im, arg, and conj"
                    .to_string(),
            );
        }
    }
    Ok(sources)
}

fn program_or_graph(source: &str, xmin: f64, xmax: f64, a: f64) -> Result<StudioCreation, String> {
    if let Some(parts) = split_program_editor(source) {
        StudioCreation::new_program(parts, xmin, xmax, a)
    } else {
        StudioCreation::new(source, xmin, xmax, a)
    }
}

/// Split editor text on `&` when it is an overlay program. `None` when the
/// source has no overlay separator.
fn split_program_editor(source: &str) -> Option<Vec<String>> {
    if !source.contains('&') {
        return None;
    }
    Some(
        source
            .split('&')
            .map(|part| part.trim().to_string())
            .collect(),
    )
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
    /// The variable `x`, or `t` in a parametric pair. In a field it is the
    /// real coordinate of the sampled point.
    Var,
    /// The imaginary coordinate `y` of a sampled field point. A field leaf:
    /// a curve has no second coordinate to read, so it has no real value.
    VarIm,
    /// The whole sampled point `z` of a field. A field leaf, for the same
    /// reason: a point of the plane is not a number on the line.
    Point,
    /// The imaginary unit `i`. A field leaf.
    ImagUnit,
    /// The adjustable parameter `a`.
    Param,
    /// A named slider: an extra finite parameter the formula bound.
    Slider(String),
    /// Unary negation.
    Neg(Box<Expr>),
    /// A binary operation.
    Bin(Op, Box<Expr>, Box<Expr>),
    /// A function call, e.g. `sin(...)`.
    Call(Func, Box<Expr>),
    /// A two-argument function call, e.g. `min(..., ...)`.
    PairCall(PairFunc, Box<Expr>, Box<Expr>),
    /// An explicit step pattern, `pat(x..x..x.)` or the editor form `x..x..x.`.
    ///
    /// Each mark is one step: `x` is a hit, `.` is a rest. The sample at `x`
    /// is 1 on a hit and 0 on a rest, wrapping like `euclid`.
    Pattern(Vec<bool>),
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
    /// One field over the plane.
    Field {
        /// Canonical source.
        source: String,
        /// Parsed field expression.
        expression: Expr,
        /// Which truth about the field the plate and voice use.
        reading: FieldReading,
    },
    /// Several graphs over one window. Every graph sings in WAV; MIDI keeps
    /// the first expression.
    Program {
        /// Canonical sources, first-seen order.
        sources: Vec<String>,
        /// Parsed expressions in the same order.
        expressions: Vec<Expr>,
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
        if source.contains('&') && source.contains(';') {
            return Err(
                "overlay programs use '&' between graphs; parametric pairs use one ';'".to_string(),
            );
        }
        if let Some(parts) = split_program_editor(source) {
            return Self::program(parts);
        }
        if !source.contains(';') {
            validate_share_source(source)?;
            let field = parse_field(source)?;
            if uses_field_vocabulary(&field) {
                return Ok(Self::Field {
                    source: source.to_string(),
                    expression: field,
                    reading: FieldReading::default(),
                });
            }
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

    /// Parse an overlay program of graphs.
    ///
    /// # Errors
    /// Returns a diagnostic when the count or any part is invalid.
    pub fn program(sources: impl IntoIterator<Item = impl Into<String>>) -> Result<Self, String> {
        let sources = validate_program_sources(
            sources
                .into_iter()
                .map(|source| source.into().trim().to_string())
                .collect(),
        )?;
        let expressions = sources
            .iter()
            .map(|source| parse(source))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self::Program {
            sources,
            expressions,
        })
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

    /// Parse a field program.
    ///
    /// # Errors
    /// Returns an expression validation or parser diagnostic.
    pub fn field(source: &str) -> Result<Self, String> {
        Self::field_with_reading(source, FieldReading::default())
    }

    /// Parse a field program with a stored reading.
    ///
    /// # Errors
    /// Returns an expression validation or parser diagnostic.
    pub fn field_with_reading(source: &str, reading: FieldReading) -> Result<Self, String> {
        validate_share_source(source)?;
        Ok(Self::Field {
            source: source.to_string(),
            expression: parse_field(source)?,
            reading,
        })
    }

    /// Keep this program's formula and bind a field reading.
    ///
    /// Graphs, pairs, and overlay programs ignore the reading.
    #[must_use]
    pub fn with_reading(self, reading: FieldReading) -> Self {
        match self {
            Self::Field {
                source, expression, ..
            } => Self::Field {
                source,
                expression,
                reading,
            },
            other => other,
        }
    }

    /// Stored field reading, when this program is a field.
    #[must_use]
    pub const fn reading(&self) -> Option<FieldReading> {
        match self {
            Self::Field { reading, .. } => Some(*reading),
            _ => None,
        }
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
        match creation.kind() {
            StudioKind::Field => {
                Self::field_with_reading(creation.source(), creation.reading().unwrap_or_default())
            }
            StudioKind::Parametric => Self::parametric(
                creation.source(),
                creation
                    .second_source()
                    .ok_or_else(|| "parametric creation is missing y(t)".to_string())?,
            ),
            StudioKind::Graph => Self::graph(creation.source()),
            StudioKind::Program => Self::program(creation.graph_sources()),
        }
    }

    /// Mathematical form of this program.
    #[must_use]
    pub fn kind(&self) -> StudioKind {
        match self {
            Self::Graph { .. } => StudioKind::Graph,
            Self::Parametric { .. } => StudioKind::Parametric,
            Self::Field { .. } => StudioKind::Field,
            Self::Program { .. } => StudioKind::Program,
        }
    }

    /// Canonical editor text.
    #[must_use]
    pub fn editor_source(&self) -> String {
        match self {
            Self::Graph { source, .. } | Self::Field { source, .. } => source.clone(),
            Self::Parametric {
                x_source, y_source, ..
            } => format!("x(t)={x_source}; y(t)={y_source}"),
            Self::Program { sources, .. } => sources.join(" & "),
        }
    }

    /// Source fields without editor labels.
    #[must_use]
    pub fn sources(&self) -> (&str, Option<&str>) {
        match self {
            Self::Graph { source, .. } | Self::Field { source, .. } => (source, None),
            Self::Parametric {
                x_source, y_source, ..
            } => (x_source, Some(y_source)),
            Self::Program { sources, .. } => (sources[0].as_str(), None),
        }
    }

    /// Evaluate one graph input or parametric time into a planar point.
    #[must_use]
    pub fn point(&self, input: f64, a: f64) -> Option<(f64, f64)> {
        self.point_named(input, a, &[])
    }

    /// Evaluate one input with named sliders bound.
    #[must_use]
    pub fn point_named(
        &self,
        input: f64,
        a: f64,
        sliders: &[crate::slider::StudioSlider],
    ) -> Option<(f64, f64)> {
        let point = match self {
            Self::Graph { expression, .. } => (input, eval_named(expression, input, a, sliders)),
            Self::Parametric {
                x_expression,
                y_expression,
                ..
            } => (
                eval_named(x_expression, input, a, sliders),
                eval_named(y_expression, input, a, sliders),
            ),
            Self::Field { .. } => return None,
            Self::Program { expressions, .. } => {
                (input, eval_named(&expressions[0], input, a, sliders))
            }
        };
        (point.0.is_finite() && point.1.is_finite()).then_some(point)
    }

    /// Overlay graph expressions, in source order. Empty when this is not a
    /// program of graphs.
    #[must_use]
    pub fn overlay_expressions(&self) -> &[Expr] {
        match self {
            Self::Program { expressions, .. } => expressions,
            _ => &[],
        }
    }

    /// Lead expression when this program sings. Graphs sing their y value;
    /// parametric paths sing their y coordinate over `t`. Height and phase
    /// fields sing along the real axis. Overlay programs keep this first
    /// graph for MIDI; [`Self::to_melody`] mixes every graph.
    #[must_use]
    pub fn voice_expression(&self) -> &Expr {
        match self {
            Self::Graph { expression, .. } | Self::Field { expression, .. } => expression,
            Self::Parametric { y_expression, .. } => y_expression,
            Self::Program { expressions, .. } => &expressions[0],
        }
    }

    /// WAV and live voice. Overlay programs mix every graph. Height and
    /// phase fields sing along the real axis; the zero reading is silent.
    #[must_use]
    pub fn to_melody(
        &self,
        xmin: f64,
        xmax: f64,
        notes: usize,
        a: f64,
        sliders: &[crate::slider::StudioSlider],
        scale: StudioScale,
    ) -> SoundSpec {
        match self {
            Self::Field {
                expression,
                reading,
                ..
            } => field_to_melody(expression, *reading, xmin, xmax, notes, a, sliders, scale),
            Self::Program { expressions, .. } => {
                mix_overlay_melodies(expressions, xmin, xmax, notes, a, sliders, scale)
            }
            _ => to_melody_with_scale_named(
                self.voice_expression(),
                xmin,
                xmax,
                notes,
                a,
                sliders,
                scale,
            ),
        }
    }

    /// MIDI voice. Overlay programs keep the first graph. A field's MIDI
    /// is the same real-axis voice as its WAV.
    #[must_use]
    pub fn to_midi_melody(
        &self,
        xmin: f64,
        xmax: f64,
        notes: usize,
        a: f64,
        sliders: &[crate::slider::StudioSlider],
        scale: StudioScale,
    ) -> SoundSpec {
        match self {
            Self::Field {
                expression,
                reading,
                ..
            } => field_to_melody(expression, *reading, xmin, xmax, notes, a, sliders, scale),
            _ => to_melody_with_scale_named(
                self.voice_expression(),
                xmin,
                xmax,
                notes,
                a,
                sliders,
                scale,
            ),
        }
    }

    /// Unique slider names this program binds, in first-seen order.
    #[must_use]
    pub fn slider_names(&self) -> Vec<String> {
        match self {
            Self::Graph { expression, .. } | Self::Field { expression, .. } => {
                crate::slider::collect_slider_names(expression)
            }
            Self::Parametric {
                x_expression,
                y_expression,
                ..
            } => {
                let mut names = crate::slider::collect_slider_names(x_expression);
                for name in crate::slider::collect_slider_names(y_expression) {
                    if !names.iter().any(|existing| existing == &name) {
                        names.push(name);
                    }
                }
                names
            }
            Self::Program { expressions, .. } => {
                let mut names = Vec::new();
                for expression in expressions {
                    for name in crate::slider::collect_slider_names(expression) {
                        if !names.iter().any(|existing| existing == &name) {
                            names.push(name);
                        }
                    }
                }
                names
            }
        }
    }
}

/// Whether a parsed expression uses the field grammar's extra vocabulary.
///
/// `y`, `z`, `i`, `re`, `im`, `arg`, and `conj` are the leaves and readers a
/// curve cannot name. A formula that never uses them is a graph even when
/// parsed as a field, so `sin(x)` stays a graph.
#[must_use]
pub fn uses_field_vocabulary(expression: &Expr) -> bool {
    match expression {
        Expr::VarIm | Expr::Point | Expr::ImagUnit => true,
        Expr::Call(Func::Re | Func::Im | Func::Arg | Func::Conj, _) => true,
        Expr::Num(_) | Expr::Var | Expr::Param | Expr::Slider(_) | Expr::Pattern(_) => false,
        Expr::Neg(inner) | Expr::Call(_, inner) => uses_field_vocabulary(inner),
        Expr::Bin(_, lhs, rhs) | Expr::PairCall(_, lhs, rhs) => {
            uses_field_vocabulary(lhs) || uses_field_vocabulary(rhs)
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
    /// Real part.
    Re,
    /// Imaginary part.
    Im,
    /// Principal argument, in `(-pi, pi]`. The origin has none.
    Arg,
    /// Complex conjugate.
    Conj,
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
    /// Euclidean rhythm: `hits` onsets spread as evenly as possible over `steps`.
    Euclid,
}

/// One sample of `euclid(hits, steps)` at `x`.
///
/// The step index is `floor(x)` wrapped into `n = floor(steps)` positions.
/// An onset sits at index `i` when `(i * k) rem n < k`, with `k` the floored
/// hit count clamped into `[0, n]`. That residue test puts an onset at 0 and
/// spreads the rest as evenly as an integer placement allows. Nonfinite
/// input, `n < 1`, or `n` above [`MAX_EUCLID_STEPS`] is undefined.
pub(crate) fn euclid_pulse(x: f64, hits: f64, steps: f64) -> f64 {
    if !x.is_finite() || !hits.is_finite() || !steps.is_finite() {
        return f64::NAN;
    }
    let n = steps.floor();
    if n < 1.0 || n > MAX_EUCLID_STEPS as f64 {
        return f64::NAN;
    }
    let k = hits.floor().clamp(0.0, n);
    let i = x.floor().rem_euclid(n);
    if (i * k).rem_euclid(n) < k { 1.0 } else { 0.0 }
}

/// One sample of `pat(x..x..x.)` at `x`.
///
/// The step index is `floor(x)` wrapped into `n` positions, the length of
/// the mark string. A hit is 1, a rest is 0. Nonfinite input or an empty
/// pattern is undefined.
pub(crate) fn pattern_pulse(x: f64, hits: &[bool]) -> f64 {
    let n = hits.len();
    if !x.is_finite() || n == 0 || n > MAX_EUCLID_STEPS {
        return f64::NAN;
    }
    let i = x.floor().rem_euclid(n as f64);
    if hits[i as usize] { 1.0 } else { 0.0 }
}

fn is_integer_value(value: f64) -> bool {
    value.is_finite() && value == value.trunc()
}

/// Pattern-text view of one 0/1 graph on an integer window.
///
/// Returns [`None`] when the window is not an integer span in
/// `1..=MAX_EUCLID_STEPS`, or when any step is not constantly 0 or 1.
#[must_use]
pub fn pattern_row(
    expr: &Expr,
    xmin: f64,
    xmax: f64,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
) -> Option<String> {
    if !is_integer_value(xmin) || !is_integer_value(xmax) {
        return None;
    }
    let span = xmax - xmin;
    if span < 1.0 || span > MAX_EUCLID_STEPS as f64 {
        return None;
    }
    let start = xmin as i64;
    let count = span as usize;
    let mut marks = String::with_capacity(count);
    for offset in 0..count {
        let x = (start + offset as i64) as f64;
        let at_start = eval_named(expr, x, a, sliders);
        let at_mid = eval_named(expr, x + 0.5, a, sliders);
        if at_start != at_mid {
            return None;
        }
        let mark = if at_start == 1.0 {
            PATTERN_HIT
        } else if at_start == 0.0 {
            PATTERN_REST
        } else {
            return None;
        };
        marks.push(mark);
    }
    Some(marks)
}

/// Numbered step-grid caption of pattern rows.
///
/// The header is the 1-based step index, ones digit only, so an eight-step
/// tresillo is `12345678` over `x..x..x.`. Empty when there are no rows, or
/// when the rows disagree about how many steps they have. This is the same
/// reading as [`StudioCreation::pattern_rows`], not a second document.
#[must_use]
pub fn pattern_grid_text(rows: &[String]) -> Option<String> {
    let width = rows.first()?.chars().count();
    if width == 0 || rows.iter().any(|row| row.chars().count() != width) {
        return None;
    }
    let header: String = (1..=width)
        .map(|step| char::from_digit((step % 10) as u32, 10).unwrap_or('0'))
        .collect();
    let mut text = header;
    for row in rows {
        text.push('\n');
        text.push_str(row);
    }
    Some(text)
}

/// Evaluate a parsed expression at variable `x` and parameter `a`.
///
/// Named sliders that are not in a table evaluate at one, the same default
/// `a` uses. Pass [`eval_named`] when the creation binds them.
#[must_use]
pub fn eval(expr: &Expr, x: f64, a: f64) -> f64 {
    eval_named(expr, x, a, &[])
}

/// Evaluate a parsed expression with named sliders bound.
#[must_use]
pub fn eval_named(expr: &Expr, x: f64, a: f64, sliders: &[crate::slider::StudioSlider]) -> f64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Var => x,
        Expr::Param => a,
        Expr::Slider(name) => crate::slider::slider_value(name, sliders),
        // A curve is parsed against a grammar that cannot produce these, so
        // they do not arise here. They are answered rather than ignored
        // because a point of the plane genuinely has no value on the line.
        Expr::VarIm | Expr::Point | Expr::ImagUnit => f64::NAN,
        Expr::Neg(inner) => -eval_named(inner, x, a, sliders),
        Expr::Bin(op, lhs, rhs) => {
            let (lhs, rhs) = (
                eval_named(lhs, x, a, sliders),
                eval_named(rhs, x, a, sliders),
            );
            match op {
                Op::Add => lhs + rhs,
                Op::Sub => lhs - rhs,
                Op::Mul => lhs * rhs,
                Op::Div => lhs / rhs,
                Op::Pow => lhs.powf(rhs),
            }
        }
        Expr::Call(func, arg) => {
            let arg = eval_named(arg, x, a, sliders);
            match func {
                Func::Sin => arg.sin(),
                Func::Cos => arg.cos(),
                Func::Tan => arg.tan(),
                Func::Exp => arg.exp(),
                Func::Ln => arg.ln(),
                Func::Abs => arg.abs(),
                Func::Sqrt => arg.sqrt(),
                Func::Floor => arg.floor(),
                Func::Re | Func::Conj => arg,
                Func::Im => 0.0,
                Func::Arg => {
                    if arg > 0.0 {
                        0.0
                    } else if arg < 0.0 {
                        PI
                    } else {
                        f64::NAN
                    }
                }
            }
        }
        Expr::PairCall(func, lhs, rhs) => {
            let (lhs, rhs) = (
                eval_named(lhs, x, a, sliders),
                eval_named(rhs, x, a, sliders),
            );
            if lhs.is_nan() || rhs.is_nan() {
                return f64::NAN;
            }
            match func {
                PairFunc::Mod => lhs.rem_euclid(rhs),
                PairFunc::Min => lhs.min(rhs),
                PairFunc::Max => lhs.max(rhs),
                PairFunc::Euclid => euclid_pulse(x, lhs, rhs),
            }
        }
        Expr::Pattern(hits) => pattern_pulse(x, hits),
    }
}

/// Evaluate a parsed field expression at the plane point `z` and parameter `a`.
///
/// The parameter stays real: it is one dial, and a dial that could leave the
/// line would need two. Functions with no meaning off the line, `floor`,
/// `mod`, `min`, `max`, and `euclid`, refuse a value with an imaginary part rather than
/// invent an ordering for the plane, and a refusal reaches the renderer as an
/// undefined sample. `ln`, `sqrt`, and a fractional power take their principal
/// branch, so a field built on one really does carry the seam that branch has.
#[must_use]
pub fn eval_field(expr: &Expr, z: Complex, a: f64) -> Complex {
    eval_field_named(expr, z, a, &[])
}

/// Evaluate a parsed field expression with named sliders bound.
#[must_use]
pub fn eval_field_named(
    expr: &Expr,
    z: Complex,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
) -> Complex {
    match expr {
        Expr::Num(n) => Complex::real(*n),
        Expr::Var => Complex::real(z.re),
        Expr::VarIm => Complex::real(z.im),
        Expr::Point => z,
        Expr::ImagUnit => Complex::I,
        Expr::Param => Complex::real(a),
        Expr::Slider(name) => Complex::real(crate::slider::slider_value(name, sliders)),
        Expr::Neg(inner) => -eval_field_named(inner, z, a, sliders),
        Expr::Bin(op, lhs, rhs) => {
            let (lhs, rhs) = (
                eval_field_named(lhs, z, a, sliders),
                eval_field_named(rhs, z, a, sliders),
            );
            match op {
                Op::Add => lhs + rhs,
                Op::Sub => lhs - rhs,
                Op::Mul => lhs * rhs,
                Op::Div => lhs / rhs,
                Op::Pow => lhs.powc(rhs),
            }
        }
        Expr::Call(func, arg) => {
            let arg = eval_field_named(arg, z, a, sliders);
            match func {
                Func::Sin => arg.sin(),
                Func::Cos => arg.cos(),
                Func::Tan => arg.tan(),
                Func::Exp => arg.exp(),
                Func::Ln => arg.ln(),
                Func::Abs => Complex::real(arg.abs()),
                Func::Sqrt => arg.sqrt(),
                Func::Floor => real_only(arg, f64::floor),
                Func::Re => Complex::real(arg.re),
                Func::Im => Complex::real(arg.im),
                Func::Arg => Complex::real(arg.arg()),
                Func::Conj => arg.conj(),
            }
        }
        Expr::PairCall(func, lhs, rhs) => {
            let (lhs, rhs) = (
                eval_field_named(lhs, z, a, sliders),
                eval_field_named(rhs, z, a, sliders),
            );
            if !lhs.is_real() || !rhs.is_real() || lhs.is_nan() || rhs.is_nan() {
                return Complex::UNDEFINED;
            }
            Complex::real(match func {
                PairFunc::Mod => lhs.re.rem_euclid(rhs.re),
                PairFunc::Min => lhs.re.min(rhs.re),
                PairFunc::Max => lhs.re.max(rhs.re),
                PairFunc::Euclid => euclid_pulse(z.re, lhs.re, rhs.re),
            })
        }
        Expr::Pattern(hits) => Complex::real(pattern_pulse(z.re, hits)),
    }
}

/// Apply a function that only the real line defines, refusing anything else.
fn real_only(value: Complex, apply: fn(f64) -> f64) -> Complex {
    if value.is_real() {
        return Complex::real(apply(value.re));
    }
    Complex::UNDEFINED
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
    to_melody_with_scale_named(expr, xmin, xmax, notes, a, &[], scale)
}

fn silent_spec() -> SoundSpec {
    SoundSpec {
        duration: 0.12,
        notes: Vec::new(),
    }
}

fn mix_overlay_melodies(
    expressions: &[Expr],
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
    scale: StudioScale,
) -> SoundSpec {
    let mut mixed = silent_spec();
    for expression in expressions {
        let voice = to_melody_with_scale_named(expression, xmin, xmax, notes, a, sliders, scale);
        mixed.duration = mixed.duration.max(voice.duration);
        mixed.notes.extend(voice.notes);
    }
    mixed
        .notes
        .sort_by(|left, right| left.start.total_cmp(&right.start));
    mixed
}

/// Height and phase along the real axis, using the plate's own numbers.
///
/// Time walks `x` through `[xmin, xmax]` at `y = 0`. Height is the doubling
/// ladder. Phase is the argument wheel. The zero reading is a proof about a
/// cell, not a sample, so it stays silent.
#[expect(
    clippy::too_many_arguments,
    reason = "a field voice is the formula, reading, window, count, knob, sliders, and scale"
)]
fn field_to_melody(
    expr: &Expr,
    reading: FieldReading,
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
    scale: StudioScale,
) -> SoundSpec {
    if reading == FieldReading::Zero {
        return silent_spec();
    }
    let notes = notes.clamp(1, MAX_MELODY_NOTES);
    let denom = (notes as f64 - 1.0).max(1.0);
    let samples: Vec<f32> = (0..notes)
        .filter_map(|i| {
            let x = xmin + (xmax - xmin) * i as f64 / denom;
            let value = eval_field_named(expr, Complex::real(x), a, sliders);
            let unit = match reading {
                FieldReading::Height => height_measure(value),
                FieldReading::Phase => phase_measure(value),
                FieldReading::Zero => None,
            }?;
            Some(unit as f32)
        })
        .collect();
    melody_from_norms(&samples, scale)
}

fn melody_from_norms(samples: &[f32], scale: StudioScale) -> SoundSpec {
    if samples.is_empty() {
        return silent_spec();
    }
    let step = 0.12_f32;
    let note_vec: Vec<Note> = samples
        .iter()
        .enumerate()
        .map(|(i, &norm)| {
            let semitones = quantized_semitones(norm.clamp(0.0, 1.0) * 24.0, scale);
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

/// Turn one expression into a melody with named sliders bound.
#[must_use]
pub fn to_melody_with_scale_named(
    expr: &Expr,
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
    scale: StudioScale,
) -> SoundSpec {
    let notes = notes.clamp(1, MAX_MELODY_NOTES);
    let step = 0.12_f32;
    let denom = (notes as f64 - 1.0).max(1.0);
    let samples: Vec<f64> = (0..notes)
        .map(|i| eval_named(expr, xmin + (xmax - xmin) * i as f64 / denom, a, sliders))
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
    sliders: &[crate::slider::StudioSlider],
    width: usize,
    height: usize,
) -> Result<StudioPlot, ProgramPlotError> {
    if width < 2 || height < 2 || input_max <= input_min {
        return Err(ProgramPlotError::Sampling(PlotTextError::InvalidGeometry));
    }
    match program {
        StudioProgram::Graph { expression, .. } => {
            plot_parsed_text_named(expression, input_min, input_max, a, sliders, width, height)
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
                    program.point_named(input, a, sliders)
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
        StudioProgram::Field { .. } => {
            Err(ProgramPlotError::Sampling(PlotTextError::InvalidGeometry))
        }
        StudioProgram::Program { expressions, .. } => {
            plot_overlay_text(expressions, input_min, input_max, a, sliders, width, height)
                .map_err(ProgramPlotError::Sampling)
        }
    }
}

fn plot_overlay_text(
    expressions: &[Expr],
    xmin: f64,
    xmax: f64,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
    width: usize,
    height: usize,
) -> Result<StudioPlot, PlotTextError> {
    if width < 2 || height < 2 || xmax <= xmin {
        return Err(PlotTextError::InvalidGeometry);
    }
    let curves: Vec<Vec<(f64, f64)>> = expressions
        .iter()
        .map(|expression| {
            (0..width)
                .map(|i| {
                    let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
                    (x, eval_named(expression, x, a, sliders))
                })
                .filter(|(_, y)| y.is_finite())
                .collect()
        })
        .collect();
    let finite: Vec<(f64, f64)> = curves.iter().flatten().copied().collect();
    if finite.is_empty() {
        return Err(PlotTextError::Undefined);
    }
    let ymin = finite
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let ymax = finite
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let yspan = (ymax - ymin).max(1e-9);
    let mut canvas = crate::canvas::Canvas::new(width, height);
    for (index, samples) in curves.iter().enumerate() {
        let mark = PROGRAM_MARKS[index.min(PROGRAM_MARKS.len() - 1)];
        let mut previous: Option<(i32, i32)> = None;
        for &(x, y) in samples {
            let sx = ((x - xmin) / (xmax - xmin) * (width as f64 - 1.0)) as i32;
            let sy = ((height as f64 - 1.0) - (y - ymin) / yspan * (height as f64 - 1.0)) as i32;
            if let Some((px, py)) = previous {
                use crate::surface::Surface;
                canvas.line(px, py, sx, sy, mark);
            }
            previous = Some((sx, sy));
        }
    }
    Ok(StudioPlot {
        text: canvas.to_text(),
        xmin,
        xmax,
        ymin,
        ymax,
    })
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
    plot_parsed_text_named(expr, xmin, xmax, a, &[], width, height)
}

pub(crate) fn plot_parsed_text_named(
    expr: &Expr,
    xmin: f64,
    xmax: f64,
    a: f64,
    sliders: &[crate::slider::StudioSlider],
    width: usize,
    height: usize,
) -> Result<(String, f64, f64), PlotTextError> {
    if width < 2 || height < 2 || xmax <= xmin {
        return Err(PlotTextError::InvalidGeometry);
    }
    let samples: Vec<(f64, f64)> = (0..width)
        .map(|i| {
            let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
            (x, eval_named(expr, x, a, sliders))
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
    parse_in(source, Grammar::Curve)
}

/// Parse a field expression over the complex plane.
///
/// The field grammar is the curve grammar plus the vocabulary a point of the
/// plane needs: `z` for the point itself, `y` for its imaginary coordinate
/// beside the `x` a curve already spells, `i` for the imaginary unit, and
/// `re`, `im`, `arg`, and `conj` for reading a value back apart. The curve
/// grammar is left exactly as it was, so every saved curve parses to the same
/// expression it always did.
///
/// # Errors
/// Returns the same bounded parser diagnostics as [`parse`].
pub fn parse_field(source: &str) -> Result<Expr, String> {
    parse_in(source, Grammar::Field)
}

/// Which vocabulary an expression is read against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grammar {
    /// One real variable and the parameter. What every saved curve uses.
    Curve,
    /// A point of the complex plane and the functions that read it.
    Field,
}

/// Whole-source tracker marks: `x..x..x.`.
///
/// A lone `x` stays the variable. A run of letters with no rest stays a
/// slider name. The form must include a `.` so those two cannot be stolen.
fn tracker_pattern_from_source(source: &str) -> Result<Option<Vec<bool>>, String> {
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 2 || !compact.contains('.') {
        return Ok(None);
    }
    if !compact
        .chars()
        .all(|mark| mark == PATTERN_HIT || mark == PATTERN_REST)
    {
        return Ok(None);
    }
    if compact.len() > MAX_EUCLID_STEPS {
        return Err(format!(
            "a tracker row may hold at most {MAX_EUCLID_STEPS} steps"
        ));
    }
    Ok(Some(
        compact.chars().map(|mark| mark == PATTERN_HIT).collect(),
    ))
}

fn parse_in(source: &str, grammar: Grammar) -> Result<Expr, String> {
    let tokenized = tokenize(source)?;
    let tokens = tokenized.tokens;
    if tokens.len() > MAX_EXPR_TOKENS {
        return Err(format!(
            "expression is too complex; limit is {MAX_EXPR_TOKENS} tokens"
        ));
    }
    if let Some(hits) = tracker_pattern_from_source(source)? {
        return Ok(Expr::Pattern(hits));
    }
    let mut parser = Parser {
        tokens,
        columns: tokenized.columns,
        end_column: tokenized.end_column,
        pos: 0,
        grammar,
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
    /// A tracker rest, and the decimal point when it is not part of a number.
    Dot,
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
            Self::Dot => "'.'".to_string(),
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
        } else if c.is_ascii_digit()
            || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
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
        } else if c == '.' {
            tokens.push(Tok::Dot);
            columns.push(i + 1);
            i += 1;
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
    grammar: Grammar,
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

    /// `pat(x..x..x.)`: an explicit step pattern.
    fn pattern_call(&mut self, name_column: usize) -> Result<Expr, String> {
        self.pos += 1;
        let mut hits = Vec::new();
        loop {
            match self.peek() {
                Some(Tok::Ident(name))
                    if !name.is_empty() && name.chars().all(|mark| mark == PATTERN_HIT) =>
                {
                    hits.extend(std::iter::repeat_n(true, name.len()));
                    self.pos += 1;
                }
                Some(Tok::Dot) => {
                    hits.push(false);
                    self.pos += 1;
                }
                Some(Tok::RParen) => break,
                Some(other) => {
                    return Err(format!(
                        "pat() expected 'x' or '.' at column {}; found {}",
                        self.current_column(),
                        other.diagnostic_name()
                    ));
                }
                None => {
                    return Err(format!(
                        "expression ended at column {}; expected ')' after pat( ",
                        self.current_column()
                    ));
                }
            }
            if hits.len() > MAX_EUCLID_STEPS {
                return Err(format!(
                    "pat() may hold at most {MAX_EUCLID_STEPS} steps at column {name_column}"
                ));
            }
        }
        self.expect_right_paren("after pat( ")?;
        if hits.is_empty() {
            return Err(format!(
                "pat() needs at least one mark at column {name_column}"
            ));
        }
        Ok(Expr::Pattern(hits))
    }

    /// Resolve an identifier: the variable, a constant, or a function call.
    fn ident(&mut self, name: &str, depth: usize, name_column: usize) -> Result<Expr, String> {
        if name == "pat" && matches!(self.peek(), Some(Tok::LParen)) {
            return self.pattern_call(name_column);
        }
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
                "re" if self.grammar == Grammar::Field => Some(Func::Re),
                "im" if self.grammar == Grammar::Field => Some(Func::Im),
                "arg" if self.grammar == Grammar::Field => Some(Func::Arg),
                "conj" if self.grammar == Grammar::Field => Some(Func::Conj),
                _ => None,
            };
            let pair = match name {
                "mod" => Some(PairFunc::Mod),
                "min" => Some(PairFunc::Min),
                "max" => Some(PairFunc::Max),
                "euclid" => Some(PairFunc::Euclid),
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
                "y" if self.grammar == Grammar::Field => Ok(Expr::VarIm),
                "z" if self.grammar == Grammar::Field => Ok(Expr::Point),
                "i" if self.grammar == Grammar::Field => Ok(Expr::ImagUnit),
                other if crate::slider::is_slider_name(other) => {
                    Ok(Expr::Slider(other.to_string()))
                }
                other => Err(format!("unknown name '{other}' at column {name_column}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Expr, FieldReading, MAX_CREDIT_CHARS, MAX_EUCLID_STEPS, MAX_EXPR_TOKENS, MAX_MELODY_NOTES,
        MAX_META_TEXT_CHARS, MAX_PARSE_DEPTH, MAX_STUDIO_SOURCE_CHARS, STUDIO_EXPERIMENTS,
        STUDIO_RECIPES, StudioCreation, StudioKind, StudioProgram, StudioScale,
        adjacent_construction_creation, adjacent_studio_experiment, eval, eval_named,
        first_studio_construction, is_returning_home_transfer, parse, returning_home_transfer,
        studio_auto_recipe, studio_construction_family, studio_experiment,
        studio_experiment_matching, studio_experiment_meta, studio_experiments_in, studio_recipe,
        studio_recipe_count, to_melody, to_melody_with_scale, to_melody_with_scale_named,
        uses_field_vocabulary,
    };
    use super::{eval_field, parse_field};
    use crate::complex::Complex;

    #[test]
    fn the_curve_grammar_still_refuses_every_field_name() {
        for source in [
            "y", "z", "i", "re(x)", "im(x)", "arg(x)", "conj(x)", "z^2 - 1",
        ] {
            let error = parse(source).expect_err("curve grammar rejects a field name");
            assert!(
                error.contains("unknown"),
                "{source} gave the wrong diagnostic: {error}"
            );
        }
    }

    #[test]
    fn every_saved_curve_still_parses_to_the_expression_it_always_did() {
        for source in STUDIO_RECIPES {
            let curve = parse(source).expect("recipe parses as a curve");
            let field = parse_field(source).expect("recipe parses as a field too");
            assert_eq!(curve, field, "{source} changed meaning between grammars");
        }
    }

    #[test]
    fn a_field_reads_the_point_its_coordinates_and_the_imaginary_unit() {
        let z = Complex::new(3.0, -4.0);
        assert_eq!(eval_field(&parse_field("z").unwrap(), z, 0.0), z);
        assert_eq!(
            eval_field(&parse_field("x").unwrap(), z, 0.0),
            Complex::real(3.0)
        );
        assert_eq!(
            eval_field(&parse_field("y").unwrap(), z, 0.0),
            Complex::real(-4.0)
        );
        assert_eq!(eval_field(&parse_field("i").unwrap(), z, 0.0), Complex::I);
        assert_eq!(
            eval_field(&parse_field("a").unwrap(), z, 2.5),
            Complex::real(2.5)
        );
        assert_eq!(
            eval_field(&parse_field("x + i*y").unwrap(), z, 0.0),
            z,
            "the coordinates rebuild the point"
        );
    }

    #[test]
    fn a_field_lands_exactly_on_its_zeros() {
        let expr = parse_field("z^2 - 1").expect("parses");
        assert_eq!(eval_field(&expr, Complex::ONE, 0.0), Complex::ZERO);
        assert_eq!(eval_field(&expr, Complex::real(-1.0), 0.0), Complex::ZERO);
        assert_eq!(
            eval_field(&expr, Complex::ZERO, 0.0),
            Complex::real(-1.0),
            "the origin is not a zero of this one"
        );
    }

    #[test]
    fn a_pole_is_reported_as_a_pole_rather_than_as_an_undefined_sample() {
        let expr = parse_field("(z^2 - 1)/(z^2 + 1)").expect("parses");
        let at_pole = eval_field(&expr, Complex::I, 0.0);
        assert!(!at_pole.is_finite());
        assert!(!at_pole.is_nan(), "a pole has a size, not no answer");
        assert_eq!(eval_field(&expr, Complex::ONE, 0.0), Complex::ZERO);
    }

    #[test]
    fn reading_a_value_apart_and_back_together_returns_it() {
        let z = Complex::new(0.75, -1.5);
        let expr = parse_field("re(z) + i*im(z)").expect("parses");
        assert_eq!(eval_field(&expr, z, 0.0), z);
        assert_eq!(
            eval_field(&parse_field("conj(z)").unwrap(), z, 0.0),
            z.conj()
        );
        let angle = eval_field(&parse_field("arg(z)").unwrap(), z, 0.0);
        assert!((angle.re - z.arg()).abs() < 1e-15);
        assert!(angle.is_real());
    }

    #[test]
    fn the_functions_the_line_owns_refuse_a_point_off_it() {
        let off_axis = Complex::new(1.5, 0.5);
        for source in [
            "floor(z)",
            "mod(z, 2)",
            "min(z, 1)",
            "max(z, 1)",
            "euclid(z, 8)",
        ] {
            let expr = parse_field(source).expect("parses");
            assert!(
                eval_field(&expr, off_axis, 0.0).is_nan(),
                "{source} answered for a point off the real line"
            );
            assert!(
                !eval_field(&expr, Complex::real(1.5), 0.0).is_nan(),
                "{source} refused a point on the real line"
            );
        }
    }

    #[test]
    fn a_real_valued_field_stays_on_the_axis_so_its_zero_set_is_a_boundary() {
        let expr = parse_field("x^2 + y^2 - 1").expect("parses");
        for point in [
            Complex::new(0.0, 0.0),
            Complex::new(2.0, 0.0),
            Complex::new(0.3, -0.9),
        ] {
            assert!(eval_field(&expr, point, 0.0).is_real());
        }
        assert_eq!(
            eval_field(&expr, Complex::new(0.0, 0.0), 0.0),
            Complex::real(-1.0)
        );
        assert_eq!(eval_field(&expr, Complex::ONE, 0.0), Complex::ZERO);
        assert!(eval_field(&expr, Complex::new(2.0, 0.0), 0.0).re > 0.0);
    }

    #[test]
    fn a_field_expression_is_held_to_the_same_bounds_as_a_curve() {
        let deep = "sin(".repeat(MAX_PARSE_DEPTH + 2) + "z" + &")".repeat(MAX_PARSE_DEPTH + 2);
        assert!(parse_field(&deep).is_err());
        let wide = vec!["z"; MAX_EXPR_TOKENS + 1].join("+");
        assert!(parse_field(&wide).is_err());
    }

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

    #[test]
    fn bundled_studio_experiments_parse_keep_lineage_and_open_by_id() {
        assert_eq!(STUDIO_EXPERIMENTS.len(), 18);
        let full = studio_experiment("full-return").expect("full-return");
        assert_eq!(full.title(), Some("A full return"));
        assert_eq!(full.kind(), StudioKind::Parametric);
        assert_eq!(full.source(), "cos(2*pi*t)");
        assert_eq!(full.second_source(), Some("sin(2*pi*(17/12)*t)"));
        assert_eq!((full.xmin(), full.xmax(), full.a()), (0.0, 12.0, 1.0));
        assert_eq!(
            studio_experiment("experiment:full-return").expect("prefixed"),
            full
        );
        assert_eq!(
            StudioCreation::from_capsule("full-return").expect("capsule id"),
            full
        );

        let almost = studio_experiment("almost-home").expect("almost-home");
        assert_eq!(almost.title(), Some("Almost home"));
        let parent = full.to_link();
        assert_eq!(almost.descends(), Some(parent.as_str()));
        assert!(
            studio_experiment_meta("almost-home")
                .expect("meta")
                .invitation
                .contains("Compare its formula")
        );

        let same = studio_experiment("same-place").expect("same-place");
        assert_eq!(same.title(), Some("Same place, another direction"));
        assert!(same.descends().is_none());

        let transfer = studio_experiment("another-ratio").expect("another-ratio");
        assert_eq!(transfer.title(), Some("Another ratio"));
        assert_eq!(transfer.source(), "cos(2*pi*t)");
        assert_eq!(transfer.second_source(), Some("sin(2*pi*t)"));
        assert!(!transfer.source().contains("8/5"));
        assert!(!transfer.second_source().unwrap().contains("8/5"));
        assert!(transfer.descends().is_none());
        assert!(
            studio_experiment_meta("another-ratio")
                .expect("meta")
                .invitation
                .contains("Change one frequency")
        );
        assert!(
            !studio_experiment_meta("another-ratio")
                .expect("meta")
                .invitation
                .contains("8/5")
        );

        let ellipse = studio_experiment("circle-to-ellipse").expect("circle-to-ellipse");
        assert_eq!(ellipse.title(), Some("Circle to ellipse"));
        assert_eq!(ellipse.source(), "a*cos(t)");
        assert_eq!(ellipse.second_source(), Some("sin(t)"));
        let uniform = studio_experiment("uniform-circle").expect("uniform-circle");
        assert_eq!(uniform.title(), Some("Uniform circle"));
        assert_eq!(uniform.second_source(), Some("a*sin(t)"));
        assert_eq!(
            studio_experiment_meta("circle-to-ellipse")
                .expect("meta")
                .family,
            "shape-and-scale"
        );

        let home = studio_experiments_in(Some("returning-home")).expect("home family");
        assert_eq!(home.len(), 4);
        assert!(
            home.iter()
                .all(|experiment| experiment.family == "returning-home")
        );
        let shapes = studio_experiments_in(Some("shape-and-scale")).expect("shape family");
        assert_eq!(shapes.len(), 2);
        let readings = studio_experiments_in(Some("three-readings")).expect("readings family");
        assert_eq!(readings.len(), 4);
        let knobs = studio_experiments_in(Some("named-sliders")).expect("slider family");
        assert_eq!(knobs.len(), 2);
        let overlay = studio_experiments_in(Some("overlay")).expect("overlay family");
        assert_eq!(overlay.len(), 2);
        let euclidean = studio_experiments_in(Some("euclidean")).expect("euclidean family");
        assert_eq!(euclidean.len(), 2);
        let tresillo = studio_experiment("tresillo").expect("tresillo");
        assert_eq!(tresillo.source(), "euclid(3,8)");
        assert_eq!(tresillo.xmin(), 0.0);
        assert_eq!(tresillo.xmax(), 8.0);
        assert_eq!(tresillo.title(), Some("Tresillo"));
        let against = studio_experiment("three-against-five").expect("three-against-five");
        assert_eq!(against.kind(), StudioKind::Program);
        assert_eq!(against.editor_source(), "euclid(3,8) & euclid(5,8)");
        let voices = studio_experiments_in(Some("two-voices")).expect("voices family");
        assert_eq!(voices.len(), 2);
        let closing = studio_experiment("closing-voices").expect("closing-voices");
        assert_eq!(closing.kind(), StudioKind::Program);
        assert_eq!(closing.editor_source(), "cos(2*pi*x) & sin(2*pi*(17/12)*x)");
        assert_eq!(closing.xmin(), 0.0);
        assert_eq!(closing.xmax(), 12.0);
        let wandering = studio_experiment("wandering-voices").expect("wandering-voices");
        assert_eq!(wandering.kind(), StudioKind::Program);
        assert!(wandering.editor_source().contains("sqrt(2)"));
        let child = against.fork(None, Some("Remix"), None).expect("fork");
        assert_eq!(child.kind(), StudioKind::Program);
        assert_eq!(child.editor_source(), "euclid(3,8) & euclid(5,8)");
        let parts = studio_experiment("the-parts").expect("the-parts");
        assert_eq!(parts.kind(), StudioKind::Program);
        assert_eq!(parts.editor_source(), "sin(x) & cos(x)");
        assert!(parts.to_num_file().starts_with("NUMINOUS_STUDIO 7\n"));
        let sum = studio_experiment("the-sum").expect("the-sum");
        assert_eq!(sum.extra_sources().len(), 2);
        assert_eq!(
            studio_experiment_matching(&parts).map(|experiment| experiment.id),
            Some("the-parts")
        );
        assert_eq!(
            studio_experiment_matching(&sum).map(|experiment| experiment.id),
            Some("the-sum")
        );
        let plot = sum.plot_text(48, 16).expect("sum plot");
        assert!(plot.text.contains('#'));
        assert!(plot.text.contains('*'));
        assert!(plot.text.contains('+'));
        let extra = studio_experiment("extra-knob").expect("extra-knob");
        assert_eq!(extra.title(), Some("An extra knob"));
        assert_eq!(extra.source(), "sin(a*x) + b");
        assert_eq!(extra.sliders().len(), 1);
        assert_eq!(extra.sliders()[0].name(), "b");
        assert!(extra.to_num_file().starts_with("NUMINOUS_STUDIO 6\n"));
        let ratio = studio_experiment("live-ratio").expect("live-ratio");
        assert_eq!(ratio.title(), Some("A live ratio"));
        assert_eq!(ratio.source(), "cos(2*pi*p*t)");
        assert_eq!(ratio.second_source(), Some("sin(2*pi*q*t)"));
        assert_eq!(ratio.sliders()[0].name(), "p");
        assert_eq!(ratio.sliders()[0].value(), 3.0);
        assert_eq!(ratio.sliders()[1].name(), "q");
        assert_eq!(ratio.sliders()[1].value(), 2.0);
        match crate::path_closure::PathClosure::of(&ratio) {
            crate::path_closure::PathClosure::Periodic(periodic) => {
                assert_eq!(periodic.period_text, "1");
            }
            other => panic!("live-ratio should close: {other:?}"),
        }
        assert_eq!(
            studio_experiment("the-circle").expect("circle").reading(),
            Some(FieldReading::Zero)
        );
        assert_eq!(
            studio_experiment("the-bowl").expect("bowl").reading(),
            Some(FieldReading::Height)
        );
        assert!(studio_experiments_in(Some("no-such-family")).is_err());

        assert!(studio_experiment("missing").is_none());
        assert!(StudioCreation::from_capsule("docs/experiments/full-return.num").is_err());

        assert_eq!(
            studio_construction_family("lissajous"),
            Some("returning-home")
        );
        assert_eq!(studio_construction_family("times-tables"), None);
        assert_eq!(
            first_studio_construction("lissajous").map(|experiment| experiment.id),
            Some("full-return")
        );
        assert_eq!(
            studio_experiment_matching(&full).map(|experiment| experiment.id),
            Some("full-return")
        );
        assert_eq!(
            adjacent_studio_experiment("full-return", 1).map(|experiment| experiment.id),
            Some("almost-home")
        );
        assert_eq!(
            adjacent_studio_experiment("almost-home", 1).map(|experiment| experiment.id),
            Some("same-place")
        );
        assert_eq!(
            adjacent_studio_experiment("same-place", 1).map(|experiment| experiment.id),
            Some("another-ratio")
        );
        assert_eq!(
            adjacent_studio_experiment("another-ratio", 1).map(|experiment| experiment.id),
            Some("closing-voices")
        );
        assert_eq!(
            adjacent_studio_experiment("closing-voices", -1).map(|experiment| experiment.id),
            Some("another-ratio")
        );
        assert!(adjacent_studio_experiment("wandering-voices", 1).is_none());
        assert!(adjacent_studio_experiment("full-return", -1).is_none());
        assert_eq!(
            adjacent_studio_experiment("same-place", -1).map(|experiment| experiment.id),
            Some("almost-home")
        );
        assert_eq!(
            adjacent_studio_experiment("another-ratio", -1).map(|experiment| experiment.id),
            Some("same-place")
        );
        assert!(adjacent_studio_experiment("circle-to-ellipse", 1).is_some());
        assert!(adjacent_studio_experiment("uniform-circle", 1).is_none());

        let transfer = returning_home_transfer();
        assert_eq!(transfer, studio_experiment("another-ratio").expect("id"));
        assert!(is_returning_home_transfer(&transfer));
        let from_same = adjacent_construction_creation(&same, 1).expect("transfer");
        assert!(is_returning_home_transfer(&from_same));
        let back = adjacent_construction_creation(&from_same, -1).expect("same-place");
        assert_eq!(back.title(), Some("Same place, another direction"));
        let voices = adjacent_construction_creation(&from_same, 1).expect("closing-voices");
        assert_eq!(voices.title(), Some("Closing voices"));
        assert_eq!(
            studio_experiment_matching(&voices).map(|experiment| experiment.id),
            Some("closing-voices")
        );
        let wandering = adjacent_construction_creation(&voices, 1).expect("wandering-voices");
        assert_eq!(wandering.title(), Some("Wandering voices"));
        assert_eq!(
            studio_experiment_matching(&wandering).map(|experiment| experiment.id),
            Some("wandering-voices")
        );
        assert!(adjacent_construction_creation(&wandering, 1).is_none());
        assert_eq!(
            crate::PathClosure::of(&transfer).status_caption(),
            Some("PERIOD 1".to_string())
        );
        let edited =
            StudioCreation::new_parametric("cos(2*pi*t)", "sin(2*pi*(8/5)*t)", 0.0, 5.0, 1.0)
                .expect("player-typed ratio");
        assert!(!is_returning_home_transfer(&edited));
        assert!(adjacent_construction_creation(&edited, 1).is_none());
        assert!(adjacent_construction_creation(&edited, -1).is_none());
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
    fn euclidean_rhythms_place_onsets_evenly() {
        // Residue test (i * k) rem n < k, which is the Cuban tresillo on 8.
        let tresillo = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        for (step, expected) in tresillo.iter().enumerate() {
            assert!(
                (at("euclid(3, 8)", step as f64) - expected).abs() < 1e-12,
                "step {step}"
            );
            assert!(
                (at("euclid(3, 8)", step as f64 + 0.75) - expected).abs() < 1e-12,
                "interior of step {step}"
            );
        }
        let five = [1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
        for (step, expected) in five.iter().enumerate() {
            assert!(
                (at("euclid(5, 8)", step as f64) - expected).abs() < 1e-12,
                "five step {step}"
            );
        }
        assert!((at("euclid(3, 8)", 8.0) - 1.0).abs() < 1e-12);
        assert!((at("euclid(3, 8)", -1.0) - 0.0).abs() < 1e-12);
        assert!((at("euclid(0, 8)", 3.0) - 0.0).abs() < 1e-12);
        assert!((at("euclid(8, 8)", 3.0) - 1.0).abs() < 1e-12);
        assert!((at("euclid(10, 8)", 1.0) - 1.0).abs() < 1e-12);
        assert!(at("euclid(3, 0)", 1.0).is_nan());
        assert!(at("euclid(3, 65)", 1.0).is_nan());
        assert!(at("euclid(3, -8)", 1.0).is_nan());
        let k = parse("euclid(k, 8)").expect("slider hits");
        let slider = crate::slider::StudioSlider::new("k", 3.0, 0.0, 8.0).expect("k");
        let bound = [slider];
        assert!((eval_named(&k, 0.0, 1.0, &bound) - 1.0).abs() < 1e-12);
        assert!((eval_named(&k, 1.0, 1.0, &bound) - 0.0).abs() < 1e-12);
        let plot = super::plot_text("euclid(3,8)", 0.0, 8.0, 1.0, 48, 8).expect("plot");
        assert!(plot.0.contains('#'), "{}", plot.0);
    }

    #[test]
    fn euclidean_graphs_have_a_pattern_text_view() {
        let tresillo = studio_experiment("tresillo").expect("tresillo");
        assert_eq!(tresillo.pattern_rows(), ["x..x..x."]);
        let five = StudioCreation::new("euclid(5,8)", 0.0, 8.0, 1.0).expect("five");
        assert_eq!(five.pattern_rows(), ["x.x.xx.x"]);
        let layered = studio_experiment("three-against-five").expect("layered");
        assert_eq!(layered.pattern_rows(), ["x..x..x.", "x.x.xx.x"]);
        let curve = StudioCreation::new("sin(x)", 0.0, 8.0, 1.0).expect("curve");
        assert!(curve.pattern_rows().is_empty());
        let mixed =
            StudioCreation::new_program(["sin(x)", "euclid(3,8)"], 0.0, 8.0, 1.0).expect("mixed");
        assert!(
            mixed.pattern_rows().is_empty(),
            "a mixed overlay is not a tracker row"
        );
        let half = StudioCreation::new("euclid(3,8)", 0.0, 8.5, 1.0).expect("half");
        assert!(half.pattern_rows().is_empty());
        let path = studio_experiment("full-return").expect("path");
        assert!(path.pattern_rows().is_empty());
        assert_eq!(
            super::pattern_grid_text(&tresillo.pattern_rows()).expect("grid"),
            "12345678\nx..x..x."
        );
        assert_eq!(
            super::pattern_grid_text(&layered.pattern_rows()).expect("layered grid"),
            "12345678\nx..x..x.\nx.x.xx.x"
        );
        assert!(super::pattern_grid_text(&[]).is_none());
        assert!(super::pattern_grid_text(&["x.".into(), "x".into()]).is_none());
        let ten = StudioCreation::new("euclid(1,10)", 0.0, 10.0, 1.0).expect("ten");
        assert_eq!(
            super::pattern_grid_text(&ten.pattern_rows()).expect("ten grid"),
            "1234567890\nx........."
        );
    }

    #[test]
    fn tracker_marks_are_an_explicit_pattern() {
        let marks = parse("x..x..x.").expect("bare");
        let named = parse("pat(x..x..x.)").expect("pat");
        assert_eq!(marks, named);
        assert_eq!(eval(&marks, 0.0, 1.0), 1.0);
        assert_eq!(eval(&marks, 1.0, 1.0), 0.0);
        assert_eq!(eval(&marks, 3.0, 1.0), 1.0);
        assert_eq!(eval(&marks, 8.0, 1.0), 1.0);
        let creation = StudioCreation::new("x..x..x.", 0.0, 8.0, 1.0).expect("row");
        assert_eq!(creation.pattern_rows(), ["x..x..x."]);
        assert_eq!(
            StudioProgram::from_editor("x..x..x.")
                .expect("editor")
                .kind(),
            StudioKind::Graph
        );
        let layered = StudioProgram::from_editor("x..x..x. & x.x.xx.x").expect("overlay");
        assert_eq!(layered.kind(), StudioKind::Program);
        let program =
            StudioCreation::new_program(["x..x..x.", "x.x.xx.x"], 0.0, 8.0, 1.0).expect("two");
        assert_eq!(program.pattern_rows(), ["x..x..x.", "x.x.xx.x"]);
        assert_eq!(
            StudioCreation::from_num_file(&program.to_num_file()).expect("overlay file"),
            program
        );
        assert!(parse("pat()").is_err());
        assert!(parse("pat").is_err());
        assert!(parse("pat(z..)").is_err());
        assert!(matches!(parse("x").expect("variable"), Expr::Var));
        assert!(matches!(parse("xx").expect("slider"), Expr::Slider(_)));
        assert_eq!(eval(&parse(".x").expect("rest then hit"), 0.0, 1.0), 0.0);
        assert_eq!(eval(&parse(".x").expect("rest then hit"), 1.0, 1.0), 1.0);
        assert!((at(".5 + x", 1.0) - 1.5).abs() < 1e-12);
        assert_eq!(at("pat(x..x..x.) * 2", 0.0), 2.0);
        assert_eq!(at("pat(x..x..x.) * 2", 1.0), 0.0);
        let five = parse("pat(x.x.xx.x)").expect("consecutive hits");
        for step in 0..8 {
            assert_eq!(
                eval(&five, step as f64, 1.0),
                at("euclid(5,8)", step as f64),
                "step {step}"
            );
        }
        for step in 0..8 {
            assert_eq!(
                at("pat(x..x..x.)", step as f64),
                at("euclid(3,8)", step as f64),
                "tresillo step {step}"
            );
        }
        let handmade = StudioCreation::new("pat(x..x..x.)", 0.0, 8.0, 1.0).expect("named");
        assert_eq!(handmade.pattern_rows(), ["x..x..x."]);
        assert_eq!(
            StudioCreation::from_num_file(&handmade.to_num_file()).expect("file"),
            handmade
        );
        assert_eq!(
            StudioCreation::from_link(&handmade.to_link()).expect("link"),
            handmade
        );
        let too_long = format!("pat({})", "x".repeat(MAX_EUCLID_STEPS + 1));
        assert!(
            parse(&too_long)
                .expect_err("overlong pat")
                .contains("at most")
        );
        let too_long_row = format!("{}x.", "x".repeat(MAX_EUCLID_STEPS));
        assert!(
            parse(&too_long_row)
                .expect_err("overlong row")
                .contains("at most")
        );
    }

    #[test]
    fn pair_functions_do_not_hide_undefined_arguments() {
        assert!(at("mod(1, 0)", 0.0).is_nan());
        assert!(at("min(sqrt(-1), 2)", 0.0).is_nan());
        assert!(at("max(2, sqrt(-1))", 0.0).is_nan());
        assert!(at("euclid(sqrt(-1), 8)", 0.0).is_nan());
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
        assert!(parse("Wut").is_err());
        assert!(parse("log").is_err());
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
        let future = "NUMINOUS_STUDIO 8\nexpr=x\nxmin=-1\nxmax=1\na=0\n";
        let err = StudioCreation::from_num_file(future).expect_err("future refused");
        assert!(err.contains("newer Numinous"), "{err}");
        let field_on_four = "NUMINOUS_STUDIO 4\nkind=field\nexpr=z\nxmin=-2\nxmax=2\nymin=-2\nymax=2\nreading=phase\na=1\n";
        let err = StudioCreation::from_num_file(field_on_four).expect_err("field needs v5");
        assert!(err.contains("NUMINOUS_STUDIO 5"), "{err}");
        let sliders_on_one = "NUMINOUS_STUDIO 1\nexpr=sin(b*x)\nxmin=-1\nxmax=1\na=1\n";
        let err = StudioCreation::from_num_file(sliders_on_one).expect_err("sliders need v6");
        assert!(err.contains("NUMINOUS_STUDIO 6"), "{err}");
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
    fn field_capsules_round_trip_as_version_five() {
        let creation = StudioCreation::new_field(
            "x^2 + y^2 - 1",
            -2.0,
            2.0,
            -2.0,
            2.0,
            1.0,
            FieldReading::Zero,
        )
        .expect("field")
        .with_title("The circle")
        .expect("title");
        assert_eq!(creation.kind(), StudioKind::Field);
        let text = creation.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 5\n"), "{text}");
        assert!(text.contains("kind=field\n"), "{text}");
        assert!(text.contains("reading=zero\n"), "{text}");
        assert!(!text.contains("scale="), "{text}");
        assert_eq!(
            StudioCreation::from_num_file(&text).expect("file"),
            creation
        );
        let link = creation.to_link();
        assert!(link.contains("kind=field"), "{link}");
        assert!(link.contains("reading=zero"), "{link}");
        assert_eq!(StudioCreation::from_link(&link).expect("link"), creation);

        let plot = creation.plot_text(72, 28).expect("plot");
        assert!(plot.text.contains('#'));
        assert!(!plot.text.contains('?'));

        let child = creation
            .fork_field(None, Some(FieldReading::Height), Some("The bowl"), None)
            .expect("fork");
        assert_eq!(child.reading(), Some(FieldReading::Height));
        assert_eq!(child.source(), creation.source());
        assert!(child.descends().unwrap().contains("kind=field"));
        assert!(
            !child.to_melody(8).notes.is_empty(),
            "height along the real axis is a sample, so it sings"
        );
        assert!(creation.to_melody(8).notes.is_empty(), "zero is a proof");

        assert!(
            StudioCreation::new_field("z^2 - 1", -2.0, 2.0, -2.0, 2.0, 1.0, FieldReading::Zero)
                .is_err()
        );
        assert_eq!(
            StudioProgram::from_editor("x^2 + y^2 - 1")
                .expect("editor")
                .kind(),
            StudioKind::Field
        );
        assert_eq!(
            StudioProgram::from_editor("sin(x)").expect("graph").kind(),
            StudioKind::Graph
        );
        assert_eq!(
            StudioProgram::from_editor("z").expect("identity").kind(),
            StudioKind::Field
        );
        assert!(uses_field_vocabulary(&parse_field("re(z)").expect("re")));
        assert!(!uses_field_vocabulary(&parse("sin(x)").expect("sin")));
    }

    #[test]
    fn height_and_phase_fields_sing_the_real_axis_and_zero_stays_silent() {
        let height = studio_experiment("the-bowl").expect("the-bowl");
        assert_eq!(height.reading(), Some(FieldReading::Height));
        let sung = height.to_melody(8);
        assert_eq!(sung.notes.len(), 8);
        assert_eq!(sung, height.to_midi_melody(8), "a field is one voice");

        let phase = studio_experiment("simple-zero").expect("simple-zero");
        assert_eq!(phase.reading(), Some(FieldReading::Phase));
        let wheel = phase.to_melody(16);
        assert_eq!(wheel.notes.len(), 16);
        let low = wheel
            .notes
            .iter()
            .map(|note| note.freq)
            .fold(f32::INFINITY, f32::min);
        let high = wheel
            .notes
            .iter()
            .map(|note| note.freq)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            high > low * 1.5,
            "z on the real axis points left, then right: {low} then {high}"
        );

        let pole = studio_experiment("a-pole").expect("a-pole");
        let pole_voice = pole.to_melody(16);
        assert!(pole_voice.notes.len() > 1);
        let first = pole_voice.notes[0].freq;
        assert!(
            pole_voice
                .notes
                .iter()
                .any(|note| (note.freq - first).abs() > 1.0),
            "1/z on the real axis changes direction at the origin"
        );

        let zero = studio_experiment("the-circle").expect("the-circle");
        assert_eq!(zero.reading(), Some(FieldReading::Zero));
        assert!(zero.to_melody(8).notes.is_empty());
        assert!(zero.to_midi_melody(8).notes.is_empty());

        let identity = StudioProgram::field("z").expect("z");
        let identity_height = identity
            .clone()
            .with_reading(FieldReading::Height)
            .to_melody(-2.0, 2.0, 5, 1.0, &[], StudioScale::Continuous);
        assert_eq!(identity_height.notes.len(), 5);
        assert!(
            identity_height.notes[2].freq < identity_height.notes[0].freq,
            "height of z along the real axis is a V: the origin is the bottom"
        );
        let identity_zero = identity.with_reading(FieldReading::Zero).to_melody(
            -2.0,
            2.0,
            5,
            1.0,
            &[],
            StudioScale::Continuous,
        );
        assert!(identity_zero.notes.is_empty());
    }

    #[test]
    fn named_sliders_round_trip_as_version_six() {
        let creation = StudioCreation::new("sin(b*x)", -1.0, 1.0, 1.0)
            .expect("creation")
            .with_sliders(vec![
                crate::slider::StudioSlider::new("b", 2.0, 0.25, 8.0).expect("b"),
            ])
            .expect("sliders");
        assert_eq!(creation.sliders().len(), 1);
        assert_eq!(creation.sliders()[0].value(), 2.0);
        let text = creation.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 6\n"), "{text}");
        assert!(text.contains("slider=b:2:0.25:8\n"), "{text}");
        assert_eq!(
            StudioCreation::from_num_file(&text).expect("file"),
            creation
        );
        let link = creation.to_link();
        assert!(link.contains("slider="), "{link}");
        assert_eq!(StudioCreation::from_link(&link).expect("link"), creation);
        let plot = creation.plot_text(48, 16).expect("plot");
        assert!(plot.text.contains('#'));
        let at_default = eval(&parse("sin(b*x)").expect("parse"), 0.5, 1.0);
        let at_bound = eval_named(
            &parse("sin(b*x)").expect("parse"),
            0.5,
            1.0,
            creation.sliders(),
        );
        assert!((at_default - (1.0_f64 * 0.5).sin()).abs() < 1e-12);
        assert!((at_bound - (2.0_f64 * 0.5).sin()).abs() < 1e-12);

        let pair =
            StudioCreation::new_parametric("cos(p*t)", "sin(q*t)", 0.0, std::f64::consts::TAU, 1.0)
                .expect("pair")
                .with_sliders(vec![
                    crate::slider::StudioSlider::new("p", 3.0, 1.0, 8.0).expect("p"),
                    crate::slider::StudioSlider::new("q", 2.0, 1.0, 8.0).expect("q"),
                ])
                .expect("ratio");
        let pair_text = pair.to_num_file();
        assert!(pair_text.starts_with("NUMINOUS_STUDIO 6\n"), "{pair_text}");
        assert!(pair_text.contains("kind=parametric\n"), "{pair_text}");
        assert_eq!(
            StudioCreation::from_num_file(&pair_text).expect("pair file"),
            pair
        );
        let child = pair.fork(None, Some("Remix"), None).expect("fork");
        assert_eq!(child.sliders()[0].value(), 3.0);
        assert_eq!(child.sliders()[1].value(), 2.0);
    }

    #[test]
    fn overlay_programs_round_trip_as_version_seven() {
        let creation = StudioCreation::new_program(
            ["sin(x)", "cos(x)"],
            -std::f64::consts::TAU,
            std::f64::consts::TAU,
            1.0,
        )
        .expect("program")
        .with_title("The parts")
        .expect("title");
        assert_eq!(creation.kind(), StudioKind::Program);
        assert_eq!(creation.extra_sources(), ["cos(x)"]);
        assert_eq!(creation.editor_source(), "sin(x) & cos(x)");
        let text = creation.to_num_file();
        assert!(text.starts_with("NUMINOUS_STUDIO 7\n"), "{text}");
        assert!(text.contains("kind=program\n"), "{text}");
        assert!(text.contains("expr=sin(x)\nexpr=cos(x)\n"), "{text}");
        assert_eq!(
            StudioCreation::from_num_file(&text).expect("file"),
            creation
        );
        let link = creation.to_link();
        assert!(link.contains("kind=program"), "{link}");
        assert_eq!(StudioCreation::from_link(&link).expect("link"), creation);
        let plot = creation.plot_text(48, 16).expect("plot");
        assert!(plot.text.contains('#'));
        assert!(plot.text.contains('*'));
        assert_eq!(
            StudioProgram::from_editor("sin(x) & cos(x)")
                .expect("editor")
                .kind(),
            StudioKind::Program
        );
        let child = creation.fork(None, Some("Remix"), None).expect("fork");
        assert_eq!(child.kind(), StudioKind::Program);
        assert_eq!(child.extra_sources(), ["cos(x)"]);
        let err = StudioCreation::new_program(["sin(x)"], -1.0, 1.0, 1.0).expect_err("one");
        assert!(err.contains("2 to"), "{err}");
        let program_on_six = "NUMINOUS_STUDIO 6\nkind=program\nexpr=sin(x)\nexpr=cos(x)\nxmin=-1\nxmax=1\na=1\nscale=continuous\n";
        let err = StudioCreation::from_num_file(program_on_six).expect_err("program needs v7");
        assert!(err.contains("NUMINOUS_STUDIO 7"), "{err}");
    }

    #[test]
    fn overlay_programs_mix_every_graph_in_wav_and_keep_the_first_in_midi() {
        let creation =
            StudioCreation::new_program(["sin(x)", "cos(x)"], -1.0, 1.0, 1.0).expect("program");
        let mixed = creation.to_melody(8);
        let lead = creation.to_midi_melody(8);
        let first = to_melody_with_scale_named(
            &parse("sin(x)").expect("sin"),
            -1.0,
            1.0,
            8,
            1.0,
            &[],
            StudioScale::Continuous,
        );
        let second = to_melody_with_scale_named(
            &parse("cos(x)").expect("cos"),
            -1.0,
            1.0,
            8,
            1.0,
            &[],
            StudioScale::Continuous,
        );
        assert_eq!(lead, first, "MIDI stays the first graph");
        assert_eq!(mixed.notes.len(), first.notes.len() + second.notes.len());
        for note in first.notes.iter().chain(second.notes.iter()) {
            assert!(
                mixed.notes.iter().any(|sung| sung == note),
                "WAV is missing {note:?}"
            );
        }
        assert_eq!(lead.midi(), first.midi());
        assert_ne!(
            mixed.render(8_000),
            first.render(8_000),
            "the second graph has to change the WAV"
        );
        let graph = StudioCreation::new("sin(x)", -1.0, 1.0, 1.0).expect("graph");
        assert_eq!(
            graph.to_melody(8),
            graph.to_midi_melody(8),
            "a single graph is the same voice in WAV and MIDI"
        );
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
