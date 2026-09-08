import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import {
  useDeleteProfile,
  useLinkRaConnectAccount,
  useLinkRaWebApi,
  useProfile,
  useRaStats,
  useRenameProfile,
  useUnlinkRa,
} from "@/hooks/use-profiles";
import { useActiveProfileId, useSetActiveProfileId } from "@/hooks/use-settings";
import { List, ListRow } from "@/components/list";
import { TextFieldRow } from "@/components/text-field-row";
import { Header } from "@/components/header";

export const Route = createFileRoute("/settings/profiles/$profileId")({
  component: ProfileDetail,
});

function ProfileDetail() {
  const { profileId } = Route.useParams();
  const navigate = useNavigate();
  const { data: profile } = useProfile(profileId);
  const activeProfileId = useActiveProfileId();
  const { data: stats } = useRaStats(profileId);

  const renameProfile = useRenameProfile();
  const deleteProfile = useDeleteProfile();
  const setActiveProfileId = useSetActiveProfileId();
  const linkWebApi = useLinkRaWebApi();
  const linkConnectAccount = useLinkRaConnectAccount();
  const unlinkRa = useUnlinkRa();

  // Shared between both link flows below -- setWebApiLink/setConnectToken both write the same
  // ra_username column server-side, since they're always the same RA account in practice. Seeded
  // from the profile once it's loaded, editable independently of either commit.
  const [username, setUsername] = useState(profile?.ra_username ?? "");
  useEffect(() => {
    if (profile?.ra_username) setUsername(profile.ra_username);
  }, [profile?.ra_username]);

  const { ref, focusKey } = usePageFocus(`SETTINGS_PROFILE_${profileId}`);
  useBackHandler(() => navigate({ to: "/settings/profiles" }));
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  if (!profile) return null; // profiles list hasn't loaded yet, or this id was just deleted

  const isActive = profile.id === activeProfileId;

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">{profile.name}</h1>

          <List>
            <TextFieldRow label="Name" value={profile.name} onCommit={(name) => renameProfile.mutate({ id: profileId, name })} />
            {!isActive && <ListRow label="Set as Active Profile" onSelect={() => setActiveProfileId.mutate(profileId)} />}
            <ListRow
              label="Delete Profile"
              onSelect={() => {
                deleteProfile.mutate(profileId);
                navigate({ to: "/settings/profiles" });
              }}
            />
          </List>

          <h2 className="mt-8 mb-4 text-xl font-bold">RetroAchievements</h2>
          <List>
            <TextFieldRow label="Username" value={username} onCommit={setUsername} />
            <TextFieldRow
              label="Web API Key"
              value={profile.has_web_api_link ? "linked" : ""}
              secret
              placeholder="Paste your Web API key"
              onCommit={(webApiKey) => {
                if (!username) {
                  toast("Enter a username first");
                  return;
                }
                linkWebApi.mutate(
                  { profileId, username, webApiKey },
                  // link_ra_web_api validates against RA's own API before persisting (see its own
                  // doc comment) -- a bad key/network hiccup rejects invoke() with the Result<(),
                  // String>'s Err value directly (a plain string, not an Error instance).
                  { onError: (error) => toast(`Couldn't link Web API key: ${error instanceof Error ? error.message : String(error)}`) },
                );
              }}
            />
            <TextFieldRow
              label="Password (link achievements)"
              value={profile.has_connect_link ? "linked" : ""}
              secret
              placeholder="Enter your RA password to link"
              onCommit={(password) => {
                if (!username) {
                  toast("Enter a username first");
                  return;
                }
                linkConnectAccount.mutate(
                  { profileId, username, password },
                  { onError: (error) => toast(`Couldn't link RetroAchievements: ${error instanceof Error ? error.message : String(error)}`) },
                );
              }}
            />
            {(profile.has_web_api_link || profile.has_connect_link) && (
              <ListRow label="Unlink RetroAchievements" onSelect={() => unlinkRa.mutate(profileId)} />
            )}
          </List>
          <p className="mt-4 max-w-md text-sm text-muted-foreground">
            The Web API key (from retroachievements.org/settings, "Keys" tab) powers the stats below and achievement matching. The
            password is used once to link in-game unlocks and hardcore mode when launching a game -- it's never stored, only the
            resulting login token is (same approach RetroArch itself uses internally).
          </p>

          {(profile.has_web_api_link || profile.has_connect_link) && (
            <>
              <h2 className="mt-8 mb-4 text-xl font-bold">Stats</h2>
              {stats ? (
                <List>
                  <ListRow label="Points" value={String(stats.points)} />
                  <ListRow label="Rank" value={stats.rank} />
                </List>
              ) : (
                <p className="text-sm text-muted-foreground">No stats cached yet -- refreshes on app start.</p>
              )}
              {stats && stats.recent_unlocks.length > 0 && (
                <List>
                  {stats.recent_unlocks.map((unlock, i) => (
                    <ListRow key={i} label={unlock.title} value={unlock.game_title} />
                  ))}
                </List>
              )}
            </>
          )}
        </div>
      </div>
    </FocusContext.Provider>
  );
}
