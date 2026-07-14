use crate::hud::{AudioSource, AudioState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Program {
    RoomScore,
    Studio,
    Radio,
}

pub(crate) fn describe(
    program: Program,
    radio_station: Option<&'static str>,
    volume: f32,
    muted: bool,
    active: bool,
    output_available: bool,
) -> AudioState {
    if !output_available {
        return AudioState::no_device();
    }
    let source = match program {
        Program::RoomScore => AudioSource::RoomScore,
        Program::Studio => AudioSource::Studio,
        Program::Radio => radio_station.map_or(AudioSource::RoomScore, AudioSource::Radio),
    };
    AudioState::new(
        source,
        (volume.clamp(0.0, 1.0) * 100.0).round() as u8,
        muted,
        active,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_state_mapping_covers_every_effective_audio_state() {
        let cases = [
            (
                describe(Program::RoomScore, None, 0.45, false, true, true),
                "ROOM MUSIC: VOL 45%",
            ),
            (
                describe(Program::Radio, Some("NUMINA FM"), 0.3, false, true, true),
                "RADIO NUMINA FM: VOL 30%",
            ),
            (
                describe(Program::Studio, None, 0.7, false, true, true),
                "STUDIO: VOL 70%",
            ),
            (
                describe(Program::RoomScore, None, 0.45, true, true, true),
                "ROOM MUSIC: MUTED",
            ),
            (
                describe(Program::RoomScore, None, 0.0, false, true, true),
                "ROOM MUSIC: VOL 0",
            ),
            (
                describe(Program::RoomScore, None, 0.45, false, false, true),
                "ROOM MUSIC: BACKGROUND SILENT",
            ),
            (
                describe(Program::RoomScore, None, 0.45, false, true, false),
                "NO SOUND DEVICE",
            ),
            (
                describe(Program::Radio, None, 0.45, false, true, true),
                "ROOM MUSIC: VOL 45%",
            ),
        ];

        for (state, expected) in cases {
            assert_eq!(state.label(), expected);
        }
    }
}
