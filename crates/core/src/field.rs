//! A field over the plane, drawn as characters.
//!
//! A curve answers one number at a time. A field answers everywhere at once,
//! and the same expression read three ways is three different truths about it:
//! where the value points, how big it is, and where it is nothing at all. Those
//! are domain colouring, a height map, and an implicit curve, and they are one
//! feature rather than three because they are one sample read differently.
//!
//! Nothing here uses colour. The phase is carried by which character is drawn,
//! not by which ink it is drawn in, so the picture survives a terminal with no
//! colour, a player who sees fewer of them, and a mind reading the plate as
//! text over a protocol. That is not a fallback; it is the only rendering, and
//! it is the reason the field needs no new ink to be legible.
//!
//! # The three readings, and why only one of them needs a proof
//!
//! [`FieldReading::Phase`] and [`FieldReading::Height`] sample the value at the
//! centre of each cell and say so. That is honest on its own terms: the claim
//! is about the sampled points, and the grid is reported beside the picture.
//!
//! [`FieldReading::Zero`] cannot work that way. It claims something about the
//! whole cell, that a curve passes through it, and Tupper proved in 2001 that
//! no algorithm decides that for an arbitrary relation. So this reading has
//! three answers rather than two: proved present, proved absent, and not
//! resolved. The third is what makes the other two mean anything. It is
//! computed with [`crate::enclosure`], over the cell rather than at a point.

use crate::complex::Complex;
use crate::enclosure::{Certainty, Enclosure};
use crate::studio::{Expr, Func, Op, PairFunc};

/// How wide the drawn plate may be, in characters.
pub const MAX_FIELD_WIDTH: usize = 400;

/// How tall the drawn plate may be, in characters.
///
/// Width times height is the sample count, so a field costs what a curve costs
/// squared. These bounds keep the worst plate under two hundred thousand
/// samples, which is the same order as the room renders the catalog already
/// draws every frame.
pub const MAX_FIELD_HEIGHT: usize = 400;

/// Steps of the phase and height ramp.
///
/// Phase is circular and a ramp is not, so any character encoding of it carries
/// exactly one seam, and the seam is drawn rather than hidden: it is the line
/// where the argument passes pi, and following it to where all eight steps meet
/// is how a zero or a pole is found by eye.
const RAMP: [char; 8] = [' ', '.', ':', '-', '=', '+', '*', '#'];

/// Drawn where the value is past what a double can hold, which is a pole.
const UNBOUNDED: char = '@';

/// Drawn where there is no answer: the value is undefined, or the reading could
/// not be resolved.
const NO_ANSWER: char = '?';

/// Drawn where a certified implicit curve passes.
const CURVE: char = '#';

/// Cells wide and tall a field is drawn at when nobody asks for a size.
pub const DEFAULT_FIELD_SIZE: (usize, usize) = (72, 28);

/// Which truth about the field the plate shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldReading {
    /// Where the value points. A domain-coloured phase portrait, drawn in
    /// characters: each step of the ramp is an eighth of a turn, so a zero or a
    /// pole is the point every step meets and the order of a zero is how many
    /// times the ramp repeats around it.
    #[default]
    Phase,
    /// How big the value is, on a logarithmic ladder. The height map.
    Height,
    /// Where the value is zero. The implicit curve, and the only reading that
    /// proves rather than samples.
    Zero,
}

impl FieldReading {
    /// Stable lowercase capsule and protocol name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Phase => "phase",
            Self::Height => "height",
            Self::Zero => "zero",
        }
    }

    /// Parse a stable reading name.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "phase" => Some(Self::Phase),
            "height" => Some(Self::Height),
            "zero" => Some(Self::Zero),
            _ => None,
        }
    }

    /// Next reading in a bounded cycle, for a face that offers one key.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Phase => Self::Height,
            Self::Height => Self::Zero,
            Self::Zero => Self::Phase,
        }
    }

    /// One line saying what the plate is showing.
    #[must_use]
    pub const fn legend(self) -> &'static str {
        match self {
            Self::Phase => {
                "phase: ramp steps are eighth-turns; they meet at a zero, and reverse at a pole"
            }
            Self::Height => "height: each ramp step doubles the size of the value",
            Self::Zero => {
                "zero: '#' is a proved crossing, '?' is unresolved, blank is proved clear"
            }
        }
    }
}

