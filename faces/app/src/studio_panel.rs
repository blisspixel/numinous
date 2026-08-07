//! App-local Studio input, parsing, audio, and drawing helpers.

use std::f64::consts::TAU;

use numinous_core::{Expr, MAX_STUDIO_SOURCE_CHARS, Raster, SoundSpec, StudioCreation, Surface};

use crate::input_legend::{self, InputMode};

fn studio_scale(width: usize) -> i32 {
    (width as i32 / 450).clamp(1, 3)
}

const DEFAULT_SOURCE: &str = "sin(a*x) + x/3";

/// Target seconds a recipe holds before Auto looks for a phrase boundary.
pub(crate) const AUTO_DWELL_SECONDS: f64 = 21.0;
/// Shared visual and audio duration for a curated recipe transition.
pub(crate) const RECIPE_MORPH_SECONDS: f64 = 0.6;
/// Phrase grid in gallery phase units: Auto advances only near these edges
/// after the dwell, so recipe changes land on musical-ish boundaries.
const AUTO_PHRASE_SLICES: f64 = 8.0;
const AUTO_PHRASE_EDGE: f64 = 0.06;

fn sound_for_expression(expr: &Expr) -> SoundSpec {
    numinous_core::to_melody(expr, -TAU, TAU, 32, 1.0)
}

/// App-local alias of the shared curated bank (core owns the list).
pub(crate) const STUDIO_RECIPES: &[&str] = numinous_core::STUDIO_RECIPES;

/// Short vocabulary for the Studio help overlay (never permanent chrome).
pub(crate) const STUDIO_HELP_LINES: &[&str] = &[
    "FORMULA JAM",
    "TYPE: BUILD A CURVE  (Y = ...)",
    "F2: RANDOM RECIPE FROM THE BANK",
    "F3: AUTO SET  (~21S, PHRASE SAFE)",
    "F4: SHARE  .NUM + LINK + PNG",
    "F5: GALLERY  THE SAVED WALL",
    "F1: TOGGLE THIS HELP",
    "TAB / ESC: CLOSE STUDIO",
    "A IN A FORMULA IS TIME",
    "EDITING PAUSES AUTO",
];

/// A reopened `.num` creation, held whole until the player takes over.
///
/// While this is present the panel draws the saved window instead of the
/// ambient one and pins `a` to the saved value instead of the gallery phase,
/// which is what makes a reopen exact rather than approximate. The complete
/// capsule is kept, not just its numbers: an untouched reopen must re-share
/// with its title, author, and lineage intact, and rebuilding from the
/// window alone would silently strip them. It opens in a paused preview, the
/// hostile-input posture for shared content: the curve is drawn, the voice
/// waits for the player. Any edit releases the pin, because from the first
/// keystroke the creation is theirs, not the file's.
#[derive(Debug, Clone)]
struct OpenedCreation {
    creation: StudioCreation,
    paused: bool,
}

/// Why the Studio has nothing to share right now. Two different problems
/// must not wear one banner: telling a player to fix a formula that parses
/// fine points them at the wrong cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShareRefusal {
    /// The typed source does not parse; there is no curve to promise.
    UnparsedFormula,
    /// The formula is fine, but the recorded parent link cannot ride in a
    /// capsule's descends field.
    LineageTooLarge,
}

#[derive(Debug, Clone)]
struct CurveMorph {
    from: Expr,
    elapsed: f64,
}

impl CurveMorph {
    fn progress(&self) -> f64 {
        let linear = (self.elapsed / RECIPE_MORPH_SECONDS).clamp(0.0, 1.0);
        linear * linear * (3.0 - 2.0 * linear)
    }
}

/// The app-local Studio panel state.
#[derive(Debug, Clone)]
pub struct StudioPanel {
    source: String,
    expr: Option<Expr>,
    error: Option<String>,
    /// Recipe index for Random (advances each draw).
    recipe_cursor: u64,
    /// Auto set: calm recipe rotation while the player watches.
    auto_active: bool,
    /// Seconds on the current recipe under Auto.
    auto_elapsed: f64,
    /// Previous valid recipe while a curated transition is visible.
    morph: Option<CurveMorph>,
    /// Dismissible help overlay; open by default on first Studio entry.
    show_help: bool,
    /// A reopened creation's saved window and knob, until the player edits.
    opened: Option<OpenedCreation>,
    /// The parent link while this Studio session is a fork. Edits keep it,
    /// because edits are the remix; a fresh open or a recipe draw clears it,
    /// because those are a different creation, not a descent.
    fork_of: Option<String>,
}

impl Default for StudioPanel {
    fn default() -> Self {
        Self::new(DEFAULT_SOURCE).expect("default Studio source is within the portable limit")
    }
}

impl StudioPanel {
    /// Build a Studio panel from source text.
    pub fn new(source: &str) -> Result<Self, String> {
        if source.chars().count() > MAX_STUDIO_SOURCE_CHARS {
            return Err(format!(
                "Studio expression is too long; limit is {MAX_STUDIO_SOURCE_CHARS} characters"
            ));
        }
        let mut panel = Self {
            source: source.to_string(),
            expr: None,
            error: None,
            // Start at 1 so the first Random draw is not the default recipe.
            recipe_cursor: 1,
            auto_active: false,
            auto_elapsed: 0.0,
            morph: None,
            // First contact shows Help once; F1 recalls it after dismiss.
            show_help: true,
            opened: None,
            fork_of: None,
        };
        let _ = panel.reparse();
        Ok(panel)
    }

