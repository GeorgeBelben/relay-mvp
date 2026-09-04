import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useRumbleEnabled, useSetRumbleEnabled } from "@/hooks/use-settings";
import { ControllerTypePicker } from "@/components/controller-type-picker";
import { WallpaperPicker } from "@/components/wallpaper-picker";
import { VolumeSlider } from "@/components/volume-slider";
import { List, ListRow } from "@/components/list";
import { Header } from "@/components/header";

export const Route = createFileRoute("/settings/ui")({
  component: UiSettings,
});

function UiSettings() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_UI_SCREEN");
  const rumbleEnabled = useRumbleEnabled();
  const setRumbleEnabled = useSetRumbleEnabled();
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">UI</h1>

          <section className="space-y-2">
            <p className="text-sm font-medium">Controller</p>
            <ControllerTypePicker />
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Wallpaper</p>
            <WallpaperPicker />
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Sound</p>
            <VolumeSlider />
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Rumble</p>
            <List>
              <ListRow
                label="Controller Rumble"
                accessory="switch"
                checked={rumbleEnabled}
                onSelect={() => setRumbleEnabled.mutate(!rumbleEnabled)}
              />
            </List>
          </section>
        </div>
      </div>
    </FocusContext.Provider>
  );
}
