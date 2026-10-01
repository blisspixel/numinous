//! Shared deterministic curve sampling and rasterization for Studio surfaces.

use numinous_core::{
    Expr, FieldReading, PATTERN_HIT, PlanarProjection, Raster, StudioSlider, Surface,
    draw_field_named as draw_field_plate, field_mark_level,
};

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

fn band_plot_height(raster: &Raster, layout: CurveLayout) -> Option<f64> {
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
    Some(plot_height)
}

fn paint_samples(
    raster: &mut Raster,
    top: f64,
    plot_height: f64,
    points: &[(usize, f64)],
    ymin: f64,
    ymax: f64,
    mark: char,
) {
    let yspan = (ymax - ymin).max(1e-9);
    let mut previous = None;
    for (column, value) in points {
        let x = *column as i32;
        let y = (top + (1.0 - (value - ymin) / yspan) * plot_height) as i32;
        if let Some((previous_x, previous_y)) = previous
            && previous_x + 1 == x
        {
            raster.line(previous_x, previous_y, x, y, mark);
        } else {
            raster.plot(x, y, mark);
        }
        previous = Some((x, y));
    }
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
    let samples = sample_curve(width, xmin, xmax, value_at)?;
    let plot_height = band_plot_height(raster, layout)?;
    paint_samples(
        raster,
        layout.top,
        plot_height,
        &samples.points,
        samples.ymin,
        samples.ymax,
        '#',
    );
    Some((samples.ymin, samples.ymax))
}

/// Draw two curves on one shared vertical axis.
///
/// Separate auto-scale would give both curves the same height when their
/// ranges differ. The first mark is `#`. The second mark is `+`. When the
/// second curve has no finite samples, the first is drawn alone on its own
/// range. Callers use this for a slope, and for the first term of a
/// two-, three-, four-, five-, six-, seven-, eight-, or nine-oscillator graph.
pub fn draw_two_curves(
    raster: &mut Raster,
    layout: CurveLayout,
    xmin: f64,
    xmax: f64,
    graph_at: impl FnMut(f64) -> Option<f64>,
    slope_at: impl FnMut(f64) -> Option<f64>,
) -> Option<(f64, f64)> {
    let width = layout.width.min(raster.width());
    let graph = sample_curve(width, xmin, xmax, graph_at);
    let slope = sample_curve(width, xmin, xmax, slope_at);
    let (ymin, ymax, paint_slope) = match (&graph, &slope) {
        (Some(graph_samples), Some(slope_samples)) => (
            graph_samples.ymin.min(slope_samples.ymin),
            graph_samples.ymax.max(slope_samples.ymax),
            true,
        ),
        (Some(graph_samples), None) => (graph_samples.ymin, graph_samples.ymax, false),
        _ => return None,
    };
    let plot_height = band_plot_height(raster, layout)?;
    if let Some(graph_samples) = &graph {
        paint_samples(
            raster,
            layout.top,
            plot_height,
            &graph_samples.points,
            ymin,
            ymax,
            '#',
        );
    }
    if paint_slope && let Some(slope_samples) = &slope {
        paint_samples(
            raster,
            layout.top,
            plot_height,
            &slope_samples.points,
            ymin,
            ymax,
            '+',
        );
    }
    Some((ymin, ymax))
}

/// Draw several graphs over one shared vertical range.
pub fn draw_overlay(
    raster: &mut Raster,
    layout: CurveLayout,
    xmin: f64,
    xmax: f64,
    curves: &mut [impl FnMut(f64) -> Option<f64>],
) -> Option<(f64, f64)> {
    let width = layout.width.min(raster.width());
    let sampled: Vec<(usize, CurveSamples)> = curves
        .iter_mut()
        .enumerate()
        .filter_map(|(index, value_at)| {
            sample_curve(width, xmin, xmax, value_at).map(|samples| (index, samples))
        })
        .collect();
    if sampled.is_empty() {
        return None;
    }
    let ymin = sampled
        .iter()
        .map(|(_, curve)| curve.ymin)
        .fold(f64::INFINITY, f64::min);
    let ymax = sampled
        .iter()
        .map(|(_, curve)| curve.ymax)
        .fold(f64::NEG_INFINITY, f64::max);
    let plot_height = band_plot_height(raster, layout)?;
    for (index, samples) in &sampled {
        let mark =
            numinous_core::PROGRAM_MARKS[(*index).min(numinous_core::PROGRAM_MARKS.len() - 1)];
        paint_samples(
            raster,
            layout.top,
            plot_height,
            &samples.points,
            ymin,
            ymax,
            mark,
        );
    }
    Some((ymin, ymax))
}

