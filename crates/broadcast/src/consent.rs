use crate::fingerprint::Compatibility;
use crate::framing::{FrameError, serialize_bounded};
use crate::queue::{EventQueue, EventQueueStatus, PreparedEvent, StoredEvent, TakenEvent};
use crate::wire::{
    ControlMarker, EventEnvelope, MAX_EVENT_BYTES, SequenceRange, SessionId, WireMessage,
};
use serde::Serialize;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::io::{self, Write};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

const MAX_PENDING_CONTROLS: usize = 64;

/// Externally visible consent state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsentState {
    /// No listener or public producer is active.
    Disabled,
    /// The human opened a listener and no guest has authenticated.
    AwaitingGuest,
    /// Both participants consented and public calls may be committed.
    Live,
    /// The authenticated guest paused public emission.
    Paused,
    /// The session ended and cannot be resumed.
    Stopped,
}

#[derive(Debug)]
struct ControlEntry {
    frame: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
enum LeaseKind {
    Event {
        sequence: u64,
        epoch: u64,
        skipped: Option<SequenceRange>,
    },
    Control,
}

#[derive(Clone, Copy, Debug)]
struct LeaseRecord {
    id: u64,
    kind: LeaseKind,
    writing: bool,
}

#[derive(Debug)]
struct Inner {
    state: ConsentState,
    epoch: u64,
    next_public_sequence: u64,
    dropped_public_events: u64,
    queue: EventQueue,
    controls: VecDeque<ControlEntry>,
    lease: Option<LeaseRecord>,
    next_lease_id: u64,
}

/// A capture of one live consent epoch at public-call admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsentTicket {
    epoch: u64,
}

impl ConsentTicket {
    /// Returns the epoch captured at call admission.
    #[must_use]
    pub const fn epoch(self) -> u64 {
        self.epoch
    }
}

/// Outcome of one atomic consent recheck and queue admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitOutcome {
    /// Consent changed while the public projection was being prepared.
    Discarded,
    /// The event entered the bounded writer queue.
    Queued {
        /// Sequence assigned at the commit boundary.
        sequence: u64,
        /// Older queued events dropped to admit this event.
        dropped: u64,
    },
    /// The event consumed a sequence but exceeded a complete wire bound.
    Dropped {
        /// Sequence represented by the next transmitted gap.
        sequence: u64,
    },
}

/// Linearizable consent, sequencing, queueing, and writer admission.
///
/// Public payload preparation happens before this coordinator is locked. The
/// commit operation assigns a sequence and enqueues in one critical section.
/// A pause waits only for a frame already inside its bounded write call, then
/// advances the epoch and removes every older event before returning.
#[derive(Debug)]
pub struct ConsentMachine {
    session_id: SessionId,
    compatibility: Compatibility,
    inner: Mutex<Inner>,
    available: Condvar,
}

impl ConsentMachine {
    /// Creates a disabled coordinator for one authenticated session identity.
    #[must_use]
    pub fn new(session_id: SessionId, compatibility: Compatibility) -> Self {
        Self {
            session_id,
            compatibility,
            inner: Mutex::new(Inner {
                state: ConsentState::Disabled,
                epoch: 0,
                next_public_sequence: 0,
                dropped_public_events: 0,
                queue: EventQueue::new(),
                controls: VecDeque::new(),
                lease: None,
                next_lease_id: 0,
            }),
            available: Condvar::new(),
        }
    }

    /// Opens the human side and establishes an awaiting-guest epoch.
    pub fn begin_awaiting(&self) -> Result<u64, TransitionError> {
        let mut inner = self.lock();
        require_state(&inner, ConsentState::Disabled)?;
        advance(&mut inner, ConsentState::AwaitingGuest)
    }

    /// Records successful guest authentication and establishes a live epoch.
    pub fn allow(&self) -> Result<u64, TransitionError> {
        let mut inner = self.lock();
        require_state(&inner, ConsentState::AwaitingGuest)?;
        advance(&mut inner, ConsentState::Live)
    }

    /// Captures the current live epoch for one allowlisted public call.
    #[must_use]
    pub fn capture(&self) -> Option<ConsentTicket> {
        let inner = self.lock();
        (inner.state == ConsentState::Live).then_some(ConsentTicket { epoch: inner.epoch })
    }

