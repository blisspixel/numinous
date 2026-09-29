//! Independently checked closure of harmonic oscillators in Studio.
//!
//! A parametric path can ask whether the motion repeats. An overlay of the
//! same oscillators can ask how many cycles each graph completes, and whether
//! those frequencies share a period. Both answers come from the ideal model.
//! A picture is not the proof. The trial does not grade the player, and it
//! does not gate rooms, study, or creation.
//!
//! When that reading names two frequencies, those frequencies can also sound.
//! The tones are the oscillators. The sampled melody is a different reading
//! and is left untouched.
//!
//! A parametric path whose coordinates are each a sum of two such oscillators
//! can show the first term beside the path and sound one tone per recognized
//! frequency. That reading does not claim a period. The player's source stays
//! the source.

use crate::sound::SoundSpec;
use crate::studio::{Expr, Func, Op, StudioCreation, StudioKind, StudioProgram, eval_named};

/// One cycle per unit time, sounded at this many hertz.
///
/// Every other named frequency is this reference times its cycles per unit
/// time. The factor is the render map. It is not a new period and it does
/// not replace an irrational frequency with a nearby ratio.
pub const OSCILLATOR_TONE_REFERENCE_HZ: f32 = 110.0;

/// How long the two sustained tones sound when rendered without a device.
const OSCILLATOR_TONE_SECONDS: f32 = 1.5;

/// Peak amplitude of each sustained tone. Two of these stay inside one sample.
const OSCILLATOR_TONE_GAIN: f32 = 0.06;

/// What a Studio creation does in time, when that question is well posed.
#[derive(Debug, Clone, PartialEq)]
pub enum PathClosure {
    /// A graph is a height over x, not a planar path that can come home.
    Graph,
    /// A field is a value over the plane, not a path that can come home.
    Field,
    /// The pair is not two harmonic oscillators of the form
    /// `A*sin(w*t+p)` or `A*cos(w*t+p)` with a constant scale and phase.
    Unsupported,
    /// A common period exists in the ideal model.
    Periodic(PeriodicClosure),
    /// The two frequencies are incommensurate, so the ideal motion never
    /// repeats.
    Aperiodic(AperiodicClosure),
    /// Two overlay graphs, each a harmonic oscillator. Counts are ideal
    /// products over the saved window, not peaks in the picture.
    Voices(VoiceClosure),
}

/// Ideal cycle counts for exactly two overlay oscillators.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceClosure {
    /// The two graphs, in source order.
    pub voices: [VoiceFact; 2],
    /// Least positive common period, when the frequencies are commensurate.
    pub period_text: Option<String>,
    /// Whether the saved window itself is a common period. Absent when the
    /// window length is not an exact integer or quarter, so "not checked"
    /// is not stored as false.
    pub window_is_common_period: Option<bool>,
    /// How many least periods the window covers, when that quotient is a
    /// positive integer.
    pub window_periods: Option<u32>,
}

/// One overlay oscillator.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceFact {
    /// Cycles per unit x, for example `1`, `17/12`, or `sqrt(2)`.
    pub frequency_text: String,
    /// The same frequency as a finite float, for the tone map only.
    /// The text is the exact claim.
    pub cycles_per_unit: f64,
    /// Ideal cycles in the saved window, for example `12`, `17/12`, or
    /// `12*sqrt(2)`. Absent when the window length is not an exact integer
    /// or quarter.
    pub window_cycles: Option<String>,
}

/// One named frequency and the tone the render map gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct OscillatorTone {
    /// The frequency text closure already named.
    pub frequency_text: String,
    /// Cycles per unit time as a finite float. The text remains exact.
    pub cycles_per_unit: f64,
    /// Hertz after [`OSCILLATOR_TONE_REFERENCE_HZ`]. Absent when that
    /// product is not a positive finite `f32`.
    pub hz: Option<f32>,
}

/// The first partial sum of a two-term parametric path.
///
/// Both coordinates are sums of two oscillators in the form closure already
/// accepts, and each speed is a frequency closure can name. Terms are in
/// source order. A subtraction is stored as a sum with the second term
/// negated, so the two terms add to the coordinate the player wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct HarmonicPartial {
    /// First term of `x(t)`, then its signed second term.
    pub x_terms: [Expr; 2],
    /// First term of `y(t)`, then its signed second term.
    pub y_terms: [Expr; 2],
    /// Recognized frequencies in first-seen order. One tone each.
    pub frequencies: Vec<OscillatorTone>,
}

/// The two sustained tones of a closure that named two frequencies.
#[derive(Debug, Clone, PartialEq)]
pub struct OscillatorTones {
    /// Hertz of frequency text `1`.
    pub reference_hz: f32,
    /// The x oscillator, then the y oscillator. An overlay keeps source order.
    pub voices: [OscillatorTone; 2],
}

/// A repeating two-oscillator path.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicClosure {
    /// Least positive common period as exact prose, for example `12` or `2*pi`.
    pub period_text: String,
    /// The same period as a finite float, for window comparison only.
    pub period: f64,
    /// Cycles of the x oscillator per unit t, for example `1` or `2`.
    pub x_frequency_text: String,
    /// The x frequency as a finite float, for the tone map only.
    pub x_cycles_per_unit: f64,
    /// Cycles of the y oscillator per unit t, for example `17/12`.
    pub y_frequency_text: String,
    /// The y frequency as a finite float, for the tone map only.
    pub y_cycles_per_unit: f64,
    /// How many x oscillations fit in one common period, when that count is
    /// an integer.
    pub x_cycles: Option<i64>,
    /// How many y oscillations fit in one common period, when that count is
    /// an integer.
    pub y_cycles: Option<i64>,
    /// How many common periods the saved window covers, when it covers a
    /// whole number of them.
    pub window_periods: Option<u32>,
    /// State at half a period after the window start. Position can return
    /// here while velocity reverses.
    pub half_period: ClosureCheckpoint,
    /// State at the saved window end.
    pub window_end: ClosureCheckpoint,
}

/// An ideal path with no positive common period.
#[derive(Debug, Clone, PartialEq)]
pub struct AperiodicClosure {
    /// Cycles of the x oscillator per unit t.
    pub x_frequency_text: String,
    /// The x frequency as a finite float, for the tone map only.
    pub x_cycles_per_unit: f64,
    /// Cycles of the y oscillator per unit t.
    pub y_frequency_text: String,
    /// The y frequency as a finite float, for the tone map only.
    pub y_cycles_per_unit: f64,
    /// State at the saved window end. A near return is still not a period.
    pub window_end: ClosureCheckpoint,
}

/// Position and full-state comparison against the window start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosureCheckpoint {
    /// The time that was compared to the window start.
    pub t: f64,
    /// Both coordinates match the start.
    pub position_returns: bool,
    /// Position and both velocities match the start.
    pub state_returns: bool,
}

impl PathClosure {
    /// Analyze one Studio creation. Graphs and unrecognized pairs stay
    /// explicit rather than guessed.
    #[must_use]
    pub fn of(creation: &StudioCreation) -> Self {
        match creation.kind() {
            StudioKind::Graph => Self::Graph,
            StudioKind::Program => analyze_program(creation),
            StudioKind::Field => Self::Field,
            StudioKind::Parametric => analyze_parametric(creation),
        }
    }

    /// Terminal lines for a report. Empty when the creation has no planar
    /// path to discuss.
    #[must_use]
    pub fn report_lines(&self) -> Vec<String> {
        match self {
            Self::Graph | Self::Field | Self::Unsupported => Vec::new(),
            Self::Periodic(periodic) => {
                let mut lines = vec![format!(
                    "closure=periodic period={} x_freq={} y_freq={}",
                    periodic.period_text, periodic.x_frequency_text, periodic.y_frequency_text
                )];
                if let (Some(x_cycles), Some(y_cycles)) = (periodic.x_cycles, periodic.y_cycles) {
                    lines.push(format!("cycles_in_period x={x_cycles} y={y_cycles}"));
                }
                if let Some(count) = periodic.window_periods {
                    lines.push(format!("window covers {count} period(s)"));
                }
                lines.push(checkpoint_line("half-period", &periodic.half_period));
                lines.push(checkpoint_line("window-end", &periodic.window_end));
                lines
            }
            Self::Aperiodic(aperiodic) => vec![
                format!(
                    "closure=aperiodic x_freq={} y_freq={}",
                    aperiodic.x_frequency_text, aperiodic.y_frequency_text
                ),
                "ideal motion has no positive common period".to_string(),
                checkpoint_line("window-end", &aperiodic.window_end),
            ],
            Self::Voices(voices) => voice_lines(voices),
        }
    }

