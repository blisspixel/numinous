//! One chosen project the player can keep, import, and preview.
//!
//! The document holds a question, one next tool call, catalog room ids,
//! typed evidence links, and an optional Studio or route creation. Keeping, importing,
//! and resuming are separate acts. Resume reports what is present, missing,
//! corrected, collided, or incompatible. It does not call the next tool, and
//! it does not write the journal or the workspace.
//!
//! There is no default path. A face that stores the chain passes an explicit
//! file. This is not the `portable-1` evidence capsule. A creation capsule
//! alone is not a project.

use std::fmt;

use crate::encounter::EncounterTool;
use crate::journal::{
    JOURNAL_SOURCE_PLAYER_PROVIDED, JOURNAL_SOURCE_SELF_AUTHORED, JOURNAL_SUBJECT_RECEIPT_PREFIX,
    Journal, JournalEntry,
};
use crate::path_closure::PathClosure;
use crate::route_creation::{MAX_ROUTE_CAPSULE_BYTES, RouteCreation};
use crate::sha256::{digest as sha256, hex as sha256_hex};
use crate::studio::{MAX_SHARE_INPUT_BYTES, StudioCreation};
use crate::{MAX_WORKSPACE_TEXT_CHARS, canonical_room_id, room_meta_by_id};

/// Portable one-revision header. A later version is refused, not skipped.
pub const PROJECT_DOCUMENT_HEADER: &str = "NUMINOUS_PROJECT 1";
/// Local append-only chain header. This file is not an import document.
pub const PROJECT_CHAIN_HEADER: &str = "numinous-project-v1";
/// Portable document version supporting authored route creations.
pub const PROJECT_DOCUMENT_HEADER_V2: &str = "NUMINOUS_PROJECT 2";
/// Append-only chain version supporting route creations and next calls.
pub const PROJECT_CHAIN_HEADER_V2: &str = "numinous-project-v2";
/// Resume preview schema name. Core does not emit JSON.
pub const PROJECT_RESUME_PREVIEW_SCHEMA: &str = "numinous.project-resume-preview";
/// Resume preview schema version.
pub const PROJECT_RESUME_PREVIEW_VERSION: u32 = 1;
/// Revisions retained in one chain, including corrections.
pub const MAX_PROJECT_REVISIONS: usize = 16;
/// Bytes allowed in one chain file.
pub const MAX_PROJECT_FILE_BYTES: u64 = 256 * 1024;
/// Rooms named by one revision.
pub const MAX_PROJECT_ROOMS: usize = 4;
/// Typed evidence links on one revision.
pub const MAX_PROJECT_EVIDENCE: usize = 4;
/// Characters allowed in a room id before the catalog check.
const MAX_ROOM_ID_CHARS: usize = 64;

const PORTABLE_FIELDS: usize = 6;
const CHAIN_FIELDS: usize = 11;

/// Why a project document or chain was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    /// The revision cap is already full.
    Capacity,
    /// The chain text would pass [`MAX_PROJECT_FILE_BYTES`].
    TooLarge,
    /// No further revision id can be assigned.
    IdentifierExhausted,
    /// The text is not this version of the document or the chain.
    InvalidFormat(String),
    /// The question is empty, too long, or contains a control character.
    InvalidQuestion(String),
    /// The next call is outside the closed set or its arguments do not fit.
    InvalidNext(String),
    /// A room id is not a catalog room, is repeated, or breaks the cap.
    InvalidRoom(String),
    /// An evidence link is not a journal or receipt digest.
    InvalidEvidence(String),
    /// The creation is not supported portable creation data.
    InvalidCreation(String),
    /// The source is outside the two project acts.
    InvalidSource,
    /// The chain has no revision to preview.
    Empty,
    /// A correction or origin target is not in this chain.
    MissingRevision(u64),
    /// That revision already has a successor.
    AlreadySuperseded(u64),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity => write!(formatter, "project revision limit reached"),
            Self::TooLarge => write!(
                formatter,
                "project chain would exceed {MAX_PROJECT_FILE_BYTES} bytes"
            ),
            Self::IdentifierExhausted => {
                write!(formatter, "project revision identifiers exhausted")
            }
            Self::InvalidFormat(message) => write!(formatter, "invalid project format: {message}"),
            Self::InvalidQuestion(message) => {
                write!(formatter, "invalid project question: {message}")
            }
            Self::InvalidNext(message) => write!(formatter, "invalid project next: {message}"),
            Self::InvalidRoom(message) => write!(formatter, "invalid project room: {message}"),
            Self::InvalidEvidence(message) => {
                write!(formatter, "invalid project evidence: {message}")
            }
            Self::InvalidCreation(message) => {
                write!(formatter, "invalid project creation: {message}")
            }
            Self::InvalidSource => write!(formatter, "project source is not recognized"),
            Self::Empty => write!(formatter, "project chain is empty"),
            Self::MissingRevision(id) => write!(formatter, "project revision {id} does not exist"),
            Self::AlreadySuperseded(id) => {
                write!(formatter, "project revision {id} is already superseded")
            }
        }
    }
}

impl std::error::Error for ProjectError {}

/// The one next call a revision may name.
///
/// Arguments are produced here. A question is never read as a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectNext {
    /// Open one portable creation. `capsule` is an id, a link, or `.num` text.
    OpenCreation {
        /// Capsule text `open_creation` accepts. Not a filesystem path.
        capsule: String,
    },
    /// Fork this revision's creation. The parent link is derived at preview.
    ForkCreation,
    /// Open one portable route creation through the shared workbench.
    OpenRoute {
        /// Canonical route capsule data, never a filesystem path.
        capsule: String,
    },
    /// Remix this revision's embedded route creation.
    RemixRoute,
    /// Play one room that this revision also names.
    PlayRoom {
        /// Canonical catalog id.
        room: String,
        /// Finite phase in `[0, 1)`, as a JSON number token, when the caller set one.
        phase: Option<String>,
    },
    /// Study one room that this revision also names.
    StudyRoom {
        /// Canonical catalog id.
        room: String,
    },
}

/// A typed link to a journal record or a receipt digest.
///
/// The project stores the digest, not the journal text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEvidence {
    /// `sha256` of one journal entry's identity bytes, plus an optional local hint.
    Journal {
        /// Content digest. The local id is not the identity.
        digest: [u8; 32],
        /// Journal-local id the caller had in hand, when they had one.
        entry_id: Option<u64>,
    },
    /// Encounter result digest, plus an optional closed tool name.
    Receipt {
        /// `result_digest` of a receipt, or the subject `receipt:<hex>`.
        digest: [u8; 32],
        /// Tool that issued the receipt, when the caller named one.
        tool: Option<EncounterTool>,
    },
}

/// Fields the caller supplies for keep or correct.
///
/// Source, parent identity, and the local id are assigned by the act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDraft {
    /// Unix seconds when this act was accepted.
    pub recorded_at_utc: u64,
    /// The question, stored as data.
    pub question: String,
    /// The one next call.
    pub next: ProjectNext,
    /// One to four catalog rooms. Aliases are resolved while keeping.
    pub rooms: Vec<String>,
    /// Zero to four typed links.
    pub evidence: Vec<ProjectEvidence>,
    /// Optional Studio capsule or bounded portable route capsule text.
    pub creation: Option<String>,
}

/// What a write did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectStore {
    /// A new revision was appended.
    Appended {
        /// Local revision id.
        revision_id: u64,
        /// Payload identity, lowercase hex.
        identity_hex: String,
    },
    /// The same payload bytes were already in the chain. Nothing was written.
    AlreadyPresent {
        /// The existing revision.
        revision_id: u64,
        /// Payload identity, lowercase hex.
        identity_hex: String,
    },
    /// The origin id is taken by different bytes. Nothing was written.
    Collided {
        /// The local revision the caller named.
        origin_revision: u64,
    },
    /// Import will append only when `confirm` is set. Nothing was written.
    NeedsConfirm,
}

/// A receipt the caller asks preview to compare.
///
/// Core does not replay the tool. The face verifies, then passes the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptCheck {
    /// The caller did not supply a receipt.
    NotSupplied,
    /// Verification produced this result digest.
    Verified([u8; 32]),
    /// The caller supplied a receipt and verification disagreed.
    Disagreed,
}

/// Whether a named room can be opened from this catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomStatus {
    /// The id is the canonical catalog id.
    Present,
    /// The token is well formed and this catalog does not list it.
    Missing,
    /// The token is not a canonical id this document can carry.
    Incompatible,
}

/// One room in a resume preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomFact {
    /// Stored room id.
    pub id: String,
    /// Catalog status of that id.
    pub status: RoomStatus,
}

/// Whether a cited record can be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    /// The digest matches a current journal entry.
    Present,
    /// No current or corrected entry has this digest.
    Missing,
    /// The cited entry has a later correction. The cited text is the one returned.
    Corrected,
    /// The hinted local id is a different record. Its text is withheld.
    Collided,
    /// A supplied receipt does not match this link.
    Incompatible,
}

/// One evidence link in a resume preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceFact {
    /// `journal` or `receipt`.
    pub kind: &'static str,
    /// Lowercase digest hex.
    pub digest_hex: String,
    /// Resolution of this link.
    pub status: EvidenceStatus,
    /// Journal entry that matched, when text is returned.
    pub entry_id: Option<u64>,
    /// Direct correction of that entry, when status is corrected.
    pub superseded_by: Option<u64>,
    /// Subject of the cited entry. Absent when the text is withheld.
    pub subject: Option<String>,
    /// Text of the cited entry. Absent when the text is withheld.
    pub text: Option<String>,
    /// Receipt tool name, when the link named one.
    pub tool: Option<&'static str>,
    /// The digest exists on some other entry, and this hint does not name it.
    pub same_bytes_elsewhere: bool,
}

/// Whether the embedded creation reopened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationStatus {
    /// No creation was stored.
    Missing,
    /// The stored creation reopened and matches its canonical text.
    Present,
    /// The stored text is not a supported creation, or it does not replay.
    Incompatible,
}

/// The closed kinds of portable creation a project may embed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationKind {
    /// A Studio expression or construction.
    Studio,
    /// An authored street network and delivery order.
    Route,
}

/// The creation a resume would hand back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreationFact {
    /// Reopen status.
    pub status: CreationStatus,
    /// Supported creation kind, when the reopen succeeded.
    pub kind: Option<CreationKind>,
    /// Canonical creation text for explicit reopening, when present.
    pub capsule: Option<String>,
    /// Studio lineage link or route parent digest, when the reopen succeeded.
    pub descends: Option<String>,
    /// The caller handed in a native link. A link does not carry lineage.
    pub lineage_was_not_in_the_link: bool,
    /// Recomputed least period when the creation is a periodic path.
    pub period_text: Option<String>,
}

/// One argument of a followable next call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectArgument {
    /// Argument name on the tool.
    pub name: &'static str,
    /// Argument value.
    pub value: ProjectArgumentValue,
}

/// Text, or a JSON number token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectArgumentValue {
    /// A string argument.
    Text(String),
    /// A number argument whose token round-trips through `f64`.
    Number(String),
}

/// A next call that can be issued as named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCall {
    /// Tool name.
    pub tool: &'static str,
    /// Arguments in a stable order.
    pub arguments: Vec<ProjectArgument>,
}

/// Why a stored next call cannot be issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompatibleNext {
    /// Tool name stored on the revision, when it was one of the closed set.
    pub tool: String,
    /// What failed.
    pub reason: String,
}

/// The next call, or the reason it cannot be followed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextPreview {
    /// The call can be issued as named.
    Ready(ProjectCall),
    /// The stored call does not fit this catalog or this creation.
    Incompatible(IncompatibleNext),
}

