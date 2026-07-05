//! The Studio's expression engine: type a function, get a curve.
//!
//! A small, safe evaluator for single-variable expressions in `x`, the seed of
//! the creative graphing calculator (Tier 1 of the extensibility model in
//! `docs/ARCHITECTURE.md`: no arbitrary code, just math). It parses `sin(3*x) +
//! x^2/2` into an AST and evaluates it, so a plotter, a quiz, or an authored room
//! can all share one safe language. See `docs/PLAYFUL.md`.

use std::f64::consts::{E, PI};

/// A parsed expression tree over the single variable `x`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal number (also holds folded constants like pi).
    Num(f64),
    /// The variable `x`.
    Var,
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

/// Evaluate a parsed expression at a given `x`.
#[must_use]
pub fn eval(expr: &Expr, x: f64) -> f64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Var => x,
        Expr::Neg(a) => -eval(a, x),
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, x), eval(b, x));
            match op {
                Op::Add => a + b,
                Op::Sub => a - b,
                Op::Mul => a * b,
                Op::Div => a / b,
                Op::Pow => a.powf(b),
            }
        }
        Expr::Call(func, a) => {
            let a = eval(a, x);
            match func {
                Func::Sin => a.sin(),
                Func::Cos => a.cos(),
                Func::Tan => a.tan(),
                Func::Exp => a.exp(),
                Func::Ln => a.ln(),
                Func::Abs => a.abs(),
                Func::Sqrt => a.sqrt(),
            }
        }
    }
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
                "pi" => Ok(Expr::Num(PI)),
                "e" => Ok(Expr::Num(E)),
                other => Err(format!("unknown name '{other}'")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{eval, parse};

    fn at(source: &str, x: f64) -> f64 {
        eval(&parse(source).expect("parse"), x)
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
    fn errors_are_reported() {
        assert!(parse("2 +").is_err());
        assert!(parse("sin(").is_err());
        assert!(parse("2 3").is_err()); // trailing input
        assert!(parse("nope(x)").is_err());
        assert!(parse("wut").is_err());
        assert!(parse("2 @ 3").is_err());
    }
}
