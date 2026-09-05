//! The Kepler engineered aha: call how orbital speed changes near the sun.
//!
//! The player first chooses an ellipse, then commits to faster, slower, or
//! the same. Equal-time beads arrive around that exact ellipse. Their spacing
//! makes the answer visible before the final sentence names it.

use crate::surface::Surface;

use super::kepler_laws::{MAX_ECCENTRICITY, orbit_geometry, point_at_mean};

/// Vertical start of the wager band, shared with the other staged rooms.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Morph progress that counts as complete.
pub const MORPH_DONE: f64 = 1.0;
const TIME_MARKS: usize = 24;

/// The player's claim about speed near the sun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedRelation {
    /// The body covers more orbit per unit time near the sun.
    Faster,
    /// The body covers less orbit per unit time near the sun.
    Slower,
    /// The speed is unchanged, which is true for the circular limit.
    Same,
}

impl SpeedRelation {
    /// Compact spoken name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Faster => "FASTER",
            Self::Slower => "SLOWER",
            Self::Same => "SAME",
        }
    }

    /// Map an App number key onto a call.
    #[must_use]
    pub fn from_key_digit(digit: u8) -> Option<Self> {
        match digit {
            1 => Some(Self::Faster),
            2 => Some(Self::Slower),
            3 => Some(Self::Same),
            _ => None,
        }
    }

    /// Map a normalized hand position on the wager band onto a call.
    #[must_use]
    pub fn from_unit_x(x: f64) -> Self {
        let x = if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.5
        };
        if x < 1.0 / 3.0 {
            Self::Faster
        } else if x < 2.0 / 3.0 {
            Self::Slower
        } else {
            Self::Same
        }
    }
}

/// Exact answer for an ellipse with the supplied eccentricity.
#[must_use]
pub fn truth_for_eccentricity(eccentricity: f64) -> SpeedRelation {
    if bounded_eccentricity(eccentricity) == 0.0 {
        SpeedRelation::Same
    } else {
        SpeedRelation::Faster
    }
}

/// Perihelion speed divided by aphelion speed for an ellipse.
///
/// This follows directly from angular momentum at the two apsides. The
/// circular limit is one.
#[must_use]
pub fn apsidal_speed_ratio(eccentricity: f64) -> f64 {
    let e = bounded_eccentricity(eccentricity);
    (1.0 + e) / (1.0 - e)
}

/// Draw the three speed calls along the bottom input band.
pub fn render_speed_band(canvas: &mut dyn Surface, hover: Option<SpeedRelation>) {
    let (width, height) = canvas.draw_bounds();
    if width < 16 || height < 6 {
        return;
    }
    let y = ((height as f64) * 0.92).round() as i32;
    let y = y.clamp(1, height as i32 - 2);
    canvas.line(0, y, width.saturating_sub(1) as i32, y, '-');
    for (index, relation) in [
        SpeedRelation::Faster,
        SpeedRelation::Slower,
        SpeedRelation::Same,
    ]
    .iter()
    .enumerate()
    {
        let x = ((index as f64 + 0.5) / 3.0 * width as f64).round() as i32;
        let mark = if hover == Some(*relation) { '#' } else { '+' };
        canvas.line(x, y - 2, x, y + 1, mark);
        canvas.plot(x, y + 2, relation.name().chars().next().unwrap_or('?'));
    }
}

/// Draw equal-time positions on the selected ellipse as the answer arrives.
///
/// Points cluster near aphelion and spread near perihelion. The visual thus
/// answers the speed call without requiring color or explanatory prose.
pub fn render_equal_time_overlay(canvas: &mut dyn Surface, progress: f64, eccentricity: f64) {
    let (width, height) = canvas.draw_bounds();
    if width < 8 || height < 8 {
        return;
    }
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let visible = (progress * TIME_MARKS as f64).floor() as usize;
    if visible == 0 {
        return;
    }
    let e = if eccentricity.is_finite() {
        eccentricity.abs().clamp(0.0, MAX_ECCENTRICITY)
    } else {
        0.0
    };
    let geometry = orbit_geometry(width, height, e, canvas.safe_char_aspect());
    for index in 0..visible.min(TIME_MARKS) {
        let mean = std::f64::consts::TAU * index as f64 / TIME_MARKS as f64;
        let (x, y) = point_at_mean(geometry, e, mean);
        canvas.plot(x, y, 'O');
        if index % 4 == 0 {
            canvas.line(
                geometry.focus_x.round() as i32,
                geometry.cy.round() as i32,
                x,
                y,
                ':',
            );
        }
    }
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnPath {
    /// A speed relation was called before the answer appeared.
    Call {
        /// What the player called.
        called: SpeedRelation,
        /// Whether the call matched the selected orbit.
        right: bool,
    },
    /// The player tuned enough ellipses to run the experiment without a call.
    Tunings {
        /// Number of completed tunings.
        count: usize,
    },
}

/// Staging for the Kepler engineered aha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// The untouched orbit is available to explore.
    Explore,
    /// An ellipse has been chosen and the speed call is invited.
    Prime,
    /// A generation act landed, with the answer still withheld.
    Withheld,
    /// Equal-time marks arrive around the selected orbit.
    Morph {
        /// Morph blend in `[0, 1]`.
        progress: f64,
    },
    /// All equal-time marks stand on the orbit.
    Confirm,
    /// The answer and full room reveal may open.
    Consolidated,
}

