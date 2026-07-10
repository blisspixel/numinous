//! The high-score table: arcade rules, every game, every mind.
//!
//! Each entry is the best score ever posted for a challenge key (for example
//! `munch seed:7 board:0` or `quiz seed:3 rounds:5`). Keys are chosen by the
//! games so that a challenge means the same thing wherever it is played: a
//! human in the terminal and an agent over MCP posting to the same key are
//! competing on the same board. Pure and deterministic; the faces own the file
//! IO. See `docs/PLAYFUL.md`.

use std::collections::BTreeMap;

const MAX_SCORE_ENTRIES: usize = 4096;
const MAX_SCORE_KEY_LEN: usize = 128;

/// The table: best score per challenge key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scoreboard {
    /// Challenge key to best score.
    pub entries: BTreeMap<String, i64>,
}

impl Scoreboard {
    /// Record `score` for `key`, keeping the best. Returns true when this is a
    /// new record (arcade rules: strictly better than what stood before).
    pub fn record(&mut self, key: &str, score: i64) -> bool {
        let key = key.trim();
        if !score_key_is_sane(key) {
            return false;
        }
        match self.entries.get(key) {
            Some(&best) if score <= best => false,
            None if self.entries.len() >= MAX_SCORE_ENTRIES => false,
            _ => {
                self.entries.insert(key.to_string(), score);
                true
            }
        }
    }

    /// The top `n` entries, best first; ties break alphabetically by key.
    #[must_use]
    pub fn top(&self, n: usize) -> Vec<(&str, i64)> {
        let mut all: Vec<(&str, i64)> =
            self.entries.iter().map(|(k, &v)| (k.as_str(), v)).collect();
        all.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        all.truncate(n);
        all
    }

    /// Serialize to the scores file format: one `score<TAB>key` line each.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (key, score) in &self.entries {
            out.push_str(&format!("{score}\t{key}\n"));
        }
        out
    }

    /// Parse the scores file format; unknown or malformed lines are ignored.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        let mut board = Scoreboard::default();
        for line in text.lines() {
            if let Some((score, key)) = line.split_once('\t')
                && let Ok(score) = score.trim().parse::<i64>()
                && !key.trim().is_empty()
            {
                board.record(key.trim(), score);
            }
        }
        board
    }
}

fn score_key_is_sane(key: &str) -> bool {
    !key.is_empty() && key.len() <= MAX_SCORE_KEY_LEN && !key.contains(['\t', '\n', '\r'])
}

#[cfg(test)]
mod tests {
    use super::Scoreboard;

    #[test]
    fn records_keep_only_the_best() {
        let mut board = Scoreboard::default();
        assert!(board.record("munch seed:7 board:0", 50));
        assert!(
            !board.record("munch seed:7 board:0", 30),
            "worse is not news"
        );
        assert!(
            !board.record("munch seed:7 board:0", 50),
            "a tie is not a record"
        );
        assert!(board.record("munch seed:7 board:0", 80));
        assert_eq!(board.entries["munch seed:7 board:0"], 80);
    }

    #[test]
    fn top_sorts_best_first() {
        let mut board = Scoreboard::default();
        board.record("a", 10);
        board.record("b", 90);
        board.record("c", 50);
        let top = board.top(2);
        assert_eq!(top, vec![("b", 90), ("c", 50)]);
    }

    #[test]
    fn text_round_trips_and_ignores_noise() {
        let mut board = Scoreboard::default();
        board.record("quiz seed:3 rounds:5", 4);
        board.record("munch seed:7 board:1", 130);
        let back = Scoreboard::from_text(&board.to_text());
        assert_eq!(back, board);
        assert_eq!(
            Scoreboard::from_text("garbage\n\t\nx\t\n"),
            Scoreboard::default()
        );
    }

    #[test]
    fn record_rejects_keys_that_can_forge_rows() {
        let mut board = Scoreboard::default();
        assert!(!board.record("munch seed:1\n999\tforged", 10));
        assert!(!board.record("munch\tseed:1", 10));
        assert!(board.entries.is_empty());
    }

    #[test]
    fn record_rejects_oversized_keys_and_caps_unique_entries() {
        let mut board = Scoreboard::default();
        assert!(!board.record(&"x".repeat(super::MAX_SCORE_KEY_LEN + 1), 10));
        assert!(board.entries.is_empty());

        for i in 0..super::MAX_SCORE_ENTRIES {
            assert!(board.record(&format!("key:{i}"), i as i64));
        }
        assert_eq!(board.entries.len(), super::MAX_SCORE_ENTRIES);
        assert!(!board.record("one-more", 1));
        assert!(board.record("key:0", i64::MAX));
        assert_eq!(board.entries["key:0"], i64::MAX);
    }

    #[test]
    fn from_text_bounds_corrupted_score_tables() {
        let text = (0..(super::MAX_SCORE_ENTRIES + 512))
            .map(|i| format!("{i}\tkey:{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let board = Scoreboard::from_text(&text);

        assert_eq!(board.entries.len(), super::MAX_SCORE_ENTRIES);
        let rejected_key = format!("key:{}", super::MAX_SCORE_ENTRIES);
        assert!(!board.entries.contains_key(&rejected_key));
    }
}
