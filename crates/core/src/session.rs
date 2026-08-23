//! A resettable session workspace: visit continuity, not a memory.
//!
//! The workspace holds only what the player put there. It does not record
//! plays, infer attention, or survive the process that owns it. Persistence
//! across visits remains the opt-in journal.

use crate::rooms::{canonical_room_id, room_meta_by_id};

/// Schema name for a session workspace projection.
pub const SESSION_WORKSPACE_SCHEMA: &str = "numinous.session-workspace";

/// Current workspace schema version.
pub const SESSION_WORKSPACE_SCHEMA_VERSION: u64 = 2;

/// Longest player-authored workspace note, intention, or prediction.
pub const MAX_WORKSPACE_TEXT_CHARS: usize = 280;

/// Longest reason attached to a retrieved journal handle.
pub const MAX_WORKSPACE_REASON_CHARS: usize = 140;

/// Longest title parked on an unfinished creation.
pub const MAX_WORKSPACE_TITLE_CHARS: usize = 48;

/// Newest player-selected observations retained in one visit.
pub const MAX_WORKSPACE_RECENT: usize = 8;

/// Journal handles the player asked to keep at hand.
pub const MAX_WORKSPACE_RETRIEVED: usize = 4;

/// Compact state that connects calls within one visit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionWorkspace {
    place: Option<WorkspacePlace>,
    intention: Option<String>,
    pending_prediction: Option<String>,
    unfinished: Option<WorkspaceUnfinished>,
    recent: Vec<WorkspaceObservation>,
    retrieved: Vec<WorkspaceRetrieval>,
    deferred: DeferredWorkspace,
}

/// Where the player says it is standing.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePlace {
    room: String,
    t: Option<f64>,
    variation: Option<u64>,
}

/// An unfinished action or creation the player wants to resume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceUnfinished {
    /// A room action still in progress.
    Action {
        /// Listed room the action belongs to.
        room: String,
        /// Player-authored remainder.
        note: String,
    },
    /// A creation still in progress.
    Creation {
        /// Optional working title.
        title: Option<String>,
        /// Player-authored remainder.
        note: String,
    },
}

/// One observation the player chose to keep nearby.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceObservation {
    room: String,
    note: String,
}

/// A journal handle the player asked to keep at hand.
///
/// The workspace stores only the identifier and optional selection reason.
/// The owning face may resolve that handle against the player-owned journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRetrieval {
    entry_id: u64,
    reason: Option<String>,
}

/// Parked copies of fields the player deferred without discarding.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeferredWorkspace {
    place: Option<WorkspacePlace>,
    intention: Option<String>,
    pending_prediction: Option<String>,
    unfinished: Option<WorkspaceUnfinished>,
    recent: Vec<WorkspaceObservation>,
    retrieved: Vec<WorkspaceRetrieval>,
}

/// Named slot a player can inspect, edit, defer, or clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceField {
    /// Current place.
    Place,
    /// Self-chosen question or intention.
    Intention,
    /// Pending prediction.
    PendingPrediction,
    /// Unfinished action or creation.
    Unfinished,
    /// Recent selected observations.
    Recent,
    /// Retrieved journal handles.
    Retrieved,
    /// The deferred parking lot.
    Deferred,
}

/// A workspace mutation or argument error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    /// The named room is not a listed catalog room.
    UnknownRoom(String),
    /// Phase was missing, non-finite, or outside `[0, 1)`.
    InvalidPhase,
    /// Player-authored text exceeded its bound.
    TextTooLong {
        /// Which argument overflowed.
        field: &'static str,
        /// Inclusive maximum.
        max_chars: usize,
    },
    /// A list exceeded its bound.
    ListTooLong {
        /// Which list overflowed.
        field: &'static str,
        /// Inclusive maximum.
        max_items: usize,
    },
    /// An action needs a listed room.
    ActionNeedsRoom,
    /// A retrieval handle must be a positive journal entry id.
    InvalidEntryId,
    /// Edit supplied no fields to change.
    NothingToEdit,
    /// Defer was asked on an empty active field, or on the deferred lot.
    NothingToDefer,
    /// Clear was asked on the empty catch-all that is not a field.
    UnknownField,
}