    /// One instrument caption for a status line. Graphs and unrecognized
    /// pairs stay quiet so ordinary Formula Jam is not a trial.
    #[must_use]
    pub fn status_caption(&self) -> Option<String> {
        match self {
            Self::Graph | Self::Field | Self::Unsupported => None,
            Self::Periodic(periodic) => {
                let mut line = format!("PERIOD {}", periodic.period_text);
                if periodic.half_period.position_returns && !periodic.half_period.state_returns {
                    line.push_str("  HALF: PLACE NOT STATE");
                }
                Some(line)
            }
            Self::Aperiodic(_) => Some("NO PERIOD".to_string()),
            Self::Voices(voices) => Some(voice_caption(voices)),
        }
    }

    /// Sustained tones for the two frequencies this closure already named.
    ///
    /// Graphs, fields, and unrecognized creations invent none. The capsule
    /// is not consulted again and is not changed.
    #[must_use]
    pub fn oscillator_tones(&self) -> Option<OscillatorTones> {
        let pair = match self {
            Self::Graph | Self::Field | Self::Unsupported => return None,
            Self::Periodic(periodic) => [
                oscillator_tone(&periodic.x_frequency_text, periodic.x_cycles_per_unit),
                oscillator_tone(&periodic.y_frequency_text, periodic.y_cycles_per_unit),
            ],
            Self::Aperiodic(aperiodic) => [
                oscillator_tone(&aperiodic.x_frequency_text, aperiodic.x_cycles_per_unit),
                oscillator_tone(&aperiodic.y_frequency_text, aperiodic.y_cycles_per_unit),
            ],
            Self::Voices(voices) => [
                oscillator_tone(
                    &voices.voices[0].frequency_text,
                    voices.voices[0].cycles_per_unit,
                ),
                oscillator_tone(
                    &voices.voices[1].frequency_text,
                    voices.voices[1].cycles_per_unit,
                ),
            ],
        };
        Some(OscillatorTones {
            reference_hz: OSCILLATOR_TONE_REFERENCE_HZ,
            voices: pair,
        })
    }
}

impl OscillatorTones {
    /// Two held sines at the mapped hertz. Absent when either tone has no hz.
    ///
    /// The type 0 MIDI projection keeps one note when two start together,
    /// so this chord is the PCM hearing. The melody file stays the sampled
    /// curve.
    #[must_use]
    pub fn sound(&self) -> Option<SoundSpec> {
        let left = self.voices[0].hz?;
        let right = self.voices[1].hz?;
        Some(SoundSpec::chord(
            &[left, right],
            OSCILLATOR_TONE_SECONDS,
            OSCILLATOR_TONE_GAIN,
        ))
    }

    /// Terminal lines for the tone reading. Empty of any period claim.
    #[must_use]
    pub fn report_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "tones basis=ideal reference_hz={}",
            self.reference_hz
        )];
        for (index, voice) in self.voices.iter().enumerate() {
            let mut line = format!("tone {} freq={}", index + 1, voice.frequency_text);
            if let Some(hz) = voice.hz {
                line.push_str(" hz=");
                line.push_str(&hz.to_string());
            }
            lines.push(line);
        }
        lines
    }
}

impl HarmonicPartial {
    /// Read one parametric creation. Anything else is absent.
    ///
    /// A single oscillator pair stays with closure. A third term, a graph,
    /// a field, and an overlay are absent. The parameter must be one of the
    /// exact values closure already accepts. The capsule is not modified.
    #[must_use]
    pub fn of(creation: &StudioCreation) -> Option<Self> {
        if creation.kind() != StudioKind::Parametric {
            return None;
        }
        let Ok(program) = creation.program() else {
            return None;
        };
        let StudioProgram::Parametric {
            x_expression,
            y_expression,
            ..
        } = program
        else {
            return None;
        };
        let parameter = Exact::from_f64(creation.a())?;
        let sliders = creation.sliders();
        let x_terms = split_terms(&x_expression, parameter, sliders)?;
        let y_terms = split_terms(&y_expression, parameter, sliders)?;
        let mut frequencies = Vec::new();
        for term in x_terms.iter().chain(y_terms.iter()) {
            if frequencies
                .iter()
                .any(|tone: &OscillatorTone| tone.frequency_text == term.frequency_text)
            {
                continue;
            }
            frequencies.push(oscillator_tone(&term.frequency_text, term.cycles));
        }
        Some(Self {
            x_terms: [x_terms[0].expression.clone(), x_terms[1].expression.clone()],
            y_terms: [y_terms[0].expression.clone(), y_terms[1].expression.clone()],
            frequencies,
        })
    }

    /// The first term as a point. Absent when either coordinate is non-finite.
    #[must_use]
    pub fn first_point(
        &self,
        t: f64,
        a: f64,
        sliders: &[crate::slider::StudioSlider],
    ) -> Option<(f64, f64)> {
        let x = eval_named(&self.x_terms[0], t, a, sliders);
        let y = eval_named(&self.y_terms[0], t, a, sliders);
        (x.is_finite() && y.is_finite()).then_some((x, y))
    }

    /// One sustained tone per recognized frequency.
    ///
    /// Absent when any tone has no hertz. This is the live App voice. The
    /// sampled melody and its MIDI file stay the player's source. Type 0
    /// MIDI keeps one note when several start together, so the chord is the
    /// PCM hearing.
    #[must_use]
    pub fn sound(&self) -> Option<SoundSpec> {
        let mut freqs = Vec::with_capacity(self.frequencies.len());
        for tone in &self.frequencies {
            freqs.push(tone.hz?);
        }
        if freqs.is_empty() {
            return None;
        }
        Some(SoundSpec::chord(
            &freqs,
            OSCILLATOR_TONE_SECONDS,
            OSCILLATOR_TONE_GAIN,
        ))
    }

    /// Terminal lines for the reading. Empty of any period claim.
    #[must_use]
    pub fn report_lines(&self) -> Vec<String> {
        let mut lines = vec!["partial basis=sum".to_string()];
        for (index, tone) in self.frequencies.iter().enumerate() {
            let mut line = format!("term {} freq={}", index + 1, tone.frequency_text);
            if let Some(hz) = tone.hz {
                line.push_str(" hz=");
                line.push_str(&hz.to_string());
            }
            lines.push(line);
        }
        lines
    }

    /// Status text: `PARTIAL` and each frequency in first-seen order.
    #[must_use]
    pub fn status_caption(&self) -> String {
        let mut line = String::from("PARTIAL");
        for tone in &self.frequencies {
            line.push_str("  ");
            line.push_str(&tone.frequency_text);
        }
        line
    }
}

fn voice_lines(voices: &VoiceClosure) -> Vec<String> {
    let mut lines = vec!["closure=voices basis=ideal".to_string()];
    for (index, voice) in voices.voices.iter().enumerate() {
        let mut line = format!("voice {} freq={}", index + 1, voice.frequency_text);
        if let Some(cycles) = &voice.window_cycles {
            line.push_str(" cycles_in_window=");
            line.push_str(cycles);
        }
        lines.push(line);
    }
    match &voices.period_text {
        Some(period) => {
            lines.push(format!("common_period={period}"));
            if let Some(count) = voices.window_periods {
                lines.push(format!("window covers {count} period(s)"));
            }
        }
        None => {
            lines.push("common_period=none".to_string());
            lines.push("ideal motion has no positive common period".to_string());
        }
    }
    lines
}

