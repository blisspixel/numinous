//! Times Tables five-beat engineered aha.
//!
//! Pure visit state for the Exceptional Path Phase A slice: a place-guess wager
//! about where the K=2 cardioid also lives, a cardioid-to-Mandelbrot morph, then
//! hand confirm. Faces own wall-clock animation and feed morph progress; this
//! module never reads a clock. See `docs/PEDAGOGY.md` and `docs/ROADMAP.md`
//! (Phase A vertical slice).

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use crate::surface::Surface;

/// Hand band for the place wager (bottom of the plate). Dial control stays above.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Morph progress at or above this value completes the restructure beat.
pub const MORPH_DONE: f64 = 1.0;
/// Samples used to stroke the shared cardioid outline.
const OUTLINE_STEPS: usize = 192;
/// Chord samples while the morph still shows the string art.
const MORPH_CHORD_SAMPLES: usize = 96;
const MORPH_POINTS: usize = 240;

/// Where the player thinks the K=2 cardioid also lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardioidHome {
    /// The Mandelbrot main bulb (the true home).
    Mandelbrot,
    /// A nephroid room (same family language, wrong place).
    Nephroid,
    /// A plain circle (the expected wrong guess).
    Circle,
}

impl CardioidHome {
    /// The three language-light options, left to right.
    pub const ALL: [Self; 3] = [Self::Mandelbrot, Self::Nephroid, Self::Circle];

    /// Map unit x in `[0, 1]` onto the three options (equal thirds).
    #[must_use]
    pub fn from_unit_x(x: f64) -> Self {
        let x = if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if x < 1.0 / 3.0 {
            Self::Mandelbrot
        } else if x < 2.0 / 3.0 {
            Self::Nephroid
        } else {
            Self::Circle
        }
    }

    /// One-based index for key bindings (1, 2, 3).
    #[must_use]
    pub fn from_key_digit(digit: u8) -> Option<Self> {
        match digit {
            1 => Some(Self::Mandelbrot),
            2 => Some(Self::Nephroid),
            3 => Some(Self::Circle),
            _ => None,
        }
    }

    /// Spoken option name for status and chrome.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Mandelbrot => "MANDELBROT",
            Self::Nephroid => "NEPHROID",
            Self::Circle => "CIRCLE",
        }
    }

    /// Single-letter compact tag.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Mandelbrot => "M",
            Self::Nephroid => "N",
            Self::Circle => "C",
        }
    }

    /// Whether this is the true home of the shared cardioid.
    #[must_use]
    pub fn is_truth(self) -> bool {
        matches!(self, Self::Mandelbrot)
    }
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnPath {
    /// The place wager was committed (right or wrong).
    Wager {
        /// The player's guess.
        guess: CardioidHome,
    },
    /// The existing room goal: land on exactly four lobes (K=5).
    FourLobes,
}

/// Staging for the engineered aha (explore plus the five pedagogy beats).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// Free dial play before a readable heart has been held.
    Explore,
    /// Heart held; invite one place guess (prime the gap).
    Prime,
    /// Generation complete; reveal still withheld until summon.
    Withheld,
    /// Chord envelope detaches onto the Mandelbrot main bulb (progress 0..1).
    Morph {
        /// Morph blend, clamped to `[0, 1]`.
        progress: f64,
    },
    /// Hand drives the times-table dial and a bead on the Mandelbrot outline.
    Confirm,
    /// Punchline available; full reveal text may open.
    Consolidated,
}

/// Pure visit state for the Times Tables engineered aha.
#[derive(Debug, Clone, PartialEq)]
pub struct TimesTablesAha {
    beat: AhaBeat,
    heart_held: bool,
    earn: Option<EarnPath>,
    hover: Option<CardioidHome>,
    morph_progress: f64,
}

impl Default for TimesTablesAha {
    fn default() -> Self {
        Self::new()
    }
}

