import { useEffect, useState } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { Header } from "@/components/header";
import { cn } from "@/lib/cn";
import type { BackendGamepadEvent, GamepadAxis, GamepadButton } from "@/lib/input/backendEvent";

export const Route = createFileRoute("/settings/debug")({
  component: RouteComponent,
});

type PadState = {
  id: number;
  name: string;
  buttons: Set<GamepadButton>;
  axes: Partial<Record<GamepadAxis, number>>;
};

// Every button relay_core::system::gamepad can report, in a sensible reading order for a debug
// grid (face buttons, dpad, shoulders/triggers, sticks-as-buttons, menu buttons) -- not the same
// order as BUTTON_ACTION_MAP in gamepad.ts, which only cares about the handful that drive nav.
const ALL_BUTTONS: GamepadButton[] = [
  "north",
  "west",
  "east",
  "south",
  "d-pad-up",
  "d-pad-left",
  "d-pad-right",
  "d-pad-down",
  "left-shoulder",
  "left-trigger",
  "right-shoulder",
  "right-trigger",
  "left-stick",
  "right-stick",
  "select",
  "start",
  "mode",
];

function emptyPad(id: number): PadState {
  return { id, name: `Gamepad ${id}`, buttons: new Set(), axes: {} };
}

// Raw introspection of every event relay_core::system::gamepad emits -- deliberately not reusing
// gamepad.ts's startGamepadListener, which only cares about the handful of buttons/axes that
// drive menu nav. This is for diagnosing the controller itself (is a button actually registering,
// which axis is which, is a stick inverted, is a bad receiver spamming phantom events) -- exactly
// the kind of thing that took a standalone `cargo run --example gamepad-test` to see before this
// existed.
function useGamepadDebugState(): PadState[] {
  const [pads, setPads] = useState<Map<number, PadState>>(new Map());

  useEffect(() => {
    invoke<number[]>("list_connected_gamepads").then((ids) => {
      setPads((prev) => {
        const next = new Map(prev);
        for (const id of ids) {
          if (!next.has(id)) next.set(id, emptyPad(id));
        }
        return next;
      });
    });

    const unlisten = listen<BackendGamepadEvent>("gamepad:event", ({ payload }) => {
      setPads((prev) => {
        const next = new Map(prev);
        const pad = next.get(payload.id) ?? emptyPad(payload.id);

        switch (payload.type) {
          case "connected":
            next.set(payload.id, { ...pad, name: payload.name });
            break;
          case "disconnected":
            next.delete(payload.id);
            break;
          case "button-pressed":
            next.set(payload.id, { ...pad, buttons: new Set(pad.buttons).add(payload.button) });
            break;
          case "button-released": {
            const buttons = new Set(pad.buttons);
            buttons.delete(payload.button);
            next.set(payload.id, { ...pad, buttons });
            break;
          }
          case "axis-changed":
            next.set(payload.id, { ...pad, axes: { ...pad.axes, [payload.axis]: payload.value } });
            break;
        }
        return next;
      });
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return [...pads.values()].sort((a, b) => a.id - b.id);
}

function RouteComponent() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_DEBUG_SCREEN");
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([{ action: "back", label: "Back" }]);

  const pads = useGamepadDebugState();

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-1 text-3xl font-bold">Debug</h1>
          <p className="mb-4 text-sm text-muted-foreground">Controller test -- raw input straight from relay-core's gamepad watcher, not the nav layer.</p>

          {pads.length === 0 ? (
            <p className="text-sm text-muted-foreground">No controller connected.</p>
          ) : (
            <div className="space-y-6">
              {pads.map((pad) => (
                <GamepadDebugCard key={pad.id} pad={pad} />
              ))}
            </div>
          )}
        </div>
      </div>
    </FocusContext.Provider>
  );
}

function GamepadDebugCard({ pad }: { pad: PadState }) {
  return (
    <section className="rounded-lg border border-border p-4">
      <h2 className="mb-3 text-lg font-semibold">
        {pad.name} <span className="text-sm font-normal text-muted-foreground">(id {pad.id})</span>
      </h2>

      <div className="mb-4 flex flex-wrap gap-2">
        {ALL_BUTTONS.map((button) => (
          <span
            key={button}
            className={cn(
              "rounded-md border px-2.5 py-1 text-xs font-medium",
              pad.buttons.has(button) ? "border-primary bg-primary text-primary-foreground" : "border-border text-muted-foreground",
            )}
          >
            {button}
          </span>
        ))}
      </div>

      <div className="flex gap-8">
        <StickVisualizer label="Left stick" x={pad.axes["left-stick-x"] ?? 0} y={pad.axes["left-stick-y"] ?? 0} />
        <StickVisualizer label="Right stick" x={pad.axes["right-stick-x"] ?? 0} y={pad.axes["right-stick-y"] ?? 0} />
      </div>
    </section>
  );
}

// A raw x/y dot inside a box -- deliberately shows the unfiltered axis value (no deadzone
// applied), so a stick that's drifting or inverted is visually obvious rather than hidden behind
// the same deadzone/rounding the real nav logic applies.
function StickVisualizer({ label, x, y }: { label: string; x: number; y: number }) {
  const clamp = (n: number) => Math.max(-1, Math.min(1, n));
  const cx = clamp(x);
  const cy = clamp(y);

  return (
    <div className="flex flex-col items-center gap-2">
      <span className="text-xs text-muted-foreground">{label}</span>
      <div className="relative size-20 rounded-full border border-border bg-muted/30">
        <div
          className="absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-primary"
          style={{ left: `${((cx + 1) / 2) * 100}%`, top: `${((cy + 1) / 2) * 100}%` }}
        />
      </div>
      <span className="font-mono text-xs text-muted-foreground">
        {cx.toFixed(2)}, {cy.toFixed(2)}
      </span>
    </div>
  );
}
