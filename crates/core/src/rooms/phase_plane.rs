//! Fitted phase portraits with one scale for both physical coordinate axes.

use crate::surface::Surface;

#[derive(Debug, Clone, Copy)]
pub(super) struct PhasePlane {
    center: (f64, f64),
    origin: (f64, f64),
    scale: (f64, f64),
}

impl PhasePlane {
    /// Fit all supplied coordinates with symmetric margins. Multiple paths
    /// must be supplied together when their extents are to be compared.
    pub(super) fn fit(
        surface: &dyn Surface,
        mut points: impl Iterator<Item = (f64, f64)>,
    ) -> Option<Self> {
        let (width, height) = surface.draw_bounds();
        if width == 0 || height == 0 {
            return None;
        }
        let (x, y) = points.next()?;
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (x, x, y, y);
        for (x, y) in points {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let dx = (max_x - min_x).max(1e-6);
        let dy = (max_y - min_y).max(1e-6);
        if !dx.is_finite() || !dy.is_finite() {
            return None;
        }
        let aspect = surface.safe_char_aspect();
        let screen_x = width.saturating_sub(1) as f64;
        let screen_y = height.saturating_sub(1) as f64;
        // Reserve eight percent horizontally and twelve and a half percent
        // vertically so App title/footer chrome cannot cover turning points.
        // Taking the smaller budget retains one scale for both coordinates;
        // terminal cells then need their height-to-width correction.
        let scale = (0.84 * screen_x / dx).min(0.75 * screen_y / (dy * aspect));
        Some(Self {
            center: (screen_x * 0.5, screen_y * 0.5),
            origin: (min_x * 0.5 + max_x * 0.5, min_y * 0.5 + max_y * 0.5),
            scale: (scale, scale * aspect),
        })
    }

    pub(super) fn point(self, x: f64, y: f64) -> (i32, i32) {
        (
            (self.center.0 + (x - self.origin.0) * self.scale.0).round() as i32,
            (self.center.1 - (y - self.origin.1) * self.scale.1).round() as i32,
        )
    }

    /// Invert continuous screen coordinates, including fractional pixel edges.
    /// One-row or one-column surfaces have no invertible two-dimensional fit.
    pub(super) fn world(self, x: f64, y: f64) -> Option<(f64, f64)> {
        if !x.is_finite()
            || !y.is_finite()
            || !self.scale.0.is_finite()
            || !self.scale.1.is_finite()
            || self.scale.0 <= 0.0
            || self.scale.1 <= 0.0
        {
            return None;
        }
        let x = self.origin.0 + (x - self.center.0) / self.scale.0;
        let y = self.origin.1 - (y - self.center.1) / self.scale.1;
        (x.is_finite() && y.is_finite()).then_some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canvas::Canvas, raster::Raster};

    #[test]
    fn inverse_recovers_cell_edges_with_equal_physical_scale() {
        let raster_wide = Raster::new(1001, 281);
        let raster_square = Raster::new(1001, 1001);
        let canvas_square = Canvas::new(101, 101);
        // Independent projections of a 48 by 28 unit lattice: vertical fit,
        // horizontal fit, and horizontal fit in half-width terminal cells.
        for (surface, left, top, right, bottom) in [
            (&raster_wide as &dyn Surface, 320.0, 35.0, 680.0, 245.0),
            (&raster_square as &dyn Surface, 80.0, 255.0, 920.0, 745.0),
            (&canvas_square as &dyn Surface, 8.0, 37.75, 92.0, 62.25),
        ] {
            let plane = PhasePlane::fit(surface, [(0.0, 0.0), (48.0, 28.0)].into_iter())
                .expect("finite lattice");
            for (pixel, expected) in [
                ((left, bottom), (0.0, 0.0)),
                ((right, top), (48.0, 28.0)),
                (((left + right) * 0.5, (top + bottom) * 0.5), (24.0, 14.0)),
                (
                    (left * 0.75 + right * 0.25, top * 0.25 + bottom * 0.75),
                    (12.0, 7.0),
                ),
            ] {
                let actual = plane.world(pixel.0, pixel.1).expect("invertible fit");
                assert!((actual.0 - expected.0).abs() < 1e-12);
                assert!((actual.1 - expected.1).abs() < 1e-12);
                assert_eq!(
                    plane.point(actual.0, actual.1),
                    (pixel.0.round() as i32, pixel.1.round() as i32)
                );
            }
            let (px, py) = ((left + right) * 0.5, (top + bottom) * 0.5);
            let upper_left = plane.world(px - 0.5, py - 0.5).expect("pixel edge");
            let lower_right = plane.world(px + 0.5, py + 0.5).expect("pixel edge");
            let world_width = lower_right.0 - upper_left.0;
            let world_height = upper_left.1 - lower_right.1;
            assert!(world_width > 0.0 && world_height > 0.0);
            assert!((world_width - world_height * surface.safe_char_aspect()).abs() < 1e-12);
        }
    }

    #[test]
    fn inverse_rejects_degenerate_scale_and_nonfinite_coordinates() {
        for surface in [Raster::new(1, 100), Raster::new(100, 1), Raster::new(1, 1)] {
            let plane = PhasePlane::fit(&surface, [(0.0, 0.0), (1.0, 1.0)].into_iter())
                .expect("nonempty surface");
            assert!(plane.world(0.0, 0.0).is_none());
        }
        let surface = Raster::new(101, 101);
        let plane =
            PhasePlane::fit(&surface, [(0.0, 0.0), (1.0, 1.0)].into_iter()).expect("finite square");
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(plane.world(invalid, 0.0).is_none());
            assert!(plane.world(0.0, invalid).is_none());
        }
        let large = PhasePlane::fit(&surface, [(0.0, 0.0), (1e300, 1e300)].into_iter())
            .expect("finite large square");
        assert!(large.world(f64::MAX, f64::MAX).is_none());
    }
}