    /// Rechecks consent, assigns a sequence, and enqueues as one operation.
    pub fn commit(
        &self,
        ticket: ConsentTicket,
        event: PreparedEvent,
    ) -> Result<CommitOutcome, TransitionError> {
        let mut inner = self.lock();
        if inner.state != ConsentState::Live || inner.epoch != ticket.epoch {
            return Ok(CommitOutcome::Discarded);
        }
        let sequence = inner.next_public_sequence;
        let worst_gap = SequenceRange {
            first: u64::MAX,
            last: u64::MAX,
        };
        let reserved = self.serialize_event(ticket.epoch, sequence, Some(worst_gap), &event);
        let reserved_bytes = match reserved {
            Ok(frame) => frame.len(),
            Err(FrameError::TooLarge { .. } | FrameError::TooDeep { .. }) => {
                return consume_projection_drop(&mut inner);
            }
            Err(_) => return Err(TransitionError::InvalidProjection),
        };
        let next_sequence = sequence
            .checked_add(1)
            .ok_or(TransitionError::SequenceExhausted)?;
        let outcome = inner.queue.push(StoredEvent {
            sequence,
            epoch: ticket.epoch,
            reserved_bytes,
            payload: event,
        });
        inner.next_public_sequence = next_sequence;
        add_drops(&mut inner, outcome.dropped)?;
        let result = CommitOutcome::Queued {
            sequence,
            dropped: outcome.dropped,
        };
        drop(inner);
        self.available.notify_one();
        Ok(result)
    }

    /// Serializes one public projection once, then atomically commits or drops it.
    ///
    /// An oversized or over-deep public value still consumes one sequence so a
    /// later viewer frame carries an exact gap instead of hiding pressure.
    pub fn prepare_and_commit<T>(
        &self,
        ticket: ConsentTicket,
        event: &T,
    ) -> Result<CommitOutcome, TransitionError>
    where
        T: Serialize,
    {
        match PreparedEvent::new(event) {
            Ok(prepared) => self.commit(ticket, prepared),
            Err(FrameError::TooLarge { .. } | FrameError::TooDeep { .. }) => {
                let mut inner = self.lock();
                if inner.state != ConsentState::Live || inner.epoch != ticket.epoch {
                    return Ok(CommitOutcome::Discarded);
                }
                consume_projection_drop(&mut inner)
            }
            Err(_) => Err(TransitionError::InvalidProjection),
        }
    }

    /// Pauses emission after any frame already in its bounded write call.
    pub fn pause(&self) -> Result<u64, TransitionError> {
        self.transition_with_marker(
            ConsentState::Live,
            ConsentState::Paused,
            ControlMarker::Paused,
        )
    }

    /// Resumes emission under a new epoch and queues an ordered marker.
    pub fn resume(&self) -> Result<u64, TransitionError> {
        self.transition_with_marker(
            ConsentState::Paused,
            ConsentState::Live,
            ControlMarker::Resumed,
        )
    }

    /// Stops the session, clears pending content, and queues a final marker.
    ///
    /// Repeated stop calls are idempotent and return the existing stopped epoch.
    pub fn stop(&self) -> Result<u64, TransitionError> {
        let mut inner = self.lock_after_write();
        if inner.state == ConsentState::Stopped {
            return Ok(inner.epoch);
        }
        self.cancel_unwritten_lease(&mut inner)?;
        let dropped = inner.queue.clear();
        add_drops(&mut inner, dropped)?;
        inner.controls.clear();
        let epoch = advance(&mut inner, ConsentState::Stopped)?;
        self.push_control(&mut inner, epoch, ControlMarker::Stopped)?;
        drop(inner);
        self.available.notify_all();
        Ok(epoch)
    }