impl TimesTablesAha {
    /// A fresh visit: no heart, no wager, no morph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beat: AhaBeat::Explore,
            heart_held: false,
            earn: None,
            hover: None,
            morph_progress: 0.0,
        }
    }

    /// Current beat.
    #[must_use]
    pub fn beat(&self) -> AhaBeat {
        self.beat
    }

    /// Whether a readable K=2 heart has been held by the hand.
    #[must_use]
    pub fn heart_held(&self) -> bool {
        self.heart_held
    }

    /// Earn path once generation has completed.
    #[must_use]
    pub fn earn(&self) -> Option<EarnPath> {
        self.earn
    }

    /// Hovered place option while priming (optional chrome hint).
    #[must_use]
    pub fn hover(&self) -> Option<CardioidHome> {
        self.hover
    }

    /// Morph progress in `[0, 1]`.
    #[must_use]
    pub fn morph_progress(&self) -> f64 {
        self.morph_progress
    }

    /// Generation is complete (wager or four-lobe goal).
    #[must_use]
    pub fn earned(&self) -> bool {
        self.earn.is_some()
    }

    /// Full reveal text may open only after the morph has consolidated.
    #[must_use]
    pub fn allow_reveal_text(&self) -> bool {
        matches!(self.beat, AhaBeat::Consolidated)
    }

    /// Summon advances withheld into morph, or confirms into punchline.
    #[must_use]
    pub fn can_summon(&self) -> bool {
        matches!(self.beat, AhaBeat::Withheld | AhaBeat::Confirm)
    }

    /// Whether the visit should draw the dual morph/confirm plate instead of
    /// the ordinary dial sweep.
    #[must_use]
    pub fn uses_aha_plate(&self) -> bool {
        matches!(
            self.beat,
            AhaBeat::Morph { .. } | AhaBeat::Confirm | AhaBeat::Consolidated
        )
    }

    /// Note a hand-controlled multiplier. A closed one-lobe heart primes the gap.
    pub fn note_hand_multiplier(&mut self, k: f64) {
        if !k.is_finite() {
            return;
        }
        let nearest = k.round();
        let closed = (k - nearest).abs() < 1e-9;
        if closed && (nearest - 2.0).abs() < f64::EPSILON {
            self.heart_held = true;
            if matches!(self.beat, AhaBeat::Explore) {
                self.beat = AhaBeat::Prime;
            }
        }
    }

    /// Hover a place option (Prime only; bottom-band chrome).
    pub fn set_hover(&mut self, place: Option<CardioidHome>) {
        if matches!(self.beat, AhaBeat::Prime) {
            self.hover = place;
        }
    }

    /// Commit the place wager. First generation act wins; later commits are no-ops.
    pub fn commit_wager(&mut self, guess: CardioidHome) -> bool {
        if self.earn.is_some() {
            return false;
        }
        if !matches!(self.beat, AhaBeat::Prime | AhaBeat::Explore) {
            return false;
        }
        // A wager from Explore still counts: the generation act itself primes.
        self.heart_held = true;
        self.earn = Some(EarnPath::Wager { guess });
        self.hover = None;
        self.beat = AhaBeat::Withheld;
        true
    }

    /// Earn via the existing four-lobe goal. Does not open the reveal.
    pub fn note_four_lobes(&mut self) -> bool {
        if self.earn.is_some() {
            return false;
        }
        self.earn = Some(EarnPath::FourLobes);
        self.beat = AhaBeat::Withheld;
        true
    }

    /// Summon the next staged beat after generation (Withheld -> Morph, Confirm -> Consolidated).
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

    /// Compact status for the footer. Stays short for narrow windows.
    #[must_use]
    pub fn status(&self, dial: Option<&str>) -> String {
        match self.beat {
            AhaBeat::Explore => dial.unwrap_or("DRAG THE DIAL").to_string(),
            AhaBeat::Prime => {
                let hover = self
                    .hover
                    .map(|h| format!(" >{}", h.tag()))
                    .unwrap_or_default();
                format!("HEART  1=M 2=N 3=C{hover}")
            }
            AhaBeat::Withheld => match self.earn {
                Some(EarnPath::Wager { guess }) if guess.is_truth() => {
                    "EARNED NAILED  PRESS E".to_string()
                }
                Some(EarnPath::Wager { guess }) => {
                    format!("EARNED {}  PRESS E", guess.tag())
                }
                Some(EarnPath::FourLobes) => "EARNED 4 LOBES  PRESS E".to_string(),
                None => "EARNED  PRESS E".to_string(),
            },
            AhaBeat::Morph { progress } => {
                let pct = (progress * 100.0).round() as i32;
                format!("MORPH {pct}%")
            }
            // E consolidates to the punchline; keep the summon cue visible even
            // when a dial readout is present so Confirm is not a dead end.
            AhaBeat::Confirm => match dial {
                Some(d) => format!("BOTH E  {d}"),
                None => "BOTH  PRESS E".to_string(),
            },
            AhaBeat::Consolidated => match dial {
                Some(d) => format!("ONE HEART  {d}"),
                None => "ONE HEART  E:WHY".to_string(),
            },
        }
    }

    /// Punchline line once consolidated (copy arrives after the morph).
    #[must_use]
    pub fn punchline(&self) -> Option<&'static str> {
        matches!(self.beat, AhaBeat::Consolidated)
            .then_some("Same cardioid: times-2 chords and the Mandelbrot main bulb.")
    }

    /// Stable beat name for playtest notes and diagnostics (not player chrome).
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
    pub fn earn_label(&self) -> Option<&'static str> {
        match self.earn {
            Some(EarnPath::Wager { guess }) => Some(match guess {
                CardioidHome::Mandelbrot => "wager:mandelbrot",
                CardioidHome::Nephroid => "wager:nephroid",
                CardioidHome::Circle => "wager:circle",
            }),
            Some(EarnPath::FourLobes) => Some("four-lobes"),
            None => None,
        }
    }
}

