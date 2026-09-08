import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useSetSetting, useSetting } from "@/hooks/use-settings";
import { List } from "@/components/list";
import { TextFieldRow } from "@/components/text-field-row";
import { Header } from "@/components/header";

export const Route = createFileRoute("/settings/metadata")({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const apiKey = useSetting("steamgriddbApiKey");
  const setApiKey = useSetSetting("steamgriddbApiKey");

  const { ref, focusKey } = usePageFocus("SETTINGS_METADATA_SCREEN");
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([
    { action: "confirm", label: "Edit" },
    { action: "back", label: "Back" },
  ]);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">Metadata</h1>
          <List>
            <TextFieldRow label="SteamGridDB API Key" value={apiKey.data ?? ""} onCommit={(value) => setApiKey.mutate(value)} secret />
          </List>
          <p className="mt-4 max-w-md text-sm text-muted-foreground">
            Box art is downloaded from SteamGridDB during a library scan. Grab a free API key from{" "}
            <span className="text-foreground">steamgriddb.com/profile/preferences/api</span> (sign up, then Preferences → API) and
            paste it above. Games already scanned will be matched on the next rescan.
          </p>
        </div>
      </div>
    </FocusContext.Provider>
  );
}