    /// Reopen a saved creation exactly: its source, window, and knob.
    ///
    /// Opens as a paused preview. The curve draws over the saved window at the
    /// saved `a`; the voice waits for [`Self::confirm_opened`]. Help closes so
    /// the creation is what the player sees.
    pub fn open_creation(&mut self, creation: &StudioCreation) {
        self.pause_auto();
        self.morph = None;
        self.show_help = false;
        // Opening a creation is a fresh start, not a descent; the fork path
        // below sets its own parent after this clears.
        self.fork_of = None;
        self.source = creation.source().to_string();
        // Parse directly rather than through reparse, which is the edit door
        // and releases the pin this method exists to set. A validated
        // creation always parses; if this seatbelt branch ever fires anyway,
        // the panel refuses the pin and keeps nothing of the previous curve,
        // so an error and a pin still cannot coexist and a stale expression
        // cannot draw under a window that was never its own.
        match numinous_core::parse(&self.source) {
            Ok(expr) => {
                self.expr = Some(expr);
                self.error = None;
                self.opened = Some(OpenedCreation {
                    creation: creation.clone(),
                    paused: true,
                });
            }
            Err(message) => {
                self.expr = None;
                self.error = Some(message);
                self.opened = None;
            }
        }
    }

    /// Whether a reopened creation is waiting in its paused preview.
    ///
    /// The run path reads the pin through the panel's own drawing and through
    /// [`Self::confirm_opened`]; these observers exist for the tests that
    /// prove the pin's lifecycle.
    #[cfg(test)]
    pub(crate) fn opened_paused(&self) -> bool {
        self.opened.as_ref().is_some_and(|opened| opened.paused)
    }

    /// Whether a reopened creation still pins the window and knob.
    #[cfg(test)]
    pub(crate) fn opened_active(&self) -> bool {
        self.opened.is_some()
    }

    /// Test-only: inject a parent link the production fork path cannot make,
    /// so the refusal for an unshareable lineage stays proven even while no
    /// real `to_link` can exceed the descends cap.
    #[cfg(test)]
    pub(crate) fn force_fork_of(&mut self, link: String) {
        self.fork_of = Some(link);
    }

    /// Fork a creation: open it as the player's own, editable and singing,
    /// and remember whose it was.
    ///
    /// No paused preview here: the player browsed the wall and chose the
    /// fork gesture, and fork must be as cheap as play. The descent rides
    /// every share until the Studio moves to a wholly different creation;
    /// edits keep it, because edits are the remix.
    pub fn fork_creation(&mut self, creation: &StudioCreation) -> Option<SoundSpec> {
        self.open_creation(creation);
        self.fork_of = Some(creation.to_link());
        if let Some(opened) = self.opened.as_mut() {
            opened.paused = false;
        }
        self.current_sound()
    }

    /// Confirm the paused preview: the creation starts singing.
    ///
    /// Returns the melody over the saved window at the saved knob, or `None`
    /// when nothing is waiting to be confirmed.
    pub fn confirm_opened(&mut self) -> Option<SoundSpec> {
        let opened = self.opened.as_mut()?;
        if !opened.paused {
            return None;
        }
        opened.paused = false;
        self.current_sound()
    }

    /// Whether Auto is rotating the recipe bank.
    #[must_use]
    pub fn auto_active(&self) -> bool {
        self.auto_active
    }

    /// Whether the help overlay is visible.
    #[must_use]
    pub fn help_visible(&self) -> bool {
        self.show_help
    }

    /// Pause Auto (edits and explicit discovery controls call this).
    pub fn pause_auto(&mut self) {
        self.auto_active = false;
        self.auto_elapsed = 0.0;
    }

    /// Toggle Auto set. Resuming resets the dwell clock on the current recipe.
    pub fn toggle_auto(&mut self) {
        if self.auto_active {
            self.pause_auto();
        } else {
            self.auto_active = true;
            self.auto_elapsed = 0.0;
        }
    }

    /// Toggle the help overlay. Dismissal is remembered until recalled.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Load the next curated recipe. Returns a melody when the recipe parses.
    /// Does not pause Auto (bank rotation is the Auto path too).
    pub fn load_random_recipe(&mut self) -> Option<SoundSpec> {
        if self.morph.is_some() {
            return None;
        }
        let previous = self.expr.clone();
        let len = STUDIO_RECIPES.len() as u64;
        let index = (self.recipe_cursor % len) as usize;
        self.recipe_cursor = self.recipe_cursor.saturating_add(1);
        // A recipe draw replaces the whole creation, so any fork descent
        // ends here: the bank's curve does not descend from the wall's.
        self.fork_of = None;
        self.source = STUDIO_RECIPES[index].to_string();
        self.auto_elapsed = 0.0;
        let spec = self.reparse();
        if spec.is_some()
            && let Some(from) = previous
            && self.expr.as_ref().is_some_and(|current| current != &from)
        {
            self.morph = Some(CurveMorph { from, elapsed: 0.0 });
        }
        spec
    }

    /// Advance Auto when dwell and phrase edge are both ready.
    ///
    /// `phase` is the app gallery phase in [0, 1). After [`AUTO_DWELL_SECONDS`],
    /// the next recipe loads only near an 1/8-phase edge so changes do not cut
    /// mid-gesture. Returns a new melody when a recipe advances.
    pub fn tick_auto(&mut self, dt: f64, phase: f64) -> Option<SoundSpec> {
        if !(dt.is_finite() && dt > 0.0) {
            return None;
        }
        // App already bounds frame dt; cap runaway values without starving tests.
        let dt = dt.min(AUTO_DWELL_SECONDS);
        if !self.auto_active {
            return None;
        }
        self.auto_elapsed = (self.auto_elapsed + dt).min(AUTO_DWELL_SECONDS * 4.0);
        if self.auto_elapsed < AUTO_DWELL_SECONDS {
            return None;
        }
        let phase = if phase.is_finite() {
            phase.rem_euclid(1.0)
        } else {
            0.0
        };
        let edge = (phase * AUTO_PHRASE_SLICES).fract();
        if edge > AUTO_PHRASE_EDGE && edge < (1.0 - AUTO_PHRASE_EDGE) {
            return None;
        }
        self.load_random_recipe()
    }