fn voice_caption(voices: &VoiceClosure) -> String {
    let counts = voices
        .voices
        .iter()
        .map(|voice| voice.window_cycles.as_deref().unwrap_or("UNCOUNTED"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut line = format!("CYCLES {counts}");
    if let Some(period) = &voices.period_text {
        line.push_str("  PERIOD ");
        line.push_str(period);
        if voices.window_is_common_period == Some(false) {
            line.push_str("  NOT THIS WINDOW");
        }
    } else {
        line.push_str("  NO PERIOD");
    }
    line
}

fn checkpoint_line(name: &str, checkpoint: &ClosureCheckpoint) -> String {
    let position = if checkpoint.position_returns {
        "position returns"
    } else {
        "position does not return"
    };
    let state = if checkpoint.state_returns {
        "state returns"
    } else {
        "state does not return"
    };
    format!("{name} t={} {position}; {state}", checkpoint.t)
}

fn analyze_parametric(creation: &StudioCreation) -> PathClosure {
    let Ok(StudioProgram::Parametric {
        x_expression,
        y_expression,
        ..
    }) = creation.program()
    else {
        return PathClosure::Unsupported;
    };
    let parameter = match Exact::from_f64(creation.a()) {
        Some(value) => value,
        None => return PathClosure::Unsupported,
    };
    let sliders = creation.sliders();
    let Some(x_osc) = oscillator(&x_expression, parameter, sliders) else {
        return PathClosure::Unsupported;
    };
    let Some(y_osc) = oscillator(&y_expression, parameter, sliders) else {
        return PathClosure::Unsupported;
    };
    let Some(x_freq) = frequency_cycles(&x_osc.omega) else {
        return PathClosure::Unsupported;
    };
    let Some(y_freq) = frequency_cycles(&y_osc.omega) else {
        return PathClosure::Unsupported;
    };
    let tmin = creation.xmin();
    let tmax = creation.xmax();
    if !tmin.is_finite() || !tmax.is_finite() || tmax <= tmin {
        return PathClosure::Unsupported;
    }
    let start = state(&x_osc, &y_osc, tmin);
    let window_end = checkpoint(&x_osc, &y_osc, start, tmax);
    match common_period_all(&[x_freq, y_freq]) {
        PeriodFinding::Unresolved => PathClosure::Unsupported,
        PeriodFinding::Aperiodic => PathClosure::Aperiodic(AperiodicClosure {
            x_frequency_text: x_freq.text(),
            x_cycles_per_unit: cycles_per_unit(&x_freq),
            y_frequency_text: y_freq.text(),
            y_cycles_per_unit: cycles_per_unit(&y_freq),
            window_end,
        }),
        PeriodFinding::Periodic(period) => {
            let half_t = tmin + period.value / 2.0;
            PathClosure::Periodic(PeriodicClosure {
                period_text: period.text.clone(),
                period: period.value,
                x_frequency_text: x_freq.text(),
                x_cycles_per_unit: cycles_per_unit(&x_freq),
                y_frequency_text: y_freq.text(),
                y_cycles_per_unit: cycles_per_unit(&y_freq),
                x_cycles: integer_cycles(&x_freq, &period),
                y_cycles: integer_cycles(&y_freq, &period),
                window_periods: whole_periods_exact(tmax - tmin, &period),
                half_period: checkpoint(&x_osc, &y_osc, start, half_t),
                window_end,
            })
        }
    }
}

fn analyze_program(creation: &StudioCreation) -> PathClosure {
    let Ok(program) = creation.program() else {
        return PathClosure::Graph;
    };
    let expressions = program.overlay_expressions();
    // Three or four graphs are a different object. Guessing a period for a
    // sum, a rhythm, or a third oscillator would teach the wrong question.
    if expressions.len() != 2 {
        return PathClosure::Graph;
    }
    let Some(parameter) = Exact::from_f64(creation.a()) else {
        return PathClosure::Graph;
    };
    let sliders = creation.sliders();
    let mut frequencies = Vec::with_capacity(2);
    for expression in expressions {
        let Some(oscillator) = oscillator(expression, parameter, sliders) else {
            return PathClosure::Graph;
        };
        let Some(frequency) = frequency_cycles(&oscillator.omega) else {
            return PathClosure::Graph;
        };
        frequencies.push(frequency);
    }
    let tmin = creation.xmin();
    let tmax = creation.xmax();
    if !tmin.is_finite() || !tmax.is_finite() || tmax <= tmin {
        return PathClosure::Graph;
    }
    let span = tmax - tmin;
    let left = voice_fact(&frequencies[0], span);
    let right = voice_fact(&frequencies[1], span);
    let window_is_common_period = match (&left.window_cycles, &right.window_cycles) {
        (Some(left_cycles), Some(right_cycles)) => {
            Some(is_integer_text(left_cycles) && is_integer_text(right_cycles))
        }
        _ => None,
    };
    match common_period_all(&frequencies) {
        PeriodFinding::Unresolved => PathClosure::Graph,
        PeriodFinding::Aperiodic => PathClosure::Voices(VoiceClosure {
            voices: [left, right],
            period_text: None,
            window_is_common_period,
            window_periods: None,
        }),
        PeriodFinding::Periodic(period) => PathClosure::Voices(VoiceClosure {
            voices: [left, right],
            window_periods: whole_periods_exact(span, &period),
            window_is_common_period,
            period_text: Some(period.text),
        }),
    }
}

fn voice_fact(frequency: &Frequency, span: f64) -> VoiceFact {
    VoiceFact {
        frequency_text: frequency.text(),
        cycles_per_unit: cycles_per_unit(frequency),
        window_cycles: cycle_text(frequency, span),
    }
}

fn cycles_per_unit(frequency: &Frequency) -> f64 {
    if frequency.den == 0 {
        return f64::NAN;
    }
    let mut value = frequency.num as f64 / frequency.den as f64;
    if frequency.rad != 1 {
        value *= (frequency.rad as f64).sqrt();
    }
    value
}

fn oscillator_tone(frequency_text: &str, cycles_per_unit: f64) -> OscillatorTone {
    let hz = if cycles_per_unit.is_finite() && cycles_per_unit != 0.0 {
        let hz = f64::from(OSCILLATOR_TONE_REFERENCE_HZ) * cycles_per_unit.abs();
        let hz = hz as f32;
        (hz.is_finite() && hz > 0.0).then_some(hz)
    } else {
        None
    };
    OscillatorTone {
        frequency_text: frequency_text.to_string(),
        cycles_per_unit,
        hz,
    }
}

fn is_integer_text(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|mark| mark.is_ascii_digit() || mark == '-')
        && text != "-"
}

#[derive(Debug, Clone, Copy)]
struct Oscillator {
    kind: Func,
    amp: Exact,
    omega: Exact,
    phase: Exact,
}

#[derive(Debug, Clone, Copy)]
struct State {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn oscillator(
    expr: &Expr,
    parameter: Exact,
    sliders: &[crate::slider::StudioSlider],
) -> Option<Oscillator> {
    match expr {
        Expr::Call(kind @ (Func::Sin | Func::Cos), arg) => {
            let (phase, omega) = affine(arg, parameter, sliders)?;
            if omega.is_zero() {
                return None;
            }
            Some(Oscillator {
                kind: *kind,
                amp: Exact::one(),
                omega,
                phase,
            })
        }
        Expr::Neg(inner) => {
            let mut osc = oscillator(inner, parameter, sliders)?;
            osc.amp = osc.amp.checked_neg()?;
            Some(osc)
        }
        Expr::Bin(Op::Mul, left, right) => {
            if let Some(scale) = constant(left, parameter, sliders) {
                let mut osc = oscillator(right, parameter, sliders)?;
                osc.amp = osc.amp.checked_mul(scale)?;
                return Some(osc);
            }
            if let Some(scale) = constant(right, parameter, sliders) {
                let mut osc = oscillator(left, parameter, sliders)?;
                osc.amp = osc.amp.checked_mul(scale)?;
                return Some(osc);
            }
            None
        }
        Expr::Bin(Op::Div, left, right) => {
            let scale = constant(right, parameter, sliders)?;
            let mut osc = oscillator(left, parameter, sliders)?;
            osc.amp = osc.amp.checked_div(scale)?;
            Some(osc)
        }
        _ => None,
    }
}

struct SignedTerm {
    expression: Expr,
    frequency_text: String,
    cycles: f64,
}

fn split_terms(
    expr: &Expr,
    parameter: Exact,
    sliders: &[crate::slider::StudioSlider],
) -> Option<[SignedTerm; 2]> {
    let (left, right, subtract) = match expr {
        Expr::Bin(Op::Add, left, right) => (left.as_ref(), right.as_ref(), false),
        Expr::Bin(Op::Sub, left, right) => (left.as_ref(), right.as_ref(), true),
        _ => return None,
    };
    let right_expr = if subtract {
        Expr::Neg(Box::new(right.clone()))
    } else {
        right.clone()
    };
    Some([
        signed_term(left, parameter, sliders)?,
        signed_term(&right_expr, parameter, sliders)?,
    ])
}

fn signed_term(
    expr: &Expr,
    parameter: Exact,
    sliders: &[crate::slider::StudioSlider],
) -> Option<SignedTerm> {
    let osc = oscillator(expr, parameter, sliders)?;
    let frequency = frequency_cycles(&osc.omega)?;
    Some(SignedTerm {
        expression: expr.clone(),
        frequency_text: frequency.text(),
        cycles: cycles_per_unit(&frequency),
    })
}

fn affine(
    expr: &Expr,
    parameter: Exact,
    sliders: &[crate::slider::StudioSlider],
) -> Option<(Exact, Exact)> {
    match expr {
        Expr::Var => Some((Exact::zero(), Exact::one())),
        Expr::Neg(inner) => {
            let (phase, omega) = affine(inner, parameter, sliders)?;
            Some((phase.checked_neg()?, omega.checked_neg()?))
        }
        Expr::Bin(Op::Add, left, right) => {
            let (p0, w0) = affine(left, parameter, sliders)?;
            let (p1, w1) = affine(right, parameter, sliders)?;
            Some((p0.checked_add(p1)?, w0.checked_add(w1)?))
        }
        Expr::Bin(Op::Sub, left, right) => {
            let (p0, w0) = affine(left, parameter, sliders)?;
            let (p1, w1) = affine(right, parameter, sliders)?;
            Some((p0.checked_sub(p1)?, w0.checked_sub(w1)?))
        }
        Expr::Bin(Op::Mul, left, right) => {
            if let Some(scale) = constant(left, parameter, sliders) {
                let (phase, omega) = affine(right, parameter, sliders)?;
                return Some((phase.checked_mul(scale)?, omega.checked_mul(scale)?));
            }
            if let Some(scale) = constant(right, parameter, sliders) {
                let (phase, omega) = affine(left, parameter, sliders)?;
                return Some((phase.checked_mul(scale)?, omega.checked_mul(scale)?));
            }
            None
        }
        Expr::Bin(Op::Div, left, right) => {
            let scale = constant(right, parameter, sliders)?;
            let (phase, omega) = affine(left, parameter, sliders)?;
            Some((phase.checked_div(scale)?, omega.checked_div(scale)?))
        }
        _ => {
            let value = constant(expr, parameter, sliders)?;
            Some((value, Exact::zero()))
        }
    }
}

fn constant(
    expr: &Expr,
    parameter: Exact,
    sliders: &[crate::slider::StudioSlider],
) -> Option<Exact> {
    match expr {
        Expr::Num(value) => Exact::from_f64(*value),
        Expr::Param => Some(parameter),
        Expr::Slider(name) => Exact::from_f64(crate::slider::slider_value(name, sliders)),
        // The variable, and the field leaves a curve grammar cannot
        // produce, are not constants.
        Expr::Var | Expr::VarIm | Expr::Point | Expr::ImagUnit => None,
        Expr::Neg(inner) => constant(inner, parameter, sliders)?.checked_neg(),
        Expr::Bin(Op::Add, left, right) => {
            constant(left, parameter, sliders)?.checked_add(constant(right, parameter, sliders)?)
        }
        Expr::Bin(Op::Sub, left, right) => {
            constant(left, parameter, sliders)?.checked_sub(constant(right, parameter, sliders)?)
        }
        Expr::Bin(Op::Mul, left, right) => {
            constant(left, parameter, sliders)?.checked_mul(constant(right, parameter, sliders)?)
        }
        Expr::Bin(Op::Div, left, right) => {
            constant(left, parameter, sliders)?.checked_div(constant(right, parameter, sliders)?)
        }
        Expr::Call(Func::Sqrt, arg) => constant(arg, parameter, sliders)?.checked_sqrt(),
        Expr::Call(_, _)
        | Expr::Bin(Op::Pow, _, _)
        | Expr::PairCall(_, _, _)
        | Expr::Pattern(_)
        | Expr::Notes(_) => None,
    }
}

fn state(x_osc: &Oscillator, y_osc: &Oscillator, t: f64) -> State {
    let (x, vx) = sample(x_osc, t);
    let (y, vy) = sample(y_osc, t);
    State { x, y, vx, vy }
}

fn sample(osc: &Oscillator, t: f64) -> (f64, f64) {
    let omega = osc.omega.to_f64();
    let phase = osc.phase.to_f64() + omega * t;
    let amp = osc.amp.to_f64();
    match osc.kind {
        Func::Cos => (amp * phase.cos(), -amp * omega * phase.sin()),
        Func::Sin => (amp * phase.sin(), amp * omega * phase.cos()),
        _ => (f64::NAN, f64::NAN),
    }
}

fn checkpoint(x_osc: &Oscillator, y_osc: &Oscillator, start: State, t: f64) -> ClosureCheckpoint {
    let here = state(x_osc, y_osc, t);
    let position = near(here.x, start.x) && near(here.y, start.y);
    let velocity = near(here.vx, start.vx) && near(here.vy, start.vy);
    ClosureCheckpoint {
        t,
        position_returns: position,
        state_returns: position && velocity,
    }
}

fn near(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1e-8 * scale
}

fn whole_periods_exact(span: f64, period: &Period) -> Option<u32> {
    // A shared square root makes the period irrational. A rational window
    // then covers a whole number of periods only in cases this exact span
    // cannot see, so the trial abstains instead of rounding.
    if period.rad != 1 {
        return None;
    }
    let span = rational_span(span)?;
    let numer = span.num.checked_mul(i64::try_from(period.den).ok()?)?;
    let denom = span.den.checked_mul(period.num.unsigned_abs())?;
    let denom = i64::try_from(denom).ok()?;
    if denom == 0 || numer % denom != 0 {
        return None;
    }
    let count = numer / denom;
    u32::try_from(count).ok().filter(|count| *count >= 1)
}

fn cycle_text(frequency: &Frequency, span: f64) -> Option<String> {
    let span = rational_span(span)?;
    let numer = frequency.num.checked_mul(span.num)?;
    let denom = frequency.den.checked_mul(span.den)?;
    let (numer, denom) = reduce(numer, denom);
    Some(radical_text(numer, denom, frequency.rad))
}

fn rational_span(span: f64) -> Option<Exact> {
    let span = Exact::from_f64(span)?;
    if span.pi != 0 || span.rad != 1 || span.num <= 0 {
        return None;
    }
    Some(span)
}

fn integer_cycles(frequency: &Frequency, period: &Period) -> Option<i64> {
    if frequency.rad != period.rad {
        return None;
    }
    let numer = frequency.num.checked_mul(period.num)?;
    let denom = frequency.den.checked_mul(period.den)?;
    let denom = i64::try_from(denom).ok()?;
    if denom == 0 || numer % denom != 0 {
        return None;
    }
    Some(numer / denom)
}

/// Cycles per unit t: `(num/den) * sqrt(rad)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frequency {
    num: i64,
    den: u64,
    rad: u64,
}

impl Frequency {
    fn text(self) -> String {
        let body = format_ratio(self.num, self.den);
        if self.rad == 1 {
            body
        } else if self.num == 1 && self.den == 1 {
            format!("sqrt({})", self.rad)
        } else {
            format!("{body}*sqrt({})", self.rad)
        }
    }
}

struct Period {
    text: String,
    value: f64,
    /// Rational coefficient `num/den`. The period is that coefficient divided
    /// by `sqrt(rad)`.
    num: i64,
    den: u64,
    rad: u64,
}

enum PeriodFinding {
    Periodic(Period),
    /// The frequency ratio is irrational in this one-radical model.
    Aperiodic,
    /// The arithmetic did not fit. This is not evidence of either answer.
    Unresolved,
}

fn frequency_cycles(omega: &Exact) -> Option<Frequency> {
    // f = omega / (2*pi). Requires omega to carry exactly one pi so the
    // quotient is a rational times a square root.
    if omega.pi != 1 || omega.is_zero() {
        return None;
    }
    let (num, den) = reduce(omega.num, omega.den.checked_mul(2)?);
    Some(Frequency {
        num,
        den,
        rad: omega.rad,
    })
}

fn common_period_all(frequencies: &[Frequency]) -> PeriodFinding {
    if frequencies.len() < 2 {
        return PeriodFinding::Unresolved;
    }
    let rad = frequencies[0].rad;
    if frequencies.iter().any(|frequency| frequency.rad != rad) {
        return PeriodFinding::Aperiodic;
    }
    if frequencies.iter().any(|frequency| frequency.num == 0) {
        return PeriodFinding::Unresolved;
    }
    let mut den = 1u64;
    for frequency in frequencies {
        match lcm_u64(den, frequency.den) {
            Some(next) if next != 0 => den = next,
            _ => return PeriodFinding::Unresolved,
        }
    }
    let mut shared: Option<u64> = None;
    for frequency in frequencies {
        let scale = den / frequency.den;
        let Some(cycles) = frequency.num.unsigned_abs().checked_mul(scale) else {
            return PeriodFinding::Unresolved;
        };
        if cycles == 0 {
            return PeriodFinding::Unresolved;
        }
        shared = Some(match shared {
            Some(previous) => gcd_u64(previous, cycles),
            None => cycles,
        });
    }
    let Some(shared) = shared.filter(|shared| *shared != 0) else {
        return PeriodFinding::Unresolved;
    };
    let Ok(den_i) = i64::try_from(den) else {
        return PeriodFinding::Unresolved;
    };
    let (num, den) = reduce(den_i, shared);
    if num <= 0 || den == 0 {
        return PeriodFinding::Unresolved;
    }
    let mut value = num as f64 / den as f64;
    if rad != 1 {
        value /= (rad as f64).sqrt();
    }
    if !value.is_finite() || value <= 0.0 {
        return PeriodFinding::Unresolved;
    }
    PeriodFinding::Periodic(Period {
        text: period_text(num, den, rad),
        value,
        num,
        den,
        rad,
    })
}

fn period_text(num: i64, den: u64, rad: u64) -> String {
    if rad == 1 {
        return format_ratio(num, den);
    }
    // (num/den)/sqrt(rad) = num*sqrt(rad)/(den*rad), then reduce the coefficient.
    let Some(denom) = den.checked_mul(rad) else {
        return format!("{num}/({den}*sqrt({rad}))");
    };
    let (num, denom) = reduce(num, denom);
    radical_text(num, denom, rad)
}

fn radical_text(num: i64, den: u64, rad: u64) -> String {
    if rad == 1 {
        return format_ratio(num, den);
    }
    let radical = format!("sqrt({rad})");
    if num == 1 && den == 1 {
        radical
    } else if den == 1 {
        format!("{num}*{radical}")
    } else if num == 1 {
        format!("{radical}/{den}")
    } else {
        format!("{num}*{radical}/{den}")
    }
}

fn format_ratio(num: i64, den: u64) -> String {
    if den == 1 {
        num.to_string()
    } else {
        format!("{num}/{den}")
    }
}

/// Exact constant: `(num/den) * pi^pi * sqrt(rad)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Exact {
    num: i64,
    den: u64,
    pi: i8,
    rad: u64,
}

