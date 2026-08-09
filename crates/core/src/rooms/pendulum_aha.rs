//! The Double Pendulum engineered aha: call where the twin ends up.
//!
//! The fourth staged aha, cloned from the anatomy the other three share
//! (see `buffon_aha.rs`, `galton_aha.rs`): explore, prime, a withheld
//! commitment, a morph that shows the truth arriving, confirm, and one
//! graded sentence at consolidation.
//!
//! The question this room asks is the best one in the catalog, because the
//! honest answer is the one nobody believes: two pendulums released a ten
//! thousandth of a radian apart, running the same deterministic equations
//! with no randomness anywhere, end up nowhere near each other. A player
//! who calls TOGETHER is reasoning correctly from determinism and is
//! wrong, which is exactly the gap the room exists to open. So the miss is
//! the fertile one here more than anywhere, and the grading says so.

use crate::surface::Surface;

use super::double_pendulum::divergence_at_full_sweep;

/// Vertical start of the wager band, the flagship convention.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Morph progress that counts as done.
pub const MORPH_DONE: f64 = 1.0;
/// Gap below which the twins are still one pendulum to the eye.
const TOGETHER_GAP: f64 = 0.05;
/// Gap below which they are visibly apart but still swinging together.
const DRIFTED_GAP: f64 = 1.0;
/// How many points trace the divergence curve overlay.
const CURVE_STEPS: usize = 80;

/// Where the shadow twin ends up, as a player would say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    /// Still one pendulum to the eye.
    Together,
    /// Visibly apart, still recognizably the same swing.
    Drifted,
    /// Somewhere else entirely.
    Lost,
}

impl Ending {
    /// Compact spoken name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Together => "TOGETHER",
            Self::Drifted => "DRIFTED",
            Self::Lost => "LOST",
        }
    }

    /// The keyboard shortcut the App offers for this call.
    #[must_use]
    pub fn from_key_digit(digit: u8) -> Option<Self> {
        match digit {
            1 => Some(Self::Together),
            2 => Some(Self::Drifted),
            3 => Some(Self::Lost),
            _ => None,
        }
    }

    /// Which ending a measured gap actually is.
    #[must_use]
    pub fn of_gap(gap: f64) -> Self {
        if !gap.is_finite() || gap >= DRIFTED_GAP {
            Self::Lost
        } else if gap < TOGETHER_GAP {
            Self::Together
        } else {
            Self::Drifted
        }
    }

    /// Map a hand position along the wager band onto a call.
    #[must_use]
    pub fn from_unit_x(x: f64) -> Self {
        let x = if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.5
        };
        if x < 1.0 / 3.0 {
            Self::Together
        } else if x < 2.0 / 3.0 {
            Self::Drifted
        } else {
            Self::Lost
        }
    }
}

/// The truth this room's call is graded against, for a given variation.
#[must_use]
pub fn truth_for(variation: u64) -> (f64, Ending) {
    let gap = divergence_at_full_sweep(variation);
    (gap, Ending::of_gap(gap))
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnPath {
    /// An ending was called before the run finished.
    Call {
        /// What the player called.
        called: Ending,
        /// Whether that call matched the truth.
        right: bool,
    },
    /// The player re-dropped enough times to have run the experiment.
    Drops {
        /// How many re-drops earned the path.
        count: usize,
    },
}

/// Staging for the engineered aha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// Free swinging before the question.
    Explore,
    /// A hand has released the arms; invite the call.
    Prime,
    /// Generation complete; the truth is withheld until summoned.
    Withheld,
    /// The divergence curve draws itself across the sweep (progress 0..1).
    Morph {
        /// Morph blend, clamped to `[0, 1]`.
        progress: f64,
    },
    /// The curve stands; the gap is on the table.
    Confirm,
    /// Punchline available; full reveal text may open.
    Consolidated,
}