    /// Re-parse the Studio text, keeping the last good curve alive on errors.
    ///
    /// This is the edit door, so it releases a reopened creation's pin: from
    /// the first keystroke the window and knob are the player's again.
    pub fn reparse(&mut self) -> Option<SoundSpec> {
        self.morph = None;
        self.opened = None;
        match numinous_core::parse(&self.source) {
            Ok(expr) => {
                let spec = sound_for_expression(&expr);
                self.expr = Some(expr);
                self.error = None;
                Some(spec)
            }
            Err(message) => {
                self.error = Some(message);
                None
            }
        }
    }

    /// Remove one character and reparse. Editing pauses Auto.
    pub fn backspace(&mut self) -> Option<SoundSpec> {
        self.pause_auto();
        self.source.pop();
        self.reparse()
    }

    /// Append ordinary text and reparse. Editing pauses Auto.
    pub fn push_text(&mut self, text: &str) -> Option<SoundSpec> {
        if !self.can_append(text) {
            return None;
        }
        self.pause_auto();
        self.source.push_str(text);
        self.reparse()
    }

    /// Append a literal space. This preserves the current parse state, matching
    /// the old event-loop behavior. Editing pauses Auto. Returns whether the
    /// portable source bound admitted the space.
    pub fn push_space(&mut self) -> bool {
        if self.can_append(" ") {
            self.pause_auto();
            self.morph = None;
            self.opened = None;
            self.source.push(' ');
            return true;
        }
        false
    }

    /// Render the last-good expression into the same deterministic Studio voice.
    ///
    /// A reopened creation sings its own saved window at its saved knob;
    /// everything else sings the ambient default.
    pub(crate) fn current_sound(&self) -> Option<SoundSpec> {
        let expr = self.expr.as_ref()?;
        if let Some(opened) = &self.opened {
            return Some(numinous_core::to_melody(
                expr,
                opened.creation.xmin(),
                opened.creation.xmax(),
                32,
                opened.creation.a(),
            ));
        }
        Some(sound_for_expression(expr))
    }

    /// Current UTF-8 byte length, used only to detect an admitted native edit.
    pub(crate) fn source_len(&self) -> usize {
        self.source.len()
    }

    /// The window and knob the panel is presenting at moment `t`: a reopened
    /// pin's own saved values, or the ambient default window with the knob as
    /// time.
    ///
    /// One helper for the screen, the share, and the postcard, because three
    /// copies of this expression is how the share stops being the curve on
    /// screen. The moment is deliberately not normalized: the console can
    /// park the phase at exactly 1.0, where a wrap would hand the share
    /// `a = 0` while the screen draws `a = tau`. Only a non-finite moment
    /// falls back, to zero, on every surface alike.
    fn window_and_knob(&self, t: f64) -> (f64, f64, f64) {
        match &self.opened {
            Some(opened) => (
                opened.creation.xmin(),
                opened.creation.xmax(),
                opened.creation.a(),
            ),
            None => {
                let a = if t.is_finite() { t * TAU } else { 0.0 };
                (-TAU, TAU, a)
            }
        }
    }

    /// The current Studio state as a shareable creation, or `None` while the
    /// typed source does not parse: an unparsed edit has no curve to promise,
    /// so it is refused rather than shared as whatever last happened to work.
    ///
    /// A reopened pin shares its saved window and knob. The ambient Studio
    /// shares the default window with the knob frozen at this moment's phase,
    /// so the shared creation is the exact curve on screen when the player
    /// pressed the key, not a moving target.
    pub(crate) fn current_creation(&self, t: f64) -> Result<StudioCreation, ShareRefusal> {
        if self.error.is_some() || self.expr.is_none() {
            return Err(ShareRefusal::UnparsedFormula);
        }
        // An untouched reopen shares the very capsule that was opened,
        // identity and all: rebuilding it from the window alone would
        // silently strip the title, author, and lineage the format exists
        // to preserve. A fork deliberately does not take this path, because
        // a fork is a new creation descending from the parent, not the
        // parent wearing its own name.
        if self.fork_of.is_none()
            && let Some(opened) = &self.opened
        {
            return Ok(opened.creation.clone());
        }
        let (xmin, xmax, a) = self.window_and_knob(t);
        let mut creation = StudioCreation::new(self.source.clone(), xmin, xmax, a)
            .map_err(|_| ShareRefusal::UnparsedFormula)?;
        if let Some(parent) = &self.fork_of {
            // A fork shares its descent, and a lineage that cannot ride is
            // its own refusal rather than a claim that the formula broke.
            creation = creation
                .with_descends(parent)
                .map_err(|_| ShareRefusal::LineageTooLarge)?;
        }
        Ok(creation)
    }

