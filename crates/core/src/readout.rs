//! Numeric room measurements, independent of status wording or field order.
//!
//! A room opts in through [`Room::numeric_readouts`]. Its channel IDs retain
//! their meaning across phases, variations, and compatible releases. Labels
//! are presentation only. Existing study grades use displayed decimal values,
//! so [`DisplayNumber`] gives the status and grader the same rounded number.

use std::fmt;

use crate::room::Room;

/// A stable numeric channel key within one room, not a vector position.
///
/// Never reuse an ID for a different quantity. Migrated channels keep their
/// former zero-based status-number index so existing goals remain compatible.
/// Discovery considers IDs in ascending order, independent of display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadoutId(usize);

impl ReadoutId {
    /// Declare a room-scoped channel key.
    #[must_use]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    /// Return the stable integer stored in existing goal and prediction fields.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// One finite measurement from a room's phase-only experiment.
///
/// The label names the quantity in the current presentation; it is not its
/// identity. The value must have the same units and rounding as the instrument
/// used for the corresponding study. A producer may omit an unavailable
/// channel, but must not replace it with another quantity under the same ID.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericReadout {
    id: ReadoutId,
    label: &'static str,
    value: f64,
}

impl NumericReadout {
    /// Construct a measurement, rejecting NaN and either infinity.
    #[must_use]
    pub fn new(id: ReadoutId, label: &'static str, value: f64) -> Option<Self> {
        value.is_finite().then_some(Self { id, label, value })
    }

    /// The stable, room-scoped identity of this quantity.
    #[must_use]
    pub const fn id(self) -> ReadoutId {
        self.id
    }

    /// Its display label, which may change without changing numeric identity.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.label
    }

    /// Its finite value in the instrument's units and precision.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.value
    }
}

/// A finite number quantized by its exact fixed-decimal display token.
///
/// This uses Rust's fixed-decimal formatting followed by decimal parsing, not
/// multiplication and arithmetic rounding. That distinction preserves ties,
/// signed zero, and the exact values formerly parsed from status text. The
/// `u8` precision bounds formatting work even for externally supplied inputs.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayNumber {
    text: String,
    value: f64,
}

impl DisplayNumber {
    /// Format with this many digits after the decimal point and parse the
    /// result back to a finite value. Non-finite inputs return `None`.
    #[must_use]
    pub fn fixed(value: f64, fractional_digits: u8) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        let digits = usize::from(fractional_digits);
        let text = format!("{value:.digits$}");
        let value = text.parse::<f64>().ok().filter(|value| value.is_finite())?;
        Some(Self { text, value })
    }

    /// The number represented by the display token, including signed zero.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Name this displayed number as a finite room measurement.
    #[must_use]
    pub fn readout(&self, id: ReadoutId, label: &'static str) -> NumericReadout {
        NumericReadout {
            id,
            label,
            value: self.value,
        }
    }
}

impl fmt::Display for DisplayNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// Preserve the established phase grid and seeded target mapping.
pub(crate) const PARAMETER_SAMPLES: usize = 64;

/// A selected channel and its values on the common discovery grid.
pub(crate) struct Readout {
    pub index: usize,
    pub label: String,
    pub span: (f64, f64),
    pub samples: Vec<f64>,
}

fn checked_channels(mut channels: Vec<NumericReadout>) -> Option<Vec<NumericReadout>> {
    channels.sort_unstable_by_key(|channel| channel.id);
    let finite = channels.iter().all(|channel| channel.value.is_finite());
    let unique = channels.windows(2).all(|pair| pair[0].id != pair[1].id);
    (finite && unique).then_some(channels)
}

fn channel_value(channels: &[NumericReadout], id: ReadoutId) -> Option<f64> {
    let index = channels
        .binary_search_by_key(&id, |channel| channel.id)
        .ok()?;
    Some(channels[index].value)
}

/// One lookup contract shared by point and local-curve grading. Phase zero
/// anchors provider support and channel identity; subsequent phases may not
/// switch between typed measurements and legacy status parsing.
pub(crate) struct ReadoutLookup<'a> {
    room: &'a dyn Room,
    initial: Option<Vec<NumericReadout>>,
}

impl<'a> ReadoutLookup<'a> {
    pub fn new(room: &'a dyn Room) -> Option<Self> {
        let initial = match room.numeric_readouts(0.0) {
            Some(channels) => Some(checked_channels(channels)?),
            None => None,
        };
        Some(Self { room, initial })
    }

    pub fn value(&self, index: usize, phase: f64) -> Option<f64> {
        // A channel absent at the discovery origin cannot be a posed channel.
        if let Some(initial) = &self.initial {
            let id = ReadoutId::new(index);
            let initial_value = channel_value(initial, id)?;
            if phase.to_bits() == 0.0_f64.to_bits() {
                return Some(initial_value);
            }
            let channels = checked_channels(self.room.numeric_readouts(phase)?)?;
            channel_value(&channels, id)
        } else {
            if self.room.numeric_readouts(phase).is_some() {
                return None;
            }
            let status = self.room.status(phase)?;
            Some(status_numbers(&status).get(index)?.1)
        }
    }
}