/// Times-table chord envelope at K=2 (unit cardioid, cusp at the right).
#[must_use]
pub fn times_table_envelope(t: f64) -> (f64, f64) {
    let t = if t.is_finite() { t } else { 0.0 };
    (
        2.0 / 3.0 * t.cos() + 1.0 / 3.0 * (2.0 * t).cos(),
        2.0 / 3.0 * t.sin() + 1.0 / 3.0 * (2.0 * t).sin(),
    )
}

/// Mandelbrot main-cardioid boundary in the complex plane.
#[must_use]
pub fn mandelbrot_main_cardioid(theta: f64) -> (f64, f64) {
    let theta = if theta.is_finite() { theta } else { 0.0 };
    (
        0.5 * theta.cos() - 0.25 * (2.0 * theta).cos(),
        0.5 * theta.sin() - 0.25 * (2.0 * theta).sin(),
    )
}

/// Affine that places the Mandelbrot main cardioid on the times-table envelope.
///
/// Proven by the room's geometry test: envelope(t) equals this transform of
/// the Mandelbrot cardioid at `theta = t + pi`.
#[must_use]
pub fn mandelbrot_as_envelope(theta: f64) -> (f64, f64) {
    let m = mandelbrot_main_cardioid(theta + PI);
    (-4.0 / 3.0 * m.0, -4.0 / 3.0 * m.1)
}