/// What resume would return. Nothing here has been applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumePreview {
    /// Schema name.
    pub schema: &'static str,
    /// Schema version.
    pub version: u32,
    /// Local revision that was previewed.
    pub revision_id: u64,
    /// The question, as stored.
    pub question: String,
    /// Always false. The question is not interpreted.
    pub interpreted: bool,
    /// The next call.
    pub next: NextPreview,
    /// Rooms, in stored order.
    pub rooms: Vec<RoomFact>,
    /// Evidence links, in stored order.
    pub evidence: Vec<EvidenceFact>,
    /// Embedded creation.
    pub creation: CreationFact,
    /// The question, a compatible next call, and every room can be shown,
    /// the creation is not incompatible, no evidence item is collided or
    /// incompatible, and this revision is still current.
    pub will_return: bool,
    /// Preview does not apply the project.
    pub not_applied: bool,
    /// Preview does not write the workspace.
    pub workspace_changed: bool,
    /// Preview does not write the journal.
    pub journal_changed: bool,
    /// Later revision that supersedes this one, when there is one.
    pub superseded_by: Option<u64>,
    /// Whether a parent identity names a revision in this chain.
    /// Absent when this revision has no parent.
    pub parent_resolved: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Payload {
    question: String,
    next: ProjectNext,
    rooms: Vec<String>,
    evidence: Vec<ProjectEvidence>,
    creation_num: Option<String>,
    parent_hex: String,
}

/// One stored revision. Local id and source are not part of the payload identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRevision {
    id: u64,
    recorded_at_utc: u64,
    source: String,
    payload: Payload,
    origin_revision: Option<u64>,
    supersedes: Option<u64>,
    creation_from_link: bool,
}

impl ProjectRevision {
    /// Local revision id.
    #[must_use]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Unix seconds when this revision was accepted.
    #[must_use]
    pub fn recorded_at_utc(&self) -> u64 {
        self.recorded_at_utc
    }

    /// `self-authored` for keep and correct, `player-provided` for import.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Stored question.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.payload.question
    }

    /// Stored next call.
    #[must_use]
    pub fn next(&self) -> &ProjectNext {
        &self.payload.next
    }

    /// Canonical room ids.
    #[must_use]
    pub fn rooms(&self) -> &[String] {
        &self.payload.rooms
    }

    /// Typed evidence links.
    #[must_use]
    pub fn evidence(&self) -> &[ProjectEvidence] {
        &self.payload.evidence
    }

    /// Canonical creation text, when a creation was stored.
    #[must_use]
    pub fn creation_num(&self) -> Option<&str> {
        self.payload.creation_num.as_deref()
    }

    /// Canonical Studio or route capsule text, when a creation was stored.
    #[must_use]
    pub fn creation_capsule(&self) -> Option<&str> {
        self.creation_num()
    }

    /// Whether the creation input was a native link.
    #[must_use]
    pub fn creation_from_link(&self) -> bool {
        self.creation_from_link
    }

    /// Caller-supplied origin id, when import recorded one.
    #[must_use]
    pub fn origin_revision(&self) -> Option<u64> {
        self.origin_revision
    }

    /// Revision this one corrects, when it is a correction.
    #[must_use]
    pub fn supersedes(&self) -> Option<u64> {
        self.supersedes
    }

    /// Parent payload identity, lowercase hex, when one was stored.
    #[must_use]
    pub fn parent_hex(&self) -> Option<&str> {
        if self.payload.parent_hex.is_empty() {
            None
        } else {
            Some(self.payload.parent_hex.as_str())
        }
    }

    /// SHA-256 of the portable payload line, lowercase hex.
    #[must_use]
    pub fn identity_hex(&self) -> String {
        self.payload.identity_hex()
    }

    /// Portable one-revision document. The link flag is chain metadata and
    /// is not part of this text.
    #[must_use]
    pub fn to_document(&self) -> String {
        let header = if self.payload.needs_route_version() {
            PROJECT_DOCUMENT_HEADER_V2
        } else {
            PROJECT_DOCUMENT_HEADER
        };
        format!("{header}\n{}\n", self.payload.portable_line())
    }
}

struct StoreAct<'a> {
    recorded_at_utc: u64,
    source: &'a str,
    creation_from_link: bool,
    origin_revision: Option<u64>,
    supersedes: Option<u64>,
    confirm: bool,
}

/// An append-only project chain.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectChain {
    revisions: Vec<ProjectRevision>,
}

impl ProjectChain {
    /// An empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stored revisions, oldest first.
    #[must_use]
    pub fn revisions(&self) -> &[ProjectRevision] {
        &self.revisions
    }

    /// Whether the chain has no revisions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.revisions.is_empty()
    }

    /// Whether a revision has no later correction.
    #[must_use]
    pub fn is_current(&self, revision_id: u64) -> bool {
        self.revisions
            .iter()
            .all(|revision| revision.supersedes != Some(revision_id))
    }

    /// Find a revision by local id.
    #[must_use]
    pub fn revision(&self, revision_id: u64) -> Option<&ProjectRevision> {
        self.revisions
            .iter()
            .find(|revision| revision.id == revision_id)
    }

    /// Keep a new project, or report that these payload bytes are already stored.
    pub fn keep(&mut self, draft: &ProjectDraft) -> Result<ProjectStore, ProjectError> {
        let (payload, from_link) = payload_from_draft(draft, "")?;
        self.store(
            payload,
            StoreAct {
                recorded_at_utc: draft.recorded_at_utc,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                creation_from_link: from_link,
                origin_revision: None,
                supersedes: None,
                confirm: true,
            },
        )
    }

    /// Import one portable document.
    ///
    /// `confirm` must be set before a new revision is appended. The same payload
    /// identity is already present and writes nothing either way. A taken origin
    /// id with different bytes collides until `confirm` appends a new local id.
    pub fn import(
        &mut self,
        document: &str,
        recorded_at_utc: u64,
        origin: Option<u64>,
        confirm: bool,
    ) -> Result<ProjectStore, ProjectError> {
        let (payload, from_link) = parse_document(document)?;
        // The same payload is already stored even when its parent row has
        // since been corrected. Identity is checked before that link.
        let identity_hex = payload.identity_hex();
        if let Some(existing) = self
            .revisions
            .iter()
            .find(|revision| revision.identity_hex() == identity_hex)
        {
            return Ok(ProjectStore::AlreadyPresent {
                revision_id: existing.id,
                identity_hex,
            });
        }
        let supersedes = self.supersedes_for_parent(&payload.parent_hex)?;
        self.store(
            payload,
            StoreAct {
                recorded_at_utc,
                source: JOURNAL_SOURCE_PLAYER_PROVIDED,
                creation_from_link: from_link,
                origin_revision: origin,
                supersedes,
                confirm,
            },
        )
    }

    /// Append a correction. The target bytes stay as they were.
    pub fn correct(
        &mut self,
        supersedes: u64,
        draft: &ProjectDraft,
    ) -> Result<ProjectStore, ProjectError> {
        let target = self
            .revision(supersedes)
            .ok_or(ProjectError::MissingRevision(supersedes))?;
        if !self.is_current(supersedes) {
            return Err(ProjectError::AlreadySuperseded(supersedes));
        }
        let parent = target.identity_hex();
        let (payload, from_link) = payload_from_draft(draft, &parent)?;
        self.store(
            payload,
            StoreAct {
                recorded_at_utc: draft.recorded_at_utc,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                creation_from_link: from_link,
                origin_revision: None,
                supersedes: Some(supersedes),
                confirm: true,
            },
        )
    }

    /// Preview one revision. `None` previews the latest revision.
    pub fn preview(
        &self,
        revision_id: Option<u64>,
        journal: &Journal,
        receipt: ReceiptCheck,
    ) -> Result<ResumePreview, ProjectError> {
        let revision = match revision_id {
            Some(id) => self.revision(id).ok_or(ProjectError::MissingRevision(id))?,
            None => self.revisions.last().ok_or(ProjectError::Empty)?,
        };
        Ok(preview_revision(self, revision, journal, receipt))
    }

    /// Serialize the local chain.
    #[must_use]
    pub fn to_text(&self) -> String {
        let header = if self
            .revisions
            .iter()
            .any(|revision| revision.payload.needs_route_version())
        {
            PROJECT_CHAIN_HEADER_V2
        } else {
            PROJECT_CHAIN_HEADER
        };
        let mut out = String::from(header);
        out.push('\n');
        for revision in &self.revisions {
            out.push_str(&revision.chain_line());
            out.push('\n');
        }
        out
    }

    /// Parse a local chain. A portable document is the wrong header.
    pub fn parse(text: &str) -> Result<Self, ProjectError> {
        if text.len() as u64 > MAX_PROJECT_FILE_BYTES {
            return Err(ProjectError::TooLarge);
        }
        let mut lines = text.lines();
        let route_version = match lines.next() {
            Some(PROJECT_CHAIN_HEADER) => false,
            Some(PROJECT_CHAIN_HEADER_V2) => true,
            _ => {
                return Err(ProjectError::InvalidFormat(
                    "unsupported project chain header".to_string(),
                ));
            }
        };
        let mut revisions = Vec::new();
        let mut previous_id = 0_u64;
        for (index, line) in lines.enumerate() {
            if line.is_empty() {
                return Err(ProjectError::InvalidFormat(
                    "chain line is empty".to_string(),
                ));
            }
            if revisions.len() == MAX_PROJECT_REVISIONS {
                return Err(ProjectError::Capacity);
            }
            let revision = ProjectRevision::parse_chain_line(line, index + 2)?;
            if !route_version && revision.payload.needs_route_version() {
                return Err(ProjectError::InvalidFormat(
                    "route content requires project chain version 2".into(),
                ));
            }
            if revision.id <= previous_id {
                return Err(ProjectError::InvalidFormat(format!(
                    "line {} revision id is not increasing",
                    index + 2
                )));
            }
            previous_id = revision.id;
            if let Some(target) = revision.supersedes {
                let known = revisions
                    .iter()
                    .any(|earlier: &ProjectRevision| earlier.id == target);
                let taken = revisions
                    .iter()
                    .any(|earlier| earlier.supersedes == Some(target));
                if !known || taken {
                    return Err(ProjectError::InvalidFormat(format!(
                        "line {} has an invalid supersedes link",
                        index + 2
                    )));
                }
            }
            revisions.push(revision);
        }
        Ok(Self { revisions })
    }

    fn store(&mut self, payload: Payload, act: StoreAct<'_>) -> Result<ProjectStore, ProjectError> {
        let identity_hex = payload.identity_hex();
        if let Some(existing) = self
            .revisions
            .iter()
            .find(|revision| revision.identity_hex() == identity_hex)
        {
            return Ok(ProjectStore::AlreadyPresent {
                revision_id: existing.id,
                identity_hex,
            });
        }
        if let Some(origin) = act.origin_revision
            && self.revision(origin).is_some()
            && !act.confirm
        {
            return Ok(ProjectStore::Collided {
                origin_revision: origin,
            });
        }
        if !act.confirm {
            return Ok(ProjectStore::NeedsConfirm);
        }
        if self.revisions.len() >= MAX_PROJECT_REVISIONS {
            return Err(ProjectError::Capacity);
        }
        let id = self
            .revisions
            .iter()
            .map(|revision| revision.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ProjectError::IdentifierExhausted)?;
        self.revisions.push(ProjectRevision {
            id,
            recorded_at_utc: act.recorded_at_utc,
            source: act.source.to_string(),
            payload,
            origin_revision: act.origin_revision,
            supersedes: act.supersedes,
            creation_from_link: act.creation_from_link,
        });
        Ok(ProjectStore::Appended {
            revision_id: id,
            identity_hex,
        })
    }

    fn supersedes_for_parent(&self, parent_hex: &str) -> Result<Option<u64>, ProjectError> {
        if parent_hex.is_empty() {
            return Ok(None);
        }
        let Some(target) = self
            .revisions
            .iter()
            .find(|revision| revision.identity_hex() == parent_hex)
        else {
            return Ok(None);
        };
        if !self.is_current(target.id) {
            return Err(ProjectError::AlreadySuperseded(target.id));
        }
        Ok(Some(target.id))
    }
}

impl Payload {
    fn needs_route_version(&self) -> bool {
        matches!(
            self.next,
            ProjectNext::OpenRoute { .. } | ProjectNext::RemixRoute
        ) || self.creation_num.as_deref().is_some_and(is_route_capsule)
    }
    fn identity_hex(&self) -> String {
        sha256_hex(&sha256(self.portable_line().as_bytes()))
    }

    fn portable_line(&self) -> String {
        join_fields(&[
            encode_field(&self.question),
            encode_field(&encode_next(&self.next)),
            encode_field(&encode_rooms(&self.rooms)),
            encode_field(&encode_evidence(&self.evidence)),
            encode_field(self.creation_num.as_deref().unwrap_or("")),
            encode_field(&self.parent_hex),
        ])
    }
}

