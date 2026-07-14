use std::time::{Duration, Instant};

const MIN_SAVE_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy)]
pub(crate) enum SaveKind {
    PlaytestNote,
    Postcard,
}

#[derive(Default)]
pub(crate) struct SaveGate {
    playtest_note: Option<Instant>,
    postcard: Option<Instant>,
}

impl SaveGate {
    /// Admit one physical press per save action and bound rapid synthetic
    /// presses. The two actions remain independent so saving a diagnostic note
    /// never prevents the player from keeping a postcard.
    pub(crate) fn admit(&mut self, kind: SaveKind, now: Instant, repeated: bool) -> bool {
        if repeated {
            return false;
        }
        let previous = match kind {
            SaveKind::PlaytestNote => &mut self.playtest_note,
            SaveKind::Postcard => &mut self.postcard,
        };
        if let Some(elapsed) = previous.and_then(|last| now.checked_duration_since(last)) {
            if elapsed < MIN_SAVE_INTERVAL {
                return false;
            }
        } else if previous.is_some() {
            return false;
        }
        *previous = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_and_rapid_save_events_are_bounded() {
        let start = Instant::now();
        let mut gate = SaveGate::default();

        assert!(gate.admit(SaveKind::Postcard, start, false));
        assert!(!gate.admit(SaveKind::Postcard, start + Duration::from_millis(1), true));
        assert!(!gate.admit(
            SaveKind::Postcard,
            start + MIN_SAVE_INTERVAL - Duration::from_millis(1),
            false
        ));
        assert!(gate.admit(SaveKind::Postcard, start + MIN_SAVE_INTERVAL, false));
    }

    #[test]
    fn save_actions_have_independent_budgets() {
        let start = Instant::now();
        let mut gate = SaveGate::default();

        assert!(gate.admit(SaveKind::PlaytestNote, start, false));
        assert!(gate.admit(SaveKind::Postcard, start, false));
        assert!(!gate.admit(SaveKind::PlaytestNote, start, true));
        assert!(!gate.admit(SaveKind::Postcard, start, true));
    }

    #[test]
    fn a_backwards_timestamp_cannot_bypass_the_gate() {
        let start = Instant::now();
        let mut gate = SaveGate::default();

        assert!(gate.admit(SaveKind::PlaytestNote, start + MIN_SAVE_INTERVAL, false));
        assert!(!gate.admit(SaveKind::PlaytestNote, start, false));
    }
}