impl Exact {
    const fn zero() -> Self {
        Self {
            num: 0,
            den: 1,
            pi: 0,
            rad: 1,
        }
    }

    const fn one() -> Self {
        Self {
            num: 1,
            den: 1,
            pi: 0,
            rad: 1,
        }
    }

    fn is_zero(self) -> bool {
        self.num == 0
    }

    fn from_int(value: i64) -> Self {
        Self {
            num: value,
            den: 1,
            pi: 0,
            rad: 1,
        }
        .normalized()
    }

    fn from_f64(value: f64) -> Option<Self> {
        if value.to_bits() == std::f64::consts::PI.to_bits() {
            return Some(Self {
                num: 1,
                den: 1,
                pi: 1,
                rad: 1,
            });
        }
        if !value.is_finite() {
            return None;
        }
        if value.fract() == 0.0 && (-1e12..=1e12).contains(&value) {
            return Some(Self::from_int(value as i64));
        }
        // Saved knob steps are quarters.
        let quarters = value * 4.0;
        if quarters.fract() == 0.0 && (-1e12..=1e12).contains(&quarters) {
            return Some(
                Self {
                    num: quarters as i64,
                    den: 4,
                    pi: 0,
                    rad: 1,
                }
                .normalized(),
            );
        }
        None
    }

