use crate::fingerprint::Compatibility;
use crate::hex;
use getrandom::fill;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::error::Error;
use std::fmt;
use std::time::Duration;

/// Maximum serialized handshake frame size.
pub const MAX_HANDSHAKE_BYTES: usize = 4 * 1_024;
/// Maximum serialized public frame size.
pub const MAX_EVENT_BYTES: usize = 64 * 1_024;
/// Maximum nesting admitted before JSON deserialization.
pub const MAX_JSON_DEPTH: usize = 16;
/// Read deadline for the initial handshake.
pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
/// Write deadline for each public frame.
pub const PUBLIC_WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// A nonsecret random identity for one broadcast session.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct SessionId([u8; 16]);

impl SessionId {
    /// Draws a fresh session identity from the operating system random source.
    pub fn generate() -> Result<Self, SessionIdError> {
        let mut bytes = [0; 16];
        fill(&mut bytes).map_err(|_| SessionIdError)?;
        Ok(Self(bytes))
    }

    #[cfg(test)]
    pub(crate) const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SessionId")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex::encode(&self.0))
    }
}

impl Serialize for SessionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        hex::decode(&value)
            .map(Self)
            .ok_or_else(|| de::Error::custom("invalid session identity"))
    }
}

/// Failure to obtain operating-system randomness for a session identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionIdError;

impl fmt::Display for SessionIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("operating-system randomness is unavailable")
    }
}

impl Error for SessionIdError {}

/// An inclusive range of public sequence numbers omitted before an event.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequenceRange {
    /// First omitted public sequence.
    pub first: u64,
    /// Last omitted public sequence.
    pub last: u64,
}

impl<'de> Deserialize<'de> for SequenceRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Fields {
            first: u64,
            last: u64,
        }

        let fields = Fields::deserialize(deserializer)?;
        Self::new(fields.first, fields.last)
            .ok_or_else(|| de::Error::custom("sequence range is reversed"))
    }
}

impl SequenceRange {
    /// Creates a nonempty ordered range.
    #[must_use]
    pub const fn new(first: u64, last: u64) -> Option<Self> {
        if first <= last {
            Some(Self { first, last })
        } else {
            None
        }
    }

    /// Extends contiguous or overlapping ranges.
    #[must_use]
    pub const fn merge(self, next: Self) -> Option<Self> {
        if next.first <= self.last.saturating_add(1) && self.first <= next.last.saturating_add(1) {
            Some(Self {
                first: if self.first < next.first {
                    self.first
                } else {
                    next.first
                },
                last: if self.last > next.last {
                    self.last
                } else {
                    next.last
                },
            })
        } else {
            None
        }
    }
}

/// A public event envelope with replay and consent identity.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventEnvelope<T> {
    /// Nonsecret session identity.
    pub session_id: SessionId,
    /// Consent epoch under which the event was committed.
    pub consent_epoch: u64,
    /// Monotonic public sequence within the session.
    pub public_sequence: u64,
    /// Exact public sequence range dropped before this event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<SequenceRange>,
    /// Replay compatibility declaration.
    pub compatibility: Compatibility,
    /// Typed public event payload.
    pub event: T,
}

impl<'de, T> Deserialize<'de> for EventEnvelope<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Fields<T> {
            session_id: SessionId,
            consent_epoch: u64,
            public_sequence: u64,
            skipped: Option<SequenceRange>,
            compatibility: Compatibility,
            event: T,
        }

        let fields = Fields::deserialize(deserializer)?;
        if fields
            .skipped
            .is_some_and(|range| range.last >= fields.public_sequence)
        {
            return Err(de::Error::custom(
                "skipped sequences must precede the event sequence",
            ));
        }
        Ok(Self {
            session_id: fields.session_id,
            consent_epoch: fields.consent_epoch,
            public_sequence: fields.public_sequence,
            skipped: fields.skipped,
            compatibility: fields.compatibility,
            event: fields.event,
        })
    }
}

/// Content-free consent marker serialized in the public stream.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlMarker {
    /// Public emission is paused for a new epoch.
    Paused,
    /// Public emission resumed under a new epoch.
    Resumed,
    /// The broadcast ended and no further frame is valid.
    Stopped,
}

/// A frame transmitted after authentication.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WireMessage<T> {
    /// One typed public event.
    Event(EventEnvelope<T>),
    /// One content-free consent transition.
    Control {
        /// Nonsecret session identity.
        session_id: SessionId,
        /// Epoch established by the transition.
        consent_epoch: u64,
        /// Transition marker.
        marker: ControlMarker,
    },
}

/// Initial guest authentication request.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandshakeRequest {
    /// Wire schema version offered by the guest.
    pub wire_version: u16,
    /// Lowercase hexadecimal one-use capability.
    pub capability: String,
    /// Guest replay compatibility declaration.
    pub compatibility: Compatibility,
}

