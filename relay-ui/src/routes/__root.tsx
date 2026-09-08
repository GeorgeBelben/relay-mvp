import { createRootRouteWithContext, Outlet } from "@tanstack/react-router";
import { Toaster } from "sonner";
import { BackdropLayer } from "@/components/backdrop-layer";
import { BootScreen } from "@/components/boot-screen";
import { HintBar } from "@/components/hint-bar";
import { LaunchOverlay } from "@/components/launch-overlay";
import { QuickMenu } from "@/components/quick-menu";
import { PowerMenu } from "@/components/power-menu";
import { useFocusBridge } from "@/lib/focus";
import { useInputListeners } from "@/lib/input";
import { useRumbleEvents } from "@/lib/rumble";
import { useSoundEvents } from "@/lib/sound";
import { useLauncherListener, useQuickMenuListener } from "@/lib/launch";
import { usePowerMenuListener } from "@/lib/power-menu";
import type { RouterContext } from "@/router";

// Deliberately minimal -- add shared kiosk chrome here as it's built (on-screen keyboard, etc. --
// most of it lived at this layer in the Electron MVP too). BackdropLayer is the one thing here
// that didn't: the Electron MVP had a static user-picked wallpaper background instead, which this
// rewrite dropped in favor of a per-game backdrop that crossfades in behind Home's carousel as
// focus moves across it (see lib/backdrop's store and backdrop-layer.tsx) -- mounted first so it
// paints behind everything else. useLauncherListener drives the launch-phase store off the
// "launcher:status" push; LaunchOverlay renders whatever phase it lands on (fade to black +
// pulsing logo while launching, a fade-back-out once "playing", or an error message) -- see
// launch-overlay.tsx. QuickMenu (REL-137) is the in-game counterpart -- it renders itself already,
// gated on the same store's quickMenuOpen, and owns "menu" while a game is playing. PowerMenu is
// the browsing-mode counterpart to that -- a separate component/store, opened by the distinct
// "power" action instead (Quit/Restart Relay, Shut Down/Reboot the device); Settings and profile
// switching live in the header instead (see header.tsx/profile-switcher.tsx), reachable via normal
// focus navigation. Several routes and components already call useActionHints (routes/index.tsx,
// games.tsx, systems/$systemId.tsx, power-menu.tsx) -- those hints just had nowhere to render
// until HintBar was mounted here.
function RootLayout() {
  useInputListeners();
  useFocusBridge();
  useSoundEvents();
  useRumbleEvents();
  useLauncherListener();
  useQuickMenuListener();
  usePowerMenuListener();

  return (
    <div className="h-svh w-full flex flex-col">
      <BackdropLayer />
      <BootScreen>
        <Outlet />
      </BootScreen>
      <HintBar />
      <LaunchOverlay />
      <QuickMenu />
      <PowerMenu />
      <Toaster theme="dark" />
    </div>
  );
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
});
