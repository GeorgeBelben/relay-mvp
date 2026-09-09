// Wire shape pushed from the Rust backend's "gamepad:event" (relay_core::system::gamepad) --
// mirrors that module's GamepadButton/GamepadAxis/GamepadEvent enums exactly, including the
// kebab-case serde rename.

export type GamepadButton =
  | "south"
  | "east"
  | "west"
  | "north"
  | "d-pad-up"
  | "d-pad-down"
  | "d-pad-left"
  | "d-pad-right"
  | "left-shoulder"
  | "left-trigger"
  | "right-shoulder"
  | "right-trigger"
  | "select"
  | "start"
  | "mode"
  | "left-stick"
  | "right-stick"
  | "unknown";

export type GamepadAxis = "left-stick-x" | "left-stick-y" | "right-stick-x" | "right-stick-y" | "unknown";

export type BackendGamepadEvent =
  | { type: "connected"; id: number; name: string }
  | { type: "disconnected"; id: number }
  | { type: "button-pressed"; id: number; button: GamepadButton }
  | { type: "button-released"; id: number; button: GamepadButton }
  | { type: "axis-changed"; id: number; axis: GamepadAxis; value: number };
