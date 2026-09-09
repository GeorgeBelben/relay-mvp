import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { navEvents, rumbleEvents, soundEvents, startInputListeners } from "./nav";
import { useInputMethodStore } from "./store";
import type { BackendGamepadEvent } from "./backendEvent";
import type { NavEvent } from "./types";

// Mounted once at the app root -- starts the keyboard/gamepad polling for the lifetime of the app.
export function useInputListeners() {
  useEffect(() => startInputListeners(), []);
}

export function useNavEvent(listener: (event: NavEvent) => void) {
  useEffect(() => navEvents.subscribe(listener), [listener]);
}

// See nav.ts's soundEvents for why this is a separate stream from useNavEvent above.
export function useSoundEvent(listener: (event: NavEvent) => void) {
  useEffect(() => soundEvents.subscribe(listener), [listener]);
}

// See nav.ts's rumbleEvents for why this is gamepad-only, unlike useSoundEvent above.
export function useRumbleEvent(listener: (event: NavEvent) => void) {
  useEffect(() => rumbleEvents.subscribe(listener), [listener]);
}

export function useLastInputMethod() {
  return useInputMethodStore((state) => state.lastInputMethod);
}

// Just presence, not identity -- no player/profile assignment. Slots are the sorted gamepad ids,
// low to high, so "controller 1/2/3/4" stays stable and predictable rather than tracking which
// physical pad is "assigned" to which slot.
//
// The backend's gamepad watcher starts once at app setup (see lib.rs), before this hook's
// listener necessarily attaches -- list_connected_gamepads pulls whatever's already connected on
// mount (mirroring get_scan_status's same "pull on mount, then stay live via the push event"
// shape), so a controller plugged in before the window loads still shows up immediately rather
// than waiting on its next connect event.
export function useConnectedControllers(): number[] {
  const [connected, setConnected] = useState<number[]>([]);

  useEffect(() => {
    let cancelled = false;
    const ids = new Set<number>();

    invoke<number[]>("list_connected_gamepads").then((initial) => {
      if (cancelled) return;
      for (const id of initial) ids.add(id);
      setConnected([...ids].sort((a, b) => a - b));
    });

    const unlisten = listen<BackendGamepadEvent>("gamepad:event", ({ payload }) => {
      if (payload.type === "connected") ids.add(payload.id);
      else if (payload.type === "disconnected") ids.delete(payload.id);
      else return; // button/axis events don't change presence
      setConnected([...ids].sort((a, b) => a - b));
    });

    return () => {
      cancelled = true;
      unlisten.then((f) => f());
    };
  }, []);

  return connected;
}