/// Pure visit state for the Double Pendulum engineered aha.
#[derive(Debug, Clone, PartialEq)]
pub struct PendulumAha {
    beat: AhaBeat,
    variation: u64,
    drops: usize,
    hover: Option<Ending>,
    earn: Option<EarnPath>,
    morph_progress: f64,
}

impl PendulumAha {
    /// A fresh visit of the room at this variation.
    #[must_use]
    pub fn new(variation: u64) -> Self {
        Self {
            beat: AhaBeat::Explore,
            variation,
            drops: 0,
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

    /// Hovered call while priming.
    #[must_use]
    pub fn hover(&self) -> Option<Ending> {
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

    /// Whether the visit should draw the divergence curve overlay.
    #[must_use]
    pub fn uses_curve_overlay(&self) -> bool {
        matches!(
            self.beat,
            AhaBeat::Morph { .. } | AhaBeat::Confirm | AhaBeat::Consolidated
        )
    }

    /// Note that a hand has released the arms.
    ///
    /// One release primes the question, because the player has now seen the
    /// pendulum obey them; four is running the experiment.
    pub fn note_drops(&mut self, drops: usize) {
        self.drops = drops;
        if drops >= 1 && matches!(self.beat, AhaBeat::Explore) {
            self.beat = AhaBeat::Prime;
        }
        if self.earn.is_none() && drops >= 4 {
            self.earn = Some(EarnPath::Drops { count: drops });
            self.hover = None;
            self.beat = AhaBeat::Withheld;
        }
    }

    /// Hover a call while priming.
    pub fn set_hover(&mut self, ending: Option<Ending>) {
        if matches!(self.beat, AhaBeat::Prime) {
            self.hover = ending;
        }
    }

    /// Commit the call. The first generation act wins.
    pub fn commit_call(&mut self, called: Ending) -> bool {
        if matches!(self.earn, Some(EarnPath::Call { .. })) {
            return false;
        }
        if !matches!(self.beat, AhaBeat::Prime | AhaBeat::Withheld) {
            return false;
        }
        let right = called == truth_for(self.variation).1;
        self.earn = Some(EarnPath::Call { called, right });
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
    pub fn status(&self, room_status: Option<&str>) -> String {
        match self.beat {
            AhaBeat::Explore => room_status.unwrap_or("CLICK:RE-DROP").to_string(),
            AhaBeat::Prime => {
                let hover = self
                    .hover
                    .map(|ending| format!(" >{}", ending.name()))
                    .unwrap_or_default();
                // The invite rides beside the room's own readout, never
                // instead of it: an interaction keeps showing what it did.
                let invite = format!("AT THE END? 1=TOGETHER 2=DRIFTED 3=LOST{hover}");
                match room_status {
                    Some(s) => format!("{s}  {invite}"),
                    None => invite,
                }
            }
            AhaBeat::Withheld => match self.earn {
                Some(EarnPath::Call { called, .. }) => {
                    format!("CALLED {}  PRESS E", called.name())
                }
                Some(EarnPath::Drops { count }) => format!("EARNED {count} DROPS  PRESS E"),
                None => "EARNED  PRESS E".to_string(),
            },
            AhaBeat::Morph { progress } => {
                let pct = (progress * 100.0).round() as i32;
                format!("THE GAP OPENS {pct}%")
            }
            AhaBeat::Confirm => match room_status {
                Some(s) => format!("SAME RULES, DIFFERENT WORLD  PRESS E  {s}"),
                None => "SAME RULES, DIFFERENT WORLD  PRESS E".to_string(),
            },
            AhaBeat::Consolidated => match room_status {
                Some(s) => format!("DETERMINISM IS NOT PREDICTION  {s}"),
                None => "DETERMINISM IS NOT PREDICTION  E:WHY".to_string(),
            },
        }
    }

    /// Punchline once consolidated.
    #[must_use]
    pub fn punchline(&self) -> Option<&'static str> {
        matches!(self.beat, AhaBeat::Consolidated).then_some(
            "Nothing here was random, and nobody could have told you where the shadow would be.",
        )
    }

    /// The committed call, once one exists.
    #[must_use]
    pub fn call(&self) -> Option<Ending> {
        match self.earn {
            Some(EarnPath::Call { called, .. }) => Some(called),
            _ => None,
        }
    }

    /// The call graded against the twin's real ending, spoken at
    /// consolidation.
    ///
    /// A player who calls TOGETHER is reasoning correctly from determinism
    /// and is wrong, which is the whole point of the room, so the miss is
    /// named as the lesson rather than as a failure.
    #[must_use]
    pub fn graded(&self) -> Option<String> {
        if !matches!(self.beat, AhaBeat::Consolidated) {
            return None;
        }
        let called = self.call()?;
        let (gap, truth) = truth_for(self.variation);
        let verdict = if called == truth {
            "Nailed."
        } else if called == Ending::Together {
            "The rules are deterministic and the answer still is not. That is the whole room."
        } else {
            "Close enough to feel the shape of it; the gap is the lesson."
        };
        Some(format!(
            "You called {}; the twin ended {} away, which is {}. {verdict}",
            called.name(),
            format_args!("{gap:.2}"),
            truth.name(),
        ))
    }

    /// Stable beat name for playtest notes and diagnostics.
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

    /// Compact earn path for playtest notes, or None before generation.
    #[must_use]
    pub fn earn_label(&self) -> Option<String> {
        match self.earn {
            Some(EarnPath::Call { called, right }) => Some(format!(
                "call:{}:{}",
                called.name().to_ascii_lowercase(),
                if right { "right" } else { "wrong" }
            )),
            Some(EarnPath::Drops { count }) => Some(format!("drops:{count}")),
            None => None,
        }
    }
}

/// Draw the divergence curve: how far the twin drifts across the sweep.
///
/// Flat, then a wall. That shape is the answer to the call, so the morph
/// reveals it left to right as the truth arrives.
pub fn render_gap_curve(canvas: &mut dyn Surface, progress: f64, variation: u64) {
    let (width, height) = canvas.draw_bounds();
    if width < 12 || height < 8 {
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
    let peak = divergence_at_full_sweep(variation).max(1e-6);
    let top = height as f64 * 0.08;
    let bottom = height as f64 * 0.34;
    let mut prev: Option<(i32, i32)> = None;
    let reach = (CURVE_STEPS as f64 * progress).round() as usize;
    for step in 0..=reach.min(CURVE_STEPS) {
        let unit = step as f64 / CURVE_STEPS as f64;
        let gap = super::double_pendulum::divergence_at_sweep_fraction(variation, unit);
        let x = (unit * (width.saturating_sub(1)) as f64).round() as i32;
        let y = (bottom - (bottom - top) * (gap / peak).clamp(0.0, 1.0)).round() as i32;
        if let Some((px, py)) = prev {
            canvas.line(px, py, x, y, '#');
        }
        prev = Some((x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::{AhaBeat, EarnPath, Ending, PendulumAha, truth_for};
    use crate::canvas::Canvas;

    #[test]
    fn the_twin_really_does_end_up_lost() {
        // The room's claim, checked rather than assumed: a ten thousandth
        // of a radian becomes a gap you cannot miss.
        let (gap, ending) = truth_for(0);
        assert!(gap > 1.0, "the twins end far apart, measured {gap}");
        assert_eq!(ending, Ending::Lost);
        assert_eq!(Ending::of_gap(0.0), Ending::Together);
        assert_eq!(Ending::of_gap(0.5), Ending::Drifted);
        assert_eq!(Ending::of_gap(f64::NAN), Ending::Lost);
    }

    #[test]
    fn the_beats_stage_in_order_and_the_call_is_graded() {
        let mut aha = PendulumAha::new(0);
        assert!(matches!(aha.beat(), AhaBeat::Explore));
        assert!(!aha.commit_call(Ending::Together), "no pendulum, no call");

        aha.note_drops(1);
        assert!(matches!(aha.beat(), AhaBeat::Prime));
        aha.set_hover(Some(Ending::Drifted));
        assert_eq!(aha.hover(), Some(Ending::Drifted));

        assert!(aha.commit_call(Ending::Together));
        assert!(matches!(aha.beat(), AhaBeat::Withheld));
        assert!(!aha.allow_reveal_text());
        assert!(!aha.commit_call(Ending::Lost), "the first call is the call");

        assert!(aha.summon());
        aha.advance_morph(0.5);
        assert!(matches!(aha.beat(), AhaBeat::Morph { .. }));
        aha.advance_morph(0.6);
        assert!(matches!(aha.beat(), AhaBeat::Confirm));
        assert!(aha.summon());
        assert!(aha.allow_reveal_text());

        let graded = aha.graded().expect("a call is answered");
        assert!(graded.contains("You called TOGETHER"), "{graded}");
        assert!(graded.contains("LOST"), "{graded}");
        assert!(
            graded.contains("deterministic and the answer still is not"),
            "the room's own lesson answers the most reasonable wrong call: {graded}"
        );
        assert_eq!(aha.earn_label().as_deref(), Some("call:together:wrong"));
    }

    #[test]
    fn calling_the_truth_is_nailed() {
        let mut aha = PendulumAha::new(0);
        aha.note_drops(1);
        assert!(aha.commit_call(truth_for(0).1));
        assert!(aha.summon());
        aha.set_morph_progress(1.0);
        assert!(aha.summon());
        let graded = aha.graded().expect("graded");
        assert!(graded.contains("Nailed"), "{graded}");
        assert_eq!(aha.earn_label().as_deref(), Some("call:lost:right"));
    }

    #[test]
    fn four_drops_earn_the_beat_without_a_call() {
        let mut aha = PendulumAha::new(0);
        aha.note_drops(4);
        assert!(matches!(aha.beat(), AhaBeat::Withheld));
        assert!(matches!(aha.earn(), Some(EarnPath::Drops { count: 4 })));
        assert!(aha.summon());
        aha.set_morph_progress(1.0);
        assert!(aha.summon());
        assert!(aha.graded().is_none(), "no call, nothing to grade");

        // A call still lands after the experiment earned the beat, exactly
        // as it does on the Galton board: naming an ending is the stronger
        // commitment. But not after the truth is already out.
        let mut late = PendulumAha::new(0);
        late.note_drops(4);
        assert!(late.commit_call(Ending::Drifted));
        assert!(matches!(late.earn(), Some(EarnPath::Call { .. })));
    }

    #[test]
    fn the_band_and_the_keys_name_the_same_three_endings() {
        assert_eq!(Ending::from_unit_x(0.1), Ending::Together);
        assert_eq!(Ending::from_unit_x(0.5), Ending::Drifted);
        assert_eq!(Ending::from_unit_x(0.9), Ending::Lost);
        assert_eq!(Ending::from_unit_x(f64::NAN), Ending::Drifted);
        assert_eq!(Ending::from_key_digit(1), Some(Ending::Together));
        assert_eq!(Ending::from_key_digit(2), Some(Ending::Drifted));
        assert_eq!(Ending::from_key_digit(3), Some(Ending::Lost));
        assert_eq!(Ending::from_key_digit(4), None);
    }

    #[test]
    fn the_curve_grows_left_to_right_as_the_truth_arrives() {
        let mut early = Canvas::new(72, 30);
        super::render_gap_curve(&mut early, 0.25, 0);
        let mut full = Canvas::new(72, 30);
        super::render_gap_curve(&mut full, 1.0, 0);
        assert!(early.ink_count() > 0, "the curve starts drawing");
        assert!(
            full.ink_count() > early.ink_count(),
            "and reaches further as the morph runs"
        );
        let mut none = Canvas::new(72, 30);
        super::render_gap_curve(&mut none, 0.0, 0);
        assert_eq!(none.ink_count(), 0);
    }
}