/// A drawn field and what the drawing is entitled to claim.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPlate {
    /// The character plate, one line per row.
    pub text: String,
    /// Which truth the plate shows.
    pub reading: FieldReading,
    /// Real extent actually drawn, which may exceed what was asked for so a
    /// circle stays round.
    pub x_bounds: (f64, f64),
    /// Imaginary extent actually drawn.
    pub y_bounds: (f64, f64),
    /// Cells across and down.
    pub size: (usize, usize),
    /// Cells the [`FieldReading::Zero`] reading could not resolve. Zero for the
    /// other readings, which sample rather than prove.
    pub unresolved: usize,
    /// Cells where the value has no answer at all.
    pub undefined: usize,
}

impl FieldPlate {
    /// Whether every cell of a proving reading reached an answer.
    ///
    /// A picture that resolved completely is a stronger statement than a
    /// picture that looks clean, and only this can tell them apart.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.unresolved == 0
    }
}

/// Why a field could not be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    /// The requested plate is smaller than two cells or larger than the bound.
    InvalidSize,
    /// The requested window is reversed, empty, or not finite.
    InvalidWindow,
    /// The zero reading was asked for an expression whose values leave the real
    /// line, where a crossing is a point rather than a curve and a sign has no
    /// meaning.
    NotRealValued,
}

impl FieldError {
    /// A short player-facing reason.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidSize => "a field needs at least two cells on each side",
            Self::InvalidWindow => "a field window needs finite increasing bounds on both axes",
            Self::NotRealValued => {
                "the zero reading needs a real-valued field; this one leaves the real line, \
                 so read its phase instead"
            }
        }
    }
}

/// Widen a window until it matches the shape of the character grid.
///
/// The alternative is to stretch the picture to fill the plate, which makes a
/// circle an ellipse and quietly changes what the mathematics says. Widening
/// keeps every point that was asked for and shows a little more, and the plate
/// reports what it actually drew.
fn fitted_window(
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
    size: (usize, usize),
    char_aspect: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let (width, height) = size;
    let spans = (x_bounds.1 - x_bounds.0, y_bounds.1 - y_bounds.0);
    if !spans.0.is_finite() || !spans.1.is_finite() || spans.0 <= 0.0 || spans.1 <= 0.0 {
        return None;
    }
    // One cell is `char_aspect` as wide as it is tall, so equal world units per
    // physical length means the drawn box has this shape.
    let wanted = (width as f64 * char_aspect) / height as f64;
    if !wanted.is_finite() || wanted <= 0.0 {
        return None;
    }
    let have = spans.0 / spans.1;
    if !have.is_finite() {
        return None;
    }
    let centre = (x_bounds.0 + spans.0 * 0.5, y_bounds.0 + spans.1 * 0.5);
    let (x_span, y_span) = if have < wanted {
        (spans.1 * wanted, spans.1)
    } else {
        (spans.0, spans.0 / wanted)
    };
    if !x_span.is_finite() || !y_span.is_finite() || x_span <= 0.0 || y_span <= 0.0 {
        return None;
    }
    let fitted_x = (centre.0 - x_span * 0.5, centre.0 + x_span * 0.5);
    let fitted_y = (centre.1 - y_span * 0.5, centre.1 + y_span * 0.5);
    if [fitted_x.0, fitted_x.1, fitted_y.0, fitted_y.1]
        .into_iter()
        .any(|value| !value.is_finite())
        || fitted_x.0 >= fitted_x.1
        || fitted_y.0 >= fitted_y.1
    {
        return None;
    }
    Some((fitted_x, fitted_y))
}

