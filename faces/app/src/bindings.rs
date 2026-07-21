use crate::gamepad::Command;
use gilrs::Button;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
        let mut bindings = Self::default();
        if let Some(path) = Self::path()
            && let Ok(content) = fs::read_to_string(path)
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