    fn to_f64(self) -> f64 {
        let mut value = self.num as f64 / self.den as f64;
        match self.pi {
            0 => {}
            1 => value *= std::f64::consts::PI,
            -1 => value /= std::f64::consts::PI,
            _ => value *= std::f64::consts::PI.powi(i32::from(self.pi)),
        }
        if self.rad != 1 {
            value *= (self.rad as f64).sqrt();
        }
        value
    }

    fn normalized(self) -> Self {
        if self.num == 0 {
            return Self::zero();
        }
        let (square, rest) = split_square(self.rad);
        let mut num = self.num;
        let den = self.den;
        if square > 1
            && let Ok(factor) = i64::try_from(square)
        {
            num = num.saturating_mul(factor);
        }
        let (num, den) = reduce(num, den);
        Self {
            num,
            den,
            pi: self.pi,
            rad: rest.max(1),
        }
    }

    fn checked_neg(self) -> Option<Self> {
        Some(
            Self {
                num: self.num.checked_neg()?,
                ..self
            }
            .normalized(),
        )
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        if self.is_zero() {
            return Some(other);
        }
        if other.is_zero() {
            return Some(self);
        }
        if self.pi != other.pi || self.rad != other.rad {
            return None;
        }
        let den = lcm_u64(self.den, other.den)?;
        let left = (den / self.den) as i64;
        let right = (den / other.den) as i64;
        let num = self
            .num
            .checked_mul(left)?
            .checked_add(other.num.checked_mul(right)?)?;
        Some(
            Self {
                num,
                den,
                pi: self.pi,
                rad: self.rad,
            }
            .normalized(),
        )
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_add(other.checked_neg()?)
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        if self.is_zero() || other.is_zero() {
            return Some(Self::zero());
        }
        let num = self.num.checked_mul(other.num)?;
        let den = self.den.checked_mul(other.den)?;
        let rad = self.rad.checked_mul(other.rad)?;
        let pi = self.pi.checked_add(other.pi)?;
        Some(Self { num, den, pi, rad }.normalized())
    }

    fn checked_div(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Self::zero());
        }
        let num = self.num.checked_mul(other.den as i64)?;
        let den = self.den.checked_mul(other.num.unsigned_abs())?;
        let num = if other.num < 0 {
            num.checked_neg()?
        } else {
            num
        };
        let rad = self.rad.checked_mul(other.rad)?;
        // sqrt in the denominator: multiply num and den by sqrt(rad) via rad
        // in both, then the extra rad in den is a square and normalizes out
        // of the radicand after we put other.rad into den as an integer by
        // writing 1/sqrt(r) = sqrt(r)/r.
        let num = num.checked_mul(i64::try_from(other.rad).ok()?)?;
        let den = den.checked_mul(other.rad)?;
        let pi = self.pi.checked_sub(other.pi)?;
        Some(Self { num, den, pi, rad }.normalized())
    }

    fn checked_sqrt(self) -> Option<Self> {
        if self.pi != 0 || self.num < 0 {
            return None;
        }
        if self.is_zero() {
            return Some(Self::zero());
        }
        let n = (self.num as u64).checked_mul(self.rad)?;
        let (num_square, num_rest) = split_square(n);
        let (den_square, den_rest) = split_square(self.den);
        if den_rest != 1 {
            // sqrt(1/d) with d not square: write as sqrt(d)/d.
            let den = self.den.checked_mul(den_rest)?;
            let rad = num_rest.checked_mul(den_rest)?;
            return Some(
                Self {
                    num: i64::try_from(num_square).ok()?,
                    den,
                    pi: 0,
                    rad,
                }
                .normalized(),
            );
        }
        Some(
            Self {
                num: i64::try_from(num_square).ok()?,
                den: den_square,
                pi: 0,
                rad: num_rest,
            }
            .normalized(),
        )
    }
}

fn reduce(num: i64, den: u64) -> (i64, u64) {
    if num == 0 {
        return (0, 1);
    }
    if den == 0 {
        return (num, 1);
    }
    let g = gcd_u64(num.unsigned_abs(), den);
    (num / g as i64, den / g)
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let rest = left % right;
        left = right;
        right = rest;
    }
    left
}

fn lcm_u64(left: u64, right: u64) -> Option<u64> {
    if left == 0 || right == 0 {
        return Some(0);
    }
    let g = gcd_u64(left, right);
    left.checked_div(g)?.checked_mul(right)
}

fn split_square(mut value: u64) -> (u64, u64) {
    if value == 0 {
        return (0, 1);
    }
    let mut square = 1u64;
    let mut rest = 1u64;
    let mut p = 2u64;
    while p.saturating_mul(p) <= value {
        let mut exp = 0u32;
        while value.is_multiple_of(p) {
            value /= p;
            exp += 1;
        }
        for _ in 0..(exp / 2) {
            square = square.saturating_mul(p);
        }
        if exp % 2 == 1 {
            rest = rest.saturating_mul(p);
        }
        p += 1;
    }
    if value > 1 {
        rest = rest.saturating_mul(value);
    }
    (square, rest)
}

