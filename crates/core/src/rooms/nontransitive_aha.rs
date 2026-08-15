//! The Nontransitive Dice engineered aha: choose first, then call the counter.
//!
//! The answer is the complete 6 by 6 outcome space, not a sampled roll. The
//! room therefore keeps tactile random rolls as texture while exact enumeration
//! owns the wager's truth.

use crate::surface::Surface;

use super::nontransitive::{Die, exact_wins, win_rate};

/// Vertical start of the counter wager band.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Morph progress that counts as complete.
pub const MORPH_DONE: f64 = 1.0;

/// Draw the three possible counter calls along the bottom input band.
pub fn render_counter_band(canvas: &mut dyn Surface, hover: Option<Die>) {
    let (width, height) = canvas.draw_bounds();
    if width < 16 || height < 6 {
        return;
    }
    let y = ((height as f64) * 0.92).round() as i32;
    let y = y.clamp(1, height as i32 - 2);
    canvas.line(0, y, width.saturating_sub(1) as i32, y, '-');
    for (index, die) in [Die::A, Die::B, Die::C].iter().enumerate() {
        let x = ((index as f64 + 0.5) / 3.0 * width as f64).round() as i32;
        let mark = if hover == Some(*die) { '#' } else { '+' };
        canvas.line(x, y - 2, x, y + 1, mark);
        canvas.plot(x, y + 2, die.name().chars().next().unwrap_or('?'));
    }
}

/// Draw the exact 36 pairwise outcomes for the counter against the chosen die.
///
/// `W` marks a counter win and `L` a loss. Distinct glyphs keep the answer
/// readable without color, and the progressive fill makes enumeration visible
/// as a finite proof rather than a decorative chart.
pub fn render_outcome_grid(canvas: &mut dyn Surface, progress: f64, chosen: Die) {
    let (width, height) = canvas.draw_bounds();
    if width < 18 || height < 12 {
        return;
    }
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let visible = (progress * 36.0).floor() as usize;
    if visible == 0 {
        return;
    }
    let counter = chosen.counter();
    let cell_width = 2_i32;
    let grid_width = 6 * cell_width;
    let left = ((width as i32 - grid_width) / 2).max(2);
    let top = ((height as i32 - 9) / 2).max(2);
    let heading = if visible == 36 {
        format!("{} vs {}", counter.name(), chosen.name())
    } else {
        format!("? vs {}", chosen.name())
    };
    for (index, ch) in heading.chars().enumerate() {
        canvas.plot(left + index as i32, top - 2, ch);
    }
    for row in 0..6 {
        for column in 0..6 {
            let index = row * 6 + column;
            if index >= visible {
                continue;
            }
            canvas.plot(
                left + column as i32 * cell_width,
                top - 1,
                char::from(b'0' + chosen.faces()[column]),
            );
            canvas.plot(
                left - 2,
                top + row as i32,
                char::from(b'0' + counter.faces()[row]),
            );
            let counter_face = counter.faces()[row];
            let chosen_face = chosen.faces()[column];
            let mark = if counter_face > chosen_face { 'W' } else { 'L' };
            canvas.plot(left + column as i32 * cell_width, top + row as i32, mark);
        }
    }
    if visible == 36 {
        let summary = format!(
            "{} W / {} L",
            exact_wins(counter, chosen),
            36 - exact_wins(counter, chosen)
        );
        for (index, ch) in summary.chars().enumerate() {
            canvas.plot(left + index as i32, top + 7, ch);
        }
    }
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnPath {
    /// A counter was called before the answer appeared.
    Call {
        /// The die chosen to oppose the player's die.
        called: Die,
        /// Whether the call matched the exact counter.
        right: bool,
    },
    /// The player tried enough chosen dice to observe without calling.
    Choices {
        /// Number of completed choices.
        count: usize,
    },
}

/// Staging for the Nontransitive Dice counter wager.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// The untouched trio is available to explore.
    Explore,
    /// A die has been chosen and its counter is invited.
    Prime,
    /// A generation act landed, with the exact outcomes withheld.
    Withheld,
    /// The 36 outcomes arrive.
    Morph {
        /// Morph blend in `[0, 1]`.
        progress: f64,
    },
    /// The exact counter and pairwise count stand together.
    Confirm,
    /// The answer and full room reveal may open.
    Consolidated,
}

/// Pure visit state for the Nontransitive Dice wager.
#[derive(Debug, Clone, PartialEq)]
pub struct NontransitiveAha {
    beat: AhaBeat,
    choices: usize,
    chosen: Option<Die>,
    hover: Option<Die>,
    earn: Option<EarnPath>,
    morph_progress: f64,
}

