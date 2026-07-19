use crate::framing::{FrameError, serialize_bounded};
use crate::wire::{MAX_EVENT_BYTES, SequenceRange};
use serde::Serialize;
use serde_json::value::RawValue;
use std::collections::VecDeque;

/// Maximum number of public events waiting for the writer.
pub const MAX_QUEUED_EVENTS: usize = 64;
/// Maximum serialized bytes waiting for the writer.
pub const MAX_QUEUED_BYTES: usize = 4 * 1_024 * 1_024;

/// An immutable, bounded public payload prepared outside the consent lock.
///
/// Preparation serializes exactly once. Queue accounting and later wire
/// serialization therefore cannot be fooled by a changing serializer or a
/// caller-provided size estimate.
#[derive(Debug)]
pub struct PreparedEvent {
    value: Box<RawValue>,
}

impl PreparedEvent {
    /// Serializes and validates one public projection within the frame limit.
    pub fn new<T>(value: &T) -> Result<Self, FrameError>
    where
        T: Serialize,
    {
        let bytes = serialize_bounded(value, MAX_EVENT_BYTES)?;
        let json = String::from_utf8(bytes).map_err(|_| FrameError::InvalidJson)?;
        let value = RawValue::from_string(json).map_err(|_| FrameError::InvalidJson)?;
        Ok(Self { value })
    }

    pub(crate) fn raw(&self) -> &RawValue {
        &self.value
    }
}

#[derive(Debug)]
pub(crate) struct StoredEvent {
    pub(crate) sequence: u64,
    pub(crate) epoch: u64,
    pub(crate) reserved_bytes: usize,
    pub(crate) payload: PreparedEvent,
}

#[derive(Debug)]
pub(crate) struct TakenEvent {
    pub(crate) event: StoredEvent,
    pub(crate) skipped: Option<SequenceRange>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct QueuePushOutcome {
    pub(crate) dropped: u64,
}

/// Snapshot of bounded queue state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventQueueStatus {
    /// Events currently waiting for the writer.
    pub queued_events: usize,
    /// Conservatively reserved serialized bytes waiting for the writer.
    pub queued_bytes: usize,
    /// Canonical skipped ranges awaiting a later public event.
    pub pending_skip_ranges: usize,
}

#[derive(Debug)]
pub(crate) struct EventQueue {
    max_events: usize,
    max_bytes: usize,
    events: VecDeque<StoredEvent>,
    reserved_bytes: usize,
    pending_skips: VecDeque<SequenceRange>,
}

impl EventQueue {
    pub(crate) fn new() -> Self {
        Self::with_limits(MAX_QUEUED_EVENTS, MAX_QUEUED_BYTES)
    }

    fn with_limits(max_events: usize, max_bytes: usize) -> Self {
        assert!(max_events > 0, "queue event limit must be nonzero");
        assert!(max_bytes > 0, "queue byte limit must be nonzero");
        Self {
            max_events,
            max_bytes,
            events: VecDeque::with_capacity(max_events),
            reserved_bytes: 0,
            pending_skips: VecDeque::new(),
        }
    }

    pub(crate) fn push(&mut self, event: StoredEvent) -> QueuePushOutcome {
        let mut dropped = 0;
        while self.events.len() >= self.max_events
            || self.reserved_bytes.saturating_add(event.reserved_bytes) > self.max_bytes
        {
            let Some(oldest) = self.events.pop_front() else {
                break;
            };
            self.reserved_bytes -= oldest.reserved_bytes;
            self.record_drop(oldest.sequence);
            dropped += 1;
        }
        self.reserved_bytes += event.reserved_bytes;
        self.events.push_back(event);
        QueuePushOutcome { dropped }
    }

    pub(crate) fn take(&mut self) -> Option<TakenEvent> {
        let event = self.events.pop_front()?;
        self.reserved_bytes -= event.reserved_bytes;
        let skipped = self
            .pending_skips
            .front()
            .filter(|range| range.last < event.sequence)
            .copied();
        if skipped.is_some() {
            self.pending_skips.pop_front();
        }
        debug_assert!(
            self.pending_skips
                .front()
                .is_none_or(|range| range.first > event.sequence),
            "a retained event cannot precede an unreported older gap"
        );
        Some(TakenEvent { event, skipped })
    }

    pub(crate) fn restore_front(&mut self, taken: TakenEvent) -> u64 {
        if let Some(skipped) = taken.skipped {
            self.record_range(skipped);
        }
        let mut dropped = 0;
        while self.events.len() >= self.max_events
            || self
                .reserved_bytes
                .saturating_add(taken.event.reserved_bytes)
                > self.max_bytes
        {
            let Some(newest) = self.events.pop_back() else {
                break;
            };
            self.reserved_bytes -= newest.reserved_bytes;
            self.record_drop(newest.sequence);
            dropped += 1;
        }
        self.reserved_bytes += taken.event.reserved_bytes;
        self.events.push_front(taken.event);
        dropped
    }

