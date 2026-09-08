import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { useMutation, useQuery } from "@tanstack/react-query";

export function useUsername(): string | undefined {
  const { data } = useQuery({ queryKey: ["system", "username"], queryFn: () => invoke<string>("get_username") });
  return data;
}

// The kiosk systemd unit runs this app as the only thing on screen -- quitting is what actually
// drops back to the console's login/CLI, not a browser-style "close tab" (commands::system::quit).
export function useQuit() {
  return useMutation({
    mutationFn: () => invoke<void>("quit"),
  });
}

// Power Menu's "Restart Relay" -- the plugin's own relaunch() exits this process and starts a
// fresh one (same mechanism Settings' update-install flow already uses, see use-updates.ts), so
// there's nothing left to await/react to on success.
export function useRestartRelay() {
  return useMutation({
    mutationFn: () => relaunch(),
  });
}

// Power Menu's "Shut Down"/"Reboot" -- these act on the physical device, not just this app (see
// commands::system::shutdown_device/reboot_device's own doc comments for the polkit prerequisite).
export function useShutdownDevice() {
  return useMutation({
    mutationFn: () => invoke<void>("shutdown_device"),
  });
}

export function useRebootDevice() {
  return useMutation({
    mutationFn: () => invoke<void>("reboot_device"),
  });
}
