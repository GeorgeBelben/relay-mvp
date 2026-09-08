import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { useMutation, useQuery } from "@tanstack/react-query";

// The currently-running app's own version -- independent of whether an update check has ever
// run, unlike anything check()/Update exposes (a null "no update available" result carries no
// version info at all). Never changes without a restart, so cached indefinitely.
export function useAppVersion() {
  return useQuery({
    queryKey: ["app", "version"],
    queryFn: () => getVersion(),
    staleTime: Infinity,
  });
}

// Tauri's updater plugin is a pull-based promise API (check() -> Update | null, then
// update.downloadAndInstall()), unlike the Electron MVP's push-based update:status channel
// (electron-updater's checking/available/downloading/downloaded events) -- there's no equivalent
// "currently in progress" state to mirror into a store here. A future Settings screen drives its
// own local progress state from downloadAndInstall's onEvent callback directly; this hook just
// exposes the two actions.
export function useCheckForUpdate() {
  return useMutation({
    mutationFn: (): Promise<Update | null> => check(),
  });
}

// The user consents once (choosing to install); downloading, installing, and relaunching happen
// without further input, same as the Electron original. `onEvent` is optional -- passed straight
// through to the plugin's own downloadAndInstall, for a caller (Settings) that wants to render
// download progress; omit it to just fire-and-await.
export function useDownloadAndInstallUpdate() {
  return useMutation({
    mutationFn: async ({ update, onEvent }: { update: Update; onEvent?: (event: DownloadEvent) => void }) => {
      await update.downloadAndInstall(onEvent);
      await relaunch();
    },
  });
}
