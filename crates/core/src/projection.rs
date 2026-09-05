//! A centered fit that preserves the geometry of a planar path.

use crate::surface::Surface;

/// A planar coordinate fit inside a bounded surface rectangle.
///
/// Both axes use one physical scale, adjusted for the surface's character
/// aspect. A circle stays round; fitting a wider ellipse does not erase its
/// axis ratio. The caller owns any margins around the supplied rectangle.
#[derive(Debug, Clone, Copy)]
pub struct PlanarProjection {
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
    origin: (f64, f64),
    normalization: f64,
    normalized_center: (f64, f64),
    screen_center: (f64, f64),
    scale: (f64, f64),
}

impl PlanarProjection {
    /// Fit finite coordinate bounds inside `(left, top, width, height)`.
    ///
    /// The rectangle is clipped to [`Surface::draw_bounds`]. The unused
    /// extent is shared equally on either side of the path. A constant axis
    /// is centered; a single point maps to the center of the rectangle.
    /// Invalid surface aspects use [`Surface::safe_char_aspect`].
    ///
    /// Returns `None` for reversed or nonfinite coordinate bounds, a clipped
    /// rectangle smaller than two cells on either axis, a varying axis whose
    /// normalized span underflows into subnormal values or zero, or a scale
    /// that underflows to zero or cannot be represented by a finite number.
    #[must_use]
    pub fn fit(
        surface: &dyn Surface,
        rect: (usize, usize, usize, usize),
        x_bounds: (f64, f64),
        y_bounds: (f64, f64),
    ) -> Option<Self> {
        if ![x_bounds.0, x_bounds.1, y_bounds.0, y_bounds.1]
            .into_iter()
            .all(f64::is_finite)
            || x_bounds.1 < x_bounds.0
            || y_bounds.1 < y_bounds.0
        {
            return None;
        }
        let (surface_width, surface_height) = surface.draw_bounds();
        let (left, top, width, height) = rect;
        let width = width.min(surface_width.saturating_sub(left));
        let height = height.min(surface_height.saturating_sub(top));
        if width < 2 || height < 2 {
            return None;
        }
        let spans = (x_bounds.1 - x_bounds.0, y_bounds.1 - y_bounds.0);
        let midpoint = |bounds: (f64, f64), span: f64| {
            if span.is_finite() {
                bounds.0 + span * 0.5
            } else {
                bounds.0 * 0.5 + bounds.1 * 0.5
            }
        };
        let origin = (midpoint(x_bounds, spans.0), midpoint(y_bounds, spans.1));
        // A common coordinate normalization keeps equal units while avoiding
        // overflow for opposite finite extremes. Centering before division
        // also preserves small paths translated far from the origin.
        let mut normalization = spans.0.max(spans.1);
        if !normalization.is_finite() {
            normalization = [x_bounds.0, x_bounds.1, y_bounds.0, y_bounds.1]
                .into_iter()
                .map(f64::abs)
                .fold(0.0_f64, f64::max);
        }
        if normalization == 0.0 {
            normalization = 1.0;
        }
        let nx = (
            (x_bounds.0 - origin.0) / normalization,
            (x_bounds.1 - origin.0) / normalization,
        );
        let ny = (
            (y_bounds.0 - origin.1) / normalization,
            (y_bounds.1 - origin.1) / normalization,
        );
        let normalized_spans = (nx.1 - nx.0, ny.1 - ny.0);
        let varying = (x_bounds.0 != x_bounds.1, y_bounds.0 != y_bounds.1);
        // A hostile aspect can magnify a tiny normalized interval into a
        // visible extent. Refuse underflow instead of treating that interval
        // as constant or amplifying a poorly resolved subnormal span.
        if (varying.0 && !normalized_spans.0.is_normal())
            || (varying.1 && !normalized_spans.1.is_normal())
        {
            return None;
        }
        let screen_spans = ((width - 1) as f64, (height - 1) as f64);
        let aspect = surface.safe_char_aspect();
        let scale = match normalized_spans {
            (0.0, 0.0) => (0.0, 0.0),
            (dx, 0.0) => (screen_spans.0 / dx, 0.0),
            (0.0, dy) => (0.0, screen_spans.1 / dy),
            (dx, dy) => {
                let scale = (screen_spans.0 / dx).min((screen_spans.1 / aspect) / dy);
                (scale, scale * aspect)
            }
        };
        if !scale.0.is_finite()
            || !scale.1.is_finite()
            || (varying.0 && scale.0 == 0.0)
            || (varying.1 && scale.1 == 0.0)
        {
            return None;
        }
        Some(Self {
            x_bounds,
            y_bounds,
            origin,
            normalization,
            normalized_center: (nx.0 * 0.5 + nx.1 * 0.5, ny.0 * 0.5 + ny.1 * 0.5),
            screen_center: (
                left as f64 + screen_spans.0 * 0.5,
                top as f64 + screen_spans.1 * 0.5,
            ),
            scale,
        })
    }

