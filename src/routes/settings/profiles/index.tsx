import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useCreateProfile, useProfiles } from "@/hooks/use-profiles";
import { useActiveProfileId } from "@/hooks/use-settings";
import { List, ListRow } from "@/components/list";
import { Header } from "@/components/header";

export const Route = createFileRoute("/settings/profiles/")({
  component: ProfilesSettings,
});

// Deliberately minimal: a flat list you add to and pick an active one from in Settings, nothing
// more -- no controller-to-profile assignment, no boot-time switcher.
function ProfilesSettings() {
  const navigate = useNavigate();
  const { data: profiles = [] } = useProfiles();
  const activeProfileId = useActiveProfileId();
  const createProfile = useCreateProfile();

  const { ref, focusKey } = usePageFocus("SETTINGS_PROFILES_SCREEN");
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
          <h1 className="mb-4 text-3xl font-bold">Profiles</h1>
          <List>
            {profiles.map((profile) => (
              <ListRow
                key={profile.id}
                label={profile.name}
                value={profile.id === activeProfileId ? "Active" : undefined}
                to={`/settings/profiles/${profile.id}`}
              />
            ))}
            <ListRow label="Add Profile" onSelect={() => createProfile.mutate(`Player ${profiles.length + 1}`)} />
          </List>
          <p className="mt-4 max-w-md text-sm text-muted-foreground">
            Each profile can link its own RetroAchievements account, so unlocks, hardcore mode, and stats are tracked per player. Select
            a profile to link an account, rename it, or make it the active one. Games are matched to RetroAchievements automatically
            during a library scan, by hashing each ROM the same way RetroAchievements itself does -- currently supported: NES, SNES,
            N64, Game Boy, Game Boy Color, Game Boy Advance, Nintendo DS, Sega Master System, Game Gear, Genesis/Mega Drive, Neo Geo
            Pocket/Color, and WonderSwan/Color.
          </p>
        </div>
      </div>
    </FocusContext.Provider>
  );
}