/// Choose the first moving stable ID, or use the unchanged status-column
/// contract for rooms which have not opted in. Mixed support is never a cue
/// to fall back to text. Typed candidates must exist at every sampled phase.
pub(crate) fn find_readout(room: &dyn Room) -> Option<Readout> {
    let Some(initial) = room.numeric_readouts(0.0) else {
        return find_legacy_readout(room);
    };
    let mut rows = Vec::with_capacity(PARAMETER_SAMPLES);
    rows.push(checked_channels(initial)?);
    for index in 1..PARAMETER_SAMPLES {
        let phase = index as f64 / PARAMETER_SAMPLES as f64;
        rows.push(checked_channels(room.numeric_readouts(phase)?)?);
    }
    rows[0].iter().find_map(|channel| {
        let samples: Vec<f64> = rows
            .iter()
            .map(|row| channel_value(row, channel.id))
            .collect::<Option<_>>()?;
        let lo = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let width = hi - lo;
        (width.is_finite() && width >= 1e-9).then(|| Readout {
            index: channel.id.get(),
            label: channel.label.to_string(),
            span: (lo, hi),
            samples,
        })
    })
}

/// Every legacy status number as (byte offset, value), from left to right.
/// Signs count only when a digit follows; exponent notation remains outside
/// this compatibility parser's historical contract.
pub(crate) fn status_numbers(status: &str) -> Vec<(usize, f64)> {
    let bytes = status.as_bytes();
    let mut numbers = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let c = bytes[start] as char;
        if c.is_ascii_digit()
            || ((c == '-' || c == '+')
                && bytes
                    .get(start + 1)
                    .is_some_and(|next| (*next as char).is_ascii_digit()))
        {
            let mut end = start + 1;
            while end < bytes.len() && ((bytes[end] as char).is_ascii_digit() || bytes[end] == b'.')
            {
                end += 1;
            }
            if let Ok(value) = status[start..end].parse() {
                numbers.push((start, value));
            }
            start = end;
        } else {
            start += 1;
        }
    }
    numbers
}

fn strip_leading_invite_tokens(prefix: &str) -> &str {
    let mut s = prefix.trim();
    while let Some(token) = s.split_whitespace().next() {
        let invite = token.split_once(':').is_some_and(|(verb, object)| {
            !verb.is_empty()
                && !object.is_empty()
                && verb.len() >= 2
                && object.len() >= 2
                && token
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == ':' || c == '-')
        });
        if !invite {
            break;
        }
        s = s[token.len()..].trim_start();
    }
    s
}

pub(crate) fn status_label(status: &str, cut: usize) -> String {
    let label = strip_leading_invite_tokens(status[..cut].trim())
        .trim_end_matches(['=', ':', ' '])
        .trim();
    if label.is_empty() {
        "READOUT".to_string()
    } else {
        label.to_string()
    }
}

fn readout_label(status: &str, numbers: &[(usize, f64)], index: usize) -> String {
    let precise = status_label(status, numbers[index].0);
    if precise.chars().any(|c| c.is_ascii_digit()) {
        status_label(status, numbers[0].0)
    } else {
        precise
    }
}

fn find_legacy_readout(room: &dyn Room) -> Option<Readout> {
    let mut statuses = Vec::with_capacity(PARAMETER_SAMPLES);
    for i in 0..PARAMETER_SAMPLES {
        let t = i as f64 / PARAMETER_SAMPLES as f64;
        if i != 0 && room.numeric_readouts(t).is_some() {
            return None;
        }
        let status = room.status(t)?;
        let numbers = status_numbers(&status);
        statuses.push((status, numbers));
    }
    let min_columns = statuses.iter().map(|(_, n)| n.len()).min().unwrap_or(0);
    let (index, lo, hi) = (0..min_columns).find_map(|index| {
        let name = readout_label(&statuses[0].0, &statuses[0].1, index);
        let aligned = statuses
            .iter()
            .all(|(s, n)| readout_label(s, n, index) == name);
        if !aligned {
            return None;
        }
        let column = statuses.iter().map(|(_, n)| n[index].1);
        let lo = column.clone().fold(f64::INFINITY, f64::min);
        let hi = column.fold(f64::NEG_INFINITY, f64::max);
        let moving = lo.is_finite() && hi.is_finite() && hi - lo >= 1e-9;
        moving.then_some((index, lo, hi))
    })?;
    let label = readout_label(&statuses[0].0, &statuses[0].1, index);
    let samples = statuses.iter().map(|(_, n)| n[index].1).collect();
    Some(Readout {
        index,
        label,
        span: (lo, hi),
        samples,
    })
}

#[cfg(test)]
mod tests;
