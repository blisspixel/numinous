//! Terminal-facing Studio input, capsule, and report policy.
//!
//! Expression evaluation, recipe resolution, capsule validation, and rendering
//! remain in `numinous-core`. This module translates the CLI's raw choices and
//! filesystem boundary into that shared contract and stable terminal prose.

use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use numinous_core::{
    FieldReading, FieldRequest, PathClosure, PlotRequest, PlotSource, SingRequest, SoundSpec,
    StudioCreation, StudioKind, StudioRequestError, StudioScale, StudioSlider, sliders_from_specs,
};

use crate::render_input::validate_render_dimensions;

/// One saved or rendered Studio domain and its portable pitch map.
#[derive(Clone, Copy)]
pub(super) struct StudioParameters {
    pub minimum: f64,
    pub maximum: f64,
    pub a: f64,
    pub scale: StudioScale,
}

/// Optional identity attached while saving a new creation.
#[derive(Clone, Copy)]
pub(super) struct CreationIdentity<'a> {
    pub title: Option<&'a str>,
    pub author: Option<&'a str>,
    pub credit: Option<&'a str>,
}

/// Atomic changes requested for one terminal fork.
#[derive(Clone, Copy)]
pub(super) struct ForkEdits<'a> {
    pub expr: Option<&'a str>,
    pub x_expr: Option<&'a str>,
    pub y_expr: Option<&'a str>,
    pub scale: Option<StudioScale>,
    pub sliders: &'a [StudioSlider],
    pub identity: CreationIdentity<'a>,
}

/// Translate mutually exclusive CLI discovery flags into one core source.
pub(super) fn resolve_plot_source(
    expr: Option<&str>,
    recipe: Option<u64>,
    seed: Option<u64>,
    auto_step: u64,
) -> Result<PlotSource, String> {
    let modes =
        usize::from(expr.is_some()) + usize::from(recipe.is_some()) + usize::from(seed.is_some());
    if modes != 1 {
        return Err(
            "plot needs exactly one of: expression, --recipe N, or --seed N (use --list-recipes)\n"
                .to_string(),
        );
    }
    if let Some(source) = expr {
        return Ok(PlotSource::Manual(source.to_string()));
    }
    if let Some(index) = recipe {
        if auto_step != 0 {
            return Err("--auto-step is only valid with --seed\n".to_string());
        }
        return Ok(PlotSource::Recipe(index));
    }
    let seed = seed.expect("seed present when exclusive");
    Ok(PlotSource::Seeded {
        seed,
        auto_step: (auto_step != 0).then_some(auto_step),
    })
}

/// Parse repeatable `--slider name=value` or `name=value:min:max` flags.
pub(super) fn parse_slider_specs(specs: &[String]) -> Result<Vec<StudioSlider>, String> {
    sliders_from_specs(specs).map_err(|error| format!("{error}\n"))
}

pub(super) fn with_sliders(
    creation: StudioCreation,
    sliders: &[StudioSlider],
) -> Result<StudioCreation, String> {
    if sliders.is_empty() {
        Ok(creation)
    } else {
        creation
            .with_sliders(sliders.to_vec())
            .map_err(|error| format!("{error}\n"))
    }
}

/// Plot `source` as y = f(x, a) over `[xmin, xmax]`, auto-scaling y.
pub(super) fn plot_report(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    width: usize,
    height: usize,
) -> Result<String, String> {
    plot_report_with(source, xmin, xmax, a, width, height, &[])
}

/// Plot a graph with named sliders bound.
pub(super) fn plot_report_with(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    width: usize,
    height: usize,
    sliders: &[StudioSlider],
) -> Result<String, String> {
    if source.contains('&') {
        let parts: Vec<String> = source
            .split('&')
            .map(|part| part.trim().to_string())
            .collect();
        let creation = with_sliders(StudioCreation::new_program(parts, xmin, xmax, a)?, sliders)?;
        return creation_report(&creation, width, height);
    }
    let mut request = PlotRequest::new(
        PlotSource::Manual(source.to_string()),
        Some(xmin),
        Some(xmax),
        Some(a),
        Some(width),
        Some(height),
    )
    .map_err(plot_request_error)?;
    if !sliders.is_empty() {
        request = request
            .with_sliders(sliders.to_vec())
            .map_err(plot_request_error)?;
    }
    let result = request.execute().map_err(plot_request_error)?;
    let pattern = StudioCreation::new(source, xmin, xmax, a)
        .ok()
        .and_then(|creation| with_sliders(creation, sliders).ok())
        .map(|creation| pattern_caption(&creation))
        .unwrap_or_default();
    Ok(format!(
        "y = {}    x in [{xmin:.3}, {xmax:.3}]    y in [{:.3}, {:.3}]{}{pattern}\n\n{}",
        terminal_safe(source),
        result.ymin,
        result.ymax,
        slider_caption(request.sliders()),
        result.text
    ))
}

