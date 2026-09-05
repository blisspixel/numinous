//! Standard MIDI File projection of a [`SoundSpec`].
//!
//! Each note gets its own nearest 12-TET key and a 14-bit pitch bend for the
//! remaining fraction of a semitone. The file declares a plus-or-minus
//! two-semitone range through Registered Parameter Number 0. Large intervals
//! between notes are preserved; only pitches outside the available keys and
//! bend range are clamped. MIDI carries note events, not the native sine
//! waveform, overlapping envelopes, or a continuous pitch glide.
//!
//! This projection is monophonic because pitch bend affects an entire channel.
//! Absolute times are rounded to 960 ticks per second. When several notes land
//! on one tick, the last valid note in source order wins; each note ends before
//! the next begins. The declared sound duration bounds notes and retains
//! trailing silence. Non-finite or nonpositive durations produce an empty
//! track. Invalid and inaudible notes are omitted, finite negative starts are
//! moved to zero, and a retained note lasts at least one tick.
//!
//! To keep the infallible encoder within Standard MIDI File limits, the track
//! ends by tick 0x0FFFFFFF, about 77.7 hours. Excess notes beyond the track byte
//! budget are omitted. Studio melodies are far below both bounds.

use crate::sound::{Note, SoundSpec};

/// Ticks per quarter note in the written file.
pub const MIDI_TICKS_PER_QUARTER: u16 = 480;
/// Microseconds per quarter note: 120 BPM.
pub const MIDI_TEMPO_MICROSECONDS: u32 = 500_000;
/// Pitch-bend range encoded in the file, in semitones each side of center.
pub const MIDI_PITCH_BEND_RANGE_SEMITONES: f32 = 2.0;

const TICKS_PER_SECOND: f64 =
    MIDI_TICKS_PER_QUARTER as f64 * 1_000_000.0 / MIDI_TEMPO_MICROSECONDS as f64;
const HEADER_LEN: usize = 14;
// SMF variable-length quantities contain at most four seven-bit groups.
const MAX_TICK: u32 = 0x0FFF_FFFF;
// Four events per note need at most 28 bytes. The reserved 64 bytes exceed
// the tempo, RPN selection, and final end-of-track event combined.
const MAX_ENCODED_NOTES: usize = (u32::MAX as usize - 64) / 28;

/// Encode `spec` as a Standard MIDI File type 0.
///
/// The projection is monophonic, quantizes time and pitch, and follows the
/// duration and format bounds documented in this module.
#[must_use]
pub fn midi_file(spec: &SoundSpec) -> Vec<u8> {
    let track = track_bytes(spec);
    let mut bytes = Vec::with_capacity(HEADER_LEN + 8 + track.len());
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6u32.to_be_bytes());
    bytes.extend_from_slice(&0u16.to_be_bytes());
    bytes.extend_from_slice(&1u16.to_be_bytes());
    bytes.extend_from_slice(&MIDI_TICKS_PER_QUARTER.to_be_bytes());
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend(track);
    bytes
}

fn track_bytes(spec: &SoundSpec) -> Vec<u8> {
    let end_tick = seconds_to_ticks(f64::from(spec.duration));
    let mut events = Vec::new();
    events.push(MidiEvent {
        tick: 0,
        kind: EventKind::Tempo,
    });
    events.push(MidiEvent {
        tick: 0,
        kind: EventKind::PitchBendRange,
    });
    let sounding = sounding_notes(&spec.notes, end_tick);
    for (index, note) in sounding.iter().take(MAX_ENCODED_NOTES).enumerate() {
        let end = sounding
            .get(index + 1)
            .map_or(note.end, |next| note.end.min(next.start));
        events.push(MidiEvent {
            tick: note.start,
            kind: EventKind::PitchBend(note.bend),
        });
        events.push(MidiEvent {
            tick: note.start,
            kind: EventKind::NoteOn {
                key: note.key,
                velocity: note.velocity,
            },
        });
        events.push(MidiEvent {
            tick: end,
            kind: EventKind::NoteOff { key: note.key },
        });
        events.push(MidiEvent {
            tick: end,
            kind: EventKind::PitchBend(8192),
        });
    }
    // Quantized starts are distinct and every note ends by the next start.
    // Appending a whole note here orders its off before the next bend and on.
    encode_events(&events, end_tick)
}

