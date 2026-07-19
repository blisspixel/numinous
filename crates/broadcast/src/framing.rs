use crate::wire::{
    HANDSHAKE_TIMEOUT, HandshakeProof, HandshakeRequest, HandshakeResponse, MAX_EVENT_BYTES,
    MAX_HANDSHAKE_BYTES, MAX_JSON_DEPTH, PUBLIC_WRITE_TIMEOUT, WireMessage,
};
use serde::{Serialize, de::DeserializeOwned};
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Write};
use std::net::TcpStream;

/// Applies the two-second handshake read and write deadlines.
pub fn configure_handshake_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
    stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))
}

/// Applies the two-second public-frame write deadline and disables read timeout.
pub fn configure_public_stream(stream: &TcpStream) -> io::Result<()> {
    stream.set_read_timeout(None)?;
    stream.set_write_timeout(Some(PUBLIC_WRITE_TIMEOUT))
}

/// Reads one bounded server-first handshake proof.
pub fn read_handshake_proof<R: BufRead>(reader: &mut R) -> Result<HandshakeProof, FrameError> {
    read_json(reader, MAX_HANDSHAKE_BYTES)
}

/// Writes one bounded server-first handshake proof.
pub fn write_handshake_proof<W: Write>(
    writer: &mut W,
    proof: &HandshakeProof,
) -> Result<(), FrameError> {
    write_json(writer, proof, MAX_HANDSHAKE_BYTES)
}

/// Reads one bounded handshake request.
pub fn read_handshake_request<R: BufRead>(reader: &mut R) -> Result<HandshakeRequest, FrameError> {
    read_json(reader, MAX_HANDSHAKE_BYTES)
}

/// Writes one bounded handshake request.
pub fn write_handshake_request<W: Write>(
    writer: &mut W,
    request: &HandshakeRequest,
) -> Result<(), FrameError> {
    write_json(writer, request, MAX_HANDSHAKE_BYTES)
}

/// Reads one bounded handshake response.
pub fn read_handshake_response<R: BufRead>(
    reader: &mut R,
) -> Result<HandshakeResponse, FrameError> {
    read_json(reader, MAX_HANDSHAKE_BYTES)
}

/// Writes one bounded handshake response.
pub fn write_handshake_response<W: Write>(
    writer: &mut W,
    response: &HandshakeResponse,
) -> Result<(), FrameError> {
    write_json(writer, response, MAX_HANDSHAKE_BYTES)
}

/// Reads one bounded public wire message.
pub fn read_public_message<R, T>(reader: &mut R) -> Result<WireMessage<T>, FrameError>
where
    R: BufRead,
    T: DeserializeOwned,
{
    read_json(reader, MAX_EVENT_BYTES)
}

/// Writes one bounded public wire message.
pub fn write_public_message<W, T>(
    writer: &mut W,
    message: &WireMessage<T>,
) -> Result<(), FrameError>
where
    W: Write,
    T: Serialize,
{
    write_json(writer, message, MAX_EVENT_BYTES)
}

fn read_json<R, T>(reader: &mut R, maximum: usize) -> Result<T, FrameError>
where
    R: BufRead,
    T: DeserializeOwned,
{
    let bytes = read_bounded_line(reader, maximum)?;
    validate_json_depth(&bytes)?;
    serde_json::from_slice(&bytes).map_err(|_| FrameError::InvalidJson)
}

fn write_json<W, T>(writer: &mut W, value: &T, maximum: usize) -> Result<(), FrameError>
where
    W: Write,
    T: Serialize,
{
    let bytes = serialize_bounded(value, maximum)?;
    writer.write_all(&bytes).map_err(FrameError::Io)?;
    writer.write_all(b"\n").map_err(FrameError::Io)?;
    writer.flush().map_err(FrameError::Io)
}

