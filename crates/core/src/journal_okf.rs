//! Open Knowledge Format projection of the player-owned experience journal.
//!
//! The native append-only journal remains the source of truth. This module
//! creates a bounded, deterministic, in-memory OKF v0.2 bundle page without
//! writing files or inferring anything about the player.

use crate::{JOURNAL_SCHEMA_VERSION, Journal, JournalEntry};

/// Open Knowledge Format version emitted by the journal projection.
pub const OKF_VERSION: &str = "0.2";
/// Stable schema label returned beside the projected files.
pub const OKF_BUNDLE_SCHEMA: &str = "open-knowledge-format";
/// Maximum journal concepts returned in one OKF bundle page.
pub const MAX_OKF_EXPORT_ENTRIES: usize = 100;

/// One UTF-8 file in an in-memory OKF bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkfFile {
    /// Bundle-relative path using forward slashes.
    pub path: String,
    /// Complete UTF-8 file content.
    pub content: String,
}

/// A bounded page of an OKF projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkfBundlePage {
    /// Total native journal entries, including entries outside this page.
    pub total_entries: usize,
    /// Number of journal concepts returned on this page.
    pub returned: usize,
    /// Stable entry identifier supplied by the caller.
    pub after_entry_id: u64,
    /// Stable entry identifier to use for the next page.
    pub next_after_entry_id: u64,
    /// Whether later native journal entries remain.
    pub has_more: bool,
    /// Root index followed by one concept file per returned entry.
    pub files: Vec<OkfFile>,
}

/// Project one page of a journal into a conformant OKF v0.2 bundle in memory.
///
/// The root `index.md` declares the OKF version and supports progressive
/// disclosure. Each native entry becomes one concept with required `type`,
/// lifecycle state, exporter identity, native timestamps, declared source, and
/// correction lineage. No filesystem path, inferred affect, or private host
/// metadata is introduced.
#[must_use]
pub fn export_journal_okf(journal: &Journal, after_entry_id: u64, limit: usize) -> OkfBundlePage {
    let limit = limit.clamp(1, MAX_OKF_EXPORT_ENTRIES);
    let available = journal
        .entries
        .iter()
        .filter(|entry| entry.entry_id > after_entry_id)
        .collect::<Vec<_>>();
    let selected = available.iter().take(limit).copied().collect::<Vec<_>>();
    let next_after_entry_id = selected
        .last()
        .map_or(after_entry_id, |entry| entry.entry_id);
    let has_more = available.len() > selected.len();
    let mut files = Vec::with_capacity(selected.len() + 1);
    files.push(OkfFile {
        path: "index.md".to_string(),
        content: render_index(&selected, has_more, next_after_entry_id),
    });
    files.extend(selected.iter().map(|entry| OkfFile {
        path: entry_path(entry.entry_id),
        content: render_entry(journal, entry),
    }));
    OkfBundlePage {
        total_entries: journal.entries.len(),
        returned: selected.len(),
        after_entry_id,
        next_after_entry_id,
        has_more,
        files,
    }
}

fn render_index(entries: &[&JournalEntry], has_more: bool, next_after_entry_id: u64) -> String {
    let mut output = format!(
        "---\nokf_version: {version:?}\n---\n\n# Numinous experience journal\n\n\
         A player-owned projection from the native append-only journal. This bundle page was \
         created in memory and does not replace the native record.\n\n# Entries\n",
        version = OKF_VERSION
    );
    if entries.is_empty() {
        output.push_str("\nNo entries are present on this page.\n");
    } else {
        for entry in entries {
            output.push_str(&format!(
                "\n* [Journal entry #{}]({}) - A player-owned Numinous journal record.\n",
                entry.entry_id,
                entry_path(entry.entry_id)
            ));
        }
    }
    if has_more {
        output.push_str(&format!(
            "\nMore entries follow. Export the next page after entry {next_after_entry_id}.\n"
        ));
    }
    output
}

