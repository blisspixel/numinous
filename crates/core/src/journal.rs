//! The MCP experience journal: an opt-in, player-owned record.
//!
//! Entries are append-only. A correction adds a new entry with an explicit
//! `supersedes` link, so the original account and both sources remain visible.

use std::fmt;

/// Current portable journal schema version.
pub const JOURNAL_SCHEMA_VERSION: u32 = 2;
/// Maximum entries retained in one journal.
pub const MAX_JOURNAL_ENTRIES: usize = 10_000;
/// Maximum characters in an entry kind.
pub const MAX_JOURNAL_KIND_CHARS: usize = 64;
/// Maximum characters in an entry subject.
pub const MAX_JOURNAL_SUBJECT_CHARS: usize = 256;
/// Maximum characters in entry text.
pub const MAX_JOURNAL_TEXT_CHARS: usize = 1_000;
/// Maximum characters in self-reported affect.
pub const MAX_JOURNAL_AFFECT_CHARS: usize = 256;
/// The caller is recording its own account.
pub const JOURNAL_SOURCE_SELF_AUTHORED: &str = "self-authored";
/// The caller is recording an account supplied by a player.
pub const JOURNAL_SOURCE_PLAYER_PROVIDED: &str = "player-provided";
/// The entry records an explicit result returned by Numinous.
pub const JOURNAL_SOURCE_NUMINOUS_RESULT: &str = "numinous-result";
/// Provenance assigned to entries migrated from the prototype format.
pub const JOURNAL_SOURCE_LEGACY_IMPORT: &str = "legacy-import";
/// Subject prefix for a promoted Numinous Encounter Receipt.
pub const JOURNAL_SUBJECT_RECEIPT_PREFIX: &str = "receipt:";

const JOURNAL_HEADER: &str = "numinous-journal-v2";

/// An entry in the experience journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// Stable journal-local entry identifier.
    pub entry_id: u64,
    /// Unix timestamp in seconds when Numinous accepted the record.
    pub recorded_at_utc: u64,
    /// Unix timestamp in seconds when the described event occurred.
    pub event_at_utc: u64,
    /// Declared source of this account.
    pub source: String,
    /// The kind of entry, such as `encounter`, `creation`, or `connection`.
    pub kind: String,
    /// The room identifier or other subject.
    pub subject: String,
    /// The player-owned text content.
    pub text: String,
    /// Optional self-reported affect. Numinous does not infer this field.
    pub affect: Option<String>,
    /// Earlier entry replaced by this interpretation, when this is a correction.
    pub supersedes: Option<u64>,
}

/// Borrowed fields for one original or corrective journal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalRecord<'a> {
    /// Unix timestamp in seconds when Numinous accepted the record.
    pub recorded_at_utc: u64,
    /// Unix timestamp in seconds when the described event occurred.
    pub event_at_utc: u64,
    /// Declared source of this account.
    pub source: &'a str,
    /// Entry kind.
    pub kind: &'a str,
    /// Room identifier or other subject.
    pub subject: &'a str,
    /// Player-owned text content.
    pub text: &'a str,
    /// Optional explicitly self-reported affect.
    pub affect: Option<&'a str>,
}

/// A journal mutation or persisted-format error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalError {
    /// The fixed entry cap has been reached.
    Capacity,
    /// A caller supplied a source outside the closed provenance vocabulary.
    InvalidSource,
    /// The event time is later than the server-owned record time.
    EventAfterRecord,
    /// Original records may not impersonate the reserved correction kind.
    ReservedCorrectionKind,
    /// A correction target does not exist.
    MissingSuperseded(u64),
    /// A correction target already has a replacement.
    AlreadySuperseded(u64),
    /// No further stable identifier can be allocated.
    IdentifierExhausted,
    /// Persisted v2 text did not match the frozen schema.
    InvalidFormat(String),
}

