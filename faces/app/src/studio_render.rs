//! Shared deterministic curve sampling and rasterization for Studio surfaces.

use numinous_core::{Raster, Surface};

struct CurveSamples {
    points: Vec<(usize, f64)>,
    ymin: f64,
    ymax: f64,
}

/// Raster dimensions and reserved chrome surrounding one curve.
#[derive(Clone, Copy)]
pub struct CurveLayout {
    /// Requested horizontal raster extent.
    pub width: usize,
    /// Requested vertical raster extent.
    pub height: usize,
    /// Rows reserved above the curve band.
    pub top: f64,
    /// Rows reserved below the curve band.
    pub bottom_margin: f64,
}

fn sample_curve(
    width: usize,
    xmin: f64,
    xmax: f64,
    mut value_at: impl FnMut(f64) -> Option<f64>,
) -> Option<CurveSamples> {
    if width < 2 || !xmin.is_finite() || !xmax.is_finite() || xmax <= xmin {
        return None;
    }
    let span = xmax - xmin;
    if !span.is_finite() {
        return None;
    }
    let points: Vec<_> = (0..width)
        .filter_map(|column| {
            let x = xmin + span * column as f64 / (width as f64 - 1.0);
            let value = value_at(x)?;
            value.is_finite().then_some((column, value))
        })
        .collect();
    if points.is_empty() {
        return None;
    }
    let ymin = points
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let ymax = points
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    Some(CurveSamples { points, ymin, ymax })
}

/// Returns the finite vertical range observed at a fixed horizontal resolution.
pub(crate) fn curve_range(
    width: usize,
    xmin: f64,
    xmax: f64,
    value_at: impl FnMut(f64) -> Option<f64>,
) -> Option<(f64, f64)> {
    let samples = sample_curve(width, xmin, xmax, value_at)?;
    Some((samples.ymin, samples.ymax))
}