    /// Takes the next ordered frame without waiting.
    pub fn lease(&self) -> Result<Option<EventLease<'_>>, TransitionError> {
        self.lease_inner(self.lock())
    }

    /// Waits up to `timeout` for the next frame or final stop.
    pub fn wait_lease(&self, timeout: Duration) -> Result<Option<EventLease<'_>>, TransitionError> {
        let inner = self.lock();
        let inner = self
            .available
            .wait_timeout_while(inner, timeout, |state| {
                state.lease.is_some()
                    || (state.state != ConsentState::Stopped
                        && state.controls.is_empty()
                        && (state.state != ConsentState::Live
                            || state.queue.status().queued_events == 0))
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .0;
        self.lease_inner(inner)
    }

    /// Returns state, epoch, sequencing, drops, and bounded queue occupancy.
    #[must_use]
    pub fn status(&self) -> ConsentStatus {
        let inner = self.lock();
        ConsentStatus {
            state: inner.state,
            epoch: inner.epoch,
            next_public_sequence: inner.next_public_sequence,
            dropped_public_events: inner.dropped_public_events,
            queue: inner.queue.status(),
            pending_controls: inner.controls.len(),
            writer_active: inner.lease.is_some_and(|lease| lease.writing),
        }
    }

    fn transition_with_marker(
        &self,
        expected: ConsentState,
        target: ConsentState,
        marker: ControlMarker,
    ) -> Result<u64, TransitionError> {
        let mut inner = self.lock_after_write();
        require_state(&inner, expected)?;
        if inner.controls.len() == MAX_PENDING_CONTROLS {
            return Err(TransitionError::ControlBackpressure);
        }
        self.cancel_unwritten_event(&mut inner)?;
        let dropped = inner.queue.clear();
        add_drops(&mut inner, dropped)?;
        let epoch = advance(&mut inner, target)?;
        self.push_control(&mut inner, epoch, marker)?;
        drop(inner);
        self.available.notify_all();
        Ok(epoch)
    }

    fn push_control(
        &self,
        inner: &mut Inner,
        epoch: u64,
        marker: ControlMarker,
    ) -> Result<(), TransitionError> {
        let message = WireMessage::<&serde_json::value::RawValue>::Control {
            session_id: self.session_id,
            consent_epoch: epoch,
            marker,
        };
        let frame = serialize_bounded(&message, MAX_EVENT_BYTES)
            .map_err(|_| TransitionError::InvalidProjection)?;
        inner.controls.push_back(ControlEntry { frame });
        Ok(())
    }

    fn lease_inner<'a>(
        &'a self,
        mut inner: MutexGuard<'a, Inner>,
    ) -> Result<Option<EventLease<'a>>, TransitionError> {
        if inner.lease.is_some() {
            return Ok(None);
        }
        let id = inner.next_lease_id;
        let next_lease_id = id.checked_add(1).ok_or(TransitionError::LeaseExhausted)?;
        let item = if let Some(control) = inner.controls.pop_front() {
            LeaseItem::Control(control)
        } else if inner.state == ConsentState::Live {
            let Some(event) = inner.queue.take() else {
                return Ok(None);
            };
            LeaseItem::Event(event)
        } else {
            return Ok(None);
        };
        let frame = match &item {
            LeaseItem::Control(control) => control.frame.clone(),
            LeaseItem::Event(taken) => match self.serialize_event(
                taken.event.epoch,
                taken.event.sequence,
                taken.skipped,
                &taken.event.payload,
            ) {
                Ok(frame) => frame,
                Err(_) => {
                    if let LeaseItem::Event(taken) = item {
                        let dropped = inner.queue.restore_front(taken);
                        add_drops(&mut inner, dropped)?;
                    }
                    return Err(TransitionError::InvalidProjection);
                }
            },
        };
        inner.next_lease_id = next_lease_id;
        let kind = match &item {
            LeaseItem::Event(taken) => LeaseKind::Event {
                sequence: taken.event.sequence,
                epoch: taken.event.epoch,
                skipped: taken.skipped,
            },
            LeaseItem::Control(_) => LeaseKind::Control,
        };
        inner.lease = Some(LeaseRecord {
            id,
            kind,
            writing: false,
        });
        drop(inner);
        Ok(Some(EventLease {
            machine: self,
            id,
            frame,
            item: Some(item),
            complete: false,
        }))
    }

    fn serialize_event(
        &self,
        epoch: u64,
        sequence: u64,
        skipped: Option<SequenceRange>,
        event: &PreparedEvent,
    ) -> Result<Vec<u8>, FrameError> {
        let message = WireMessage::Event(EventEnvelope {
            session_id: self.session_id,
            consent_epoch: epoch,
            public_sequence: sequence,
            skipped,
            compatibility: self.compatibility.clone(),
            event: event.raw(),
        });
        serialize_bounded(&message, MAX_EVENT_BYTES)
    }

    fn cancel_unwritten_event(&self, inner: &mut Inner) -> Result<(), TransitionError> {
        let Some(lease) = inner.lease else {
            return Ok(());
        };
        let LeaseKind::Event {
            sequence, skipped, ..
        } = lease.kind
        else {
            return Ok(());
        };
        debug_assert!(!lease.writing);
        if let Some(skipped) = skipped {
            inner.queue.record_range_for_session(skipped);
        }
        inner.queue.record_drop(sequence);
        add_drops(inner, 1)?;
        inner.lease = None;
        Ok(())
    }

    fn cancel_unwritten_lease(&self, inner: &mut Inner) -> Result<(), TransitionError> {
        let Some(lease) = inner.lease else {
            return Ok(());
        };
        debug_assert!(!lease.writing);
        if let LeaseKind::Event {
            sequence, skipped, ..
        } = lease.kind
        {
            if let Some(skipped) = skipped {
                inner.queue.record_range_for_session(skipped);
            }
            inner.queue.record_drop(sequence);
            add_drops(inner, 1)?;
        }
        inner.lease = None;
        Ok(())
    }

    fn begin_write(&self, id: u64) -> Result<(), WriteError> {
        let mut inner = self.lock();
        let Some(lease) = inner.lease else {
            return Err(WriteError::Revoked);
        };
        if lease.id != id {
            return Err(WriteError::Revoked);
        }
        if let LeaseKind::Event { epoch, .. } = lease.kind
            && (inner.state != ConsentState::Live || inner.epoch != epoch)
        {
            inner.lease = None;
            return Err(WriteError::Revoked);
        }
        if let Some(lease) = inner.lease.as_mut() {
            lease.writing = true;
        }
        Ok(())
    }

    fn acknowledge_write(&self, id: u64) {
        let mut inner = self.lock();
        if inner.lease.is_some_and(|lease| lease.id == id) {
            inner.lease = None;
        }
        drop(inner);
        self.available.notify_all();
    }

    fn fail_write(&self, id: u64, item: LeaseItem) {
        let mut inner = self.lock();
        if inner.lease.is_some_and(|lease| lease.id == id) {
            terminate_failed_write(&mut inner, item);
        }
        drop(inner);
        self.available.notify_all();
    }

    fn abandon(&self, id: u64, item: LeaseItem) {
        let mut inner = self.lock();
        let Some(lease) = inner.lease.filter(|lease| lease.id == id) else {
            return;
        };
        if lease.writing {
            terminate_failed_write(&mut inner, item);
            drop(inner);
            self.available.notify_all();
            return;
        }
        inner.lease = None;
        match item {
            LeaseItem::Control(control) => inner.controls.push_front(control),
            LeaseItem::Event(taken) => {
                let dropped = inner.queue.restore_front(taken);
                inner.dropped_public_events = inner.dropped_public_events.saturating_add(dropped);
            }
        }
        drop(inner);
        self.available.notify_all();
    }

    fn lock_after_write(&self) -> MutexGuard<'_, Inner> {
        let inner = self.lock();
        self.available
            .wait_while(inner, |state| {
                state.lease.is_some_and(|lease| lease.writing)
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn consume_projection_drop(inner: &mut Inner) -> Result<CommitOutcome, TransitionError> {
    let sequence = inner.next_public_sequence;
    inner.next_public_sequence = sequence
        .checked_add(1)
        .ok_or(TransitionError::SequenceExhausted)?;
    inner.queue.record_drop(sequence);
    add_drops(inner, 1)?;
    Ok(CommitOutcome::Dropped { sequence })
}

fn terminate_failed_write(inner: &mut Inner, item: LeaseItem) {
    if let LeaseItem::Event(taken) = item {
        if let Some(skipped) = taken.skipped {
            inner.queue.record_range_for_session(skipped);
        }
        inner.queue.record_drop(taken.event.sequence);
        inner.dropped_public_events = inner.dropped_public_events.saturating_add(1);
    }
    let dropped = inner.queue.clear();
    inner.dropped_public_events = inner.dropped_public_events.saturating_add(dropped);
    inner.controls.clear();
    inner.lease = None;
    if inner.state != ConsentState::Stopped {
        inner.epoch = inner.epoch.saturating_add(1);
        inner.state = ConsentState::Stopped;
    }
}

#[derive(Debug)]
enum LeaseItem {
    Event(TakenEvent),
    Control(ControlEntry),
}

/// One ordered frame recoverable until its destination write begins.
#[derive(Debug)]
pub struct EventLease<'a> {
    machine: &'a ConsentMachine,
    id: u64,
    frame: Vec<u8>,
    item: Option<LeaseItem>,
    complete: bool,
}

impl EventLease<'_> {
    /// Returns the validated JSON frame bytes without the newline delimiter.
    #[must_use]
    pub fn frame(&self) -> &[u8] {
        &self.frame
    }

    /// Writes and acknowledges this frame under the consent revocation barrier.
    pub fn write_to<W>(mut self, writer: &mut W) -> Result<(), WriteError>
    where
        W: Write,
    {
        self.machine.begin_write(self.id)?;
        let result = writer
            .write_all(&self.frame)
            .and_then(|()| writer.write_all(b"\n"))
            .and_then(|()| writer.flush());
        if let Err(error) = result {
            if let Some(item) = self.item.take() {
                self.machine.fail_write(self.id, item);
            }
            self.complete = true;
            return Err(WriteError::Io(error));
        }
        self.machine.acknowledge_write(self.id);
        self.item = None;
        self.complete = true;
        Ok(())
    }
}

