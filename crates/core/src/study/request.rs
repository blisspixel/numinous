//! Shared defaults and selection policy for optional study requests.

use std::fmt;

use crate::Room;

use super::{
    RoomStudy, StudyBlock, StudyDepth, StudyDepthError, StudyLocale, StudyLocaleError,
    StudyLocaleResolution, room_study_for_locale,
};

/// Maximum bytes in a directly addressed room-study block ID.
pub const MAX_STUDY_BLOCK_ID_BYTES: usize = 128;

/// An explicit depth or one directly addressed block, never both.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudySelection {
    /// Every available block at the selected optional depth.
    Depth(StudyDepth),
    /// One stable, room-qualified block ID.
    Block(String),
}

impl StudySelection {
    fn matches(&self, block: &StudyBlock) -> bool {
        match self {
            Self::Depth(depth) => block.depth == *depth,
            Self::Block(id) => block.id == *id,
        }
    }
}

/// A validated request whose defaults and selection semantics are shared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyRequest {
    locale: StudyLocale,
    selection: StudySelection,
}

impl StudyRequest {
    /// Validate optional transport strings without inferring a player state.
    ///
    /// Omitted locale means English. Omitting both depth and block means
    /// [`StudyDepth::Explanation`]. An explicitly empty value is not omission.
    /// Depth uses [`StudyDepth::parse`]. Block IDs contain lowercase ASCII
    /// alphanumeric components separated by dots or hyphens, with at least
    /// one dot and no empty component; at most [`MAX_STUDY_BLOCK_ID_BYTES`]
    /// bytes are admitted. Existence is checked when selecting content.
    ///
    /// # Errors
    /// A supplied depth and block conflict before individual value validation.
    /// Otherwise malformed locale, depth, or block values return typed errors.
    pub fn parse(
        locale: Option<&str>,
        depth: Option<&str>,
        block: Option<&str>,
    ) -> Result<Self, StudyRequestError> {
        if depth.is_some() && block.is_some() {
            return Err(StudyRequestError::ConflictingSelection);
        }
        let locale = match locale {
            Some(value) => StudyLocale::parse(value).map_err(StudyRequestError::Locale)?,
            None => StudyLocale::default(),
        };
        let selection = if let Some(id) = block {
            if id.len() > MAX_STUDY_BLOCK_ID_BYTES {
                return Err(StudyRequestError::BlockTooLong);
            }
            if !id.contains('.')
                || id.split(['.', '-']).any(|component| {
                    component.is_empty()
                        || !component
                            .bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                })
            {
                return Err(StudyRequestError::InvalidBlockSyntax);
            }
            StudySelection::Block(id.to_string())
        } else {
            StudySelection::Depth(match depth {
                Some(value) => StudyDepth::parse(value).map_err(StudyRequestError::Depth)?,
                None => StudyDepth::Explanation,
            })
        };
        Ok(Self { locale, selection })
    }

    /// Canonical requested language, reusable when reading a document separately.
    #[must_use]
    pub fn locale(&self) -> &StudyLocale {
        &self.locale
    }

    /// Explicit selection after applying the shared omission defaults.
    #[must_use]
    pub fn selection(&self) -> &StudySelection {
        &self.selection
    }

    /// Read and select room content without recording visits or reading rewards.
    ///
    /// # Errors
    /// Returns a typed unavailable-depth or unavailable-block error. Notes or
    /// another block never substitute for deliberately requested mathematics.
    pub fn read(&self, room: &dyn Room) -> Result<StudyResponse, StudyRequestError> {
        self.select(room_study_for_locale(room, &self.locale))
    }

    /// Select from a previously read document in this request's language.
    ///
    /// # Errors
    /// Refuses a mismatched requested locale or unavailable selection. Resolving
    /// a supported locale to a parent language or English is not a mismatch.
    pub fn select(&self, document: RoomStudy) -> Result<StudyResponse, StudyRequestError> {
        if document.locale.requested != self.locale {
            return Err(StudyRequestError::LocaleMismatch);
        }
        if !document
            .blocks
            .iter()
            .any(|block| self.selection.matches(block))
        {
            return Err(match &self.selection {
                StudySelection::Depth(depth) => StudyRequestError::DepthUnavailable(*depth),
                StudySelection::Block(id) => StudyRequestError::BlockUnavailable(id.clone()),
            });
        }
        Ok(StudyResponse {
            document,
            selection: self.selection.clone(),
        })
    }
}

