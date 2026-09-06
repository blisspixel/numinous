//! Optional room explanations and directly addressable mathematical study.
//!
//! This boundary reads existing room prose and authored treatments without
//! Journey state, visits, scores, or reading rewards. Content depth describes
//! what is present, not demonstrated understanding or a complete room audit.
//! Faces own navigation and rendering; equations and prose arrive separately
//! and require no Markdown parser.

use std::{fmt, str::FromStr};

use crate::Room;

mod lissajous;
mod request;

pub use request::{
    MAX_STUDY_BLOCK_ID_BYTES, StudyRequest, StudyRequestError, StudyResponse, StudySelection,
};

#[cfg(test)]
mod tests;

/// Maximum bytes in an accepted study-language request.
pub const MAX_STUDY_LOCALE_BYTES: usize = 63;

/// A bounded, case-insensitive language request for study content.
///
/// The accepted syntax is two through eight ASCII letters, followed by at
/// most seven hyphen-separated subtags of two through eight ASCII letters or
/// digits. Examples include `en`, `ja-JP`, `zh-Hant-TW`, `haw`, and `tlh`.
/// Canonical storage is lowercase ASCII. Whitespace, underscores, singleton
/// extensions and private-use tags are not accepted.
///
/// This is a documented input grammar, not full BCP 47 validation: it neither
/// checks the language registry nor implements extension/variant semantics.
/// Syntactically admitted unknown requests receive an explicit fallback.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StudyLocale(String);

impl StudyLocale {
    /// Validate and canonicalize a request using this type's bounded grammar.
    ///
    /// # Errors
    /// Returns a typed error for empty, oversized, or unsupported syntax.
    pub fn parse(requested: &str) -> Result<Self, StudyLocaleError> {
        if requested.is_empty() {
            return Err(StudyLocaleError::Empty);
        }
        if requested.len() > MAX_STUDY_LOCALE_BYTES {
            return Err(StudyLocaleError::TooLong);
        }
        let mut parts = requested.split('-');
        let Some(language) = parts.next() else {
            return Err(StudyLocaleError::Empty);
        };
        if !(2..=8).contains(&language.len())
            || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            return Err(StudyLocaleError::InvalidSyntax);
        }
        for (index, subtag) in parts.enumerate() {
            if index >= 7
                || !(2..=8).contains(&subtag.len())
                || !subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                return Err(StudyLocaleError::InvalidSyntax);
            }
        }
        Ok(Self(requested.to_ascii_lowercase()))
    }

    /// Canonical request, suitable for a persisted preference or protocol value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Primary language subtag, used by the documented parent-language lookup.
    #[must_use]
    pub fn language(&self) -> &str {
        self.0
            .split_once('-')
            .map_or(self.0.as_str(), |(language, _)| language)
    }
}

impl Default for StudyLocale {
    fn default() -> Self {
        Self("en".to_string())
    }
}

impl FromStr for StudyLocale {
    type Err = StudyLocaleError;

    fn from_str(requested: &str) -> Result<Self, Self::Err> {
        Self::parse(requested)
    }
}

impl fmt::Display for StudyLocale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Rejection of a study-language request before content lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyLocaleError {
    /// An explicit request must contain a language tag; the default is `en`.
    Empty,
    /// The request exceeds [`MAX_STUDY_LOCALE_BYTES`].
    TooLong,
    /// The request falls outside [`StudyLocale`]'s documented syntax.
    InvalidSyntax,
}

impl fmt::Display for StudyLocaleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("study language must not be empty"),
            Self::TooLong => write!(
                formatter,
                "study language must be at most {MAX_STUDY_LOCALE_BYTES} bytes"
            ),
            Self::InvalidSyntax => formatter.write_str(
                "study language must use 2-8 ASCII letters followed by at most seven \
                 hyphen-separated 2-8 letter-or-digit subtags",
            ),
        }
    }
}

impl std::error::Error for StudyLocaleError {}