impl ProjectRevision {
    fn chain_line(&self) -> String {
        let creation = match &self.payload.creation_num {
            Some(num) if self.creation_from_link => format!("link\n{num}"),
            Some(num) => format!("file\n{num}"),
            None => String::new(),
        };
        join_fields(&[
            self.id.to_string(),
            self.recorded_at_utc.to_string(),
            encode_field(&self.source),
            encode_field(&self.payload.question),
            encode_field(&encode_next(&self.payload.next)),
            encode_field(&encode_rooms(&self.payload.rooms)),
            encode_field(&encode_evidence(&self.payload.evidence)),
            encode_field(&creation),
            self.origin_revision
                .map(|id| id.to_string())
                .unwrap_or_default(),
            self.supersedes.map(|id| id.to_string()).unwrap_or_default(),
            encode_field(&self.payload.parent_hex),
        ])
    }

    fn parse_chain_line(line: &str, line_number: usize) -> Result<Self, ProjectError> {
        let fields = split_fields(line, CHAIN_FIELDS, line_number)?;
        let id = parse_u64(&fields[0], line_number, "revision id")?;
        let recorded_at_utc = parse_u64(&fields[1], line_number, "record time")?;
        if !is_project_source(&fields[2]) {
            return Err(ProjectError::InvalidSource);
        }
        let (creation_num, creation_from_link) = parse_stored_creation(&fields[7])?;
        let payload = Payload {
            question: validate_stored_question(&fields[3])?,
            next: decode_next(&fields[4])?,
            rooms: decode_rooms(&fields[5])?,
            evidence: decode_evidence(&fields[6])?,
            creation_num,
            parent_hex: parse_parent(&fields[10])?,
        };
        Ok(Self {
            id,
            recorded_at_utc,
            source: fields[2].clone(),
            payload,
            origin_revision: parse_optional_u64(&fields[8], line_number, "origin")?,
            supersedes: parse_optional_u64(&fields[9], line_number, "supersedes")?,
            creation_from_link,
        })
    }
}

fn payload_from_draft(
    draft: &ProjectDraft,
    parent_hex: &str,
) -> Result<(Payload, bool), ProjectError> {
    let (creation_num, creation_from_link) = match &draft.creation {
        Some(input) => {
            let (num, from_link) = canonical_creation(input)?;
            (Some(num), from_link)
        }
        None => (None, false),
    };
    let rooms = canonical_rooms(&draft.rooms)?;
    let next = canonical_next(&draft.next, &rooms, creation_num.as_deref())?;
    let payload = Payload {
        question: canonical_question(&draft.question)?,
        next,
        rooms,
        evidence: canonical_evidence(&draft.evidence)?,
        creation_num,
        parent_hex: parent_hex.to_string(),
    };
    Ok((payload, creation_from_link))
}

fn parse_document(text: &str) -> Result<(Payload, bool), ProjectError> {
    if text.len() as u64 > MAX_PROJECT_FILE_BYTES {
        return Err(ProjectError::TooLarge);
    }
    let mut lines = text.lines();
    let route_version = match lines.next() {
        Some(PROJECT_DOCUMENT_HEADER) => false,
        Some(PROJECT_DOCUMENT_HEADER_V2) => true,
        _ => {
            return Err(ProjectError::InvalidFormat(
                "unsupported project document header".to_string(),
            ));
        }
    };
    let Some(line) = lines.next() else {
        return Err(ProjectError::InvalidFormat(
            "document has no payload line".to_string(),
        ));
    };
    if line.is_empty() || lines.next().is_some() {
        return Err(ProjectError::InvalidFormat(
            "document must be one payload line".to_string(),
        ));
    }
    let fields = split_fields(line, PORTABLE_FIELDS, 2)?;
    let (creation_num, from_link) = if fields[4].is_empty() {
        (None, false)
    } else if fields[4].trim_start().starts_with("numinous://") {
        let (num, _) = canonical_creation(&fields[4])?;
        (Some(num), true)
    } else {
        let (num, from_link) = canonical_creation(&fields[4])?;
        (Some(num), from_link)
    };
    let rooms = decode_rooms(&fields[2])?;
    require_catalog_rooms(&rooms)?;
    let next = canonical_next(&decode_next(&fields[1])?, &rooms, creation_num.as_deref())?;
    let payload = Payload {
        question: canonical_question(&fields[0])?,
        next,
        rooms,
        evidence: decode_evidence(&fields[3])?,
        creation_num,
        parent_hex: parse_parent(&fields[5])?,
    };
    if !route_version && payload.needs_route_version() {
        return Err(ProjectError::InvalidFormat(
            "route content requires project document version 2".into(),
        ));
    }
    Ok((payload, from_link))
}

fn canonical_question(raw: &str) -> Result<String, ProjectError> {
    if raw.chars().any(is_forbidden_control) {
        return Err(ProjectError::InvalidQuestion(
            "control characters are refused".to_string(),
        ));
    }
    let question = raw.trim();
    if question.is_empty() {
        return Err(ProjectError::InvalidQuestion(
            "a question is required".to_string(),
        ));
    }
    if question.chars().count() > MAX_WORKSPACE_TEXT_CHARS {
        return Err(ProjectError::InvalidQuestion(format!(
            "a question is at most {MAX_WORKSPACE_TEXT_CHARS} characters"
        )));
    }
    Ok(question.to_string())
}

fn validate_stored_question(question: &str) -> Result<String, ProjectError> {
    if question.chars().any(is_forbidden_control)
        || question.is_empty()
        || question.chars().count() > MAX_WORKSPACE_TEXT_CHARS
        || question != question.trim()
    {
        return Err(ProjectError::InvalidFormat(
            "stored question is not valid".to_string(),
        ));
    }
    Ok(question.to_string())
}

fn canonical_rooms(rooms: &[String]) -> Result<Vec<String>, ProjectError> {
    if rooms.is_empty() || rooms.len() > MAX_PROJECT_ROOMS {
        return Err(ProjectError::InvalidRoom(format!(
            "a project names 1 to {MAX_PROJECT_ROOMS} rooms"
        )));
    }
    let mut canonical = Vec::with_capacity(rooms.len());
    for room in rooms {
        if room.chars().count() > MAX_ROOM_ID_CHARS {
            return Err(ProjectError::InvalidRoom(
                "room id exceeds 64 characters".to_string(),
            ));
        }
        if room.chars().any(is_forbidden_control) {
            return Err(ProjectError::InvalidRoom(
                "room id contains a control character".to_string(),
            ));
        }
        let resolved = canonical_room_id(room);
        if room_meta_by_id(resolved).is_none() {
            return Err(ProjectError::InvalidRoom(format!("unknown room {room}")));
        }
        if canonical.iter().any(|id: &String| id == resolved) {
            return Err(ProjectError::InvalidRoom(format!(
                "room {resolved} is repeated"
            )));
        }
        canonical.push(resolved.to_string());
    }
    Ok(canonical)
}

fn require_catalog_rooms(rooms: &[String]) -> Result<(), ProjectError> {
    for room in rooms {
        if room_meta_by_id(room).is_none() || canonical_room_id(room) != room {
            return Err(ProjectError::InvalidRoom(format!("unknown room {room}")));
        }
    }
    Ok(())
}

fn canonical_evidence(evidence: &[ProjectEvidence]) -> Result<Vec<ProjectEvidence>, ProjectError> {
    if evidence.len() > MAX_PROJECT_EVIDENCE {
        return Err(ProjectError::InvalidEvidence(format!(
            "a project names at most {MAX_PROJECT_EVIDENCE} evidence links"
        )));
    }
    Ok(evidence.to_vec())
}

fn canonical_next(
    next: &ProjectNext,
    rooms: &[String],
    creation_num: Option<&str>,
) -> Result<ProjectNext, ProjectError> {
    match next {
        ProjectNext::OpenCreation { capsule } => Ok(ProjectNext::OpenCreation {
            capsule: canonical_open_capsule(capsule)?,
        }),
        ProjectNext::ForkCreation => {
            if creation_num.is_none() || creation_num.is_some_and(is_route_capsule) {
                return Err(ProjectError::InvalidNext(
                    "fork_creation needs the project's Studio creation".to_string(),
                ));
            }
            Ok(ProjectNext::ForkCreation)
        }
        ProjectNext::OpenRoute { capsule } => Ok(ProjectNext::OpenRoute {
            capsule: canonical_route(capsule)?,
        }),
        ProjectNext::RemixRoute => {
            let Some(capsule) = creation_num.filter(|capsule| is_route_capsule(capsule)) else {
                return Err(ProjectError::InvalidNext(
                    "remix_route needs the project's route creation".into(),
                ));
            };
            canonical_route(capsule)?;
            Ok(ProjectNext::RemixRoute)
        }
        ProjectNext::PlayRoom { room, phase } => {
            let room = require_listed_room(room, rooms)?;
            let phase = match phase {
                Some(token) => Some(canonical_phase_token(token)?),
                None => None,
            };
            Ok(ProjectNext::PlayRoom { room, phase })
        }
        ProjectNext::StudyRoom { room } => Ok(ProjectNext::StudyRoom {
            room: require_listed_room(room, rooms)?,
        }),
    }
}

fn canonical_open_capsule(input: &str) -> Result<String, ProjectError> {
    if input.len() > MAX_SHARE_INPUT_BYTES {
        return Err(ProjectError::InvalidNext(
            "open_creation capsule exceeds the Studio byte cap".to_string(),
        ));
    }
    let creation = StudioCreation::from_capsule(input).map_err(ProjectError::InvalidNext)?;
    let trimmed = input.trim();
    if crate::studio::studio_experiment(trimmed).is_some()
        && !trimmed.contains('\n')
        && !trimmed.starts_with("NUMINOUS_")
    {
        return Ok(trimmed.to_string());
    }
    let num = creation.to_num_file();
    if num.len() > MAX_SHARE_INPUT_BYTES {
        return Err(ProjectError::InvalidNext(
            "open_creation capsule exceeds the Studio byte cap".to_string(),
        ));
    }
    Ok(num)
}

fn canonical_creation(input: &str) -> Result<(String, bool), ProjectError> {
    if is_route_capsule(input) {
        return canonical_route(input).map(|capsule| (capsule, false));
    }
    if input.len() > MAX_SHARE_INPUT_BYTES {
        return Err(ProjectError::InvalidCreation(
            "creation exceeds the Studio byte cap".to_string(),
        ));
    }
    let from_link = input.trim_start().starts_with("numinous://");
    let creation = StudioCreation::from_capsule(input).map_err(ProjectError::InvalidCreation)?;
    let num = creation.to_num_file();
    if num.len() > MAX_SHARE_INPUT_BYTES {
        return Err(ProjectError::InvalidCreation(
            "creation exceeds the Studio byte cap".to_string(),
        ));
    }
    Ok((num, from_link))
}

fn is_route_capsule(input: &str) -> bool {
    input.starts_with("NUMINOUS_ROUTE")
}

fn canonical_route(input: &str) -> Result<String, ProjectError> {
    RouteCreation::from_capsule(input)
        .map(|creation| creation.to_capsule())
        .map_err(|error| ProjectError::InvalidCreation(error.to_string()))
}

fn require_listed_room(room: &str, rooms: &[String]) -> Result<String, ProjectError> {
    if room.chars().count() > MAX_ROOM_ID_CHARS || room.chars().any(is_forbidden_control) {
        return Err(ProjectError::InvalidNext(
            "next room id is not valid".to_string(),
        ));
    }
    let resolved = canonical_room_id(room).to_string();
    if !rooms.iter().any(|id| id == &resolved) {
        return Err(ProjectError::InvalidNext(format!(
            "{resolved} is not one of the project's rooms"
        )));
    }
    Ok(resolved)
}

fn canonical_phase_token(token: &str) -> Result<String, ProjectError> {
    let phase = parse_phase(token)?;
    let text = format!("{phase}");
    let round_trip: f64 = text.parse().map_err(|_| {
        ProjectError::InvalidNext("play_room phase does not round-trip".to_string())
    })?;
    if round_trip == phase {
        return Ok(text);
    }
    let text = format!("{phase:?}");
    let round_trip: f64 = text.parse().map_err(|_| {
        ProjectError::InvalidNext("play_room phase does not round-trip".to_string())
    })?;
    if round_trip == phase {
        Ok(text)
    } else {
        Err(ProjectError::InvalidNext(
            "play_room phase does not round-trip".to_string(),
        ))
    }
}