#[derive(Clone, Copy)]
struct MidiNote {
    start: u32,
    end: u32,
    key: u8,
    bend: u16,
    velocity: u8,
}

fn sounding_notes(notes: &[Note], end_tick: u32) -> Vec<MidiNote> {
    let mut sounding = notes
        .iter()
        .filter_map(|note| {
            if !note.start.is_finite()
                || !note.dur.is_finite()
                || note.dur <= 0.0
                || !note.amp.is_finite()
                || note.amp <= 0.0
            {
                return None;
            }
            let (key, bend) = midi_from_hz(note.freq)?;
            let start_seconds = f64::from(note.start.max(0.0));
            let start = seconds_to_ticks(start_seconds);
            if start >= end_tick {
                return None;
            }
            // Add in seconds before rounding. Adding rounded durations to
            // rounded starts can overlap the next note by a tick.
            let end = seconds_to_ticks(start_seconds + f64::from(note.dur))
                .max(start + 1)
                .min(end_tick);
            Some(MidiNote {
                start,
                end,
                key,
                bend,
                velocity: velocity(note.amp),
            })
        })
        .collect::<Vec<_>>();
    sounding.sort_by_key(|note| note.start);
    sounding.dedup_by(|later, earlier| {
        if later.start == earlier.start {
            *earlier = *later;
            true
        } else {
            false
        }
    });
    sounding
}

fn midi_from_hz(freq: f32) -> Option<(u8, u16)> {
    if !freq.is_finite() || freq <= 0.0 {
        return None;
    }
    // Widen before division so positive subnormal frequencies do not
    // underflow to zero before their documented range clamp.
    let semitones = 69.0 + 12.0 * (f64::from(freq) / 440.0).log2();
    let nearest = semitones.round().clamp(0.0, 127.0);
    let key = nearest as u8;
    let range = f64::from(MIDI_PITCH_BEND_RANGE_SEMITONES);
    let residual = (semitones - nearest).clamp(-range, range);
    let bend = (8192.0 + residual / range * 8192.0)
        .round()
        .clamp(0.0, 16383.0) as u16;
    Some((key, bend))
}

fn velocity(amp: f32) -> u8 {
    let scaled = (amp.clamp(0.0, 1.0) * 127.0).round();
    scaled.clamp(1.0, 127.0) as u8
}

fn seconds_to_ticks(seconds: f64) -> u32 {
    if !seconds.is_finite() || seconds <= 0.0 {
        0
    } else {
        (seconds * TICKS_PER_SECOND)
            .round()
            .min(f64::from(MAX_TICK)) as u32
    }
}

struct MidiEvent {
    tick: u32,
    kind: EventKind,
}

#[derive(Clone, Copy)]
enum EventKind {
    Tempo,
    PitchBendRange,
    PitchBend(u16),
    NoteOff { key: u8 },
    NoteOn { key: u8, velocity: u8 },
}

fn pitch_bend_range_semitones() -> u8 {
    MIDI_PITCH_BEND_RANGE_SEMITONES.round().clamp(0.0, 127.0) as u8
}

fn encode_events(events: &[MidiEvent], end_tick: u32) -> Vec<u8> {
    let mut track = Vec::new();
    let mut cursor = 0u32;
    for event in events {
        write_vlq(&mut track, event.tick - cursor);
        cursor = event.tick;
        match event.kind {
            EventKind::Tempo => {
                track.extend_from_slice(&[0xFF, 0x51, 0x03]);
                track.push(((MIDI_TEMPO_MICROSECONDS >> 16) & 0xFF) as u8);
                track.push(((MIDI_TEMPO_MICROSECONDS >> 8) & 0xFF) as u8);
                track.push((MIDI_TEMPO_MICROSECONDS & 0xFF) as u8);
            }
            EventKind::PitchBendRange => {
                // RPN 0 (pitch-bend sensitivity): two semitones, zero cents,
                // then the null RPN so a later Data Entry cannot retune it.
                let messages = [
                    (0x65, 0x00),
                    (0x64, 0x00),
                    (0x06, pitch_bend_range_semitones()),
                    (0x26, 0x00),
                    (0x65, 0x7F),
                    (0x64, 0x7F),
                ];
                for (index, (controller, value)) in messages.into_iter().enumerate() {
                    if index > 0 {
                        write_vlq(&mut track, 0);
                    }
                    track.extend_from_slice(&[0xB0, controller, value]);
                }
            }
            EventKind::PitchBend(bend) => {
                track.push(0xE0);
                track.push((bend & 0x7F) as u8);
                track.push(((bend >> 7) & 0x7F) as u8);
            }
            EventKind::NoteOn { key, velocity } => {
                track.extend_from_slice(&[0x90, key, velocity]);
            }
            EventKind::NoteOff { key } => {
                track.extend_from_slice(&[0x80, key, 0x40]);
            }
        }
    }
    write_vlq(&mut track, end_tick - cursor);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    track
}