impl SessionWorkspace {
    /// An empty visit workspace.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True when every active and deferred slot is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.place.is_none()
            && self.intention.is_none()
            && self.pending_prediction.is_none()
            && self.unfinished.is_none()
            && self.recent.is_empty()
            && self.retrieved.is_empty()
            && self.deferred.is_empty()
    }

    /// Current place, if set.
    #[must_use]
    pub fn place(&self) -> Option<&WorkspacePlace> {
        self.place.as_ref()
    }

    /// Self-chosen question or intention.
    #[must_use]
    pub fn intention(&self) -> Option<&str> {
        self.intention.as_deref()
    }

    /// Pending prediction text.
    #[must_use]
    pub fn pending_prediction(&self) -> Option<&str> {
        self.pending_prediction.as_deref()
    }

    /// Unfinished action or creation.
    #[must_use]
    pub fn unfinished(&self) -> Option<&WorkspaceUnfinished> {
        self.unfinished.as_ref()
    }

    /// Player-selected recent observations, newest last.
    #[must_use]
    pub fn recent(&self) -> &[WorkspaceObservation] {
        &self.recent
    }

    /// Journal handles the player asked to keep at hand.
    #[must_use]
    pub fn retrieved(&self) -> &[WorkspaceRetrieval] {
        &self.retrieved
    }

    /// Parked copies of deferred fields.
    #[must_use]
    pub fn deferred(&self) -> &DeferredWorkspace {
        &self.deferred
    }

    /// Replace the named active fields. Omitted fields stay as they are.
    pub fn edit(&mut self, update: WorkspaceUpdate) -> Result<(), WorkspaceError> {
        if update.is_empty() {
            return Err(WorkspaceError::NothingToEdit);
        }
        let place = match update.place {
            Some(value) => Some(Some(validated_place(value)?)),
            None => None,
        };
        let intention = match update.intention {
            Some(value) => Some(Some(bounded_text(
                value,
                "intention",
                MAX_WORKSPACE_TEXT_CHARS,
            )?)),
            None => None,
        };
        let pending_prediction = match update.pending_prediction {
            Some(value) => Some(Some(bounded_text(
                value,
                "pending_prediction",
                MAX_WORKSPACE_TEXT_CHARS,
            )?)),
            None => None,
        };
        let unfinished = match update.unfinished {
            Some(value) => Some(Some(validated_unfinished(value)?)),
            None => None,
        };
        let recent = match update.recent {
            Some(value) => Some(validated_recent(value)?),
            None => None,
        };
        let retrieved = match update.retrieved {
            Some(value) => Some(validated_retrieved(value)?),
            None => None,
        };

        if let Some(place) = place {
            self.place = place;
        }
        if let Some(intention) = intention {
            self.intention = intention;
        }
        if let Some(pending_prediction) = pending_prediction {
            self.pending_prediction = pending_prediction;
        }
        if let Some(unfinished) = unfinished {
            self.unfinished = unfinished;
        }
        if let Some(recent) = recent {
            self.recent = recent;
        }
        if let Some(retrieved) = retrieved {
            self.retrieved = retrieved;
        }
        Ok(())
    }

    /// Park the active field and clear it. The deferred copy remains inspectable.
    pub fn defer(&mut self, field: WorkspaceField) -> Result<(), WorkspaceError> {
        match field {
            WorkspaceField::Place => {
                let value = self.place.take().ok_or(WorkspaceError::NothingToDefer)?;
                self.deferred.place = Some(value);
            }
            WorkspaceField::Intention => {
                let value = self
                    .intention
                    .take()
                    .ok_or(WorkspaceError::NothingToDefer)?;
                self.deferred.intention = Some(value);
            }
            WorkspaceField::PendingPrediction => {
                let value = self
                    .pending_prediction
                    .take()
                    .ok_or(WorkspaceError::NothingToDefer)?;
                self.deferred.pending_prediction = Some(value);
            }
            WorkspaceField::Unfinished => {
                let value = self
                    .unfinished
                    .take()
                    .ok_or(WorkspaceError::NothingToDefer)?;
                self.deferred.unfinished = Some(value);
            }
            WorkspaceField::Recent => {
                if self.recent.is_empty() {
                    return Err(WorkspaceError::NothingToDefer);
                }
                self.deferred.recent = std::mem::take(&mut self.recent);
            }
            WorkspaceField::Retrieved => {
                if self.retrieved.is_empty() {
                    return Err(WorkspaceError::NothingToDefer);
                }
                self.deferred.retrieved = std::mem::take(&mut self.retrieved);
            }
            WorkspaceField::Deferred => return Err(WorkspaceError::NothingToDefer),
        }
        Ok(())
    }

    /// Clear one field, the deferred lot, or the whole workspace.
    pub fn clear(&mut self, field: WorkspaceClear) {
        match field {
            WorkspaceClear::Place => self.place = None,
            WorkspaceClear::Intention => self.intention = None,
            WorkspaceClear::PendingPrediction => self.pending_prediction = None,
            WorkspaceClear::Unfinished => self.unfinished = None,
            WorkspaceClear::Recent => self.recent.clear(),
            WorkspaceClear::Retrieved => self.retrieved.clear(),
            WorkspaceClear::Deferred => self.deferred = DeferredWorkspace::default(),
            WorkspaceClear::All => *self = Self::default(),
        }
    }
}