fn parse_phase(token: &str) -> Result<f64, ProjectError> {
    if token.is_empty()
        || !token
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-'))
    {
        return Err(ProjectError::InvalidNext(
            "play_room phase must be a finite number in [0, 1)".to_string(),
        ));
    }
    let phase: f64 = token.parse().map_err(|_| {
        ProjectError::InvalidNext("play_room phase must be a finite number in [0, 1)".to_string())
    })?;
    if !phase.is_finite() || !(0.0..1.0).contains(&phase) {
        return Err(ProjectError::InvalidNext(
            "play_room phase must be a finite number in [0, 1)".to_string(),
        ));
    }
    Ok(phase)
}

fn preview_revision(
    chain: &ProjectChain,
    revision: &ProjectRevision,
    journal: &Journal,
    receipt: ReceiptCheck,
) -> ResumePreview {
    let rooms = revision
        .rooms()
        .iter()
        .map(|id| RoomFact {
            status: room_status(id),
            id: id.clone(),
        })
        .collect::<Vec<_>>();
    let evidence = revision
        .evidence()
        .iter()
        .map(|item| preview_evidence(item, journal, receipt))
        .collect::<Vec<_>>();
    let creation = preview_creation(revision);
    let next = preview_next(revision, &creation);
    let superseded_by = chain
        .revisions
        .iter()
        .find(|later| later.supersedes == Some(revision.id))
        .map(|later| later.id);
    let parent_resolved = revision.parent_hex().map(|parent| {
        chain
            .revisions
            .iter()
            .any(|earlier| earlier.identity_hex() == parent)
    });
    let rooms_ready = rooms.iter().all(|room| room.status == RoomStatus::Present);
    let evidence_ready = evidence.iter().all(|item| {
        matches!(
            item.status,
            EvidenceStatus::Present | EvidenceStatus::Missing | EvidenceStatus::Corrected
        )
    });
    let will_return = superseded_by.is_none()
        && rooms_ready
        && evidence_ready
        && creation.status != CreationStatus::Incompatible
        && matches!(next, NextPreview::Ready(_));
    ResumePreview {
        schema: PROJECT_RESUME_PREVIEW_SCHEMA,
        version: PROJECT_RESUME_PREVIEW_VERSION,
        revision_id: revision.id,
        question: revision.question().to_string(),
        interpreted: false,
        next,
        rooms,
        evidence,
        creation,
        will_return,
        not_applied: true,
        workspace_changed: false,
        journal_changed: false,
        superseded_by,
        parent_resolved,
    }
}

fn room_status(id: &str) -> RoomStatus {
    if !is_room_token(id) || canonical_room_id(id) != id {
        return RoomStatus::Incompatible;
    }
    if room_meta_by_id(id).is_none() {
        RoomStatus::Missing
    } else {
        RoomStatus::Present
    }
}

fn is_room_token(id: &str) -> bool {
    let length = id.chars().count();
    (1..=MAX_ROOM_ID_CHARS).contains(&length)
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-'))
}

fn preview_evidence(
    evidence: &ProjectEvidence,
    journal: &Journal,
    receipt: ReceiptCheck,
) -> EvidenceFact {
    let (kind, digest, hint, tool) = match *evidence {
        ProjectEvidence::Journal { digest, entry_id } => ("journal", digest, entry_id, None),
        ProjectEvidence::Receipt { digest, tool } => ("receipt", digest, None, tool),
    };
    let mut fact = resolve_evidence(kind, digest, hint, tool, journal);
    if kind == "receipt" {
        let disagrees = match receipt {
            ReceiptCheck::NotSupplied => false,
            ReceiptCheck::Disagreed => true,
            ReceiptCheck::Verified(supplied) => supplied != digest,
        };
        if disagrees {
            fact.status = EvidenceStatus::Incompatible;
            fact.entry_id = None;
            fact.superseded_by = None;
            fact.subject = None;
            fact.text = None;
        }
    }
    fact
}

fn resolve_evidence(
    kind: &'static str,
    digest: [u8; 32],
    hint: Option<u64>,
    tool: Option<EncounterTool>,
    journal: &Journal,
) -> EvidenceFact {
    let digest_hex = sha256_hex(&digest);
    let matched = journal
        .entries
        .iter()
        .find(|entry| evidence_matches(kind, digest, entry));
    let hint_entry = hint.and_then(|id| journal.entry(id));
    let hint_collides = hint_entry.is_some_and(|entry| !evidence_matches(kind, digest, entry));
    let same_bytes_elsewhere = hint_collides && matched.is_some();
    if hint_collides {
        return EvidenceFact {
            kind,
            digest_hex,
            status: EvidenceStatus::Collided,
            entry_id: None,
            superseded_by: None,
            subject: None,
            text: None,
            tool: tool.map(EncounterTool::name),
            same_bytes_elsewhere,
        };
    }
    let Some(entry) = matched else {
        return EvidenceFact {
            kind,
            digest_hex,
            status: EvidenceStatus::Missing,
            entry_id: None,
            superseded_by: None,
            subject: None,
            text: None,
            tool: tool.map(EncounterTool::name),
            same_bytes_elsewhere: false,
        };
    };
    let tool_name = tool.map(EncounterTool::name);
    if journal.is_current(entry.entry_id) {
        EvidenceFact {
            kind,
            digest_hex,
            status: EvidenceStatus::Present,
            entry_id: Some(entry.entry_id),
            superseded_by: None,
            subject: Some(entry.subject.clone()),
            text: Some(entry.text.clone()),
            tool: tool_name,
            same_bytes_elsewhere: false,
        }
    } else {
        EvidenceFact {
            kind,
            digest_hex,
            status: EvidenceStatus::Corrected,
            entry_id: Some(entry.entry_id),
            superseded_by: journal
                .superseding_entry(entry.entry_id)
                .map(|later| later.entry_id),
            subject: Some(entry.subject.clone()),
            text: Some(entry.text.clone()),
            tool: tool_name,
            same_bytes_elsewhere: false,
        }
    }
}

fn evidence_matches(kind: &str, digest: [u8; 32], entry: &JournalEntry) -> bool {
    if entry.identity_digest() == digest {
        return true;
    }
    kind == "receipt"
        && entry.subject == format!("{JOURNAL_SUBJECT_RECEIPT_PREFIX}{}", sha256_hex(&digest))
}

fn preview_creation(revision: &ProjectRevision) -> CreationFact {
    let Some(num) = revision.creation_num() else {
        return CreationFact {
            status: CreationStatus::Missing,
            kind: None,
            capsule: None,
            descends: None,
            lineage_was_not_in_the_link: false,
            period_text: None,
        };
    };
    if is_route_capsule(num) {
        return match RouteCreation::from_capsule(num) {
            Ok(route) if route.to_capsule() == num => CreationFact {
                status: CreationStatus::Present,
                kind: Some(CreationKind::Route),
                capsule: Some(num.to_string()),
                descends: route.parent_identity().map(|parent| sha256_hex(&parent)),
                lineage_was_not_in_the_link: false,
                period_text: None,
            },
            _ => CreationFact {
                status: CreationStatus::Incompatible,
                kind: None,
                capsule: None,
                descends: None,
                lineage_was_not_in_the_link: false,
                period_text: None,
            },
        };
    }
    let Ok(creation) = StudioCreation::from_capsule(num) else {
        return CreationFact {
            status: CreationStatus::Incompatible,
            kind: None,
            capsule: None,
            descends: None,
            lineage_was_not_in_the_link: revision.creation_from_link(),
            period_text: None,
        };
    };
    if creation.to_num_file() != num {
        return CreationFact {
            status: CreationStatus::Incompatible,
            kind: None,
            capsule: None,
            descends: None,
            lineage_was_not_in_the_link: revision.creation_from_link(),
            period_text: None,
        };
    }
    let period_text = match PathClosure::of(&creation) {
        PathClosure::Periodic(periodic) => Some(periodic.period_text),
        _ => None,
    };
    CreationFact {
        status: CreationStatus::Present,
        kind: Some(CreationKind::Studio),
        capsule: Some(num.to_string()),
        descends: creation.descends().map(str::to_string),
        lineage_was_not_in_the_link: revision.creation_from_link(),
        period_text,
    }
}

fn preview_next(revision: &ProjectRevision, creation: &CreationFact) -> NextPreview {
    match revision.next() {
        ProjectNext::OpenRoute { capsule } => route_next(capsule, "open"),
        ProjectNext::RemixRoute => match (creation.kind, creation.capsule.as_deref()) {
            (Some(CreationKind::Route), Some(capsule)) => route_next(capsule, "remix"),
            _ => incompatible("route_lab", "the project has no route creation to remix"),
        },
        ProjectNext::OpenCreation { capsule } => {
            if StudioCreation::from_capsule(capsule).is_ok() {
                NextPreview::Ready(ProjectCall {
                    tool: "open_creation",
                    arguments: vec![ProjectArgument {
                        name: "capsule",
                        value: ProjectArgumentValue::Text(capsule.clone()),
                    }],
                })
            } else {
                incompatible("open_creation", "capsule does not reopen")
            }
        }
        ProjectNext::ForkCreation => match creation.status {
            CreationStatus::Present => {
                let Ok(parsed) =
                    StudioCreation::from_capsule(revision.creation_num().unwrap_or(""))
                else {
                    return incompatible("fork_creation", "creation does not reopen");
                };
                NextPreview::Ready(ProjectCall {
                    tool: "fork_creation",
                    arguments: vec![ProjectArgument {
                        name: "parent",
                        value: ProjectArgumentValue::Text(parsed.to_link()),
                    }],
                })
            }
            _ => incompatible("fork_creation", "the project has no creation to fork"),
        },
        ProjectNext::PlayRoom { room, phase } => {
            if !revision.rooms().iter().any(|id| id == room)
                || room_status(room) != RoomStatus::Present
            {
                return incompatible("play_room", "room is not available");
            }
            if let Some(token) = phase
                && parse_phase(token).is_err()
            {
                return incompatible("play_room", "phase is not in [0, 1)");
            }
            let mut arguments = vec![ProjectArgument {
                name: "id",
                value: ProjectArgumentValue::Text(room.clone()),
            }];
            if let Some(token) = phase {
                arguments.push(ProjectArgument {
                    name: "t",
                    value: ProjectArgumentValue::Number(token.clone()),
                });
            }
            NextPreview::Ready(ProjectCall {
                tool: "play_room",
                arguments,
            })
        }
        ProjectNext::StudyRoom { room } => {
            if !revision.rooms().iter().any(|id| id == room)
                || room_status(room) != RoomStatus::Present
            {
                incompatible("study_room", "room is not available")
            } else {
                NextPreview::Ready(ProjectCall {
                    tool: "study_room",
                    arguments: vec![ProjectArgument {
                        name: "room",
                        value: ProjectArgumentValue::Text(room.clone()),
                    }],
                })
            }
        }
    }
}

fn route_next(capsule: &str, action: &str) -> NextPreview {
    if RouteCreation::from_capsule(capsule).is_err() {
        return incompatible("route_lab", "route capsule does not reopen");
    }
    NextPreview::Ready(ProjectCall {
        tool: "route_lab",
        arguments: vec![
            ProjectArgument {
                name: "capsule",
                value: ProjectArgumentValue::Text(capsule.into()),
            },
            ProjectArgument {
                name: "action",
                value: ProjectArgumentValue::Text(action.into()),
            },
        ],
    })
}

fn incompatible(tool: &str, reason: &str) -> NextPreview {
    NextPreview::Incompatible(IncompatibleNext {
        tool: tool.to_string(),
        reason: reason.to_string(),
    })
}

fn encode_next(next: &ProjectNext) -> String {
    match next {
        ProjectNext::OpenCreation { capsule } => format!("open_creation\n{capsule}"),
        ProjectNext::ForkCreation => "fork_creation".to_string(),
        ProjectNext::OpenRoute { capsule } => format!("open_route\n{capsule}"),
        ProjectNext::RemixRoute => "remix_route".to_string(),
        ProjectNext::PlayRoom { room, phase: None } => format!("play_room\n{room}"),
        ProjectNext::PlayRoom {
            room,
            phase: Some(phase),
        } => format!("play_room\n{room}\n{phase}"),
        ProjectNext::StudyRoom { room } => format!("study_room\n{room}"),
    }
}

