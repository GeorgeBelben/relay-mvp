import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { navEvents } from "@/lib/input/nav";
import { useLaunchStore } from "@/lib/launch/store";
import { usePowerMenuStore } from "./store";
import { usePowerMenuListener } from "./usePowerMenuListener";

const game = { id: "g1", title: "Chrono Trigger", system_id: "snes" } as Parameters<
  ReturnType<typeof useLaunchStore.getState>["start"]
>[0];

describe("usePowerMenuListener", () => {
  beforeEach(() => {
    useLaunchStore.setState({ phase: "idle", game: null, message: null, quickMenuOpen: false });
    usePowerMenuStore.setState({ open: false });
  });

  it("toggles the power menu open then closed on repeated power presses while browsing", () => {
    renderHook(() => usePowerMenuListener());

    navEvents.emit({ type: "action", action: "power" });
    expect(usePowerMenuStore.getState().open).toBe(true);

    navEvents.emit({ type: "action", action: "power" });
    expect(usePowerMenuStore.getState().open).toBe(false);
  });

  it("ignores a power press while a game is playing (REL-137's quick menu owns that context instead)", () => {
    useLaunchStore.getState().start(game);
    useLaunchStore.getState().enterPlaying();
    renderHook(() => usePowerMenuListener());

    navEvents.emit({ type: "action", action: "power" });

    expect(usePowerMenuStore.getState().open).toBe(false);
  });

  it("ignores non-power actions", () => {
    renderHook(() => usePowerMenuListener());

    navEvents.emit({ type: "action", action: "menu" });
    navEvents.emit({ type: "action", action: "confirm" });
    navEvents.emit({ type: "direction", direction: "up" });

    expect(usePowerMenuStore.getState().open).toBe(false);
  });
});
