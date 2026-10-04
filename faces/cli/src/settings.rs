//! Read-only presentation of the App's shared saved audio preferences.

use std::path::Path;

pub(crate) fn report(path: &Path, as_json: bool) -> Result<String, String> {
    let snapshot = numinous_core::read_app_preferences_snapshot(path)
        .map_err(|error| format!("Could not read saved App settings: {error}"))?;
    let levels = snapshot.preferences;
    if as_json {
        let value = serde_json::json!({
            "schema": "numinous.app-audio-settings",
            "schemaVersion": 1,
            "source": snapshot.source.name(),
            "readOnly": true,
            "scope": "app-playback-preferences",
            "levels": {
                "masterPercent": levels.volume_percent,
                "radioPercent": levels.music_volume_percent,
                "roomSoundPercent": levels.room_volume_percent,
                "effectsPercent": levels.effect_volume_percent,
            },
            "muted": levels.muted,
        });
        return Ok(format!("{value:#}\n"));
    }
    Ok(format!(
        "APP AUDIO SETTINGS ({})\nMaster: {}%\nRadio: {}%\nRoom Sound: {}%\nEffects: {}%\nMuted: {}\n\nRead-only App launch preferences; device and live playback state are not observed.\nCLI and MCP sound exports remain pre-master sources.\n",
        snapshot.source.name(),
        levels.volume_percent,
        levels.music_volume_percent,
        levels.room_volume_percent,
        levels.effect_volume_percent,
        if levels.muted { "yes" } else { "no" },
    ))
}