    /// Render the current curve as a square postcard frame: title, formula,
    /// and the curve over the same window and knob a share would save.
    ///
    /// No footer, help, or cursor: a postcard is the creation, not the
    /// editing session around it.
    pub(crate) fn postcard_rgba(&self, t: f64, size: usize, era: numinous_core::Era) -> Vec<u8> {
        let mut raster = Raster::new(size, size);
        let scale = studio_scale(size).max(2);
        numinous_core::draw_text(&mut raster, "NUMINOUS STUDIO", 10, 10, scale, '#');
        let typed = format!("Y = {}", self.source.to_uppercase());
        numinous_core::draw_text(&mut raster, &typed, 10, 10 + 12 * scale, scale + 1, '#');
        if let Some(expr) = &self.expr {
            let (xmin, xmax, a) = self.window_and_knob(t);
            // The postcard must match what creation.num reopens, so it
            // evaluates the settled expression directly rather than through
            // curve_value, whose recipe-morph blend is a 600 ms presentation
            // effect the capsule does not record. A share taken mid-morph
            // stays self-consistent instead of shipping a picture no reopen
            // can reproduce.
            let _ = numinous_app::studio_render::draw_curve(
                &mut raster,
                numinous_app::studio_render::CurveLayout {
                    width: size,
                    height: size,
                    top: f64::from(60 * scale),
                    bottom_margin: f64::from(24 * scale),
                },
                xmin,
                xmax,
                |x| {
                    let value = numinous_core::eval(expr, x, a);
                    value.is_finite().then_some(value)
                },
            );
        }
        let mut rgba = raster.to_rgba();
        era.apply(&mut rgba, size, size);
        rgba
    }

    pub(crate) fn advance_morph(&mut self, dt: f64) {
        if !(dt.is_finite() && dt > 0.0) {
            return;
        }
        let Some(morph) = self.morph.as_mut() else {
            return;
        };
        morph.elapsed = (morph.elapsed + dt).min(RECIPE_MORPH_SECONDS);
        if morph.elapsed >= RECIPE_MORPH_SECONDS {
            self.morph = None;
        }
    }

    fn curve_value(&self, x: f64, a: f64) -> Option<f64> {
        let current = self
            .expr
            .as_ref()
            .map(|expr| numinous_core::eval(expr, x, a))
            .filter(|value| value.is_finite());
        let Some(morph) = &self.morph else {
            return current;
        };
        let previous =
            Some(numinous_core::eval(&morph.from, x, a)).filter(|value| value.is_finite());
        match (previous, current) {
            (Some(from), Some(to)) => Some(from + (to - from) * morph.progress()),
            (Some(from), None) => Some(from),
            (None, current) => current,
        }
    }

    fn can_append(&self, text: &str) -> bool {
        let current = self.source.chars().count();
        let Some(remaining) = MAX_STUDIO_SOURCE_CHARS.checked_sub(current) else {
            return false;
        };
        text.chars().take(remaining + 1).count() <= remaining
    }

    /// Draw the Studio panel into the raster.
    #[cfg(test)]
    pub(crate) fn draw(
        &self,
        raster: &mut Raster,
        mode: InputMode,
        width: usize,
        height: usize,
        t: f64,
    ) {
        self.draw_with_controller(
            raster,
            mode,
            crate::input_legend::ControllerFace::Generic.into(),
            width,
            height,
            t,
        );
    }

    pub(crate) fn draw_with_controller(
        &self,
        raster: &mut Raster,
        mode: InputMode,
        copy: crate::input_legend::ControllerCopy,
        width: usize,
        height: usize,
        t: f64,
    ) {
        let width = width.min(raster.width());
        let height = height.min(raster.height());
        let scale = studio_scale(width);
        let title = if self.auto_active() {
            "THE STUDIO  AUTO"
        } else {
            "THE STUDIO"
        };
        numinous_core::draw_text(raster, title, 10, 10, scale, '#');
        let typed = format!("Y = {}_", self.source.to_uppercase());
        numinous_core::draw_text(raster, &typed, 10, 10 + 12 * scale, scale + 1, '#');
        if let Some(error) = &self.error {
            numinous_core::draw_text(
                raster,
                &error.to_uppercase(),
                10,
                10 + 34 * scale,
                scale,
                '-',
            );
        } else if let Some(opened) = &self.opened {
            // The pin and the error share a row because they cannot coexist:
            // editing releases the pin before it can produce a parse error.
            let (xmin, xmax, a) = (
                opened.creation.xmin(),
                opened.creation.xmax(),
                opened.creation.a(),
            );
            let line = if opened.paused {
                format!("REOPENED  X {xmin:.1} TO {xmax:.1}  A {a:.2}  ENTER: PLAY")
            } else {
                format!("REOPENED  X {xmin:.1} TO {xmax:.1}  A {a:.2}  TYPE: TAKE OVER")
            };
            numinous_core::draw_text(raster, &line, 10, 10 + 34 * scale, scale, '*');
        }
        if self.help_visible() && height > 40 {
            let help_top = 10 + 48 * scale;
            raster.clear_rows(
                help_top - 4,
                help_top + STUDIO_HELP_LINES.len() as i32 * 10 * scale + 4,
            );
            for (i, line) in STUDIO_HELP_LINES.iter().enumerate() {
                numinous_core::draw_text(
                    raster,
                    line,
                    10,
                    help_top + i as i32 * 10 * scale,
                    scale,
                    '*',
                );
            }
        }
        if height >= 20 {
            let footer = input_legend::studio_controls_with_controller(mode, copy);
            raster.clear_rows(height as i32 - 16 * scale, height as i32);
            numinous_core::draw_text(raster, &footer, 10, height as i32 - 11 * scale, scale, '#');
        }

        if self.expr.is_none() {
            return;
        }
        // A reopened creation draws its saved window at its saved knob; the
        // ambient Studio draws the default window with the knob as time. The
        // same helper feeds the share and the postcard, so what is saved is
        // what is on screen by construction.
        let (xmin, xmax, a) = self.window_and_knob(t);
        let top = (60 * scale) as f64;
        let _ = numinous_app::studio_render::draw_curve(
            raster,
            numinous_app::studio_render::CurveLayout {
                width,
                height,
                top,
                bottom_margin: f64::from(24 * scale),
            },
            xmin,
            xmax,
            |x| self.curve_value(x, a),
        );
    }