/// Linear blend from the times-table envelope frame toward a Mandelbrot-centered
/// outline in the same plate coordinates. At `u = 0` the curve is the envelope;
/// at `u = 1` it is the Mandelbrot cardioid scaled into the same radius.
#[must_use]
pub fn morph_cardioid_point(t: f64, u: f64) -> (f64, f64) {
    let u = if u.is_finite() {
        u.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let env = times_table_envelope(t);
    // At full morph, re-express the same affine shape so endpoints stay exact,
    // then bias the plate slightly left so the Mandelbrot cusp reads as a bulb.
    let mandel_frame = mandelbrot_as_envelope(t);
    let shift = u * 0.08;
    (
        (1.0 - u) * env.0 + u * (mandel_frame.0 - shift),
        (1.0 - u) * env.1 + u * mandel_frame.1,
    )
}

fn plate_radius(width: f64, height: f64, aspect: f64) -> f64 {
    (width / 2.0).min(height / (2.0 * aspect.max(0.25))) * 0.84
}

fn to_pixel(cx: f64, cy: f64, radius: f64, aspect: f64, point: (f64, f64)) -> (i32, i32) {
    let x = cx + radius * point.0;
    let y = cy + radius * point.1 * aspect;
    (x.round() as i32, y.round() as i32)
}

fn chord_point(i: usize, cx: f64, cy: f64, radius: f64, aspect: f64) -> (i32, i32) {
    let angle = (i as f64 / MORPH_POINTS as f64) * TAU - FRAC_PI_2;
    let x = cx + radius * angle.cos();
    let y = cy + radius * angle.sin() * aspect;
    (x.round() as i32, y.round() as i32)
}

fn draw_k2_chords(
    canvas: &mut dyn Surface,
    cx: f64,
    cy: f64,
    radius: f64,
    aspect: f64,
    mark: char,
) {
    for sample in 0..MORPH_CHORD_SAMPLES {
        let n = sample * MORPH_POINTS / MORPH_CHORD_SAMPLES;
        let target = ((n as f64) * 2.0).round() as usize % MORPH_POINTS;
        let (x0, y0) = chord_point(n, cx, cy, radius, aspect);
        let (x1, y1) = chord_point(target, cx, cy, radius, aspect);
        canvas.line(x0, y0, x1, y1, mark);
    }
}

fn draw_multiplier_chords(
    canvas: &mut dyn Surface,
    cx: f64,
    cy: f64,
    radius: f64,
    aspect: f64,
    multiplier: f64,
    mark: char,
) {
    let multiplier = if multiplier.is_finite() {
        multiplier
    } else {
        2.0
    };
    for sample in 0..MORPH_CHORD_SAMPLES {
        let n = sample * MORPH_POINTS / MORPH_CHORD_SAMPLES;
        let target = ((n as f64) * multiplier).round() as usize % MORPH_POINTS;
        let (x0, y0) = chord_point(n, cx, cy, radius, aspect);
        let (x1, y1) = chord_point(target, cx, cy, radius, aspect);
        canvas.line(x0, y0, x1, y1, mark);
    }
}

fn draw_outline(
    canvas: &mut dyn Surface,
    cx: f64,
    cy: f64,
    radius: f64,
    aspect: f64,
    u: f64,
    mark: char,
) {
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=OUTLINE_STEPS {
        let t = TAU * (i as f64 / OUTLINE_STEPS as f64);
        let p = morph_cardioid_point(t, u);
        let px = to_pixel(cx, cy, radius, aspect, p);
        if let Some(prev) = prev {
            canvas.line(prev.0, prev.1, px.0, px.1, mark);
        }
        prev = Some(px);
    }
}

fn draw_bead(
    canvas: &mut dyn Surface,
    cx: f64,
    cy: f64,
    radius: f64,
    aspect: f64,
    theta: f64,
    mark: char,
) {
    let p = morph_cardioid_point(theta, 1.0);
    let (px, py) = to_pixel(cx, cy, radius, aspect, p);
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(px + dx, py + dy, mark);
            }
        }
    }
}

/// Draw the restructure / confirm plate for the engineered aha.
///
/// `multiplier` is the live dial value used in Confirm and Consolidated so the
/// hand re-derives the shared shape. During Morph, chords stay at K=2 while the
/// outline blends onto the Mandelbrot frame.
pub fn render_aha_plate(canvas: &mut dyn Surface, beat: AhaBeat, multiplier: f64) {
    let width = canvas.width() as f64;
    let height = canvas.height() as f64;
    if width < 2.0 || height < 2.0 {
        return;
    }
    let aspect = canvas.char_aspect();
    let cx = width / 2.0;
    let cy = height / 2.0;
    let radius = plate_radius(width, height, aspect);

    match beat {
        AhaBeat::Morph { progress } => {
            let u = if progress.is_finite() {
                progress.clamp(0.0, 1.0)
            } else {
                0.0
            };
            if u < 0.92 {
                let mark = if u < 0.45 { '*' } else { '.' };
                draw_k2_chords(canvas, cx, cy, radius, aspect, mark);
            }
            let outline_mark = if u < 0.35 { '#' } else { '@' };
            draw_outline(canvas, cx, cy, radius, aspect, u, outline_mark);
        }
        AhaBeat::Confirm | AhaBeat::Consolidated => {
            draw_multiplier_chords(canvas, cx, cy, radius, aspect, multiplier, '*');
            draw_outline(canvas, cx, cy, radius, aspect, 1.0, '@');
            // Bead rides the Mandelbrot-frame outline with the dial angle.
            let phase = ((multiplier - 2.0) / 8.0).clamp(0.0, 1.0);
            let theta = phase * TAU;
            draw_bead(canvas, cx, cy, radius, aspect, theta, '#');
        }
        AhaBeat::Explore | AhaBeat::Prime | AhaBeat::Withheld => {
            // Caller should use the ordinary room plate; keep a safe fallback.
            draw_k2_chords(canvas, cx, cy, radius, aspect, '*');
        }
    }
}

