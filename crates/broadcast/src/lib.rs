//! Bounded local session-broadcast primitives.
//!
//! This crate owns no gameplay and no persistence. It supplies the consent,
//! pairing, framing, replay identity, and queue boundaries shared by the App
//! viewer and the MCP producer.

mod consent;
mod fingerprint;
mod framing;
mod hex;
mod pairing;
mod projection;
mod queue;
mod receiver;
mod wire;

pub use consent::{
    CommitOutcome, ConsentMachine, ConsentState, ConsentStatus, ConsentTicket, EventLease,
    TransitionError, WriteError,
};
pub use fingerprint::{
    BUILD_SEMANTIC_ID, Compatibility, CompatibilityError, CompatibilityFingerprint,
    REPLAY_ABI_VERSION, WIRE_VERSION,
};
pub use framing::{
    FrameError, configure_handshake_stream, configure_public_stream, read_handshake_proof,
    read_handshake_request, read_handshake_response, read_public_message, write_handshake_proof,
    write_handshake_request, write_handshake_response, write_public_message,
};
pub use pairing::{
    MAX_HANDSHAKE_ATTEMPTS, MAX_PAIRING_CODE_BYTES, PAIRING_TTL, PairingCode, PairingError,
    PairingGate, PairingOffer, PairingVerdict,
};
pub use projection::{
    ALL_PUBLIC_TOOLS, NUMINOUS_GAME_IDS, PLAY_ROOM_MAX_HEIGHT, PLAY_ROOM_MAX_WIDTH,
    ProjectionError, PublicTool, PublicToolEvent, numinous_compatibility,
};
pub use queue::{EventQueueStatus, MAX_QUEUED_BYTES, MAX_QUEUED_EVENTS, PreparedEvent};
pub use receiver::{PublicReceiver, ReceiveError, ReceiveOutcome, ReceiverState};
pub use wire::{
    ControlMarker, EventEnvelope, HANDSHAKE_TIMEOUT, HandshakeProof, HandshakeRequest,
    HandshakeResponse, MAX_EVENT_BYTES, MAX_HANDSHAKE_BYTES, MAX_JSON_DEPTH, PUBLIC_WRITE_TIMEOUT,
    SequenceRange, SessionId, SessionIdError, WireMessage,
};