    pub(crate) fn record_drop(&mut self, sequence: u64) {
        self.record_range(SequenceRange {
            first: sequence,
            last: sequence,
        });
    }

    pub(crate) fn record_range_for_session(&mut self, range: SequenceRange) {
        self.record_range(range);
    }

    fn record_range(&mut self, mut incoming: SequenceRange) {
        let mut index = 0;
        while index < self.pending_skips.len() {
            let current = self.pending_skips[index];
            if current.last.saturating_add(1) < incoming.first {
                index += 1;
                continue;
            }
            if incoming.last.saturating_add(1) < current.first {
                break;
            }
            incoming = incoming
                .merge(current)
                .expect("overlapping ranges must merge");
            self.pending_skips.remove(index);
        }
        self.pending_skips.insert(index, incoming);
        debug_assert!(self.pending_skips.len() <= self.events.len().saturating_add(1));
    }

    pub(crate) fn clear(&mut self) -> u64 {
        let mut dropped = 0;
        while let Some(event) = self.events.pop_front() {
            self.record_drop(event.sequence);
            dropped += 1;
        }
        self.reserved_bytes = 0;
        dropped
    }

    pub(crate) fn status(&self) -> EventQueueStatus {
        EventQueueStatus {
            queued_events: self.events.len(),
            queued_bytes: self.reserved_bytes,
            pending_skip_ranges: self.pending_skips.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventQueue, PreparedEvent, StoredEvent};
    use crate::SequenceRange;

    fn event(sequence: u64, reserved_bytes: usize) -> StoredEvent {
        StoredEvent {
            sequence,
            epoch: 1,
            reserved_bytes,
            payload: PreparedEvent::new(&sequence).expect("prepare event"),
        }
    }

    #[test]
    fn count_pressure_drops_oldest_and_reports_the_exact_gap() {
        let mut queue = EventQueue::with_limits(2, 100);
        queue.push(event(0, 10));
        queue.push(event(1, 10));
        assert_eq!(queue.push(event(2, 10)).dropped, 1);
        let first = queue.take().expect("first retained event");
        assert_eq!(first.event.sequence, 1);
        assert_eq!(first.skipped, SequenceRange::new(0, 0));
        assert_eq!(
            queue.take().expect("second retained event").event.sequence,
            2
        );
    }

    #[test]
    fn byte_pressure_drops_as_many_oldest_events_as_needed() {
        let mut queue = EventQueue::with_limits(8, 10);
        queue.push(event(0, 4));
        queue.push(event(1, 4));
        assert_eq!(queue.push(event(2, 7)).dropped, 2);
        let retained = queue.take().expect("retained event");
        assert_eq!(retained.event.sequence, 2);
        assert_eq!(retained.skipped, SequenceRange::new(0, 1));
        assert_eq!(queue.status().queued_bytes, 0);
    }

    #[test]
    fn future_gaps_never_attach_to_older_events() {
        let mut queue = EventQueue::with_limits(64, 1_000);
        for sequence in 0..64 {
            queue.push(event(sequence, 1));
        }
        queue.record_drop(64);
        queue.push(event(65, 1));
        let first = queue.take().expect("oldest retained event");
        assert_eq!(first.event.sequence, 1);
        assert_eq!(first.skipped, SequenceRange::new(0, 0));
        while queue.take().is_some() {}
        assert!(queue.status().pending_skip_ranges <= 1);
    }

    #[test]
    fn an_abandoned_lease_restores_its_event_and_gap() {
        let mut queue = EventQueue::with_limits(2, 100);
        queue.record_drop(0);
        queue.push(event(1, 10));
        let taken = queue.take().expect("taken event");
        assert_eq!(queue.restore_front(taken), 0);
        let restored = queue.take().expect("restored event");
        assert_eq!(restored.event.sequence, 1);
        assert_eq!(restored.skipped, SequenceRange::new(0, 0));
    }

    #[test]
    fn lease_restoration_preserves_byte_bounds_and_drops_newest() {
        let mut queue = EventQueue::with_limits(8, 10);
        queue.push(event(0, 6));
        queue.push(event(1, 4));
        let leased = queue.take().expect("lease oldest");
        queue.push(event(2, 6));
        assert_eq!(queue.restore_front(leased), 1);
        let status = queue.status();
        assert_eq!(status.queued_events, 2);
        assert_eq!(status.queued_bytes, 10);
        assert_eq!(queue.take().expect("restored oldest").event.sequence, 0);
        assert_eq!(queue.take().expect("retained middle").event.sequence, 1);
        queue.push(event(3, 1));
        let later = queue.take().expect("later event");
        assert_eq!(later.event.sequence, 3);
        assert_eq!(later.skipped, SequenceRange::new(2, 2));
    }
}