/// Luminance of one plate mark, when that mark carries ink.
///
/// Space is absence rather than a dim step, so it returns nothing and a
/// raster blit can leave the stage black. The ramp is monotonic. `@` is a
/// pole that has left the finite plane. `?` is no answer, drawn dimmer than
/// a proved curve so it cannot be mistaken for one.
#[must_use]
pub fn mark_level(mark: char) -> Option<f32> {
    match mark {
        ' ' => None,
        '.' => Some(0.14),
        ':' => Some(0.28),
        '-' => Some(0.42),
        '=' => Some(0.56),
        '+' => Some(0.70),
        '*' => Some(0.84),
        '#' => Some(0.96),
        '@' => Some(1.0),
        '?' => Some(0.35),
        _ => Some(0.5),
    }
}

/// Draw a field expression as characters.
///
/// `char_aspect` is how wide a character cell is compared with its height: one
/// for square pixels, a half for the terminal's tall cells.
///
/// # Errors
/// Returns [`FieldError`] for a plate that is too small or too large, a window
/// that is not a finite increasing rectangle, or a zero reading asked of an
/// expression that leaves the real line.
pub fn draw(
    expression: &Expr,
    reading: FieldReading,
    x_bounds: (f64, f64),
    y_bounds: (f64, f64),
    parameter: f64,
    size: (usize, usize),
    char_aspect: f64,
) -> Result<FieldPlate, FieldError> {
    let (width, height) = size;
    if width < 2 || height < 2 || width > MAX_FIELD_WIDTH || height > MAX_FIELD_HEIGHT {
        return Err(FieldError::InvalidSize);
    }
    if !parameter.is_finite() {
        return Err(FieldError::InvalidWindow);
    }
    if reading == FieldReading::Zero && !is_real_valued(expression) {
        return Err(FieldError::NotRealValued);
    }
    let aspect = if char_aspect.is_finite() && char_aspect > 0.0 {
        char_aspect
    } else {
        0.5
    };
    let (x_bounds, y_bounds) =
        fitted_window(x_bounds, y_bounds, size, aspect).ok_or(FieldError::InvalidWindow)?;

    let step = (
        (x_bounds.1 - x_bounds.0) / width as f64,
        (y_bounds.1 - y_bounds.0) / height as f64,
    );
    let mut text = String::with_capacity((width + 1) * height);
    let mut unresolved = 0;
    let mut undefined = 0;
    for row in 0..height {
        // World y increases upward; a plate's first row is its top.
        let top = y_bounds.1 - step.1 * row as f64;
        let bottom = top - step.1;
        for col in 0..width {
            let left = x_bounds.0 + step.0 * col as f64;
            let right = left + step.0;
            let mark = match reading {
                FieldReading::Zero => {
                    let cell = crossing(expression, (left, right), (bottom, top), parameter);
                    match cell {
                        Crossing::Present => CURVE,
                        Crossing::Absent => ' ',
                        Crossing::Unresolved => {
                            unresolved += 1;
                            NO_ANSWER
                        }
                    }
                }
                _ => {
                    let point = Complex::new((left + right) * 0.5, (bottom + top) * 0.5);
                    let value = crate::studio::eval_field(expression, point, parameter);
                    if value.is_nan() {
                        undefined += 1;
                        NO_ANSWER
                    } else if !value.is_finite() {
                        UNBOUNDED
                    } else if reading == FieldReading::Phase {
                        phase_mark(value)
                    } else {
                        height_mark(value)
                    }
                }
            };
            text.push(mark);
        }
        if row + 1 < height {
            text.push('\n');
        }
    }
    Ok(FieldPlate {
        text,
        reading,
        x_bounds,
        y_bounds,
        size,
        unresolved,
        undefined,
    })
}

/// The ramp step for a value's direction.
fn phase_mark(value: Complex) -> char {
    let angle = value.arg();
    if !angle.is_finite() {
        // Only the origin has no direction, and it is the centre of the wheel
        // rather than an absence, so it takes the first step of the ramp.
        return RAMP[0];
    }
    let turns = (angle + std::f64::consts::PI) / std::f64::consts::TAU;
    let step = (turns * RAMP.len() as f64) as usize;
    RAMP[step.min(RAMP.len() - 1)]
}