impl fmt::Display for JournalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capacity => write!(formatter, "journal entry limit reached"),
            Self::InvalidSource => write!(formatter, "journal source is not recognized"),
            Self::EventAfterRecord => {
                write!(
                    formatter,
                    "journal event time is later than its record time"
                )
            }
            Self::ReservedCorrectionKind => {
                write!(
                    formatter,
                    "journal kind 'correction' requires a supersedes link"
                )
            }
            Self::MissingSuperseded(entry_id) => {
                write!(formatter, "journal entry {entry_id} does not exist")
            }
            Self::AlreadySuperseded(entry_id) => {
                write!(formatter, "journal entry {entry_id} is already superseded")
            }
            Self::IdentifierExhausted => write!(formatter, "journal entry identifiers exhausted"),
            Self::InvalidFormat(message) => write!(formatter, "invalid journal format: {message}"),
        }
    }
}

impl std::error::Error for JournalError {}

/// The opt-in player-owned experience journal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Journal {
    /// The ordered append-only experience records.
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    /// Create a new, empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an original entry and return its stable identifier.
    pub fn record(&mut self, record: JournalRecord<'_>) -> Result<u64, JournalError> {
        if record.kind == "correction" {
            return Err(JournalError::ReservedCorrectionKind);
        }
        self.append(record, None)
    }

    /// Append a correction without modifying the target entry.
    pub fn correct(
        &mut self,
        recorded_at_utc: u64,
        event_at_utc: Option<u64>,
        source: &str,
        supersedes: u64,
        text: &str,
        affect: Option<&str>,
    ) -> Result<u64, JournalError> {
        let target = self
            .entries
            .iter()
            .find(|entry| entry.entry_id == supersedes)
            .ok_or(JournalError::MissingSuperseded(supersedes))?;
        if !self.is_current(supersedes) {
            return Err(JournalError::AlreadySuperseded(supersedes));
        }
        let event_at_utc = event_at_utc.unwrap_or(target.event_at_utc);
        let subject = target.subject.clone();
        self.append(
            JournalRecord {
                recorded_at_utc,
                event_at_utc,
                source,
                kind: "correction",
                subject: &subject,
                text,
                affect,
            },
            Some(supersedes),
        )
    }

    /// Whether an entry has no later correction in this journal.
    #[must_use]
    pub fn is_current(&self, entry_id: u64) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.supersedes != Some(entry_id))
    }

    /// Erase every entry in memory.
    pub fn erase(&mut self) {
        self.entries.clear();
    }

    /// Serialize the journal to its durable v2 text format.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::from(JOURNAL_HEADER);
        output.push('\n');
        for entry in &self.entries {
            let affect = entry.affect.as_deref().unwrap_or("");
            let supersedes = entry
                .supersedes
                .map_or_else(String::new, |entry_id| entry_id.to_string());
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                entry.entry_id,
                entry.recorded_at_utc,
                entry.event_at_utc,
                encode_field(&entry.source),
                encode_field(&entry.kind),
                encode_field(&entry.subject),
                encode_field(&entry.text),
                encode_field(affect),
                supersedes,
            ));
        }
        output
    }

    /// Parse persisted text, including migration from the prototype format.
    pub fn try_from_text(text: &str) -> Result<Self, JournalError> {
        let mut lines = text.lines();
        if lines.next() == Some(JOURNAL_HEADER) {
            Self::parse_v2(lines)
        } else {
            Self::parse_legacy(text)
        }
    }

    /// Parse persisted text, returning an empty journal when it is invalid.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self::try_from_text(text).unwrap_or_default()
    }

    fn append(
        &mut self,
        record: JournalRecord<'_>,
        supersedes: Option<u64>,
    ) -> Result<u64, JournalError> {
        if self.entries.len() >= MAX_JOURNAL_ENTRIES {
            return Err(JournalError::Capacity);
        }
        if !valid_source(record.source) {
            return Err(JournalError::InvalidSource);
        }
        if record.event_at_utc > record.recorded_at_utc {
            return Err(JournalError::EventAfterRecord);
        }
        let entry_id = self
            .entries
            .iter()
            .map(|entry| entry.entry_id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(JournalError::IdentifierExhausted)?;
        self.entries.push(JournalEntry {
            entry_id,
            recorded_at_utc: record.recorded_at_utc,
            event_at_utc: record.event_at_utc,
            source: truncate(record.source, MAX_JOURNAL_KIND_CHARS),
            kind: truncate(record.kind, MAX_JOURNAL_KIND_CHARS),
            subject: truncate(record.subject, MAX_JOURNAL_SUBJECT_CHARS),
            text: truncate(record.text, MAX_JOURNAL_TEXT_CHARS),
            affect: record
                .affect
                .filter(|value| !value.is_empty())
                .map(|value| truncate(value, MAX_JOURNAL_AFFECT_CHARS)),
            supersedes,
        });
        Ok(entry_id)
    }

    fn parse_v2<'a>(lines: impl Iterator<Item = &'a str>) -> Result<Self, JournalError> {
        let mut entries = Vec::new();
        let mut previous_id = 0_u64;
        for (index, line) in lines.enumerate() {
            if line.is_empty() {
                continue;
            }
            if entries.len() == MAX_JOURNAL_ENTRIES {
                return Err(JournalError::Capacity);
            }
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 9 {
                return Err(JournalError::InvalidFormat(format!(
                    "line {} has {} fields, expected 9",
                    index + 2,
                    fields.len()
                )));
            }
            let entry_id = parse_u64(fields[0], index + 2, "entry id")?;
            if entry_id <= previous_id {
                return Err(JournalError::InvalidFormat(format!(
                    "line {} entry id is not increasing",
                    index + 2
                )));
            }
            previous_id = entry_id;
            let source = decode_field(fields[3])?;
            if !valid_source(&source) {
                return Err(JournalError::InvalidSource);
            }
            let recorded_at_utc = parse_u64(fields[1], index + 2, "record time")?;
            let event_at_utc = parse_u64(fields[2], index + 2, "event time")?;
            if event_at_utc > recorded_at_utc {
                return Err(JournalError::InvalidFormat(format!(
                    "line {} event time is later than its record time",
                    index + 2
                )));
            }
            let kind = decode_field(fields[4])?;
            let subject = decode_field(fields[5])?;
            let text = decode_field(fields[6])?;
            let affect = if fields[7].is_empty() {
                None
            } else {
                Some(decode_field(fields[7])?)
            };
            validate_field_length(&kind, MAX_JOURNAL_KIND_CHARS, index + 2, "kind")?;
            validate_field_length(&subject, MAX_JOURNAL_SUBJECT_CHARS, index + 2, "subject")?;
            validate_field_length(&text, MAX_JOURNAL_TEXT_CHARS, index + 2, "text")?;
            if let Some(affect) = &affect {
                validate_field_length(affect, MAX_JOURNAL_AFFECT_CHARS, index + 2, "affect")?;
            }
            let supersedes = if fields[8].is_empty() {
                None
            } else {
                Some(parse_u64(fields[8], index + 2, "supersedes")?)
            };
            if let Some(target) = supersedes
                && (!entries
                    .iter()
                    .any(|entry: &JournalEntry| entry.entry_id == target)
                    || entries.iter().any(|entry| entry.supersedes == Some(target)))
            {
                return Err(JournalError::InvalidFormat(format!(
                    "line {} has an invalid supersedes link",
                    index + 2
                )));
            }
            if supersedes.is_some() && kind != "correction" {
                return Err(JournalError::InvalidFormat(format!(
                    "line {} supersedes another entry but is not a correction",
                    index + 2
                )));
            }
            entries.push(JournalEntry {
                entry_id,
                recorded_at_utc,
                event_at_utc,
                source,
                kind,
                subject,
                text,
                affect,
                supersedes,
            });
        }
        Ok(Self { entries })
    }

    fn parse_legacy(text: &str) -> Result<Self, JournalError> {
        let mut entries = Vec::new();
        for (index, line) in text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .enumerate()
        {
            if entries.len() == MAX_JOURNAL_ENTRIES {
                return Err(JournalError::Capacity);
            }
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() < 4 {
                return Err(JournalError::InvalidFormat(format!(
                    "legacy line {} has fewer than 4 fields",
                    index + 1
                )));
            }
            let timestamp = parse_u64(fields[0], index + 1, "legacy timestamp")?;
            entries.push(JournalEntry {
                entry_id: entries.len() as u64 + 1,
                recorded_at_utc: timestamp,
                event_at_utc: timestamp,
                source: JOURNAL_SOURCE_LEGACY_IMPORT.to_string(),
                kind: truncate(fields[1], MAX_JOURNAL_KIND_CHARS),
                subject: truncate(fields[2], MAX_JOURNAL_SUBJECT_CHARS),
                text: truncate(
                    &fields[3].replace("\\n", "\n").replace("\\t", "\t"),
                    MAX_JOURNAL_TEXT_CHARS,
                ),
                affect: fields
                    .get(4)
                    .filter(|value| !value.is_empty())
                    .map(|value| {
                        truncate(
                            &value.replace("\\n", "\n").replace("\\t", "\t"),
                            MAX_JOURNAL_AFFECT_CHARS,
                        )
                    }),
                supersedes: None,
            });
        }
        Ok(Self { entries })
    }
}