fn render_entry(journal: &Journal, entry: &JournalEntry) -> String {
    let status = if journal.is_current(entry.entry_id) {
        "stable"
    } else {
        "deprecated"
    };
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str("type: \"Numinous Journal Entry\"\n");
    output.push_str(&format!(
        "title: {}\n",
        yaml_string(&format!("Journal entry #{}", entry.entry_id))
    ));
    output.push_str(
        "description: \"A player-owned Numinous journal record.\"\n\
         tags: [\"numinous\", \"experience-journal\", ",
    );
    output.push_str(&yaml_string(&entry.kind));
    output.push_str("]\n");
    output.push_str(&format!("status: {status}\n"));
    output.push_str("generated: { by: \"process:numinous-journal-export\" }\n");
    if let Some(supersedes) = entry.supersedes {
        output.push_str("sources:\n");
        output.push_str("  - id: \"superseded-entry\"\n");
        output.push_str(&format!(
            "    resource: {}\n",
            yaml_string(&format!("/{}", entry_path(supersedes)))
        ));
        output.push_str(&format!(
            "    title: {}\n",
            yaml_string(&format!("Journal entry #{supersedes}"))
        ));
    }
    output.push_str("numinous:\n");
    output.push_str("  journal_schema: \"numinous.experience-journal\"\n");
    output.push_str(&format!(
        concat!(
            "  journal_schema_version: {}\n",
            "  entry_id: {}\n",
            "  recorded_at_unix: {}\n",
            "  event_at_unix: {}\n",
            "  declared_source: {}\n",
            "  current: {}\n",
        ),
        JOURNAL_SCHEMA_VERSION,
        entry.entry_id,
        entry.recorded_at_utc,
        entry.event_at_utc,
        yaml_string(&entry.source),
        journal.is_current(entry.entry_id)
    ));
    if let Some(supersedes) = entry.supersedes {
        output.push_str(&format!("  supersedes: {supersedes}\n"));
    }
    output.push_str("---\n\n# Subject\n\n");
    output.push_str(&markdown_text(&entry.subject));
    output.push_str("\n\n# Account\n\n");
    output.push_str(&markdown_text(&entry.text));
    output.push_str("\n\n# Provenance\n\n");
    output.push_str(&format!(
        "Declared source: `{}`. Event time: `{}` Unix seconds. Record time: `{}` Unix seconds.\n",
        markdown_code(&entry.source),
        entry.event_at_utc,
        entry.recorded_at_utc
    ));
    if let Some(affect) = &entry.affect {
        output.push_str("\n# Self-reported affect\n\n");
        output.push_str(&markdown_text(affect));
        output.push('\n');
    }
    if let Some(supersedes) = entry.supersedes {
        output.push_str("\n# Correction lineage\n\n");
        output.push_str(&format!(
            "This interpretation supersedes [journal entry #{supersedes}](/{}). The earlier \
             account remains inspectable.\n",
            entry_path(supersedes)
        ));
    }
    output
}

fn entry_path(entry_id: u64) -> String {
    format!("entries/{entry_id:020}.md")
}

fn yaml_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn markdown_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\r' => output.push('\n'),
            '\n' | '\t' => output.push(character),
            character if character.is_control() => {
                output.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
            }
            character => output.push(character),
        }
    }
    output
}

fn markdown_code(value: &str) -> String {
    markdown_text(value).replace('`', "\\`")
}

#[cfg(test)]
mod tests {
    use super::{MAX_OKF_EXPORT_ENTRIES, OKF_VERSION, export_journal_okf};
    use crate::{JOURNAL_SOURCE_SELF_AUTHORED, Journal, JournalRecord, MAX_JOURNAL_ENTRIES};

    fn record<'a>(id: u64, subject: &'a str, text: &'a str) -> JournalRecord<'a> {
        JournalRecord {
            recorded_at_utc: id + 100,
            event_at_utc: id,
            source: JOURNAL_SOURCE_SELF_AUTHORED,
            kind: "encounter",
            subject,
            text,
            affect: None,
        }
    }