fn slider_caption(sliders: &[StudioSlider]) -> String {
    if sliders.is_empty() {
        String::new()
    } else {
        let body = sliders
            .iter()
            .map(StudioSlider::to_spec)
            .collect::<Vec<_>>()
            .join(" ");
        format!("    sliders {body}")
    }
}

/// Plot one field over a requested rectangle of the plane.
#[expect(
    clippy::too_many_arguments,
    reason = "a field report is one source plus the rectangle, reading, and sliders"
)]
pub(super) fn field_report_with(
    source: &str,
    reading: FieldReading,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    a: f64,
    sliders: &[StudioSlider],
    size: (usize, usize),
) -> Result<String, String> {
    let mut request = FieldRequest::new(
        source,
        Some(reading),
        Some(xmin),
        Some(xmax),
        Some(ymin),
        Some(ymax),
        Some(a),
        Some(size.0),
        Some(size.1),
        Some(0.5),
    )
    .map_err(plot_request_error)?;
    if !sliders.is_empty() {
        request = request
            .with_sliders(sliders.to_vec())
            .map_err(plot_request_error)?;
    }
    let plate = request.execute().map_err(plot_request_error)?;
    Ok(format!(
        "f = {}    reading {}    x in [{:.3}, {:.3}]    y in [{:.3}, {:.3}]{}\n{}\n\n{}",
        terminal_safe(source),
        reading.name(),
        plate.x_bounds.0,
        plate.x_bounds.1,
        plate.y_bounds.0,
        plate.y_bounds.1,
        slider_caption(request.sliders()),
        reading.legend(),
        plate.text
    ))
}

#[expect(
    clippy::too_many_arguments,
    reason = "a field save is one source plus the rectangle, reading, and identity"
)]
pub(super) fn save_field_creation(
    source: &str,
    reading: FieldReading,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    a: f64,
    identity: CreationIdentity<'_>,
    sliders: &[StudioSlider],
    path: &Path,
) -> Result<String, String> {
    let creation = with_sliders(
        StudioCreation::new_field(source, xmin, xmax, ymin, ymax, a, reading)?,
        sliders,
    )?;
    save_creation(creation, identity, path)
}

/// Plot one parametric pair over `[tmin, tmax]`, auto-scaling both axes.
pub(super) fn parametric_report(
    x_source: &str,
    y_source: &str,
    parameters: StudioParameters,
    size: (usize, usize),
) -> Result<String, String> {
    parametric_report_with(x_source, y_source, parameters, size, &[])
}

/// Plot a parametric pair with named sliders bound.
pub(super) fn parametric_report_with(
    x_source: &str,
    y_source: &str,
    parameters: StudioParameters,
    size: (usize, usize),
    sliders: &[StudioSlider],
) -> Result<String, String> {
    let creation = with_sliders(
        StudioCreation::new_parametric(
            x_source,
            y_source,
            parameters.minimum,
            parameters.maximum,
            parameters.a,
        )?
        .with_scale(parameters.scale),
        sliders,
    )?;
    creation_report(&creation, size.0, size.1)
}