impl WorkspacePlace {
    /// Listed room id after alias resolution.
    #[must_use]
    pub fn room(&self) -> &str {
        &self.room
    }

    /// Optional destination phase in `[0, 1)`.
    #[must_use]
    pub fn t(&self) -> Option<f64> {
        self.t
    }

    /// Optional variation seed.
    #[must_use]
    pub fn variation(&self) -> Option<u64> {
        self.variation
    }
}

impl WorkspaceObservation {
    /// Listed room this note is about.
    #[must_use]
    pub fn room(&self) -> &str {
        &self.room
    }

    /// Player-authored note.
    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }
}

impl WorkspaceRetrieval {
    /// Journal entry identifier. Resolution remains the owning face's job.
    #[must_use]
    pub fn entry_id(&self) -> u64 {
        self.entry_id
    }

    /// Optional reason the player asked to keep this handle.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

impl DeferredWorkspace {
    /// True when nothing is parked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.place.is_none()
            && self.intention.is_none()
            && self.pending_prediction.is_none()
            && self.unfinished.is_none()
            && self.recent.is_empty()
            && self.retrieved.is_empty()
    }

    /// Parked place.
    #[must_use]
    pub fn place(&self) -> Option<&WorkspacePlace> {
        self.place.as_ref()
    }

    /// Parked intention.
    #[must_use]
    pub fn intention(&self) -> Option<&str> {
        self.intention.as_deref()
    }

    /// Parked pending prediction.
    #[must_use]
    pub fn pending_prediction(&self) -> Option<&str> {
        self.pending_prediction.as_deref()
    }

    /// Parked unfinished work.
    #[must_use]
    pub fn unfinished(&self) -> Option<&WorkspaceUnfinished> {
        self.unfinished.as_ref()
    }

    /// Parked recent observations.
    #[must_use]
    pub fn recent(&self) -> &[WorkspaceObservation] {
        &self.recent
    }

    /// Parked retrieval handles.
    #[must_use]
    pub fn retrieved(&self) -> &[WorkspaceRetrieval] {
        &self.retrieved
    }
}

/// Fields an edit may replace. `None` means leave the current value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkspaceUpdate {
    /// Replace the current place.
    pub place: Option<WorkspacePlaceDraft>,
    /// Replace the current intention.
    pub intention: Option<String>,
    /// Replace the current pending prediction.
    pub pending_prediction: Option<String>,
    /// Replace the current unfinished work.
    pub unfinished: Option<WorkspaceUnfinishedDraft>,
    /// Replace the recent observation list.
    pub recent: Option<Vec<WorkspaceObservationDraft>>,
    /// Replace the retrieved-handle list.
    pub retrieved: Option<Vec<WorkspaceRetrievalDraft>>,
}

impl WorkspaceUpdate {
    fn is_empty(&self) -> bool {
        self.place.is_none()
            && self.intention.is_none()
            && self.pending_prediction.is_none()
            && self.unfinished.is_none()
            && self.recent.is_none()
            && self.retrieved.is_none()
    }
}

/// Unvalidated place supplied by a caller.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacePlaceDraft {
    /// Room id or alias.
    pub room: String,
    /// Optional phase in `[0, 1)`.
    pub t: Option<f64>,
    /// Optional variation seed.
    pub variation: Option<u64>,
}

/// Unvalidated unfinished work supplied by a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceUnfinishedDraft {
    /// A room action still in progress.
    Action {
        /// Room id or alias.
        room: Option<String>,
        /// Player-authored remainder.
        note: String,
    },
    /// A creation still in progress.
    Creation {
        /// Optional working title.
        title: Option<String>,
        /// Player-authored remainder.
        note: String,
    },
}

/// Unvalidated observation supplied by a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceObservationDraft {
    /// Room id or alias.
    pub room: String,
    /// Player-authored note.
    pub note: String,
}