    #[cfg(test)]
    pub(crate) fn source_for_test(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AUTO_DWELL_SECONDS, MAX_STUDIO_SOURCE_CHARS, RECIPE_MORPH_SECONDS, STUDIO_RECIPES,
        StudioPanel, studio_scale,
    };
    use crate::input_legend::{self, InputMode};
    use numinous_core::Raster;

    #[test]
    fn default_panel_has_a_curve_and_a_voice() {
        let mut panel = StudioPanel::default();
        let spec = panel.reparse().expect("melody");
        assert_eq!(panel.source, "sin(a*x) + x/3");
        assert!(panel.expr.is_some());
        assert!(panel.error.is_none());
        assert_eq!(spec.notes.len(), 32);
    }

    #[test]
    fn bad_edits_keep_the_last_good_curve_alive() {
        let mut panel = StudioPanel::new("x").expect("panel");
        assert!(panel.push_text("@").is_none());
        assert!(panel.error.is_some());
        assert!(panel.expr.is_some());
        let mut raster = Raster::new(120, 90);
        panel.draw(&mut raster, InputMode::KeyboardMouse, 120, 90, 0.25);
        assert!(raster.lit_count() > 0, "last good curve should still draw");
    }

    #[test]
    fn draw_handles_tiny_and_mismatched_sizes() {
        let mut panel = StudioPanel::new("sin(x)").expect("panel");
        let mut zero = Raster::new(0, 0);
        panel.draw(&mut zero, InputMode::KeyboardMouse, 0, 0, 0.0);
        assert_eq!(zero.lit_count(), 0);

        let mut one = Raster::new(1, 1);
        panel.draw(&mut one, InputMode::KeyboardMouse, 1, 1, 0.0);

        let mut short = Raster::new(80, 20);
        panel.draw(&mut short, InputMode::KeyboardMouse, 500, 20, 0.0);
        assert!(short.lit_count() > 0);

        let mut mismatched = Raster::new(24, 90);
        panel.draw(&mut mismatched, InputMode::KeyboardMouse, 200, 90, 0.5);
        assert!(mismatched.lit_count() > 0);

        panel.toggle_auto();
        let mut auto = Raster::new(120, 90);
        panel.draw(&mut auto, InputMode::KeyboardMouse, 120, 90, 0.5);
        assert!(auto.lit_count() > 0);

        panel.expr = None;
        let mut no_expression = Raster::new(120, 90);
        panel.draw(&mut no_expression, InputMode::KeyboardMouse, 120, 90, 0.5);

        let non_finite = StudioPanel::new("1/0").expect("parseable non-finite expression");
        let mut no_samples = Raster::new(120, 90);
        non_finite.draw(&mut no_samples, InputMode::KeyboardMouse, 120, 90, 0.5);
    }

    #[test]
    fn editing_operations_update_source_predictably() {
        let mut panel = StudioPanel::new("x").expect("panel");
        assert_eq!(panel.source_len(), 1);
        assert!(panel.push_space());
        assert_eq!(panel.source, "x ");
        assert!(panel.push_text("+ 1").is_some());
        assert_eq!(panel.source, "x + 1");
        assert!(panel.backspace().is_none());
        assert_eq!(panel.source, "x + ");
        assert!(panel.error.is_some());
        assert!(
            panel.current_sound().is_some(),
            "an invalid edit must retain a playable last-good expression"
        );
    }

    #[test]
    fn every_recipe_parses_and_random_cycles_the_bank() {
        for recipe in STUDIO_RECIPES {
            let panel = StudioPanel::new(recipe).expect("recipe must be portable");
            assert!(
                panel.error.is_none(),
                "recipe {recipe:?} must parse cleanly"
            );
            assert!(panel.expr.is_some(), "recipe {recipe:?} must yield an expr");
        }
        let mut panel = StudioPanel::default();
        let first = panel.source.clone();
        assert!(panel.load_random_recipe().is_some());
        assert_ne!(panel.source, first);
        panel.advance_morph(RECIPE_MORPH_SECONDS);
        // Cursor starts at 1 (second recipe); after remaining bank draws, wrap.
        for _ in 1..STUDIO_RECIPES.len() {
            assert!(panel.load_random_recipe().is_some());
            panel.advance_morph(RECIPE_MORPH_SECONDS);
        }
        assert_eq!(panel.source, STUDIO_RECIPES[0]);
    }

    #[test]
    fn auto_waits_for_dwell_and_phrase_edge_then_advances() {
        let mut panel = StudioPanel::default();
        let start = panel.source.clone();
        assert!(panel.tick_auto(1.0, 0.3).is_none(), "Auto is inactive");
        panel.toggle_auto();
        assert!(panel.auto_active());
        assert!(
            panel.tick_auto(f64::NAN, 0.3).is_none(),
            "bad time is inert"
        );
        // Phase 0.3 sits between 1/8 edges (0.3 * 8 = 2.4).
        assert!(panel.tick_auto(1.0, 0.3).is_none(), "dwell not met");
        assert_eq!(panel.source, start);
        // Dwell complete but mid-phrase: still wait.
        assert!(
            panel.tick_auto(AUTO_DWELL_SECONDS, 0.3).is_none(),
            "mid-phrase must not cut"
        );
        assert_eq!(panel.source, start);
        // Near a phrase edge after dwell: advance.
        let advanced = panel.tick_auto(0.1, f64::NAN);
        assert!(advanced.is_some(), "phrase edge after dwell advances");
        assert_ne!(panel.source, start);
        assert!(panel.morph.is_some(), "Auto begins one recipe morph");
    }

