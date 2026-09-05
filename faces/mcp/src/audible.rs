//! Sound a protocol can carry.
//!
//! Six packaged playtests ended on the same sentence. The player could read the
//! notes, name the intervals, measure the step in cents, and still write "I
//! cannot hear the two hills." That was never a missing number. A mind that
//! reaches this house down a pipe has no audio device to open, so the only way
//! it can hear anything is if the sound arrives as bytes it already knows how
//! to handle.
//!
//! The protocol has a place for exactly that. An audio content block carries a
//! base64 payload and a media type, and a client that can pass audio to its
//! model passes it. So a melody leaves here as a real WAV file rather than as a
//! description of one, and whether it can be heard stops being our excuse.

use numinous_core::SoundSpec;
use serde_json::{Value, json};

/// Sample rate for sound sent down the wire.
///
/// Low enough that a few seconds of melody stays a reasonable message, high
/// enough that every note this house sings sits well under the Nyquist limit:
/// the top of the singable range is a few kilohertz and this leaves eight.
pub(super) const WIRE_SAMPLE_RATE: u32 = 16_000;

/// The most encoded audio one reply will carry, in bytes.
///
/// A refusal that names this number and what was asked for is more useful than
/// a reply that silently drops the sound, and far more useful than one that
/// buries a caller under a megabyte they did not ask for.
pub(super) const MAX_WIRE_AUDIO_BYTES: usize = 1_500_000;

/// Whether this call asked to be able to hear the result.
///
/// Absent means no, because a caller that cannot use audio should not be made
/// to pay for it on every request.
pub(super) fn requested(arguments: &Value) -> Result<bool, String> {
    flag(arguments, "audio")
}

/// Whether this call asked for a Standard MIDI File of the same melody.
pub(super) fn midi_requested(arguments: &Value) -> Result<bool, String> {
    flag(arguments, "midi")
}

fn flag(arguments: &Value, name: &'static str) -> Result<bool, String> {
    match arguments.get(name) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(wanted)) => Ok(*wanted),
        Some(_) => Err(format!("Argument '{name}' must be true or false.")),
    }
}

/// Render a sound to a protocol audio block, or say why it will not fit.
///
/// The error names the budget, the size, and the seconds, in the same shape the
/// dwell budget uses, so a caller who is refused knows what to ask for instead.
pub(super) fn block(spec: &SoundSpec) -> Result<(Value, Value), String> {
    let wav = spec.wav(WIRE_SAMPLE_RATE);
    let encoded_len = base64_len(wav.len());
    if encoded_len > MAX_WIRE_AUDIO_BYTES {
        let seconds =
            MAX_WIRE_AUDIO_BYTES as f64 / base64_len(WIRE_SAMPLE_RATE as usize * 2) as f64;
        return Err(format!(
            "One reply carries at most {MAX_WIRE_AUDIO_BYTES} bytes of encoded audio, which is about {seconds:.0} seconds at {WIRE_SAMPLE_RATE} Hz. This sound is {:.1} seconds and would encode to {encoded_len} bytes. Ask for a shorter sound, or leave 'audio' off and read the notes.",
            spec.duration
        ));
    }
    let encoded = base64(&wav);
    let described = json!({
        "mimeType": "audio/wav",
        "sampleRate": WIRE_SAMPLE_RATE,
        "channels": 1,
        "bitsPerSample": 16,
        "durationSeconds": spec.duration,
        "encodedBytes": encoded.len(),
    });
    Ok((
        json!({ "type": "audio", "data": encoded, "mimeType": "audio/wav" }),
        described,
    ))
}

/// Wrap a melody as a MIDI resource block, or say why it will not fit.
pub(super) fn midi_block(spec: &SoundSpec) -> Result<(Value, Value), String> {
    let midi = spec.midi();
    let encoded_len = base64_len(midi.len());
    if encoded_len > MAX_WIRE_AUDIO_BYTES {
        return Err(format!(
            "One reply carries at most {MAX_WIRE_AUDIO_BYTES} bytes of encoded MIDI. This file would encode to {encoded_len} bytes. Leave 'midi' off and read the notes."
        ));
    }
    let encoded = base64(&midi);
    let described = json!({
        "mimeType": "audio/midi",
        "format": "smf-type-0",
        "ticksPerQuarter": numinous_core::MIDI_TICKS_PER_QUARTER,
        "tempoMicroseconds": numinous_core::MIDI_TEMPO_MICROSECONDS,
        "pitchBendRangeSemitones": numinous_core::MIDI_PITCH_BEND_RANGE_SEMITONES,
        "sourceNoteCount": spec.notes.len(),
        "encodedBytes": encoded.len(),
        "loss": "nearest 12-TET keys plus 14-bit pitch bend; large note intervals are preserved; one voice at 960 ticks per second, last valid source note wins shared starts; declared duration retained, native waveform and overlapping envelopes omitted",
    });
    Ok((
        json!({
            "type": "resource",
            "resource": {
                "uri": "numinous://studio-melody.mid",
                "mimeType": "audio/midi",
                "blob": encoded
            }
        }),
        described,
    ))
}