fn decode_next(field: &str) -> Result<ProjectNext, ProjectError> {
    let (tool, rest) = field
        .split_once('\n')
        .map_or((field, None), |(tool, rest)| (tool, Some(rest)));
    match tool {
        "open_route" => {
            let capsule = rest
                .ok_or_else(|| ProjectError::InvalidFormat("open_route needs a capsule".into()))?;
            if capsule.is_empty() || capsule.len() > MAX_ROUTE_CAPSULE_BYTES {
                return Err(ProjectError::InvalidFormat(
                    "route capsule is empty or too large".into(),
                ));
            }
            if canonical_route(capsule)? != capsule {
                return Err(ProjectError::InvalidFormat(
                    "stored route next is not canonical".into(),
                ));
            }
            Ok(ProjectNext::OpenRoute {
                capsule: capsule.into(),
            })
        }
        "remix_route" => {
            if rest.is_some() {
                return Err(ProjectError::InvalidFormat(
                    "remix_route takes no stored argument".into(),
                ));
            }
            Ok(ProjectNext::RemixRoute)
        }
        "open_creation" => {
            let capsule = rest.ok_or_else(|| {
                ProjectError::InvalidFormat("open_creation needs a capsule".to_string())
            })?;
            if capsule.is_empty() || capsule.len() > MAX_SHARE_INPUT_BYTES {
                return Err(ProjectError::InvalidFormat(
                    "open_creation capsule is empty or too large".to_string(),
                ));
            }
            Ok(ProjectNext::OpenCreation {
                capsule: capsule.to_string(),
            })
        }
        "fork_creation" => {
            if rest.is_some_and(|rest| !rest.is_empty()) {
                return Err(ProjectError::InvalidFormat(
                    "fork_creation takes no stored argument".to_string(),
                ));
            }
            Ok(ProjectNext::ForkCreation)
        }
        "play_room" => decode_play_room(rest),
        "study_room" => {
            let room = rest.ok_or_else(|| {
                ProjectError::InvalidFormat("study_room needs a room".to_string())
            })?;
            if room.contains('\n') || !is_room_token(room) {
                return Err(ProjectError::InvalidFormat(
                    "study_room room is not valid".to_string(),
                ));
            }
            Ok(ProjectNext::StudyRoom {
                room: room.to_string(),
            })
        }
        _ => Err(ProjectError::InvalidFormat(
            "next tool is outside the closed set".to_string(),
        )),
    }
}

fn decode_play_room(rest: Option<&str>) -> Result<ProjectNext, ProjectError> {
    let rest =
        rest.ok_or_else(|| ProjectError::InvalidFormat("play_room needs a room".to_string()))?;
    let mut parts = rest.split('\n');
    let room = parts.next().unwrap_or("");
    let phase = parts.next();
    if parts.next().is_some() || !is_room_token(room) {
        return Err(ProjectError::InvalidFormat(
            "play_room room is not valid".to_string(),
        ));
    }
    let phase = match phase {
        None => None,
        Some(token) => Some(canonical_phase_token(token).map_err(|_| {
            ProjectError::InvalidFormat("play_room phase is not valid".to_string())
        })?),
    };
    Ok(ProjectNext::PlayRoom {
        room: room.to_string(),
        phase,
    })
}

fn encode_rooms(rooms: &[String]) -> String {
    rooms.join(",")
}

fn decode_rooms(field: &str) -> Result<Vec<String>, ProjectError> {
    if field.is_empty() {
        return Err(ProjectError::InvalidFormat(
            "a project names 1 to 4 rooms".to_string(),
        ));
    }
    let rooms = field.split(',').map(str::to_string).collect::<Vec<_>>();
    if rooms.is_empty() || rooms.len() > MAX_PROJECT_ROOMS {
        return Err(ProjectError::InvalidFormat(
            "a project names 1 to 4 rooms".to_string(),
        ));
    }
    for room in &rooms {
        if !is_room_token(room) {
            return Err(ProjectError::InvalidFormat(
                "stored room id is not valid".to_string(),
            ));
        }
    }
    if rooms
        .iter()
        .any(|room| rooms.iter().filter(|other| *other == room).count() > 1)
    {
        return Err(ProjectError::InvalidFormat(
            "stored room id is repeated".to_string(),
        ));
    }
    Ok(rooms)
}