impl Drop for EventLease<'_> {
    fn drop(&mut self) {
        if self.complete {
            return;
        }
        if let Some(item) = self.item.take() {
            self.machine.abandon(self.id, item);
        }
    }
}

/// A bounded snapshot returned by the public status control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsentStatus {
    /// Current consent state.
    pub state: ConsentState,
    /// Current consent epoch.
    pub epoch: u64,
    /// Sequence that the next successfully committed public event receives.
    pub next_public_sequence: u64,
    /// Cumulative count of public events dropped by backpressure or revocation.
    pub dropped_public_events: u64,
    /// Current public event queue occupancy.
    pub queue: EventQueueStatus,
    /// Ordered content-free markers awaiting transmission.
    pub pending_controls: usize,
    /// Whether one frame is inside its bounded write call.
    pub writer_active: bool,
}

fn require_state(inner: &Inner, expected: ConsentState) -> Result<(), TransitionError> {
    if inner.state == expected {
        Ok(())
    } else {
        Err(TransitionError::InvalidState {
            expected,
            actual: inner.state,
        })
    }
}

fn advance(inner: &mut Inner, state: ConsentState) -> Result<u64, TransitionError> {
    let epoch = inner
        .epoch
        .checked_add(1)
        .ok_or(TransitionError::EpochExhausted)?;
    inner.epoch = epoch;
    inner.state = state;
    Ok(epoch)
}

fn add_drops(inner: &mut Inner, count: u64) -> Result<(), TransitionError> {
    inner.dropped_public_events = inner
        .dropped_public_events
        .checked_add(count)
        .ok_or(TransitionError::DropCountExhausted)?;
    Ok(())
}