    /// Map a point within the fitted coordinate bounds to the nearest cell.
    ///
    /// World y increases upward; screen y increases downward. Returns `None`
    /// for nonfinite coordinates or a point outside the bounds supplied to
    /// [`Self::fit`]. Undefined samples can therefore remain gaps in a path.
    #[must_use]
    pub fn point(self, x: f64, y: f64) -> Option<(i32, i32)> {
        if !x.is_finite()
            || !y.is_finite()
            || x < self.x_bounds.0
            || x > self.x_bounds.1
            || y < self.y_bounds.0
            || y > self.y_bounds.1
        {
            return None;
        }
        let x = (x - self.origin.0) / self.normalization - self.normalized_center.0;
        let y = (y - self.origin.1) / self.normalization - self.normalized_center.1;
        Some((
            (self.screen_center.0 + x * self.scale.0).round() as i32,
            (self.screen_center.1 - y * self.scale.1).round() as i32,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::PlanarProjection;
    use crate::{Canvas, Raster, Surface};

    #[test]
    fn one_scale_fits_known_coordinates_inside_an_offset_rectangle() {
        let raster = Raster::new(241, 161);
        let projection =
            PlanarProjection::fit(&raster, (20, 30, 201, 101), (-2.0, 2.0), (-1.0, 1.0)).unwrap();
        assert_eq!(projection.point(-2.0, -1.0), Some((20, 130)));
        assert_eq!(projection.point(2.0, 1.0), Some((220, 30)));
        assert_eq!(projection.point(0.0, 0.0), Some((120, 80)));
        // An equal x/y displacement moves equally far on square pixels.
        assert_eq!(projection.point(1.0, 1.0), Some((170, 30)));

        let canvas = Canvas::new(241, 161);
        let projection =
            PlanarProjection::fit(&canvas, (20, 30, 201, 101), (-2.0, 2.0), (-1.0, 1.0)).unwrap();
        assert_eq!(projection.point(-2.0, -1.0), Some((20, 105)));
        assert_eq!(projection.point(2.0, 1.0), Some((220, 55)));
        assert_eq!(projection.point(1.0, 1.0), Some((170, 55)));
    }

    #[test]
    fn finite_extremes_tiny_spans_and_large_translations_remain_resolved() {
        let raster = Raster::new(101, 101);
        for bound in [f64::MAX, 1e300, 1e-300, f64::from_bits(1)] {
            let projection =
                PlanarProjection::fit(&raster, (0, 0, 101, 101), (-bound, bound), (-bound, bound))
                    .unwrap();
            assert_eq!(projection.point(-bound, -bound), Some((0, 100)));
            assert_eq!(projection.point(bound, bound), Some((100, 0)));
            assert_eq!(projection.point(0.0, 0.0), Some((50, 50)));
        }
        let projection = PlanarProjection::fit(
            &raster,
            (0, 0, 101, 101),
            (1e12 - 1.0, 1e12 + 1.0),
            (1e300, 1e300),
        )
        .unwrap();
        assert_eq!(projection.point(1e12 - 1.0, 1e300), Some((0, 50)));
        assert_eq!(projection.point(1e12 + 1.0, 1e300), Some((100, 50)));
    }

    #[test]
    fn invalid_bounds_are_rejected_and_hostile_rectangles_are_clipped() {
        let raster = Raster::new(101, 61);
        for bounds in [(1.0, -1.0), (f64::NAN, 1.0), (0.0, f64::INFINITY)] {
            assert!(PlanarProjection::fit(&raster, (0, 0, 101, 61), bounds, (0.0, 1.0)).is_none());
            assert!(PlanarProjection::fit(&raster, (0, 0, 101, 61), (0.0, 1.0), bounds).is_none());
        }
        for rect in [
            (0, 0, 0, 61),
            (0, 0, 101, 1),
            (usize::MAX, 0, usize::MAX, 61),
        ] {
            assert!(PlanarProjection::fit(&raster, rect, (0.0, 1.0), (0.0, 1.0)).is_none());
        }
        let projection = PlanarProjection::fit(
            &raster,
            (20, 10, usize::MAX, usize::MAX),
            (-1.0, 1.0),
            (-1.0, 1.0),
        )
        .unwrap();
        assert_eq!(projection.point(-1.0, -1.0), Some((35, 60)));
        assert_eq!(projection.point(1.0, 1.0), Some((85, 10)));
        for point in [(f64::NAN, 0.0), (0.0, f64::INFINITY), (2.0, 0.0)] {
            assert_eq!(projection.point(point.0, point.1), None);
        }
    }

    #[test]
    fn invalid_surface_aspects_use_the_shared_terminal_fallback() {
        struct Aspect(f64);
        impl Surface for Aspect {
            fn width(&self) -> usize {
                101
            }
            fn height(&self) -> usize {
                101
            }
            fn char_aspect(&self) -> f64 {
                self.0
            }
            fn plot(&mut self, _: i32, _: i32, _: char) {}
        }
        for aspect in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            let projection =
                PlanarProjection::fit(&Aspect(aspect), (0, 0, 101, 101), (-1.0, 1.0), (-1.0, 1.0))
                    .unwrap();
            assert_eq!(projection.point(1.0, 1.0), Some((100, 25)));
        }
        // In the first fixture, dx/(dy*aspect)=1/2: it has a visible
        // 50-by-100 footprint despite the tiny x extent. Losing that extent
        // internally must not forge a vertical line. Nearby subnormal spans
        // can retain only a few significant bits, so they are refused too.
        for bits in [1, 2, 3, 5] {
            assert!(
                PlanarProjection::fit(
                    &Aspect(f64::from_bits(1)),
                    (0, 0, 101, 101),
                    (0.0, f64::from_bits(bits)),
                    (0.0, 2.0),
                )
                .is_none()
            );
        }
        // This physical footprint is nearly square, but its intermediate
        // scale cannot be represented. The documented refusal stays explicit.
        assert!(
            PlanarProjection::fit(&Aspect(1e308), (0, 0, 101, 101), (0.0, 1.0), (0.0, 1e-308),)
                .is_none()
        );
    }
}
