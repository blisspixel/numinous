use crate::{Compatibility, ControlMarker, EventEnvelope, SequenceRange, SessionId, WireMessage};
use std::error::Error;
use std::fmt;

/// Consent state independently verified by a public-stream receiver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiverState {
    /// Public events are admitted for the current epoch.
    Live,
    /// The producer paused public emission and advanced the epoch.
    Paused,
    /// The producer ended the session. No later frame is valid.
    Stopped,
}

/// One validated public-stream outcome.
#[derive(Clone, Debug, PartialEq)]
pub enum ReceiveOutcome<T> {
    /// A self-contained public event that is safe to retain or render.
    Event(EventEnvelope<T>),
    /// A content-free consent transition.
    Control(ControlMarker),
}

/// Stateful validation for one authenticated public stream.
///
/// Framing proves that a message has the strict wire shape. This receiver then
/// proves that it belongs to the authenticated session, matches the negotiated
/// replay identity, follows the current consent epoch, and accounts for every
/// public sequence exactly once.
#[derive(Debug)]
pub struct PublicReceiver {
    session_id: SessionId,
    compatibility: Compatibility,
    state: ReceiverState,
    epoch: u64,
    next_sequence: u64,
}

impl PublicReceiver {
    /// Starts verification at the live epoch returned by the handshake.
    #[must_use]
    pub fn new(session_id: SessionId, compatibility: Compatibility, live_epoch: u64) -> Self {
        Self {
            session_id,
            compatibility,
            state: ReceiverState::Live,
            epoch: live_epoch,
            next_sequence: 0,
        }
    }

    /// Validates and advances one ordered public message.
    pub fn receive<T>(
        &mut self,
        message: WireMessage<T>,
    ) -> Result<ReceiveOutcome<T>, ReceiveError> {
        match message {
            WireMessage::Event(envelope) => self.receive_event(envelope),
            WireMessage::Control {
                session_id,
                consent_epoch,
                marker,
            } => self.receive_control(session_id, consent_epoch, marker),
        }
    }

    /// Returns the last verified consent state.
    #[must_use]
    pub const fn state(&self) -> ReceiverState {
        self.state
    }

    /// Returns the last verified consent epoch.
    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Returns the next public sequence that must be present or exactly skipped.
    #[must_use]
    pub const fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    fn receive_event<T>(
        &mut self,
        envelope: EventEnvelope<T>,
    ) -> Result<ReceiveOutcome<T>, ReceiveError> {
        self.require_session(envelope.session_id)?;
        if envelope.compatibility != self.compatibility {
            return Err(ReceiveError::CompatibilityMismatch);
        }
        if self.state != ReceiverState::Live {
            return Err(ReceiveError::EventOutsideLiveEpoch);
        }
        if envelope.consent_epoch != self.epoch {
            return Err(ReceiveError::EpochMismatch);
        }
        require_exact_sequence(
            self.next_sequence,
            envelope.public_sequence,
            envelope.skipped,
        )?;
        self.next_sequence = envelope
            .public_sequence
            .checked_add(1)
            .ok_or(ReceiveError::SequenceExhausted)?;
        Ok(ReceiveOutcome::Event(envelope))
    }

    fn receive_control<T>(
        &mut self,
        session_id: SessionId,
        consent_epoch: u64,
        marker: ControlMarker,
    ) -> Result<ReceiveOutcome<T>, ReceiveError> {
        self.require_session(session_id)?;
        let next_epoch = self
            .epoch
            .checked_add(1)
            .ok_or(ReceiveError::EpochExhausted)?;
        if consent_epoch != next_epoch {
            return Err(ReceiveError::EpochMismatch);
        }
        self.state = match (self.state, marker) {
            (ReceiverState::Live, ControlMarker::Paused) => ReceiverState::Paused,
            (ReceiverState::Paused, ControlMarker::Resumed) => ReceiverState::Live,
            (ReceiverState::Live | ReceiverState::Paused, ControlMarker::Stopped) => {
                ReceiverState::Stopped
            }
            _ => return Err(ReceiveError::InvalidTransition),
        };
        self.epoch = consent_epoch;
        Ok(ReceiveOutcome::Control(marker))
    }

    fn require_session(&self, session_id: SessionId) -> Result<(), ReceiveError> {
        if session_id == self.session_id {
            Ok(())
        } else {
            Err(ReceiveError::SessionMismatch)
        }
    }
}

fn require_exact_sequence(
    expected: u64,
    actual: u64,
    skipped: Option<SequenceRange>,
) -> Result<(), ReceiveError> {
    match skipped {
        None if actual == expected => Ok(()),
        Some(range)
            if range.first == expected
                && range.last.checked_add(1) == Some(actual)
                && actual > expected =>
        {
            Ok(())
        }
        _ => Err(ReceiveError::SequenceMismatch),
    }
}

/// A fail-closed public-stream validation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiveError {
    /// A frame named a different authenticated session.
    SessionMismatch,
    /// A frame used a different replay identity from the handshake.
    CompatibilityMismatch,
    /// A frame did not advance or reuse the consent epoch exactly as required.
    EpochMismatch,
    /// A content frame arrived while the verified state was not live.
    EventOutsideLiveEpoch,
    /// A sequence was duplicated, reordered, or hidden by an inexact gap.
    SequenceMismatch,
    /// A control marker did not follow the consent state machine.
    InvalidTransition,
    /// The consent epoch cannot advance without wrapping.
    EpochExhausted,
    /// The public sequence cannot advance without wrapping.
    SequenceExhausted,
}

