//! Buffon's Needle five-beat engineered aha: guess what number the crossings
//! settle on, then watch a circle grow out of a floor with no circles.
//!
//! Pure visit state for the second room on the Exceptional Path generation-
//! before-reveal pattern (Times Tables was first). Faces own wall-clock morph
//! progress; this module never reads a clock. See `docs/PEDAGOGY.md`.

use std::f64::consts::PI;

use crate::surface::Surface;

/// Bottom band for the number wager; throws stay above it.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Morph progress at or above this value completes the restructure beat.
pub const MORPH_DONE: f64 = 1.0;
/// Inclusive low end of the guess number line.
pub const GUESS_MIN: f64 = 1.5;
/// Inclusive high end of the guess number line.
pub const GUESS_MAX: f64 = 4.5;
/// Throws that open the prime invite (one real generation act).
pub const MIN_THROWS_TO_PRIME: usize = 1;
/// Alternate earn: enough player throws without a place-number wager.
pub const MIN_THROWS_TO_EARN: usize = 8;
/// Absolute error vs pi that counts as nailed.
const NAILED_ABS: f64 = 0.08;
/// Absolute error vs pi that counts as close (fertile band).
const CLOSE_ABS: f64 = 0.25;
const CIRCLE_STEPS: usize = 160;

/// Learning-progress band for a committed number guess against pi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessBand {
    /// Within `NAILED_ABS` of pi.
    Nailed,
    /// Within `CLOSE_ABS` of pi.
    Close,
    /// Further than that.
    Wild,
}

impl GuessBand {
    /// Compact spoken name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Nailed => "NAILED",
            Self::Close => "CLOSE",
            Self::Wild => "WILD",
        }
    }

    /// Grade a finite guess against pi.
    #[must_use]
    pub fn grade(guess: f64) -> Self {
        if !guess.is_finite() {
            return Self::Wild;
        }
        let err = (guess - PI).abs();
        if err <= NAILED_ABS {
            Self::Nailed
        } else if err <= CLOSE_ABS {
            Self::Close
        } else {
            Self::Wild
        }
    }
}

/// Map unit x in `[0, 1]` onto the guess number line.
#[must_use]
pub fn guess_from_unit_x(x: f64) -> f64 {
    let x = if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    };
    GUESS_MIN + x * (GUESS_MAX - GUESS_MIN)
}

/// Keyboard shortcuts for common wrong (and rare right) guesses.
#[must_use]
pub fn guess_from_key_digit(digit: u8) -> Option<f64> {
    match digit {
        1 => Some(2.0),
        2 => Some(std::f64::consts::E),
        3 => Some(3.0),
        4 => Some(PI),
        _ => None,
    }
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EarnPath {
    /// Number wager committed (right or wrong).
    Wager {
        /// The player's guess on the number line.
        guess: f64,
        /// Band relative to pi at commit time.
        band: GuessBand,
    },
    /// Enough player throws to have run the experiment without a wager.
    Throws {
        /// How many throws earned the path.
        count: usize,
    },
}

/// Staging for the engineered aha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// Free throws before the gap invite.
    Explore,
    /// At least one throw; invite a number guess.
    Prime,
    /// Generation complete; reveal withheld until summon.
    Withheld,
    /// Circle grows out of the needle floor (progress 0..1).
    Morph {
        /// Morph blend, clamped to `[0, 1]`.
        progress: f64,
    },
    /// Throws continue; full circle stays as the hiding place of pi.
    Confirm,
    /// Punchline available; full reveal text may open.
    Consolidated,
}

/// Pure visit state for the Buffon engineered aha.
#[derive(Debug, Clone, PartialEq)]
pub struct BuffonAha {
    beat: AhaBeat,
    throws: usize,
    hover: Option<f64>,
    earn: Option<EarnPath>,
    morph_progress: f64,
}

impl Default for BuffonAha {
    fn default() -> Self {
        Self::new()
    }
}

impl BuffonAha {
    /// A fresh visit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beat: AhaBeat::Explore,
            throws: 0,
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

    /// Player throws observed this visit.
    #[must_use]
    pub fn throws(&self) -> usize {
        self.throws
    }

    /// Hovered guess while priming.
    #[must_use]
    pub fn hover(&self) -> Option<f64> {
        self.hover
    }

    /// Earn path once generation has completed.
    #[must_use]
    pub fn earn(&self) -> Option<EarnPath> {
        self.earn
    }

