import { useEffect, useRef, useState } from "react";
import { Modal } from "./modal";
import { List, ListRow } from "./list";
import { AchievementsView } from "./achievements-view";
import { useLaunchStore } from "@/lib/launch/store";
import {
  useKillGame,
  usePauseStandaloneGame,
  usePauseToggleGame,
  useResumeStandaloneGame,
  useSaveStateGame,
} from "@/hooks/use-launcher";
import { useSystem } from "@/hooks/use-systems";
import { useActiveProfileId } from "@/hooks/use-settings";
import { useProfile } from "@/hooks/use-profiles";
import type { LibraryGame } from "@/hooks/use-library";

type View = "menu" | "achievements";

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

  // No game yet (shouldn't normally render before a launch, but keeps this safe to mount
  // unconditionally at the root rather than needing a guard at the call site).
  if (!game) return null;

  return (
    <Modal open={open} onClose={close} focusKey="QUICK_MENU" className="w-[28rem]">
      {view === "menu" && (
        <QuickMenuActions
          game={game}
          isRetroarchCore={isRetroarchCore}
          onAchievements={() => setView("achievements")}
          onClose={close}
        />
      )}
      {view === "achievements" && <AchievementsView game={game} onBack={() => setView("menu")} />}
    </Modal>
  );
}

function QuickMenuActions({
  game,
  isRetroarchCore,
  onAchievements,
  onClose,
}: {
  game: LibraryGame;
  isRetroarchCore: boolean;
  onAchievements: () => void;
  onClose: () => void;
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
  const killGame = useKillGame();

  return (
    <>
      <h2 className="truncate px-4 pb-2 text-base font-semibold">{game.title}</h2>
      <List>
        {achievementsEnabled && <ListRow label="Achievements" onSelect={onAchievements} />}
        {isRetroarchCore && <ListRow label="Save State" onSelect={() => saveState.mutate()} />}
        <ListRow
          label="Quit to Relay"
          onSelect={() => {
            // Optimistic close -- the real "exited" launcher-status push (once the killed
            // process actually exits) resets the launch store fully too (see
            // useLauncherListener.ts's dismiss()), but there's no reason to wait for that
            // round-trip just to close this menu.
            onClose();
            killGame.mutate();
          }}
        />
      </List>
    </>
  );
}
