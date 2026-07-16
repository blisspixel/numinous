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
/// Phrase grid in gallery phase units: Auto advances only near these edges
/// after the dwell, so recipe changes land on musical-ish boundaries.
const AUTO_PHRASE_SLICES: f64 = 8.0;
const AUTO_PHRASE_EDGE: f64 = 0.06;

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
        let len = STUDIO_RECIPES.len() as u64;
        let index = (self.recipe_cursor % len) as usize;
        self.recipe_cursor = self.recipe_cursor.saturating_add(1);
        self.source = STUDIO_RECIPES[index].to_string();
        self.auto_elapsed = 0.0;
        self.reparse()
    }

    /// Advance Auto when dwell and phrase edge are both ready.
    ///
    /// `phase` is the app gallery phase in [0, 1). After [`AUTO_DWELL_SECONDS`],
    /// the next recipe loads only near an 1/8-phase edge so changes do not cut
    /// mid-gesture. Returns a new melody when a recipe advances.
    pub fn tick_auto(&mut self, dt: f64, phase: f64) -> Option<SoundSpec> {
        if !self.auto_active {
            return None;
        }
        if !(dt.is_finite() && dt > 0.0) {
            return None;
        }
        // App already bounds frame dt; cap runaway values without starving tests.
        let dt = dt.min(AUTO_DWELL_SECONDS);
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
        match numinous_core::parse(&self.source) {
            Ok(expr) => {
                let spec = numinous_core::to_melody(&expr, -TAU, TAU, 32, 1.0);
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
    /// the old event-loop behavior. Editing pauses Auto.
    pub fn push_space(&mut self) {
        if self.can_append(" ") {
            self.pause_auto();
            self.source.push(' ');
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

        let Some(expr) = &self.expr else {
            return;
        };
        if width < 2 {
            return;
        }
        let a = t * TAU;
        let (xmin, xmax) = (-TAU, TAU);
        let samples: Vec<(usize, f64)> = (0..width)
            .map(|i| {
                let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
                (i, numinous_core::eval(expr, x, a))
            })
            .filter(|(_, y)| y.is_finite())
            .collect();
        if samples.is_empty() {
            return;
        }
        let ymin = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let ymax = samples
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let yspan = (ymax - ymin).max(1e-9);
        let top = (60 * scale) as f64;
        let plot_h = height as f64 - top - f64::from(24 * scale);
        if plot_h < 8.0 {
            return;
        }
        let mut previous: Option<(i32, i32)> = None;
        for &(i, y) in &samples {
            let sx = i as i32;
            let sy = (top + (1.0 - (y - ymin) / yspan) * plot_h) as i32;
            if let Some((px, py)) = previous {
                raster.line(px, py, sx, sy, '#');
            }
            previous = Some((sx, sy));
        }
    }

    #[cfg(test)]
    pub(crate) fn source_for_test(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AUTO_DWELL_SECONDS, MAX_STUDIO_SOURCE_CHARS, STUDIO_RECIPES, StudioPanel, studio_scale,
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
        let panel = StudioPanel::new("sin(x)").expect("panel");
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
    }

    #[test]
    fn editing_operations_update_source_predictably() {
        let mut panel = StudioPanel::new("x").expect("panel");
        panel.push_space();
        assert_eq!(panel.source, "x ");
        assert!(panel.push_text("+ 1").is_some());
        assert_eq!(panel.source, "x + 1");
        assert!(panel.backspace().is_none());
        assert_eq!(panel.source, "x + ");
        assert!(panel.error.is_some());
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
        // Cursor starts at 1 (second recipe); after remaining bank draws, wrap.
        for _ in 1..STUDIO_RECIPES.len() {
            assert!(panel.load_random_recipe().is_some());
        }
        assert_eq!(panel.source, STUDIO_RECIPES[0]);
    }

    #[test]
    fn auto_waits_for_dwell_and_phrase_edge_then_advances() {
        let mut panel = StudioPanel::default();
        let start = panel.source.clone();
        panel.toggle_auto();
        assert!(panel.auto_active());
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
        let advanced = panel.tick_auto(0.1, 0.0);
        assert!(advanced.is_some(), "phrase edge after dwell advances");
        assert_ne!(panel.source, start);
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