/// Render any validated Studio capsule through its own form.
pub(super) fn creation_report(
    creation: &StudioCreation,
    width: usize,
    height: usize,
) -> Result<String, String> {
    validate_render_dimensions(width, height)?;
    let plot = creation.plot_text(width, height).map_err(|error| {
        if error.contains("undefined") {
            "nothing to plot: the creation is undefined across this range\n".to_string()
        } else {
            format!("{}\n", terminal_safe(&error))
        }
    })?;
    match creation.kind() {
        StudioKind::Graph => Ok(format!(
            "y = {}    x in [{:.3}, {:.3}]    y in [{:.3}, {:.3}]{}{}\n\n{}",
            terminal_safe(creation.source()),
            creation.xmin(),
            creation.xmax(),
            plot.ymin,
            plot.ymax,
            slider_caption(creation.sliders()),
            pattern_caption(creation),
            plot.text
        )),
        StudioKind::Parametric => Ok(format!(
            "x(t) = {}    y(t) = {}\nt in [{:.3}, {:.3}]    x in [{:.3}, {:.3}]    y in [{:.3}, {:.3}]    scale {}{}\n\n{}",
            terminal_safe(creation.source()),
            terminal_safe(creation.second_source().expect("parametric y source")),
            creation.xmin(),
            creation.xmax(),
            plot.xmin,
            plot.xmax,
            plot.ymin,
            plot.ymax,
            creation.scale().name(),
            slider_caption(creation.sliders()),
            plot.text
        )),
        StudioKind::Field => {
            let reading = creation.reading().expect("field reading");
            Ok(format!(
                "f = {}    reading {}    x in [{:.3}, {:.3}]    y in [{:.3}, {:.3}]{}\n{}\n\n{}",
                terminal_safe(creation.source()),
                reading.name(),
                plot.xmin,
                plot.xmax,
                plot.ymin,
                plot.ymax,
                slider_caption(creation.sliders()),
                reading.legend(),
                plot.text
            ))
        }
        StudioKind::Program => {
            let legend = overlay_legend(&creation.graph_sources());
            Ok(format!(
                "y = {}    x in [{:.3}, {:.3}]    y in [{:.3}, {:.3}]{}{}\n{legend}\n\n{}",
                terminal_safe(&creation.editor_source()),
                creation.xmin(),
                creation.xmax(),
                plot.ymin,
                plot.ymax,
                slider_caption(creation.sliders()),
                pattern_caption(creation),
                plot.text
            ))
        }
    }
}

fn pattern_caption(creation: &StudioCreation) -> String {
    let rows = creation.pattern_rows();
    if rows.is_empty() {
        return String::new();
    }
    if creation.kind() == StudioKind::Program {
        let body = rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mark =
                    numinous_core::PROGRAM_MARKS[index.min(numinous_core::PROGRAM_MARKS.len() - 1)];
                format!("{mark} {row}")
            })
            .collect::<Vec<_>>()
            .join("    ");
        format!("\npattern {body}")
    } else {
        format!("\npattern {}", rows[0])
    }
}

fn overlay_legend(sources: &[String]) -> String {
    sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let mark =
                numinous_core::PROGRAM_MARKS[index.min(numinous_core::PROGRAM_MARKS.len() - 1)];
            format!("{mark} = {}", terminal_safe(source))
        })
        .collect::<Vec<_>>()
        .join("    ")
}

pub(super) fn plot_request_error(error: StudioRequestError) -> String {
    match error {
        StudioRequestError::Undefined => {
            "nothing to plot: the function is undefined across this range\n".to_string()
        }
        StudioRequestError::InvalidPlotSize { .. } => {
            "need width >= 2, height >= 2, and xmax > xmin\n".to_string()
        }
        StudioRequestError::PlotTooLarge { width, height } => {
            match validate_render_dimensions(width, height) {
                Err(message) => message,
                Ok(()) => format!("plot size {width}x{height} exceeds the core limit\n"),
            }
        }
        other => format!("{}\n", terminal_safe(&other.to_string())),
    }
}

/// WAV mixes every overlay graph; MIDI keeps the first graph.
pub(super) fn overlay_or_graph_melody(
    source: &str,
    xmin: f64,
    xmax: f64,
    notes: usize,
    a: f64,
    scale: StudioScale,
    sliders: &[StudioSlider],
) -> Result<(SoundSpec, SoundSpec), String> {
    if source.contains('&') {
        let parts: Vec<String> = source
            .split('&')
            .map(|part| part.trim().to_string())
            .collect();
        let mut creation = StudioCreation::new_program(parts, xmin, xmax, a)?;
        if !sliders.is_empty() {
            creation = creation.with_sliders(sliders.to_vec())?;
        }
        let creation = creation.with_scale(scale);
        let wav = creation.to_melody(notes);
        if wav.notes.is_empty() {
            return Err(sing_request_error(StudioRequestError::Undefined));
        }
        Ok((wav, creation.to_midi_melody(notes)))
    } else {
        let mut request = SingRequest::new(source, Some(xmin), Some(xmax), Some(a), Some(notes))
            .map_err(sing_request_error)?;
        if !sliders.is_empty() {
            request = request
                .with_sliders(sliders.to_vec())
                .map_err(sing_request_error)?;
        }
        let spec = request
            .execute_with_scale(scale)
            .map_err(sing_request_error)?;
        Ok((spec.clone(), spec))
    }
}

