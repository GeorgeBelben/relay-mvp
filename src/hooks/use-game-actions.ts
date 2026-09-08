import { invoke } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

// Mirrors src-tauri/src/game_actions.rs's ReidentifyCandidate -- boxart_url here is the *remote*
// SteamGridDB image, straight from their CDN, for preview only. Nothing's downloaded or persisted
// until the user actually picks one (see useApplyReidentify).
export type ReidentifyCandidate = {
  steamgriddb_id: number;
  title: string;
  boxart_url: string | null;
};

// A free-text search (not tied to a game_id -- the caller supplies whatever query it wants,
// typically prefilled from the game's scanned_title but freely editable), so a mutation rather
// than a query with a stable cache key fits the same one-shot, on-demand shape as useCreateGame/
// useRescan elsewhere in this file set.
export function useSearchForReidentify() {
  return useMutation({
    mutationFn: (query: string) => invoke<ReidentifyCandidate[]>("search_for_reidentify", { query }),
  });
}

export function useApplyReidentify() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { gameId: string; steamgriddbId: number; title: string }) =>
      invoke<void>("apply_reidentify", { gameId: args.gameId, steamgriddbId: args.steamgriddbId, title: args.title }),
    onSuccess: () => {
      // Reidentifying changes the game's title/boxart, which every library view surfaces.
      queryClient.invalidateQueries({ queryKey: ["games"] });
      queryClient.invalidateQueries({ queryKey: ["library"] });
    },
  });
}

// Every image file currently sitting in this game's media folder -- whether it got there via
// auto-enrich, a reidentify, or the user dropping it in directly via the filesystem. A real query
// (not a mutation): passive, refetched every time the Artwork view opens (`refetchOnMount:
// "always"`) since files can appear there without this app's involvement.
export function useListGameMediaFiles(gameId: string) {
  return useQuery({
    queryKey: ["game-media-files", gameId],
    queryFn: () => invoke<string[]>("list_game_media_files", { gameId }),
    refetchOnMount: "always",
  });
}

// Points a game's active box art at a file already in its media folder -- no download, no network
// call, just a DB row update (see game_media_files::select_boxart_file).
export function useSelectBoxartFile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (args: { gameId: string; filename: string }) =>
      invoke<void>("select_boxart_file", { gameId: args.gameId, filename: args.filename }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["games"] });
      queryClient.invalidateQueries({ queryKey: ["library"] });
    },
  });
}

// Mirrors src-tauri/src/game_actions.rs's AchievementView/GameAchievementsProgress. badge_url is
// already resolved to the locked/unlocked variant server-side.
export type Achievement = {
  id: number;
  title: string;
  description: string;
  points: number;
  badge_url: string;
  unlocked: boolean;
};

export type GameAchievementsProgress = {
  game_id: number;
  title: string;
  console_name: string;
  num_achievements: number;
  num_awarded_to_user: number;
  user_completion: string;
  highest_award_kind: string | null;
  achievements: Achievement[];
};

// null means "this game isn't matched to a RetroAchievements entry" (or no profile is
// RA-linked) -- a normal, expected outcome, not an error. A query rather than a mutation, unlike
// the two above: this backs a passive "show current progress" view (the drawer's Achievements
// tab, tile focus), not a one-shot user action, even though the backend also opportunistically
// persists the fetched highest_award_kind as a side effect.
// `enabled` defaults to true (the drawer's own Achievements view always wants this fetched), but
// a grid that renders many tiles at once (see game-tile.tsx's All Games usage) needs to gate this
// on focus -- without it, mounting hundreds of tiles fires hundreds of simultaneous achievement
// lookups instead of the one for whichever tile is actually highlighted.
export function useAchievements(gameId: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: ["games", gameId, "achievements"],
    queryFn: () => invoke<GameAchievementsProgress | null>("get_achievements", { gameId }),
    enabled: options?.enabled ?? true,
  });
}
