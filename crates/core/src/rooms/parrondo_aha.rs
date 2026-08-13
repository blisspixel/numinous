//! The Parrondo engineered aha: call which policy wins in expectation.
//!
//! The room keeps its sampled walks for play, but the wager is answered by an
//! exact Markov expectation. This separates mathematical truth from variance.

use crate::surface::Surface;

use super::parrondo::{DEMONSTRATION_STEPS, Policy, expected_end, expected_path};

/// Vertical start of the wager band, shared with the other staged rooms.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Morph progress that counts as complete.
pub const MORPH_DONE: f64 = 1.0;

/// Draw the three policy calls along the bottom input band.
pub fn render_policy_band(canvas: &mut dyn Surface, hover: Option<Policy>) {
    let (width, height) = canvas.draw_bounds();
    if width < 16 || height < 6 {
        return;
    }
    let y = ((height as f64) * 0.92).round() as i32;
    let y = y.clamp(1, height as i32 - 2);
    canvas.line(0, y, width.saturating_sub(1) as i32, y, '-');
    for (index, policy) in [Policy::OnlyA, Policy::OnlyB, Policy::CycleAbb]
        .iter()
        .enumerate()
    {
        let x = ((index as f64 + 0.5) / 3.0 * width as f64).round() as i32;
        let mark = if hover == Some(*policy) { '#' } else { '+' };
        canvas.line(x, y - 2, x, y + 1, mark);
        let label = match policy {
            Policy::OnlyA => 'A',
            Policy::OnlyB => 'B',
            Policy::CycleAbb => 'O',
        };
        canvas.plot(x, y + 2, label);
    }
}

/// Draw exact expected-capital paths as the answer arrives.
///
/// A, B, and O mark the A-only, B-only, and ABB paths. The distinct glyphs
/// keep the comparison readable without relying on color.
pub fn render_expectation_overlay(canvas: &mut dyn Surface, progress: f64) {
    let (width, height) = canvas.draw_bounds();
    if width < 12 || height < 8 {
        return;
    }
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let visible = (progress * DEMONSTRATION_STEPS as f64).floor() as usize;
    if visible == 0 {
        return;
    }
    let paths = [
        (Policy::OnlyA, 'A'),
        (Policy::OnlyB, 'B'),
        (Policy::CycleAbb, 'O'),
    ]
    .map(|(policy, mark)| (expected_path(policy, DEMONSTRATION_STEPS), mark));
    let min = paths
        .iter()
        .flat_map(|(path, _)| path.iter().copied())
        .fold(0.0_f64, f64::min);
    let max = paths
        .iter()
        .flat_map(|(path, _)| path.iter().copied())
        .fold(0.0_f64, f64::max);
    let span = (max - min).max(f64::EPSILON);
    let plot_height = height.saturating_sub(4).max(1);
    let map_point = |turn: usize, capital: f64| {
        let x = (turn as f64 / DEMONSTRATION_STEPS as f64 * width.saturating_sub(1) as f64).round()
            as i32;
        let y = ((1.0 - (capital - min) / span) * plot_height.saturating_sub(1) as f64).round()
            as i32
            + 2;
        (x, y)
    };
    let zero_y = map_point(0, 0.0).1;
    canvas.line(0, zero_y, width.saturating_sub(1) as i32, zero_y, '-');
    canvas.plot(1, 0, 'A');
    canvas.plot(3, 0, 'B');
    canvas.plot(5, 0, 'O');
    for (path, mark) in paths {
        let mut previous = map_point(0, path[0]);
        for (turn, capital) in path.iter().copied().enumerate().take(visible + 1).skip(1) {
            let point = map_point(turn, capital);
            canvas.line(previous.0, previous.1, point.0, point.1, mark);
            previous = point;
        }
    }
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnPath {
    /// A policy was called before the answer appeared.
    Call {
        /// What the player called.
        called: Policy,
        /// Whether the call matched the exact expectation.
        right: bool,
    },
    /// The player tried enough policies to observe without calling.
    Selections {
        /// Number of completed selections.
        count: usize,
    },
}

/// Staging for the Parrondo policy wager.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// The untouched random walk is available to explore.
    Explore,
    /// A policy has been tried and the prediction is invited.
    Prime,
    /// A generation act landed, with the answer still withheld.
    Withheld,
    /// Exact expectation paths arrive.
    Morph {
        /// Morph blend in `[0, 1]`.
        progress: f64,
    },
    /// All three expected paths stand together.
    Confirm,
    /// The answer and full room reveal may open.
    Consolidated,
}

/// Pure visit state for the Parrondo policy wager.
#[derive(Debug, Clone, PartialEq)]
pub struct ParrondoAha {
    beat: AhaBeat,
    selections: usize,
    hover: Option<Policy>,
    earn: Option<EarnPath>,
    morph_progress: f64,
}

