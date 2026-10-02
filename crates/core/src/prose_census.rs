//! What the current documents say, for the locks that hold prose to live data.
//!
//! A positive lock asks that a document contain the right count, and that
//! alone never fails on a wrong one. That is how "41 tools" and "354 rooms"
//! outlived the inventory and the catalog they described: the right phrase
//! was present elsewhere, so nothing noticed the stale one. This census reads
//! every current Markdown document and returns every number stated in a
//! locked phrase, so a lock can require each of them to be the live value.
//!
//! History is not current prose. The changelog, the published release notes,
//! the packaged playtest records, committed evidence, and the frozen roadmap
//! ledger record what was true on their dates, and are not read.
//!
//! The faces include this file by path for their own locks (the tool
//! inventory lives in the MCP face, the screen matrix in the App's examples),
//! so it stays std-only and names nothing from the crate that includes it.

use std::path::Path;

/// Documents that record history rather than state the present, relative to
/// the repository root with `/` separators. A path ending in `/` names a
/// whole directory.
const HISTORY: &[&str] = &[
    "CHANGELOG.md",
    "docs/releases/",
    "docs/PLAYTESTS.md",
    "docs/evidence/",
    "docs/history/",
];

/// The roadmap section that is not read yet, and why.
///
/// On this revision its entries are being revised one by one, and two still
/// quote counts from the day they were written ("all 354 rooms" in entry 2,
/// "41 tools" in entry 7). When those are restated, delete this exception so
/// the section is held to the same lock as the rest of the roadmap.
const UNREAD_ROADMAP_SECTION: &str = "### Decisions the am-track is waiting on";

/// One current Markdown document: its path from the repository root and its
/// text.
pub struct Document {
    /// The path from the repository root, with `/` separators.
    pub path: String,
    /// The document's text.
    pub text: String,
}

/// Every current Markdown document under `root`: the root's own files and
/// everything under `docs`, `plugins`, and `assets`, less [`HISTORY`].
///
/// Panics when the walk finds neither the README nor the roadmap, because a
/// census that read nothing would pass every lock.
pub fn current_documents(root: &Path) -> Vec<Document> {
    let mut documents = Vec::new();
    collect(root, root, false, &mut documents);
    for tree in ["docs", "plugins", "assets"] {
        collect(root, &root.join(tree), true, &mut documents);
    }
    documents.sort_by(|left, right| left.path.cmp(&right.path));
    for anchor in ["README.md", "docs/ROADMAP.md"] {
        assert!(
            documents.iter().any(|document| document.path == anchor),
            "the census found no {anchor} under {}",
            root.display()
        );
    }
    documents
}

fn collect(root: &Path, directory: &Path, recurse: bool, documents: &mut Vec<Document>) {
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
    for entry in entries {
        let path = entry.expect("a directory entry is readable").path();
        let relative = path
            .strip_prefix(root)
            .expect("the walk stays under the root")
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            if recurse && !HISTORY.contains(&format!("{relative}/").as_str()) {
                collect(root, &path, true, documents);
            }
            continue;
        }
        if !relative.ends_with(".md") || HISTORY.contains(&relative.as_str()) {
            continue;
        }
        let mut text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {relative}: {error}"));
        if relative == "docs/ROADMAP.md" {
            text = without_unread_section(&text);
        }
        documents.push(Document {
            path: relative,
            text,
        });
    }
}

/// The roadmap with [`UNREAD_ROADMAP_SECTION`] removed, up to the next
/// heading of the same level.
pub fn without_unread_section(text: &str) -> String {
    let Some((before, rest)) = text.split_once(UNREAD_ROADMAP_SECTION) else {
        return text.to_string();
    };
    match rest.split_once("\n### ") {
        Some((_, after)) => format!("{before}### {after}"),
        None => before.to_string(),
    }
}

/// Every place a document states a number in `pattern` other than `live`.
///
/// `pattern` is a phrase with `#` standing for a whole number, such as
/// `"# catalog room"` or `"all # rooms"`. Words match without regard to case,
/// surrounding punctuation, Markdown emphasis, or line breaks, and a final
/// `s` is allowed on any word, so `"# catalog room"` matches "356 catalog
/// rooms" and "1 catalog room" alike. Returns the misstatements, each with its
/// document and context, and how many statements matched at all, so a caller
/// can refuse a lock that matches nothing.
pub fn misstated(documents: &[Document], pattern: &str, live: usize) -> (Vec<String>, usize) {
    let wanted: Vec<String> = pattern.split_whitespace().map(str::to_lowercase).collect();
    let mut misstatements = Vec::new();
    let mut matched = 0;
    for document in documents {
        let words: Vec<&str> = document.text.split_whitespace().collect();
        let plain: Vec<String> = words.iter().map(|word| plain_word(word)).collect();
        for start in 0..plain.len() {
            let Some(count) = stated_at(&plain[start..], &wanted) else {
                continue;
            };
            matched += 1;
            if count != live {
                let from = start.saturating_sub(4);
                let to = (start + wanted.len() + 4).min(words.len());
                misstatements.push(format!(
                    "{}: states {count} where the live count is {live}: \"...{}...\"",
                    document.path,
                    words[from..to].join(" ")
                ));
            }
        }
    }
    (misstatements, matched)
}

/// A word with its surrounding punctuation and emphasis removed, lowercased.
/// Commas inside a number are dropped, so "3,065" reads as 3065.
fn plain_word(word: &str) -> String {
    word.trim_matches(|character: char| !character.is_alphanumeric())
        .replace(',', "")
        .to_lowercase()
}

/// The number stated at the start of `words` when they follow `wanted`.
fn stated_at(words: &[String], wanted: &[String]) -> Option<usize> {
    if words.len() < wanted.len() {
        return None;
    }
    let mut count = None;
    for (word, expected) in words.iter().zip(wanted) {
        if expected == "#" {
            if word.is_empty() || !word.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            count = word.parse().ok();
        } else if word != expected && word.strip_suffix('s') != Some(expected.as_str()) {
            return None;
        }
    }
    count
}