impl fmt::Display for ReceiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SessionMismatch => "broadcast session identity mismatch",
            Self::CompatibilityMismatch => "broadcast replay compatibility mismatch",
            Self::EpochMismatch => "broadcast consent epoch mismatch",
            Self::EventOutsideLiveEpoch => "broadcast event arrived outside a live epoch",
            Self::SequenceMismatch => "broadcast public sequence mismatch",
            Self::InvalidTransition => "invalid broadcast consent transition",
            Self::EpochExhausted => "broadcast consent epoch exhausted",
            Self::SequenceExhausted => "broadcast public sequence exhausted",
        };
        formatter.write_str(message)
    }
}

impl Error for ReceiveError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn compatibility() -> Compatibility {
        Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"]).expect("valid compatibility")
    }

    fn session(byte: u8) -> SessionId {
        SessionId::from_bytes([byte; 16])
    }

    fn event(
        session_id: SessionId,
        compatibility: Compatibility,
        epoch: u64,
        sequence: u64,
        skipped: Option<SequenceRange>,
    ) -> WireMessage<&'static str> {
        WireMessage::Event(EventEnvelope {
            session_id,
            consent_epoch: epoch,
            public_sequence: sequence,
            skipped,
            compatibility,
            event: "public",
        })
    }

    #[test]
    fn exact_events_and_accounted_gaps_advance_once() {
        let compatibility = compatibility();
        let mut receiver = PublicReceiver::new(session(1), compatibility.clone(), 2);

        assert!(matches!(
            receiver.receive(event(session(1), compatibility.clone(), 2, 0, None)),
            Ok(ReceiveOutcome::Event(_))
        ));
        assert!(matches!(
            receiver.receive(event(
                session(1),
                compatibility,
                2,
                4,
                SequenceRange::new(1, 3)
            )),
            Ok(ReceiveOutcome::Event(_))
        ));
        assert_eq!(receiver.next_sequence(), 5);
    }

    #[test]
    fn reordered_duplicated_and_inexact_gap_sequences_fail_closed() {
        let compatibility = compatibility();
        for (sequence, skipped) in [
            (1, None),
            (2, SequenceRange::new(1, 1)),
            (3, SequenceRange::new(0, 1)),
            (3, SequenceRange::new(1, 2)),
        ] {
            let mut receiver = PublicReceiver::new(session(1), compatibility.clone(), 2);
            assert_eq!(
                receiver.receive(event(
                    session(1),
                    compatibility.clone(),
                    2,
                    sequence,
                    skipped
                )),
                Err(ReceiveError::SequenceMismatch)
            );
        }
    }

    #[test]
    fn controls_require_exact_epochs_and_legal_transitions() {
        let compatibility = compatibility();
        let mut receiver = PublicReceiver::new(session(1), compatibility, 2);
        let control = |epoch, marker| WireMessage::<()>::Control {
            session_id: session(1),
            consent_epoch: epoch,
            marker,
        };

        assert_eq!(
            receiver.receive(control(3, ControlMarker::Paused)),
            Ok(ReceiveOutcome::Control(ControlMarker::Paused))
        );
        assert_eq!(receiver.state(), ReceiverState::Paused);
        assert_eq!(receiver.epoch(), 3);
        assert_eq!(
            receiver.receive(control(4, ControlMarker::Resumed)),
            Ok(ReceiveOutcome::Control(ControlMarker::Resumed))
        );
        assert_eq!(
            receiver.receive(control(5, ControlMarker::Stopped)),
            Ok(ReceiveOutcome::Control(ControlMarker::Stopped))
        );
        assert_eq!(receiver.state(), ReceiverState::Stopped);
        assert_eq!(
            receiver.receive(control(6, ControlMarker::Resumed)),
            Err(ReceiveError::InvalidTransition)
        );
    }

    #[test]
    fn identity_compatibility_epoch_and_paused_content_are_rejected() {
        let compatibility = compatibility();
        let other = Compatibility::from_catalogs(["life"], ["rossler"], ["munch"])
            .expect("different compatibility");
        let mut receiver = PublicReceiver::new(session(1), compatibility.clone(), 2);
        assert_eq!(
            receiver.receive(event(session(2), compatibility.clone(), 2, 0, None)),
            Err(ReceiveError::SessionMismatch)
        );
        assert_eq!(
            receiver.receive(event(session(1), other, 2, 0, None)),
            Err(ReceiveError::CompatibilityMismatch)
        );
        assert_eq!(
            receiver.receive(event(session(1), compatibility.clone(), 3, 0, None)),
            Err(ReceiveError::EpochMismatch)
        );
        receiver
            .receive(WireMessage::<&str>::Control {
                session_id: session(1),
                consent_epoch: 3,
                marker: ControlMarker::Paused,
            })
            .expect("pause");
        assert_eq!(
            receiver.receive(event(session(1), compatibility, 3, 0, None)),
            Err(ReceiveError::EventOutsideLiveEpoch)
        );
    }
}
