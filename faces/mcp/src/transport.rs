//! Bounded newline transport for the MCP stdio face.
//!
//! JSON-RPC semantics, protocol negotiation, dispatch, and tool execution stay
//! in the server. This module owns only record framing, memory bounds, overflow
//! resynchronization, and newline-terminated response writes.

use std::io::{self, BufRead, Read, Write};

use serde_json::Value;

/// The most bytes one JSON-RPC request line may hold. Every legitimate call
/// is a few KiB; without a cap a client streaming an endless newline-free
/// request would grow the line buffer without bound.
const MAX_REQUEST_BYTES: usize = 1_048_576;

/// Read one newline-terminated request into `line`, holding at most the request
/// bound plus one byte used to detect overflow.
///
/// An oversized line is drained to its newline and replaced with a tiny invalid
/// JSON marker, so the caller emits its ordinary parse error. Returns false at
/// end of input.
pub(super) fn read_bounded_line(reader: &mut impl BufRead, line: &mut Vec<u8>) -> io::Result<bool> {
    line.clear();
    let read = reader
        .take(MAX_REQUEST_BYTES as u64 + 1)
        .read_until(b'\n', line)?;
    if read == 0 {
        return Ok(false);
    }
    if line.len() > MAX_REQUEST_BYTES {
        let newline_was_consumed = line.last() == Some(&b'\n');
        line.clear();
        line.push(b'{');
        if !newline_was_consumed {
            drain_record(reader)?;
        }
    }
    Ok(true)
}

fn drain_record(reader: &mut impl BufRead) -> io::Result<()> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(());
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        let finished = available.get(consumed - 1) == Some(&b'\n');
        reader.consume(consumed);
        if finished {
            return Ok(());
        }
    }
}

/// Write a single JSON-RPC message as one newline-terminated line.
pub(super) fn write_message(out: &mut impl Write, message: &Value) -> io::Result<()> {
    writeln!(out, "{message}")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::{MAX_REQUEST_BYTES, read_bounded_line, write_message};
    use serde_json::json;

    #[test]
    fn oversized_records_are_replaced_and_the_reader_resynchronizes() {
        let mut input = Vec::new();
        input.extend(std::iter::repeat_n(b'x', MAX_REQUEST_BYTES + 100));
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        input.push(b'\n');
        let mut reader = std::io::BufReader::new(&input[..]);
        let mut line = Vec::new();

        assert!(read_bounded_line(&mut reader, &mut line).expect("oversized record"));
        assert_eq!(line, b"{");
        assert!(serde_json::from_slice::<serde_json::Value>(&line).is_err());

        assert!(read_bounded_line(&mut reader, &mut line).expect("following record"));
        assert!(serde_json::from_slice::<serde_json::Value>(&line).is_ok());
        assert!(!read_bounded_line(&mut reader, &mut line).expect("EOF"));
    }

    #[test]
    fn exact_total_bound_is_preserved() {
        let mut input = vec![b'x'; MAX_REQUEST_BYTES - 1];
        input.push(b'\n');
        let mut reader = std::io::BufReader::new(&input[..]);
        let mut line = Vec::new();

        assert!(read_bounded_line(&mut reader, &mut line).expect("exact record"));
        assert_eq!(line, input);
        assert!(!read_bounded_line(&mut reader, &mut line).expect("EOF"));
    }

    #[test]
    fn first_oversized_record_does_not_consume_following_records() {
        let mut input = vec![b'x'; MAX_REQUEST_BYTES];
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        input.push(b'\n');
        input.extend_from_slice(br#"{"jsonrpc":"2.0","id":2,"method":"ping"}"#);
        input.push(b'\n');
        let mut reader = std::io::BufReader::new(&input[..]);
        let mut line = Vec::new();

        assert!(read_bounded_line(&mut reader, &mut line).expect("oversized record"));
        assert!(serde_json::from_slice::<serde_json::Value>(&line).is_err());
        for expected_id in [1, 2] {
            assert!(read_bounded_line(&mut reader, &mut line).expect("request"));
            let request: serde_json::Value = serde_json::from_slice(&line).expect("valid request");
            assert_eq!(request["id"], expected_id);
        }
        assert!(!read_bounded_line(&mut reader, &mut line).expect("EOF"));
    }

    #[test]
    fn message_write_is_one_flushed_json_line() {
        let mut output = Vec::new();
        write_message(&mut output, &json!({"ok": true})).expect("write");
        let line = String::from_utf8(output).expect("UTF-8");
        assert_eq!(line.lines().count(), 1);
        assert!(line.ends_with('\n'));
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).expect("valid JSON");
        assert_eq!(parsed["ok"], true);
    }
}
