//! The Studio's expression engine: type a function, get a curve.
//!
//! A small, safe evaluator for single-variable expressions in `x`, the seed of
//! the creative graphing calculator (Tier 1 of the extensibility model in
//! `docs/ARCHITECTURE.md`: no arbitrary code, just math). It parses `sin(3*x) +
//! x^2/2` into an AST and evaluates it, so a plotter, a quiz, or an authored room
//! can all share one safe language. See `docs/PLAYFUL.md`.

use std::f64::consts::{E, PI};

use crate::sound::{Note, SoundSpec};

/// Maximum accepted Studio source length for share files and links.
pub const MAX_STUDIO_SOURCE_CHARS: usize = 512;

/// A shareable Studio expression plus its viewing parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct StudioCreation {
    source: String,
    xmin: f64,
    xmax: f64,
    a: f64,
}

impl StudioCreation {
    /// Build a validated Studio creation.
    ///
    /// # Errors
    /// Returns a message if the source is empty, too large, contains control
    /// characters, does not parse, or if the range/parameter are not finite.
    pub fn new(source: impl Into<String>, xmin: f64, xmax: f64, a: f64) -> Result<Self, String> {
        let source = source.into().trim().to_string();
        validate_share_source(&source)?;
        parse(&source)?;
        validate_share_numbers(xmin, xmax, a)?;
        Ok(Self {
            source,
            xmin,
            xmax,
            a,
        })
    }

    /// The Studio expression source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Left edge of the shared x range.
    #[must_use]
    pub fn xmin(&self) -> f64 {
        self.xmin
    }

    /// Right edge of the shared x range.
    #[must_use]
    pub fn xmax(&self) -> f64 {
        self.xmax
    }

    /// Shared value for the parameter `a`.
    #[must_use]
    pub fn a(&self) -> f64 {
        self.a
    }

    /// Serialize to the first `.num` Studio file format.
    #[must_use]
    pub fn to_num_file(&self) -> String {
        format!(
            "NUMINOUS_STUDIO 1\nexpr={}\nxmin={}\nxmax={}\na={}\n",
            self.source,
            format_share_number(self.xmin),
            format_share_number(self.xmax),
            format_share_number(self.a)
        )
    }

    /// Parse a `.num` Studio file.
    ///
    /// # Errors
    /// Returns a message if the file is malformed or describes an invalid
    /// Studio expression.
    pub fn from_num_file(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        match lines.next() {
            Some("NUMINOUS_STUDIO 1") => {}
            _ => return Err("not a Numinous Studio .num file".to_string()),
        }
        let mut source: Option<String> = None;
        let mut xmin: Option<f64> = None;
        let mut xmax: Option<f64> = None;
        let mut a: Option<f64> = None;
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("bad Studio .num line '{line}'"))?;
            match key {
                "expr" if source.is_none() => source = Some(value.to_string()),
                "xmin" if xmin.is_none() => xmin = Some(parse_share_number("xmin", value)?),
                "xmax" if xmax.is_none() => xmax = Some(parse_share_number("xmax", value)?),
                "a" if a.is_none() => a = Some(parse_share_number("a", value)?),
                "expr" | "xmin" | "xmax" | "a" => {
                    return Err(format!("duplicate Studio .num field '{key}'"));
                }
                other => return Err(format!("unknown Studio .num field '{other}'")),
            }
        }
        Self::new(
            source.ok_or_else(|| "missing Studio expression".to_string())?,
            xmin.ok_or_else(|| "missing xmin".to_string())?,
            xmax.ok_or_else(|| "missing xmax".to_string())?,
            a.ok_or_else(|| "missing a".to_string())?,
        )
    }

    /// Produce a native `numinous://` Studio link for this creation.
    #[must_use]
    pub fn to_link(&self) -> String {
        format!(
            "numinous://studio?expr={}&xmin={}&xmax={}&a={}",
            percent_encode(&self.source),
            format_share_number(self.xmin),
            format_share_number(self.xmax),
            format_share_number(self.a)
        )
    }

    /// Parse a native `numinous://` Studio link.
    ///
    /// # Errors
    /// Returns a message if the link is malformed or describes an invalid
    /// Studio expression.
    pub fn from_link(link: &str) -> Result<Self, String> {
        let query = link
            .strip_prefix("numinous://studio?")
            .or_else(|| link.strip_prefix("numinous://studio/?"))
            .ok_or_else(|| "not a Numinous Studio link".to_string())?;
        let mut source: Option<String> = None;
        let mut xmin: Option<f64> = None;
        let mut xmax: Option<f64> = None;
        let mut a: Option<f64> = None;
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("bad Studio link parameter '{pair}'"))?;
            match key {
                "expr" if source.is_none() => source = Some(percent_decode(value)?),
                "xmin" if xmin.is_none() => xmin = Some(parse_share_number("xmin", value)?),
                "xmax" if xmax.is_none() => xmax = Some(parse_share_number("xmax", value)?),
                "a" if a.is_none() => a = Some(parse_share_number("a", value)?),
                "expr" | "xmin" | "xmax" | "a" => {
                    return Err(format!("duplicate Studio link field '{key}'"));
                }
                other => return Err(format!("unknown Studio link field '{other}'")),
            }
        }
        Self::new(
            source.ok_or_else(|| "missing Studio expression".to_string())?,
            xmin.ok_or_else(|| "missing xmin".to_string())?,
            xmax.ok_or_else(|| "missing xmax".to_string())?,
            a.ok_or_else(|| "missing a".to_string())?,
        )
    }
}