/// Draw three place options along the bottom wager band (Prime only).
pub fn render_wager_options(canvas: &mut dyn Surface, hover: Option<CardioidHome>) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    if width < 24 || height < 8 {
        return;
    }
    let y = (height as f64 * 0.92).round() as i32;
    let y = y.clamp(1, height - 2);
    for (i, place) in CardioidHome::ALL.iter().enumerate() {
        let x = ((i as f64 + 0.5) / 3.0 * width as f64).round() as i32;
        let mark = if hover == Some(*place) { '#' } else { '+' };
        canvas.plot(x, y, mark);
        canvas.plot(x, y - 1, mark);
        // Tiny letter tag via a short stem so narrow rasters stay readable.
        if matches!(place, CardioidHome::Mandelbrot) {
            canvas.plot(x, y + 1, 'M');
        } else if matches!(place, CardioidHome::Nephroid) {
            canvas.plot(x, y + 1, 'N');
        } else {
            canvas.plot(x, y + 1, 'C');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    #[test]
    fn place_thirds_cover_the_three_homes() {
        assert_eq!(CardioidHome::from_unit_x(0.0), CardioidHome::Mandelbrot);
        assert_eq!(CardioidHome::from_unit_x(0.32), CardioidHome::Mandelbrot);
        assert_eq!(CardioidHome::from_unit_x(0.34), CardioidHome::Nephroid);
        assert_eq!(CardioidHome::from_unit_x(0.66), CardioidHome::Nephroid);
        assert_eq!(CardioidHome::from_unit_x(0.67), CardioidHome::Circle);
        assert_eq!(CardioidHome::from_unit_x(1.0), CardioidHome::Circle);
        assert_eq!(
            CardioidHome::from_unit_x(f64::NAN),
            CardioidHome::Mandelbrot
        );
    }

    #[test]
    fn key_digits_map_left_to_right() {
        assert_eq!(
            CardioidHome::from_key_digit(1),
            Some(CardioidHome::Mandelbrot)
        );
        assert_eq!(
            CardioidHome::from_key_digit(2),
            Some(CardioidHome::Nephroid)
        );
        assert_eq!(CardioidHome::from_key_digit(3), Some(CardioidHome::Circle));
        assert!(CardioidHome::from_key_digit(0).is_none());
        assert!(CardioidHome::from_key_digit(4).is_none());
    }

    #[test]
    fn only_mandelbrot_is_truth() {
        assert!(CardioidHome::Mandelbrot.is_truth());
        assert!(!CardioidHome::Nephroid.is_truth());
        assert!(!CardioidHome::Circle.is_truth());
    }

    #[test]
    fn heart_hold_primes_then_wager_withholds() {
        let mut aha = TimesTablesAha::new();
        assert_eq!(aha.beat(), AhaBeat::Explore);
        assert_eq!(aha.beat_label(), "explore");
        assert!(aha.earn_label().is_none());
        aha.note_hand_multiplier(2.0);
        assert!(aha.heart_held());
        assert_eq!(aha.beat(), AhaBeat::Prime);
        assert_eq!(aha.beat_label(), "prime");
        assert!(aha.commit_wager(CardioidHome::Circle));
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert_eq!(aha.beat_label(), "withheld");
        assert_eq!(aha.earn_label(), Some("wager:circle"));
        assert!(!aha.allow_reveal_text());
        assert_eq!(
            aha.earn(),
            Some(EarnPath::Wager {
                guess: CardioidHome::Circle
            })
        );
        // Second wager is ignored.
        assert!(!aha.commit_wager(CardioidHome::Mandelbrot));
    }

    #[test]
    fn four_lobes_earns_without_wager() {
        let mut aha = TimesTablesAha::new();
        assert!(aha.note_four_lobes());
        assert_eq!(aha.earn(), Some(EarnPath::FourLobes));
        assert_eq!(aha.beat(), AhaBeat::Withheld);
        assert!(!aha.allow_reveal_text());
        assert!(!aha.note_four_lobes());
    }

    #[test]
    fn summon_morphs_then_confirm_then_consolidate() {
        let mut aha = TimesTablesAha::new();
        aha.note_hand_multiplier(2.0);
        assert!(aha.commit_wager(CardioidHome::Mandelbrot));
        assert!(aha.summon());
        assert!(matches!(aha.beat(), AhaBeat::Morph { progress } if progress == 0.0));
        aha.set_morph_progress(0.4);
        assert_eq!(aha.beat(), AhaBeat::Morph { progress: 0.4 });
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
        let mut aha = TimesTablesAha::new();
        assert!(!aha.summon());
        aha.set_morph_progress(1.0);
        assert_eq!(aha.beat(), AhaBeat::Explore);
        assert!(!aha.uses_aha_plate());
    }

    #[test]
    fn advance_morph_is_non_negative_and_finite() {
        let mut aha = TimesTablesAha::new();
        aha.note_four_lobes();
        aha.summon();
        aha.advance_morph(-1.0);
        assert_eq!(aha.morph_progress(), 0.0);
        aha.advance_morph(f64::NAN);
        assert_eq!(aha.morph_progress(), 0.0);
        aha.advance_morph(0.6);
        assert!((aha.morph_progress() - 0.6).abs() < 1e-12);
        aha.advance_morph(0.5);
        assert_eq!(aha.beat(), AhaBeat::Confirm);
    }

    #[test]
    fn envelope_matches_transformed_mandelbrot_cardioid() {
        for fraction in [0.0_f64, 0.125, 0.25, 0.5, 0.875] {
            let t = TAU * fraction;
            let envelope = times_table_envelope(t);
            let transformed = mandelbrot_as_envelope(t);
            assert!((envelope.0 - transformed.0).abs() < 1e-12);
            assert!((envelope.1 - transformed.1).abs() < 1e-12);
        }
    }

    #[test]
    fn morph_endpoints_are_exact() {
        for fraction in [0.0_f64, 0.2, 0.7] {
            let t = TAU * fraction;
            let start = morph_cardioid_point(t, 0.0);
            let env = times_table_envelope(t);
            assert!((start.0 - env.0).abs() < 1e-12);
            assert!((start.1 - env.1).abs() < 1e-12);

            let end = morph_cardioid_point(t, 1.0);
            let mandel = mandelbrot_as_envelope(t);
            // Full morph includes a fixed left shift for plate reading.
            assert!((end.0 - (mandel.0 - 0.08)).abs() < 1e-12);
            assert!((end.1 - mandel.1).abs() < 1e-12);
        }
    }

    #[test]
    fn status_stays_compact() {
        let mut aha = TimesTablesAha::new();
        aha.note_hand_multiplier(2.0);
        assert!(aha.status(None).chars().count() <= 24);
        aha.commit_wager(CardioidHome::Nephroid);
        assert!(aha.status(None).chars().count() <= 24);
        aha.summon();
        aha.set_morph_progress(0.5);
        assert!(aha.status(None).chars().count() <= 16);
        aha.set_morph_progress(1.0);
        assert_eq!(aha.beat(), AhaBeat::Confirm);
        assert!(aha.status(None).contains("PRESS E"));
        assert!(aha.status(None).chars().count() <= 16);
        assert!(aha.summon());
        assert!(aha.status(None).contains("E:WHY"));
        assert!(aha.status(None).chars().count() <= 20);
    }

    #[test]
    fn aha_plate_draws_ink_at_morph_and_confirm() {
        let mut canvas = Canvas::new(80, 40);
        render_aha_plate(&mut canvas, AhaBeat::Morph { progress: 0.0 }, 2.0);
        assert!(canvas.ink_count() > 0);
        canvas = Canvas::new(80, 40);
        render_aha_plate(&mut canvas, AhaBeat::Morph { progress: 1.0 }, 2.0);
        assert!(canvas.ink_count() > 0);
        canvas = Canvas::new(80, 40);
        render_aha_plate(&mut canvas, AhaBeat::Confirm, 5.0);
        assert!(canvas.ink_count() > 0);
    }

    #[test]
    fn confirm_multiplier_changes_the_plate() {
        let mut a = Canvas::new(64, 32);
        let mut b = Canvas::new(64, 32);
        render_aha_plate(&mut a, AhaBeat::Confirm, 2.0);
        render_aha_plate(&mut b, AhaBeat::Confirm, 5.0);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn wager_options_mark_three_places() {
        let mut canvas = Canvas::new(90, 40);
        render_wager_options(&mut canvas, Some(CardioidHome::Mandelbrot));
        let text = canvas.to_text();
        assert!(text.contains('M'));
        assert!(text.contains('N'));
        assert!(text.contains('C'));
    }
}