fn valid_source(source: &str) -> bool {
    matches!(
        source,
        JOURNAL_SOURCE_SELF_AUTHORED
            | JOURNAL_SOURCE_PLAYER_PROVIDED
            | JOURNAL_SOURCE_NUMINOUS_RESULT
            | JOURNAL_SOURCE_LEGACY_IMPORT
    )
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
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

fn decode_field(value: &str) -> Result<String, JournalError> {
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
            Some(other) => {
                return Err(JournalError::InvalidFormat(format!(
                    "unknown field escape \\{other}"
                )));
            }
            None => {
                return Err(JournalError::InvalidFormat(
                    "field ends with an escape".to_string(),
                ));
            }
        }
    }
    Ok(decoded)
}

fn parse_u64(value: &str, line: usize, field: &str) -> Result<u64, JournalError> {
    value
        .parse()
        .map_err(|_| JournalError::InvalidFormat(format!("line {line} has an invalid {field}")))
}

fn validate_field_length(
    value: &str,
    maximum: usize,
    line: usize,
    field: &str,
) -> Result<(), JournalError> {
    if value.chars().count() > maximum {
        return Err(JournalError::InvalidFormat(format!(
            "line {line} {field} exceeds {maximum} characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        JOURNAL_HEADER, JOURNAL_SOURCE_LEGACY_IMPORT, JOURNAL_SOURCE_NUMINOUS_RESULT,
        JOURNAL_SOURCE_SELF_AUTHORED, Journal, JournalEntry, JournalError, JournalRecord,
        MAX_JOURNAL_ENTRIES,
    };

    #[test]
    fn v2_round_trip_preserves_all_fields_and_escapes() {
        let mut journal = Journal::new();
        assert_eq!(
            journal.record(JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 21,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "connection",
                subject: "times-tables",
                text: "future",
                affect: None,
            }),
            Err(JournalError::EventAfterRecord)
        );
        assert_eq!(
            journal.record(JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 20,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "correction",
                subject: "times-tables",
                text: "unlinked",
                affect: None,
            }),
            Err(JournalError::ReservedCorrectionKind)
        );
        let entry_id = journal
            .record(JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 10,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "connection\tkind",
                subject: "times\\tables",
                text: "line one\nline two",
                affect: Some("curious\rsteady"),
            })
            .expect("record");
        assert_eq!(entry_id, 1);

        let encoded = journal.to_text();
        assert!(encoded.starts_with(JOURNAL_HEADER));
        assert_eq!(Journal::try_from_text(&encoded).expect("parse"), journal);
    }

    #[test]
    fn correction_is_append_only_and_has_one_current_interpretation() {
        let mut journal = Journal::new();
        let original = journal
            .record(JournalRecord {
                recorded_at_utc: 20,
                event_at_utc: 10,
                source: JOURNAL_SOURCE_NUMINOUS_RESULT,
                kind: "connection",
                subject: "times-tables",
                text: "I saw nine lobes",
                affect: None,
            })
            .expect("original");
        let correction = journal
            .correct(
                30,
                None,
                JOURNAL_SOURCE_SELF_AUTHORED,
                original,
                "I saw ten lobes",
                Some("certain"),
            )
            .expect("correction");

        assert_eq!(journal.entries.len(), 2);
        assert_eq!(journal.entries[0].text, "I saw nine lobes");
        assert_eq!(journal.entries[1].supersedes, Some(original));
        assert_eq!(journal.entries[1].event_at_utc, 10);
        assert!(!journal.is_current(original));
        assert!(journal.is_current(correction));
        assert_eq!(
            journal.correct(
                40,
                None,
                JOURNAL_SOURCE_SELF_AUTHORED,
                original,
                "another",
                None,
            ),
            Err(JournalError::AlreadySuperseded(original))
        );
    }

    #[test]
    fn missing_correction_target_is_rejected_without_mutation() {
        let mut journal = Journal::new();
        assert_eq!(
            journal.correct(
                1,
                None,
                JOURNAL_SOURCE_SELF_AUTHORED,
                42,
                "correction",
                None,
            ),
            Err(JournalError::MissingSuperseded(42))
        );
        assert!(journal.entries.is_empty());
    }

    #[test]
    fn prototype_rows_receive_stable_migration_provenance() {
        let legacy = "12\tconnection\tlorenz\tclose\\nfar\tcurious\n";
        let journal = Journal::try_from_text(legacy).expect("legacy migration");
        assert_eq!(journal.entries[0].entry_id, 1);
        assert_eq!(journal.entries[0].recorded_at_utc, 12);
        assert_eq!(journal.entries[0].event_at_utc, 12);
        assert_eq!(journal.entries[0].source, JOURNAL_SOURCE_LEGACY_IMPORT);
        assert_eq!(journal.entries[0].text, "close\nfar");
        assert_eq!(
            Journal::try_from_text(&journal.to_text()).expect("v2 migration"),
            journal
        );
    }

    #[test]
    fn malformed_v2_is_not_silently_repaired() {
        let error =
            Journal::try_from_text("numinous-journal-v2\n1\t2\n").expect_err("malformed v2");
        assert!(matches!(error, JournalError::InvalidFormat(_)));
    }

    #[test]
    fn capacity_failure_never_discards_an_original_entry() {
        let entry = JournalEntry {
            entry_id: 1,
            recorded_at_utc: 1,
            event_at_utc: 1,
            source: JOURNAL_SOURCE_SELF_AUTHORED.to_string(),
            kind: "encounter".to_string(),
            subject: "lorenz".to_string(),
            text: "original".to_string(),
            affect: None,
            supersedes: None,
        };
        let mut journal = Journal {
            entries: vec![entry; MAX_JOURNAL_ENTRIES],
        };
        assert_eq!(
            journal.record(JournalRecord {
                recorded_at_utc: 2,
                event_at_utc: 2,
                source: JOURNAL_SOURCE_SELF_AUTHORED,
                kind: "encounter",
                subject: "lorenz",
                text: "new",
                affect: None,
            }),
            Err(JournalError::Capacity)
        );
        assert_eq!(journal.entries.len(), MAX_JOURNAL_ENTRIES);
        assert_eq!(journal.entries[0].text, "original");
    }
}