fn validate_share_source(source: &str) -> Result<(), String> {
    if source.is_empty() {
        return Err("Studio expression is empty".to_string());
    }
    if source.chars().count() > MAX_STUDIO_SOURCE_CHARS {
        return Err(format!(
            "Studio expression is too long; limit is {MAX_STUDIO_SOURCE_CHARS} characters"
        ));
    }
    if source.chars().any(char::is_control) {
        return Err("Studio expression cannot contain control characters".to_string());
    }
    Ok(())
}

fn validate_share_numbers(xmin: f64, xmax: f64, a: f64) -> Result<(), String> {
    if !xmin.is_finite() || !xmax.is_finite() || !a.is_finite() {
        return Err("Studio share numbers must be finite".to_string());
    }
    if xmax <= xmin {
        return Err("Studio share needs xmax > xmin".to_string());
    }
    Ok(())
}

fn format_share_number(value: f64) -> String {
    value.to_string()
}

fn parse_share_number(name: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("bad Studio number for {name}: '{value}'"))
}

fn percent_encode(source: &str) -> String {
    let mut out = String::new();
    for byte in source.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn percent_decode(source: &str) -> Result<String, String> {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hi = bytes
                    .get(i + 1)
                    .copied()
                    .ok_or_else(|| "truncated percent escape".to_string())?;
                let lo = bytes
                    .get(i + 2)
                    .copied()
                    .ok_or_else(|| "truncated percent escape".to_string())?;
                let value = hex_value(hi)
                    .and_then(|h| hex_value(lo).map(|l| h * 16 + l))
                    .ok_or_else(|| "bad percent escape".to_string())?;
                out.push(value);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| "Studio link is not valid UTF-8".to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// A parsed expression tree over the single variable `x`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal number (also holds folded constants like pi).
    Num(f64),
    /// The variable `x`.
    Var,
    /// The animation parameter `a`.
    Param,
    /// Unary negation.
    Neg(Box<Expr>),
    /// A binary operation.
    Bin(Op, Box<Expr>, Box<Expr>),
    /// A function call, e.g. `sin(...)`.
    Call(Func, Box<Expr>),
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Exponentiation.
    Pow,
}

/// A supported single-argument function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    /// Sine.
    Sin,
    /// Cosine.
    Cos,
    /// Tangent.
    Tan,
    /// Natural exponential.
    Exp,
    /// Natural logarithm.
    Ln,
    /// Absolute value.
    Abs,
    /// Square root.
    Sqrt,
}

