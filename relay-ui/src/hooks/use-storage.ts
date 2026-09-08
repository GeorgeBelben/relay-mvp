import { invoke } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";

// Mirrors src-tauri/src/system/storage.rs's StorageUsage -- flat per-category fields, not an
// array, since the category set is fixed (unlike the Electron MVP's breakdown: {category, bytes}[]).
export type StorageUsage = {
  total_bytes: number;
  free_bytes: number;
  games_bytes: number;
  bios_bytes: number;
  media_bytes: number;
  saves_bytes: number;
  system_bytes: number;
};

const QUERY_KEY = ["storage", "usage"];

// A real directory walk, not instant -- fine for a settings screen fetched on mount, but not
// something to refetch on window focus or on an interval. A manual "Refresh" row covers "I just
// deleted some games, update the numbers."
export function useStorageUsage() {
  return useQuery({
    queryKey: QUERY_KEY,
    queryFn: () => invoke<StorageUsage>("get_storage_usage"),
    refetchOnWindowFocus: false,
  });
}

export function useInvalidateStorageUsage() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: QUERY_KEY });
}