    #[test]
    fn recipe_morph_interpolates_exact_endpoints_and_finishes_on_time() {
        let mut panel = StudioPanel::default();
        let (x, a) = (0.73, 0.41);
        let old = numinous_core::eval(panel.expr.as_ref().expect("opening expression"), x, a);

        assert!(panel.load_random_recipe().is_some());
        let new = numinous_core::eval(panel.expr.as_ref().expect("new expression"), x, a);
        assert!((old - new).abs() > 1.0e-3, "fixture must expose the morph");
        assert!((panel.curve_value(x, a).expect("morph start") - old).abs() < 1.0e-12);

        panel.advance_morph(RECIPE_MORPH_SECONDS / 2.0);
        let halfway = panel.curve_value(x, a).expect("halfway morph");
        assert!((halfway - (old + new) / 2.0).abs() < 1.0e-12);

        panel.advance_morph(RECIPE_MORPH_SECONDS / 2.0);
        assert!(panel.morph.is_none());
        assert!((panel.curve_value(x, a).expect("morph end") - new).abs() < 1.0e-12);

        let invalid = numinous_core::parse("1/0").expect("parseable non-finite expression");
        let valid = panel.expr.clone().expect("valid target expression");
        panel.morph = Some(super::CurveMorph {
            from: valid.clone(),
            elapsed: RECIPE_MORPH_SECONDS / 2.0,
        });
        panel.expr = Some(invalid.clone());
        assert!((panel.curve_value(x, a).expect("finite previous") - new).abs() < 1.0e-12);
        panel.morph = Some(super::CurveMorph {
            from: invalid,
            elapsed: RECIPE_MORPH_SECONDS / 2.0,
        });
        panel.expr = Some(valid);
        assert!((panel.curve_value(x, a).expect("finite current") - new).abs() < 1.0e-12);
    }

    #[test]
    fn manual_edit_cancels_recipe_morph_and_invalid_time_cannot_advance_it() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        assert!(panel.morph.is_some());

        panel.advance_morph(f64::NAN);
        assert_eq!(panel.morph.as_ref().expect("morph remains").elapsed, 0.0);