/// Evaluate a parsed expression at variable `x` and parameter `a`.
#[must_use]
pub fn eval(expr: &Expr, x: f64, a: f64) -> f64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Var => x,
        Expr::Param => a,
        Expr::Neg(inner) => -eval(inner, x, a),
        Expr::Bin(op, lhs, rhs) => {
            let (lhs, rhs) = (eval(lhs, x, a), eval(rhs, x, a));
            match op {
                Op::Add => lhs + rhs,
                Op::Sub => lhs - rhs,
                Op::Mul => lhs * rhs,
                Op::Div => lhs / rhs,
                Op::Pow => lhs.powf(rhs),
            }
        }
        Expr::Call(func, arg) => {
            let arg = eval(arg, x, a);
            match func {
                Func::Sin => arg.sin(),
                Func::Cos => arg.cos(),
                Func::Tan => arg.tan(),
                Func::Exp => arg.exp(),
                Func::Ln => arg.ln(),
                Func::Abs => arg.abs(),
                Func::Sqrt => arg.sqrt(),
            }
        }
    }
}

/// Turn an expression into a melody: sample `y = f(x)` across `[xmin, xmax]` and
/// map each value to a pitch, stepping through time. You hear the curve.
#[must_use]
pub fn to_melody(expr: &Expr, xmin: f64, xmax: f64, notes: usize, a: f64) -> SoundSpec {
    let notes = notes.max(1);
    let step = 0.12_f32;
    let denom = (notes as f64 - 1.0).max(1.0);
    let samples: Vec<f64> = (0..notes)
        .map(|i| eval(expr, xmin + (xmax - xmin) * i as f64 / denom, a))
        .filter(|y| y.is_finite())
        .collect();
    if samples.is_empty() {
        return SoundSpec {
            duration: step,
            notes: Vec::new(),
        };
    }
    let ymin = samples.iter().copied().fold(f64::INFINITY, f64::min);
    let ymax = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (ymax - ymin).max(1e-9);
    let note_vec: Vec<Note> = samples
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            let norm = ((y - ymin) / span) as f32; // 0..1
            Note {
                freq: 220.0 * 2.0_f32.powf(norm * 2.0), // two octaves, 220..880 Hz
                start: i as f32 * step,
                dur: step * 1.4,
                amp: 0.3,
            }
        })
        .collect();
    SoundSpec {
        duration: note_vec.len() as f32 * step + 0.3,
        notes: note_vec,
    }
}

/// Plot `source` as ASCII over `[xmin, xmax]` at parameter `a`, auto-scaling y.
/// Returns the picture and the y range it covered.
///
/// # Errors
/// Returns a message if the expression does not parse, the ranges are invalid,
/// or the function is undefined across the whole range.
pub fn plot_text(
    source: &str,
    xmin: f64,
    xmax: f64,
    a: f64,
    width: usize,
    height: usize,
) -> Result<(String, f64, f64), String> {
    let expr = parse(source)?;
    if width < 2 || height < 2 || xmax <= xmin {
        return Err("need width >= 2, height >= 2, and xmax > xmin".to_string());
    }
    let samples: Vec<(f64, f64)> = (0..width)
        .map(|i| {
            let x = xmin + (xmax - xmin) * i as f64 / (width as f64 - 1.0);
            (x, eval(&expr, x, a))
        })
        .filter(|(_, y)| y.is_finite())
        .collect();
    if samples.is_empty() {
        return Err("nothing to plot: the function is undefined across this range".to_string());
    }
    let ymin = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let ymax = samples
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let yspan = (ymax - ymin).max(1e-9);

    let mut canvas = crate::canvas::Canvas::new(width, height);
    let mut previous: Option<(i32, i32)> = None;
    for &(x, y) in &samples {
        let sx = ((x - xmin) / (xmax - xmin) * (width as f64 - 1.0)) as i32;
        let sy = ((height as f64 - 1.0) - (y - ymin) / yspan * (height as f64 - 1.0)) as i32;
        if let Some((px, py)) = previous {
            use crate::surface::Surface;
            canvas.line(px, py, sx, sy, '#');
        }
        previous = Some((sx, sy));
    }
    Ok((canvas.to_text(), ymin, ymax))
}

/// Parse an expression in `x`, or return a human-readable error.
///
/// # Errors
/// Returns a message describing the first problem (unexpected token, unknown
/// name, unbalanced parentheses, or trailing input).
pub fn parse(source: &str) -> Result<Expr, String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.expr()?;
    if parser.pos != parser.tokens.len() {
        return Err(format!("unexpected trailing input at token {}", parser.pos));
    }
    Ok(expr)
}

/// A token in an expression.
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