/// Draw integer 0/1 graphs as a step grid, one row per overlay curve.
///
/// This is the Pattern Studio grid reading of the same formula that already
/// draws and sings. Empty rows, mixed widths, or a band too small to mark
/// a cell return false and leave the raster unchanged.
pub fn draw_pattern_grid(raster: &mut Raster, layout: CurveLayout, rows: &[String]) -> bool {
    let width = layout.width.min(raster.width());
    let height = layout.height.min(raster.height());
    let plot_height = height as f64 - layout.top - layout.bottom_margin;
    if rows.is_empty()
        || !layout.top.is_finite()
        || !layout.bottom_margin.is_finite()
        || layout.top < 0.0
        || layout.bottom_margin < 0.0
        || plot_height < 8.0
        || width < 8
    {
        return false;
    }
    let steps = rows[0].chars().count();
    if steps == 0 || rows.iter().any(|row| row.chars().count() != steps) {
        return false;
    }
    let layers = rows.len();
    let cell_w = ((width as f64) / steps as f64).floor().max(1.0) as i32;
    let cell_h = (plot_height / layers as f64).floor().max(1.0) as i32;
    if cell_w < 1 || cell_h < 1 {
        return false;
    }
    let inset = (cell_w.min(cell_h) / 8).max(1);
    let inner_w = (cell_w - 2 * inset).max(1);
    let inner_h = (cell_h - 2 * inset).max(1);
    let top = layout.top.round().max(0.0) as i32;
    const LAYER_LEVEL: [f32; 4] = [0.96, 0.70, 0.56, 0.42];
    for (layer, row) in rows.iter().enumerate() {
        let level = LAYER_LEVEL[layer.min(LAYER_LEVEL.len() - 1)];
        for (step, mark) in row.chars().enumerate() {
            if mark != PATTERN_HIT {
                continue;
            }
            raster.shade_rect(
                (step as i32) * cell_w + inset,
                top + (layer as i32) * cell_h + inset,
                inner_w,
                inner_h,
                level,
            );
        }
    }
    true
}

/// Fit one parametric path with equal physical coordinate units into the band.
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

/// Fit one parametric path inside a rectangle without changing its proportions.
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
    let projection = PlanarProjection::fit(
        raster,
        (rect.left, rect.top, width, height),
        (xmin, xmax),
        (ymin, ymax),
    )?;
    paint_parametric(raster, projection, &points, '#');
    Some((xmin, xmax, ymin, ymax))
}

fn paint_parametric(
    raster: &mut Raster,
    projection: PlanarProjection,
    points: &[Option<(f64, f64)>],
    mark: char,
) {
    let mut previous = None;
    for point in points {
        let Some((px, py)) = point.and_then(|(x, y)| projection.point(x, y)) else {
            previous = None;
            continue;
        };
        if let Some((previous_x, previous_y)) = previous {
            raster.line(previous_x, previous_y, px, py, mark);
        } else {
            raster.plot(px, py, mark);
        }
        previous = Some((px, py));
    }
}

/// Draw a parametric path and its first partial sum on one shared frame.
///
/// Separate auto-scale would stretch the first term until it filled the
/// same box as the whole path. The path mark is `#`. The first term is `+`.
/// When the first term has no finite samples, the path is drawn alone.
pub fn draw_parametric_pair(
    raster: &mut Raster,
    layout: CurveLayout,
    tmin: f64,
    tmax: f64,
    full_at: impl FnMut(f64) -> Option<(f64, f64)>,
    partial_at: impl FnMut(f64) -> Option<(f64, f64)>,
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
    let rect = CurveRect {
        left: 0,
        top: layout.top.round() as usize,
        width,
        height: plot_height.round() as usize,
    };
    let width = rect.width.min(raster.width().saturating_sub(rect.left));
    let height = rect.height.min(raster.height().saturating_sub(rect.top));
    if width < 2 || height < 2 {
        return None;
    }
    let full = sample_parametric(width, tmin, tmax, full_at)?;
    let partial = sample_parametric(width, tmin, tmax, partial_at);
    let (xmin, xmax, ymin, ymax) = if let Some(partial) = &partial {
        (
            full.xmin.min(partial.xmin),
            full.xmax.max(partial.xmax),
            full.ymin.min(partial.ymin),
            full.ymax.max(partial.ymax),
        )
    } else {
        (full.xmin, full.xmax, full.ymin, full.ymax)
    };
    let projection = PlanarProjection::fit(
        raster,
        (rect.left, rect.top, width, height),
        (xmin, xmax),
        (ymin, ymax),
    )?;
    paint_parametric(raster, projection, &full.points, '#');
    if let Some(partial) = &partial {
        paint_parametric(raster, projection, &partial.points, '+');
    }
    Some((xmin, xmax, ymin, ymax))
}