#[cfg(test)]
mod tests {
    use super::{
        HarmonicPartial, OSCILLATOR_TONE_REFERENCE_HZ, OSCILLATOR_TONE_SECONDS, PathClosure,
        PeriodicClosure,
    };
    use crate::studio::StudioCreation;

    fn bundled(id: &str) -> StudioCreation {
        StudioCreation::from_capsule(id).expect("bundled experiment")
    }

    fn periodic(id: &str) -> PeriodicClosure {
        match PathClosure::of(&bundled(id)) {
            PathClosure::Periodic(periodic) => periodic,
            other => panic!("{id} should be periodic, got {other:?}"),
        }
    }

    #[test]
    fn a_graph_is_not_a_path_that_comes_home() {
        let graph = StudioCreation::new("sin(x)", -1.0, 1.0, 1.0).expect("graph");
        assert_eq!(PathClosure::of(&graph), PathClosure::Graph);
        assert!(PathClosure::of(&graph).report_lines().is_empty());
        assert_eq!(PathClosure::of(&graph).status_caption(), None);
    }

    #[test]
    fn full_return_has_period_twelve_with_seventeen_y_cycles() {
        let closure = periodic("full-return");
        assert_eq!(closure.period_text, "12");
        assert_eq!(closure.x_frequency_text, "1");
        assert_eq!(closure.y_frequency_text, "17/12");
        assert_eq!(closure.x_cycles, Some(12));
        assert_eq!(closure.y_cycles, Some(17));
        assert_eq!(closure.window_periods, Some(1));
        assert!(closure.window_end.state_returns);
        assert!(closure.window_end.position_returns);
    }

    #[test]
    fn same_place_half_period_is_the_deceptive_return() {
        let closure = periodic("same-place");
        assert_eq!(closure.period_text, "1");
        assert_eq!(closure.x_frequency_text, "2");
        assert_eq!(closure.y_frequency_text, "3");
        assert_eq!(closure.x_cycles, Some(2));
        assert_eq!(closure.y_cycles, Some(3));
        assert_eq!(closure.half_period.t, 0.5);
        assert!(
            closure.half_period.position_returns,
            "position at t=0.5 matches t=0"
        );
        assert!(
            !closure.half_period.state_returns,
            "velocity reverses at t=0.5"
        );
        assert!(closure.window_end.state_returns);
    }

    #[test]
    fn almost_home_has_no_period() {
        match PathClosure::of(&bundled("almost-home")) {
            PathClosure::Aperiodic(aperiodic) => {
                assert_eq!(aperiodic.x_frequency_text, "1");
                assert_eq!(aperiodic.y_frequency_text, "sqrt(2)");
                assert!(!aperiodic.window_end.state_returns);
            }
            other => panic!("almost-home should be aperiodic, got {other:?}"),
        }
    }

    #[test]
    fn an_unseen_eight_fifths_ratio_has_period_five() {
        let creation =
            StudioCreation::new_parametric("cos(2*pi*t)", "sin(2*pi*(8/5)*t)", 0.0, 5.0, 1.0)
                .expect("unseen ratio");
        match PathClosure::of(&creation) {
            PathClosure::Periodic(periodic) => {
                assert_eq!(periodic.period_text, "5");
                assert_eq!(periodic.x_frequency_text, "1");
                assert_eq!(periodic.y_frequency_text, "8/5");
                assert_eq!(periodic.x_cycles, Some(5));
                assert_eq!(periodic.y_cycles, Some(8));
                assert_eq!(periodic.window_periods, Some(1));
                assert!(periodic.window_end.state_returns);
            }
            other => panic!("8/5 should be periodic with T=5, got {other:?}"),
        }
    }

    #[test]
    fn save_and_reopen_preserves_full_return_closure() {
        let original = bundled("full-return");
        let reopened = StudioCreation::from_capsule(&original.to_num_file()).expect("reopen");
        assert_eq!(PathClosure::of(&original), PathClosure::of(&reopened));
        let from_link = StudioCreation::from_capsule(&original.to_link()).expect("link");
        assert_eq!(PathClosure::of(&original), PathClosure::of(&from_link));
    }

    #[test]
    fn status_captions_name_the_trial_without_a_lecture() {
        assert_eq!(
            PathClosure::of(&bundled("full-return")).status_caption(),
            Some("PERIOD 12  HALF: PLACE NOT STATE".to_string())
        );
        assert_eq!(
            PathClosure::of(&bundled("same-place")).status_caption(),
            Some("PERIOD 1  HALF: PLACE NOT STATE".to_string())
        );
        assert_eq!(
            PathClosure::of(&bundled("almost-home")).status_caption(),
            Some("NO PERIOD".to_string())
        );
        assert_eq!(
            PathClosure::of(&bundled("another-ratio")).status_caption(),
            Some("PERIOD 1".to_string())
        );
        let unseen =
            StudioCreation::new_parametric("cos(2*pi*t)", "sin(2*pi*(8/5)*t)", 0.0, 5.0, 1.0)
                .expect("unseen ratio");
        assert_eq!(
            PathClosure::of(&unseen).status_caption(),
            Some("PERIOD 5".to_string())
        );
    }

    #[test]
    fn a_fractional_least_period_is_not_truncated_to_zero() {
        let creation =
            StudioCreation::new_parametric("cos(2*pi*2*t)", "sin(2*pi*4*t)", 0.0, 1.0, 1.0)
                .expect("double frequency");
        match PathClosure::of(&creation) {
            PathClosure::Periodic(periodic) => {
                assert_eq!(periodic.period_text, "1/2");
                assert_eq!(periodic.x_cycles, Some(1));
                assert_eq!(periodic.y_cycles, Some(2));
                assert_eq!(periodic.window_periods, Some(2));
                assert!(periodic.window_end.state_returns);
            }
            other => panic!("2 and 4 should have period 1/2, got {other:?}"),
        }
    }

    #[test]
    fn commensurate_square_roots_have_a_period() {
        let creation = StudioCreation::new_parametric(
            "cos(2*pi*sqrt(2)*t)",
            "sin(2*pi*2*sqrt(2)*t)",
            0.0,
            1.0,
            1.0,
        )
        .expect("shared radical");
        match PathClosure::of(&creation) {
            PathClosure::Periodic(periodic) => {
                assert_eq!(periodic.period_text, "sqrt(2)/2");
                assert_eq!(periodic.x_frequency_text, "sqrt(2)");
                assert_eq!(periodic.y_frequency_text, "2*sqrt(2)");
                assert_eq!(periodic.x_cycles, Some(1));
                assert_eq!(periodic.y_cycles, Some(2));
                assert_eq!(periodic.window_periods, None);
                assert!(
                    !periodic.window_end.state_returns,
                    "a window of length 1 is not the irrational period"
                );
            }
            other => panic!("shared sqrt(2) should be periodic, got {other:?}"),
        }
    }

    fn voices(id: &str) -> super::VoiceClosure {
        match PathClosure::of(&bundled(id)) {
            PathClosure::Voices(voices) => voices,
            other => panic!("{id} should be an oscillator overlay, got {other:?}"),
        }
    }

    #[test]
    fn closing_voices_count_twelve_and_seventeen_cycles() {
        let voices = voices("closing-voices");
        assert_eq!(voices.voices[0].frequency_text, "1");
        assert_eq!(voices.voices[0].window_cycles.as_deref(), Some("12"));
        assert_eq!(voices.voices[1].frequency_text, "17/12");
        assert_eq!(voices.voices[1].window_cycles.as_deref(), Some("17"));
        assert_eq!(voices.period_text.as_deref(), Some("12"));
        assert_eq!(voices.window_periods, Some(1));
        assert_eq!(voices.window_is_common_period, Some(true));
        assert_eq!(
            PathClosure::of(&bundled("closing-voices")).status_caption(),
            Some("CYCLES 12 17  PERIOD 12".to_string())
        );
    }

    #[test]
    fn a_shorter_window_keeps_the_period_and_refuses_the_counts() {
        let voices = voices("shorter-window");
        assert_eq!(voices.voices[0].window_cycles.as_deref(), Some("1"));
        assert_eq!(voices.voices[1].window_cycles.as_deref(), Some("17/12"));
        assert_eq!(voices.period_text.as_deref(), Some("12"));
        assert_eq!(voices.window_periods, None);
        assert_eq!(voices.window_is_common_period, Some(false));
        assert_eq!(
            PathClosure::of(&bundled("shorter-window")).status_caption(),
            Some("CYCLES 1 17/12  PERIOD 12  NOT THIS WINDOW".to_string())
        );
    }