fn write_vlq(out: &mut Vec<u8>, value: u32) {
    let mut bytes = vec![(value & 0x7F) as u8];
    let mut rest = value >> 7;
    while rest > 0 {
        bytes.push(((rest & 0x7F) as u8) | 0x80);
        rest >>= 7;
    }
    bytes.reverse();
    out.extend(bytes);
}

impl SoundSpec {
    /// Encode this sound as a Standard MIDI File type 0.
    ///
    /// See [`midi_file`] for the projection's timing and pitch limits.
    #[must_use]
    pub fn midi(&self) -> Vec<u8> {
        midi_file(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{midi_file, write_vlq};
    use crate::sound::{Note, SoundSpec};

    #[derive(Debug)]
    struct DecodedNote {
        key: u8,
        velocity: u8,
        bend: u16,
        start: u32,
        end: u32,
    }

    #[derive(Debug)]
    struct Decoded {
        notes: Vec<DecodedNote>,
        end: u32,
        tempo: u32,
        division: u16,
    }

    impl Decoded {
        fn seconds(&self, ticks: u32) -> f64 {
            f64::from(ticks) * f64::from(self.tempo) / (1_000_000.0 * f64::from(self.division))
        }
    }

    fn read_vlq(bytes: &[u8], offset: &mut usize) -> u32 {
        let mut value = 0;
        for length in 1..=4 {
            let byte = bytes[*offset];
            *offset += 1;
            value = value * 128 + u32::from(byte & 127);
            if byte < 128 {
                return value;
            }
            assert!(
                length < 4,
                "SMF variable-length quantity exceeds four bytes"
            );
        }
        unreachable!("the fourth byte either returns or fails")
    }

    // Read the wire format independently of the encoder's event types and
    // conversion helpers. An active channel voice must end before any other
    // note or pitch bend can change that channel's state.
    fn decode(bytes: &[u8]) -> Decoded {
        assert_eq!(&bytes[..4], b"MThd");
        assert_eq!(u32::from_be_bytes(bytes[4..8].try_into().unwrap()), 6);
        assert_eq!(u16::from_be_bytes(bytes[8..10].try_into().unwrap()), 0);
        assert_eq!(u16::from_be_bytes(bytes[10..12].try_into().unwrap()), 1);
        let division = u16::from_be_bytes(bytes[12..14].try_into().unwrap());
        assert_eq!(division, 480);
        assert_eq!(&bytes[14..18], b"MTrk");
        let length = u32::from_be_bytes(bytes[18..22].try_into().unwrap()) as usize;
        assert_eq!(length, bytes.len() - 22);
        let mut result = Decoded {
            notes: Vec::new(),
            end: 0,
            tempo: 0,
            division,
        };
        let mut offset = 22;
        let mut tick = 0u32;
        let mut bend = 8192;
        let mut rpn = (127, 127);
        let mut range = (0, 0);
        let mut active: Option<DecodedNote> = None;
        loop {
            tick = tick.checked_add(read_vlq(bytes, &mut offset)).unwrap();
            let status = bytes[offset];
            offset += 1;
            if status == 0xFF {
                let kind = bytes[offset];
                offset += 1;
                let length = read_vlq(bytes, &mut offset) as usize;
                let data = &bytes[offset..offset + length];
                offset += length;
                match kind {
                    0x51 => {
                        assert_eq!(tick, 0);
                        assert_eq!(data.len(), 3);
                        result.tempo = u32::from_be_bytes([0, data[0], data[1], data[2]]);
                        assert_eq!(result.tempo, 500_000);
                    }
                    0x2F => {
                        assert!(data.is_empty());
                        assert!(active.is_none(), "end-of-track leaves a note sounding");
                        assert_eq!(offset, bytes.len(), "data follows end-of-track");
                        result.end = tick;
                        return result;
                    }
                    _ => panic!("unexpected meta event {kind:#x}"),
                }
                continue;
            }
            let first = bytes[offset];
            let second = bytes[offset + 1];
            offset += 2;
            assert!(first < 128 && second < 128, "non-MIDI data byte");
            match status {
                0xB0 => {
                    assert!(active.is_none());
                    match first {
                        101 => rpn.0 = second,
                        100 => rpn.1 = second,
                        6 if rpn == (0, 0) => range.0 = second,
                        38 if rpn == (0, 0) => range.1 = second,
                        _ => panic!("unexpected controller {first} for RPN {rpn:?}"),
                    }
                }
                0xE0 => {
                    assert!(active.is_none(), "pitch bend retunes an active note");
                    bend = u16::from(first) + 128 * u16::from(second);
                }
                0x90 => {
                    assert!(active.is_none(), "MIDI notes overlap at tick {tick}");
                    assert!(second > 0, "a zero-velocity note-on is a note-off");
                    assert_eq!(range, (2, 0), "declare bend sensitivity before notes");
                    assert_eq!(rpn, (127, 127), "finish configuring the RPN before notes");
                    active = Some(DecodedNote {
                        key: first,
                        velocity: second,
                        bend,
                        start: tick,
                        end: 0,
                    });
                }
                0x80 => {
                    let mut note = active.take().expect("note-off without note-on");
                    assert_eq!(first, note.key);
                    assert!(tick > note.start, "note has no positive lifetime");
                    note.end = tick;
                    result.notes.push(note);
                }
                _ => panic!("unexpected channel event {status:#x}"),
            }
        }
    }

    fn note(freq: f32, start: f32, dur: f32) -> Note {
        Note {
            freq,
            start,
            dur,
            amp: 0.3,
        }
    }

    fn assert_pitch(note: &DecodedNote, expected_hz: f32) {
        let semitones = f64::from(note.key) - 69.0 + 2.0 * (f64::from(note.bend) - 8192.0) / 8192.0;
        let actual_hz = 440.0 * 2.0_f64.powf(semitones / 12.0);
        let cents_error = 1200.0 * (actual_hz / f64::from(expected_hz)).log2();
        // A two-semitone, 14-bit bend has a half-step error of 0.01221 cents.
        assert!(
            cents_error.abs() < 0.013,
            "pitch differs by {cents_error} cents"
        );
    }

    #[test]
    fn concert_a_declares_tempo_and_tuning_before_sounding() {
        let decoded = decode(&SoundSpec::tone(440.0, 0.5, 0.5).midi());
        assert_eq!(decoded.notes.len(), 1);
        let note = &decoded.notes[0];
        assert_eq!((note.key, note.bend, note.velocity), (69, 8192, 64));
        assert_eq!((note.start, note.end, decoded.end), (0, 480, 480));
        assert_eq!(decoded.seconds(decoded.end), 0.5);
    }

    #[test]
    fn jumps_choose_new_keys_and_fractional_pitches_keep_their_cents() {
        let frequencies = [
            220.0,
            880.0,
            445.0,
            1760.0,
            440.0 * 2.0_f32.powf(1.0 / 12.0),
        ];
        let decoded = decode(&SoundSpec::arpeggio(&frequencies, 1.0, 0.3).midi());
        assert_eq!(decoded.notes.len(), frequencies.len());
        for (note, frequency) in decoded.notes.iter().zip(frequencies) {
            assert_pitch(note, frequency);
        }
        assert_eq!(decoded.notes[0].key, 57);
        assert_eq!(decoded.notes[1].key, 81);
        assert_eq!(decoded.notes[4].key, 70);
    }

    #[test]
    fn overlap_ends_before_the_next_pitch_bend_and_note_on() {
        let decoded = decode(&midi_file(&SoundSpec {
            duration: 2.0,
            notes: vec![note(445.0, 0.0, 1.0), note(660.0, 0.5, 1.0)],
        }));
        assert_eq!(decoded.notes.len(), 2);
        assert_eq!(decoded.notes[0].end, 480);
        assert_eq!(decoded.notes[1].start, 480);
        assert_eq!(decoded.notes[1].end, 1440);
        assert_pitch(&decoded.notes[0], 445.0);
        assert_pitch(&decoded.notes[1], 660.0);
    }

    #[test]
    fn simultaneous_notes_keep_the_last_valid_source_note() {
        let decoded = decode(
            &SoundSpec {
                duration: 1.0,
                notes: vec![
                    note(445.0, 0.0, 0.5),
                    note(660.0, 0.0, 0.5),
                    note(f32::NAN, 0.0, 0.5),
                ],
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 1);
        assert_pitch(&decoded.notes[0], 660.0);
        assert_eq!((decoded.notes[0].start, decoded.notes[0].end), (0, 480));
    }

    #[test]
    fn distinct_source_onsets_on_one_tick_have_the_same_collision_policy() {
        // Source order chooses the winner even when the inputs are unsorted.
        let decoded = decode(
            &SoundSpec {
                duration: 0.01,
                notes: vec![
                    note(330.0, 0.0003, 0.002),
                    note(445.0, 0.0001, 0.002),
                    note(660.0, 0.0002, 0.002),
                ],
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 1);
        assert_pitch(&decoded.notes[0], 660.0);
        assert_eq!((decoded.notes[0].start, decoded.notes[0].end), (0, 2));
    }

    #[test]
    fn rounding_absolute_endpoints_cannot_create_a_one_tick_overlap() {
        let decoded = decode(
            &SoundSpec {
                duration: 0.01,
                notes: vec![
                    note(445.0, 0.000625, 0.001666667),
                    note(660.0, 0.002291667, 0.001),
                ],
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 2);
        assert_eq!((decoded.notes[0].start, decoded.notes[0].end), (1, 2));
        assert_eq!((decoded.notes[1].start, decoded.notes[1].end), (2, 3));
    }

    #[test]
    fn a_sub_tick_note_has_a_positive_lifetime_within_the_sound() {
        let decoded = decode(
            &SoundSpec {
                duration: 0.01,
                notes: vec![note(440.0, 0.001, f32::MIN_POSITIVE)],
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 1);
        assert_eq!((decoded.notes[0].start, decoded.notes[0].end), (1, 2));
        assert_eq!(decoded.end, 10);
    }

    #[test]
    fn sound_duration_clips_notes_and_excludes_later_onsets() {
        let decoded = decode(
            &SoundSpec {
                duration: 0.5,
                notes: vec![
                    note(445.0, 0.0, 10.0),
                    note(660.0, 0.5, 1.0),
                    note(880.0, 1.0, 1.0),
                ],
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 1);
        assert_eq!(decoded.notes[0].end, 480);
        assert_eq!(decoded.end, 480);
    }

    #[test]
    fn end_of_track_retains_trailing_and_wholly_silent_time() {
        for notes in [Vec::new(), vec![note(440.0, 0.0, 0.5)]] {
            let decoded = decode(
                &SoundSpec {
                    duration: 1.25,
                    notes,
                }
                .midi(),
            );
            assert_eq!(decoded.end, 1200);
            assert_eq!(decoded.seconds(decoded.end), 1.25);
        }
    }

    #[test]
    fn invalid_sound_durations_produce_no_notes_or_time() {
        for duration in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let decoded = decode(
                &SoundSpec {
                    duration,
                    notes: vec![note(440.0, 0.0, 1.0)],
                }
                .midi(),
            );
            assert!(decoded.notes.is_empty());
            assert_eq!(decoded.end, 0);
        }
    }

    #[test]
    fn invalid_notes_are_omitted_without_hiding_valid_ones() {
        let mut notes = Vec::new();
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            notes.extend([
                note(value, 0.0, 0.5),
                note(440.0, value, 0.5),
                note(440.0, 0.0, value),
                Note {
                    amp: value,
                    ..note(440.0, 0.0, 0.5)
                },
            ]);
        }
        for value in [0.0, -1.0] {
            notes.extend([
                note(value, 0.0, 0.5),
                note(440.0, 0.0, value),
                Note {
                    amp: value,
                    ..note(440.0, 0.0, 0.5)
                },
            ]);
        }
        notes.push(note(445.0, 0.0, 0.5));
        let decoded = decode(
            &SoundSpec {
                duration: 1.0,
                notes,
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 1);
        assert_pitch(&decoded.notes[0], 445.0);
    }

    #[test]
    fn negative_starts_follow_the_wave_projection_and_velocities_stay_sounding() {
        let decoded = decode(
            &SoundSpec {
                duration: 1.0,
                notes: vec![
                    Note {
                        amp: 3.0,
                        ..note(440.0, -1.0, 0.25)
                    },
                    Note {
                        amp: f32::MIN_POSITIVE,
                        ..note(660.0, 0.5, 0.25)
                    },
                ],
            }
            .midi(),
        );
        assert_eq!(decoded.notes.len(), 2);
        assert_eq!((decoded.notes[0].start, decoded.notes[0].end), (0, 240));
        assert_eq!(decoded.notes[0].velocity, 127);
        assert_eq!(decoded.notes[1].velocity, 1);
    }

    #[test]
    fn pitches_beyond_the_key_and_bend_range_are_clamped() {
        let decoded = decode(&SoundSpec::arpeggio(&[f32::from_bits(1), f32::MAX], 1.0, 0.3).midi());
        assert_eq!(decoded.notes.len(), 2);
        assert_eq!((decoded.notes[0].key, decoded.notes[0].bend), (0, 0));
        assert_eq!((decoded.notes[1].key, decoded.notes[1].bend), (127, 16383));
    }

    #[test]
    fn huge_finite_times_cannot_write_five_byte_deltas_or_unended_notes() {
        for notes in [
            Vec::new(),
            vec![note(440.0, 0.0, f32::MAX)],
            vec![note(440.0, f32::MAX, f32::MAX)],
        ] {
            let decoded = decode(
                &SoundSpec {
                    duration: f32::MAX,
                    notes,
                }
                .midi(),
            );
            assert_eq!(decoded.end, 0x0FFF_FFFF);
            for note in decoded.notes {
                assert!(note.start < note.end && note.end <= decoded.end);
            }
        }
    }

    #[test]
    fn studio_melodies_keep_pitch_order_and_declared_loop_length() {
        let expression = crate::studio::parse("sin(x)").unwrap();
        for count in [1, 32, crate::studio::MAX_MELODY_NOTES] {
            let source =
                crate::studio::to_melody(&expression, 0.0, std::f64::consts::TAU, count, 0.0);
            let decoded = decode(&source.midi());
            assert_eq!(decoded.notes.len(), count);
            for (midi_note, source_note) in decoded.notes.iter().zip(&source.notes) {
                assert_pitch(midi_note, source_note.freq);
                assert!(
                    (decoded.seconds(midi_note.start) - f64::from(source_note.start)).abs()
                        <= 0.5 / 960.0
                );
            }
            assert!(
                (decoded.seconds(decoded.end) - f64::from(source.duration)).abs() <= 0.5 / 960.0
            );
            let silence = decoded.seconds(decoded.end - decoded.notes.last().unwrap().end);
            assert!(
                (0.25..0.254).contains(&silence),
                "trailing silence was {silence}"
            );
        }
    }

    #[test]
    fn vlq_matches_the_standard_boundary_examples() {
        for (value, expected) in [
            (0, vec![0x00]),
            (127, vec![0x7F]),
            (128, vec![0x81, 0x00]),
            (0x200000, vec![0x81, 0x80, 0x80, 0x00]),
            (0x0FFF_FFFF, vec![0xFF, 0xFF, 0xFF, 0x7F]),
        ] {
            let mut encoded = Vec::new();
            write_vlq(&mut encoded, value);
            assert_eq!(encoded, expected);
        }
    }
}
