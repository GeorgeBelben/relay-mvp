//! Raw gamepad input, read directly from the OS rather than through a browser's Gamepad API --
//! WebKitGTK (Tauri's webview on Linux) doesn't reliably ship that API at all (it depends on
//! libmanette, which distro webkit2gtk builds commonly don't link against), so relay-ui's menu
//! navigation needs its own source of truth for controller input, the same way RetroArch's own
//! recommended Linux joypad driver (`udev`) reads `/dev/input` directly rather than going through
//! a browser.

use gilrs::{Axis, Button, EventType, Gilrs};
use serde::Serialize;

// ~60Hz, matching a typical game loop's polling rate -- gilrs's own event queue drains
// immediately regardless, this just caps how often we check it.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftShoulder,
    LeftTrigger,
    RightShoulder,
    RightTrigger,
    Select,
    Start,
    Mode,
    LeftStick,
    RightStick,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    RightStickX,
    RightStickY,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum GamepadEvent {
    Connected { id: usize, name: String },
    Disconnected { id: usize },
    ButtonPressed { id: usize, button: GamepadButton },
    ButtonReleased { id: usize, button: GamepadButton },
    AxisChanged { id: usize, axis: GamepadAxis, value: f32 },
}

fn map_button(button: Button) -> GamepadButton {
    match button {
        Button::South => GamepadButton::South,
        Button::East => GamepadButton::East,
        Button::West => GamepadButton::West,
        Button::North => GamepadButton::North,
        Button::DPadUp => GamepadButton::DPadUp,
        Button::DPadDown => GamepadButton::DPadDown,
        Button::DPadLeft => GamepadButton::DPadLeft,
        Button::DPadRight => GamepadButton::DPadRight,
        Button::LeftTrigger => GamepadButton::LeftShoulder,
        Button::LeftTrigger2 => GamepadButton::LeftTrigger,
        Button::RightTrigger => GamepadButton::RightShoulder,
        Button::RightTrigger2 => GamepadButton::RightTrigger,
        Button::Select => GamepadButton::Select,
        Button::Start => GamepadButton::Start,
        Button::Mode => GamepadButton::Mode,
        Button::LeftThumb => GamepadButton::LeftStick,
        Button::RightThumb => GamepadButton::RightStick,
        _ => GamepadButton::Unknown,
    }
}

fn map_axis(axis: Axis) -> GamepadAxis {
    match axis {
        Axis::LeftStickX => GamepadAxis::LeftStickX,
        Axis::LeftStickY => GamepadAxis::LeftStickY,
        Axis::RightStickX => GamepadAxis::RightStickX,
        Axis::RightStickY => GamepadAxis::RightStickY,
        _ => GamepadAxis::Unknown,
    }
}

/// Spawns a dedicated OS thread that polls every connected gamepad for as long as the process
/// runs, calling `on_event` for each button/axis/connect/disconnect change.
///
/// gilrs has no async API -- its polling loop is inherently blocking -- so this gets a real OS
/// thread rather than a tokio task, which would either block a worker thread or need an unsafe
/// Send wrapper for no real benefit. As a side effect, continuously reading the device this way is
/// also what keeps some 8BitDo receivers' firmware watchdog satisfied (confirmed against real
/// hardware): those receivers reset the USB link if nothing reads from it for about a second, and
/// this loop never lets that gap happen once a game or the menu is actually running.
pub fn watch_gamepads(on_event: impl Fn(GamepadEvent) + Send + 'static) {
    std::thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(gilrs) => gilrs,
            Err(err) => {
                eprintln!("gamepad support unavailable: {err}");
                return;
            }
        };

        loop {
            while let Some(gilrs::Event { id: gamepad_id, event, .. }) = gilrs.next_event() {
                let id: usize = gamepad_id.into();
                let mapped = match event {
                    EventType::Connected => Some(GamepadEvent::Connected { id, name: gilrs.gamepad(gamepad_id).name().to_string() }),
                    EventType::Disconnected => Some(GamepadEvent::Disconnected { id }),
                    EventType::ButtonPressed(button, _) => Some(GamepadEvent::ButtonPressed { id, button: map_button(button) }),
                    EventType::ButtonReleased(button, _) => Some(GamepadEvent::ButtonReleased { id, button: map_button(button) }),
                    EventType::AxisChanged(axis, value, _) => Some(GamepadEvent::AxisChanged { id, axis: map_axis(axis), value }),
                    _ => None,
                };
                if let Some(event) = mapped {
                    on_event(event);
                }
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_button_covers_every_standard_face_and_dpad_button() {
        assert_eq!(map_button(Button::South), GamepadButton::South);
        assert_eq!(map_button(Button::East), GamepadButton::East);
        assert_eq!(map_button(Button::DPadUp), GamepadButton::DPadUp);
    }

    #[test]
    fn map_button_falls_back_to_unknown_for_an_unrecognized_button() {
        assert_eq!(map_button(Button::Unknown), GamepadButton::Unknown);
    }

    #[test]
    fn map_axis_covers_both_sticks() {
        assert_eq!(map_axis(Axis::LeftStickX), GamepadAxis::LeftStickX);
        assert_eq!(map_axis(Axis::RightStickY), GamepadAxis::RightStickY);
    }

    #[test]
    fn map_axis_falls_back_to_unknown_for_an_unrecognized_axis() {
        assert_eq!(map_axis(Axis::Unknown), GamepadAxis::Unknown);
    }
}
