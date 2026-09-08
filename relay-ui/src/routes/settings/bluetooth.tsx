import { useState } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { RiBatteryFill, RiBatteryLowFill, RiBluetoothFill } from "@remixicon/react";
import { FocusContext, useBackHandler, useFocusable, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import {
  pairBluetoothDevice,
  removeBluetoothDevice,
  useBluetoothScan,
  useInvalidatePairedDevices,
  usePairedDevices,
  type BluetoothDevice,
} from "@/hooks/use-bluetooth";
import { List, ListRow } from "@/components/list";
import { Modal } from "@/components/modal";
import { Header } from "@/components/header";
import { cn } from "@/lib/cn";

export const Route = createFileRoute("/settings/bluetooth")({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_BLUETOOTH_SCREEN");
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  const paired = usePairedDevices();
  const invalidatePaired = useInvalidatePairedDevices();
  const { devices: discovered, scanning, scan, error } = useBluetoothScan();

  const pairedAddresses = new Set(paired.map((d) => d.address));
  const unpairedDiscovered = discovered.filter((d) => !pairedAddresses.has(d.address));

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">Bluetooth</h1>

          <section className="space-y-2">
            <p className="text-sm font-medium">Paired Devices</p>
            <List>
              {paired.length === 0 && <ListRow label="No devices paired yet" />}
              {paired.map((device) => (
                <PairedDeviceRow key={device.address} device={device} onForgotten={invalidatePaired} />
              ))}
            </List>
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Add a Controller</p>
            <p className="text-sm text-muted-foreground">Press and hold the sync button on your controller, then scan.</p>
            <List>
              <ListRow label="Scan for Devices" onSelect={() => scan()} value={scanning ? "Scanning..." : undefined} />
              {unpairedDiscovered.map((device) => (
                <DiscoveredDeviceRow key={device.address} device={device} onPaired={invalidatePaired} />
              ))}
            </List>
            {error && <p className="mt-2 text-sm text-destructive">{error.message}</p>}
          </section>
        </div>
      </div>
    </FocusContext.Provider>
  );
}

function batteryIcon(percent: number) {
  return percent <= 20 ? RiBatteryLowFill : RiBatteryFill;
}

function PairedDeviceRow({ device, onForgotten }: { device: BluetoothDevice; onForgotten: () => void }) {
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [forgetting, setForgetting] = useState(false);

  const { ref, focused } = useFocusable({ onEnterPress: () => setConfirmOpen(true) });
  const BatteryIcon = device.battery_percent !== null ? batteryIcon(device.battery_percent) : null;

  const forget = async () => {
    setForgetting(true);
    await removeBluetoothDevice(device.address);
    setForgetting(false);
    setConfirmOpen(false);
    onForgotten();
  };

  return (
    <>
      <div
        ref={ref}
        className={cn(
          "flex items-center justify-between gap-3 px-4 py-3 text-sm font-medium rounded transition-bounce",
          focused ? "bg-gray-700 my-1 py-5" : "bg-gray-800",
        )}
      >
        <div className="flex min-w-0 items-center gap-2">
          <RiBluetoothFill className={cn("size-5 shrink-0", device.connected ? "text-white" : "text-gray-600")} aria-hidden="true" />
          <span className="truncate">{device.name}</span>
        </div>
        <div className="flex shrink-0 items-center gap-2 text-muted-foreground">
          {BatteryIcon && device.battery_percent !== null && (
            <span className="flex items-center gap-1">
              <BatteryIcon className="size-4" aria-hidden="true" />
              {device.battery_percent}%
            </span>
          )}
          <span>{device.connected ? "Connected" : "Not connected"}</span>
        </div>
      </div>

      <Modal open={confirmOpen} onClose={() => setConfirmOpen(false)} focusKey="FORGET_DEVICE_MODAL" className="w-[32rem]">
        <div className="space-y-4 rounded-lg bg-gray-800 p-6">
          <h2 className="font-space-grotesk text-lg font-bold">{device.name}</h2>
          <List>
            <ListRow label="Forget Device" onSelect={forget} value={forgetting ? "Forgetting..." : undefined} />
          </List>
        </div>
      </Modal>
    </>
  );
}

type PairState = { phase: "idle" } | { phase: "pairing" } | { phase: "error"; message: string };

function DiscoveredDeviceRow({ device, onPaired }: { device: BluetoothDevice; onPaired: () => void }) {
  const [state, setState] = useState<PairState>({ phase: "idle" });

  const pair = async () => {
    setState({ phase: "pairing" });
    const error = await pairBluetoothDevice(device.address);
    if (!error) {
      setState({ phase: "idle" });
      onPaired();
    } else {
      // Pairing rejected vs. device unreachable need to read differently -- `reason`
      // distinguishes them (see BluetoothPairError's own comment on why this is a tagged return
      // rather than a thrown Error), the message itself is fine to show verbatim either way.
      setState({ phase: "error", message: error.message });
    }
  };

  const { ref, focused } = useFocusable({ onEnterPress: pair });

  return (
    <div
      ref={ref}
      className={cn(
        "flex flex-col gap-1 px-4 py-3 text-sm font-medium rounded transition-bounce",
        focused ? "bg-gray-700 my-1 py-5" : "bg-gray-800",
      )}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <RiBluetoothFill className="size-5 shrink-0 text-gray-400" aria-hidden="true" />
          <span className="truncate">{device.name}</span>
        </div>
        <span className="shrink-0 text-muted-foreground">{state.phase === "pairing" ? "Pairing..." : "Pair"}</span>
      </div>
      {state.phase === "error" && <p className="text-xs text-destructive">{state.message}</p>}
    </div>
  );
}
