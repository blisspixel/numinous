use std::time::{Duration, Instant};

const MIN_SAVE_INTERVAL: Duration = Duration::from_millis(500);
/// Short loops encode many frames; keep a longer cooldown so one hold cannot
/// flood the home directory with multi-megabyte APNGs.
const MIN_LOOP_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
pub(crate) enum SaveKind {
    PlaytestNote,
    Postcard,
    ShortLoop,
}

#[derive(Default)]
pub(crate) struct SaveGate {
    playtest_note: Option<Instant>,
    postcard: Option<Instant>,
    short_loop: Option<Instant>,
}

impl SaveGate {
    /// Admit one physical press per save action and bound rapid synthetic
    /// presses. Actions remain independent so a diagnostic note, a still
    /// postcard, and a short loop never block each other.
    pub(crate) fn admit(&mut self, kind: SaveKind, now: Instant, repeated: bool) -> bool {
        if repeated {
            return false;
        }
        let (previous, min_interval) = match kind {
            SaveKind::PlaytestNote => (&mut self.playtest_note, MIN_SAVE_INTERVAL),
            SaveKind::Postcard => (&mut self.postcard, MIN_SAVE_INTERVAL),
            SaveKind::ShortLoop => (&mut self.short_loop, MIN_LOOP_INTERVAL),
        };
        if let Some(elapsed) = previous.and_then(|last| now.checked_duration_since(last)) {
            if elapsed < min_interval {
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
        assert!(gate.admit(SaveKind::ShortLoop, start, false));
        assert!(!gate.admit(SaveKind::PlaytestNote, start, true));
        assert!(!gate.admit(SaveKind::Postcard, start, true));
        assert!(!gate.admit(SaveKind::ShortLoop, start, true));
    }

    #[test]
    fn short_loop_uses_a_longer_cooldown() {
        let start = Instant::now();
        let mut gate = SaveGate::default();

        assert!(gate.admit(SaveKind::ShortLoop, start, false));
        assert!(!gate.admit(SaveKind::ShortLoop, start + MIN_SAVE_INTERVAL, false));
        assert!(gate.admit(SaveKind::ShortLoop, start + MIN_LOOP_INTERVAL, false));
    }

    #[test]
    fn a_backwards_timestamp_cannot_bypass_the_gate() {
        let start = Instant::now();
        let mut gate = SaveGate::default();

        assert!(gate.admit(SaveKind::PlaytestNote, start + MIN_SAVE_INTERVAL, false));
        assert!(!gate.admit(SaveKind::PlaytestNote, start, false));
    }
}