    /// Morph progress in `[0, 1]`.
    #[must_use]
    pub fn morph_progress(&self) -> f64 {
        self.morph_progress
    }

    /// Generation is complete.
    #[must_use]
    pub fn earned(&self) -> bool {
        self.earn.is_some()
    }

    /// Full reveal text may open only after the morph has consolidated.
    #[must_use]
    pub fn allow_reveal_text(&self) -> bool {
        matches!(self.beat, AhaBeat::Consolidated)
    }

    /// Summon advances withheld into morph, or confirm into punchline.
    #[must_use]
    pub fn can_summon(&self) -> bool {
        matches!(self.beat, AhaBeat::Withheld | AhaBeat::Confirm)
    }

    /// Whether the visit should draw the circle morph/confirm overlay.
    #[must_use]
    pub fn uses_circle_overlay(&self) -> bool {
        matches!(
            self.beat,
            AhaBeat::Morph { .. } | AhaBeat::Confirm | AhaBeat::Consolidated
        )
    }

    /// Note the current throw count from the room's own input grading.
    pub fn note_throws(&mut self, throws: usize) {
        self.throws = throws;
        if throws >= MIN_THROWS_TO_PRIME && matches!(self.beat, AhaBeat::Explore) {
            self.beat = AhaBeat::Prime;
        }
        if self.earn.is_none() && throws >= MIN_THROWS_TO_EARN {
            self.earn = Some(EarnPath::Throws { count: throws });
            self.hover = None;
            self.beat = AhaBeat::Withheld;
        }
    }

    /// Hover a number on the guess line (Prime only).
    pub fn set_hover(&mut self, guess: Option<f64>) {
        if !matches!(self.beat, AhaBeat::Prime) {
            return;
        }
        self.hover = guess.and_then(|g| g.is_finite().then_some(g.clamp(GUESS_MIN, GUESS_MAX)));
    }

    /// Commit the number wager. First generation act wins.
    pub fn commit_wager(&mut self, guess: f64) -> bool {
        if self.earn.is_some() {
            return false;
        }
        if !matches!(self.beat, AhaBeat::Prime | AhaBeat::Explore) {
            return false;
        }
        if !guess.is_finite() {
            return false;
        }
        let guess = guess.clamp(GUESS_MIN, GUESS_MAX);
        let band = GuessBand::grade(guess);
        self.earn = Some(EarnPath::Wager { guess, band });
        self.hover = None;
        self.beat = AhaBeat::Withheld;
        true
    }

    /// Summon the next staged beat after generation.
    pub fn summon(&mut self) -> bool {
        match self.beat {
            AhaBeat::Withheld if self.earn.is_some() => {
                self.morph_progress = 0.0;
                self.beat = AhaBeat::Morph { progress: 0.0 };
                true
            }
            AhaBeat::Confirm => {
                self.beat = AhaBeat::Consolidated;
                true
            }
            AhaBeat::Morph { progress } if progress >= MORPH_DONE - 1e-9 => {
                self.morph_progress = 1.0;
                self.beat = AhaBeat::Confirm;
                true
            }
            _ => false,
        }
    }

    /// Face-driven morph progress. Completing the blend enters Confirm.
    pub fn set_morph_progress(&mut self, progress: f64) {
        if !matches!(self.beat, AhaBeat::Morph { .. }) {
            return;
        }
        let progress = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.morph_progress = progress;
        if progress >= MORPH_DONE - 1e-9 {
            self.morph_progress = 1.0;
            self.beat = AhaBeat::Confirm;
        } else {
            self.beat = AhaBeat::Morph { progress };
        }
    }

    /// Advance morph by a non-negative delta (faces convert wall time).
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

    /// Compact status for the footer.
    #[must_use]
    pub fn status(&self, throw_status: Option<&str>) -> String {
        match self.beat {
            AhaBeat::Explore => throw_status
                .unwrap_or("CLICK: THROW  NO CIRCLES")
                .to_string(),
            AhaBeat::Prime => {
                let hover = self.hover.map(|g| format!(" >{g:.2}")).unwrap_or_default();
                format!("GUESS 1.5-4.5{hover}")
            }
            AhaBeat::Withheld => match self.earn {
                Some(EarnPath::Wager { guess, band }) => {
                    format!("EARNED {:.2} {}  E", guess, band.name())
                }
                Some(EarnPath::Throws { count }) => {
                    format!("EARNED {count} THROW  PRESS E")
                }
                None => "EARNED  PRESS E".to_string(),
            },
            AhaBeat::Morph { progress } => {
                let pct = (progress * 100.0).round() as i32;
                format!("CIRCLE {pct}%")
            }
            AhaBeat::Confirm => match throw_status {
                Some(s) => format!("PI HIDES  {s}"),
                None => "PI HIDES  THROW MORE".to_string(),
            },
            AhaBeat::Consolidated => match throw_status {
                Some(s) => format!("PI FROM STICKS  {s}"),
                None => "PI FROM STICKS".to_string(),
            },
        }
    }