/// Why the resolved text language differs from the canonical request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyFallback {
    /// A regional or other subtag request uses its available primary language.
    ParentLanguage,
    /// Text is unavailable in the requested language and is returned in English.
    TranslationUnavailable,
}

impl StudyFallback {
    /// Stable protocol name, independent of the language used to explain it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ParentLanguage => "parent_language",
            Self::TranslationUnavailable => "translation_unavailable",
        }
    }
}

/// Explicit language lookup result for a document or individual block.
///
/// Lookup tries the requested tag's primary language, then English. The
/// document result describes its authored treatment when present; each block
/// also reports its own result, because existing catalog notes may remain in
/// English beside a translated treatment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyLocaleResolution {
    /// Validated canonical request, not an inferred operating-system setting.
    pub requested: StudyLocale,
    /// Language actually used for human prose in this result.
    pub resolved: &'static str,
    /// Explicit explanation of any fallback.
    pub fallback: Option<StudyFallback>,
}

impl StudyLocaleResolution {
    fn new(requested: &StudyLocale, resolved: &'static str) -> Self {
        let fallback = if requested.as_str() == resolved {
            None
        } else if requested.language() == resolved {
            Some(StudyFallback::ParentLanguage)
        } else {
            Some(StudyFallback::TranslationUnavailable)
        };
        Self {
            requested: requested.clone(),
            resolved,
            fallback,
        }
    }
}

/// Optional reading depth, independent of progression and language review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StudyDepth {
    /// Brief existing explanation or an authored experiment and intuition.
    Explanation,
    /// Existing deep cuts and reading suggestions, not a rigorous treatment.
    Notes,
    /// An authored worked treatment with assumptions, derivations and limits.
    Mathematics,
}

impl StudyDepth {
    /// The complete depth set in display order; this order is not a reading gate.
    pub const ALL: [Self; 3] = [Self::Explanation, Self::Notes, Self::Mathematics];

    /// Stable protocol name, independent of translated reader labels.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explanation => "explanation",
            Self::Notes => "notes",
            Self::Mathematics => "mathematics",
        }
    }

    /// Parse one exact canonical depth name, with no aliases or inferred default.
    ///
    /// # Errors
    /// Returns [`StudyDepthError`] unless the value is `explanation`, `notes`,
    /// or `mathematics`. Leading/trailing whitespace and case changes refuse.
    pub fn parse(value: &str) -> Result<Self, StudyDepthError> {
        match value {
            "explanation" => Ok(Self::Explanation),
            "notes" => Ok(Self::Notes),
            "mathematics" => Ok(Self::Mathematics),
            _ => Err(StudyDepthError),
        }
    }
}

impl FromStr for StudyDepth {
    type Err = StudyDepthError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// A depth request did not name one of [`StudyDepth::ALL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudyDepthError;

impl fmt::Display for StudyDepthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("study depth must be explanation, notes, or mathematics")
    }
}

impl std::error::Error for StudyDepthError {}

/// Evidence about translation, separate from whether a depth is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyTranslationStatus {
    /// Original English prose; this makes no independent review claim.
    Original,
    /// A pilot translation with an independent text and mathematical review.
    ///
    /// This remains a draft: no native-speaker participant study, rendered
    /// reader validation, complete product localization, or learning result
    /// is implied. The review's four shared mathematical clarifications are
    /// incorporated in both pilot languages.
    ReviewedDraft,
}

impl StudyTranslationStatus {
    /// Stable protocol name for the stated translation evidence.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::ReviewedDraft => "reviewed_draft",
        }
    }
}

/// A language-independent scientific reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StudySource {
    /// Stable scientific source identifier, unchanged by translation.
    pub id: &'static str,
    /// Original reference title, including any precise section locator.
    pub title: &'static str,
    /// Direct scientific source URL, shared by all languages.
    pub url: &'static str,
}

/// One inline role for a text formatter or case-preserving glyph-run reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyInline {
    /// Human prose, preserved exactly in its resolved language.
    Text(&'static str),
    /// Literal scientific notation; never translate or uppercase it.
    Math(&'static str),
}

impl StudyInline {
    /// Return the exact source spelling, without changing case or notation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text(text) | Self::Math(text) => text,
        }
    }
}

