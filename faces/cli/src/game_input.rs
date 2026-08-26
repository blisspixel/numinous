//! Bounded terminal input policy shared by the CLI games.
//!
//! Game rules, progression, scoring, and concepts remain in `numinous-core`
//! and the command handlers. This module owns only record boundaries, neutral
//! departures, and the terminal prose associated with those decisions.

use std::io::{BufRead, Read, Write};

pub(super) const MAX_CLI_INPUT_BYTES: usize = 4 * 1024;

/// Treat `?` as an honest question without spending a move.
pub(super) fn asked_why(line: &str, game: &str) -> bool {
    if line.trim() != "?" {
        return false;
    }
    if let Some(text) = numinous_core::concept(game) {
        println!(
            "
{text}
"
        );
    }
    true
}

/// Read one prompted game input without turning a closed pipe into a move.
///
/// EOF, overlong records, and read errors are neutral departures. Callers must
/// not mutate progression or post a score when this function returns `None`.
pub(super) fn read_game_line(input: &mut impl BufRead, prompt: &str) -> Option<String> {
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    match read_bounded_input_line(input) {
        Ok(BoundedInputLine::Eof) => {
            println!("\nINPUT CLOSED. LEAVING WITHOUT COUNTING A MOVE.");
            None
        }
        Ok(BoundedInputLine::Line(line)) => Some(line),
        Ok(BoundedInputLine::TooLong) => {
            println!("\nINPUT TOO LONG. LEAVING WITHOUT COUNTING A MOVE.");
            None
        }
        Err(error) => {
            eprintln!("\nCould not read game input: {error}. Leaving without counting a move.");
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum BoundedInputLine {
    Eof,
    Line(String),
    TooLong,
}

/// Read one UTF-8 line while retaining at most the payload limit and its line
/// ending. Overlong input is drained through LF so a later read starts at the
/// next record instead of parsing a truncated suffix.
pub(super) fn read_bounded_input_line(
    input: &mut impl BufRead,
) -> std::io::Result<BoundedInputLine> {
    let mut bytes = Vec::with_capacity(MAX_CLI_INPUT_BYTES + 2);
    let read = (&mut *input)
        .take((MAX_CLI_INPUT_BYTES + 2) as u64)
        .read_until(b'\n', &mut bytes)?;
    if read == 0 {
        return Ok(BoundedInputLine::Eof);
    }

    let has_lf = bytes.last() == Some(&b'\n');
    let ending_len = if has_lf && bytes.get(bytes.len().saturating_sub(2)) == Some(&b'\r') {
        2
    } else {
        usize::from(has_lf)
    };
    if bytes.len().saturating_sub(ending_len) > MAX_CLI_INPUT_BYTES {
        if !has_lf {
            drain_input_line(input)?;
        }
        return Ok(BoundedInputLine::TooLong);
    }

    String::from_utf8(bytes)
        .map(BoundedInputLine::Line)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn drain_input_line(input: &mut impl BufRead) -> std::io::Result<()> {
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let finished = available.get(consumed - 1) == Some(&b'\n');
        input.consume(consumed);
        if finished {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoundedInputLine, MAX_CLI_INPUT_BYTES, read_bounded_input_line, read_game_line};

    #[test]
    fn bounded_input_preserves_boundaries_and_resynchronizes() {
        let mut exact = vec![b'x'; MAX_CLI_INPUT_BYTES];
        exact.push(b'\n');
        let mut exact = std::io::Cursor::new(exact);
        assert!(matches!(
            read_bounded_input_line(&mut exact).expect("exact LF line"),
            BoundedInputLine::Line(line) if line.len() == MAX_CLI_INPUT_BYTES + 1
        ));

        let mut crlf = vec![b'x'; MAX_CLI_INPUT_BYTES];
        crlf.extend_from_slice(b"\r\n");
        let mut crlf = std::io::Cursor::new(crlf);
        assert!(matches!(
            read_bounded_input_line(&mut crlf).expect("exact CRLF line"),
            BoundedInputLine::Line(line) if line.len() == MAX_CLI_INPUT_BYTES + 2
        ));

        let mut overflow = vec![b'x'; MAX_CLI_INPUT_BYTES + 1];
        overflow.extend_from_slice(b"\nok\n");
        let mut overflow = std::io::Cursor::new(overflow);
        assert_eq!(
            read_bounded_input_line(&mut overflow).expect("overlong line"),
            BoundedInputLine::TooLong
        );
        assert_eq!(
            read_bounded_input_line(&mut overflow).expect("following line"),
            BoundedInputLine::Line("ok\n".to_string())
        );

        let mut eof = std::io::Cursor::new(vec![b'x'; MAX_CLI_INPUT_BYTES]);
        assert!(matches!(
            read_bounded_input_line(&mut eof).expect("exact EOF line"),
            BoundedInputLine::Line(line) if line.len() == MAX_CLI_INPUT_BYTES
        ));
        assert_eq!(
            read_bounded_input_line(&mut eof).expect("EOF"),
            BoundedInputLine::Eof
        );
    }

    #[test]
    fn bounded_input_rejects_invalid_utf8_without_desynchronizing() {
        let mut input = std::io::Cursor::new(b"\xff\nnext\n".to_vec());
        let error = read_bounded_input_line(&mut input).expect_err("invalid UTF-8");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            read_bounded_input_line(&mut input).expect("following line"),
            BoundedInputLine::Line("next\n".to_string())
        );
    }

    #[test]
    fn game_line_returns_only_a_valid_bounded_record() {
        let mut valid = std::io::Cursor::new(b"move\n".to_vec());
        assert_eq!(read_game_line(&mut valid, ""), Some("move\n".to_string()));

        let mut invalid = std::io::Cursor::new(b"\xff\n".to_vec());
        assert_eq!(read_game_line(&mut invalid, ""), None);
    }
}
