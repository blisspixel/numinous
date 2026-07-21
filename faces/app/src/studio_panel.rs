//! App-local Studio input, parsing, audio, and drawing helpers.

use std::f64::consts::TAU;

use numinous_core::{Expr, MAX_STUDIO_SOURCE_CHARS, Raster, SoundSpec, Surface};

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

/// Curated Formula Jam recipes: each must parse and make a melody.
/// Random discovery draws only from this bank, never free assembly.
pub(crate) const STUDIO_RECIPES: &[&str] = &[
    "sin(a*x) + x/3",
    "sin(x) + sin(2*x)/2",
    "cos(x)*sin(a*x)",
    "abs(sin(x))",
    "x^2/12 - 1",
    "sin(x) + cos(a*x)/2",
    "sin(3*x)/3 + sin(x)",
    "cos(x + a) + x/8",
    "abs(x)/3 - cos(x)",
    "sin(a*x) * cos(x)",
    "x/4 + sin(2*x)",
    "cos(x)^2 - sin(x)^2",
];

/// Short vocabulary for the Studio help overlay (never permanent chrome).
pub(crate) const STUDIO_HELP_LINES: &[&str] = &[
    "FORMULA JAM",
    "TYPE: BUILD A CURVE  (Y = ...)",
    "F2: RANDOM RECIPE FROM THE BANK",
    "F3: AUTO SET  (~21S, PHRASE SAFE)",
    "F1: TOGGLE THIS HELP",
    "TAB / ESC: CLOSE STUDIO",
    "A IN A FORMULA IS TIME",
    "EDITING PAUSES AUTO",
];

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
        };
        let _ = panel.reparse();
        Ok(panel)
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
    pub fn reparse(&mut self) -> Option<SoundSpec> {
        self.morph = None;
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
            self.source.push(' ');
            return true;
        }
        false
    }

    /// Render the last-good expression into the same deterministic Studio voice.
    pub(crate) fn current_sound(&self) -> Option<SoundSpec> {
        self.expr.as_ref().map(sound_for_expression)
    }

    /// Current UTF-8 byte length, used only to detect an admitted native edit.
    pub(crate) fn source_len(&self) -> usize {
        self.source.len()
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
    pub(crate) fn draw(
        &self,
        raster: &mut Raster,
        mode: InputMode,
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
            let footer = input_legend::studio_controls(mode);
            raster.clear_rows(height as i32 - 16 * scale, height as i32);
            numinous_core::draw_text(raster, &footer, 10, height as i32 - 11 * scale, scale, '#');
        }

        if self.expr.is_none() {
            return;
        }
        let a = t * TAU;
        let (xmin, xmax) = (-TAU, TAU);
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
