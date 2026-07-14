//! App-local Studio input, parsing, audio, and drawing helpers.

use std::f64::consts::TAU;

use numinous_core::{Expr, MAX_STUDIO_SOURCE_CHARS, Raster, SoundSpec, Surface};

use crate::input_legend::{self, InputMode};

fn studio_scale(width: usize) -> i32 {
    (width as i32 / 450).clamp(1, 3)
}

const DEFAULT_SOURCE: &str = "sin(a*x) + x/3";

/// The app-local Studio panel state.
#[derive(Debug, Clone)]
pub struct StudioPanel {
    source: String,
    expr: Option<Expr>,
    error: Option<String>,
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
        };
        let _ = panel.reparse();
        Ok(panel)
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

    /// Remove one character and reparse.
    pub fn backspace(&mut self) -> Option<SoundSpec> {
        self.source.pop();
        self.reparse()
    }

    /// Append ordinary text and reparse.
    pub fn push_text(&mut self, text: &str) -> Option<SoundSpec> {
        if !self.can_append(text) {
            return None;
        }
        self.source.push_str(text);
        self.reparse()
    }

    /// Append a literal space. This preserves the current parse state, matching
    /// the old event-loop behavior.
    pub fn push_space(&mut self) {
        if self.can_append(" ") {
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
        numinous_core::draw_text(raster, "THE STUDIO", 10, 10, scale, '#');
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
}

#[cfg(test)]
mod tests {
    use super::{MAX_STUDIO_SOURCE_CHARS, StudioPanel, studio_scale};
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