pub(crate) fn serialize_bounded<T>(value: &T, maximum: usize) -> Result<Vec<u8>, FrameError>
where
    T: Serialize,
{
    let mut buffer = LimitedBuffer::new(maximum);
    if serde_json::to_writer(&mut buffer, value).is_err() {
        return if buffer.overflowed {
            Err(FrameError::TooLarge { maximum })
        } else {
            Err(FrameError::InvalidJson)
        };
    }
    validate_json_depth(&buffer.bytes)?;
    Ok(buffer.bytes)
}

fn read_bounded_line<R: BufRead>(reader: &mut R, maximum: usize) -> Result<Vec<u8>, FrameError> {
    let mut output = Vec::with_capacity(maximum.min(4_096).saturating_add(1));
    loop {
        let available = reader.fill_buf().map_err(FrameError::Io)?;
        if available.is_empty() {
            return Err(FrameError::Truncated);
        }
        if let Some(newline) = available.iter().position(|byte| *byte == b'\n') {
            let room = maximum.saturating_add(1).saturating_sub(output.len());
            let copy = newline.min(room);
            output.extend_from_slice(&available[..copy]);
            if output.len() > maximum || newline > copy {
                reader.consume(copy);
                return Err(FrameError::TooLarge { maximum });
            }
            reader.consume(newline + 1);
            if output.is_empty() {
                return Err(FrameError::Empty);
            }
            return Ok(output);
        }

        let room = maximum.saturating_add(1).saturating_sub(output.len());
        let copy = available.len().min(room);
        output.extend_from_slice(&available[..copy]);
        reader.consume(copy);
        if output.len() > maximum {
            return Err(FrameError::TooLarge { maximum });
        }
    }
}

fn validate_json_depth(input: &[u8]) -> Result<(), FrameError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in input {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MAX_JSON_DEPTH {
                    return Err(FrameError::TooDeep {
                        maximum: MAX_JSON_DEPTH,
                    });
                }
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).ok_or(FrameError::InvalidJson)?;
            }
            _ => {}
        }
    }
    if in_string || escaped || depth != 0 {
        return Err(FrameError::InvalidJson);
    }
    Ok(())
}

struct LimitedBuffer {
    bytes: Vec<u8>,
    maximum: usize,
    overflowed: bool,
}

impl LimitedBuffer {
    fn new(maximum: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(maximum.min(4_096)),
            maximum,
            overflowed: false,
        }
    }
}

