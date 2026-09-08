import { useState } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { RiLockFill, RiWifiFill } from "@remixicon/react";
import { FocusContext, useBackHandler, useFocusable, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { connectToWifiNetwork, useInvalidateWifiNetworks, useWifiNetworks, type WifiNetwork } from "@/hooks/use-network";
import { List, ListRow } from "@/components/list";
import { TextFieldRow } from "@/components/text-field-row";
import { Modal } from "@/components/modal";
import { Header } from "@/components/header";
import { cn } from "@/lib/cn";

export const Route = createFileRoute("/settings/network")({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_NETWORK_SCREEN");
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  const { networks, scanning, scan, error } = useWifiNetworks();
  const invalidate = useInvalidateWifiNetworks();
  const connected = networks.find((n) => n.in_use);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">Network</h1>

          <section className="space-y-2">
            <p className="text-sm font-medium">Status</p>
            <List>
              <ListRow label={connected ? connected.ssid : "Not connected"} value={connected ? `${connected.signal}%` : undefined} />
            </List>
          </section>

          <section className="mt-6 space-y-2">
            <p className="text-sm font-medium">Networks</p>
            <List>
              <ListRow label="Scan for Networks" onSelect={() => scan()} value={scanning ? "Scanning..." : undefined} />
              {networks
                .filter((n) => !n.in_use)
                .map((network) => (
                  <WifiNetworkRow key={network.ssid} network={network} onConnected={invalidate} />
                ))}
            </List>
            {error && <p className="mt-2 text-sm text-destructive">{error.message}</p>}
          </section>
        </div>
      </div>
    </FocusContext.Provider>
  );
}

// nmcli's SIGNAL is 0-100 with no fixed step count to map onto discrete bar icons -- three opacity
// tiers on a single RiWifiFill reads clearly enough at this screen's icon size without needing
// four separate bar-count icon assets.
function signalColorClass(signal: number): string {
  if (signal >= 67) return "text-white";
  if (signal >= 34) return "text-gray-400";
  return "text-gray-600";
}

type ConnectState = { phase: "idle" } | { phase: "connecting" } | { phase: "error"; message: string };

function WifiNetworkRow({ network, onConnected }: { network: WifiNetwork; onConnected: () => void }) {
  const [passwordModalOpen, setPasswordModalOpen] = useState(false);
  const [connectState, setConnectState] = useState<ConnectState>({ phase: "idle" });

  const connect = async (password?: string) => {
    setConnectState({ phase: "connecting" });
    const error = await connectToWifiNetwork(network.ssid, password);
    if (!error) {
      setPasswordModalOpen(false);
      setConnectState({ phase: "idle" });
      onConnected();
    } else {
      // Wrong password vs. connection failure need to read differently -- `reason` distinguishes
      // them (see WifiConnectError's own comment on why this is a tagged return, not a thrown
      // Error), but the message nmcli itself produced is fine to show verbatim either way.
      setConnectState({ phase: "error", message: error.message });
    }
  };

  const handleSelect = () => {
    if (network.secured) {
      setPasswordModalOpen(true);
    } else {
      connect();
    }
  };

  const { ref, focused } = useFocusable({ onEnterPress: handleSelect });

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
          <RiWifiFill className={cn("size-5 shrink-0", signalColorClass(network.signal))} aria-hidden="true" />
          <span className="truncate">{network.ssid}</span>
          {network.secured && <RiLockFill className="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />}
        </div>
        <span className="shrink-0 text-muted-foreground">
          {connectState.phase === "connecting" && !network.secured ? "Connecting..." : `${network.signal}%`}
        </span>
      </div>

      {network.secured && (
        <WifiPasswordModal
          ssid={network.ssid}
          open={passwordModalOpen}
          connectState={connectState}
          onClose={() => {
            setPasswordModalOpen(false);
            setConnectState({ phase: "idle" });
          }}
          onSubmit={connect}
        />
      )}
    </>
  );
}

function WifiPasswordModal({
  ssid,
  open,
  connectState,
  onClose,
  onSubmit,
}: {
  ssid: string;
  open: boolean;
  connectState: ConnectState;
  onClose: () => void;
  onSubmit: (password: string) => void;
}) {
  useActionHints(open ? [{ action: "back", label: "Cancel" }] : null);

  return (
    <Modal open={open} onClose={onClose} focusKey="WIFI_PASSWORD_MODAL" className="w-[32rem]">
      <div className="space-y-4 rounded-lg bg-gray-800 p-6">
        <div className="flex items-center gap-2">
          <RiWifiFill className="size-5 text-white" aria-hidden="true" />
          <h2 className="font-space-grotesk text-lg font-bold">{ssid}</h2>
        </div>
        <List>
          {/* value reset to "" on every render on purpose -- a wrong-password attempt should be
              cleared, not left sitting in the field as if it might now be correct. */}
          <TextFieldRow label="Password" value="" onCommit={onSubmit} secret placeholder="Enter password" />
        </List>
        {connectState.phase === "connecting" && <p className="text-sm text-muted-foreground">Connecting...</p>}
        {connectState.phase === "error" && <p className="text-sm text-destructive">{connectState.message}</p>}
      </div>
    </Modal>
  );
}
