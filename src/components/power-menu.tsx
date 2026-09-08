import { useState } from "react";
import { Modal } from "./modal";
import { List, ListRow } from "./list";
import { ConfirmActionView } from "./confirm-action-view";
import { usePowerMenuStore } from "@/lib/power-menu";
import { useQuit, useRebootDevice, useRestartRelay, useShutdownDevice } from "@/hooks/use-system";

type View = "menu" | "confirm-quit" | "confirm-restart" | "confirm-shutdown" | "confirm-reboot";

// Browsing-mode counterpart to QuickMenu (REL-137's in-game menu) -- opened by the "power" action
// (see lib/power-menu/usePowerMenuListener) while nothing's playing. Deliberately a separate
// component and store from QuickMenu, not a phase-branch of it (REL-138) -- device-level actions
// only (Quit/Restart Relay, Shut Down/Reboot the machine); Settings and profile switching live in
// the header instead (see header.tsx/profile-switcher.tsx), reachable via normal focus navigation
// rather than needing this menu to carry them.
export function PowerMenu() {
  const open = usePowerMenuStore((state) => state.open);
  const closeMenu = usePowerMenuStore((state) => state.closeMenu);
  const [view, setView] = useState<View>("menu");

  const close = () => {
    closeMenu();
    // Deliberately not reset immediately -- same as QuickMenu/GameCardDrawer's own
    // closeAndReset, so the menu doesn't visibly flash back to its default view while still
    // animating shut.
    setTimeout(() => setView("menu"), 200);
  };

  const quit = useQuit();
  const restartRelay = useRestartRelay();
  const shutdownDevice = useShutdownDevice();
  const rebootDevice = useRebootDevice();

  return (
    <Modal open={open} onClose={close} focusKey="POWER_MENU" className="w-[28rem]">
      {view === "menu" && (
        <PowerMenuActions
          onConfirmQuit={() => setView("confirm-quit")}
          onConfirmRestart={() => setView("confirm-restart")}
          onConfirmShutdown={() => setView("confirm-shutdown")}
          onConfirmReboot={() => setView("confirm-reboot")}
        />
      )}
      {view === "confirm-quit" && (
        <ConfirmActionView
          title="Quit Relay?"
          description="Closes Relay and returns to the system console."
          confirmLabel="Quit Relay"
          onBack={() => setView("menu")}
          onConfirm={() => {
            close();
            quit.mutate();
          }}
        />
      )}
      {view === "confirm-restart" && (
        <ConfirmActionView
          title="Restart Relay?"
          description="Closes and reopens Relay."
          confirmLabel="Restart Relay"
          onBack={() => setView("menu")}
          onConfirm={() => {
            close();
            restartRelay.mutate();
          }}
        />
      )}
      {view === "confirm-shutdown" && (
        <ConfirmActionView
          title="Shut down?"
          description="Turns off the device. You'll need to press its power button to turn it back on."
          confirmLabel="Shut Down"
          onBack={() => setView("menu")}
          onConfirm={() => {
            close();
            shutdownDevice.mutate();
          }}
        />
      )}
      {view === "confirm-reboot" && (
        <ConfirmActionView
          title="Reboot?"
          description="Restarts the device. Relay will come back up automatically."
          confirmLabel="Reboot"
          onBack={() => setView("menu")}
          onConfirm={() => {
            close();
            rebootDevice.mutate();
          }}
        />
      )}
    </Modal>
  );
}

function PowerMenuActions({
  onConfirmQuit,
  onConfirmRestart,
  onConfirmShutdown,
  onConfirmReboot,
}: {
  onConfirmQuit: () => void;
  onConfirmRestart: () => void;
  onConfirmShutdown: () => void;
  onConfirmReboot: () => void;
}) {
  return (
    <>
      <h2 className="px-4 pb-2 text-base font-semibold">Power</h2>
      <List>
        <ListRow label="Quit Relay" onSelect={onConfirmQuit} />
        <ListRow label="Restart Relay" onSelect={onConfirmRestart} />
        <ListRow label="Reboot" onSelect={onConfirmReboot} />
        <ListRow label="Shut Down" onSelect={onConfirmShutdown} />
      </List>
    </>
  );
}