        assert!(panel.push_text("(").is_none());
        assert!(panel.morph.is_none());
        assert!(panel.current_sound().is_some());
    }

    #[test]
    fn presentation_time_advances_a_morph_without_advancing_auto() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        let cursor = panel.recipe_cursor;

        panel.advance_morph(RECIPE_MORPH_SECONDS);

        assert!(panel.morph.is_none());
        panel.advance_morph(0.1);
        assert_eq!(panel.recipe_cursor, cursor);
        assert_eq!(panel.auto_elapsed, 0.0);
    }

    #[test]
    fn repeated_recipe_request_cannot_jump_an_active_morph() {
        let mut panel = StudioPanel::default();
        assert!(panel.load_random_recipe().is_some());
        let source = panel.source.clone();
        let cursor = panel.recipe_cursor;

        assert!(panel.load_random_recipe().is_none());
        assert_eq!(panel.source, source);
        assert_eq!(panel.recipe_cursor, cursor);
        assert_eq!(panel.morph.as_ref().expect("morph remains").elapsed, 0.0);
    }

    #[test]
    fn editing_pauses_auto_and_help_toggles() {
        let mut panel = StudioPanel::default();
        assert!(panel.help_visible(), "first contact shows help");
        panel.toggle_help();
        assert!(!panel.help_visible());
        panel.toggle_help();
        assert!(panel.help_visible());

        panel.toggle_auto();
        assert!(panel.auto_active());
        panel.toggle_auto();
        assert!(!panel.auto_active());
        panel.toggle_auto();
        assert!(panel.auto_active());
        let _ = panel.push_text("+0");
        assert!(
            !panel.auto_active(),
            "typing must pause Auto so the player owns the formula"
        );
        panel.toggle_auto();
        assert!(panel.auto_active());
        let _ = panel.backspace();
        assert!(!panel.auto_active());
    }

    #[test]
    fn a_reopened_creation_pins_window_and_knob_and_waits_paused() {
        use std::f64::consts::TAU;
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.toggle_auto();
        assert!(panel.auto_active());

        panel.open_creation(&creation);
        assert!(!panel.auto_active(), "a reopen pauses Auto");
        assert!(
            !panel.help_visible(),
            "the creation is what the player sees"
        );
        assert!(panel.opened_active());
        assert!(panel.opened_paused());
        assert_eq!(panel.source_for_test(), "sin(a*x)");

        // Exact: the voice is the saved window at the saved knob, not the
        // ambient default of either.
        let expr = numinous_core::parse("sin(a*x)").expect("expr");
        let exact = numinous_core::to_melody(&expr, 0.0, 1.0, 32, 0.25);
        assert_ne!(
            exact,
            numinous_core::to_melody(&expr, -TAU, TAU, 32, 1.0),
            "fixture must expose the pin"
        );
        assert_eq!(panel.current_sound().expect("voice"), exact);

        // Confirm starts the singing once; a second confirm has nothing left.
        assert_eq!(panel.confirm_opened().expect("confirmed melody"), exact);
        assert!(!panel.opened_paused());
        assert!(panel.opened_active(), "confirming does not release the pin");
        assert!(panel.confirm_opened().is_none());
    }

    #[test]
    fn an_edit_releases_the_reopened_pin() {
        use std::f64::consts::TAU;
        let creation =
            numinous_core::StudioCreation::new("sin(a*x)", 0.0, 1.0, 0.25).expect("creation");
        let mut panel = StudioPanel::default();
        panel.open_creation(&creation);
        assert!(panel.confirm_opened().is_some());

        // From the first keystroke the window and knob are the player's.
        let spec = panel.push_text("+0").expect("still parses");
        assert!(!panel.opened_active());
        let expr = numinous_core::parse("sin(a*x)+0").expect("expr");
        assert_eq!(spec, numinous_core::to_melody(&expr, -TAU, TAU, 32, 1.0));

        // A space is an edit too, even though it keeps the parse state.
        panel.open_creation(&creation);
        assert!(panel.opened_paused());
        assert!(panel.push_space());
        assert!(!panel.opened_active());

        // So is a recipe draw: the bank replaces the reopened source.
        panel.open_creation(&creation);
        assert!(panel.load_random_recipe().is_some());
        assert!(!panel.opened_active());
    }

    #[test]
    fn the_reopened_window_is_the_window_the_curve_draws() {
        // Same source, two saved windows: the curve band must differ, or the
        // reopen would only be echoing the numbers while drawing the ambient
        // window anyway.
        let narrow = numinous_core::StudioCreation::new("sin(x)", 0.0, 1.0, 1.0).expect("narrow");
        let wide = numinous_core::StudioCreation::new("sin(x)", -6.0, 6.0, 1.0).expect("wide");
        let curve_band = |creation: &numinous_core::StudioCreation| {
            let mut panel = StudioPanel::default();
            panel.open_creation(creation);
            let mut raster = Raster::new(200, 150);
            panel.draw(&mut raster, InputMode::KeyboardMouse, 200, 150, 0.25);
            // Rows inside the curve area only, clear of chrome and footer,
            // so the difference cannot come from the reopened status line.
            raster.to_rgba()[200 * 4 * 70..200 * 4 * 120].to_vec()
        };
        assert_ne!(curve_band(&narrow), curve_band(&wide));
    }

    #[test]
    fn the_shared_creation_is_the_curve_on_screen() {
        use std::f64::consts::TAU;
        // Ambient Studio: the knob freezes at this moment's phase, so the
        // share is the exact curve the player was hearing, not a=1.0 always.
        let panel = StudioPanel::new("sin(a*x)").expect("panel");
        let creation = panel.current_creation(0.25).expect("creation");
        assert_eq!(creation.source(), "sin(a*x)");
        assert!((creation.xmin() + TAU).abs() < 1e-12);
        assert!((creation.xmax() - TAU).abs() < 1e-12);
        assert!((creation.a() - 0.25 * TAU).abs() < 1e-12);

        // The console can park the phase at exactly 1.0. The screen draws
        // a = tau there, so the share must save a = tau, not wrap to zero.
        let parked = panel.current_creation(1.0).expect("parked");
        assert!((parked.a() - TAU).abs() < 1e-12);

        // A reopened pin shares its own saved window and knob.
        let saved = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5).expect("saved");
        let mut reopened = StudioPanel::default();
        reopened.open_creation(&saved);
        let shared = reopened.current_creation(0.75).expect("shared");
        assert_eq!(shared, saved);

        // An unparsed edit has no curve to promise.
        let mut broken = StudioPanel::new("sin(a*x)").expect("panel");
        assert!(broken.push_text("(").is_none());
        assert_eq!(
            broken.current_creation(0.25),
            Err(super::ShareRefusal::UnparsedFormula)
        );

        // A non-finite moment falls back to phase zero rather than a NaN knob.
        assert!(panel.current_creation(f64::NAN).is_ok());
    }

    #[test]
    fn a_fork_sings_at_once_and_its_shares_record_the_descent() {
        let parent = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
            .expect("parent")
            .with_title("Parent Wave")
            .expect("title");
        let mut panel = StudioPanel::default();

        let spec = panel.fork_creation(&parent);
        assert!(spec.is_some(), "a chosen fork sings without a preview");
        assert!(!panel.opened_paused());
        assert!(
            panel.opened_active(),
            "the parent window and knob still pin"
        );

        let shared = panel.current_creation(0.25).expect("shared");
        assert_eq!(shared.descends(), Some(parent.to_link().as_str()));

        // Edits keep the descent, because edits are the remix.
        assert!(panel.push_text("+0").is_some());
        let edited = panel.current_creation(0.25).expect("edited");
        assert_eq!(edited.descends(), Some(parent.to_link().as_str()));

        // A recipe draw is a different creation, not a descent.
        assert!(panel.load_random_recipe().is_some());
        let drawn = panel.current_creation(0.25).expect("recipe");
        assert_eq!(drawn.descends(), None);

        // So is opening something else.
        assert!(panel.fork_creation(&parent).is_some());
        let other = numinous_core::StudioCreation::new("x*x", -1.0, 1.0, 0.0).expect("other");
        panel.open_creation(&other);
        assert_eq!(
            panel.current_creation(0.25).expect("opened").descends(),
            None
        );
    }

    #[test]
    fn an_untouched_reopen_reshares_its_whole_identity() {
        // The capsule format exists to carry title, author, and lineage;
        // re-sharing an unedited reopen must not silently strip them by
        // rebuilding the creation from its numbers alone.
        let grandparent =
            numinous_core::StudioCreation::new("sin(x)", -1.0, 1.0, 0.0).expect("grandparent");
        let full = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
            .expect("creation")
            .with_title("Slow Waves")
            .expect("title")
            .with_author("A Curious Mind")
            .expect("author")
            .with_descends(&grandparent.to_link())
            .expect("descends");
        let mut panel = StudioPanel::default();
        panel.open_creation(&full);
        assert_eq!(
            panel.current_creation(0.25).expect("reshared"),
            full,
            "identity and lineage survive an untouched re-share"
        );

        // The first edit makes it the player's: identity intentionally
        // drops with the pin, and the descent does not follow an open.
        assert!(panel.push_text("+0").is_some());
        let taken_over = panel.current_creation(0.25).expect("taken over");
        assert_eq!(taken_over.title(), None);
        assert_eq!(taken_over.descends(), None);
    }

    #[test]
    fn a_fork_descends_from_its_parent_but_does_not_wear_its_name() {
        let parent = numinous_core::StudioCreation::new("sin(a*x)", 0.0, 2.0, 0.5)
            .expect("parent")
            .with_title("Parent Wave")
            .expect("title");
        let mut panel = StudioPanel::default();
        assert!(panel.fork_creation(&parent).is_some());
        let fork = panel.current_creation(0.25).expect("fork");
        assert_eq!(fork.descends(), Some(parent.to_link().as_str()));
        assert_eq!(
            fork.title(),
            None,
            "a fork is a new creation descending from the parent, not the \
             parent wearing its own name"
        );
    }

    #[test]
    fn an_unshareable_lineage_is_named_not_blamed_on_the_formula() {
        let mut panel = StudioPanel::default();
        // No real to_link can exceed the descends cap today, so the panel is
        // handed an oversized parent directly: the refusal path must stay
        // honest even for states nothing currently produces.
        panel.force_fork_of(format!(
            "numinous://studio?expr=x&xmin=-1&xmax=1&a=0{}",
            "&".repeat(5000)
        ));
        assert_eq!(
            panel.current_creation(0.25),
            Err(super::ShareRefusal::LineageTooLarge),
            "a lineage that cannot ride is its own refusal"
        );
    }

    #[test]
    fn a_mid_morph_postcard_matches_the_capsule_not_the_blend() {
        // The bundle's promise is that postcard.png shows what creation.num
        // reopens. A recipe morph is a 600 ms presentation blend the capsule
        // does not record, so the postcard must ignore it.
        let mut morphing = StudioPanel::default();
        assert!(morphing.load_random_recipe().is_some());
        let mid_morph = morphing.postcard_rgba(0.25, 240, numinous_core::Era::Modern);

        let mut settled = StudioPanel::default();
        assert!(settled.load_random_recipe().is_some());
        settled.advance_morph(RECIPE_MORPH_SECONDS);
        let after = settled.postcard_rgba(0.25, 240, numinous_core::Era::Modern);

        assert_eq!(
            mid_morph, after,
            "the postcard is the settled curve whether or not a morph is \
             mid-flight on screen"
        );
    }

    #[test]
    fn the_postcard_draws_the_shared_window() {
        // Two saved windows, one source: the postcards must differ, or the
        // postcard would be drawing some other window than the one the
        // bundle's creation.num promises to reopen.
        let narrow = numinous_core::StudioCreation::new("sin(x)", 0.0, 1.0, 1.0).expect("narrow");
        let wide = numinous_core::StudioCreation::new("sin(x)", -6.0, 6.0, 1.0).expect("wide");
        let postcard = |creation: &numinous_core::StudioCreation| {
            let mut panel = StudioPanel::default();
            panel.open_creation(creation);
            panel.postcard_rgba(0.25, 300, numinous_core::Era::Modern)
        };
        let narrow_rgba = postcard(&narrow);
        assert!(
            narrow_rgba.iter().any(|&byte| byte > 32),
            "a postcard has ink"
        );
        assert_ne!(narrow_rgba, postcard(&wide));
    }

    #[test]
    fn editing_stops_at_the_portable_source_limit() {
        let mut panel = StudioPanel::new("x").expect("panel");
        for _ in 1..numinous_core::MAX_STUDIO_SOURCE_CHARS {
            panel.push_space();
        }
        assert_eq!(
            panel.source.chars().count(),
            numinous_core::MAX_STUDIO_SOURCE_CHARS
        );

        panel.push_space();
        assert_eq!(
            panel.source.chars().count(),
            numinous_core::MAX_STUDIO_SOURCE_CHARS
        );
        panel.source = "x".repeat(numinous_core::MAX_STUDIO_SOURCE_CHARS + 1);
        assert!(!panel.push_space());
    }

    #[test]
    fn over_limit_character_events_are_rejected_atomically() {
        let source = format!(
            "{}x",
            " ".repeat(numinous_core::MAX_STUDIO_SOURCE_CHARS - 1)
        );
        let mut panel = StudioPanel::new(&source).expect("panel");

        assert!(panel.push_text("+1").is_none());
        assert_eq!(panel.source, source);
        assert!(panel.expr.is_some());
        assert!(panel.error.is_none());
    }

    #[test]
    fn construction_rejects_over_limit_unicode_source() {
        let source = "π".repeat(MAX_STUDIO_SOURCE_CHARS + 1);
        assert!(StudioPanel::new(&source).is_err());
    }

    #[test]
    fn controller_footer_names_the_keyboard_requirement_and_fits() {
        let copy = input_legend::studio_controls(InputMode::Controller);
        assert_eq!(copy, "KEYBOARD TYPES   EAST CLOSES   START HELP");
        assert!(copy.starts_with("KEYBOARD TYPES"));

        for (width, height) in [(360, 240), (900, 700)] {
            let scale = studio_scale(width);
            assert!(
                10 + numinous_core::text_width(&copy, scale) <= width as i32,
                "Studio controls clip at {width}x{height}"
            );
            let panel = StudioPanel::default();
            let mut raster = Raster::new(width, height);
            panel.draw(&mut raster, InputMode::Controller, width, height, 0.25);
            assert!(raster.lit_count() > 100);
        }
    }
}