/// Structured content without embedded Markdown or a parser dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyPart {
    /// A paragraph whose inline runs are concatenated without added spaces.
    Paragraph(Vec<StudyInline>),
    /// Display notation with significant newlines, shared across languages.
    Equation(&'static str),
    /// A shared source accompanied by a localized description of its use.
    Reference {
        /// Canonical scientific identity, title and URL.
        source: &'static StudySource,
        /// Human description in the block's resolved language.
        description: &'static str,
    },
}

impl StudyPart {
    /// Render literal text with equations and source URLs intact.
    ///
    /// Paragraph runs concatenate without inserted spaces. Display equations
    /// retain newlines. A reference includes its original title, translated
    /// description, and canonical URL on separate lines. No Markdown parsing,
    /// escaping, uppercasing, or language fallback is performed here.
    #[must_use]
    pub fn plain_text(&self) -> String {
        match self {
            Self::Paragraph(runs) => runs.iter().map(StudyInline::as_str).collect(),
            Self::Equation(text) => (*text).to_string(),
            Self::Reference {
                source,
                description,
            } => format!("{}\n{description}\n{}", source.title, source.url),
        }
    }
}

/// An optional, directly addressable block of room study.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyBlock {
    /// Stable room-qualified ID; sequence order is not a prerequisite.
    pub id: String,
    /// Heading in the block's resolved language.
    pub title: &'static str,
    /// What kind of content the block supplies.
    pub depth: StudyDepth,
    /// The block's actual text language, including explicit fallback.
    pub locale: StudyLocaleResolution,
    /// Translation evidence; independent of mathematical depth availability.
    pub translation: StudyTranslationStatus,
    /// Ordered content parts; faces may wrap or scroll them without parsing.
    pub parts: Vec<StudyPart>,
}

impl StudyBlock {
    /// Render the heading and literal body for shared CLI/MCP text output.
    ///
    /// The caller reports locale, availability and translation evidence
    /// separately; those metadata are not silently mixed into scientific prose.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut text = self.title.to_string();
        for part in &self.parts {
            text.push_str("\n\n");
            text.push_str(&part.plain_text());
        }
        text
    }
}

/// Current study content for one room, with no player or progression state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomStudy {
    /// Canonical room ID, reused from [`crate::RoomMetadata::meta`].
    pub room_id: &'static str,
    /// Preferred document language; individual blocks may explicitly fall back.
    pub locale: StudyLocaleResolution,
    /// Languages with at least one authored block, not complete UI coverage.
    pub content_locales: &'static [&'static str],
    /// All available blocks. There are no hidden or progression-locked blocks.
    pub blocks: Vec<StudyBlock>,
}

impl RoomStudy {
    /// Open any stable block ID directly, without reading earlier blocks.
    #[must_use]
    pub fn block(&self, id: &str) -> Option<&StudyBlock> {
        self.blocks.iter().find(|block| block.id == id)
    }

    /// Whether this depth has content. Notes do not satisfy `Mathematics`.
    #[must_use]
    pub fn has_depth(&self, depth: StudyDepth) -> bool {
        self.blocks.iter().any(|block| block.depth == depth)
    }

    /// All blocks at a chosen depth, with no visitation or reading side effects.
    pub fn blocks_at(&self, depth: StudyDepth) -> impl Iterator<Item = &StudyBlock> {
        self.blocks.iter().filter(move |block| block.depth == depth)
    }
}

/// Rooms with an authored [`StudyDepth::Mathematics`] treatment, in catalog order.
///
/// This is a content-coverage fact, not a permission list. Reading is never
/// gated, so every depth a room has is open to every player from the first
/// moment. What this names is where a treatment has actually been *written*,
/// which is a much smaller set and grows one authored room at a time.
///
/// It exists so an unwritten depth reads as unwritten rather than as broken.
/// A player who asks for mathematics and is refused can otherwise only find
/// out where it does exist by asking again, room by room, across the catalog.
pub const AUTHORED_MATHEMATICS_ROOMS: &[&str] = &["lissajous"];

