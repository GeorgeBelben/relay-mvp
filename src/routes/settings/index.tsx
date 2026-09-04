import { useState } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import type { Update } from "@tauri-apps/plugin-updater";
import { FOCUS_RING, FocusContext, useBackHandler, useFocusable, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useAppVersion, useCheckForUpdate, useDownloadAndInstallUpdate } from "@/hooks/use-updates";
import { useQuit } from "@/hooks/use-system";
import { List, ListRow } from "@/components/list";
import { Modal } from "@/components/modal";
import { Header } from "@/components/header";
import { cn } from "@/lib/cn";

export const Route = createFileRoute("/settings/")({
  component: Settings,
});

// Unlike the Electron MVP's push-based update:status channel (a store fed by main-process
// events), Tauri's updater plugin is a pull-based promise API (check() -> Update | null, then
// downloadAndInstall()) -- there's no "currently in progress" state to mirror from anywhere else,
// so this screen owns that state machine itself rather than reading it off a store.
type UpdateState =
  | { state: "idle" }
  | { state: "checking" }
  | { state: "not-available" }
  | { state: "available"; update: Update }
  | { state: "downloading"; update: Update; percent: number }
  | { state: "error"; message: string };

function Settings() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_SCREEN");
  const version = useAppVersion();
  const checkForUpdate = useCheckForUpdate();
  const downloadAndInstall = useDownloadAndInstallUpdate();
  const quit = useQuit();

  const [updateState, setUpdateState] = useState<UpdateState>({ state: "idle" });

  useBackHandler(() => navigate({ to: "/" }));

  // UpdateDialog registers its own (higher-priority) hints while open, correctly shadowing
  // these via the hint stack -- no need to duplicate that check here.
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  const checking = updateState.state === "checking";
  const dialogOpen = updateState.state === "available" || updateState.state === "downloading";

  const runCheck = async () => {
    if (checking) return;
    setUpdateState({ state: "checking" });
    try {
      const update = await checkForUpdate.mutateAsync();
      setUpdateState(update ? { state: "available", update } : { state: "not-available" });
    } catch (err) {
      setUpdateState({ state: "error", message: err instanceof Error ? err.message : "Update check failed" });
    }
  };

  // relaunch() (inside downloadAndInstall's own mutationFn) exits the process on success --
  // nothing meaningfully runs after that await resolves, so there's no further state transition
  // to make for the happy path, only the error one.
  const install = async () => {
    if (updateState.state !== "available") return;
    const update = updateState.update;
    setUpdateState({ state: "downloading", update, percent: 0 });

    let totalBytes = 0;
    let downloadedBytes = 0;
    try {
      await downloadAndInstall.mutateAsync({
        update,
        onEvent: (event) => {
          if (event.event === "Started") {
            totalBytes = event.data.contentLength ?? 0;
          } else if (event.event === "Progress") {
            downloadedBytes += event.data.chunkLength;
            const percent = totalBytes > 0 ? Math.round((downloadedBytes / totalBytes) * 100) : 0;
            setUpdateState({ state: "downloading", update, percent });
          }
        },
      });
    } catch (err) {
      setUpdateState({ state: "error", message: err instanceof Error ? err.message : "Update install failed" });
    }
  };

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">Settings</h1>

          <section className="space-y-2">
            <List>
              <ListRow label="Version" value={version.data ?? "..."} />
              <ListRow label="Check for Updates" value={checking ? "Checking..." : undefined} onSelect={runCheck} />
              <ListRow label="UI" to="/settings/ui" />
              <ListRow label="Network" to="/settings/network" />
              <ListRow label="Bluetooth" to="/settings/bluetooth" />
              <ListRow label="Date & Time" to="/settings/datetime" />
              <ListRow label="Storage" to="/settings/storage" />
              <ListRow label="Library" to="/settings/library" />
              <ListRow label="Metadata" to="/settings/metadata" />
              <ListRow label="Profiles" to="/settings/profiles" />
            </List>
            {updateState.state === "not-available" && (
              <p className="text-sm text-muted-foreground">You're on the latest version.</p>
            )}
            {updateState.state === "error" && <p className="text-sm text-destructive">Update check failed: {updateState.message}</p>}
          </section>

          <section className="mt-6 space-y-2">
            <List>
              <ListRow label="Quit" onSelect={() => quit.mutate()} />
            </List>
            <p className="text-sm text-muted-foreground">Closes Relay and returns to the system console.</p>
          </section>
        </div>
      </div>

      <UpdateDialog open={dialogOpen} state={updateState} onInstall={install} onDismiss={() => setUpdateState({ state: "idle" })} />
    </FocusContext.Provider>
  );
}

function UpdateDialog({
  open,
  state,
  onInstall,
  onDismiss,
}: {
  open: boolean;
  state: UpdateState;
  onInstall: () => void;
  onDismiss: () => void;
}) {
  useActionHints(
    !open
      ? null // not on screen -- let Settings' own hints show through
      : state.state === "available"
        ? [
            { action: "confirm", label: "Install and Restart" },
            { action: "back", label: "Not Now" },
          ]
        : [], // downloading: on screen, deliberately zero actions available
  );

  return (
    <Modal
      open={open}
      focusKey="UPDATE_DIALOG"
      // Downloading is past the point of no return -- Back is swallowed rather than dismissing
      // the dialog, so you can't wander off mid-install.
      onClose={() => {
        if (state.state === "available") onDismiss();
      }}
    >
      {state.state === "available" && (
        <>
          <h2 className="text-base font-semibold">Update available</h2>
          <p className="text-sm text-muted-foreground">Version {state.update.version} is ready to install.</p>
          <InstallButton onInstall={onInstall} />
        </>
      )}
      {state.state === "downloading" && (
        <>
          <h2 className="text-base font-semibold">Downloading update</h2>
          <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
            <div className="h-full bg-primary transition-[width]" style={{ width: `${state.percent}%` }} />
          </div>
        </>
      )}
    </Modal>
  );
}

function InstallButton({ onInstall }: { onInstall: () => void }) {
  const { ref, focused } = useFocusable({ onEnterPress: onInstall });

  return (
    <div
      ref={ref}
      className={cn("w-full rounded-md bg-primary px-3 py-1.5 text-center text-sm text-primary-foreground", focused && FOCUS_RING)}
    >
      Install and Restart
    </div>
  );
}