/// A document and its validated selection, with no mutable player state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudyResponse {
    document: RoomStudy,
    selection: StudySelection,
}

impl StudyResponse {
    /// All content and availability metadata for the selected room.
    #[must_use]
    pub fn document(&self) -> &RoomStudy {
        &self.document
    }

    /// The exact selection that was satisfied, including any defaulted depth.
    #[must_use]
    pub fn selection(&self) -> &StudySelection {
        &self.selection
    }

    /// Selected blocks in authored order; direct ID selection yields one block.
    pub fn selected_blocks(&self) -> impl Iterator<Item = &StudyBlock> {
        self.document
            .blocks
            .iter()
            .filter(move |block| self.selection.matches(block))
    }

    /// Shared literal text response, including explicit coverage and fallback.
    ///
    /// The metadata labels are canonical English transport labels. Scientific
    /// content and headings retain each block's resolved language and case;
    /// this formatter does not claim a translated application shell.
    #[must_use]
    pub fn plain_text(&self) -> String {
        let depths = StudyDepth::ALL
            .iter()
            .map(|depth| {
                let availability = if self.document.has_depth(*depth) {
                    "available"
                } else {
                    "unavailable"
                };
                format!("{}={availability}", depth.as_str())
            })
            .collect::<Vec<_>>()
            .join(", ");
        let selection = match &self.selection {
            StudySelection::Depth(depth) => format!("depth={}", depth.as_str()),
            StudySelection::Block(id) => format!("block={id}"),
        };
        let block_ids = self
            .document
            .blocks
            .iter()
            .map(|block| block.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let mut text = format!(
            "Room: {}\nStudy language: {}\nLanguages with content: {}\nDepths: {depths}\nAvailable blocks: {block_ids}\nSelection: {selection}",
            self.document.room_id,
            locale_text(&self.document.locale),
            self.document.content_locales.join(", "),
        );
        for block in self.selected_blocks() {
            text.push_str(&format!(
                "\n\nBlock: {}\nDepth: {}\nText language: {}\nTranslation: {}\n\n{}",
                block.id,
                block.depth.as_str(),
                locale_text(&block.locale),
                block.translation.as_str(),
                block.plain_text(),
            ));
        }
        text.push('\n');
        text
    }
}

fn locale_text(locale: &StudyLocaleResolution) -> String {
    let fallback = locale.fallback.map_or("none", |reason| reason.as_str());
    format!(
        "requested={} resolved={} fallback={fallback}",
        locale.requested, locale.resolved,
    )
}

/// A study request was malformed or could not select the requested content.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StudyRequestError {
    /// The explicit language request did not pass the shared bounded grammar.
    Locale(StudyLocaleError),
    /// The explicit depth was not a canonical name.
    Depth(StudyDepthError),
    /// A request supplied both a depth and a directly addressed block.
    ConflictingSelection,
    /// A block ID exceeded [`MAX_STUDY_BLOCK_ID_BYTES`].
    BlockTooLong,
    /// A block ID did not use the documented room-qualified syntax.
    InvalidBlockSyntax,
    /// The document contains no block at the requested depth.
    DepthUnavailable(StudyDepth),
    /// The document does not contain this validated, bounded block ID.
    BlockUnavailable(String),
    /// A two-stage selection used a different document request language.
    LocaleMismatch,
}

impl fmt::Display for StudyRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locale(error) => fmt::Display::fmt(error, formatter),
            Self::Depth(error) => fmt::Display::fmt(error, formatter),
            Self::ConflictingSelection => {
                formatter.write_str("choose a study depth or a block ID, not both")
            }
            Self::BlockTooLong => write!(
                formatter,
                "study block ID must be at most {MAX_STUDY_BLOCK_ID_BYTES} bytes"
            ),
            Self::InvalidBlockSyntax => formatter.write_str(
                "study block ID must be room-qualified lowercase ASCII components \
                 separated by dots or hyphens, with no empty component",
            ),
            Self::DepthUnavailable(depth) => {
                write!(
                    formatter,
                    "study depth {} is unavailable for this room",
                    depth.as_str()
                )
            }
            Self::BlockUnavailable(id) => {
                write!(formatter, "study block {id} is unavailable for this room")
            }
            Self::LocaleMismatch => formatter
                .write_str("study document and request must use the same requested language"),
        }
    }
}

impl std::error::Error for StudyRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Locale(error) => Some(error),
            Self::Depth(error) => Some(error),
            _ => None,
        }
    }
}
