//! The MCP Experience Journal: an opt-in, player-owned record.
//!
//! Tracks timestamped room encounters, creations, self-authored connections,
//! and optional self-reported affect. Can be exported, inspected, and erased.

/// An entry in the experience journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    /// Unix timestamp in seconds.
    pub timestamp_utc: u64,
    /// The kind of entry (e.g. "encounter", "creation", "connection").
    pub kind: String,
    /// The subject (e.g. room id or creation name).
    pub subject: String,
    /// The actual text content.
    pub text: String,
    /// Optional affect reported by the user.
    pub affect: Option<String>,
}

/// The opt-in player-owned experience journal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Journal {
    /// The ordered list of experience records.
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    /// Create a new, empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new entry to the journal.
    pub fn record(
        &mut self,
        timestamp_utc: u64,
        kind: &str,
        subject: &str,
        text: &str,
        affect: Option<&str>,
    ) {
        self.entries.push(JournalEntry {
            timestamp_utc,
            kind: kind.chars().take(64).collect(),
            subject: subject.chars().take(256).collect(),
            text: text.chars().take(1000).collect(),
            affect: affect.map(|s| s.chars().take(256).collect()),
        });
        // Keep it bounded, though a real player journal might grow.
        if self.entries.len() > 10_000 {
            self.entries.remove(0);
        }
    }

    /// Erase the journal entirely.
    pub fn erase(&mut self) {
        self.entries.clear();
    }

    /// Serialize to text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for entry in &self.entries {
            // Escape newlines and tabs
            let text = entry.text.replace('\t', "\\t").replace('\n', "\\n");
            let affect = entry
                .affect
                .as_deref()
                .unwrap_or("")
                .replace('\t', "\\t")
                .replace('\n', "\\n");
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                entry.timestamp_utc, entry.kind, entry.subject, text, affect
            ));
        }
        out
    }

    /// Parse from text.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let mut journal = Journal::default();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 4 {
                let timestamp_utc = parts[0].parse().unwrap_or(0);
                let kind = parts[1].to_string();
                let subject = parts[2].to_string();
                let text = parts[3].replace("\\n", "\n").replace("\\t", "\t");
                let affect = if parts.len() > 4 && !parts[4].is_empty() {
                    Some(parts[4].replace("\\n", "\n").replace("\\t", "\t"))
                } else {
                    None
                };
                journal.entries.push(JournalEntry {
                    timestamp_utc,
                    kind,
                    subject,
                    text,
                    affect,
                });
            }
        }
        journal
    }
}
