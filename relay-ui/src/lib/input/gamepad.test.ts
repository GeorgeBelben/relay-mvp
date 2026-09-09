import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { startGamepadListener, stickToDirection } from "./gamepad";
import type { BackendGamepadEvent } from "./backendEvent";

type Listener = (event: { payload: BackendGamepadEvent }) => void;
let capturedListener: Listener | undefined;
const unlistenMock = vi.fn();
const listenMock = vi.fn((_event: string, handler: Listener) => {
  capturedListener = handler;
  return Promise.resolve(unlistenMock);
});
vi.mock("@tauri-apps/api/event", () => ({
  listen: (event: string, handler: Listener) => listenMock(event, handler),
}));

function emit(event: BackendGamepadEvent) {
  capturedListener!({ payload: event });
}

describe("stickToDirection", () => {
  it("returns null inside the deadzone", () => {
    expect(stickToDirection(0.1, 0.1)).toBeNull();
  });

  it("picks the dominant axis and its sign", () => {
    expect(stickToDirection(0.9, 0.1)).toBe("right");
    expect(stickToDirection(-0.9, 0.1)).toBe("left");
    expect(stickToDirection(0.1, 0.9)).toBe("down");
    expect(stickToDirection(0.1, -0.9)).toBe("up");
  });
});

describe("startGamepadListener", () => {
  beforeEach(() => {
    capturedListener = undefined;
    unlistenMock.mockClear();
    listenMock.mockClear();
    vi.useFakeTimers({ toFake: ["setInterval", "clearInterval", "performance"] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("fires a direction event once on press, not on every subsequent timer tick", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "d-pad-up" });
    vi.advanceTimersByTime(100); // well within the 400ms initial repeat delay

    expect(onEvent).toHaveBeenCalledTimes(1);
    expect(onEvent).toHaveBeenCalledWith({ type: "direction", direction: "up" });
  });

  it("repeats a held direction after the initial delay, then at the repeat interval", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "d-pad-up" }); // initial press
    vi.advanceTimersByTime(500); // past the 400ms initial delay -> repeat
    vi.advanceTimersByTime(150); // past the 120ms repeat interval since the last repeat -> repeat again

    expect(onEvent).toHaveBeenCalledTimes(3);
  });

  it("stops repeating once the button is released", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "d-pad-up" });
    emit({ type: "button-released", id: 0, button: "d-pad-up" });
    onEvent.mockClear();

    vi.advanceTimersByTime(1000);
    expect(onEvent).not.toHaveBeenCalled();
  });

  it("fires a confirm/back action once on press, not on release", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "south" }); // confirm
    emit({ type: "button-released", id: 0, button: "south" });

    expect(onEvent).toHaveBeenCalledTimes(1);
    expect(onEvent).toHaveBeenCalledWith({ type: "action", action: "confirm" });
  });

  it("fires a power action for the Guide/PS/Xbox button", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "mode" });

    expect(onEvent).toHaveBeenCalledWith({ type: "action", action: "power" });
  });

  it("allows a released and re-pressed action to fire again", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "east" }); // back
    emit({ type: "button-released", id: 0, button: "east" });
    emit({ type: "button-pressed", id: 0, button: "east" });

    expect(onEvent).toHaveBeenCalledTimes(2);
    expect(onEvent).toHaveBeenNthCalledWith(2, { type: "action", action: "back" });
  });

  it("drives direction from the left stick, ignoring axis changes inside the deadzone", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "axis-changed", id: 0, axis: "left-stick-x", value: 0.1 });
    vi.advanceTimersByTime(16);
    expect(onEvent).not.toHaveBeenCalled();

    emit({ type: "axis-changed", id: 0, axis: "left-stick-x", value: 0.9 });
    vi.advanceTimersByTime(16);
    expect(onEvent).toHaveBeenCalledWith({ type: "direction", direction: "right" });
  });

  it("forgets a disconnected pad's held direction", () => {
    const onEvent = vi.fn();
    startGamepadListener(onEvent, vi.fn());

    emit({ type: "button-pressed", id: 0, button: "d-pad-up" });
    emit({ type: "disconnected", id: 0 });
    onEvent.mockClear();

    vi.advanceTimersByTime(1000);
    expect(onEvent).not.toHaveBeenCalled();
  });

  it("calls onUsed only when input actually happened", () => {
    const onUsed = vi.fn();
    startGamepadListener(vi.fn(), onUsed);

    emit({ type: "axis-changed", id: 0, axis: "left-stick-x", value: 0.1 }); // inside deadzone
    vi.advanceTimersByTime(16);
    expect(onUsed).not.toHaveBeenCalled();

    emit({ type: "button-pressed", id: 0, button: "south" });
    expect(onUsed).toHaveBeenCalledOnce();
  });

  it("stops the repeat timer and unlistens when torn down", async () => {
    const stop = startGamepadListener(vi.fn(), vi.fn());
    stop();
    await Promise.resolve(); // the unlisten function only resolves after stop()'s .then()

    expect(unlistenMock).toHaveBeenCalled();
  });
});
