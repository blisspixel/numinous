//! App-local Studio input, parsing, audio, and drawing helpers.

use std::f64::consts::TAU;

use numinous_core::{Expr, Raster, SoundSpec, Surface};

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
        Self::new(DEFAULT_SOURCE)
    }
}

impl StudioPanel {
    /// Build a Studio panel from source text.
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut panel = Self {
            source: source.to_string(),
            expr: None,
            error: None,
        };
        let _ = panel.reparse();
        panel
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
        self.source.push_str(text);
        self.reparse()
    }

    /// Append a literal space. This preserves the current parse state, matching
    /// the old event-loop behavior.
    pub fn push_space(&mut self) {
        self.source.push(' ');
    }

    /// Draw the Studio panel into the raster.
    pub fn draw(&self, raster: &mut Raster, width: usize, height: usize, t: f64) {
        let width = width.min(raster.width());
        let height = height.min(raster.height());
        let scale = (width as i32 / 500).clamp(1, 3);
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
            let footer = "TYPE A FORMULA   TAB CLOSES   ESC MENU";
            raster.clear_rows(height as i32 - 16 * scale, height as i32);
            numinous_core::draw_text(raster, footer, 10, height as i32 - 11 * scale, scale, '#');
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
    use super::StudioPanel;
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
        let mut panel = StudioPanel::new("x");
        assert!(panel.push_text("@").is_none());
        assert!(panel.error.is_some());
        assert!(panel.expr.is_some());
        let mut raster = Raster::new(120, 90);
        panel.draw(&mut raster, 120, 90, 0.25);
        assert!(raster.lit_count() > 0, "last good curve should still draw");
    }

    #[test]
    fn draw_handles_tiny_and_mismatched_sizes() {
        let panel = StudioPanel::new("sin(x)");
        let mut zero = Raster::new(0, 0);
        panel.draw(&mut zero, 0, 0, 0.0);
        assert_eq!(zero.lit_count(), 0);

        let mut one = Raster::new(1, 1);
        panel.draw(&mut one, 1, 1, 0.0);

        let mut short = Raster::new(80, 20);
        panel.draw(&mut short, 500, 20, 0.0);
        assert!(short.lit_count() > 0);

        let mut mismatched = Raster::new(24, 90);
        panel.draw(&mut mismatched, 200, 90, 0.5);
        assert!(mismatched.lit_count() > 0);
    }

    #[test]
    fn editing_operations_update_source_predictably() {
        let mut panel = StudioPanel::new("x");
        panel.push_space();
        assert_eq!(panel.source, "x ");
        assert!(panel.push_text("+ 1").is_some());
        assert_eq!(panel.source, "x + 1");
        assert!(panel.backspace().is_none());
        assert_eq!(panel.source, "x + ");
        assert!(panel.error.is_some());
    }
}