impl NontransitiveAha {
    /// Begin an untouched visit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beat: AhaBeat::Explore,
            choices: 0,
            chosen: None,
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

    /// The newest die chosen before the wager closed.
    #[must_use]
    pub fn chosen(&self) -> Option<Die> {
        self.chosen
    }

    /// Hovered counter while priming.
    #[must_use]
    pub fn hover(&self) -> Option<Die> {
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

    /// Record completed die choices.
    ///
    /// One choice primes the question. Four choices earn an observation path
    /// for a player who prefers not to predict.
    pub fn note_choices(&mut self, chosen: Option<Die>, count: usize) {
        self.choices = count;
        if self.earn.is_none() && count > 0 {
            self.chosen = chosen;
        }
        if count >= 1 && matches!(self.beat, AhaBeat::Explore) && self.chosen.is_some() {
            self.beat = AhaBeat::Prime;
        }
        if self.earn.is_none() && count >= 4 && self.chosen.is_some() {
            self.earn = Some(EarnPath::Choices { count });
            self.hover = None;
            self.beat = AhaBeat::Withheld;
        }
    }

    /// Number of completed choices in this visit.
    #[must_use]
    pub fn choices(&self) -> usize {
        self.choices
    }

    /// The exact die that counters the chosen die.
    #[must_use]
    pub fn truth(&self) -> Option<Die> {
        self.chosen.map(Die::counter)
    }

    /// Hover a counter while the question is open.
    pub fn set_hover(&mut self, die: Option<Die>) {
        if matches!(self.beat, AhaBeat::Prime) {
            self.hover = die;
        }
    }

    /// Commit a counter call. The first call owns the visit.
    pub fn commit_call(&mut self, called: Die) -> bool {
        if matches!(self.earn, Some(EarnPath::Call { .. }))
            || !matches!(self.beat, AhaBeat::Prime | AhaBeat::Withheld)
        {
            return false;
        }
        let Some(truth) = self.truth() else {
            return false;
        };
        self.earn = Some(EarnPath::Call {
            called,
            right: called == truth,
        });
        self.hover = None;
        self.beat = AhaBeat::Withheld;
        true
    }

    /// The committed counter call, if this path used one.
    #[must_use]
    pub fn call(&self) -> Option<Die> {
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

    /// Whether the exact outcome grid should be drawn.
    #[must_use]
    pub fn uses_outcome_grid(&self) -> bool {
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
            AhaBeat::Explore => room_status.unwrap_or("CHOOSE A DIE").to_string(),
            AhaBeat::Prime => format!(
                "WHAT BEATS {}? 1=A 2=B 3=C",
                self.chosen.map_or("?", Die::name)
            ),
            // The experiment path earns without a call, so it must not borrow
            // the CALLED sentence. Telling a player they called something they
            // never called is the same lie as hiding a call they did make.
            AhaBeat::Withheld => match self.earn {
                Some(EarnPath::Call { called, .. }) => format!(
                    "CALLED {} AGAINST {}  PRESS E",
                    called.name(),
                    self.chosen.map_or("?", Die::name)
                ),
                Some(EarnPath::Choices { count }) => format!(
                    "{count} CHOICES HELD ON {}  PRESS E",
                    self.chosen.map_or("?", Die::name)
                ),
                None => "READY  PRESS E".to_string(),
            },
            AhaBeat::Morph { progress } => {
                format!("36 OUTCOMES {:>3}%", (progress * 100.0).round() as u8)
            }
            AhaBeat::Confirm => {
                let chosen = self.chosen.unwrap_or(Die::A);
                let counter = chosen.counter();
                format!(
                    "{} BEATS {} {}/36  PRESS E",
                    counter.name(),
                    chosen.name(),
                    exact_wins(counter, chosen)
                )
            }
            AhaBeat::Consolidated => self.punchline().to_string(),
        }
    }

    /// One-sentence consolidation of the cycle's strategic meaning.
    #[must_use]
    pub fn punchline(&self) -> &'static str {
        "There is no best die: choosing first lets the other player choose its counter."
    }

