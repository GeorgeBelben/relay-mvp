import { useEffect, useRef, useState } from "react";
import { Modal } from "./modal";
import { List, ListRow } from "./list";
import { AchievementsView } from "./achievements-view";
import { VolumeSlider } from "./volume-slider";
import { useLaunchStore } from "@/lib/launch/store";
import {
  useKillGame,
  useLoadStateGame,
  usePauseStandaloneGame,
  usePauseToggleGame,
  useResetGame,
  useResumeStandaloneGame,
  useSaveStateGame,
} from "@/hooks/use-launcher";
import { useSystem } from "@/hooks/use-systems";
import { useActiveProfileId } from "@/hooks/use-settings";
import { useProfile } from "@/hooks/use-profiles";
import { FocusContext, useBackHandler, useFocusable } from "@/lib/focus";
import type { LibraryGame } from "@/hooks/use-library";

type View = "menu" | "achievements" | "confirm-quit" | "confirm-restart";

// Home-button quick menu (REL-23): opened by useQuickMenuListener while a game is playing.
// Mounted once at the app root (see __root.tsx) -- reads the active game straight off
// useLaunchStore rather than taking it as a prop, same reasoning as BootScreen reading its own
// scan status.
export function QuickMenu() {
  const game = useLaunchStore((state) => state.game);
  const open = useLaunchStore((state) => state.quickMenuOpen);
  const closeQuickMenu = useLaunchStore((state) => state.closeQuickMenu);

  const [view, setView] = useState<View>("menu");

  // `game` is null before a launch (this mounts unconditionally at the root -- see the `if
  // (!game) return null` guard below) -- useSystem's `enabled` gate skips the query rather than
  // this needing its own conditional-hook workaround.
  const system = useSystem(game?.system_id);
  const isRetroarchCore = Boolean(system.data?.retroarch_core);

  const pauseToggle = usePauseToggleGame();
  // A standalone binary (PCSX2/Dolphin/yabause-qt) has no remote command interface the way a
  // RetroArch core does -- pauseToggle would be a silent no-op for one, leaving it running,
  // unpaused, behind the menu (REL-147). SIGSTOP/SIGCONT at the OS level is the fallback for
  // those instead.
  const pauseStandalone = usePauseStandaloneGame();
  const resumeStandalone = useResumeStandaloneGame();

  // Exactly one pause/resume per real open<->close transition -- RetroArch's PAUSE_TOGGLE command
  // is a toggle with no separate pause/unpause (see retroarch_command.rs), so firing it more than
  // once per transition (e.g. on every render) would desync from the emulator's actual paused
  // state; SIGSTOP/SIGCONT aren't a toggle at all, so `open` picks which one directly rather than
  // alternating. Mirrors modal.tsx's own "wasOpen" ref pattern for its open/close sound, for the
  // same reason: only a real flip should trigger the side effect, not a re-render with the same
  // `open` value. Guarded on `system.data` being loaded -- by the time a game is actually playing
  // this is effectively always already cached, but guessing which mechanism applies would risk
  // sending the wrong one instead of just skipping a single edge-case transition.
  const wasOpen = useRef(open);
  useEffect(() => {
    if (wasOpen.current !== open && system.data) {
      if (isRetroarchCore) {
        pauseToggle.mutate();
      } else if (open) {
        pauseStandalone.mutate();
      } else {
        resumeStandalone.mutate();
      }
    }
    wasOpen.current = open;
    // The mutation objects are fresh every render -- only `open`/`isRetroarchCore` should ever
    // retrigger this.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, isRetroarchCore]);

  const close = () => {
    closeQuickMenu();
    // Deliberately not reset immediately -- same as GameCardDrawer's closeAndReset, so the menu
    // doesn't visibly flash back to its default view while still animating shut.
    setTimeout(() => setView("menu"), 200);
  };

  // Both destructive (lose progress since the last save) and both confirmed the same way
  // (REL-148/REL-150) -- owned here rather than inside their own confirm sub-view, since the
  // relevant mutation differs per action but the confirm UI itself doesn't.
  const killGame = useKillGame();
  const resetGame = useResetGame();

  // No game yet (shouldn't normally render before a launch, but keeps this safe to mount
  // unconditionally at the root rather than needing a guard at the call site).
  if (!game) return null;

  return (
    <Modal open={open} onClose={close} focusKey="QUICK_MENU" className="w-[28rem]">
      {view === "menu" && (
        <QuickMenuActions
          game={game}
          isRetroarchCore={isRetroarchCore}
          onResume={close}
          onAchievements={() => setView("achievements")}
          onConfirmQuit={() => setView("confirm-quit")}
          onConfirmRestart={() => setView("confirm-restart")}
        />
      )}
      {view === "achievements" && <AchievementsView game={game} onBack={() => setView("menu")} />}
      {view === "confirm-quit" && (
        <ConfirmActionView
          title="Quit without saving?"
          description="Any progress since your last save will be lost."
          confirmLabel="Quit to Relay"
          onBack={() => setView("menu")}
          onConfirm={() => {
            // Optimistic close -- the real "exited" launcher-status push (once the killed
            // process actually exits) resets the launch store fully too (see
            // useLauncherListener.ts's dismiss()), but there's no reason to wait for that
            // round-trip just to close this menu.
            close();
            killGame.mutate();
          }}
        />
      )}
      {view === "confirm-restart" && (
        <ConfirmActionView
          title="Restart without saving?"
          description="Any progress since your last save will be lost."
          confirmLabel="Restart"
          onBack={() => setView("menu")}
          onConfirm={() => {
            resetGame.mutate();
            close();
          }}
        />
      )}
    </Modal>
  );
}

function QuickMenuActions({
  game,
  isRetroarchCore,
  onResume,
  onAchievements,
  onConfirmQuit,
  onConfirmRestart,
}: {
  game: LibraryGame;
  isRetroarchCore: boolean;
  onResume: () => void;
  onAchievements: () => void;
  onConfirmQuit: () => void;
  onConfirmRestart: () => void;
}) {
  const activeProfileId = useActiveProfileId();
  const activeProfile = useProfile(activeProfileId);
  // "if enabled": only offer the Achievements tab for a profile actually linked to
  // RetroAchievements -- an unlinked profile has nothing to show there (see AchievementsView's own
  // null-progress case for the further "not matched to an RA game" case this doesn't cover).
  const achievementsEnabled = Boolean(
    activeProfile.data?.has_web_api_link || activeProfile.data?.has_connect_link,
  );

  const saveState = useSaveStateGame();
  const loadState = useLoadStateGame();

  return (
    <>
      <h2 className="truncate px-4 pb-2 text-base font-semibold">{game.title}</h2>
      <List>
        {/* First row, purely for discoverability -- the same "menu" button that opened this also
            closes it, but that's not otherwise hinted anywhere on screen (REL-152). */}
        <ListRow label="Resume" onSelect={onResume} />
        {achievementsEnabled && <ListRow label="Achievements" onSelect={onAchievements} />}
        {isRetroarchCore && <ListRow label="Save State" onSelect={() => saveState.mutate()} />}
        {isRetroarchCore && <ListRow label="Load State" onSelect={() => loadState.mutate()} />}
        {/* Restart/Quit both go through a confirm step first (REL-148/REL-150) rather than firing
            immediately -- both lose progress since the last save, and a misclick or reflex press
            here used to have no way back. Restart is RetroArch-core only: PCSX2/Dolphin/yabause-qt
            have no equivalent remote-reset interface (same reasoning as Save/Load State above). */}
        {isRetroarchCore && <ListRow label="Restart" onSelect={onConfirmRestart} />}
        <ListRow label="Quit to Relay" onSelect={onConfirmQuit} />
      </List>
      {/* Same focusKey as Settings > UI's own VolumeSlider (REL-151) -- fine since the two are
          never mounted/focused at once (Settings isn't reachable while a game is playing), and
          this reuses the exact same control rather than a bespoke compact copy of it. */}
      <div className="mt-1">
        <VolumeSlider />
      </div>
    </>
  );
}

// Own sub-view rather than an inline conditional inside QuickMenuActions -- same reasoning as
// AchievementsView: a distinct focus subtree needs its own FocusContext + focusSelf() so the
// gamepad lands on it rather than wherever focus was left in the previous view. Shared by both
// Quit and Restart (REL-148/REL-150) -- same confirm shape, only the copy and the mutation that
// fires differ, which the caller supplies rather than this needing to know about either action.
function ConfirmActionView({
  title,
  description,
  confirmLabel,
  onConfirm,
  onBack,
}: {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void;
  onBack: () => void;
}) {
  useBackHandler(onBack);

  const { ref, focusKey, focusSelf } = useFocusable({
    trackChildren: true,
    saveLastFocusedChild: true,
  });
  useEffect(() => {
    focusSelf();
    // Mount-only -- nothing async to wait for here, unlike AchievementsView's own focusSelf
    // effect keyed on its data arriving.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref}>
        <h2 className="px-4 pb-2 text-base font-semibold">{title}</h2>
        <p className="px-4 pb-3 text-sm text-muted-foreground">{description}</p>
        <List>
          <ListRow label={confirmLabel} onSelect={onConfirm} />
          <ListRow label="Back" onSelect={onBack} />
        </List>
      </div>
    </FocusContext.Provider>
  );
}
