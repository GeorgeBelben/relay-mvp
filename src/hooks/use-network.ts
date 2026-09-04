import { invoke } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";

// Mirrors src-tauri/src/system/network.rs's WifiNetwork.
export type WifiNetwork = {
  ssid: string;
  signal: number;
  secured: boolean;
  in_use: boolean;
};

export type WifiConnectError =
  | { reason: "wrong-password"; message: string }
  | { reason: "unreachable"; message: string }
  | { reason: "unknown"; message: string };

const QUERY_KEY = ["network", "wifi"];
const EMPTY_NETWORKS: WifiNetwork[] = [];

// A scan is a real over-the-air operation nmcli itself takes a few seconds over -- deliberately
// no refetchOnWindowFocus/refetchInterval here, unlike most query hooks in this app. It still
// fires once on mount so the screen isn't empty, but every scan after that is the user explicitly
// pressing "Scan for Networks" and calling scan() themselves.
export function useWifiNetworks() {
  const { data, isFetching, refetch, error } = useQuery({
    queryKey: QUERY_KEY,
    queryFn: () => invoke<WifiNetwork[]>("list_wifi_networks"),
    refetchOnWindowFocus: false,
    retry: false,
  });

  return { networks: data ?? EMPTY_NETWORKS, scanning: isFetching, scan: refetch, error: error as Error | null };
}

// Split out from useWifiNetworks above only so a caller can invalidate the list after a
// successful connect without importing react-query directly itself.
export function useInvalidateWifiNetworks() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: QUERY_KEY });
}

// Result, not a thrown Error -- wrong password vs. connection failure (WifiConnectError's reason)
// need to read differently to the player, and invoke() rejecting loses that shape down to a plain
// string message. Resolves to null on success.
export async function connectToWifiNetwork(ssid: string, password?: string): Promise<WifiConnectError | null> {
  try {
    await invoke<void>("connect_to_wifi_network", { ssid, password: password ?? null });
    return null;
  } catch (error) {
    return error as WifiConnectError;
  }
}