impl ParrondoAha {
    /// Begin an untouched visit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beat: AhaBeat::Explore,
            selections: 0,
            hover: None,
            earn: None,
            morph_progress: 0.0,
        }
    }

    /// Current beat.
    #[must_use]
    pub fn beat(&self) -> AhaBeat {
        self.beat
    }

    /// Hovered policy while priming.
    #[must_use]
    pub fn hover(&self) -> Option<Policy> {
        self.hover
    }

    /// Generation path once one exists.
    #[must_use]
    pub fn earn(&self) -> Option<EarnPath> {
        self.earn
    }

    /// Whether a generation act has landed.
    #[must_use]
    pub fn earned(&self) -> bool {
        self.earn.is_some()
    }

    /// The exact winning policy among the offered choices.
    #[must_use]
    pub fn truth(&self) -> Policy {
        Policy::CycleAbb
    }

    /// Record completed room policy selections.
    ///
    /// One selection primes the question. Four selections earn an observation
    /// path for a player who prefers not to make a prediction.
    pub fn note_selections(&mut self, count: usize) {
        self.selections = count;
        if count >= 1 && matches!(self.beat, AhaBeat::Explore) {
            self.beat = AhaBeat::Prime;
        }
        if self.earn.is_none() && count >= 4 {
            self.earn = Some(EarnPath::Selections { count });
            self.hover = None;
            self.beat = AhaBeat::Withheld;
        }
    }

    /// Number of completed selections in this visit.
    #[must_use]
    pub fn selections(&self) -> usize {
        self.selections
    }

    /// Hover a call while the question is open.
    pub fn set_hover(&mut self, policy: Option<Policy>) {
        if matches!(self.beat, AhaBeat::Prime) {
            self.hover = policy;
        }
    }

    /// Commit a policy call. The first call owns the visit.
    pub fn commit_call(&mut self, called: Policy) -> bool {
        if matches!(self.earn, Some(EarnPath::Call { .. }))
            || !matches!(self.beat, AhaBeat::Prime | AhaBeat::Withheld)
        {
            return false;
        }
        let right = called == self.truth();
        self.earn = Some(EarnPath::Call { called, right });
        self.hover = None;
        self.beat = AhaBeat::Withheld;
        true
    }

    /// The committed policy call, if this path used one.
    #[must_use]
    pub fn call(&self) -> Option<Policy> {
        match self.earn {
            Some(EarnPath::Call { called, .. }) => Some(called),
            _ => None,
        }
    }

    /// Full reveal text may open only after consolidation.
    #[must_use]
    pub fn allow_reveal_text(&self) -> bool {
        matches!(self.beat, AhaBeat::Consolidated)
    }

    /// Whether Inspect or `aha_summon` can advance the current beat.
    #[must_use]
    pub fn can_summon(&self) -> bool {
        matches!(self.beat, AhaBeat::Withheld | AhaBeat::Confirm)
    }

    /// Whether exact expectation paths should be drawn.
    #[must_use]
    pub fn uses_expectation_overlay(&self) -> bool {
        matches!(
            self.beat,
            AhaBeat::Morph { .. } | AhaBeat::Confirm | AhaBeat::Consolidated
        )
    }

    /// Advance from withheld to morph, or confirm to consolidation.
    pub fn summon(&mut self) -> bool {
        match self.beat {
            AhaBeat::Withheld if self.earned() => {
                self.morph_progress = 0.0;
                self.beat = AhaBeat::Morph { progress: 0.0 };
                true
            }
            AhaBeat::Confirm => {
                self.beat = AhaBeat::Consolidated;
                true
            }
            _ => false,
        }
    }

    /// Set morph progress, clamped to `[0, 1]`.
    pub fn set_morph_progress(&mut self, progress: f64) {
        if !matches!(self.beat, AhaBeat::Morph { .. }) {
            return;
        }
        let progress = if progress.is_finite() {
            progress.clamp(0.0, MORPH_DONE)
        } else {
            0.0
        };
        self.morph_progress = progress;
        if progress >= MORPH_DONE {
            self.beat = AhaBeat::Confirm;
        } else {
            self.beat = AhaBeat::Morph { progress };
        }
    }

    /// Advance morph by a nonnegative face-owned time delta.
    pub fn advance_morph(&mut self, delta: f64) {
        if !matches!(self.beat, AhaBeat::Morph { .. }) {
            return;
        }
        let delta = if delta.is_finite() {
            delta.max(0.0)
        } else {
            0.0
        };
        self.set_morph_progress(self.morph_progress + delta);
    }

    /// Compact footer status for all beats.
    #[must_use]
    pub fn status(&self, room_status: Option<&str>) -> String {
        match self.beat {
            AhaBeat::Explore => room_status.unwrap_or("DRAG:RULE").to_string(),
            AhaBeat::Prime => "WHICH WINS? 1=A 2=B 3=ABB".to_string(),
            AhaBeat::Withheld => format!(
                "CALLED {}  PRESS E",
                self.call().map_or("EXPERIMENT", Policy::name)
            ),
            AhaBeat::Morph { progress } => {
                format!("EXACT EXPECTATION {:>3}%", (progress * 100.0).round() as u8)
            }
            AhaBeat::Confirm => "A LOSES  B LOSES  ABB WINS  PRESS E".to_string(),
            AhaBeat::Consolidated => self.punchline().to_string(),
        }
    }

    /// One-sentence consolidation of the paradox.
    #[must_use]
    pub fn punchline(&self) -> &'static str {
        "A shifts the residues, so ABB meets B's bad coin less often."
    }

    /// Exact expected end capital for an offered policy.
    #[must_use]
    pub fn expected_end(&self, policy: Policy) -> f64 {
        expected_end(policy)
    }

    /// Answer the player's exact call against the Markov expectation.
    #[must_use]
    pub fn graded(&self) -> Option<String> {
        if !matches!(self.beat, AhaBeat::Consolidated) {
            return None;
        }
        let called = self.call()?;
        let truth = self.truth();
        let verdict = if called == truth {
            "Nailed."
        } else {
            "The fertile miss: the schedule changes which residues meet game B."
        };
        Some(format!(
            "You called {}; after {DEMONSTRATION_STEPS} turns, exact expected capital is A {:+.2}, B {:+.2}, ABB {:+.2}. The winner is {}. {verdict}",
            called.name(),
            expected_end(Policy::OnlyA),
            expected_end(Policy::OnlyB),
            expected_end(Policy::CycleAbb),
            truth.name(),
        ))
    }

    /// Stable beat name for structured results and playtest notes.
    #[must_use]
    pub fn beat_label(&self) -> &'static str {
        match self.beat {
            AhaBeat::Explore => "explore",
            AhaBeat::Prime => "prime",
            AhaBeat::Withheld => "withheld",
            AhaBeat::Morph { .. } => "morph",
            AhaBeat::Confirm => "confirm",
            AhaBeat::Consolidated => "consolidated",
        }
    }

    /// Compact generation path for diagnostics.
    #[must_use]
    pub fn earn_label(&self) -> Option<String> {
        match self.earn {
            Some(EarnPath::Call { called, right }) => Some(format!(
                "call:{}:{}",
                called.name().to_ascii_lowercase(),
                if right { "right" } else { "wrong" }
            )),
            Some(EarnPath::Selections { count }) => Some(format!("selections:{count}")),
            None => None,
        }
    }
}