fn encode_evidence(evidence: &[ProjectEvidence]) -> String {
    evidence
        .iter()
        .map(|item| match *item {
            ProjectEvidence::Journal {
                digest,
                entry_id: None,
            } => format!("journal:{}", sha256_hex(&digest)),
            ProjectEvidence::Journal {
                digest,
                entry_id: Some(id),
            } => format!("journal:{}@{id}", sha256_hex(&digest)),
            ProjectEvidence::Receipt { digest, tool: None } => {
                format!("receipt:{}", sha256_hex(&digest))
            }
            ProjectEvidence::Receipt {
                digest,
                tool: Some(tool),
            } => format!("receipt:{}@{}", sha256_hex(&digest), tool.name()),
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_evidence(field: &str) -> Result<Vec<ProjectEvidence>, ProjectError> {
    if field.is_empty() {
        return Ok(Vec::new());
    }
    let mut evidence = Vec::new();
    for item in field.split(',') {
        if evidence.len() == MAX_PROJECT_EVIDENCE {
            return Err(ProjectError::InvalidFormat(
                "too many evidence links".to_string(),
            ));
        }
        evidence.push(decode_evidence_item(item)?);
    }
    Ok(evidence)
}

fn decode_evidence_item(item: &str) -> Result<ProjectEvidence, ProjectError> {
    let invalid = || ProjectError::InvalidFormat("evidence link is not valid".to_string());
    let (kind, rest) = item.split_once(':').ok_or_else(invalid)?;
    let (hex_digest, suffix) = rest
        .split_once('@')
        .map_or((rest, None), |(hex_digest, suffix)| {
            (hex_digest, Some(suffix))
        });
    let digest = parse_hex_32(hex_digest).ok_or_else(invalid)?;
    match kind {
        "journal" => {
            let entry_id = match suffix {
                None => None,
                Some(token) => Some(token.parse::<u64>().map_err(|_| invalid())?),
            };
            Ok(ProjectEvidence::Journal { digest, entry_id })
        }
        "receipt" => {
            let tool = match suffix {
                None => None,
                Some(name) => Some(EncounterTool::from_name(name).ok_or_else(invalid)?),
            };
            Ok(ProjectEvidence::Receipt { digest, tool })
        }
        _ => Err(invalid()),
    }
}

fn parse_stored_creation(field: &str) -> Result<(Option<String>, bool), ProjectError> {
    if field.is_empty() {
        return Ok((None, false));
    }
    let (kind, num) = field.split_once('\n').ok_or_else(|| {
        ProjectError::InvalidFormat("stored creation needs a link or file prefix".to_string())
    })?;
    if num.is_empty() || num.len() > MAX_SHARE_INPUT_BYTES {
        return Err(ProjectError::InvalidFormat(
            "stored creation is empty or too large".to_string(),
        ));
    }
    if is_route_capsule(num) && (kind != "file" || canonical_route(num)? != num) {
        return Err(ProjectError::InvalidFormat(
            "stored route creation is not canonical file data".into(),
        ));
    }
    match kind {
        "link" => Ok((Some(num.to_string()), true)),
        "file" => Ok((Some(num.to_string()), false)),
        _ => Err(ProjectError::InvalidFormat(
            "stored creation prefix is not link or file".to_string(),
        )),
    }
}

fn parse_parent(text: &str) -> Result<String, ProjectError> {
    if text.is_empty() {
        return Ok(String::new());
    }
    if parse_hex_32(text).is_none() {
        return Err(ProjectError::InvalidFormat(
            "parent identity is not 64 lowercase hex characters".to_string(),
        ));
    }
    Ok(text.to_string())
}

fn parse_hex_32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0_u8; 32];
    let bytes = text.as_bytes();
    for index in 0..32 {
        let high = hex_value(bytes[index * 2])?;
        let low = hex_value(bytes[index * 2 + 1])?;
        out[index] = (high << 4) | low;
    }
    Some(out)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn is_project_source(source: &str) -> bool {
    source == JOURNAL_SOURCE_SELF_AUTHORED || source == JOURNAL_SOURCE_PLAYER_PROVIDED
}

fn is_forbidden_control(character: char) -> bool {
    matches!(character, '\0'..='\u{1f}' | '\u{7f}')
}

fn parse_u64(value: &str, line: usize, field: &str) -> Result<u64, ProjectError> {
    value
        .parse()
        .map_err(|_| ProjectError::InvalidFormat(format!("line {line} has an invalid {field}")))
}

fn parse_optional_u64(value: &str, line: usize, field: &str) -> Result<Option<u64>, ProjectError> {
    if value.is_empty() {
        Ok(None)
    } else {
        parse_u64(value, line, field).map(Some)
    }
}

fn join_fields(fields: &[String]) -> String {
    fields.join("\t")
}

fn split_fields(line: &str, count: usize, line_number: usize) -> Result<Vec<String>, ProjectError> {
    let raw = line.split('\t').collect::<Vec<_>>();
    if raw.len() != count {
        return Err(ProjectError::InvalidFormat(format!(
            "line {line_number} has {} fields, expected {count}",
            raw.len()
        )));
    }
    raw.into_iter().map(decode_field).collect()
}

fn encode_field(value: &str) -> String {
    let mut encoded = String::new();
    for character in value.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            '\t' => encoded.push_str("\\t"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            other => encoded.push(other),
        }
    }
    encoded
}

#[cfg(test)]
mod route_project_tests {
    use super::*;
    use crate::route_workbench::RouteWorkbench;

    fn route_draft(creation: &RouteCreation) -> ProjectDraft {
        ProjectDraft {
            recorded_at_utc: 7,
            question: "Can closing this road change the best delivery order?".into(),
            next: ProjectNext::OpenRoute {
                capsule: creation.to_capsule(),
            },
            rooms: vec!["route-lab".into()],
            evidence: Vec::new(),
            creation: Some(creation.to_capsule()),
        }
    }

    fn studio_draft() -> ProjectDraft {
        ProjectDraft {
            recorded_at_utc: 7,
            question: "Q".into(),
            next: ProjectNext::StudyRoom {
                room: "lissajous".into(),
            },
            rooms: vec!["lissajous".into()],
            evidence: Vec::new(),
            creation: Some("NUMINOUS_STUDIO 1\nexpr=t\nxmin=0\nxmax=1\na=1\n".into()),
        }
    }

    #[test]
    fn legacy_studio_document_bytes_and_identity_are_unchanged() {
        let expected_line = "Q\tstudy_room\\nlissajous\tlissajous\t\tNUMINOUS_STUDIO 1\\nexpr=t\\nxmin=0\\nxmax=1\\na=1\\n\t";
        let expected_document = format!("NUMINOUS_PROJECT 1\n{expected_line}\n");
        let expected_chain = "numinous-project-v1\n1\t7\tself-authored\tQ\tstudy_room\\nlissajous\tlissajous\t\tfile\\nNUMINOUS_STUDIO 1\\nexpr=t\\nxmin=0\\nxmax=1\\na=1\\n\t\t\t\n";
        let mut chain = ProjectChain::new();
        chain.keep(&studio_draft()).unwrap();
        let row = chain.revision(1).unwrap();
        assert_eq!(row.to_document(), expected_document);
        assert_eq!(
            row.identity_hex(),
            "f58dedb381dc8bcdae0fa510993c415086a21a5a3e94f15a9059bd7beb46e6a2"
        );
        assert_eq!(chain.to_text(), expected_chain);
        assert_eq!(
            ProjectChain::parse(expected_chain).unwrap().to_text(),
            expected_chain
        );
        let mut imported = ProjectChain::new();
        imported.import(&expected_document, 9, None, true).unwrap();
        assert_eq!(
            imported.revision(1).unwrap().identity_hex(),
            row.identity_hex()
        );
    }

    #[test]
    fn mixed_chain_preserves_old_rows_and_reopens_corrected_route() {
        let parent = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut chain = ProjectChain::new();
        chain.keep(&studio_draft()).unwrap();
        let old_row = chain.revision(1).unwrap().chain_line();
        let old_identity = chain.revision(1).unwrap().identity_hex();
        chain.keep(&route_draft(&parent)).unwrap();
        assert!(chain.to_text().starts_with(PROJECT_CHAIN_HEADER_V2));
        assert_eq!(chain.revision(1).unwrap().chain_line(), old_row);
        assert_eq!(chain.revision(1).unwrap().identity_hex(), old_identity);
        assert!(
            chain
                .revision(1)
                .unwrap()
                .to_document()
                .starts_with(PROJECT_DOCUMENT_HEADER)
        );
        assert!(
            chain
                .revision(2)
                .unwrap()
                .to_document()
                .starts_with(PROJECT_DOCUMENT_HEADER_V2)
        );
        let mut changed = parent.town().clone();
        changed.roads[3].road.cost = 5;
        let child = parent.remix(changed).unwrap();
        chain.correct(2, &route_draft(&child)).unwrap();
        let text = chain.to_text();
        let restored = ProjectChain::parse(&text).unwrap();
        assert_eq!(restored.to_text(), text);
        let preview = restored
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .unwrap();
        assert!(preview.will_return);
        assert_eq!(preview.creation.kind, Some(CreationKind::Route));
        assert_eq!(preview.creation.capsule, Some(child.to_capsule()));
        assert_eq!(preview.creation.descends, Some(parent.identity_hex()));
        assert!(preview.creation.period_text.is_none());
        assert_eq!(preview.parent_resolved, Some(true));
        assert_eq!(
            restored.revision(2).unwrap().creation_capsule(),
            Some(parent.to_capsule().as_str())
        );
        let NextPreview::Ready(call) = preview.next else {
            panic!("ready route door");
        };
        assert_eq!(call.tool, "route_lab");
        assert_eq!(call.arguments[0].name, "capsule");
        assert_eq!(
            call.arguments[0].value,
            ProjectArgumentValue::Text(child.to_capsule())
        );
        assert_eq!(call.arguments[1].name, "action");
        assert_eq!(
            call.arguments[1].value,
            ProjectArgumentValue::Text("open".into())
        );
        let reopened = RouteCreation::from_capsule(preview.creation.capsule.as_deref().unwrap())
            .unwrap()
            .open();
        assert_eq!(reopened.compare().unwrap().exact.tour.cost, 9);
    }

    #[test]
    fn route_import_keeps_infeasibility_and_explicit_remix_door() {
        let mut town = RouteWorkbench::first_town().town().clone();
        for road in &mut town.roads {
            road.open = false;
        }
        let creation = RouteCreation::new(town).unwrap();
        let mut draft = route_draft(&creation);
        draft.next = ProjectNext::RemixRoute;
        let mut chain = ProjectChain::new();
        chain.keep(&draft).unwrap();
        let document = chain.revision(1).unwrap().to_document();
        let mut imported = ProjectChain::new();
        assert_eq!(
            imported.import(&document, 9, None, false).unwrap(),
            ProjectStore::NeedsConfirm
        );
        assert!(imported.is_empty());
        imported.import(&document, 9, None, true).unwrap();
        let preview = imported
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .unwrap();
        assert!(preview.will_return);
        let NextPreview::Ready(call) = preview.next else {
            panic!("ready remix door");
        };
        assert_eq!(
            call.arguments[1].value,
            ProjectArgumentValue::Text("remix".into())
        );
        let capsule = preview.creation.capsule.unwrap();
        let reopened = RouteCreation::from_capsule(&capsule).unwrap().open();
        assert_eq!(reopened.town(), creation.town());
        assert!(reopened.compare().is_err());
    }

    #[test]
    fn route_tampering_wrong_kind_and_v1_masquerades_are_refused_atomically() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut chain = ProjectChain::new();
        chain.keep(&route_draft(&creation)).unwrap();
        let before = chain.to_text();
        let document = chain.revision(1).unwrap().to_document();
        let tampered = document.replace("road 1 3 3 open", "road 1 3 5 open");
        assert!(chain.import(&tampered, 9, None, true).is_err());
        assert_eq!(chain.to_text(), before);
        assert!(
            ProjectChain::parse(&before.replace("road 1 3 3 open", "road 1 3 5 open")).is_err()
        );
        assert!(
            ProjectChain::parse(&before.replacen(PROJECT_CHAIN_HEADER_V2, PROJECT_CHAIN_HEADER, 1))
                .is_err()
        );
        assert!(
            chain
                .import(
                    &document.replacen(PROJECT_DOCUMENT_HEADER_V2, PROJECT_DOCUMENT_HEADER, 1),
                    9,
                    None,
                    true
                )
                .is_err()
        );
        let mut wrong = route_draft(&creation);
        wrong.next = ProjectNext::ForkCreation;
        assert!(chain.keep(&wrong).is_err());
        wrong.creation = None;
        wrong.next = ProjectNext::RemixRoute;
        assert!(chain.keep(&wrong).is_err());
        wrong.creation = Some("x".repeat(MAX_ROUTE_CAPSULE_BYTES + 1));
        assert!(chain.keep(&wrong).is_err());
        assert_eq!(chain.to_text(), before);
    }

    #[test]
    fn version_two_reads_legacy_studio_without_changing_identity_or_provenance() {
        let mut authored = ProjectChain::new();
        authored.keep(&studio_draft()).unwrap();
        let original = authored.revision(1).unwrap();
        let identity = original.identity_hex();
        let document = original.to_document();
        let future_document =
            document.replacen(PROJECT_DOCUMENT_HEADER, PROJECT_DOCUMENT_HEADER_V2, 1);
        let future_chain =
            authored
                .to_text()
                .replacen(PROJECT_CHAIN_HEADER, PROJECT_CHAIN_HEADER_V2, 1);
        assert_eq!(
            ProjectChain::parse(&future_chain).unwrap().to_text(),
            authored.to_text()
        );
        let mut imported = ProjectChain::new();
        imported
            .import(&future_document, 23, Some(91), true)
            .unwrap();
        let row = imported.revision(1).unwrap();
        assert_eq!(row.identity_hex(), identity);
        assert_eq!(row.to_document(), document);
        assert_eq!(row.recorded_at_utc(), 23);
        assert_eq!(row.source(), JOURNAL_SOURCE_PLAYER_PROVIDED);
        assert_eq!(row.origin_revision(), Some(91));
        assert_eq!(original.source(), JOURNAL_SOURCE_SELF_AUTHORED);
        assert_eq!(original.recorded_at_utc(), 7);
    }

    #[test]
    fn route_next_and_embedded_creation_independently_require_version_two() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        for (next, stored) in [
            (
                ProjectNext::OpenRoute {
                    capsule: creation.to_capsule(),
                },
                None,
            ),
            (
                ProjectNext::StudyRoom {
                    room: "route-lab".into(),
                },
                Some(creation.to_capsule()),
            ),
        ] {
            let mut draft = route_draft(&creation);
            draft.next = next;
            draft.creation = stored;
            let mut chain = ProjectChain::new();
            chain.keep(&draft).unwrap();
            let document = chain.revision(1).unwrap().to_document();
            assert!(document.starts_with(PROJECT_DOCUMENT_HEADER_V2));
            assert!(chain.to_text().starts_with(PROJECT_CHAIN_HEADER_V2));
            let mut imported = ProjectChain::new();
            imported.import(&document, 11, None, true).unwrap();
            let preview = imported
                .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
                .unwrap();
            assert!(preview.will_return);
            assert_eq!(
                preview.creation.status,
                if draft.creation.is_some() {
                    CreationStatus::Present
                } else {
                    CreationStatus::Missing
                }
            );
            assert_eq!(
                preview.creation.kind,
                draft.creation.as_ref().map(|_| CreationKind::Route)
            );
            let NextPreview::Ready(call) = preview.next else {
                panic!("followable route or study door");
            };
            assert_eq!(
                call.tool,
                if draft.creation.is_some() {
                    "study_room"
                } else {
                    "route_lab"
                }
            );
        }
    }

    #[test]
    fn authored_route_inputs_canonicalize_but_stored_noncanonical_capsules_are_refused() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let canonical = creation.to_capsule();
        let noncanonical = canonical.replace("road 0 1 1 open", "road 1 0 1 open");
        let mut draft = route_draft(&creation);
        draft.creation = Some(noncanonical.clone());
        draft.next = ProjectNext::OpenRoute {
            capsule: noncanonical.clone(),
        };
        let mut chain = ProjectChain::new();
        chain.keep(&draft).unwrap();
        let text = chain.to_text();
        assert_eq!(
            chain.revision(1).unwrap().creation_capsule(),
            Some(canonical.as_str())
        );
        let line = chain.revision(1).unwrap().chain_line();
        for field in [4, 7] {
            let mut fields = split_fields(&line, CHAIN_FIELDS, 2).unwrap();
            fields[field] = fields[field].replace("road 0 1 1 open", "road 1 0 1 open");
            let altered = fields
                .iter()
                .map(|field| encode_field(field))
                .collect::<Vec<_>>()
                .join("\t");
            assert!(
                ProjectChain::parse(&format!("{PROJECT_CHAIN_HEADER_V2}\n{altered}\n")).is_err()
            );
        }
        assert!(
            ProjectChain::parse(&text.replacen(
                "file\\nNUMINOUS_ROUTE",
                "link\\nNUMINOUS_ROUTE",
                1
            ))
            .is_err()
        );
        assert_eq!(
            chain.keep(&route_draft(&creation)).unwrap(),
            ProjectStore::AlreadyPresent {
                revision_id: 1,
                identity_hex: chain.revision(1).unwrap().identity_hex()
            }
        );
        assert_eq!(chain.to_text(), text);
    }

    #[test]
    fn imported_route_corrections_resolve_only_current_matching_project_parents() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut original = ProjectChain::new();
        original.keep(&route_draft(&creation)).unwrap();
        let base_document = original.revision(1).unwrap().to_document();
        let mut changed = route_draft(&creation);
        changed.question = "Does the same network answer this new question?".into();
        original.correct(1, &changed).unwrap();
        let correction_document = original.revision(2).unwrap().to_document();
        let mut orphan = ProjectChain::new();
        orphan.import(&correction_document, 31, None, true).unwrap();
        let preview = orphan
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .unwrap();
        assert_eq!(preview.parent_resolved, Some(false));
        assert!(preview.will_return);
        let mut target = ProjectChain::new();
        target.import(&base_document, 32, None, true).unwrap();
        target.import(&correction_document, 33, None, true).unwrap();
        assert!(!target.is_current(1));
        assert_eq!(target.revision(2).unwrap().supersedes(), Some(1));
        let mut alternative = route_draft(&creation);
        alternative.question = "A different correction of the same original question?".into();
        let mut sibling = ProjectChain::new();
        sibling.import(&base_document, 34, None, true).unwrap();
        sibling.correct(1, &alternative).unwrap();
        let before = target.to_text();
        assert_eq!(
            target.import(&sibling.revision(2).unwrap().to_document(), 35, None, true),
            Err(ProjectError::AlreadySuperseded(1))
        );
        assert_eq!(target.to_text(), before);
    }

    #[test]
    fn route_projects_refuse_exhausted_storage_and_malformed_chain_links() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut chain = ProjectChain::new();
        chain.keep(&route_draft(&creation)).unwrap();
        let row = chain.revision(1).unwrap().chain_line();
        let format_row = |id: u64, source: &str, supersedes: &str| {
            let mut fields = split_fields(&row, CHAIN_FIELDS, 2).unwrap();
            fields[0] = id.to_string();
            fields[2] = source.into();
            fields[9] = supersedes.into();
            fields
                .iter()
                .map(|field| encode_field(field))
                .collect::<Vec<_>>()
                .join("\t")
        };
        for invalid in [
            format!("{PROJECT_CHAIN_HEADER_V2}\n\n"),
            format!("{PROJECT_CHAIN_HEADER_V2}\n{row}\n{row}\n"),
            format!(
                "{PROJECT_CHAIN_HEADER_V2}\n{}\n",
                format_row(1, "third-party", "")
            ),
            format!(
                "{PROJECT_CHAIN_HEADER_V2}\n{}\n",
                format_row(1, JOURNAL_SOURCE_SELF_AUTHORED, "99")
            ),
            format!(
                "{PROJECT_CHAIN_HEADER_V2}\n{row}\n{}\n{}\n",
                format_row(2, JOURNAL_SOURCE_SELF_AUTHORED, "1"),
                format_row(3, JOURNAL_SOURCE_SELF_AUTHORED, "1")
            ),
        ] {
            assert!(ProjectChain::parse(&invalid).is_err());
        }
        let max_id = format!(
            "{PROJECT_CHAIN_HEADER_V2}\n{}\n",
            format_row(u64::MAX, JOURNAL_SOURCE_SELF_AUTHORED, "")
        );
        let mut exhausted = ProjectChain::parse(&max_id).unwrap();
        let before = exhausted.to_text();
        let mut new = route_draft(&creation);
        new.question = "Another project?".into();
        assert_eq!(exhausted.keep(&new), Err(ProjectError::IdentifierExhausted));
        assert_eq!(exhausted.to_text(), before);
        let over_capacity = format!(
            "{PROJECT_CHAIN_HEADER_V2}\n{}",
            (1..=MAX_PROJECT_REVISIONS + 1)
                .map(|id| format!(
                    "{}\n",
                    format_row(id as u64, JOURNAL_SOURCE_SELF_AUTHORED, "")
                ))
                .collect::<String>()
        );
        assert_eq!(
            ProjectChain::parse(&over_capacity),
            Err(ProjectError::Capacity)
        );
        for index in 1..MAX_PROJECT_REVISIONS {
            new.question = format!("Question {index}?");
            chain.keep(&new).unwrap();
        }
        let before = chain.to_text();
        new.question = "Question beyond the limit?".into();
        assert_eq!(chain.keep(&new), Err(ProjectError::Capacity));
        assert_eq!(chain.to_text(), before);
        assert_eq!(
            chain.correct(99, &new),
            Err(ProjectError::MissingRevision(99))
        );
        assert_eq!(
            chain.preview(Some(99), &Journal::new(), ReceiptCheck::NotSupplied),
            Err(ProjectError::MissingRevision(99))
        );
        assert_eq!(
            ProjectChain::parse(&"x".repeat(MAX_PROJECT_FILE_BYTES as usize + 1)),
            Err(ProjectError::TooLarge)
        );
    }

    #[test]
    fn route_project_admission_refuses_ambiguous_questions_rooms_and_phases_atomically() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut chain = ProjectChain::new();
        chain.keep(&route_draft(&creation)).unwrap();
        let before = chain.to_text();
        for question in [
            " ".into(),
            "q".repeat(MAX_WORKSPACE_TEXT_CHARS + 1),
            "bad\0question".into(),
        ] {
            let mut draft = route_draft(&creation);
            draft.question = question;
            let error = chain.keep(&draft).unwrap_err();
            assert!(matches!(error, ProjectError::InvalidQuestion(_)));
            assert!(error.to_string().starts_with("invalid project question:"));
        }
        for rooms in [
            vec![],
            vec!["route-lab".into(); MAX_PROJECT_ROOMS + 1],
            vec!["route-lab".into(), "route-lab".into()],
            vec!["x".repeat(MAX_ROOM_ID_CHARS + 1)],
            vec!["route\0lab".into()],
            vec!["absent-room".into()],
        ] {
            let mut draft = route_draft(&creation);
            draft.rooms = rooms;
            let error = chain.keep(&draft).unwrap_err();
            assert!(matches!(error, ProjectError::InvalidRoom(_)));
            assert!(error.to_string().starts_with("invalid project room:"));
        }
        for room in [
            "lissajous".into(),
            "x".repeat(MAX_ROOM_ID_CHARS + 1),
            "route\0lab".into(),
        ] {
            let mut draft = route_draft(&creation);
            draft.next = ProjectNext::StudyRoom { room };
            let error = chain.keep(&draft).unwrap_err();
            assert!(matches!(error, ProjectError::InvalidNext(_)));
            assert!(error.to_string().starts_with("invalid project next:"));
        }
        for phase in ["", "nan", "1.0", "-0.1", "1e309", "1e", "0.2.3"] {
            let mut draft = route_draft(&creation);
            draft.next = ProjectNext::PlayRoom {
                room: "route-lab".into(),
                phase: Some(phase.into()),
            };
            assert!(matches!(
                chain.keep(&draft),
                Err(ProjectError::InvalidNext(_))
            ));
        }
        let mut valid = route_draft(&creation);
        valid.next = ProjectNext::PlayRoom {
            room: "route-lab".into(),
            phase: Some("5e-1".into()),
        };
        assert_eq!(chain.to_text(), before);
        chain.keep(&valid).unwrap();
        let reopened = ProjectChain::parse(&chain.to_text()).unwrap();
        let preview = reopened
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .unwrap();
        let NextPreview::Ready(call) = preview.next else {
            panic!("ready room playback");
        };
        assert_eq!(
            call.arguments[1].value,
            ProjectArgumentValue::Number("0.5".into())
        );
    }

    #[test]
    fn portable_route_projects_refuse_incomplete_or_conflicting_documents_without_writing() {
        let creation = RouteCreation::new(RouteWorkbench::first_town().town().clone()).unwrap();
        let mut chain = ProjectChain::new();
        chain.keep(&route_draft(&creation)).unwrap();
        let before = chain.to_text();
        let document = chain.revision(1).unwrap().to_document();
        for invalid in [
            format!("{PROJECT_DOCUMENT_HEADER_V2}\n"),
            format!("{PROJECT_DOCUMENT_HEADER_V2}\n\n"),
            document.clone() + "additional payload\n",
            format!("{PROJECT_DOCUMENT_HEADER_V2}\nopen_route\n"),
            "x".repeat(MAX_PROJECT_FILE_BYTES as usize + 1),
        ] {
            let error = chain.import(&invalid, 42, None, true).unwrap_err();
            assert!(matches!(
                error,
                ProjectError::InvalidFormat(_) | ProjectError::TooLarge
            ));
            assert_eq!(chain.to_text(), before);
        }
        let payload_line = document.lines().nth(1).unwrap();
        let fields = split_fields(payload_line, PORTABLE_FIELDS, 2).unwrap();
        let document_with = |index: usize, replacement: &str| {
            let mut altered = fields.clone();
            altered[index] = replacement.into();
            format!(
                "{PROJECT_DOCUMENT_HEADER_V2}\n{}\n",
                altered
                    .iter()
                    .map(|field| encode_field(field))
                    .collect::<Vec<_>>()
                    .join("\t")
            )
        };
        for invalid in [
            document_with(1, "open_route"),
            document_with(1, "open_route\n"),
            document_with(1, "open_route\nnot-a-capsule"),
            document_with(1, "remix_route\nignored"),
            document_with(
                1,
                &format!("open_route\n{}", "x".repeat(MAX_ROUTE_CAPSULE_BYTES + 1)),
            ),
            document_with(2, "absent-room"),
            document_with(5, "invalid-parent"),
        ] {
            assert!(chain.import(&invalid, 42, None, true).is_err());
            assert_eq!(chain.to_text(), before);
        }
        let mut missing = fields.clone();
        missing[1] = "remix_route".into();
        missing[4].clear();
        let invalid = format!(
            "{PROJECT_DOCUMENT_HEADER_V2}\n{}\n",
            missing
                .iter()
                .map(|field| encode_field(field))
                .collect::<Vec<_>>()
                .join("\t")
        );
        assert!(matches!(
            chain.import(&invalid, 42, None, true),
            Err(ProjectError::InvalidNext(_))
        ));
        let mut unsupported = route_draft(&creation);
        unsupported.next = ProjectNext::RemixRoute;
        unsupported.creation = studio_draft().creation;
        assert!(matches!(
            chain.keep(&unsupported),
            Err(ProjectError::InvalidNext(_))
        ));
        assert_eq!(chain.to_text(), before);
    }
}