/// Unvalidated retrieval handle supplied by a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRetrievalDraft {
    /// Journal entry identifier.
    pub entry_id: u64,
    /// Optional reason for keeping the handle.
    pub reason: Option<String>,
}

/// What a clear request may drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceClear {
    /// Active place.
    Place,
    /// Active intention.
    Intention,
    /// Active pending prediction.
    PendingPrediction,
    /// Active unfinished work.
    Unfinished,
    /// Active recent observations.
    Recent,
    /// Active retrieved handles.
    Retrieved,
    /// The deferred parking lot.
    Deferred,
    /// Every active and deferred slot.
    All,
}

impl WorkspaceField {
    /// Parse a wire field name.
    pub fn parse(name: &str) -> Result<Self, WorkspaceError> {
        match name {
            "place" => Ok(Self::Place),
            "intention" => Ok(Self::Intention),
            "pending_prediction" => Ok(Self::PendingPrediction),
            "unfinished" => Ok(Self::Unfinished),
            "recent" => Ok(Self::Recent),
            "retrieved" => Ok(Self::Retrieved),
            "deferred" => Ok(Self::Deferred),
            _ => Err(WorkspaceError::UnknownField),
        }
    }

    /// Wire name for this field.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Place => "place",
            Self::Intention => "intention",
            Self::PendingPrediction => "pending_prediction",
            Self::Unfinished => "unfinished",
            Self::Recent => "recent",
            Self::Retrieved => "retrieved",
            Self::Deferred => "deferred",
        }
    }
}

impl WorkspaceClear {
    /// Parse a wire clear target, including `all`.
    pub fn parse(name: &str) -> Result<Self, WorkspaceError> {
        match name {
            "all" => Ok(Self::All),
            other => match WorkspaceField::parse(other)? {
                WorkspaceField::Place => Ok(Self::Place),
                WorkspaceField::Intention => Ok(Self::Intention),
                WorkspaceField::PendingPrediction => Ok(Self::PendingPrediction),
                WorkspaceField::Unfinished => Ok(Self::Unfinished),
                WorkspaceField::Recent => Ok(Self::Recent),
                WorkspaceField::Retrieved => Ok(Self::Retrieved),
                WorkspaceField::Deferred => Ok(Self::Deferred),
            },
        }
    }
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRoom(id) => write!(
                f,
                "No listed room with id '{}'. Call list_rooms to browse the catalog.",
                crate::echoable_id(id)
            ),
            Self::InvalidPhase => {
                write!(f, "Place t must be a finite phase in [0, 1).")
            }
            Self::TextTooLong { field, max_chars } => {
                write!(f, "{field} must be at most {max_chars} characters.")
            }
            Self::ListTooLong { field, max_items } => {
                write!(f, "{field} holds at most {max_items} items.")
            }
            Self::ActionNeedsRoom => {
                write!(f, "An unfinished action needs a listed room id.")
            }
            Self::InvalidEntryId => {
                write!(f, "A retrieved handle needs a positive journal entry id.")
            }
            Self::NothingToEdit => {
                write!(f, "Edit needs at least one field to set.")
            }
            Self::NothingToDefer => {
                write!(
                    f,
                    "Defer needs a filled active field; the deferred lot cannot be deferred."
                )
            }
            Self::UnknownField => {
                write!(
                    f,
                    "Unknown workspace field. Use place, intention, pending_prediction, unfinished, recent, retrieved, deferred, or all."
                )
            }
        }
    }
}

fn validated_place(draft: WorkspacePlaceDraft) -> Result<WorkspacePlace, WorkspaceError> {
    Ok(WorkspacePlace {
        room: listed_room(&draft.room)?,
        t: match draft.t {
            Some(t) => Some(validated_phase(t)?),
            None => None,
        },
        variation: draft.variation,
    })
}

fn validated_unfinished(
    draft: WorkspaceUnfinishedDraft,
) -> Result<WorkspaceUnfinished, WorkspaceError> {
    match draft {
        WorkspaceUnfinishedDraft::Action { room, note } => {
            let room = room.ok_or(WorkspaceError::ActionNeedsRoom)?;
            Ok(WorkspaceUnfinished::Action {
                room: listed_room(&room)?,
                note: bounded_text(note, "unfinished.note", MAX_WORKSPACE_TEXT_CHARS)?,
            })
        }
        WorkspaceUnfinishedDraft::Creation { title, note } => Ok(WorkspaceUnfinished::Creation {
            title: match title {
                Some(title) => Some(bounded_text(
                    title,
                    "unfinished.title",
                    MAX_WORKSPACE_TITLE_CHARS,
                )?),
                None => None,
            },
            note: bounded_text(note, "unfinished.note", MAX_WORKSPACE_TEXT_CHARS)?,
        }),
    }
}

