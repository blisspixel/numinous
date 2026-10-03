use crate::hud::{AudioSource, AudioState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Program {
    RoomScore,
    Studio,
    Radio,
    /// Local reconstruction of consented public MCP play.
    WatchAgent,
}

/// The player's source levels beneath master, in whole percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceLevels {
    /// The radio.
    pub(crate) radio_percent: u8,
    /// Room sound: the room score, Studio, and Watch Agent replay.
    pub(crate) room_percent: u8,
}

impl Default for SourceLevels {
    /// Every source at full level, as on a fresh install.
    fn default() -> Self {
        Self {
            radio_percent: 100,
            room_percent: 100,
        }
    }
}

pub(crate) fn describe(
    program: Program,
    radio_station: Option<&'static str>,
    volume: f32,
    levels: SourceLevels,
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
        Program::WatchAgent => AudioSource::WatchAgent,
    };
    let source_level = match source {
        AudioSource::Radio(_) => levels.radio_percent,
        _ => levels.room_percent,
    };
    AudioState::new(
        source,
        (volume.clamp(0.0, 1.0) * 100.0).round() as u8,
        muted,
        active,
    )
    .with_source_level(source_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_state_mapping_covers_every_effective_audio_state() {
        let full = SourceLevels::default();
        let room_off = SourceLevels {
            radio_percent: 100,
            room_percent: 0,
        };
        let radio_off = SourceLevels {
            radio_percent: 0,
            room_percent: 100,
        };
        let cases = [
            (
                describe(Program::RoomScore, None, 0.45, full, false, true, true),
                "ROOM MUSIC: VOL 45%",
            ),
            (
                describe(
                    Program::Radio,
                    Some("NUMINA FM"),
                    0.3,
                    full,
                    false,
                    true,
                    true,
                ),
                "RADIO NUMINA FM: VOL 30%",
            ),
            (
                describe(Program::Studio, None, 0.7, full, false, true, true),
                "STUDIO: VOL 70%",
            ),
            (
                describe(Program::WatchAgent, None, 0.6, full, false, true, true),
                "WATCH AGENT: VOL 60%",
            ),
            (
                describe(Program::RoomScore, None, 0.45, full, true, true, true),
                "ROOM MUSIC: MUTED",
            ),
            (
                describe(Program::RoomScore, None, 0.0, full, false, true, true),
                "ROOM MUSIC: VOL 0",
            ),
            (
                describe(Program::RoomScore, None, 0.45, full, false, false, true),
                "ROOM MUSIC: BACKGROUND SILENT",
            ),
            (
                describe(Program::RoomScore, None, 0.45, full, false, true, false),
                "NO SOUND DEVICE",
            ),
            (
                describe(Program::Radio, None, 0.45, full, false, true, true),
                "ROOM MUSIC: VOL 45%",
            ),
            (
                describe(Program::RoomScore, None, 0.45, room_off, false, true, true),
                "ROOM MUSIC: ROOM SOUND 0",
            ),
            (
                describe(Program::Studio, None, 0.45, room_off, false, true, true),
                "STUDIO: ROOM SOUND 0",
            ),
            (
                describe(Program::WatchAgent, None, 0.45, room_off, false, true, true),
                "WATCH AGENT: ROOM SOUND 0",
            ),
            (
                describe(
                    Program::Radio,
                    Some("NUMINA FM"),
                    0.3,
                    radio_off,
                    false,
                    true,
                    true,
                ),
                "RADIO NUMINA FM: RADIO 0",
            ),
            (
                describe(
                    Program::Radio,
                    Some("NUMINA FM"),
                    0.3,
                    room_off,
                    false,
                    true,
                    true,
                ),
                "RADIO NUMINA FM: VOL 30%",
            ),
            (
                describe(Program::RoomScore, None, 0.45, radio_off, false, true, true),
                "ROOM MUSIC: VOL 45%",
            ),
            (
                describe(Program::RoomScore, None, 0.45, room_off, true, true, true),
                "ROOM MUSIC: MUTED",
            ),
            (
                describe(Program::RoomScore, None, 0.0, room_off, false, true, true),
                "ROOM MUSIC: VOL 0",
            ),
        ];

        for (state, expected) in cases {
            assert_eq!(state.label(), expected);
        }
    }
}
