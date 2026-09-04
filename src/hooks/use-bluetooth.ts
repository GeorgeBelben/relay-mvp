import { invoke } from "@tauri-apps/api/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";

// Mirrors src-tauri/src/system/bluetooth.rs's BluetoothDevice.
export type BluetoothDevice = {
  address: string;
  name: string;
  paired: boolean;
  connected: boolean;
  battery_percent: number | null;
};

export type BluetoothPairError =
  | { reason: "rejected"; message: string }
  | { reason: "unreachable"; message: string }
  | { reason: "unknown"; message: string };

const PAIRED_QUERY_KEY = ["bluetooth", "paired"];
const SCAN_QUERY_KEY = ["bluetooth", "scan"];
const EMPTY_DEVICES: BluetoothDevice[] = [];

export function usePairedDevices() {
  const { data } = useQuery({
    queryKey: PAIRED_QUERY_KEY,
    queryFn: () => invoke<BluetoothDevice[]>("list_paired_bluetooth_devices"),
  });
  return data ?? EMPTY_DEVICES;
}

// A scan takes ~10 real seconds over the air -- same reasoning as useWifiNetworks: no
// refetchOnWindowFocus/interval, fires once on mount so the screen isn't empty, every scan after
// that is the user pressing "Scan for Devices" and calling scan() themselves.
export function useBluetoothScan() {
  const { data, isFetching, refetch, error } = useQuery({
    queryKey: SCAN_QUERY_KEY,
    queryFn: () => invoke<BluetoothDevice[]>("scan_for_bluetooth_devices"),
    refetchOnWindowFocus: false,
    retry: false,
  });

  return { devices: data ?? EMPTY_DEVICES, scanning: isFetching, scan: refetch, error: error as Error | null };
}

// Split out for the same reason as useInvalidateWifiNetworks: lets Settings screens invalidate
// the paired list after a pair/remove without importing react-query directly.
export function useInvalidatePairedDevices() {
  const queryClient = useQueryClient();
  return () => queryClient.invalidateQueries({ queryKey: PAIRED_QUERY_KEY });
}

// Result, not a thrown Error -- pairing rejected vs. device unreachable (BluetoothPairError's
// reason) need to read differently to the player. Resolves to null on success.
export async function pairBluetoothDevice(address: string): Promise<BluetoothPairError | null> {
  try {
    await invoke<void>("pair_bluetooth_device", { address });
    return null;
  } catch (error) {
    return error as BluetoothPairError;
  }
}

export async function removeBluetoothDevice(address: string): Promise<void> {
  await invoke<void>("remove_bluetooth_device", { address });
}