pub(super) fn sing_request_error(error: StudioRequestError) -> String {
    match error {
        StudioRequestError::Undefined => {
            "nothing to sing: the function is undefined across this range\n".to_string()
        }
        other => format!("{}\n", terminal_safe(&other.to_string())),
    }
}

/// Save a Studio creation as a `.num` file and return the share link. The
/// file uses the lowest capsule version that can carry its fields.
#[cfg(test)]
pub(super) fn save_studio_creation(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    title: Option<&str>,
    author: Option<&str>,
    path: &Path,
) -> Result<String, String> {
    save_studio_creation_with_scale(
        source,
        StudioParameters {
            minimum: xmin,
            maximum: xmax,
            a,
            scale: StudioScale::Continuous,
        },
        CreationIdentity {
            title,
            author,
            credit: None,
        },
        path,
    )
}

#[cfg(test)]
pub(super) fn save_studio_creation_with_scale(
    source: &str,
    parameters: StudioParameters,
    identity: CreationIdentity<'_>,
    path: &Path,
) -> Result<String, String> {
    let creation = with_sliders(
        StudioCreation::new(source, parameters.minimum, parameters.maximum, parameters.a)?
            .with_scale(parameters.scale),
        &[],
    )?;
    save_creation(creation, identity, path)
}

pub(super) fn save_studio_creation_with_sliders(
    source: &str,
    parameters: StudioParameters,
    identity: CreationIdentity<'_>,
    sliders: &[StudioSlider],
    path: &Path,
) -> Result<String, String> {
    let creation = with_sliders(
        if source.contains('&') {
            let parts: Vec<String> = source
                .split('&')
                .map(|part| part.trim().to_string())
                .collect();
            StudioCreation::new_program(
                parts,
                parameters.minimum,
                parameters.maximum,
                parameters.a,
            )?
        } else {
            StudioCreation::new(source, parameters.minimum, parameters.maximum, parameters.a)?
        }
        .with_scale(parameters.scale),
        sliders,
    )?;
    save_creation(creation, identity, path)
}

pub(super) fn save_parametric_creation(
    x_source: &str,
    y_source: &str,
    parameters: StudioParameters,
    identity: CreationIdentity<'_>,
    sliders: &[StudioSlider],
    path: &Path,
) -> Result<String, String> {
    let creation = with_sliders(
        StudioCreation::new_parametric(
            x_source,
            y_source,
            parameters.minimum,
            parameters.maximum,
            parameters.a,
        )?
        .with_scale(parameters.scale),
        sliders,
    )?;
    save_creation(creation, identity, path)
}

fn save_creation(
    mut creation: StudioCreation,
    identity: CreationIdentity<'_>,
    path: &Path,
) -> Result<String, String> {
    if let Some(title) = identity.title {
        creation = creation.with_title(title).map_err(|e| format!("{e}\n"))?;
    }
    if let Some(author) = identity.author {
        creation = creation.with_author(author).map_err(|e| format!("{e}\n"))?;
    }
    creation = creation
        .with_credit_override(identity.credit)
        .map_err(|error| format!("{error}\n"))?;
    write_create_new(path, creation.to_num_file().as_bytes())?;
    Ok(format!(
        "saved Studio creation: {}\nlink: {}\n",
        terminal_safe_path(path),
        creation.to_link()
    ))
}

/// A creation's voice travels through the terminal: sing accepts the same
/// `.num` files and links the rest of the Studio surface speaks, and the
/// capsule supplies its own window and knob unless flags override them.
type ResolvedSingInput = (String, f64, f64, f64, StudioScale, Vec<StudioSlider>);

pub(super) fn resolve_sing_input(
    input: &str,
    xmin: Option<f64>,
    xmax: Option<f64>,
    a: Option<f64>,
) -> Result<ResolvedSingInput, String> {
    if names_a_studio_creation(input) {
        let creation = load_studio_creation(input)?;
        if creation.kind() == StudioKind::Field {
            return Err("a field is seen first; it has no melody yet\n".to_string());
        }
        let source = if creation.kind() == StudioKind::Program {
            creation.editor_source()
        } else {
            creation
                .second_source()
                .unwrap_or_else(|| creation.source())
                .to_string()
        };
        return Ok((
            source,
            xmin.unwrap_or_else(|| creation.xmin()),
            xmax.unwrap_or_else(|| creation.xmax()),
            a.unwrap_or_else(|| creation.a()),
            creation.scale(),
            creation.sliders().to_vec(),
        ));
    }
    Ok((
        input.to_string(),
        xmin.unwrap_or(-std::f64::consts::TAU),
        xmax.unwrap_or(std::f64::consts::TAU),
        a.unwrap_or(1.0),
        StudioScale::Continuous,
        Vec::new(),
    ))
}