fn validated_recent(
    drafts: Vec<WorkspaceObservationDraft>,
) -> Result<Vec<WorkspaceObservation>, WorkspaceError> {
    if drafts.len() > MAX_WORKSPACE_RECENT {
        return Err(WorkspaceError::ListTooLong {
            field: "recent",
            max_items: MAX_WORKSPACE_RECENT,
        });
    }
    drafts
        .into_iter()
        .map(|draft| {
            Ok(WorkspaceObservation {
                room: listed_room(&draft.room)?,
                note: bounded_text(draft.note, "recent.note", MAX_WORKSPACE_TEXT_CHARS)?,
            })
        })
        .collect()
}

fn validated_retrieved(
    drafts: Vec<WorkspaceRetrievalDraft>,
) -> Result<Vec<WorkspaceRetrieval>, WorkspaceError> {
    if drafts.len() > MAX_WORKSPACE_RETRIEVED {
        return Err(WorkspaceError::ListTooLong {
            field: "retrieved",
            max_items: MAX_WORKSPACE_RETRIEVED,
        });
    }
    drafts
        .into_iter()
        .map(|draft| {
            if draft.entry_id == 0 {
                return Err(WorkspaceError::InvalidEntryId);
            }
            Ok(WorkspaceRetrieval {
                entry_id: draft.entry_id,
                reason: match draft.reason {
                    Some(reason) => Some(bounded_text(
                        reason,
                        "retrieved.reason",
                        MAX_WORKSPACE_REASON_CHARS,
                    )?),
                    None => None,
                },
            })
        })
        .collect()
}

fn listed_room(id: &str) -> Result<String, WorkspaceError> {
    let canonical = canonical_room_id(id);
    match room_meta_by_id(canonical) {
        Some(meta) => Ok(meta.id.to_string()),
        None => Err(WorkspaceError::UnknownRoom(id.to_string())),
    }
}

fn validated_phase(t: f64) -> Result<f64, WorkspaceError> {
    if t.is_finite() && (0.0..1.0).contains(&t) {
        Ok(t)
    } else {
        Err(WorkspaceError::InvalidPhase)
    }
}