/// Which rooms carry an authored treatment at this depth, for honest refusals.
///
/// [`StudyDepth::Explanation`] and [`StudyDepth::Notes`] return an empty slice
/// on purpose. Every room reuses its existing concept, reveal, deep cuts, and
/// citation for those two, so both are always available and naming rooms would
/// name the entire catalog while telling the reader nothing.
#[must_use]
pub fn rooms_with_authored_depth(depth: StudyDepth) -> &'static [&'static str] {
    match depth {
        StudyDepth::Mathematics => AUTHORED_MATHEMATICS_ROOMS,
        StudyDepth::Explanation | StudyDepth::Notes => &[],
    }
}

/// Read optional room study using a bounded explicit language request.
///
/// Every room reuses its existing concept and reveal, plus its existing deep
/// cuts and citation as notes. Only the authored Lissajous pilot currently
/// supplies [`StudyDepth::Mathematics`], in English and draft Japanese.
/// This function does not accept or mutate Journey state.
///
/// # Errors
/// Returns [`StudyLocaleError`] for a request outside the documented grammar.
pub fn room_study(room: &dyn Room, requested_locale: &str) -> Result<RoomStudy, StudyLocaleError> {
    let requested = StudyLocale::parse(requested_locale)?;
    Ok(room_study_for_locale(room, &requested))
}

/// Read room study with a previously validated request, such as a preference.
#[must_use]
pub fn room_study_for_locale(room: &dyn Room, requested: &StudyLocale) -> RoomStudy {
    let room_id = room.meta().id;
    // One source of truth with AUTHORED_MATHEMATICS_ROOMS: a room that supplies
    // authored blocks and a room that is advertised as having them must be the
    // same room, or a refusal would name a room that refuses in turn.
    let has_pilot = AUTHORED_MATHEMATICS_ROOMS.contains(&room_id);
    let resolved = if has_pilot && requested.language() == "ja" {
        "ja"
    } else {
        "en"
    };
    let locale = StudyLocaleResolution::new(requested, resolved);
    let original = StudyLocaleResolution::new(requested, "en");
    let mut explanation = Vec::new();
    if let Some(concept) = room.concept() {
        explanation.push(StudyPart::Paragraph(vec![StudyInline::Text(concept)]));
    }
    explanation.push(StudyPart::Paragraph(vec![StudyInline::Text(room.reveal())]));
    let mut blocks = if has_pilot {
        lissajous::blocks(&locale)
    } else {
        Vec::new()
    };
    blocks.push(catalog_block(
        format!("{room_id}.explanation"),
        if has_pilot {
            "Existing room explanation"
        } else {
            "Room explanation"
        },
        if has_pilot {
            StudyDepth::Notes
        } else {
            StudyDepth::Explanation
        },
        &original,
        explanation,
    ));
    for (index, cut) in room.deep_cuts().iter().enumerate() {
        blocks.push(catalog_block(
            format!("{room_id}.note.{index}"),
            "Existing note",
            StudyDepth::Notes,
            &original,
            vec![StudyPart::Paragraph(vec![StudyInline::Text(cut)])],
        ));
    }
    blocks.push(catalog_block(
        format!("{room_id}.citation"),
        "Existing reading suggestion",
        StudyDepth::Notes,
        &original,
        vec![StudyPart::Paragraph(vec![StudyInline::Text(
            room.citation(),
        )])],
    ));
    RoomStudy {
        room_id,
        locale,
        content_locales: if has_pilot { &["en", "ja"] } else { &["en"] },
        blocks,
    }
}

fn catalog_block(
    id: String,
    title: &'static str,
    depth: StudyDepth,
    locale: &StudyLocaleResolution,
    parts: Vec<StudyPart>,
) -> StudyBlock {
    StudyBlock {
        id,
        title,
        depth,
        locale: locale.clone(),
        translation: StudyTranslationStatus::Original,
        parts,
    }
}