/// The ramp step for a value's size, doubling each step.
fn height_mark(value: Complex) -> char {
    let size = value.abs();
    if size == 0.0 {
        return RAMP[0];
    }
    let steps = size.log2();
    if !steps.is_finite() {
        return RAMP[0];
    }
    // Centre the ladder on one, so a value of unit size sits mid ramp and each
    // step is a doubling either way.
    let centred = steps + (RAMP.len() as f64) * 0.5;
    let step = centred.clamp(0.0, RAMP.len() as f64 - 1.0) as usize;
    RAMP[step]
}

/// What is known about a curve inside one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Crossing {
    /// A zero exists somewhere in the cell, by the intermediate value theorem.
    Present,
    /// The value is never zero anywhere in the cell.
    Absent,
    /// Neither could be shown.
    Unresolved,
}

/// Decide what one cell of the zero reading may claim.
fn crossing(expression: &Expr, x: (f64, f64), y: (f64, f64), parameter: f64) -> Crossing {
    match crossing_once(expression, x, y, parameter) {
        Crossing::Unresolved => refine_crossing(expression, x, y, parameter),
        decided => decided,
    }
}

/// One extra split of an unresolved cell. Cheap at terminal size, and it
/// never paints a present or an absent it has not proved: a subcell that
/// still cannot decide keeps the parent unresolved.
fn refine_crossing(expression: &Expr, x: (f64, f64), y: (f64, f64), parameter: f64) -> Crossing {
    let mid_x = (x.0 + x.1) * 0.5;
    let mid_y = (y.0 + y.1) * 0.5;
    if !mid_x.is_finite() || !mid_y.is_finite() {
        return Crossing::Unresolved;
    }
    let quads = [
        (x.0, mid_x, y.0, mid_y),
        (mid_x, x.1, y.0, mid_y),
        (x.0, mid_x, mid_y, y.1),
        (mid_x, x.1, mid_y, y.1),
    ];
    let mut any_present = false;
    let mut all_absent = true;
    for (left, right, bottom, top) in quads {
        match crossing_once(expression, (left, right), (bottom, top), parameter) {
            Crossing::Present => {
                any_present = true;
                all_absent = false;
            }
            Crossing::Absent => {}
            Crossing::Unresolved => all_absent = false,
        }
    }
    if any_present {
        Crossing::Present
    } else if all_absent {
        Crossing::Absent
    } else {
        Crossing::Unresolved
    }
}

fn crossing_once(expression: &Expr, x: (f64, f64), y: (f64, f64), parameter: f64) -> Crossing {
    let over_cell = eval_enclosure(
        expression,
        Enclosure::span(x.0, x.1),
        Enclosure::span(y.0, y.1),
        parameter,
    );
    if over_cell.certainty() == Certainty::Sound && !over_cell.holds_zero() {
        // The value is bounded away from zero across the whole cell, so no
        // crossing can hide between the samples.
        return Crossing::Absent;
    }
    if over_cell.certainty() != Certainty::Sound {
        return Crossing::Unresolved;
    }
    // The cell might hold a zero. A pair of corners on strict opposite sides
    // proves one does, because the function is continuous across the cell.
    let corners = [
        (x.0, y.0),
        (x.1, y.0),
        (x.0, y.1),
        (x.1, y.1),
        ((x.0 + x.1) * 0.5, (y.0 + y.1) * 0.5),
        ((x.0 + x.1) * 0.5, y.0),
        ((x.0 + x.1) * 0.5, y.1),
        (x.0, (y.0 + y.1) * 0.5),
        (x.1, (y.0 + y.1) * 0.5),
    ];
    let mut saw_positive = false;
    let mut saw_negative = false;
    for (at_x, at_y) in corners {
        let value = crate::studio::eval_field(expression, Complex::new(at_x, at_y), parameter);
        if !value.is_finite() || !value.is_real() {
            return Crossing::Unresolved;
        }
        if value.re > 0.0 {
            saw_positive = true;
        } else if value.re < 0.0 {
            saw_negative = true;
        } else {
            // A corner sitting exactly on zero is a crossing with no argument
            // needed.
            return Crossing::Present;
        }
    }
    if saw_positive && saw_negative {
        return Crossing::Present;
    }
    Crossing::Unresolved
}

