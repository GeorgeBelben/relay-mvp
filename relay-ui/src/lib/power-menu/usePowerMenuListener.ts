import { useNavEvent } from "@/lib/input";
import { useLaunchStore } from "@/lib/launch/store";
import { usePowerMenuStore } from "./store";

// Mounted once at the app root (see __root.tsx), alongside useQuickMenuListener. The "power"
// action (gamepad Guide/PS/Xbox button, or Backspace on keyboard -- see lib/input/keyboard.ts)
// toggles this menu, but only while browsing -- while a game is playing, "menu" already owns the
// equivalent role via REL-137's in-game QuickMenu, and this deliberately doesn't also try to
// cover that case (see REL-138 for why these stayed two separate surfaces rather than one
// phase-branched component).
export function usePowerMenuListener() {
  const toggleMenu = usePowerMenuStore((state) => state.toggleMenu);

  useNavEvent((event) => {
    if (event.type !== "action" || event.action !== "power") return;
    if (useLaunchStore.getState().phase === "playing") return;
    toggleMenu();
  });
}