/// The terminal maker's remix verb.
#[cfg(test)]
pub(super) fn fork_studio_creation(
    parent_input: &str,
    expr: Option<&str>,
    title: Option<&str>,
    author: Option<&str>,
    out: &Path,
) -> Result<String, String> {
    fork_studio_creation_extended(
        parent_input,
        ForkEdits {
            expr,
            x_expr: None,
            y_expr: None,
            scale: None,
            sliders: &[],
            identity: CreationIdentity {
                title,
                author,
                credit: None,
            },
        },
        out,
    )
}

pub(super) fn fork_studio_creation_extended(
    parent_input: &str,
    edits: ForkEdits<'_>,
    out: &Path,
) -> Result<String, String> {
    let parent = load_studio_creation(parent_input)?;
    let mut fork = match parent.kind() {
        StudioKind::Graph | StudioKind::Program => {
            if edits.x_expr.is_some() || edits.y_expr.is_some() {
                return Err("a graph fork accepts --expr, not --x-expr or --y-expr\n".to_string());
            }
            parent.fork(edits.expr, edits.identity.title, edits.identity.author)
        }
        StudioKind::Field => {
            if edits.x_expr.is_some() || edits.y_expr.is_some() {
                return Err("a field fork accepts --expr, not --x-expr or --y-expr\n".to_string());
            }
            parent.fork_field(
                edits.expr,
                None,
                edits.identity.title,
                edits.identity.author,
            )
        }
        StudioKind::Parametric => {
            if edits.expr.is_some() {
                return Err(
                    "a parametric fork accepts --x-expr and --y-expr, not --expr\n".to_string(),
                );
            }
            parent.fork_parametric(
                edits.x_expr,
                edits.y_expr,
                edits.identity.title,
                edits.identity.author,
            )
        }
    }
    .map_err(|error| format!("{error}\n"))?;
    if let Some(scale) = edits.scale {
        fork = fork.with_scale(scale);
    }
    if !edits.sliders.is_empty() {
        fork = with_sliders(fork, edits.sliders)?;
    }
    fork = fork
        .with_credit_override(edits.identity.credit)
        .map_err(|error| format!("{error}\n"))?;
    write_create_new(out, fork.to_num_file().as_bytes())?;
    Ok(format!(
        "forked from {}\nsaved Studio creation: {}\nlink: {}\n",
        parent.to_link(),
        terminal_safe_path(out),
        fork.to_link()
    ))
}

pub(super) fn load_studio_creation(input: &str) -> Result<StudioCreation, String> {
    if let Some(creation) = numinous_core::studio_experiment(input) {
        return Ok(creation);
    }
    if input.starts_with("numinous://") {
        return StudioCreation::from_link(input)
            .map_err(|_| "invalid Numinous Studio link\n".to_string());
    }
    let path = Path::new(input);
    // The bounded read lives in the core loader every face shares; this face
    // only owns how each refusal is spoken to a terminal.
    StudioCreation::from_num_path(path).map_err(|error| match error {
        numinous_core::NumFileError::Io(e) => format!(
            "could not read Studio .num file '{}': {e}\n",
            terminal_safe_path(path)
        ),
        numinous_core::NumFileError::TooLarge => format!(
            "Studio .num file is too large; limit is {} bytes\n",
            numinous_core::MAX_SHARE_INPUT_BYTES
        ),
        numinous_core::NumFileError::Invalid(_) => {
            "invalid Numinous Studio .num file\n".to_string()
        }
    })
}