fn decode_field(value: &str) -> Result<String, ProjectError> {
    let mut decoded = String::new();
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => decoded.push('\\'),
            Some('t') => decoded.push('\t'),
            Some('n') => decoded.push('\n'),
            Some('r') => decoded.push('\r'),
            _ => {
                return Err(ProjectError::InvalidFormat(
                    "unknown field escape".to_string(),
                ));
            }
        }
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::{
        CreationStatus, EvidenceStatus, NextPreview, PROJECT_DOCUMENT_HEADER, ProjectArgumentValue,
        ProjectChain, ProjectDraft, ProjectError, ProjectEvidence, ProjectNext, ProjectStore,
        ReceiptCheck, RoomStatus,
    };
    use crate::journal::{JOURNAL_SOURCE_SELF_AUTHORED, Journal, JournalRecord};
    use crate::path_closure::PathClosure;
    use crate::studio::StudioCreation;

    fn draft(question: &str, next: ProjectNext, creation: Option<&str>) -> ProjectDraft {
        ProjectDraft {
            recorded_at_utc: 1_700_000_000,
            question: question.to_string(),
            next,
            rooms: vec!["lissajous".to_string()],
            evidence: Vec::new(),
            creation: creation.map(str::to_string),
        }
    }

    fn fixture() -> ProjectDraft {
        draft(
            "What period do these two oscillators share?",
            ProjectNext::OpenCreation {
                capsule: "closing-voices".to_string(),
            },
            Some("full-return"),
        )
    }

    #[test]
    fn full_return_reopens_with_period_12_and_the_question_stays_data() {
        let mut chain = ProjectChain::new();
        let stored = chain.keep(&fixture()).expect("keep");
        let ProjectStore::Appended { revision_id, .. } = stored else {
            panic!("new payload should append");
        };
        let text = chain.to_text();
        let restored = ProjectChain::parse(&text).expect("chain roundtrip");
        assert_eq!(restored.to_text(), text);
        let revision = restored.revision(revision_id).expect("revision");
        let num = revision.creation_num().expect("creation");
        assert_eq!(
            num,
            StudioCreation::from_capsule("full-return")
                .expect("id")
                .to_num_file()
        );
        let PathClosure::Periodic(periodic) =
            PathClosure::of(&StudioCreation::from_capsule(num).expect("reopen"))
        else {
            panic!("full-return is a periodic path");
        };
        assert_eq!(periodic.period_text, "12");
        let preview = restored
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(
            preview.question,
            "What period do these two oscillators share?"
        );
        assert!(!preview.interpreted);
        assert!(preview.will_return);
        assert!(preview.not_applied);
        assert!(!preview.workspace_changed);
        assert!(!preview.journal_changed);
        assert_eq!(preview.creation.status, CreationStatus::Present);
        assert_eq!(preview.creation.period_text.as_deref(), Some("12"));
        assert_eq!(preview.rooms[0].status, RoomStatus::Present);
        let NextPreview::Ready(call) = &preview.next else {
            panic!("next call should be ready");
        };
        assert_eq!(call.tool, "open_creation");
        assert_eq!(call.arguments[0].name, "capsule");
        assert_eq!(
            call.arguments[0].value,
            ProjectArgumentValue::Text("closing-voices".to_string())
        );
        assert!(!format!("{preview:?}").contains("learned"));
        let again = chain.keep(&fixture()).expect("second keep");
        assert!(matches!(again, ProjectStore::AlreadyPresent { .. }));
        assert_eq!(chain.revisions().len(), 1);
    }

    #[test]
    fn a_question_that_looks_like_a_tool_call_is_not_dispatched() {
        let mut chain = ProjectChain::new();
        let question = r#"{"tool":"play_room","arguments":{"id":"lissajous"}}"#;
        chain
            .keep(&draft(
                question,
                ProjectNext::StudyRoom {
                    room: "lissajous".to_string(),
                },
                None,
            ))
            .expect("keep");
        let preview = chain
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(preview.question, question);
        let NextPreview::Ready(call) = preview.next else {
            panic!("study_room should be ready");
        };
        assert_eq!(call.tool, "study_room");
        assert_eq!(call.arguments[0].name, "room");
    }

    #[test]
    fn bad_header_extra_field_control_character_and_hostile_capsule_are_refused() {
        let mut chain = ProjectChain::new();
        assert!(ProjectChain::parse("NUMINOUS_PROJECT 2\n").is_err());
        assert!(ProjectChain::parse("NUMINOUS_STUDIO 7\nkind=program\n").is_err());
        assert!(
            chain
                .import(
                    r#"{"schema":"numinous.portable-evidence-capsule","version":1}"#,
                    30,
                    None,
                    true,
                )
                .is_err()
        );
        let document = format!("{PROJECT_DOCUMENT_HEADER}\na\tb\tc\td\te\tf\textra\n");
        assert!(matches!(
            chain.import(&document, 30, None, true),
            Err(ProjectError::InvalidFormat(_))
        ));
        let mut bad = fixture();
        bad.question = "hello\nthere".to_string();
        assert!(matches!(
            chain.keep(&bad),
            Err(ProjectError::InvalidQuestion(_))
        ));
        bad.question = "a question".to_string();
        bad.creation = Some("../secret.num".to_string());
        assert!(matches!(
            chain.keep(&bad),
            Err(ProjectError::InvalidCreation(_))
        ));
        bad.creation = Some("full-return".to_string());
        bad.next = ProjectNext::OpenCreation {
            capsule: r"C:\secret.num".to_string(),
        };
        assert!(matches!(
            chain.keep(&bad),
            Err(ProjectError::InvalidNext(_))
        ));
        assert!(chain.is_empty());
    }

    #[test]
    fn correction_keeps_the_target_and_a_colliding_origin_does_not_replace_it() {
        let mut chain = ProjectChain::new();
        chain.keep(&fixture()).expect("keep");
        let original = chain.to_text();
        let identity = chain.revision(1).expect("first").identity_hex();
        let corrected = draft(
            "A different question about the same oscillators.",
            ProjectNext::StudyRoom {
                room: "lissajous".to_string(),
            },
            Some("full-return"),
        );
        chain.correct(1, &corrected).expect("correct");
        assert_eq!(
            chain.revision(1).expect("target").question(),
            fixture().question
        );
        assert!(original.lines().nth(1).unwrap().starts_with("1\t"));
        assert_eq!(chain.revision(2).expect("correction").supersedes(), Some(1));
        assert_eq!(
            chain.revision(2).expect("correction").parent_hex(),
            Some(identity.as_str())
        );
        assert!(!chain.is_current(1));
        assert!(matches!(
            chain.correct(1, &corrected),
            Err(ProjectError::AlreadySuperseded(1))
        ));

        let document = chain.revision(2).expect("correction").to_document();
        let collided = chain.import(&document, 30, Some(1), false).expect("hint");
        assert!(matches!(
            collided,
            ProjectStore::AlreadyPresent { revision_id: 2, .. }
        ));
        let other = draft(
            "Another question entirely.",
            ProjectNext::StudyRoom {
                room: "lissajous".to_string(),
            },
            None,
        );
        let mut fresh = ProjectChain::new();
        fresh.keep(&fixture()).expect("keep");
        let portable = {
            let mut held = ProjectChain::new();
            held.keep(&other).expect("other");
            held.revision(1).expect("row").to_document()
        };
        let pending = fresh
            .import(&portable, 30, Some(1), false)
            .expect("collide");
        assert_eq!(pending, ProjectStore::Collided { origin_revision: 1 });
        assert_eq!(fresh.revisions().len(), 1);
        assert_eq!(
            fresh.revision(1).expect("row").question(),
            fixture().question
        );
        let appended = fresh.import(&portable, 30, Some(1), true).expect("confirm");
        assert!(matches!(
            appended,
            ProjectStore::Appended { revision_id: 2, .. }
        ));
        assert_eq!(
            fresh.revision(1).expect("row").question(),
            fixture().question
        );
        assert_eq!(fresh.revision(2).expect("new").origin_revision(), Some(1));
        let mut pending_chain = ProjectChain::new();
        pending_chain
            .keep(&draft(
                "Needs an explicit confirm.",
                ProjectNext::StudyRoom {
                    room: "lissajous".to_string(),
                },
                None,
            ))
            .expect("source");
        let needs_confirm = fresh
            .import(
                &pending_chain.revision(1).expect("row").to_document(),
                30,
                None,
                false,
            )
            .expect("hold");
        assert_eq!(needs_confirm, ProjectStore::NeedsConfirm);
        assert_eq!(fresh.revisions().len(), 2);
    }

    #[test]
    fn journal_text_is_not_copied_and_erasure_of_either_store_leaves_the_other() {
        let mut journal = Journal::new();
        let entry_id = journal
            .record(JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 10,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "lissajous",
                text: "violet-quartz-token",
                affect: None,
            })
            .expect("record");
        let entry = journal.entry(entry_id).expect("entry").clone();
        let mut kept = fixture();
        kept.evidence.push(ProjectEvidence::Journal {
            digest: entry.identity_digest(),
            entry_id: Some(entry_id),
        });
        let mut chain = ProjectChain::new();
        chain.keep(&kept).expect("keep");
        let stored = chain.to_text();
        assert!(!stored.contains("violet-quartz-token"));
        let preview = chain
            .preview(None, &journal, ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(preview.evidence[0].status, EvidenceStatus::Present);
        assert_eq!(
            preview.evidence[0].text.as_deref(),
            Some("violet-quartz-token")
        );
        journal.erase();
        let after = chain
            .preview(None, &journal, ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(after.evidence[0].status, EvidenceStatus::Missing);
        assert!(after.evidence[0].text.is_none());
        assert!(chain.to_text().contains("What period"));
    }

    #[test]
    fn alias_link_flag_and_phase_round_trip_without_becoming_identity() {
        let mut chain = ProjectChain::new();
        let mut aliased = fixture();
        aliased.rooms = vec!["kepler-areas".to_string()];
        aliased.next = ProjectNext::StudyRoom {
            room: "kepler-areas".to_string(),
        };
        aliased.creation = None;
        chain.keep(&aliased).expect("alias");
        assert_eq!(
            chain.revision(1).expect("row").rooms(),
            &["kepler-laws".to_string()]
        );
        let preview = chain
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(preview.rooms[0].id, "kepler-laws");
        assert_eq!(preview.rooms[0].status, RoomStatus::Present);
        let NextPreview::Ready(call) = preview.next else {
            panic!("study room should be ready");
        };
        assert_eq!(call.tool, "study_room");
        assert_eq!(
            call.arguments[0].value,
            ProjectArgumentValue::Text("kepler-laws".to_string())
        );

        let creation = StudioCreation::from_capsule("full-return").expect("creation");
        let mut linked = fixture();
        linked.creation = Some(creation.to_link());
        let mut fresh = ProjectChain::new();
        fresh.keep(&linked).expect("link");
        let stored = fresh.revision(1).expect("row");
        assert!(stored.creation_from_link());
        let num = stored.creation_num().expect("num");
        assert!(num.starts_with("NUMINOUS_STUDIO"));
        let parsed = ProjectChain::parse(&fresh.to_text()).expect("chain");
        assert!(parsed.revision(1).expect("row").creation_from_link());
        assert_eq!(parsed.revision(1).expect("row").creation_num(), Some(num));
        assert!(
            !parsed
                .revision(1)
                .expect("row")
                .to_document()
                .contains("link\n")
        );
        let mut same_bytes = fixture();
        same_bytes.creation = Some(num.to_string());
        let repeated = fresh.keep(&same_bytes).expect("same creation bytes");
        assert!(matches!(
            repeated,
            ProjectStore::AlreadyPresent { revision_id: 1, .. }
        ));
        assert!(fresh.revision(1).expect("row").creation_from_link());
        assert_eq!(fresh.revisions().len(), 1);

        let mut phased = fixture();
        phased.next = ProjectNext::PlayRoom {
            room: "lissajous".to_string(),
            phase: Some("0.250".to_string()),
        };
        let mut played = ProjectChain::new();
        played.keep(&phased).expect("phase");
        let preview = played
            .preview(None, &Journal::new(), ReceiptCheck::NotSupplied)
            .expect("preview");
        let NextPreview::Ready(call) = preview.next else {
            panic!("play room should be ready");
        };
        assert_eq!(call.tool, "play_room");
        assert_eq!(call.arguments[0].name, "id");
        assert_eq!(
            call.arguments[1],
            crate::ProjectArgument {
                name: "t",
                value: ProjectArgumentValue::Number("0.25".to_string()),
            }
        );
    }

    #[test]
    fn evidence_collision_correction_and_receipt_disagreement_withhold_the_wrong_text() {
        use crate::encounter::EncounterTool;

        let mut journal = Journal::new();
        let cited = journal
            .record(JournalRecord {
                recorded_at_utc: 4,
                event_at_utc: 3,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "lissajous",
                text: "cited-quartz-token",
                affect: None,
            })
            .expect("cited");
        let other = journal
            .record(JournalRecord {
                recorded_at_utc: 5,
                event_at_utc: 4,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "lissajous",
                text: "other-quartz-token",
                affect: None,
            })
            .expect("other");
        let cited_entry = journal.entry(cited).expect("cited entry").clone();
        let other_digest = journal.entry(other).expect("other entry").identity_digest();
        let mut collided = fixture();
        collided.evidence.push(ProjectEvidence::Journal {
            digest: other_digest,
            entry_id: Some(cited),
        });
        let mut chain = ProjectChain::new();
        chain.keep(&collided).expect("keep");
        let preview = chain
            .preview(None, &journal, ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(preview.evidence[0].status, EvidenceStatus::Collided);
        assert!(preview.evidence[0].text.is_none());
        assert!(preview.evidence[0].same_bytes_elsewhere);
        assert!(!preview.will_return);
        assert!(!chain.to_text().contains("cited-quartz-token"));
        assert!(!chain.to_text().contains("other-quartz-token"));

        let correction = journal
            .correct(
                6,
                None,
                JOURNAL_SOURCE_SELF_AUTHORED,
                cited,
                "replacement-quartz-token",
                None,
            )
            .expect("correct");
        let mut corrected = fixture();
        corrected.evidence.push(ProjectEvidence::Journal {
            digest: cited_entry.identity_digest(),
            entry_id: Some(cited),
        });
        let mut corrected_chain = ProjectChain::new();
        corrected_chain.keep(&corrected).expect("keep");
        let preview = corrected_chain
            .preview(None, &journal, ReceiptCheck::NotSupplied)
            .expect("preview");
        assert_eq!(preview.evidence[0].status, EvidenceStatus::Corrected);
        assert_eq!(
            preview.evidence[0].text.as_deref(),
            Some("cited-quartz-token")
        );
        assert_eq!(preview.evidence[0].superseded_by, Some(correction));
        assert!(!corrected_chain.to_text().contains("cited-quartz-token"));
        assert!(
            !corrected_chain
                .to_text()
                .contains("replacement-quartz-token")
        );

        let digest = [0xab; 32];
        let digest_hex = crate::sha256::hex(&digest);
        journal
            .record(JournalRecord {
                recorded_at_utc: 7,
                event_at_utc: 6,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: &format!("receipt:{digest_hex}"),
                text: "receipt-body-token",
                affect: None,
            })
            .expect("receipt");
        let mut receipt_draft = fixture();
        receipt_draft.evidence.push(ProjectEvidence::Receipt {
            digest,
            tool: Some(EncounterTool::PlayRoom),
        });
        let mut receipt_chain = ProjectChain::new();
        receipt_chain.keep(&receipt_draft).expect("keep");
        let present = receipt_chain
            .preview(None, &journal, ReceiptCheck::Verified(digest))
            .expect("verified");
        assert_eq!(present.evidence[0].status, EvidenceStatus::Present);
        assert_eq!(
            present.evidence[0].text.as_deref(),
            Some("receipt-body-token")
        );
        assert_eq!(
            present.evidence[0].tool,
            Some(EncounterTool::PlayRoom.name())
        );
        let disagreed = receipt_chain
            .preview(None, &journal, ReceiptCheck::Disagreed)
            .expect("disagreed");
        assert_eq!(disagreed.evidence[0].status, EvidenceStatus::Incompatible);
        assert!(disagreed.evidence[0].text.is_none());
        assert!(disagreed.evidence[0].subject.is_none());
        assert_eq!(disagreed.evidence[0].tool, Some("play_room"));
        assert!(!disagreed.will_return);
        let mismatch = receipt_chain
            .preview(None, &journal, ReceiptCheck::Verified([0xcd; 32]))
            .expect("mismatch");
        assert_eq!(mismatch.evidence[0].status, EvidenceStatus::Incompatible);
        assert!(mismatch.evidence[0].text.is_none());
        assert_eq!(mismatch.evidence[0].tool, Some("play_room"));
        assert!(!receipt_chain.to_text().contains("receipt-body-token"));
    }
}