impl Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.maximum.saturating_sub(self.bytes.len());
        if bytes.len() > remaining {
            self.bytes.extend_from_slice(&bytes[..remaining]);
            self.overflowed = true;
            return Err(io::Error::other("serialized frame exceeded its bound"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Failure to read, validate, serialize, or write one bounded frame.
#[derive(Debug)]
pub enum FrameError {
    /// The peer ended the stream before a newline completed the frame.
    Truncated,
    /// The peer sent an empty line.
    Empty,
    /// The frame exceeded its declared byte budget.
    TooLarge {
        /// Maximum admitted bytes, excluding the newline delimiter.
        maximum: usize,
    },
    /// JSON nesting exceeded the pre-deserialization depth budget.
    TooDeep {
        /// Maximum admitted object and array nesting.
        maximum: usize,
    },
    /// The frame was not valid JSON for the expected typed message.
    InvalidJson,
    /// The stream failed while reading or writing a frame.
    Io(io::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("truncated broadcast frame"),
            Self::Empty => formatter.write_str("empty broadcast frame"),
            Self::TooLarge { maximum } => {
                write!(formatter, "broadcast frame exceeds {maximum} bytes")
            }
            Self::TooDeep { maximum } => {
                write!(formatter, "broadcast JSON exceeds depth {maximum}")
            }
            Self::InvalidJson => formatter.write_str("invalid broadcast frame"),
            Self::Io(_) => formatter.write_str("broadcast frame I/O failed"),
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FrameError, LimitedBuffer, configure_handshake_stream, configure_public_stream,
        read_bounded_line, read_handshake_proof, read_handshake_request, read_handshake_response,
        read_json, read_public_message, serialize_bounded, validate_json_depth,
        write_handshake_proof, write_handshake_request, write_handshake_response, write_json,
        write_public_message,
    };
    use crate::{
        Compatibility, ControlMarker, HandshakeProof, HandshakeRequest, HandshakeResponse,
        SessionId, WireMessage,
    };
    use serde::{Deserialize, Serialize, Serializer};
    use std::error::Error;
    use std::io::{self, Cursor, Write};
    use std::net::{TcpListener, TcpStream};

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(deny_unknown_fields)]
    struct Fixture {
        value: String,
    }

    #[test]
    fn exact_boundary_is_accepted_and_oversize_stops_at_max_plus_one() {
        let mut exact = Cursor::new(b"12345\n".to_vec());
        assert_eq!(
            read_bounded_line(&mut exact, 5).expect("exact frame"),
            b"12345"
        );

        let mut oversized = Cursor::new(b"123456\nnext\n".to_vec());
        assert!(matches!(
            read_bounded_line(&mut oversized, 5),
            Err(FrameError::TooLarge { maximum: 5 })
        ));
        assert_eq!(oversized.position(), 6);
        assert!(matches!(
            read_bounded_line(&mut Cursor::new(b"123456"), 5),
            Err(FrameError::TooLarge { maximum: 5 })
        ));
    }

    #[test]
    fn truncated_empty_invalid_utf8_and_deep_frames_fail_closed() {
        assert!(matches!(
            read_bounded_line(&mut Cursor::new(b"partial"), 20),
            Err(FrameError::Truncated)
        ));
        assert!(matches!(
            read_bounded_line(&mut Cursor::new(b"\n"), 20),
            Err(FrameError::Empty)
        ));
        assert!(matches!(
            read_json::<_, serde_json::Value>(&mut Cursor::new(vec![0xff, b'\n']), 20),
            Err(FrameError::InvalidJson)
        ));
        let deep = format!("{}0{}", "[".repeat(17), "]".repeat(17));
        assert!(matches!(
            validate_json_depth(deep.as_bytes()),
            Err(FrameError::TooDeep { maximum: 16 })
        ));
    }

    #[test]
    fn braces_inside_strings_do_not_consume_depth() {
        let value = format!("{{\"value\":\"\\\\\\\"{}\"}}", "[".repeat(100));
        validate_json_depth(value.as_bytes()).expect("string punctuation is inert");
    }

    #[test]
    fn typed_read_rejects_unknown_fields() {
        let mut input = Cursor::new(b"{\"value\":\"ok\",\"secret\":1}\n".to_vec());
        assert!(matches!(
            read_json::<_, Fixture>(&mut input, 100),
            Err(FrameError::InvalidJson)
        ));
    }

    #[test]
    fn serialization_is_complete_before_the_destination_is_written() {
        let fixture = Fixture {
            value: "x".repeat(100),
        };
        let mut output = Vec::new();
        assert!(matches!(
            write_json(&mut output, &fixture, 10),
            Err(FrameError::TooLarge { maximum: 10 })
        ));
        assert!(output.is_empty());

        write_json(
            &mut output,
            &Fixture {
                value: "ok".to_owned(),
            },
            64,
        )
        .expect("bounded serialization");
        assert_eq!(output.last(), Some(&b'\n'));

        let deep = serde_json::from_str::<serde_json::Value>(&format!(
            "{}0{}",
            "[".repeat(17),
            "]".repeat(17)
        ))
        .expect("valid deep JSON");
        let mut rejected = Vec::new();
        assert!(matches!(
            write_json(&mut rejected, &deep, 1_024),
            Err(FrameError::TooDeep { maximum: 16 })
        ));
        assert!(rejected.is_empty());
    }

    fn compatibility() -> Compatibility {
        Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"]).expect("valid compatibility")
    }

    #[test]
    fn public_framing_apis_round_trip_each_wire_class() {
        let proof = HandshakeProof {
            wire_version: 1,
            proof: "11".repeat(32),
        };
        let mut proof_bytes = Vec::new();
        write_handshake_proof(&mut proof_bytes, &proof).expect("write proof");
        assert_eq!(
            read_handshake_proof(&mut Cursor::new(proof_bytes)).expect("read proof"),
            proof
        );
        let request = HandshakeRequest {
            wire_version: 1,
            capability: "00".repeat(16),
            compatibility: compatibility(),
        };
        let mut request_bytes = Vec::new();
        write_handshake_request(&mut request_bytes, &request).expect("write request");
        assert_eq!(
            read_handshake_request(&mut Cursor::new(request_bytes)).expect("read request"),
            request
        );

        let response = HandshakeResponse::Accepted {
            session_id: SessionId::from_bytes([5; 16]),
            consent_epoch: 2,
            compatibility: compatibility(),
        };
        let mut response_bytes = Vec::new();
        write_handshake_response(&mut response_bytes, &response).expect("write response");
        assert_eq!(
            read_handshake_response(&mut Cursor::new(response_bytes)).expect("read response"),
            response
        );

        let message = WireMessage::<serde_json::Value>::Control {
            session_id: SessionId::from_bytes([6; 16]),
            consent_epoch: 3,
            marker: ControlMarker::Paused,
        };
        let mut event_bytes = Vec::new();
        write_public_message(&mut event_bytes, &message).expect("write public message");
        assert_eq!(
            read_public_message(&mut Cursor::new(event_bytes)).expect("read public message"),
            message
        );
    }

    #[test]
    fn stream_configuration_sets_bounded_deadlines() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
        let client = TcpStream::connect(listener.local_addr().expect("listener address"))
            .expect("connect loopback");
        let (server, _) = listener.accept().expect("accept loopback");
        configure_handshake_stream(&server).expect("configure handshake");
        assert_eq!(
            server.read_timeout().expect("read timeout"),
            Some(super::HANDSHAKE_TIMEOUT)
        );
        assert_eq!(
            server.write_timeout().expect("write timeout"),
            Some(super::HANDSHAKE_TIMEOUT)
        );
        configure_public_stream(&server).expect("configure public stream");
        assert_eq!(server.read_timeout().expect("read timeout"), None);
        assert_eq!(
            server.write_timeout().expect("write timeout"),
            Some(super::PUBLIC_WRITE_TIMEOUT)
        );
        drop(client);
    }

    struct Broken;

    impl Serialize for Broken {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Err(serde::ser::Error::custom("broken fixture"))
        }
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn serializer_and_destination_errors_are_classified_without_partial_output() {
        assert!(matches!(
            serialize_bounded(&Broken, 10),
            Err(FrameError::InvalidJson)
        ));
        let error = write_json(
            &mut FailingWriter,
            &Fixture {
                value: "ok".to_owned(),
            },
            64,
        )
        .expect_err("destination failure");
        assert_eq!(error.to_string(), "broadcast frame I/O failed");
        assert!(error.source().is_some());
        assert_eq!(
            FrameError::Truncated.to_string(),
            "truncated broadcast frame"
        );
        assert_eq!(FrameError::Empty.to_string(), "empty broadcast frame");
        assert_eq!(
            FrameError::TooLarge { maximum: 7 }.to_string(),
            "broadcast frame exceeds 7 bytes"
        );
        assert_eq!(
            FrameError::TooDeep { maximum: 4 }.to_string(),
            "broadcast JSON exceeds depth 4"
        );
        assert_eq!(
            FrameError::InvalidJson.to_string(),
            "invalid broadcast frame"
        );
        assert!(FrameError::InvalidJson.source().is_none());
        let mut buffer = LimitedBuffer::new(2);
        buffer.flush().expect("limited buffer flush");
    }
}
