import { listen } from "@tauri-apps/api/event";
import type { BackendGamepadEvent, GamepadButton } from "./backendEvent";
import type { NavAction, NavDirection, NavEvent } from "./types";

// Standard mapping, matching what 8BitDo controllers (and Xbox/generic pads generally) report
// themselves as -- same face-button semantics the old browser-Gamepad-API version used, just
// named instead of indexed, since relay-core's gilrs-backed events already carry semantic names.
export const BUTTON_ACTION_MAP: Partial<Record<GamepadButton, NavAction>> = {
  south: "confirm", // A / Cross
  east: "back", // B / Circle
  north: "menu", // Y / Triangle / X (north face button)
  mode: "power", // Guide / PS / Xbox button -- opens the Power Menu, distinct from "menu" above
};

const STICK_DEADZONE = 0.5;
const REPEAT_INITIAL_DELAY_MS = 400;
const REPEAT_INTERVAL_MS = 120;
// How often the repeat timer re-checks the currently-held direction -- not tied to a frame rate
// like the old requestAnimationFrame version, since there's no longer a per-frame device poll to
// piggyback on; this just needs to be frequent enough that REPEAT_INTERVAL_MS feels smooth.
const REPEAT_CHECK_INTERVAL_MS = 16;

export function stickToDirection(x: number, y: number): NavDirection | null {
  if (Math.abs(x) < STICK_DEADZONE && Math.abs(y) < STICK_DEADZONE) return null;
  return Math.abs(x) > Math.abs(y) ? (x > 0 ? "right" : "left") : y > 0 ? "down" : "up";
}

function directionFromButton(button: GamepadButton): NavDirection | null {
  switch (button) {
    case "d-pad-up":
      return "up";
    case "d-pad-down":
      return "down";
    case "d-pad-left":
      return "left";
    case "d-pad-right":
      return "right";
    default:
      return null;
  }
}

export function startGamepadListener(onEvent: (event: NavEvent) => void, onUsed: () => void): () => void {
  // Undifferentiated pool of controllers, same model as before (no per-player assignment) -- held
  // state is tracked per gamepad id only so multiple pads' dpad/stick state can't clobber each
  // other, not to treat them as separate players. Whichever pad has an active direction "wins",
  // same "whoever's plugged in drives the shared UI" behavior as before.
  const heldDpadDirection = new Map<number, NavDirection>();
  const stickAxes = new Map<number, { x: number; y: number }>();
  let activeDirection: { direction: NavDirection; since: number; lastRepeat: number } | null = null;

  function currentDirection(): NavDirection | null {
    for (const direction of heldDpadDirection.values()) return direction;
    for (const { x, y } of stickAxes.values()) {
      const direction = stickToDirection(x, y);
      if (direction) return direction;
    }
    return null;
  }

  function checkRepeat() {
    const direction = currentDirection();
    const now = performance.now();

    if (!direction) {
      activeDirection = null;
      return;
    }
    if (!activeDirection || activeDirection.direction !== direction) {
      activeDirection = { direction, since: now, lastRepeat: now };
      onEvent({ type: "direction", direction });
      onUsed();
    } else if (now - activeDirection.since > REPEAT_INITIAL_DELAY_MS && now - activeDirection.lastRepeat > REPEAT_INTERVAL_MS) {
      activeDirection.lastRepeat = now;
      onEvent({ type: "direction", direction });
      onUsed();
    }
  }
  const repeatInterval = window.setInterval(checkRepeat, REPEAT_CHECK_INTERVAL_MS);

  const unlisten = listen<BackendGamepadEvent>("gamepad:event", ({ payload }) => {
    switch (payload.type) {
      case "button-pressed": {
        const action = BUTTON_ACTION_MAP[payload.button];
        if (action) {
          onEvent({ type: "action", action });
          onUsed();
        }
        const direction = directionFromButton(payload.button);
        if (direction) {
          heldDpadDirection.set(payload.id, direction);
          checkRepeat(); // fire immediately on press rather than waiting for the next tick
        }
        break;
      }
      case "button-released": {
        const direction = directionFromButton(payload.button);
        if (direction && heldDpadDirection.get(payload.id) === direction) {
          heldDpadDirection.delete(payload.id);
        }
        break;
      }
      case "axis-changed": {
        if (payload.axis !== "left-stick-x" && payload.axis !== "left-stick-y") break; // only the left stick drives nav, matches the old readDirection
        const current = stickAxes.get(payload.id) ?? { x: 0, y: 0 };
        if (payload.axis === "left-stick-x") current.x = payload.value;
        else current.y = payload.value;
        stickAxes.set(payload.id, current);
        break;
      }
      case "disconnected":
        heldDpadDirection.delete(payload.id);
        stickAxes.delete(payload.id);
        break;
    }
  });

  return () => {
    window.clearInterval(repeatInterval);
    unlisten.then((f) => f());
  };
}