    /// Punchline once consolidated.
    #[must_use]
    pub fn punchline(&self) -> Option<&'static str> {
        matches!(self.beat, AhaBeat::Consolidated)
            .then_some("No circle on the floor, yet the crossings settle on pi.")
    }
}

/// Draw a growing circle over the plate (morph / confirm overlay).
///
/// `progress` 0 is a point; 1 is a full readable circle. Does not clear the
/// underlying needle floor: callers render the room first, then this overlay.
pub fn render_circle_overlay(canvas: &mut dyn Surface, progress: f64) {
    let (width, height) = canvas.draw_bounds();
    if width < 4 || height < 4 {
        return;
    }
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if progress < 0.02 {
        return;
    }
    let aspect = canvas.char_aspect().max(0.25);
    let cx = (width.saturating_sub(1) as f64) / 2.0;
    let cy = (height.saturating_sub(1) as f64) / 2.0;
    let max_r = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.42;
    let radius = max_r * progress;
    let mark = if progress < 0.55 { '#' } else { '@' };
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=CIRCLE_STEPS {
        let th = std::f64::consts::TAU * (i as f64 / CIRCLE_STEPS as f64);
        let x = (cx + radius * th.cos()).round() as i32;
        let y = (cy + radius * th.sin() * aspect).round() as i32;
        if let Some((px, py)) = prev {
            canvas.line(px, py, x, y, mark);
        }
        prev = Some((x, y));
    }
}