/// Draws one auto-scaled deterministic curve into a bounded vertical band.
pub fn draw_curve(
    raster: &mut Raster,
    layout: CurveLayout,
    xmin: f64,
    xmax: f64,
    value_at: impl FnMut(f64) -> Option<f64>,
) -> Option<(f64, f64)> {
    let width = layout.width.min(raster.width());
    let height = layout.height.min(raster.height());
    let samples = sample_curve(width, xmin, xmax, value_at)?;
    let plot_height = height as f64 - layout.top - layout.bottom_margin;
    if !layout.top.is_finite()
        || !layout.bottom_margin.is_finite()
        || layout.top < 0.0
        || layout.bottom_margin < 0.0
        || plot_height < 8.0
    {
        return None;
    }
    let yspan = (samples.ymax - samples.ymin).max(1e-9);
    let mut previous = None;
    for (column, value) in samples.points {
        let x = column as i32;
        let y = (layout.top + (1.0 - (value - samples.ymin) / yspan) * plot_height) as i32;
        if let Some((previous_x, previous_y)) = previous {
            raster.line(previous_x, previous_y, x, y, '#');
        }
        previous = Some((x, y));
    }
    Some((samples.ymin, samples.ymax))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_sampling_rejects_invalid_geometry_and_undefined_functions() {
        assert!(curve_range(1, -1.0, 1.0, Some).is_none());
        assert!(curve_range(8, f64::NAN, 1.0, Some).is_none());
        assert!(curve_range(8, -1.0, f64::NAN, Some).is_none());
        assert!(curve_range(8, 1.0, -1.0, Some).is_none());
        assert!(curve_range(8, -f64::MAX, f64::MAX, Some).is_none());
        assert!(curve_range(8, -1.0, 1.0, |_| None).is_none());
    }

    #[test]
    fn the_window_frames_a_curve_exactly_as_the_other_faces_do() {
        // The App draws pixels and the CLI and MCP draw characters, so their
        // pictures cannot be compared byte for byte. What can, and what 0.7
        // actually asks for, is that all three agree about the curve: the same
        // samples, the same discards, and the same vertical framing.
        //
        // This crate samples and auto-scales in `sample_curve`; the core does
        // it again inside `plot_text`. Two implementations of one rule stay in
        // step only while something checks, and `scripts/creator-parity.py`
        // already holds the other two faces together.
        for (source, xmin, xmax, a) in [
            ("sin(x)", -std::f64::consts::TAU, std::f64::consts::TAU, 1.0),
            ("x*x", -2.0, 2.0, 1.0),
            (
                "sin(a*x)",
                -std::f64::consts::TAU,
                std::f64::consts::TAU,
                2.5,
            ),
            (
                "sin(a*x)",
                -std::f64::consts::TAU,
                std::f64::consts::TAU,
                -3.0,
            ),
            ("sin(x)", 0.0, 10.0, 1.0),
            // Undefined at x = 0, so both sides must discard the same point
            // rather than one of them framing around an infinity.
            //
            // The widths below matter for this case and are the reason odd ones
            // are here. A column lands on x = 0 only when (width - 1) / 2 is a
            // whole number, so at an even width the grid straddles the
            // singularity and nothing is discarded at all. This case ran at 40,
            // 72 and 200 for a long time and never once exercised the discard
            // it exists for.
            ("1/x", -std::f64::consts::TAU, std::f64::consts::TAU, 1.0),
        ] {
            let expr = numinous_core::parse(source).expect("parses");
            let mut discarded_somewhere = false;
            for width in [40usize, 41, 72, 73, 200, 201] {
                let (_, core_min, core_max) =
                    numinous_core::plot_text(source, xmin, xmax, a, width, 24)
                        .expect("core plots it");
                let samples = sample_curve(width, xmin, xmax, |x| {
                    Some(numinous_core::eval(&expr, x, a))
                })
                .expect("the window samples it");
                discarded_somewhere |= samples.points.len() < width;
                assert_eq!(
                    (samples.ymin, samples.ymax),
                    (core_min, core_max),
                    "{source} at a={a} over [{xmin}, {xmax}] at width {width}"
                );
            }
            // A case whose whole point is the discard must actually discard.
            // Without this the widths could drift back to all-even and the
            // case would go quietly inert again, passing either way.
            if source == "1/x" {
                assert!(
                    discarded_somewhere,
                    "no width put a column on the singularity, so the discard path \
                     was never taken and this case proves nothing"
                );
            }
        }
    }

    #[test]
    fn curve_sampling_and_drawing_share_the_exact_range() {
        let range = curve_range(64, -1.0, 1.0, |x| Some(x * x)).expect("finite range");
        let mut raster = Raster::new(64, 80);
        let drawn = draw_curve(
            &mut raster,
            CurveLayout {
                width: 64,
                height: 80,
                top: 12.0,
                bottom_margin: 8.0,
            },
            -1.0,
            1.0,
            |x| Some(x * x),
        )
        .expect("drawn curve");
        assert_eq!(drawn, range);
        assert!(raster.lit_count() > 0);
    }

    #[test]
    fn curve_drawing_is_safe_for_tiny_and_mismatched_surfaces() {
        let mut zero = Raster::new(0, 0);
        assert!(
            draw_curve(
                &mut zero,
                CurveLayout {
                    width: 20,
                    height: 20,
                    top: 0.0,
                    bottom_margin: 0.0,
                },
                -1.0,
                1.0,
                Some,
            )
            .is_none()
        );
        let mut short = Raster::new(100, 10);
        assert!(
            draw_curve(
                &mut short,
                CurveLayout {
                    width: 200,
                    height: 200,
                    top: 8.0,
                    bottom_margin: 8.0,
                },
                -1.0,
                1.0,
                Some,
            )
            .is_none()
        );

        let invalid_layouts = [
            CurveLayout {
                width: 32,
                height: 32,
                top: f64::NAN,
                bottom_margin: 0.0,
            },
            CurveLayout {
                width: 32,
                height: 32,
                top: 0.0,
                bottom_margin: f64::NAN,
            },
            CurveLayout {
                width: 32,
                height: 32,
                top: -1.0,
                bottom_margin: 0.0,
            },
            CurveLayout {
                width: 32,
                height: 32,
                top: 0.0,
                bottom_margin: -1.0,
            },
        ];
        for layout in invalid_layouts {
            let mut raster = Raster::new(32, 32);
            assert!(draw_curve(&mut raster, layout, -1.0, 1.0, Some).is_none());
        }
    }
}