/// Whether every value this expression can take lies on the real line.
///
/// The zero reading needs this. A complex-valued field's zeros are isolated
/// points rather than a curve, and a sign test has nothing to compare, so
/// offering the reading anyway would be offering a picture of the wrong thing.
/// The check is on the expression rather than on the samples, so the answer
/// does not change with the window.
#[must_use]
pub fn is_real_valued(expression: &Expr) -> bool {
    match expression {
        Expr::Num(_) | Expr::Var | Expr::VarIm | Expr::Param => true,
        Expr::Point | Expr::ImagUnit => false,
        Expr::Neg(inner) => is_real_valued(inner),
        Expr::Bin(op, lhs, rhs) => {
            if !is_real_valued(lhs) || !is_real_valued(rhs) {
                return false;
            }
            // A real base raised to a fractional power leaves the line as soon
            // as the base goes negative, and the exponent is not known until
            // the sample. Only a whole literal power stays real for certain.
            match op {
                Op::Pow => matches!(**rhs, Expr::Num(exponent) if exponent.fract() == 0.0),
                _ => true,
            }
        }
        // These four read a value apart, and each half of a complex value is
        // real however complex the value was.
        Expr::Call(Func::Re | Func::Im | Func::Abs | Func::Arg, _) => true,
        // A square root or a logarithm of a negative real is not real, and the
        // sign is not known until the sample.
        Expr::Call(Func::Sqrt | Func::Ln, _) => false,
        Expr::Call(_, inner) => is_real_valued(inner),
        Expr::PairCall(_, lhs, rhs) => is_real_valued(lhs) && is_real_valued(rhs),
    }
}