/// Failure to perform a consent, sequencing, or queue transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    /// The transition is invalid from the current state.
    InvalidState {
        /// State required by the transition.
        expected: ConsentState,
        /// State observed at the transition boundary.
        actual: ConsentState,
    },
    /// The consent epoch cannot advance without wrapping.
    EpochExhausted,
    /// The public sequence cannot advance without wrapping.
    SequenceExhausted,
    /// The cumulative public-drop count cannot advance without wrapping.
    DropCountExhausted,
    /// The internal lease identity cannot advance without wrapping.
    LeaseExhausted,
    /// Too many unsent consent markers are already pending.
    ControlBackpressure,
    /// A prepared projection could not form a valid bounded wire frame.
    InvalidProjection,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected:?} consent state, found {actual:?}"
                )
            }
            Self::EpochExhausted => formatter.write_str("consent epoch exhausted"),
            Self::SequenceExhausted => formatter.write_str("public sequence exhausted"),
            Self::DropCountExhausted => formatter.write_str("public drop count exhausted"),
            Self::LeaseExhausted => formatter.write_str("writer lease identity exhausted"),
            Self::ControlBackpressure => formatter.write_str("consent marker queue is full"),
            Self::InvalidProjection => formatter.write_str("invalid public projection"),
        }
    }
}

impl Error for TransitionError {}

/// Failure to enter or complete one consent-guarded frame write.
#[derive(Debug)]
pub enum WriteError {
    /// Consent changed before this frame entered its write call.
    Revoked,
    /// The bounded destination write failed.
    Io(io::Error),
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Revoked => formatter.write_str("broadcast frame was revoked"),
            Self::Io(_) => formatter.write_str("broadcast frame write failed"),
        }
    }
}