/// Draw the number-line wager band (Prime only).
pub fn render_guess_band(canvas: &mut dyn Surface, hover: Option<f64>) {
    let (width, height) = canvas.draw_bounds();
    if width < 16 || height < 6 {
        return;
    }
    let y = (height as f64 * 0.92).round() as i32;
    let y = y.clamp(1, height as i32 - 2);
    let left = (width as f64 * 0.08).round() as i32;
    let right = (width as f64 * 0.92).round() as i32;
    canvas.line(left, y, right, y, '-');
    // Tick marks near 2, e, 3, pi for language-light anchors.
    for value in [2.0_f64, std::f64::consts::E, 3.0, PI] {
        let phase = ((value - GUESS_MIN) / (GUESS_MAX - GUESS_MIN)).clamp(0.0, 1.0);
        let x = left as f64 + (right - left) as f64 * phase;
        let x = x.round() as i32;
        canvas.line(x, y - 2, x, y + 2, '*');
    }
    if let Some(guess) = hover.filter(|g| g.is_finite()) {
        let phase = ((guess - GUESS_MIN) / (GUESS_MAX - GUESS_MIN)).clamp(0.0, 1.0);
        let x = left as f64 + (right - left) as f64 * phase;
        let x = x.round() as i32;
        canvas.line(x, y - 4, x, y + 4, '#');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    #[test]
    fn guess_line_maps_unit_x() {
        assert!((guess_from_unit_x(0.0) - GUESS_MIN).abs() < 1e-12);
        assert!((guess_from_unit_x(1.0) - GUESS_MAX).abs() < 1e-12);
        assert!((guess_from_unit_x(0.5) - 3.0).abs() < 1e-12);
        assert!((guess_from_unit_x(f64::NAN) - GUESS_MIN).abs() < 1e-12);
    }

    #[test]
    fn key_digits_name_common_guesses() {
        assert_eq!(guess_from_key_digit(1), Some(2.0));
        assert_eq!(guess_from_key_digit(2), Some(std::f64::consts::E));
        assert_eq!(guess_from_key_digit(3), Some(3.0));
        assert_eq!(guess_from_key_digit(4), Some(PI));
        assert!(guess_from_key_digit(0).is_none());
    }

    #[test]
    fn bands_grade_against_pi() {
        assert_eq!(GuessBand::grade(PI), GuessBand::Nailed);
        assert_eq!(GuessBand::grade(PI - 0.05), GuessBand::Nailed);
        assert_eq!(GuessBand::grade(3.0), GuessBand::Close);
        assert_eq!(GuessBand::grade(2.0), GuessBand::Wild);
        assert_eq!(GuessBand::grade(f64::NAN), GuessBand::Wild);
    }

    #[test]
    fn first_throw_primes_then_wager_withholds() {
        let mut aha = BuffonAha::new();
        assert_eq!(aha.beat(), AhaBeat::Explore);
        aha.note_throws(1);
        assert_eq!(aha.beat(), AhaBeat::Prime);
        assert!(aha.commit_wager(2.0));
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert!(!aha.allow_reveal_text());
        match aha.earn() {
            Some(EarnPath::Wager { guess, band }) => {
                assert!((guess - 2.0).abs() < 1e-12);
                assert_eq!(band, GuessBand::Wild);
            }
            other => panic!("expected wager earn, got {other:?}"),
        }
        assert!(!aha.commit_wager(PI));
    }

    #[test]
    fn eight_throws_earn_without_wager() {
        let mut aha = BuffonAha::new();
        aha.note_throws(7);
        assert_eq!(aha.beat(), AhaBeat::Prime);
        assert!(!aha.earned());
        aha.note_throws(8);
        assert_eq!(aha.earn(), Some(EarnPath::Throws { count: 8 }));
        assert_eq!(aha.beat(), AhaBeat::Withheld);
    }

    #[test]
    fn summon_morphs_then_confirm_then_consolidate() {
        let mut aha = BuffonAha::new();
        aha.note_throws(1);
        assert!(aha.commit_wager(3.0));
        assert!(aha.summon());
        assert!(matches!(aha.beat(), AhaBeat::Morph { progress } if progress == 0.0));
        aha.set_morph_progress(0.5);
        assert_eq!(aha.beat(), AhaBeat::Morph { progress: 0.5 });
        aha.set_morph_progress(1.0);
        assert_eq!(aha.beat(), AhaBeat::Confirm);
        assert!(!aha.allow_reveal_text());
        assert!(aha.summon());
        assert_eq!(aha.beat(), AhaBeat::Consolidated);
        assert!(aha.allow_reveal_text());
        assert!(aha.punchline().is_some());
    }

    #[test]
    fn morph_cannot_run_before_earn() {
        let mut aha = BuffonAha::new();
        assert!(!aha.summon());
        aha.set_morph_progress(1.0);
        assert_eq!(aha.beat(), AhaBeat::Explore);
        assert!(!aha.uses_circle_overlay());
    }

    #[test]
    fn advance_morph_is_non_negative() {
        let mut aha = BuffonAha::new();
        aha.note_throws(8);
        aha.summon();
        aha.advance_morph(-1.0);
        assert_eq!(aha.morph_progress(), 0.0);
        aha.advance_morph(0.7);
        assert!((aha.morph_progress() - 0.7).abs() < 1e-12);
        aha.advance_morph(0.5);
        assert_eq!(aha.beat(), AhaBeat::Confirm);
    }

    #[test]
    fn status_stays_compact() {
        let mut aha = BuffonAha::new();
        aha.note_throws(1);
        assert!(aha.status(None).chars().count() <= 28);
        aha.commit_wager(2.5);
        assert!(aha.status(None).chars().count() <= 28);
        aha.summon();
        aha.set_morph_progress(0.4);
        assert!(aha.status(None).chars().count() <= 16);
    }

    #[test]
    fn circle_overlay_draws_ink_when_progress_is_positive() {
        let mut canvas = Canvas::new(80, 40);
        render_circle_overlay(&mut canvas, 0.0);
        assert_eq!(canvas.ink_count(), 0);
        render_circle_overlay(&mut canvas, 0.5);
        assert!(canvas.ink_count() > 0);
        let mid = canvas.ink_count();
        let mut full = Canvas::new(80, 40);
        render_circle_overlay(&mut full, 1.0);
        assert!(full.ink_count() >= mid);
    }

    #[test]
    fn guess_band_marks_hover() {
        let mut canvas = Canvas::new(90, 40);
        render_guess_band(&mut canvas, Some(PI));
        assert!(canvas.ink_count() > 0);
    }

    #[test]
    fn show_style_explore_does_not_auto_earn_from_zero_throws() {
        let aha = BuffonAha::new();
        assert!(!aha.earned());
        assert_eq!(aha.beat(), AhaBeat::Explore);
    }
}