/// Pure visit state for the Kepler speed wager.
#[derive(Debug, Clone, PartialEq)]
pub struct KeplerAha {
    beat: AhaBeat,
    eccentricity: f64,
    tunings: usize,
    hover: Option<SpeedRelation>,
    earn: Option<EarnPath>,
    morph_progress: f64,
}

impl KeplerAha {
    /// Begin a visit with a deterministic orbit.
    #[must_use]
    pub fn new(eccentricity: f64) -> Self {
        Self {
            beat: AhaBeat::Explore,
            eccentricity: bounded_eccentricity(eccentricity),
            tunings: 0,
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

    /// Selected eccentricity, locked when generation completes.
    #[must_use]
    pub fn eccentricity(&self) -> f64 {
        self.eccentricity
    }

    /// Hovered speed call while priming.
    #[must_use]
    pub fn hover(&self) -> Option<SpeedRelation> {
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

    /// The mathematical answer for the selected orbit.
    #[must_use]
    pub fn truth(&self) -> SpeedRelation {
        truth_for_eccentricity(self.eccentricity)
    }

    /// Update the selected orbit before the generation act locks it.
    pub fn bind_eccentricity(&mut self, eccentricity: f64) -> bool {
        if self.earned() || !eccentricity.is_finite() {
            return false;
        }
        self.eccentricity = bounded_eccentricity(eccentricity);
        true
    }

    /// Record completed orbit tunings.
    ///
    /// One tuning primes the question. Four tunings earn the experiment path
    /// for a player who prefers to observe before naming a prediction.
    pub fn note_tunings(&mut self, count: usize) {
        self.tunings = count;
        if count >= 1 && matches!(self.beat, AhaBeat::Explore) {
            self.beat = AhaBeat::Prime;
        }
        if self.earn.is_none() && count >= 4 {
            self.earn = Some(EarnPath::Tunings { count });
            self.hover = None;
            self.beat = AhaBeat::Withheld;
        }
    }

    /// Number of completed orbit tunings in this visit.
    #[must_use]
    pub fn tunings(&self) -> usize {
        self.tunings
    }

    /// Hover a call while the question is open.
    pub fn set_hover(&mut self, relation: Option<SpeedRelation>) {
        if matches!(self.beat, AhaBeat::Prime) {
            self.hover = relation;
        }
    }

    /// Commit a speed call. The first generation act owns the visit.
    pub fn commit_call(&mut self, called: SpeedRelation) -> bool {
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

    /// The committed speed call, if this path used one.
    #[must_use]
    pub fn call(&self) -> Option<SpeedRelation> {
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

    /// Whether equal-time marks should be drawn.
    #[must_use]
    pub fn uses_time_overlay(&self) -> bool {
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
            AhaBeat::Explore => room_status.unwrap_or("DRAG:TUNE ECC").to_string(),
            AhaBeat::Prime => "NEAR SUN? 1=FASTER 2=SLOWER 3=SAME".to_string(),
            // The experiment path earns without a call, so it must not borrow
            // the CALLED sentence. Telling a player they called something they
            // never called is the same lie as hiding a call they did make.
            AhaBeat::Withheld => match self.earn {
                Some(EarnPath::Call { called, .. }) => {
                    format!("CALLED {}  PRESS E", called.name())
                }
                Some(EarnPath::Tunings { count }) => {
                    format!("{count} TUNINGS HELD  PRESS E")
                }
                None => "READY  PRESS E".to_string(),
            },
            AhaBeat::Morph { progress } => {
                format!("EQUAL TIME MARKS {:>3}%", (progress * 100.0).round() as u8)
            }
            AhaBeat::Confirm => format!("NEAR SUN: {}  PRESS E", self.truth().name()),
            AhaBeat::Consolidated => self.punchline().to_string(),
        }
    }

    /// One-sentence consolidation for the selected orbit.
    #[must_use]
    pub fn punchline(&self) -> &'static str {
        match self.truth() {
            SpeedRelation::Same => "A circle has no nearer side: same speed all around.",
            SpeedRelation::Faster => "Equal time, more orbit near the sun: faster.",
            SpeedRelation::Slower => "A bounded ellipse never answers slower near the sun.",
        }
    }

    /// Answer the player's exact call against the selected orbit.
    #[must_use]
    pub fn graded(&self) -> Option<String> {
        if !matches!(self.beat, AhaBeat::Consolidated) {
            return None;
        }
        let called = self.call()?;
        let truth = self.truth();
        let e = self.eccentricity;
        let ratio = apsidal_speed_ratio(e);
        let displayed_e = if e > 0.0 && e < 0.001 {
            format!("{e:.3e}")
        } else {
            e.to_string()
        };
        // Preserve a positive difference even when adding it to one would
        // round away in f64. This is the stable identity ratio = 1 + 2e/(1-e).
        let displayed_ratio = if e > 0.0 && e < 0.005 {
            format!("1 + {:.3e}", 2.0 * e / (1.0 - e))
        } else {
            format!("{ratio:.2}")
        };
        let verdict = if called == truth {
            "Nailed."
        } else if truth == SpeedRelation::Same {
            "The fertile miss: this orbit is circular, so it has no nearer side."
        } else {
            "The fertile miss: equal-time marks spread where the orbit nears the sun."
        };
        Some(format!(
            "You called {}; at e={displayed_e}, the truth is {} (perihelion/aphelion speed ratio approximately {displayed_ratio}). {verdict}",
            called.name(),
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
            Some(EarnPath::Tunings { count }) => Some(format!("tunings:{count}")),
            None => None,
        }
    }
}

fn bounded_eccentricity(eccentricity: f64) -> f64 {
    if eccentricity.is_finite() {
        eccentricity.abs().clamp(0.0, MAX_ECCENTRICITY)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AhaBeat, KeplerAha, SpeedRelation, apsidal_speed_ratio, render_equal_time_overlay,
        truth_for_eccentricity,
    };
    use crate::canvas::Canvas;

    #[test]
    fn ellipse_call_walks_all_five_beats_and_grades_the_gap() {
        let mut aha = KeplerAha::new(0.72);
        aha.note_tunings(1);
        assert_eq!(aha.beat(), AhaBeat::Prime);
        assert!(aha.commit_call(SpeedRelation::Same));
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
        assert!(grade.contains("called SAME"), "{grade}");
        assert!(grade.contains("truth is FASTER"), "{grade}");
        assert_eq!(aha.earn_label().as_deref(), Some("call:same:wrong"));
    }

    #[test]
    fn circular_limit_is_same_and_locked_after_the_call() {
        let mut aha = KeplerAha::new(0.0);
        aha.note_tunings(1);
        assert!(aha.commit_call(SpeedRelation::Same));
        assert!(!aha.bind_eccentricity(0.8));
        assert_eq!(aha.truth(), SpeedRelation::Same);
        assert_eq!(truth_for_eccentricity(0.03), SpeedRelation::Faster);
    }

    #[test]
    fn every_nonzero_eccentricity_answers_faster_even_below_pixel_resolution() {
        for e in [f64::EPSILON, 0.001, 0.01, 0.02, 0.03, 0.9] {
            assert_eq!(truth_for_eccentricity(e), SpeedRelation::Faster, "e={e}");
        }
        let mut aha = KeplerAha::new(0.01);
        aha.note_tunings(1);
        assert!(aha.commit_call(SpeedRelation::Faster));
        assert_eq!(aha.earn_label().as_deref(), Some("call:faster:right"));
    }

    #[test]
    fn four_tunings_earn_observation_without_forcing_a_call() {
        let mut aha = KeplerAha::new(0.6);
        aha.note_tunings(4);
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert_eq!(aha.earn_label().as_deref(), Some("tunings:4"));
        assert!(aha.summon());
    }

    #[test]
    fn small_ellipses_do_not_print_a_circular_eccentricity_or_unit_speed_ratio() {
        for e in [f64::from_bits(1), f64::EPSILON, 0.0001, 0.001] {
            let mut aha = KeplerAha::new(e);
            aha.note_tunings(1);
            assert!(aha.commit_call(SpeedRelation::Faster));
            assert!(aha.summon());
            aha.advance_morph(1.0);
            assert!(aha.summon());
            let grade = aha.graded().expect("consolidated call");
            assert!(grade.contains("truth is FASTER"), "{grade}");
            assert!(!grade.contains("e=0.000,"), "{grade}");
            assert!(grade.contains("approximately 1 + "), "{grade}");
            assert!(!grade.contains("1 + 0.000"), "{grade}");
        }
    }

    #[test]
    fn apsidal_ratio_matches_circle_and_an_eccentric_orbit() {
        assert_eq!(apsidal_speed_ratio(0.0), 1.0);
        assert!((apsidal_speed_ratio(0.5) - 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn equal_time_marks_arrive_progressively_without_color() {
        let mut none = Canvas::new(72, 30);
        render_equal_time_overlay(&mut none, 0.0, 0.75);
        assert_eq!(none.ink_count(), 0);
        let mut early = Canvas::new(72, 30);
        render_equal_time_overlay(&mut early, 0.25, 0.75);
        let mut full = Canvas::new(72, 30);
        render_equal_time_overlay(&mut full, 1.0, 0.75);
        assert!(early.ink_count() > 0);
        assert!(full.ink_count() > early.ink_count());
        assert!(full.to_text().contains('O'));
    }

    #[test]
    fn hostile_values_are_bounded() {
        let aha = KeplerAha::new(f64::NAN);
        assert_eq!(aha.eccentricity(), 0.0);
        assert_eq!(apsidal_speed_ratio(f64::INFINITY), 1.0);
        let mut tiny = Canvas::new(2, 2);
        render_equal_time_overlay(&mut tiny, f64::NAN, f64::INFINITY);
        assert_eq!(tiny.ink_count(), 0);
    }
}