    #[test]
    fn wandering_voices_name_the_exact_product_and_no_period() {
        let voices = voices("wandering-voices");
        assert_eq!(voices.voices[0].window_cycles.as_deref(), Some("12"));
        assert_eq!(voices.voices[1].frequency_text, "sqrt(2)");
        assert_eq!(
            voices.voices[1].window_cycles.as_deref(),
            Some("12*sqrt(2)")
        );
        assert!(voices.period_text.is_none());
        assert_eq!(voices.window_is_common_period, Some(false));
        let lines = PathClosure::of(&bundled("wandering-voices")).report_lines();
        assert!(
            lines
                .iter()
                .any(|line| line == "ideal motion has no positive common period"),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("16.97") && !line.contains("irrational")),
            "{lines:?}"
        );
        assert_eq!(
            PathClosure::of(&bundled("wandering-voices")).status_caption(),
            Some("CYCLES 12 12*sqrt(2)  NO PERIOD".to_string())
        );
    }

    #[test]
    fn an_unseen_overlay_ratio_keeps_its_period_when_the_window_changes() {
        let on_period =
            StudioCreation::new_program(["cos(2*pi*x)", "sin(2*pi*(8/5)*x)"], 0.0, 5.0, 1.0)
                .expect("unseen overlay");
        match PathClosure::of(&on_period) {
            PathClosure::Voices(voices) => {
                assert_eq!(voices.period_text.as_deref(), Some("5"));
                assert_eq!(voices.voices[0].window_cycles.as_deref(), Some("5"));
                assert_eq!(voices.voices[1].window_cycles.as_deref(), Some("8"));
                assert_eq!(voices.window_periods, Some(1));
                assert_eq!(voices.window_is_common_period, Some(true));
            }
            other => panic!("8/5 overlay should have period 5, got {other:?}"),
        }
        let longer =
            StudioCreation::new_program(["cos(2*pi*x)", "sin(2*pi*(8/5)*x)"], 0.0, 12.0, 1.0)
                .expect("longer window");
        match PathClosure::of(&longer) {
            PathClosure::Voices(voices) => {
                assert_eq!(voices.period_text.as_deref(), Some("5"));
                assert_eq!(voices.voices[0].window_cycles.as_deref(), Some("12"));
                assert_eq!(voices.voices[1].window_cycles.as_deref(), Some("96/5"));
                assert_eq!(voices.window_is_common_period, Some(false));
            }
            other => panic!("a longer window does not change the period, got {other:?}"),
        }
        let reopened = StudioCreation::from_capsule(&on_period.to_num_file()).expect("reopen");
        assert_eq!(PathClosure::of(&on_period), PathClosure::of(&reopened));
        let from_link = StudioCreation::from_capsule(&on_period.to_link()).expect("link");
        assert_eq!(PathClosure::of(&on_period), PathClosure::of(&from_link));
    }

    #[test]
    fn two_windows_of_closing_voices_do_not_rename_the_period() {
        let wide =
            StudioCreation::new_program(["cos(2*pi*x)", "sin(2*pi*(17/12)*x)"], 0.0, 24.0, 1.0)
                .expect("two periods");
        match PathClosure::of(&wide) {
            PathClosure::Voices(voices) => {
                assert_eq!(voices.period_text.as_deref(), Some("12"));
                assert_eq!(voices.voices[0].window_cycles.as_deref(), Some("24"));
                assert_eq!(voices.voices[1].window_cycles.as_deref(), Some("34"));
                assert_eq!(voices.window_periods, Some(2));
                assert_eq!(voices.window_is_common_period, Some(true));
                assert_eq!(
                    PathClosure::of(&wide).status_caption().as_deref(),
                    Some("CYCLES 24 34  PERIOD 12")
                );
            }
            other => panic!("two windows still have period 12, got {other:?}"),
        }
    }

    #[test]
    fn an_overlay_fractional_period_matches_the_path() {
        let overlay = StudioCreation::new_program(["cos(4*pi*x)", "sin(8*pi*x)"], 0.0, 1.0, 1.0)
            .expect("overlay");
        let path = StudioCreation::new_parametric("cos(4*pi*t)", "sin(8*pi*t)", 0.0, 1.0, 1.0)
            .expect("path");
        match (PathClosure::of(&overlay), PathClosure::of(&path)) {
            (PathClosure::Voices(voices), PathClosure::Periodic(periodic)) => {
                assert_eq!(voices.period_text.as_deref(), Some("1/2"));
                assert_eq!(periodic.period_text, "1/2");
                assert_eq!(voices.window_periods, Some(2));
                assert_eq!(periodic.window_periods, Some(2));
            }
            other => panic!("overlay and path should agree on period 1/2, got {other:?}"),
        }
    }

    #[test]
    fn equal_radicals_are_not_called_aperiodic() {
        let creation = StudioCreation::new_program(
            ["cos(2*pi*sqrt(2)*x)", "sin(2*pi*sqrt(2)*x)"],
            0.0,
            1.0,
            1.0,
        )
        .expect("equal radicals");
        match PathClosure::of(&creation) {
            PathClosure::Voices(voices) => {
                assert_eq!(voices.period_text.as_deref(), Some("sqrt(2)/2"));
                assert_ne!(
                    PathClosure::of(&creation).status_caption().as_deref(),
                    Some("NO PERIOD")
                );
            }
            other => panic!("equal sqrt(2) voices share a period, got {other:?}"),
        }
    }

    #[test]
    fn an_ordinary_overlay_does_not_invent_a_voice_trial() {
        for id in ["the-parts", "the-sum", "tresillo", "full-return"] {
            assert!(
                matches!(
                    PathClosure::of(&bundled(id)),
                    PathClosure::Graph | PathClosure::Periodic(_)
                ),
                "{id} should not become a voice trial"
            );
            assert!(
                !PathClosure::of(&bundled(id))
                    .report_lines()
                    .iter()
                    .any(|line| line.contains("closure=voices")),
                "{id}"
            );
        }
        let mixed = StudioCreation::new_program(["sin(x)", "cos(2*pi*x)"], 0.0, 1.0, 1.0)
            .expect("mixed overlay");
        assert!(matches!(PathClosure::of(&mixed), PathClosure::Graph));
        let three = StudioCreation::new_program(
            ["cos(2*pi*x)", "sin(2*pi*x)", "cos(4*pi*x)"],
            0.0,
            1.0,
            1.0,
        )
        .expect("three oscillators");
        assert!(matches!(PathClosure::of(&three), PathClosure::Graph));
        let lone = StudioCreation::new("cos(2*pi*x)", 0.0, 1.0, 1.0).expect("one graph");
        assert!(matches!(PathClosure::of(&lone), PathClosure::Graph));
    }

    fn tone_ratio(tones: &super::OscillatorTones) -> f32 {
        let left = tones.voices[0].hz.expect("left hz");
        let right = tones.voices[1].hz.expect("right hz");
        right / left
    }

    #[test]
    fn named_frequencies_sound_without_replacing_the_melody_or_the_capsule() {
        let full = bundled("full-return");
        let capsule = full.to_num_file();
        let closure = PathClosure::of(&full);
        let tones = closure.oscillator_tones().expect("full-return tones");
        assert_eq!(full.to_num_file(), capsule);
        assert_eq!(tones.reference_hz, super::OSCILLATOR_TONE_REFERENCE_HZ);
        assert_eq!(tones.voices[0].frequency_text, "1");
        assert_eq!(tones.voices[0].hz, Some(110.0));
        assert_eq!(tones.voices[1].frequency_text, "17/12");
        assert!((tone_ratio(&tones) - 17.0 / 12.0).abs() < 1e-5);
        let sound = tones.sound().expect("full-return sound");
        assert_eq!(sound.notes.len(), 2);
        assert_eq!(sound.notes[0].freq, tones.voices[0].hz.expect("root"));
        assert_eq!(sound.notes[1].freq, tones.voices[1].hz.expect("upper"));
        assert!(
            sound
                .notes
                .iter()
                .all(|note| note.start == 0.0 && note.dur == sound.duration)
        );
        let melody = full.to_melody(32);
        assert!(melody.notes.len() > 2);
        assert_ne!(melody, sound);
        let upper_only =
            crate::sound::SoundSpec::tone(sound.notes[1].freq, sound.duration, sound.notes[1].amp);
        assert_eq!(
            sound.midi(),
            upper_only.midi(),
            "two tones that start together are not a MIDI chord"
        );
        assert_ne!(melody.midi(), sound.midi());
        let report = tones.report_lines().join("\n");
        assert!(report.contains("reference_hz=110"), "{report}");
        assert!(report.contains("freq=17/12"), "{report}");
        assert!(!report.contains("period"), "{report}");
        assert!(
            closure
                .report_lines()
                .iter()
                .all(|line| !line.contains("hz=")),
            "the period report does not become a pitch"
        );

        let almost = PathClosure::of(&bundled("almost-home"))
            .oscillator_tones()
            .expect("almost-home tones");
        assert_eq!(almost.voices[0].frequency_text, "1");
        assert_eq!(almost.voices[1].frequency_text, "sqrt(2)");
        let almost_ratio = tone_ratio(&almost);
        assert!((almost_ratio - 2f32.sqrt()).abs() < 1e-5);
        assert!((almost_ratio - 17.0 / 12.0).abs() > 1e-3);
        assert!((almost_ratio - 7.0 / 5.0).abs() > 1e-2);
        let full_upper = tones.voices[1].hz.expect("full upper");
        let almost_upper = almost.voices[1].hz.expect("almost upper");
        assert!((full_upper - almost_upper).abs() > 0.2);

        let same = PathClosure::of(&bundled("same-place"));
        assert!(
            same.status_caption()
                .expect("caption")
                .contains("HALF: PLACE NOT STATE")
        );
        let same_tones = same.oscillator_tones().expect("same-place tones");
        assert_eq!(same_tones.voices[0].frequency_text, "2");
        assert_eq!(same_tones.voices[1].frequency_text, "3");

        let closing = PathClosure::of(&bundled("closing-voices"))
            .oscillator_tones()
            .expect("closing tones");
        let shorter = PathClosure::of(&bundled("shorter-window"))
            .oscillator_tones()
            .expect("shorter tones");
        assert_eq!(closing.voices[0].frequency_text, "1");
        assert_eq!(closing.voices[1].frequency_text, "17/12");
        assert_eq!(closing.voices[1].hz, shorter.voices[1].hz);

        let wandering = PathClosure::of(&bundled("wandering-voices"))
            .oscillator_tones()
            .expect("wandering tones");
        assert_eq!(wandering.voices[1].frequency_text, "sqrt(2)");
        assert!((tone_ratio(&wandering) - 2f32.sqrt()).abs() < 1e-5);

        assert!(
            PathClosure::of(&bundled("simple-zero"))
                .oscillator_tones()
                .is_none()
        );
        let graph = StudioCreation::new("sin(x)", -1.0, 1.0, 1.0).expect("graph");
        assert!(PathClosure::of(&graph).oscillator_tones().is_none());
        let plain = StudioCreation::new_parametric("cos(t)", "sin(2*t)", 0.0, 1.0, 1.0)
            .expect("plain trig");
        assert!(PathClosure::of(&plain).oscillator_tones().is_none());
        let sum =
            StudioCreation::new_parametric("cos(2*pi*t)+cos(6*pi*t)", "sin(2*pi*t)", 0.0, 1.0, 1.0)
                .expect("sum");
        assert!(PathClosure::of(&sum).oscillator_tones().is_none());
    }

    #[test]
    fn a_two_term_path_names_its_first_partial_and_one_tone_per_frequency() {
        let x = "cos(2*pi*t)+0.5*cos(6*pi*t)";
        let y = "sin(2*pi*t)+0.5*sin(6*pi*t)";
        let creation = StudioCreation::new_parametric(x, y, 0.0, 1.0, 1.0).expect("epicycle");
        let file = creation.to_num_file();
        let partial = HarmonicPartial::of(&creation).expect("partial");
        assert_eq!(creation.to_num_file(), file);
        assert!(matches!(
            PathClosure::of(&creation),
            PathClosure::Unsupported
        ));
        assert!(PathClosure::of(&creation).oscillator_tones().is_none());
        assert_eq!(partial.status_caption(), "PARTIAL  1  3");
        assert_eq!(
            partial.report_lines(),
            vec![
                "partial basis=sum".to_string(),
                "term 1 freq=1 hz=110".to_string(),
                "term 2 freq=3 hz=330".to_string(),
            ]
        );
        let sound = partial.sound().expect("tones");
        assert_eq!(sound.duration, OSCILLATOR_TONE_SECONDS);
        assert_eq!(sound.notes.len(), 2);
        assert_eq!(sound.notes[0].freq, 110.0);
        assert_eq!(sound.notes[1].freq, 330.0);
        assert_ne!(sound, creation.to_melody(32));

        let program = creation.program().expect("program");
        for t in [0.0, 0.25, 0.5, 0.8] {
            let point = program.point(t, 1.0).expect("path");
            let x_sum = crate::studio::eval(&partial.x_terms[0], t, 1.0)
                + crate::studio::eval(&partial.x_terms[1], t, 1.0);
            let y_sum = crate::studio::eval(&partial.y_terms[0], t, 1.0)
                + crate::studio::eval(&partial.y_terms[1], t, 1.0);
            assert!((point.0 - x_sum).abs() < 1e-9, "t={t}");
            assert!((point.1 - y_sum).abs() < 1e-9, "t={t}");
            let first = partial.first_point(t, 1.0, &[]).expect("first term");
            assert!((first.0 - t.mul_add(std::f64::consts::TAU, 0.0).cos()).abs() < 1e-9);
            assert!((first.1 - t.mul_add(std::f64::consts::TAU, 0.0).sin()).abs() < 1e-9);
        }

        let subtracted = StudioCreation::new_parametric(
            "cos(2*pi*t)-0.5*cos(6*pi*t)",
            "sin(2*pi*t)-0.5*sin(6*pi*t)",
            0.0,
            1.0,
            1.0,
        )
        .expect("difference");
        let difference = HarmonicPartial::of(&subtracted).expect("signed partial");
        assert_eq!(difference.frequencies.len(), 2);
        assert_eq!(difference.frequencies[1].frequency_text, "3");
        let program = subtracted.program().expect("program");
        let point = program.point(0.2, 1.0).expect("path");
        let x_sum = crate::studio::eval(&difference.x_terms[0], 0.2, 1.0)
            + crate::studio::eval(&difference.x_terms[1], 0.2, 1.0);
        let y_sum = crate::studio::eval(&difference.y_terms[0], 0.2, 1.0)
            + crate::studio::eval(&difference.y_terms[1], 0.2, 1.0);
        assert!((point.0 - x_sum).abs() < 1e-9);
        assert!((point.1 - y_sum).abs() < 1e-9);

        let irrational = StudioCreation::new_parametric(
            "cos(2*pi*t)+cos(2*pi*sqrt(2)*t)",
            "sin(2*pi*t)+sin(2*pi*sqrt(2)*t)",
            0.0,
            1.0,
            1.0,
        )
        .expect("irrational");
        let irrational = HarmonicPartial::of(&irrational).expect("sqrt partial");
        assert_eq!(irrational.frequencies[0].frequency_text, "1");
        assert_eq!(irrational.frequencies[1].frequency_text, "sqrt(2)");
        let hz = irrational.frequencies[1].hz.expect("hz");
        assert!(
            (f64::from(hz) - f64::from(OSCILLATOR_TONE_REFERENCE_HZ) * 2f64.sqrt()).abs() < 1e-3
        );
        assert!((f64::from(hz) - 110.0 * 17.0 / 12.0).abs() > 0.2);

        for (x_source, y_source) in [
            ("cos(2*pi*t)", "sin(2*pi*t)"),
            ("cos(2*pi*t)+cos(6*pi*t)", "sin(2*pi*t)"),
            (
                "cos(2*pi*t)+cos(6*pi*t)+cos(10*pi*t)",
                "sin(2*pi*t)+sin(6*pi*t)+sin(10*pi*t)",
            ),
            ("cos(t)+cos(3*t)", "sin(t)+sin(3*t)"),
            ("cos(2*pi*t)+t", "sin(2*pi*t)+t"),
        ] {
            let creation =
                StudioCreation::new_parametric(x_source, y_source, 0.0, 1.0, 1.0).expect(x_source);
            assert!(HarmonicPartial::of(&creation).is_none(), "{x_source}");
        }
        let graph = StudioCreation::new("sin(2*pi*x)+sin(6*pi*x)", 0.0, 1.0, 1.0).expect("graph");
        assert!(HarmonicPartial::of(&graph).is_none());
        assert!(HarmonicPartial::of(&bundled("full-return")).is_none());
        let inexact = StudioCreation::new_parametric(x, y, 0.0, 1.0, 0.1).expect("knob");
        assert!(HarmonicPartial::of(&inexact).is_none());
    }
}