fn bounded_text(
    value: String,
    field: &'static str,
    max_chars: usize,
) -> Result<String, WorkspaceError> {
    if value.chars().count() > max_chars {
        return Err(WorkspaceError::TextTooLong { field, max_chars });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(room: &str) -> WorkspacePlaceDraft {
        WorkspacePlaceDraft {
            room: room.to_string(),
            t: Some(0.25),
            variation: Some(3),
        }
    }

    #[test]
    fn empty_workspace_has_no_inferred_state() {
        let workspace = SessionWorkspace::new();
        assert!(workspace.is_empty());
        assert!(workspace.place().is_none());
        assert!(workspace.intention().is_none());
        assert!(workspace.recent().is_empty());
        assert!(workspace.deferred().is_empty());
    }

    #[test]
    fn edit_is_transactional_and_resolves_aliases() {
        let mut workspace = SessionWorkspace::new();
        let err = workspace
            .edit(WorkspaceUpdate {
                place: Some(place("no-such-room")),
                intention: Some("stay with the pendulum".to_string()),
                ..WorkspaceUpdate::default()
            })
            .unwrap_err();
        assert!(matches!(err, WorkspaceError::UnknownRoom(_)));
        assert!(workspace.is_empty());

        workspace
            .edit(WorkspaceUpdate {
                place: Some(place("kepler-areas")),
                intention: Some("equal time near the sun".to_string()),
                ..WorkspaceUpdate::default()
            })
            .unwrap();
        let standing = workspace.place().expect("place");
        assert_eq!(standing.room(), "kepler-laws");
        assert_eq!(standing.t(), Some(0.25));
        assert_eq!(standing.variation(), Some(3));
        assert_eq!(workspace.intention(), Some("equal time near the sun"));
    }

    #[test]
    fn play_is_not_recorded_unless_the_player_edits() {
        let workspace = SessionWorkspace::new();
        assert!(workspace.is_empty());
    }

    #[test]
    fn defer_parks_without_discarding_and_clear_all_resets() {
        let mut workspace = SessionWorkspace::new();
        workspace
            .edit(WorkspaceUpdate {
                intention: Some("come back to Kepler".to_string()),
                pending_prediction: Some("faster near the sun".to_string()),
                ..WorkspaceUpdate::default()
            })
            .unwrap();
        workspace.defer(WorkspaceField::Intention).unwrap();
        assert!(workspace.intention().is_none());
        assert_eq!(
            workspace.deferred().intention(),
            Some("come back to Kepler")
        );
        assert_eq!(workspace.pending_prediction(), Some("faster near the sun"));
        assert!(workspace.defer(WorkspaceField::Intention).is_err());
        assert!(workspace.defer(WorkspaceField::Deferred).is_err());

        workspace.clear(WorkspaceClear::PendingPrediction);
        assert!(workspace.pending_prediction().is_none());
        assert_eq!(
            workspace.deferred().intention(),
            Some("come back to Kepler")
        );

        workspace.clear(WorkspaceClear::All);
        assert!(workspace.is_empty());
    }

    #[test]
    fn lists_are_bounded_and_retrieval_stores_handles_only() {
        let mut workspace = SessionWorkspace::new();
        let too_many: Vec<_> = (0..=MAX_WORKSPACE_RECENT)
            .map(|i| WorkspaceObservationDraft {
                room: "lorenz".to_string(),
                note: format!("look {i}"),
            })
            .collect();
        let err = workspace
            .edit(WorkspaceUpdate {
                recent: Some(too_many),
                ..WorkspaceUpdate::default()
            })
            .unwrap_err();
        assert!(matches!(
            err,
            WorkspaceError::ListTooLong {
                field: "recent",
                ..
            }
        ));

        workspace
            .edit(WorkspaceUpdate {
                retrieved: Some(vec![WorkspaceRetrievalDraft {
                    entry_id: 7,
                    reason: Some("the first pendulum drop".to_string()),
                }]),
                ..WorkspaceUpdate::default()
            })
            .unwrap();
        assert_eq!(workspace.retrieved()[0].entry_id(), 7);
        assert_eq!(
            workspace.retrieved()[0].reason(),
            Some("the first pendulum drop")
        );

        let err = workspace
            .edit(WorkspaceUpdate {
                retrieved: Some(vec![WorkspaceRetrievalDraft {
                    entry_id: 0,
                    reason: None,
                }]),
                ..WorkspaceUpdate::default()
            })
            .unwrap_err();
        assert_eq!(err, WorkspaceError::InvalidEntryId);
        assert_eq!(workspace.retrieved()[0].entry_id(), 7);
    }

    #[test]
    fn unfinished_action_requires_a_listed_room() {
        let mut workspace = SessionWorkspace::new();
        let err = workspace
            .edit(WorkspaceUpdate {
                unfinished: Some(WorkspaceUnfinishedDraft::Action {
                    room: None,
                    note: "still holding the bob".to_string(),
                }),
                ..WorkspaceUpdate::default()
            })
            .unwrap_err();
        assert_eq!(err, WorkspaceError::ActionNeedsRoom);

        workspace
            .edit(WorkspaceUpdate {
                unfinished: Some(WorkspaceUnfinishedDraft::Creation {
                    title: Some("twin hills".to_string()),
                    note: "sing the ratio".to_string(),
                }),
                ..WorkspaceUpdate::default()
            })
            .unwrap();
        match workspace.unfinished() {
            Some(WorkspaceUnfinished::Creation { title, note }) => {
                assert_eq!(title.as_deref(), Some("twin hills"));
                assert_eq!(note, "sing the ratio");
            }
            other => panic!("expected creation, got {other:?}"),
        }
    }

    #[test]
    fn phase_and_text_bounds_are_enforced() {
        let mut workspace = SessionWorkspace::new();
        assert_eq!(
            workspace
                .edit(WorkspaceUpdate {
                    place: Some(WorkspacePlaceDraft {
                        room: "lorenz".to_string(),
                        t: Some(1.0),
                        variation: None,
                    }),
                    ..WorkspaceUpdate::default()
                })
                .unwrap_err(),
            WorkspaceError::InvalidPhase
        );
        let long = "x".repeat(MAX_WORKSPACE_TEXT_CHARS + 1);
        let err = workspace
            .edit(WorkspaceUpdate {
                intention: Some(long),
                ..WorkspaceUpdate::default()
            })
            .unwrap_err();
        assert!(matches!(
            err,
            WorkspaceError::TextTooLong {
                field: "intention",
                ..
            }
        ));
        assert!(workspace.is_empty());
    }
}