/// Split `source` into tokens.
fn tokenize(source: &str) -> Result<Vec<Tok>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let value = text
                .parse::<f64>()
                .map_err(|_| format!("bad number '{text}'"))?;
            tokens.push(Tok::Num(value));
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            tokens.push(Tok::Ident(chars[start..i].iter().collect()));
        } else {
            tokens.push(match c {
                '+' => Tok::Plus,
                '-' => Tok::Minus,
                '*' => Tok::Star,
                '/' => Tok::Slash,
                '^' => Tok::Caret,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                other => return Err(format!("unexpected character '{other}'")),
            });
            i += 1;
        }
    }
    Ok(tokens)
}

/// A recursive-descent parser over a token slice.
struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    /// expr := term (('+' | '-') term)*
    fn expr(&mut self) -> Result<Expr, String> {
        let mut left = self.term()?;
        while let Some(op) = match self.peek() {
            Some(Tok::Plus) => Some(Op::Add),
            Some(Tok::Minus) => Some(Op::Sub),
            _ => None,
        } {
            self.pos += 1;
            let right = self.term()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// term := factor (('*' | '/') factor)*
    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.factor()?;
        while let Some(op) = match self.peek() {
            Some(Tok::Star) => Some(Op::Mul),
            Some(Tok::Slash) => Some(Op::Div),
            _ => None,
        } {
            self.pos += 1;
            let right = self.factor()?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// factor := unary ('^' factor)?  (right associative)
    fn factor(&mut self) -> Result<Expr, String> {
        let base = self.unary()?;
        if matches!(self.peek(), Some(Tok::Caret)) {
            self.pos += 1;
            let exp = self.factor()?;
            return Ok(Expr::Bin(Op::Pow, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    /// unary := '-' unary | atom
    fn unary(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Tok::Minus)) {
            self.pos += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.atom()
    }

    /// atom := number | name | name '(' expr ')' | '(' expr ')'
    fn atom(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::LParen) => {
                let inner = self.expr()?;
                match self.bump() {
                    Some(Tok::RParen) => Ok(inner),
                    _ => Err("expected ')'".to_string()),
                }
            }
            Some(Tok::Ident(name)) => self.ident(&name),
            other => Err(format!("unexpected token {other:?}")),
        }
    }

    /// Resolve an identifier: the variable, a constant, or a function call.
    fn ident(&mut self, name: &str) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Tok::LParen)) {
            let func = match name {
                "sin" => Func::Sin,
                "cos" => Func::Cos,
                "tan" => Func::Tan,
                "exp" => Func::Exp,
                "ln" | "log" => Func::Ln,
                "abs" => Func::Abs,
                "sqrt" => Func::Sqrt,
                other => return Err(format!("unknown function '{other}'")),
            };
            self.pos += 1; // consume '('
            let arg = self.expr()?;
            match self.bump() {
                Some(Tok::RParen) => Ok(Expr::Call(func, Box::new(arg))),
                _ => Err(format!("expected ')' after {name}(")),
            }
        } else {
            match name {
                "x" => Ok(Expr::Var),
                "a" => Ok(Expr::Param),
                "pi" => Ok(Expr::Num(PI)),
                "e" => Ok(Expr::Num(E)),
                other => Err(format!("unknown name '{other}'")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_STUDIO_SOURCE_CHARS, StudioCreation, eval, parse};

    fn at(source: &str, x: f64) -> f64 {
        eval(&parse(source).expect("parse"), x, 0.0)
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert!((at("2 + 3 * 4", 0.0) - 14.0).abs() < 1e-9);
        assert!((at("(2 + 3) * 4", 0.0) - 20.0).abs() < 1e-9);
        assert!((at("2 - 3 - 4", 0.0) - -5.0).abs() < 1e-9); // left associative
    }

    #[test]
    fn power_is_right_associative() {
        assert!((at("2 ^ 3 ^ 2", 0.0) - 512.0).abs() < 1e-9); // 2^(3^2)
    }

    #[test]
    fn variable_and_unary_minus() {
        assert!((at("x^2", 3.0) - 9.0).abs() < 1e-9);
        assert!((at("-x + 1", 4.0) - -3.0).abs() < 1e-9);
    }

    #[test]
    fn functions_and_constants() {
        assert!(at("sin(0)", 0.0).abs() < 1e-9);
        assert!((at("cos(0)", 0.0) - 1.0).abs() < 1e-9);
        assert!((at("sqrt(x)", 16.0) - 4.0).abs() < 1e-9);
        assert!((at("pi", 0.0) - std::f64::consts::PI).abs() < 1e-9);
        assert!((at("ln(e)", 0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_parameter_a_is_read() {
        let expr = parse("a * x").expect("parse");
        assert!((eval(&expr, 3.0, 2.0) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn a_function_becomes_a_rising_melody() {
        let expr = parse("x").expect("parse");
        let spec = super::to_melody(&expr, -1.0, 1.0, 8, 0.0);
        assert_eq!(spec.notes.len(), 8);
        assert!(spec.duration > 0.0);
        assert!(spec.notes.last().unwrap().freq > spec.notes[0].freq);
    }

    #[test]
    fn plot_text_draws_and_reports_the_range() {
        let (text, ymin, ymax) = super::plot_text("x", -1.0, 1.0, 0.0, 24, 8).expect("plot");
        assert!(text.contains('#'));
        assert!((ymin - -1.0).abs() < 0.1 && (ymax - 1.0).abs() < 0.1);
        assert!(super::plot_text("sin(", -1.0, 1.0, 0.0, 24, 8).is_err());
        assert!(super::plot_text("x", 1.0, -1.0, 0.0, 24, 8).is_err());
    }

    #[test]
    fn errors_are_reported() {
        assert!(parse("2 +").is_err());
        assert!(parse("sin(").is_err());
        assert!(parse("2 3").is_err()); // trailing input
        assert!(parse("nope(x)").is_err());
        assert!(parse("wut").is_err());
        assert!(parse("2 @ 3").is_err());
    }

    #[test]
    fn studio_creation_round_trips_num_files_and_links() {
        let creation = StudioCreation::new("sin(a*x) + x/2", -3.0, 3.0, 1.25).expect("creation");
        let file = creation.to_num_file();
        assert!(file.starts_with("NUMINOUS_STUDIO 1\n"));
        assert!(file.contains("expr=sin(a*x) + x/2\n"));
        assert_eq!(
            StudioCreation::from_num_file(&file).expect("file round trip"),
            creation
        );

        let link = creation.to_link();
        assert!(link.starts_with("numinous://studio?expr=sin%28a%2Ax%29%20%2B%20x%2F2"));
        assert_eq!(
            StudioCreation::from_link(&link).expect("link round trip"),
            creation
        );
    }

    #[test]
    fn studio_creation_preserves_tiny_ranges() {
        let creation = StudioCreation::new("x", 0.0, 1e-20, 1e-30).expect("tiny creation");
        let from_file =
            StudioCreation::from_num_file(&creation.to_num_file()).expect("file round trip");
        assert_eq!(from_file.xmin(), 0.0);
        assert_eq!(from_file.xmax(), 1e-20);
        assert_eq!(from_file.a(), 1e-30);
        let from_link = StudioCreation::from_link(&creation.to_link()).expect("link round trip");
        assert_eq!(from_link, creation);
    }

    #[test]
    fn studio_creation_validates_source_and_range() {
        assert!(StudioCreation::new("", -1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("sin(", -1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("x\nx", -1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("x", 1.0, 1.0, 1.0).is_err());
        assert!(StudioCreation::new("x", -1.0, 1.0, f64::NAN).is_err());
        let too_long = "x".repeat(MAX_STUDIO_SOURCE_CHARS + 1);
        assert!(StudioCreation::new(too_long, -1.0, 1.0, 1.0).is_err());
    }

    #[test]
    fn studio_creation_rejects_malformed_artifacts() {
        assert!(StudioCreation::from_num_file("nope").is_err());
        assert!(
            StudioCreation::from_num_file(
                "NUMINOUS_STUDIO 1\nexpr=x\nxmin=-1\nxmax=1\na=1\nunknown=2\n"
            )
            .is_err()
        );
        assert!(StudioCreation::from_link("https://example.com").is_err());
        assert!(StudioCreation::from_link("numinous://studio?expr=x&xmin=-1&xmax=1&a=%").is_err());
        assert!(
            StudioCreation::from_link("numinous://studio?expr=x&expr=x&xmin=-1&xmax=1&a=1")
                .is_err()
        );
    }
}