    /// Answer the player's exact call against complete enumeration.
    #[must_use]
    pub fn graded(&self) -> Option<String> {
        if !matches!(self.beat, AhaBeat::Consolidated) {
            return None;
        }
        let chosen = self.chosen?;
        let called = self.call()?;
        let truth = chosen.counter();
        let called_wins = exact_wins(called, chosen);
        let truth_wins = exact_wins(truth, chosen);
        let verdict = if called == truth {
            "Nailed."
        } else {
            "The fertile miss: size alone cannot rank these dice."
        };
        Some(format!(
            "You chose {} and called {}; {} wins {called_wins}/36 against {}. The counter is {}, winning {truth_wins}/36 ({:.2}%). {verdict}",
            chosen.name(),
            called.name(),
            called.name(),
            chosen.name(),
            truth.name(),
            win_rate(truth, chosen) * 100.0,
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
            Some(EarnPath::Choices { count }) => Some(format!("choices:{count}")),
            None => None,
        }
    }
}

impl Default for NontransitiveAha {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{AhaBeat, NontransitiveAha, render_outcome_grid};
    use crate::canvas::Canvas;
    use crate::rooms::nontransitive::Die;

    #[test]
    fn wrong_call_walks_all_beats_and_meets_exact_counter() {
        let mut aha = NontransitiveAha::new();
        aha.note_choices(Some(Die::A), 1);
        assert_eq!(aha.beat(), AhaBeat::Prime);
        assert_eq!(aha.truth(), Some(Die::C));
        assert!(aha.commit_call(Die::B));
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert!(!aha.allow_reveal_text());
        assert!(aha.summon());
        aha.advance_morph(0.5);
        assert!(matches!(aha.beat(), AhaBeat::Morph { .. }));
        aha.advance_morph(0.6);
        assert_eq!(aha.beat(), AhaBeat::Confirm);
        assert!(aha.summon());
        let grade = aha.graded().expect("the call is answered");
        assert!(grade.contains("called B"), "{grade}");
        assert!(grade.contains("counter is C"), "{grade}");
        assert!(grade.contains("20/36"), "{grade}");
        assert_eq!(aha.earn_label().as_deref(), Some("call:b:wrong"));
    }

    #[test]
    fn every_chosen_die_has_the_exact_counter() {
        for (chosen, counter, wins) in [
            (Die::A, Die::C, 20),
            (Die::B, Die::A, 24),
            (Die::C, Die::B, 24),
        ] {
            let mut aha = NontransitiveAha::new();
            aha.note_choices(Some(chosen), 1);
            assert_eq!(aha.truth(), Some(counter));
            assert!(aha.commit_call(counter));
            assert!(aha.summon());
            aha.set_morph_progress(1.0);
            assert!(aha.summon());
            let grade = aha.graded().expect("right call is graded");
            assert!(grade.contains(&format!("{wins}/36")), "{grade}");
            assert!(grade.contains("Nailed"), "{grade}");
        }
    }

    #[test]
    fn four_choices_earn_observation_without_forcing_a_call() {
        let mut aha = NontransitiveAha::new();
        aha.note_choices(Some(Die::B), 4);
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert_eq!(aha.earn_label().as_deref(), Some("choices:4"));
        assert!(aha.summon());
    }

    #[test]
    fn outcome_grid_arrives_progressively_without_color() {
        let mut none = Canvas::new(48, 24);
        render_outcome_grid(&mut none, 0.0, Die::A);
        assert_eq!(none.ink_count(), 0);
        let mut early = Canvas::new(48, 24);
        render_outcome_grid(&mut early, 0.25, Die::A);
        let mut full = Canvas::new(48, 24);
        render_outcome_grid(&mut full, 1.0, Die::A);
        assert!(early.ink_count() > 0);
        assert!(full.ink_count() > early.ink_count());
        let text = full.to_text();
        assert!(text.contains("C vs A"));
        assert!(text.contains("20 W / 16 L"));
        assert!(text.contains('W') && text.contains('L'));
    }

    #[test]
    fn hostile_progress_and_tiny_canvases_are_safe() {
        let mut tiny = Canvas::new(2, 2);
        render_outcome_grid(&mut tiny, f64::NAN, Die::C);
        assert_eq!(tiny.ink_count(), 0);
        let mut aha = NontransitiveAha::new();
        assert!(!aha.commit_call(Die::A));
        aha.note_choices(None, 1);
        assert_eq!(aha.beat(), AhaBeat::Explore);
        aha.note_choices(Some(Die::C), 1);
        assert!(aha.commit_call(Die::B));
        assert!(aha.summon());
        aha.advance_morph(f64::INFINITY);
        assert!(matches!(aha.beat(), AhaBeat::Morph { progress: 0.0 }));
    }
}
