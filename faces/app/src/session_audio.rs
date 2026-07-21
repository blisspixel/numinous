//! Selection ownership for local Watch Agent sound replay.

/// Tracks which retained public event currently owns the local audio source.
#[derive(Default)]
pub(crate) struct SessionAudio {
    active: bool,
    selected_sequence: Option<u64>,
}

impl SessionAudio {
    /// Begin a fresh viewer session. The caller must publish silence once.
    pub(crate) fn begin(&mut self) {
        self.active = true;
        self.selected_sequence = None;
    }

    /// Admit one semantic audio update for a changed public selection.
    pub(crate) fn select(&mut self, public_sequence: Option<u64>) -> bool {
        if !self.active || self.selected_sequence == public_sequence {
            return false;
        }
        self.selected_sequence = public_sequence;
        true
    }

    /// Retire all viewer audio ownership.
    pub(crate) fn end(&mut self) {
        self.active = false;
        self.selected_sequence = None;
    }
}

#[cfg(test)]
mod tests {
    use super::SessionAudio;

    #[test]
    fn selection_changes_emit_once_only_while_the_viewer_owns_audio() {
        let mut audio = SessionAudio::default();
        assert!(!audio.select(Some(0)));

        audio.begin();
        assert!(!audio.select(None));
        assert!(audio.select(Some(0)));
        assert!(!audio.select(Some(0)));
        assert!(audio.select(Some(1)));
        assert!(audio.select(None));
        assert!(!audio.select(None));

        audio.end();
        assert!(!audio.select(Some(2)));
        audio.begin();
        assert!(audio.select(Some(2)));
    }
}