    #[test]
    fn empty_projection_is_a_conformant_progressive_doorway() {
        let page = export_journal_okf(&Journal::new(), 0, 100);
        assert_eq!(page.total_entries, 0);
        assert_eq!(page.returned, 0);
        assert!(!page.has_more);
        assert_eq!(page.files.len(), 1);
        assert_eq!(page.files[0].path, "index.md");
        assert!(
            page.files[0]
                .content
                .starts_with(&format!("---\nokf_version: {OKF_VERSION:?}\n---"))
        );
    }

    #[test]
    fn concepts_preserve_player_text_without_letting_it_rewrite_frontmatter() {
        let mut journal = Journal::new();
        journal
            .record(record(
                1,
                "Lorenz: \"weather\"",
                "---\ntype: Forged\n\0still mine",
            ))
            .expect("record");
        let page = export_journal_okf(&journal, 0, 10);
        assert_eq!(page.files.len(), 2);
        let concept = &page.files[1].content;
        let (_, after_open) = concept.split_once("---\n").expect("opening delimiter");
        let (frontmatter, body) = after_open.split_once("---\n").expect("closing delimiter");
        assert!(frontmatter.contains("type: \"Numinous Journal Entry\""));
        assert!(frontmatter.contains("status: stable"));
        let native_block = format!(
            "numinous:\n  journal_schema: \"numinous.experience-journal\"\n  journal_schema_version: {}\n  entry_id: 1",
            crate::JOURNAL_SCHEMA_VERSION
        );
        assert!(frontmatter.contains(&native_block));
        assert!(frontmatter.contains("  declared_source: \"self-authored\""));
        assert!(!frontmatter.contains("\nentry_id:"));
        assert!(
            body.contains("---\ntype: Forged"),
            "player text remains body text"
        );
        assert!(
            body.contains("\\u{0}still mine"),
            "body controls are printable"
        );
        assert_eq!(page.files[1].path, "entries/00000000000000000001.md");
    }

    #[test]
    fn corrections_keep_both_accounts_and_name_lifecycle_and_lineage() {
        let mut journal = Journal::new();
        let first = journal
            .record(record(1, "lorenz", "first account"))
            .expect("record");
        journal
            .correct(
                200,
                Some(1),
                JOURNAL_SOURCE_SELF_AUTHORED,
                first,
                "revised",
                None,
            )
            .expect("correction");
        let page = export_journal_okf(&journal, 0, 10);
        assert_eq!(page.returned, 2);
        assert!(page.files[1].content.contains("status: deprecated"));
        assert!(page.files[2].content.contains("status: stable"));
        assert!(page.files[2].content.contains("sources:\n"));
        assert!(
            page.files[2]
                .content
                .contains("/entries/00000000000000000001.md")
        );
        assert!(
            page.files[2]
                .content
                .contains("The earlier account remains inspectable")
        );
    }

    #[test]
    fn paging_is_bounded_stable_and_complete() {
        let mut journal = Journal::new();
        for entry_id in 1..=MAX_OKF_EXPORT_ENTRIES as u64 + 1 {
            journal
                .record(record(entry_id, "room", "account"))
                .expect("record");
        }
        assert!(journal.entries.len() < MAX_JOURNAL_ENTRIES);
        let first = export_journal_okf(&journal, 0, usize::MAX);
        assert_eq!(first.returned, MAX_OKF_EXPORT_ENTRIES);
        assert!(first.has_more);
        assert_eq!(first.next_after_entry_id, MAX_OKF_EXPORT_ENTRIES as u64);
        let second = export_journal_okf(&journal, first.next_after_entry_id, 10);
        assert_eq!(second.returned, 1);
        assert!(!second.has_more);
        assert_eq!(second.files[1].path, "entries/00000000000000000101.md");
    }
}