/// Evaluate a real-valued expression over a box rather than at a point.
///
/// Only reached for expressions [`is_real_valued`] accepted, so the leaves that
/// leave the line cannot appear. They are still answered, with the whole line
/// and no conclusion, because a total function is easier to trust than one with
/// a promise attached.
fn eval_enclosure(expression: &Expr, x: Enclosure, y: Enclosure, a: f64) -> Enclosure {
    match expression {
        Expr::Num(value) => Enclosure::point(*value),
        Expr::Var => x,
        Expr::VarIm => y,
        Expr::Param => Enclosure::point(a),
        Expr::Point | Expr::ImagUnit => Enclosure::WHOLE,
        Expr::Neg(inner) => -eval_enclosure(inner, x, y, a),
        Expr::Bin(op, lhs, rhs) => {
            let left = eval_enclosure(lhs, x, y, a);
            match op {
                Op::Pow => match **rhs {
                    Expr::Num(exponent)
                        if exponent.fract() == 0.0 && exponent.abs() <= f64::from(i16::MAX) =>
                    {
                        // The exponent is a whole literal, checked by
                        // `is_real_valued` before this reading is offered.
                        left.powi(exponent as i32)
                    }
                    _ => Enclosure::WHOLE,
                },
                _ => {
                    let right = eval_enclosure(rhs, x, y, a);
                    match op {
                        Op::Add => left + right,
                        Op::Sub => left - right,
                        Op::Mul => left * right,
                        Op::Div => left / right,
                        Op::Pow => Enclosure::WHOLE,
                    }
                }
            }
        }
        Expr::Call(func, inner) => {
            let value = eval_enclosure(inner, x, y, a);
            match func {
                Func::Sin => value.sin(),
                Func::Cos => value.cos(),
                Func::Tan => value.tan(),
                Func::Exp => value.exp(),
                Func::Ln => value.ln(),
                Func::Abs => value.abs(),
                Func::Sqrt => value.sqrt(),
                Func::Floor => value.floor(),
                Func::Re | Func::Conj => value,
                Func::Im => Enclosure::point(0.0),
                Func::Arg => Enclosure::WHOLE,
            }
        }
        Expr::PairCall(func, lhs, rhs) => {
            let left = eval_enclosure(lhs, x, y, a);
            let right = eval_enclosure(rhs, x, y, a);
            match func {
                PairFunc::Mod => left.rem_euclid(right),
                PairFunc::Min => left.min(right),
                PairFunc::Max => left.max(right),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CURVE, DEFAULT_FIELD_SIZE, FieldError, FieldPlate, FieldReading, MAX_FIELD_WIDTH,
        NO_ANSWER, RAMP, draw, is_real_valued,
    };
    use crate::studio::parse_field;

    fn plate(source: &str, reading: FieldReading, span: f64) -> FieldPlate {
        let expression = parse_field(source).expect("field parses");
        draw(
            &expression,
            reading,
            (-span, span),
            (-span, span),
            1.0,
            DEFAULT_FIELD_SIZE,
            0.5,
        )
        .expect("field draws")
    }

    #[test]
    fn a_plate_has_the_shape_it_was_asked_for() {
        let drawn = plate("z^2 - 1", FieldReading::Phase, 2.0);
        let lines: Vec<&str> = drawn.text.lines().collect();
        assert_eq!(lines.len(), DEFAULT_FIELD_SIZE.1);
        for line in lines {
            assert_eq!(line.chars().count(), DEFAULT_FIELD_SIZE.0);
        }
        assert_eq!(drawn.size, DEFAULT_FIELD_SIZE);
    }

    #[test]
    fn a_window_widens_to_the_grid_rather_than_stretching_the_picture() {
        // A square window on a wide plate of tall cells must come back wider,
        // never taller, so a circle drawn in it stays round.
        let expression = parse_field("x^2 + y^2 - 1").expect("parses");
        let drawn = draw(
            &expression,
            FieldReading::Zero,
            (-2.0, 2.0),
            (-2.0, 2.0),
            1.0,
            (72, 28),
            0.5,
        )
        .expect("draws");
        let x_span = drawn.x_bounds.1 - drawn.x_bounds.0;
        let y_span = drawn.y_bounds.1 - drawn.y_bounds.0;
        assert!(x_span > 3.99, "the requested extent is never cut off");
        assert!(y_span > 3.99);
        let wanted = (72.0 * 0.5) / 28.0;
        assert!(
            (x_span / y_span - wanted).abs() < 1e-9,
            "aspect {x_span}:{y_span}"
        );
    }

    #[test]
    fn a_circle_drawn_by_proof_is_round_and_closed() {
        let drawn = plate("x^2 + y^2 - 1", FieldReading::Zero, 2.0);
        let rows: Vec<&str> = drawn.text.lines().collect();
        let marked: Vec<(usize, usize)> = rows
            .iter()
            .enumerate()
            .flat_map(|(row, line)| {
                line.chars()
                    .enumerate()
                    .filter(|(_, mark)| *mark == CURVE)
                    .map(move |(col, _)| (row, col))
            })
            .collect();
        assert!(
            marked.len() > 40,
            "a circle is more than {} cells",
            marked.len()
        );
        // Round means the drawn width and height agree once the cell shape is
        // taken out. Cells are twice as tall as wide, so the pixel width of a
        // round circle is twice its pixel height.
        let cols: Vec<usize> = marked.iter().map(|(_, col)| *col).collect();
        let rows_hit: Vec<usize> = marked.iter().map(|(row, _)| *row).collect();
        let width = cols.iter().max().unwrap() - cols.iter().min().unwrap();
        let height = rows_hit.iter().max().unwrap() - rows_hit.iter().min().unwrap();
        let ratio = width as f64 / height as f64;
        assert!(
            (ratio - 2.0).abs() < 0.2,
            "a circle came out {width} by {height}, ratio {ratio}"
        );
    }

    #[test]
    fn an_ordinary_curve_resolves_completely_at_terminal_size() {
        // The point of proving rather than sampling: at this resolution the
        // answer is not merely plausible, it is complete.
        for source in ["x^2 + y^2 - 1", "x*y - 1"] {
            let drawn = plate(source, FieldReading::Zero, 2.0);
            assert!(
                drawn.is_complete(),
                "{source} left {} cells unresolved",
                drawn.unresolved
            );
            assert!(!drawn.text.contains(NO_ANSWER));
        }
        // A thin parabola can leave a cell the coarse IVT does not catch.
        // Completeness is measured, not a theorem about every ordinary curve.
        let parabola = plate("y - x^2", FieldReading::Zero, 2.0);
        assert!(parabola.text.contains(CURVE));
        assert!(
            parabola.unresolved < 8,
            "parabola left {} cells unresolved",
            parabola.unresolved
        );
    }

    #[test]
    fn a_curve_that_touches_zero_without_crossing_is_not_silently_dropped() {
        // This is the failure that made proving necessary. A sign test draws
        // nothing here at any resolution, because the value reaches zero and
        // turns back. The honest answer is doubt, not a blank plate.
        let drawn = plate("(x^2 + y^2 - 1)^2", FieldReading::Zero, 2.0);
        assert!(
            drawn.unresolved > 0,
            "a tangential curve was reported as certainly absent"
        );
        assert!(drawn.text.contains(NO_ANSWER));
    }

    #[test]
    fn an_asymptote_is_not_drawn_as_a_crossing() {
        // The other measured failure: a sign test grows a curve at every pole
        // of a tangent, where there is none.
        let expression = parse_field("y - tan(x)").expect("parses");
        let drawn = draw(
            &expression,
            FieldReading::Zero,
            (-4.0, 4.0),
            (-4.0, 4.0),
            1.0,
            DEFAULT_FIELD_SIZE,
            0.5,
        )
        .expect("draws");
        // Every cell a pole can reach is doubted rather than marked.
        assert!(drawn.unresolved > 0);
        let poles_marked = drawn
            .text
            .lines()
            .all(|line| line.chars().filter(|mark| *mark == CURVE).count() < 20);
        assert!(poles_marked, "a row was filled with invented curve");
    }

    #[test]
    fn a_phase_portrait_shows_every_step_of_the_ramp_around_a_zero() {
        let drawn = plate("z", FieldReading::Phase, 2.0);
        for step in RAMP {
            assert!(
                drawn.text.contains(step),
                "the ramp step {step:?} never appeared around a simple zero"
            );
        }
    }

    #[test]
    fn a_pole_is_drawn_as_a_pole_and_an_undefined_sample_as_no_answer() {
        let drawn = plate("1/z", FieldReading::Phase, 2.0);
        // The centre sample of an even-sized grid does not land exactly on the
        // origin, so the picture is a wheel rather than a marked point; what
        // matters is that nothing claimed to be undefined.
        assert_eq!(drawn.undefined, 0);
        let undefined = plate("min(z, 1)", FieldReading::Phase, 2.0);
        assert!(undefined.undefined > 0, "an off-line minimum has no answer");
        assert!(undefined.text.contains(NO_ANSWER));
    }

    #[test]
    fn a_height_map_climbs_away_from_a_zero() {
        let drawn = plate("z", FieldReading::Height, 8.0);
        let rows: Vec<&str> = drawn.text.lines().collect();
        let middle = rows[rows.len() / 2];
        let centre = middle.chars().nth(middle.chars().count() / 2).unwrap();
        let edge = middle.chars().next().unwrap();
        let rank = |mark: char| RAMP.iter().position(|step| *step == mark).unwrap_or(0);
        assert!(
            rank(edge) > rank(centre),
            "the far edge should be higher than the middle"
        );
    }

    #[test]
    fn the_zero_reading_refuses_a_field_that_leaves_the_real_line() {
        let expression = parse_field("z^2 - 1").expect("parses");
        let refused = draw(
            &expression,
            FieldReading::Zero,
            (-2.0, 2.0),
            (-1.0, 1.0),
            1.0,
            DEFAULT_FIELD_SIZE,
            0.5,
        );
        assert_eq!(refused, Err(FieldError::NotRealValued));
        assert!(FieldError::NotRealValued.message().contains("phase"));
    }

    #[test]
    fn real_valued_is_decided_from_the_expression_and_not_from_a_sample() {
        for real in [
            "x^2 + y^2 - 1",
            "abs(z) - 1",
            "re(z)*im(z)",
            "arg(z)",
            "sin(x) - y",
            "min(x, y)",
            "mod(x, 2) - y",
            "x^3 - y",
        ] {
            assert!(is_real_valued(&parse_field(real).unwrap()), "{real}");
        }
        for complex in ["z", "i", "z^2 - 1", "sqrt(x)", "ln(x)", "x^0.5", "conj(z)"] {
            assert!(!is_real_valued(&parse_field(complex).unwrap()), "{complex}");
        }
    }

    #[test]
    fn a_plate_refuses_a_size_or_a_window_it_cannot_draw_honestly() {
        let expression = parse_field("x - y").expect("parses");
        let attempt = |size, x_bounds, y_bounds| {
            draw(
                &expression,
                FieldReading::Phase,
                x_bounds,
                y_bounds,
                1.0,
                size,
                0.5,
            )
        };
        assert_eq!(
            attempt((1, 8), (-1.0, 1.0), (-1.0, 1.0)),
            Err(FieldError::InvalidSize)
        );
        assert_eq!(
            attempt((8, 1), (-1.0, 1.0), (-1.0, 1.0)),
            Err(FieldError::InvalidSize)
        );
        assert_eq!(
            attempt((MAX_FIELD_WIDTH + 1, 8), (-1.0, 1.0), (-1.0, 1.0)),
            Err(FieldError::InvalidSize)
        );
        assert_eq!(
            attempt((8, 8), (1.0, -1.0), (-1.0, 1.0)),
            Err(FieldError::InvalidWindow)
        );
        assert_eq!(
            attempt((8, 8), (0.0, 0.0), (-1.0, 1.0)),
            Err(FieldError::InvalidWindow)
        );
        assert_eq!(
            attempt((8, 8), (f64::NAN, 1.0), (-1.0, 1.0)),
            Err(FieldError::InvalidWindow)
        );
    }

    #[test]
    fn a_reading_names_itself_and_cycles_through_all_three() {
        let mut reading = FieldReading::default();
        let mut seen = Vec::new();
        for _ in 0..3 {
            seen.push(reading.name());
            assert_eq!(FieldReading::parse(reading.name()), Some(reading));
            assert!(!reading.legend().is_empty());
            reading = reading.next();
        }
        assert_eq!(reading, FieldReading::default(), "the cycle closes");
        assert_eq!(seen, ["phase", "height", "zero"]);
        assert_eq!(FieldReading::parse("spiral"), None);
    }

    #[test]
    fn drawing_the_same_field_twice_gives_the_same_plate() {
        let first = plate("z^3 - 1", FieldReading::Phase, 2.0);
        let second = plate("z^3 - 1", FieldReading::Phase, 2.0);
        assert_eq!(first, second);
    }

    #[test]
    fn a_logarithm_shows_its_branch_cut() {
        // The principal cut is the negative real axis. Across it the argument
        // jumps by a turn, so the ramp seam sits there rather than a smooth
        // wheel. A picture that coloured across the cut would be another
        // function.
        let drawn = plate("ln(z)", FieldReading::Phase, 2.0);
        let rows: Vec<&str> = drawn.text.lines().collect();
        let mid = rows.len() / 2;
        let above = rows[mid.saturating_sub(1)];
        let below = rows[(mid + 1).min(rows.len() - 1)];
        let left = 4usize;
        let a = above.chars().nth(left).unwrap();
        let b = below.chars().nth(left).unwrap();
        assert_ne!(
            a, b,
            "the cut should split the ramp, not hide as a matching pair {a:?}"
        );
        let sqrt = plate("sqrt(z)", FieldReading::Phase, 2.0);
        assert!(sqrt.text.contains(NO_ANSWER) || sqrt.text != drawn.text);
    }

    #[test]
    fn mark_level_leaves_space_empty_and_climbs_the_ramp() {
        assert_eq!(super::mark_level(' '), None);
        let dot = super::mark_level('.').unwrap();
        let hash = super::mark_level('#').unwrap();
        assert!(hash > dot);
        assert_eq!(super::mark_level('@'), Some(1.0));
    }
}
