import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useRescan } from "@/hooks/use-ingest";
import {
  useRetroarchCoresPath,
  useRunAheadEnabled,
  useSetRetroarchCoresPath,
  useSetRunAheadEnabled,
  useSetVideoScaleInteger,
  useSetVideoSmooth,
  useVideoScaleInteger,
  useVideoSmooth,
} from "@/hooks/use-settings";
import { List, ListRow } from "@/components/list";
import { TextFieldRow } from "@/components/text-field-row";
import { Header } from "@/components/header";

export const Route = createFileRoute("/settings/library")({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const coresPath = useRetroarchCoresPath();
  const setCoresPath = useSetRetroarchCoresPath();
  const rescan = useRescan();
  const videoSmooth = useVideoSmooth();
  const setVideoSmooth = useSetVideoSmooth();
  const videoScaleInteger = useVideoScaleInteger();
  const setVideoScaleInteger = useSetVideoScaleInteger();
  const runAheadEnabled = useRunAheadEnabled();
  const setRunAheadEnabled = useSetRunAheadEnabled();
  const { ref, focusKey } = usePageFocus("SETTINGS_LIBRARY_SCREEN");
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
          <h1 className="mb-4 text-3xl font-bold">Library</h1>
          <List>
            {/* No local "scanning..." state needed here -- BootScreen (mounted at the app root)
                already takes over the whole screen for the duration of any rescan, triggered or
                not, and returns here once it's done. */}
            <ListRow label="Rescan Library" onSelect={() => rescan.mutate()} />
          </List>

          <h2 className="mt-8 mb-4 text-xl font-bold">RetroArch</h2>
          <List>
            <TextFieldRow label="Cores Directory" value={coresPath} onCommit={(path) => setCoresPath.mutate(path)} />
          </List>
          <p className="mt-4 max-w-md text-sm text-muted-foreground">
            Where RetroArch's "&lt;core&gt;_libretro.so" files live -- each system's core is resolved against this
            directory when launching a game. Defaults to the apt-installed location on Ubuntu.
          </p>

          <h2 className="mt-8 mb-4 text-xl font-bold">Performance</h2>
          <List>
            <ListRow
              label="Smooth Video"
              accessory="switch"
              checked={videoSmooth}
              onSelect={() => setVideoSmooth.mutate(!videoSmooth)}
            />
            <ListRow
              label="Integer Scaling"
              accessory="switch"
              checked={videoScaleInteger}
              onSelect={() => setVideoScaleInteger.mutate(!videoScaleInteger)}
            />
            <ListRow
              label="Runahead"
              accessory="switch"
              checked={runAheadEnabled}
              onSelect={() => setRunAheadEnabled.mutate(!runAheadEnabled)}
            />
          </List>
          <p className="mt-4 max-w-md text-sm text-muted-foreground">
            Runahead reduces input lag by simulating ahead of what's on screen and rewinding if nothing changed --
            costs extra CPU per frame, off by default. Smooth Video applies bilinear filtering instead of RetroArch's
            default nearest-neighbor scaling.
          </p>
        </div>
      </div>
    </FocusContext.Provider>
  );
}