pub(super) fn open_studio_report(
    input: &str,
    width: usize,
    height: usize,
) -> Result<String, String> {
    let creation = load_studio_creation(input)?;
    let report = creation_report(&creation, width, height)?;
    let mut lines = vec!["Studio creation".to_string()];
    if let Some(title) = creation.title() {
        lines.push(format!("title={}", terminal_safe(title)));
    }
    if let Some(author) = creation.author() {
        lines.push(format!("author={}", terminal_safe(author)));
    }
    if let Some(credit) = creation.credit() {
        lines.push(format!("credit={}", terminal_safe(credit)));
    }
    lines.push(format!("kind={}", creation.kind().name()));
    match creation.kind() {
        StudioKind::Parametric => {
            lines.push(format!("xexpr={}", terminal_safe(creation.source())));
            lines.push(format!(
                "yexpr={}",
                terminal_safe(creation.second_source().expect("parametric y source"))
            ));
            lines.push(format!("tmin={}", creation.xmin()));
            lines.push(format!("tmax={}", creation.xmax()));
            lines.push(format!("scale={}", creation.scale().name()));
        }
        StudioKind::Field => {
            lines.push(format!("expr={}", terminal_safe(creation.source())));
            lines.push(format!("xmin={}", creation.xmin()));
            lines.push(format!("xmax={}", creation.xmax()));
            lines.push(format!("ymin={}", creation.ymin().expect("field ymin")));
            lines.push(format!("ymax={}", creation.ymax().expect("field ymax")));
            lines.push(format!(
                "reading={}",
                creation.reading().expect("field reading").name()
            ));
        }
        StudioKind::Graph => {
            lines.push(format!("expr={}", terminal_safe(creation.source())));
            lines.push(format!("xmin={}", creation.xmin()));
            lines.push(format!("xmax={}", creation.xmax()));
            lines.push(format!("scale={}", creation.scale().name()));
        }
        StudioKind::Program => {
            for source in creation.graph_sources() {
                lines.push(format!("expr={}", terminal_safe(&source)));
            }
            lines.push(format!("xmin={}", creation.xmin()));
            lines.push(format!("xmax={}", creation.xmax()));
            lines.push(format!("scale={}", creation.scale().name()));
        }
    }
    lines.push(format!("a={}", creation.a()));
    for slider in creation.sliders() {
        lines.push(format!("slider={}", slider.to_file_value()));
    }
    if let Some(era) = creation.era() {
        lines.push(format!("era={}", era.name()));
    }
    if let Some(descends) = creation.descends() {
        lines.push(format!("descends={}", terminal_safe(descends)));
    }
    lines.push(format!("link={}", creation.to_link()));
    // Quoted: the link's own & separators would otherwise split the command
    // in bash, PowerShell, and cmd alike.
    lines.push(format!(
        "remix it: numinous fork \"{}\" --out my-remix.num",
        creation.to_link()
    ));
    lines.extend(PathClosure::of(&creation).report_lines());
    for row in creation.pattern_rows() {
        lines.push(format!("pattern={}", row));
    }
    Ok(format!("{}\n\n{}", lines.join("\n"), report))
}

fn names_a_studio_creation(input: &str) -> bool {
    input.starts_with("numinous://")
        || Path::new(input)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("num"))
}

/// The first sibling name not already taken, bounded so a hostile directory
/// cannot spin this search forever.
fn next_free_sibling(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_str()?;
    let extension = path.extension()?.to_str()?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    (2..=99)
        .map(|n| parent.join(format!("{stem}-{n}.{extension}")))
        .find(|candidate| !candidate.exists())
}

fn write_create_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let base = path.file_name().unwrap_or_else(|| OsStr::new("studio.num"));
    let mut last_error = None;
    for attempt in 0..8 {
        let mut temp_name = base.to_os_string();
        temp_name.push(format!(".tmp.{}.{}", std::process::id(), attempt));
        let temp = parent.join(temp_name);
        let mut created_temp = false;
        let write_result = (|| -> Result<(), String> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|err| {
                    format!("could not create {}: {err}\n", terminal_safe_path(&temp))
                })?;
            created_temp = true;
            file.write_all(bytes)
                .map_err(|err| format!("could not write {}: {err}\n", terminal_safe_path(&temp)))?;
            file.flush()
                .map_err(|err| format!("could not flush {}: {err}\n", terminal_safe_path(&temp)))
        })();
        if let Err(message) = write_result {
            if created_temp {
                let _ = std::fs::remove_file(&temp);
            }
            last_error = Some(message);
            continue;
        }
        match std::fs::hard_link(&temp, path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&temp);
                return Ok(());
            }
            Err(err) => {
                let _ = std::fs::remove_file(&temp);
                if path.exists() {
                    let hint = next_free_sibling(path)
                        .map(|free| format!(" {} is free.", terminal_safe_path(&free)))
                        .unwrap_or_default();
                    return Err(format!(
                        "could not create {}: already exists.{hint}\n",
                        terminal_safe_path(path)
                    ));
                }
                last_error = Some(format!(
                    "could not create {}: {err}\n",
                    terminal_safe_path(path)
                ));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| format!("could not create {}\n", terminal_safe_path(path))))
}

fn terminal_safe(text: &str) -> String {
    numinous_core::display_safe(text)
}

fn terminal_safe_path(path: &Path) -> String {
    terminal_safe(&path.to_string_lossy())
}