impl Error for WriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Revoked => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommitOutcome, ConsentMachine, ConsentState, MAX_PENDING_CONTROLS, TransitionError,
        WriteError, add_drops,
    };
    use crate::{Compatibility, PreparedEvent, SessionId, WireMessage};
    use std::error::Error;
    use std::io::{self, Write};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn machine() -> ConsentMachine {
        let compatibility = Compatibility::from_catalogs(["life"], ["lorenz"], ["munch"])
            .expect("valid compatibility");
        ConsentMachine::new(SessionId::from_bytes([3; 16]), compatibility)
    }

    fn live() -> ConsentMachine {
        let machine = machine();
        machine.begin_awaiting().expect("await guest");
        machine.allow().expect("allow guest");
        machine
    }

    #[test]
    fn valid_lifecycle_serializes_markers_and_public_events() {
        let machine = live();
        let ticket = machine.capture().expect("live ticket");
        assert_eq!(
            machine.commit(ticket, PreparedEvent::new(&42).expect("prepare")),
            Ok(CommitOutcome::Queued {
                sequence: 0,
                dropped: 0
            })
        );
        let event = machine.lease().expect("lease result").expect("event");
        let decoded: WireMessage<serde_json::Value> =
            serde_json::from_slice(event.frame()).expect("event frame");
        assert!(matches!(decoded, WireMessage::Event(_)));
        event.write_to(&mut Vec::new()).expect("write event");

        assert_eq!(machine.pause(), Ok(3));
        let paused = machine
            .lease()
            .expect("lease result")
            .expect("pause marker");
        paused.write_to(&mut Vec::new()).expect("write pause");
        assert_eq!(machine.resume(), Ok(4));
        let resumed = machine
            .lease()
            .expect("lease result")
            .expect("resume marker");
        resumed.write_to(&mut Vec::new()).expect("write resume");
        assert_eq!(machine.stop(), Ok(5));
        assert_eq!(machine.stop(), Ok(5));
        assert_eq!(machine.status().state, ConsentState::Stopped);
    }

    #[test]
    fn a_call_spanning_pause_is_discarded_without_a_sequence_gap() {
        let machine = live();
        let stale = machine.capture().expect("live ticket");
        machine.pause().expect("pause");
        assert_eq!(
            machine.commit(stale, PreparedEvent::new(&1).expect("prepare")),
            Ok(CommitOutcome::Discarded)
        );
        assert_eq!(machine.status().next_public_sequence, 0);
    }

    #[test]
    fn concurrent_commit_order_is_the_only_sequence_order() {
        let machine = live();
        let first_ticket = machine.capture().expect("first ticket");
        let second_ticket = machine.capture().expect("second ticket");
        thread::scope(|scope| {
            let second = scope.spawn(|| {
                machine.commit(
                    second_ticket,
                    PreparedEvent::new(&"second").expect("prepare second"),
                )
            });
            let first = scope.spawn(|| {
                thread::yield_now();
                machine.commit(
                    first_ticket,
                    PreparedEvent::new(&"first").expect("prepare first"),
                )
            });
            let outcomes = [
                first.join().expect("first producer").expect("first commit"),
                second
                    .join()
                    .expect("second producer")
                    .expect("second commit"),
            ];
            assert!(outcomes.contains(&CommitOutcome::Queued {
                sequence: 0,
                dropped: 0
            }));
            assert!(outcomes.contains(&CommitOutcome::Queued {
                sequence: 1,
                dropped: 0
            }));
        });
        assert_eq!(machine.status().queue.queued_events, 2);
    }

    #[test]
    fn abandoning_a_lease_restores_its_event_and_gap() {
        let machine = live();
        for value in 0..66 {
            let ticket = machine.capture().expect("ticket");
            machine
                .commit(ticket, PreparedEvent::new(&value).expect("prepare"))
                .expect("commit");
        }
        let first = machine.lease().expect("lease result").expect("first lease");
        let frame = first.frame().to_vec();
        drop(first);
        let restored = machine
            .lease()
            .expect("lease result")
            .expect("restored lease");
        assert_eq!(restored.frame(), frame);
    }

    #[test]
    fn abandoning_a_lease_never_exceeds_the_event_bound() {
        let machine = live();
        for value in 0..crate::MAX_QUEUED_EVENTS {
            let ticket = machine.capture().expect("ticket");
            machine
                .commit(ticket, PreparedEvent::new(&value).expect("prepare"))
                .expect("commit");
        }
        let oldest = machine.lease().expect("lease result").expect("oldest");
        let ticket = machine.capture().expect("ticket");
        machine
            .commit(ticket, PreparedEvent::new(&64).expect("prepare"))
            .expect("commit");
        assert_eq!(
            machine.status().queue.queued_events,
            crate::MAX_QUEUED_EVENTS
        );
        drop(oldest);
        let status = machine.status();
        assert_eq!(status.queue.queued_events, crate::MAX_QUEUED_EVENTS);
        assert!(status.queue.queued_bytes <= crate::MAX_QUEUED_BYTES);
        assert_eq!(status.dropped_public_events, 1);
    }

    #[test]
    fn pause_cancels_an_unwritten_lease_and_reports_its_gap_after_resume() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        machine
            .commit(ticket, PreparedEvent::new(&"old").expect("prepare"))
            .expect("commit");
        let stale = machine.lease().expect("lease result").expect("stale lease");
        machine.pause().expect("pause");
        assert!(matches!(
            stale.write_to(&mut Vec::new()),
            Err(WriteError::Revoked)
        ));
        machine
            .lease()
            .expect("lease result")
            .expect("pause marker")
            .write_to(&mut Vec::new())
            .expect("write pause");
        machine.resume().expect("resume");
        machine
            .lease()
            .expect("lease result")
            .expect("resume marker")
            .write_to(&mut Vec::new())
            .expect("write resume");
        let ticket = machine.capture().expect("fresh ticket");
        machine
            .commit(ticket, PreparedEvent::new(&"fresh").expect("prepare"))
            .expect("commit");
        let fresh = machine.lease().expect("lease result").expect("fresh event");
        let message: WireMessage<serde_json::Value> =
            serde_json::from_slice(fresh.frame()).expect("decode fresh event");
        let WireMessage::Event(envelope) = message else {
            panic!("expected event");
        };
        assert_eq!(envelope.skipped, crate::SequenceRange::new(0, 0));
    }

    struct BlockingWriter {
        entered: mpsc::SyncSender<()>,
        release: mpsc::Receiver<()>,
        blocked: bool,
    }

    impl Write for BlockingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if !self.blocked {
                self.blocked = true;
                self.entered.send(()).expect("signal writer entry");
                self.release.recv().expect("release writer");
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn pause_waits_for_a_write_that_already_crossed_the_barrier() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        machine
            .commit(ticket, PreparedEvent::new(&1).expect("prepare"))
            .expect("commit");
        let lease = machine.lease().expect("lease result").expect("event lease");
        let (entered_tx, entered_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let (paused_tx, paused_rx) = mpsc::sync_channel(0);
        thread::scope(|scope| {
            scope.spawn(move || {
                lease
                    .write_to(&mut BlockingWriter {
                        entered: entered_tx,
                        release: release_rx,
                        blocked: false,
                    })
                    .expect("bounded write");
            });
            entered_rx.recv().expect("writer entered");
            scope.spawn(|| {
                machine.pause().expect("pause");
                paused_tx.send(()).expect("signal pause");
            });
            assert!(paused_rx.recv_timeout(Duration::from_millis(20)).is_err());
            release_tx.send(()).expect("release writer");
            paused_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("pause completed");
        });
    }

    #[test]
    fn invalid_transitions_fail_without_changing_the_epoch() {
        let machine = machine();
        assert_eq!(
            machine.allow(),
            Err(TransitionError::InvalidState {
                expected: ConsentState::AwaitingGuest,
                actual: ConsentState::Disabled,
            })
        );
        assert_eq!(machine.status().epoch, 0);
        assert!(machine.capture().is_none());
    }

    #[test]
    fn complete_envelope_bounds_consume_an_exact_drop_sequence() {
        let machine = live();
        let payload = "x".repeat(crate::MAX_EVENT_BYTES - 2);
        let prepared = PreparedEvent::new(&payload).expect("payload alone fits");
        let ticket = machine.capture().expect("ticket");
        assert_eq!(
            machine.commit(ticket, prepared),
            Ok(CommitOutcome::Dropped { sequence: 0 })
        );
        let status = machine.status();
        assert_eq!(status.next_public_sequence, 1);
        assert_eq!(status.dropped_public_events, 1);
        assert_eq!(status.queue.pending_skip_ranges, 1);
    }

    #[test]
    fn oversized_prepared_projection_consumes_a_visible_drop_sequence() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        let oversized = "x".repeat(crate::MAX_EVENT_BYTES + 1);
        assert_eq!(
            machine.prepare_and_commit(ticket, &oversized),
            Ok(CommitOutcome::Dropped { sequence: 0 })
        );
        let status = machine.status();
        assert_eq!(status.next_public_sequence, 1);
        assert_eq!(status.dropped_public_events, 1);
        assert_eq!(status.queue.pending_skip_ranges, 1);
    }

    #[test]
    fn revoked_projection_preparation_does_not_consume_a_sequence() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        machine.pause().expect("pause");
        let oversized = "x".repeat(crate::MAX_EVENT_BYTES + 1);
        assert_eq!(
            machine.prepare_and_commit(ticket, &oversized),
            Ok(CommitOutcome::Discarded)
        );
        assert_eq!(machine.status().next_public_sequence, 0);
    }

    #[test]
    fn queue_pressure_is_nonblocking_and_cumulatively_accounted() {
        let machine = live();
        for value in 0..=crate::MAX_QUEUED_EVENTS {
            let ticket = machine.capture().expect("ticket");
            machine
                .commit(ticket, PreparedEvent::new(&value).expect("prepare"))
                .expect("commit");
        }
        let status = machine.status();
        assert_eq!(status.queue.queued_events, crate::MAX_QUEUED_EVENTS);
        assert_eq!(status.dropped_public_events, 1);
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
    fn failed_writes_stop_and_clear_the_session_without_retry() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        machine
            .commit(ticket, PreparedEvent::new(&7).expect("prepare"))
            .expect("commit");
        let lease = machine.lease().expect("lease result").expect("lease");
        let error = lease
            .write_to(&mut FailingWriter)
            .expect_err("write must fail");
        assert_eq!(error.to_string(), "broadcast frame write failed");
        assert!(error.source().is_some());
        let status = machine.status();
        assert_eq!(status.state, ConsentState::Stopped);
        assert_eq!(status.dropped_public_events, 1);
        assert!(machine.lease().expect("lease result").is_none());
    }

    struct PartialWriter {
        bytes: Vec<u8>,
        failed: bool,
    }

    impl Write for PartialWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.failed {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"));
            }
            self.failed = true;
            let count = bytes.len().min(8);
            self.bytes.extend_from_slice(&bytes[..count]);
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn partial_write_failure_can_never_prefix_a_retried_frame() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        machine
            .commit(ticket, PreparedEvent::new(&7).expect("prepare"))
            .expect("commit");
        let lease = machine.lease().expect("lease result").expect("lease");
        let mut writer = PartialWriter {
            bytes: Vec::new(),
            failed: false,
        };
        assert!(matches!(
            lease.write_to(&mut writer),
            Err(WriteError::Io(_))
        ));
        assert_eq!(writer.bytes.len(), 8);
        assert_eq!(machine.status().state, ConsentState::Stopped);
        assert!(machine.lease().expect("lease result").is_none());
    }

    struct PanickingWriter;

    impl Write for PanickingWriter {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            panic!("intentional writer panic fixture")
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn writer_unwind_uses_the_same_terminal_cleanup_as_an_io_error() {
        let machine = live();
        let ticket = machine.capture().expect("ticket");
        machine
            .commit(ticket, PreparedEvent::new(&7).expect("prepare"))
            .expect("commit");
        let lease = machine.lease().expect("lease result").expect("lease");
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = lease.write_to(&mut PanickingWriter);
        }));
        assert!(unwound.is_err());
        let status = machine.status();
        assert_eq!(status.state, ConsentState::Stopped);
        assert_eq!(status.dropped_public_events, 1);
        assert!(machine.lease().expect("lease result").is_none());
    }

    #[test]
    fn wait_and_control_leases_are_bounded_and_recoverable() {
        let machine = live();
        assert!(
            machine
                .wait_lease(Duration::from_millis(1))
                .expect("empty wait")
                .is_none()
        );
        machine.pause().expect("pause");
        let marker = machine
            .wait_lease(Duration::from_secs(1))
            .expect("marker wait")
            .expect("marker");
        let bytes = marker.frame().to_vec();
        assert_eq!(machine.status().pending_controls, 0);
        drop(marker);
        let restored = machine
            .lease()
            .expect("lease result")
            .expect("restored marker");
        assert_eq!(restored.frame(), bytes);
        restored.write_to(&mut Vec::new()).expect("write marker");
        assert!(machine.lease().expect("lease result").is_none());
    }

    #[test]
    fn a_leased_control_remains_ordered_across_the_next_transition() {
        let machine = live();
        machine.pause().expect("pause");
        let paused = machine
            .lease()
            .expect("lease result")
            .expect("pause marker");
        machine.resume().expect("resume");
        assert!(machine.lease().expect("lease result").is_none());
        paused
            .write_to(&mut Vec::new())
            .expect("write pause marker");
        machine
            .lease()
            .expect("lease result")
            .expect("resume marker")
            .write_to(&mut Vec::new())
            .expect("write resume marker");
    }

    #[test]
    fn pending_control_markers_have_a_fixed_backpressure_limit() {
        let machine = live();
        for _ in 0..(MAX_PENDING_CONTROLS / 2) {
            machine.pause().expect("pause");
            machine.resume().expect("resume");
        }
        assert_eq!(machine.status().pending_controls, MAX_PENDING_CONTROLS);
        assert_eq!(machine.pause(), Err(TransitionError::ControlBackpressure));
        machine.stop().expect("stop clears pending controls");
        assert_eq!(machine.status().pending_controls, 1);
    }

    #[test]
    fn stop_clears_queued_and_unwritten_content_before_the_final_marker() {
        let machine = live();
        for value in 0..2 {
            let ticket = machine.capture().expect("ticket");
            machine
                .commit(ticket, PreparedEvent::new(&value).expect("prepare"))
                .expect("commit");
        }
        let leased = machine
            .lease()
            .expect("lease result")
            .expect("leased event");
        machine.stop().expect("stop");
        assert!(matches!(
            leased.write_to(&mut Vec::new()),
            Err(WriteError::Revoked)
        ));
        let status = machine.status();
        assert_eq!(status.dropped_public_events, 2);
        assert_eq!(status.queue.queued_events, 0);
        machine
            .lease()
            .expect("lease result")
            .expect("stop marker")
            .write_to(&mut Vec::new())
            .expect("write stop marker");
    }

    #[test]
    fn stop_revokes_unwritten_controls_and_final_wait_returns_immediately() {
        let machine = live();
        machine.pause().expect("pause");
        let stale = machine
            .lease()
            .expect("lease result")
            .expect("pause marker");
        machine.stop().expect("stop");
        assert!(matches!(
            stale.write_to(&mut Vec::new()),
            Err(WriteError::Revoked)
        ));
        machine
            .lease()
            .expect("lease result")
            .expect("stop marker")
            .write_to(&mut Vec::new())
            .expect("write stop marker");
        let started = std::time::Instant::now();
        assert!(
            machine
                .wait_lease(Duration::from_secs(5))
                .expect("final wait")
                .is_none()
        );
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn numeric_exhaustion_paths_fail_without_wrapping_or_loss() {
        let epoch = machine();
        epoch.lock().epoch = u64::MAX;
        assert_eq!(epoch.begin_awaiting(), Err(TransitionError::EpochExhausted));

        let sequence = live();
        sequence.lock().next_public_sequence = u64::MAX;
        let ticket = sequence.capture().expect("ticket");
        assert_eq!(
            sequence.commit(ticket, PreparedEvent::new(&1).expect("prepare")),
            Err(TransitionError::SequenceExhausted)
        );

        let drops = machine();
        let mut inner = drops.lock();
        inner.dropped_public_events = u64::MAX;
        assert_eq!(
            add_drops(&mut inner, 1),
            Err(TransitionError::DropCountExhausted)
        );
        drop(inner);

        let leases = live();
        let ticket = leases.capture().expect("ticket");
        leases
            .commit(ticket, PreparedEvent::new(&1).expect("prepare"))
            .expect("commit");
        leases.lock().next_lease_id = u64::MAX;
        assert!(matches!(
            leases.lease(),
            Err(TransitionError::LeaseExhausted)
        ));
        assert_eq!(leases.status().queue.queued_events, 1);
    }

    #[test]
    fn error_messages_cover_every_stable_public_category() {
        let errors = [
            (TransitionError::EpochExhausted, "consent epoch exhausted"),
            (
                TransitionError::SequenceExhausted,
                "public sequence exhausted",
            ),
            (
                TransitionError::DropCountExhausted,
                "public drop count exhausted",
            ),
            (
                TransitionError::LeaseExhausted,
                "writer lease identity exhausted",
            ),
            (
                TransitionError::ControlBackpressure,
                "consent marker queue is full",
            ),
            (
                TransitionError::InvalidProjection,
                "invalid public projection",
            ),
        ];
        for (error, expected) in errors {
            assert_eq!(error.to_string(), expected);
        }
        assert_eq!(
            WriteError::Revoked.to_string(),
            "broadcast frame was revoked"
        );
        let state = TransitionError::InvalidState {
            expected: ConsentState::Live,
            actual: ConsentState::Paused,
        };
        assert_eq!(
            state.to_string(),
            "expected Live consent state, found Paused"
        );
        assert!(WriteError::Revoked.source().is_none());
        assert!(TransitionError::EpochExhausted.source().is_none());
    }
}