impl Default for ParrondoAha {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{AhaBeat, ParrondoAha, render_expectation_overlay};
    use crate::canvas::Canvas;
    use crate::rooms::parrondo::Policy;

    #[test]
    fn wrong_call_walks_all_beats_and_meets_exact_truth() {
        let mut aha = ParrondoAha::new();
        aha.note_selections(1);
        assert_eq!(aha.beat(), AhaBeat::Prime);
        assert!(aha.commit_call(Policy::OnlyA));
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert!(!aha.allow_reveal_text());
        assert!(aha.summon());
        aha.advance_morph(0.5);
        assert!(matches!(aha.beat(), AhaBeat::Morph { .. }));
        aha.advance_morph(0.6);
        assert_eq!(aha.beat(), AhaBeat::Confirm);
        assert!(aha.summon());
        assert!(aha.allow_reveal_text());
        let grade = aha.graded().expect("the call is answered");
        assert!(grade.contains("called A"), "{grade}");
        assert!(grade.contains("winner is ABB"), "{grade}");
        assert_eq!(aha.earn_label().as_deref(), Some("call:a:wrong"));
    }

    #[test]
    fn four_selections_earn_observation_without_forcing_a_call() {
        let mut aha = ParrondoAha::new();
        aha.note_selections(4);
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert_eq!(aha.earn_label().as_deref(), Some("selections:4"));
        assert!(aha.summon());
    }

    #[test]
    fn expectation_paths_arrive_progressively_without_color() {
        let mut none = Canvas::new(72, 30);
        render_expectation_overlay(&mut none, 0.0);
        assert_eq!(none.ink_count(), 0);
        let mut early = Canvas::new(72, 30);
        render_expectation_overlay(&mut early, 0.25);
        let mut full = Canvas::new(72, 30);
        render_expectation_overlay(&mut full, 1.0);
        assert!(early.ink_count() > 0);
        assert!(full.ink_count() >= early.ink_count());
        let text = full.to_text();
        assert!(text.contains('A') && text.contains('B') && text.contains('O'));
    }

    #[test]
    fn hostile_progress_and_tiny_canvases_are_safe() {
        let mut tiny = Canvas::new(2, 2);
        render_expectation_overlay(&mut tiny, f64::NAN);
        assert_eq!(tiny.ink_count(), 0);
        let mut aha = ParrondoAha::new();
        aha.note_selections(1);
        assert!(aha.commit_call(Policy::CycleAbb));
        assert!(aha.summon());
        aha.advance_morph(f64::INFINITY);
        assert!(matches!(aha.beat(), AhaBeat::Morph { progress: 0.0 }));
    }
}