/// Attach an audio block to a tool result that already has content.
pub(super) fn attach(mut result: Value, audio: Value) -> Value {
    if let Some(content) = result
        .get_mut("content")
        .and_then(serde_json::Value::as_array_mut)
    {
        content.push(audio);
    }
    result
}

/// How many bytes `len` raw bytes become once encoded.
fn base64_len(len: usize) -> usize {
    len.div_ceil(3) * 4
}

/// Standard base64 with padding, written here so this path needs no dependency.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(base64_len(bytes.len()));
    for group in bytes.chunks(3) {
        // Pack the group left-aligned into 24 bits, so a short final group is
        // padded with zero bits rather than read past its end.
        let packed = group
            .iter()
            .enumerate()
            .fold(0u32, |packed, (index, &byte)| {
                packed | (u32::from(byte) << (16 - 8 * index))
            });
        for slot in 0..4 {
            if slot <= group.len() {
                let index = (packed >> (18 - 6 * slot)) & 0x3F;
                out.push(char::from(ALPHABET[index as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_WIRE_AUDIO_BYTES, WIRE_SAMPLE_RATE, attach, base64, block, midi_block, midi_requested,
        requested,
    };
    use numinous_core::SoundSpec;
    use serde_json::json;

    #[test]
    fn base64_matches_the_standard_on_the_cases_that_break_encoders() {
        // The classic vectors, which pin all three group lengths and both
        // padding shapes. Anything that gets these right gets a WAV right.
        for (raw, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(raw.as_bytes()), expected, "encoding {raw:?}");
        }
        // High bytes are where a sign-extension bug would show, and a WAV is
        // full of them.
        assert_eq!(base64(&[0xFF, 0xFE, 0xFD]), "//79");
        assert_eq!(base64(&[0x00, 0x80, 0xFF]), "AID/");
    }

    #[test]
    fn a_melody_leaves_as_a_playable_file() {
        let spec = SoundSpec::arpeggio(&[261.63, 329.63, 392.0], 1.0, 0.4);
        let (audio, described) = block(&spec).expect("a one second melody fits");
        assert_eq!(audio["type"], "audio");
        assert_eq!(audio["mimeType"], "audio/wav");
        assert_eq!(described["sampleRate"], WIRE_SAMPLE_RATE);
        // The payload has to be decodable by something that is not us, so it
        // must be padded base64 of a file that starts where a WAV starts.
        let data = audio["data"].as_str().expect("payload");
        assert!(
            data.starts_with("UklGR"),
            "not a RIFF header: {}",
            &data[..8]
        );
        assert!(data.len().is_multiple_of(4), "unpadded base64");
        assert!(
            data.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')),
            "payload is not base64"
        );
        assert_eq!(described["encodedBytes"], data.len());
    }

    #[test]
    fn a_sound_too_long_to_send_is_refused_with_its_own_numbers() {
        let spec = SoundSpec::tone(440.0, 600.0, 0.3);
        let error = block(&spec).expect_err("ten minutes cannot fit one reply");
        assert!(error.contains(&MAX_WIRE_AUDIO_BYTES.to_string()), "{error}");
        assert!(error.contains("600.0 seconds"), "{error}");
        assert!(error.contains("shorter"), "{error}");
    }

    #[test]
    fn asking_to_hear_is_opt_in_and_typed() {
        assert_eq!(requested(&json!({})), Ok(false));
        assert_eq!(requested(&json!({"audio": null})), Ok(false));
        assert_eq!(requested(&json!({"audio": false})), Ok(false));
        assert_eq!(requested(&json!({"audio": true})), Ok(true));
        assert!(requested(&json!({"audio": "yes"})).is_err());
        assert!(requested(&json!({"audio": 1})).is_err());
    }

    #[test]
    fn a_melody_leaves_as_a_standard_midi_file() {
        let spec = SoundSpec::tone(440.0, 0.5, 0.5);
        let (block, described) = midi_block(&spec).expect("a short melody fits");
        assert_eq!(block["type"], "resource");
        assert_eq!(block["resource"]["mimeType"], "audio/midi");
        assert_eq!(described["format"], "smf-type-0");
        let data = block["resource"]["blob"].as_str().expect("payload");
        assert!(
            data.starts_with("TVRoZA"),
            "not an MThd header: {}",
            &data[..8]
        );
        assert_eq!(described["encodedBytes"], data.len());
    }

    #[test]
    fn asking_for_midi_is_opt_in_and_typed() {
        assert_eq!(midi_requested(&json!({})), Ok(false));
        assert_eq!(midi_requested(&json!({"midi": true})), Ok(true));
        assert!(midi_requested(&json!({"midi": "yes"})).is_err());
    }

    #[test]
    fn attaching_audio_keeps_the_text_that_was_already_there() {
        // A client that cannot play audio still has to get the reading, so the
        // sound is added beside the prose and never in place of it.
        let result = json!({
            "content": [{"type": "text", "text": "eight notes"}],
            "isError": false
        });
        let attached = attach(result, json!({"type": "audio"}));
        assert_eq!(attached["content"][0]["text"], "eight notes");
        assert_eq!(attached["content"][1]["type"], "audio");
    }
}
