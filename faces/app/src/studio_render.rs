//! Shared deterministic curve sampling and rasterization for Studio surfaces.

use numinous_core::{Raster, Surface};

struct CurveSamples {
    points: Vec<(usize, f64)>,
    ymin: f64,
    ymax: f64,
}

struct ParametricSamples {
    points: Vec<Option<(f64, f64)>>,
    xmin: f64,
    xmax: f64,
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

/// One bounded raster rectangle for a curve preview.
#[derive(Clone, Copy)]
pub struct CurveRect {
    /// Left pixel column.
    pub left: usize,
    /// Top pixel row.
    pub top: usize,
    /// Requested width in pixels.
    pub width: usize,
    /// Requested height in pixels.
    pub height: usize,
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

/// Draw one auto-scaled parametric path into the same bounded vertical band.
/// Sampling is denser than the pixel width so closed curves do not become a
/// sparse polygon at small windows, but stays capped independently of input.
pub fn draw_parametric(
    raster: &mut Raster,
    layout: CurveLayout,
    tmin: f64,
    tmax: f64,
    point_at: impl FnMut(f64) -> Option<(f64, f64)>,
) -> Option<(f64, f64, f64, f64)> {
    let width = layout.width.min(raster.width());
    let height = layout.height.min(raster.height());
    let plot_height = height as f64 - layout.top - layout.bottom_margin;
    if !layout.top.is_finite()
        || !layout.bottom_margin.is_finite()
        || layout.top < 0.0
        || layout.bottom_margin < 0.0
        || plot_height < 8.0
    {
        return None;
    }
    draw_parametric_rect(
        raster,
        CurveRect {
            left: 0,
            top: layout.top.round() as usize,
            width,
            height: plot_height.round() as usize,
        },
        tmin,
        tmax,
        point_at,
    )
}

fn sample_parametric(
    width: usize,
    tmin: f64,
    tmax: f64,
    mut point_at: impl FnMut(f64) -> Option<(f64, f64)>,
) -> Option<ParametricSamples> {
    if width < 2 || !tmin.is_finite() || !tmax.is_finite() || tmax <= tmin {
        return None;
    }
    let sample_count = width.saturating_mul(4).clamp(64, 16_384);
    let span = tmax - tmin;
    if !span.is_finite() {
        return None;
    }
    let points: Vec<Option<(f64, f64)>> = (0..sample_count)
        .map(|index| {
            let t = tmin + span * index as f64 / (sample_count - 1) as f64;
            point_at(t).filter(|(x, y)| x.is_finite() && y.is_finite())
        })
        .collect();
    let finite: Vec<(f64, f64)> = points.iter().flatten().copied().collect();
    if finite.is_empty() {
        return None;
    }
    let xmin = finite
        .iter()
        .map(|point| point.0)
        .fold(f64::INFINITY, f64::min);
    let xmax = finite
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let ymin = finite
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let ymax = finite
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    Some(ParametricSamples {
        points,
        xmin,
        xmax,
        ymin,
        ymax,
    })
}

/// Draw one auto-scaled parametric path inside an explicit raster rectangle.
/// This is the gallery form of [`draw_parametric`], with the same sampling
/// and finite-value behavior but no dependence on full-surface chrome.
pub fn draw_parametric_rect(
    raster: &mut Raster,
    rect: CurveRect,
    tmin: f64,
    tmax: f64,
    point_at: impl FnMut(f64) -> Option<(f64, f64)>,
) -> Option<(f64, f64, f64, f64)> {
    let width = rect.width.min(raster.width().saturating_sub(rect.left));
    let height = rect.height.min(raster.height().saturating_sub(rect.top));
    if width < 2 || height < 2 {
        return None;
    }
    let samples = sample_parametric(width, tmin, tmax, point_at)?;
    let ParametricSamples {
        points,
        xmin,
        xmax,
        ymin,
        ymax,
    } = samples;
    let xspan = (xmax - xmin).max(1e-9);
    let yspan = (ymax - ymin).max(1e-9);
    let mut previous = None;
    for point in points {
        let Some((x, y)) = point else {
            previous = None;
            continue;
        };
        let px = rect.left as i32 + ((x - xmin) / xspan * (width as f64 - 1.0)).round() as i32;
        let py =
            rect.top as i32 + ((1.0 - (y - ymin) / yspan) * (height as f64 - 1.0)).round() as i32;
        if let Some((previous_x, previous_y)) = previous {
            raster.line(previous_x, previous_y, px, py, '#');
        }
        previous = Some((px, py));
    }
    Some((xmin, xmax, ymin, ymax))
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

    /// The columns the core's character plot puts a mark in.
    ///
    /// Lines are trimmed of trailing space, so a column is marked when some row
    /// is long enough to reach it and holds a non-space there.
    fn marked_columns(text: &str) -> Vec<usize> {
        let mut columns: Vec<usize> = Vec::new();
        for line in text.lines() {
            for (column, glyph) in line.chars().enumerate() {
                if glyph != ' ' && !columns.contains(&column) {
                    columns.push(column);
                }
            }
        }
        columns.sort_unstable();
        columns
    }

    /// The columns the App's raster ends up with ink in, for the same curve.
    ///
    /// Drawn rather than sampled, because the core's plot joins its samples
    /// with a line and so marks the columns between two distant points too.
    /// Comparing the App's raw sample columns against that would report a
    /// difference that is only the two faces filling a gap the same way.
    fn drawn_columns(
        width: usize,
        xmin: f64,
        xmax: f64,
        value_at: impl FnMut(f64) -> Option<f64>,
    ) -> Vec<usize> {
        let mut raster = Raster::new(width, 24);
        let layout = CurveLayout {
            width,
            height: 24,
            top: 0.0,
            // One row of margin, so the lowest sample maps to the last row
            // rather than one past it. Without this the bottom-most point is
            // clipped away and a column whose only pixel is that point reads
            // as unmarked, which looks exactly like the two faces disagreeing.
            bottom_margin: 1.0,
        };
        draw_curve(&mut raster, layout, xmin, xmax, value_at).expect("the window draws it");
        let rgba = raster.to_rgba();
        let mut columns = Vec::new();
        for column in 0..width {
            let lit = (0..24).any(|row| {
                let at = (row * width + column) * 4;
                rgba[at..at + 3] != [10, 11, 15]
            });
            if lit {
                columns.push(column);
            }
        }
        columns
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
                let (core_text, core_min, core_max) =
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

                // The framing alone is too weak to say the two agree about
                // samples. Dropping the App's last column changes neither the
                // minimum nor the maximum for any of these functions, so an
                // off-by-one in the sample grid passed this test until the
                // columns themselves were compared. Both faces put a mark in
                // the same columns, so compare which columns those are.
                let core_columns = marked_columns(&core_text);
                let window_columns = drawn_columns(width, xmin, xmax, |x| {
                    Some(numinous_core::eval(&expr, x, a))
                });
                assert_eq!(
                    window_columns, core_columns,
                    "{source} at a={a} over [{xmin}, {xmax}] at width {width}: the two \
                     faces draw the curve across different columns"
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

    #[test]
    fn parametric_drawing_closes_a_circle_and_reports_both_axes() {
        let mut raster = Raster::new(80, 80);
        let bounds = draw_parametric(
            &mut raster,
            CurveLayout {
                width: 80,
                height: 80,
                top: 8.0,
                bottom_margin: 8.0,
            },
            0.0,
            std::f64::consts::TAU,
            |t| Some((t.cos(), t.sin())),
        )
        .expect("circle");
        assert!((bounds.0 + 1.0).abs() < 0.01);
        assert!((bounds.1 - 1.0).abs() < 0.01);
        assert!((bounds.2 + 1.0).abs() < 0.01);
        assert!((bounds.3 - 1.0).abs() < 0.01);
        assert!(raster.lit_count() > 100);
    }
}