/// Blit a field plate into a raster band as a luminance grid.
///
/// Each character of the plate becomes a block of pixels whose brightness
/// follows the ramp. The plate is the picture: this is not a second colourful
/// encoding. Square cells (`char_aspect` 1) keep a circle round on pixels.
#[expect(
    clippy::too_many_arguments,
    reason = "a field blit needs the plate, the band, and the requested window"
)]
pub fn draw_field(
    raster: &mut Raster,
    layout: CurveLayout,
    left: i32,
    expression: &Expr,
    reading: FieldReading,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    a: f64,
    sliders: &[StudioSlider],
) -> bool {
    let width = layout.width.min(raster.width());
    let height = layout.height.min(raster.height());
    let plot_height = height as f64 - layout.top - layout.bottom_margin;
    if !layout.top.is_finite()
        || !layout.bottom_margin.is_finite()
        || layout.top < 0.0
        || layout.bottom_margin < 0.0
        || plot_height < 8.0
        || width < 8
    {
        return false;
    }
    let top = layout.top.round().max(0.0) as i32;
    let band = plot_height.round().max(8.0) as usize;
    let cols = width.clamp(8, 160);
    let rows = (band / 4).clamp(8, 90);
    let Ok(plate) = draw_field_plate(
        expression,
        reading,
        (xmin, xmax),
        (ymin, ymax),
        a,
        sliders,
        (cols, rows),
        1.0,
    ) else {
        return false;
    };
    let cell_w = (width / cols).max(1) as i32;
    let cell_h = (band / rows).max(1) as i32;
    for (row, line) in plate.text.lines().enumerate() {
        for (col, mark) in line.chars().enumerate() {
            let Some(level) = field_mark_level(mark) else {
                continue;
            };
            raster.shade_rect(
                left + (col as i32) * cell_w,
                top + (row as i32) * cell_h,
                cell_w,
                cell_h,
                level,
            );
        }
    }
    true
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
    fn graph_views_leave_undefined_columns_blank_on_the_shared_axis() {
        let layout = CurveLayout {
            width: 9,
            height: 20,
            top: 0.0,
            bottom_margin: 1.0,
        };
        let islands = |x: f64| {
            let value = (x * x - 0.25).sqrt();
            value.is_finite().then_some(value)
        };
        for view in 0..3 {
            let mut raster = Raster::new(9, 20);
            let bounds = match view {
                0 => draw_curve(&mut raster, layout, -1.0, 1.0, islands),
                1 => draw_two_curves(&mut raster, layout, -1.0, 1.0, islands, |x| {
                    islands(x).map(|y| -y)
                }),
                _ => {
                    let mut curves = [islands, islands];
                    draw_overlay(&mut raster, layout, -1.0, 1.0, &mut curves)
                }
            };
            assert!(bounds.is_some(), "view {view} must draw its finite islands");
            let ink = lit_points(&raster);
            for column in [0, 1, 2, 6, 7, 8] {
                assert!(
                    ink.iter().any(|&(x, _)| x == column),
                    "view {view}, {column}"
                );
            }
            assert!(
                ink.iter().all(|&(x, _)| !(3..=5).contains(&x)),
                "view {view} joined across undefined samples: {ink:?}"
            );
        }
    }

    #[test]
    fn graph_finite_isolated_samples_are_drawn_without_a_bridge() {
        let mut raster = Raster::new(9, 20);
        let bounds = draw_curve(
            &mut raster,
            CurveLayout {
                width: 9,
                height: 20,
                top: 0.0,
                bottom_margin: 1.0,
            },
            -1.0,
            1.0,
            |x| match x {
                -1.0 => Some(0.0),
                1.0 => Some(1.0),
                _ => None,
            },
        );
        assert_eq!(bounds, Some((0.0, 1.0)));
        assert_eq!(lit_points(&raster), [(8, 0), (0, 19)]);
    }

    #[test]
    fn undefined_overlay_voices_keep_each_defined_curves_original_mark() {
        for index in 1..numinous_core::PROGRAM_MARKS.len() {
            let mut curves: [fn(f64) -> Option<f64>; 4] = [|_| None; 4];
            // Isolated samples test palette identity without line endpoints
            // adding a second dose of ink to the same raster pixel.
            curves[index] = |x| {
                if x == -0.5 {
                    Some(0.0)
                } else if x == 0.5 {
                    Some(1.0)
                } else {
                    None
                }
            };
            let mut raster = Raster::new(9, 20);
            assert_eq!(
                draw_overlay(
                    &mut raster,
                    CurveLayout {
                        width: 9,
                        height: 20,
                        top: 0.0,
                        bottom_margin: 1.0,
                    },
                    -1.0,
                    1.0,
                    &mut curves,
                ),
                Some((0.0, 1.0))
            );
            let mut expected = Raster::new(9, 20);
            expected.plot(2, 19, numinous_core::PROGRAM_MARKS[index]);
            expected.plot(6, 0, numinous_core::PROGRAM_MARKS[index]);
            assert_eq!(raster.to_rgba(), expected.to_rgba(), "voice {index}");
        }
        let creation =
            numinous_core::StudioCreation::new_program(["sqrt(-1)", "0"], -1.0, 1.0, 1.0)
                .expect("one defined voice");
        let text = creation.plot_text(9, 20).expect("text view").text;
        assert_eq!(text.trim(), "*********");
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
    /// Inspect the rendered picture rather than the sampler, so a line that
    /// incorrectly fills an undefined column cannot satisfy the parity check.
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
                let columns =
                    drawn_columns(41, xmin, xmax, |x| Some(numinous_core::eval(&expr, x, a)));
                assert!(
                    !columns.contains(&20),
                    "the sampled singularity must stay blank in the picture"
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
    fn a_step_grid_marks_hits_and_leaves_rests() {
        let mut raster = Raster::new(80, 40);
        let layout = CurveLayout {
            width: 80,
            height: 40,
            top: 0.0,
            bottom_margin: 1.0,
        };
        assert!(draw_pattern_grid(&mut raster, layout, &["x..x..x.".into()]));
        let lit = raster.lit_count();
        assert!(lit > 10, "tresillo has three hits: {lit}");
        let mut empty = Raster::new(80, 40);
        assert!(draw_pattern_grid(&mut empty, layout, &["........".into()]));
        assert_eq!(empty.lit_count(), 0);
        let mut mixed = Raster::new(80, 40);
        assert!(!draw_pattern_grid(
            &mut mixed,
            layout,
            &["x.".into(), "x".into()]
        ));
        assert_eq!(mixed.lit_count(), 0);
        let mut tiny = Raster::new(80, 40);
        assert!(!draw_pattern_grid(
            &mut tiny,
            CurveLayout {
                width: 80,
                height: 40,
                top: 0.0,
                bottom_margin: 39.0,
            },
            &["x..x..x.".into()]
        ));
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

    fn lit_points(raster: &Raster) -> Vec<(usize, usize)> {
        let blank = Raster::new(raster.width(), raster.height()).to_rgba();
        raster
            .to_rgba()
            .chunks_exact(4)
            .zip(blank.chunks_exact(4))
            .enumerate()
            .filter_map(|(index, (actual, empty))| {
                (actual != empty).then_some((index % raster.width(), index / raster.width()))
            })
            .collect()
    }

    #[test]
    fn parametric_pixels_preserve_circle_and_ellipse_geometry_inside_each_viewport() {
        for (width, height, rect) in [
            (
                80,
                80,
                CurveRect {
                    left: 0,
                    top: 8,
                    width: 80,
                    height: 64,
                },
            ),
            (
                360,
                240,
                CurveRect {
                    left: 13,
                    top: 66,
                    width: 332,
                    height: 150,
                },
            ),
            (
                240,
                360,
                CurveRect {
                    left: 13,
                    top: 66,
                    width: 214,
                    height: 270,
                },
            ),
            (
                900,
                900,
                CurveRect {
                    left: 0,
                    top: 120,
                    width: 900,
                    height: 732,
                },
            ),
            (
                120,
                80,
                CurveRect {
                    left: 100,
                    top: 60,
                    width: 100,
                    height: 100,
                },
            ),
        ] {
            let right = (rect.left + rect.width).min(width) - 1;
            let bottom = (rect.top + rect.height).min(height) - 1;
            for ratio in [1.0, 4.0] {
                for (offset_x, offset_y) in [(0.0, 0.0), (25.0, -17.0)] {
                    let mut raster = Raster::new(width, height);
                    let bounds =
                        draw_parametric_rect(&mut raster, rect, 0.0, std::f64::consts::TAU, |t| {
                            Some((offset_x + ratio * t.cos(), offset_y + t.sin()))
                        })
                        .expect("finite ellipse");
                    assert!((bounds.0 - (offset_x - ratio)).abs() < 0.01);
                    assert!((bounds.1 - (offset_x + ratio)).abs() < 0.01);
                    assert!((bounds.2 - (offset_y - 1.0)).abs() < 0.01);
                    assert!((bounds.3 - (offset_y + 1.0)).abs() < 0.01);
                    let pixels = lit_points(&raster);
                    assert!(pixels.len() > 10);
                    let min_x = pixels.iter().map(|p| p.0).min().expect("ink") as f64;
                    let max_x = pixels.iter().map(|p| p.0).max().expect("ink") as f64;
                    let min_y = pixels.iter().map(|p| p.1).min().expect("ink") as f64;
                    let max_y = pixels.iter().map(|p| p.1).max().expect("ink") as f64;
                    let dx = max_x - min_x;
                    let dy = max_y - min_y;
                    assert!(
                        (dx - ratio * dy).abs() <= ratio + 1.0,
                        "{width}x{height}, ratio {ratio}: diameters {dx}x{dy}"
                    );
                    assert!((min_x + max_x - (rect.left + right) as f64).abs() <= 1.0);
                    assert!((min_y + max_y - (rect.top + bottom) as f64).abs() <= 1.0);
                    let center = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
                    let radius = dy / 2.0;
                    for (x, y) in pixels {
                        assert!((rect.left..=right).contains(&x));
                        assert!((rect.top..=bottom).contains(&y));
                        // Undo only the requested ellipse ratio. The observed
                        // path must lie within rasterization error of a circle.
                        let distance = ((x as f64 - center.0) / ratio).hypot(y as f64 - center.1);
                        assert!((distance - radius).abs() <= 1.5);
                    }
                }
            }
        }
    }

    #[test]
    fn parametric_finite_islands_stay_visible_without_bridging_undefined_samples() {
        let rect = CurveRect {
            left: 4,
            top: 7,
            width: 65,
            height: 41,
        };
        let mut raster = Raster::new(80, 60);
        let mut index = 0;
        let bounds = draw_parametric_rect(&mut raster, rect, 0.0, 1.0, |_| {
            let point = match index {
                0 => Some((-1.0, 0.0)),
                130 => Some((1.0, 0.0)),
                _ => None,
            };
            index += 1;
            point
        })
        .expect("two isolated finite samples");
        assert_eq!(bounds, (-1.0, 1.0, 0.0, 0.0));
        assert_eq!(lit_points(&raster), [(4, 27), (68, 27)]);

        let mut point = Raster::new(80, 60);
        draw_parametric_rect(&mut point, rect, 0.0, 1.0, |_| Some((3.0, -9.0)))
            .expect("constant point");
        assert_eq!(lit_points(&point), [(36, 27)]);
    }

    #[test]
    fn parametric_rectangles_reject_empty_and_outside_destinations() {
        for rect in [
            CurveRect {
                left: 0,
                top: 0,
                width: 0,
                height: 20,
            },
            CurveRect {
                left: 0,
                top: 0,
                width: 20,
                height: 0,
            },
            CurveRect {
                left: usize::MAX,
                top: 0,
                width: 20,
                height: 20,
            },
            CurveRect {
                left: 0,
                top: usize::MAX,
                width: 20,
                height: 20,
            },
        ] {
            let mut raster = Raster::new(40, 40);
            assert!(
                draw_parametric_rect(&mut raster, rect, 0.0, 1.0, |_| Some((1.0, 1.0))).is_none()
            );
            assert_eq!(raster.lit_count(), 0);
        }
    }

    #[test]
    fn two_curves_share_one_vertical_axis() {
        let graph = numinous_core::parse("sin(a*x)").expect("graph");
        let slope = numinous_core::parse("a*cos(a*x)").expect("slope");
        let layout = CurveLayout {
            width: 80,
            height: 40,
            top: 0.0,
            bottom_margin: 1.0,
        };
        let finite = |value: f64| value.is_finite().then_some(value);
        let mut paired = Raster::new(80, 40);
        let mut alone = Raster::new(80, 40);
        let shared = draw_two_curves(
            &mut paired,
            layout,
            -2.0,
            2.0,
            |x| finite(numinous_core::eval(&graph, x, 2.0)),
            |x| finite(numinous_core::eval(&slope, x, 2.0)),
        )
        .expect("pair");
        let graph_only = draw_curve(&mut alone, layout, -2.0, 2.0, |x| {
            finite(numinous_core::eval(&graph, x, 2.0))
        })
        .expect("graph");
        assert!(shared.1 - shared.0 > graph_only.1 - graph_only.0);
        assert_ne!(paired.to_rgba(), alone.to_rgba());

        let mut fallback = Raster::new(80, 40);
        let mut reference = Raster::new(80, 40);
        draw_two_curves(
            &mut fallback,
            layout,
            -2.0,
            2.0,
            |x| finite(numinous_core::eval(&graph, x, 2.0)),
            |_| None,
        )
        .expect("graph alone");
        draw_curve(&mut reference, layout, -2.0, 2.0, |x| {
            finite(numinous_core::eval(&graph, x, 2.0))
        })
        .expect("reference");
        assert_eq!(fallback.to_rgba(), reference.to_rgba());
    }

    #[test]
    fn two_parametric_paths_share_one_frame() {
        let full_x = numinous_core::parse("cos(2*pi*t)+0.5*cos(6*pi*t)").expect("x");
        let full_y = numinous_core::parse("sin(2*pi*t)+0.5*sin(6*pi*t)").expect("y");
        let first_x = numinous_core::parse("cos(2*pi*t)").expect("first x");
        let first_y = numinous_core::parse("sin(2*pi*t)").expect("first y");
        let layout = CurveLayout {
            width: 80,
            height: 40,
            top: 0.0,
            bottom_margin: 1.0,
        };
        let point = |x: &numinous_core::Expr, y: &numinous_core::Expr, t: f64| {
            let px = numinous_core::eval(x, t, 1.0);
            let py = numinous_core::eval(y, t, 1.0);
            (px.is_finite() && py.is_finite()).then_some((px, py))
        };
        let mut paired = Raster::new(80, 40);
        let mut full_only = Raster::new(80, 40);
        let mut first_only = Raster::new(80, 40);
        let shared = draw_parametric_pair(
            &mut paired,
            layout,
            0.0,
            1.0,
            |t| point(&full_x, &full_y, t),
            |t| point(&first_x, &first_y, t),
        )
        .expect("pair");
        let full_bounds = draw_parametric(&mut full_only, layout, 0.0, 1.0, |t| {
            point(&full_x, &full_y, t)
        })
        .expect("full");
        let first_bounds = draw_parametric(&mut first_only, layout, 0.0, 1.0, |t| {
            point(&first_x, &first_y, t)
        })
        .expect("first");
        assert!(
            shared.1 > first_bounds.1 + 0.2,
            "the full path sticks out past the first circle: shared {shared:?}, first {first_bounds:?}"
        );
        assert!((shared.0 - full_bounds.0).abs() < 1e-9);
        assert!((shared.1 - full_bounds.1).abs() < 1e-9);
        assert!((shared.2 - full_bounds.2).abs() < 1e-9);
        assert!((shared.3 - full_bounds.3).abs() < 1e-9);
        assert_ne!(paired.to_rgba(), full_only.to_rgba());

        let mut fallback = Raster::new(80, 40);
        let mut reference = Raster::new(80, 40);
        draw_parametric_pair(
            &mut fallback,
            layout,
            0.0,
            1.0,
            |t| point(&full_x, &full_y, t),
            |_| None,
        )
        .expect("path alone");
        draw_parametric(&mut reference, layout, 0.0, 1.0, |t| {
            point(&full_x, &full_y, t)
        })
        .expect("reference");
        assert_eq!(fallback.to_rgba(), reference.to_rgba());
    }
}