impl fmt::Debug for HandshakeRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HandshakeRequest")
            .field("wire_version", &self.wire_version)
            .field("capability", &"[REDACTED]")
            .field("compatibility", &self.compatibility)
            .finish()
    }
}

/// Bounded reply to an initial guest authentication request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HandshakeResponse {
    /// Consent is live for this session and epoch.
    Accepted {
        /// Nonsecret session identity.
        session_id: SessionId,
        /// Initial live consent epoch.
        consent_epoch: u64,
        /// Host replay compatibility declaration.
        compatibility: Compatibility,
    },
    /// Authentication failed without reflecting a capability or detailed cause.
    Rejected,
}

#[cfg(test)]
mod tests {
    use super::{
        ControlMarker, EventEnvelope, HandshakeResponse, SequenceRange, SessionId, WireMessage,
    };
    use crate::Compatibility;
    use std::error::Error;

    #[test]
    fn ranges_merge_only_when_no_public_sequence_is_hidden_between_them() {
        let first = SequenceRange::new(2, 4).expect("ordered range");
        assert_eq!(
            first.merge(SequenceRange::new(5, 8).expect("ordered range")),
            SequenceRange::new(2, 8)
        );
        assert_eq!(
            first.merge(SequenceRange::new(6, 8).expect("ordered range")),
            None
        );
    }

    #[test]
    fn control_markers_carry_no_event_content() {
        let message = WireMessage::<serde_json::Value>::Control {
            session_id: SessionId::from_bytes([7; 16]),
            consent_epoch: 3,
            marker: ControlMarker::Paused,
        };
        let json = serde_json::to_string(&message).expect("serialize marker");
        assert!(!json.contains("event"));
        assert!(!json.contains("result"));
        let decoded: WireMessage<serde_json::Value> =
            serde_json::from_str(&json).expect("deserialize marker");
        assert_eq!(decoded, message);
    }

    #[test]
    fn compatibility_is_inside_every_event_envelope() {
        let compatibility = Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"])
            .expect("valid compatibility");
        let json = serde_json::json!({
            "sessionId": SessionId::from_bytes([9; 16]),
            "consentEpoch": 1,
            "publicSequence": 0,
            "compatibility": compatibility,
            "event": {"room": "life"}
        });
        let encoded = serde_json::to_vec(&json).expect("encode fixture");
        let decoded: super::EventEnvelope<serde_json::Value> =
            serde_json::from_slice(&encoded).expect("decode envelope");
        assert_eq!(decoded.public_sequence, 0);
    }

    #[test]
    fn hostile_owned_values_cannot_bypass_wire_invariants() {
        assert!(
            serde_json::from_value::<SequenceRange>(serde_json::json!({
                "first": 9,
                "last": 3
            }))
            .is_err()
        );
        let compatibility = Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"])
            .expect("valid compatibility");
        let invalid = serde_json::json!({
            "sessionId": SessionId::from_bytes([9; 16]),
            "consentEpoch": 1,
            "publicSequence": 2,
            "skipped": {"first": 4, "last": 7},
            "compatibility": compatibility,
            "event": null
        });
        assert!(serde_json::from_value::<EventEnvelope<serde_json::Value>>(invalid).is_err());
        let session = serde_json::to_value(SessionId::from_bytes([7; 16])).expect("session value");
        assert_eq!(
            serde_json::from_value::<SessionId>(session).expect("owned session string"),
            SessionId::from_bytes([7; 16])
        );
    }

    #[test]
    fn wire_json_is_camel_case_and_rejects_unknown_fields() {
        let compatibility = Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"])
            .expect("valid compatibility");
        let session_id = SessionId::from_bytes([7; 16]);
        let control = WireMessage::<serde_json::Value>::Control {
            session_id,
            consent_epoch: 3,
            marker: ControlMarker::Paused,
        };
        let control_json = serde_json::to_string(&control).expect("control JSON");
        assert_eq!(
            control_json,
            format!(
                "{{\"kind\":\"control\",\"payload\":{{\"sessionId\":\"{session_id}\",\"consentEpoch\":3,\"marker\":\"paused\"}}}}"
            )
        );
        let accepted = HandshakeResponse::Accepted {
            session_id,
            consent_epoch: 4,
            compatibility,
        };
        let accepted_json = serde_json::to_string(&accepted).expect("accepted JSON");
        assert!(accepted_json.contains("\"sessionId\""));
        assert!(accepted_json.contains("\"consentEpoch\""));
        let hostile = control_json.replacen("{\"sessionId\"", "{\"extra\":1,\"sessionId\"", 1);
        assert!(serde_json::from_str::<WireMessage<serde_json::Value>>(&hostile).is_err());
        let error = super::SessionIdError;
        assert_eq!(
            error.to_string(),
            "operating-system randomness is unavailable"
        );
        assert!(error.source().is_none());
    }
}
