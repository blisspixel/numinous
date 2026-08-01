use crate::gamepad::Command;
use gilrs::Button;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_BINDINGS_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone)]
pub struct Bindings {
    pub gamepad: HashMap<Button, Command>,
}

impl Default for Bindings {
    fn default() -> Self {
        let mut gamepad = HashMap::new();
        gamepad.insert(Button::South, Command::PrimaryDown);
        gamepad.insert(Button::East, Command::Back);
        gamepad.insert(Button::Start, Command::Menu);
        gamepad.insert(Button::Select, Command::Inspect);
        gamepad.insert(Button::LeftThumb, Command::Reset);
        gamepad.insert(Button::LeftTrigger, Command::PreviousRoom);
        gamepad.insert(Button::RightTrigger, Command::NextRoom);
        gamepad.insert(Button::LeftTrigger2, Command::Slower);
        gamepad.insert(Button::RightTrigger2, Command::Faster);
        gamepad.insert(Button::DPadUp, Command::Up);
        gamepad.insert(Button::DPadDown, Command::Down);
        gamepad.insert(Button::DPadLeft, Command::Left);
        gamepad.insert(Button::DPadRight, Command::Right);
        gamepad.insert(Button::West, Command::CycleEra);
        gamepad.insert(Button::RightThumb, Command::Pause);
        Self { gamepad }
    }
}

impl Bindings {
    pub fn load() -> Self {
        Self::path().map_or_else(Self::default, |path| Self::load_from(&path))
    }

    fn load_from(path: &Path) -> Self {
        let mut bindings = Self::default();
        let mut content = String::new();
        if let Ok(file) = std::fs::File::open(path)
            && file
                .take(MAX_BINDINGS_BYTES + 1)
                .read_to_string(&mut content)
                .is_ok()
            && content.len() as u64 <= MAX_BINDINGS_BYTES
            && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(map) = json.as_object()
        {
            for (k, v) in map {
                let button = match k.as_str() {
                    "South" => Button::South,
                    "East" => Button::East,
                    "North" => Button::North,
                    "West" => Button::West,
                    "Start" => Button::Start,
                    "Select" => Button::Select,
                    "LeftThumb" => Button::LeftThumb,
                    "RightThumb" => Button::RightThumb,
                    "LeftTrigger" => Button::LeftTrigger,
                    "RightTrigger" => Button::RightTrigger,
                    "LeftTrigger2" => Button::LeftTrigger2,
                    "RightTrigger2" => Button::RightTrigger2,
                    "DPadUp" => Button::DPadUp,
                    "DPadDown" => Button::DPadDown,
                    "DPadLeft" => Button::DPadLeft,
                    "DPadRight" => Button::DPadRight,
                    _ => continue,
                };
                if let Some(action_str) = v.as_str() {
                    let command = match action_str {
                        "PrimaryDown" => Command::PrimaryDown,
                        "Back" => Command::Back,
                        "Menu" => Command::Menu,
                        "Inspect" => Command::Inspect,
                        "Reset" => Command::Reset,
                        "PreviousRoom" => Command::PreviousRoom,
                        "NextRoom" => Command::NextRoom,
                        "Slower" => Command::Slower,
                        "Faster" => Command::Faster,
                        "Up" => Command::Up,
                        "Down" => Command::Down,
                        "Left" => Command::Left,
                        "Right" => Command::Right,
                        "CycleEra" => Command::CycleEra,
                        "CycleRadio" => Command::CycleRadio,
                        "ToggleMute" => Command::ToggleMute,
                        "VolumeDown" => Command::VolumeDown,
                        "VolumeUp" => Command::VolumeUp,
                        "Pause" => Command::Pause,
                        _ => continue,
                    };
                    bindings.gamepad.insert(button, command);
                }
            }
        }
        bindings
    }

    fn path() -> Option<PathBuf> {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".numinous-bindings.json"))
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{Bindings, MAX_BINDINGS_BYTES};
    use crate::gamepad::Command;
    use gilrs::Button;

    #[test]
    fn oversized_bindings_file_falls_back_without_parsing_the_tail() {
        let path = std::env::temp_dir().join(format!(
            "numinous_bindings_oversized_{}.json",
            std::process::id()
        ));
        let mut content = String::from("{\"South\":\"Pause\"");
        content.push_str(&" ".repeat(MAX_BINDINGS_BYTES as usize));
        content.push('}');
        std::fs::write(&path, content).expect("oversized bindings fixture");

        let bindings = Bindings::load_from(&path);

        assert_eq!(
            bindings.gamepad.get(&Button::South),
            Some(&Command::PrimaryDown)
        );
        std::fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn bounded_bindings_file_overrides_known_commands_only() {
        let path = std::env::temp_dir().join(format!(
            "numinous_bindings_valid_{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"South":"Pause","North":"CycleRadio","West":"PrimaryDown","Mode":"Reset","East":"unknown"}"#,
        )
        .expect("bindings fixture");

        let bindings = Bindings::load_from(&path);

        assert_eq!(bindings.gamepad.get(&Button::South), Some(&Command::Pause));
        assert_eq!(
            bindings.gamepad.get(&Button::North),
            Some(&Command::CycleRadio)
        );
        assert_eq!(
            bindings.gamepad.get(&Button::West),
            Some(&Command::PrimaryDown)
        );
        assert_eq!(bindings.gamepad.get(&Button::East), Some(&Command::Back));
        assert!(!bindings.gamepad.contains_key(&Button::Mode));
        std::fs::remove_file(path).expect("cleanup");
    }
}
