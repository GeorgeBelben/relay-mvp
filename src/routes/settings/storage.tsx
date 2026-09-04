import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { FocusContext, useBackHandler, usePageFocus } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { useInvalidateStorageUsage, useStorageUsage } from "@/hooks/use-storage";
import { formatBytes } from "@/lib/storage/formatBytes";
import { CATEGORY_META, StorageBar, storageBreakdown } from "@/components/storage-bar";
import { List, ListRow } from "@/components/list";
import { Header } from "@/components/header";
import { cn } from "@/lib/cn";

export const Route = createFileRoute("/settings/storage")({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const { ref, focusKey } = usePageFocus("SETTINGS_STORAGE_SCREEN");
  useBackHandler(() => navigate({ to: "/settings" }));
  useActionHints([
    { action: "confirm", label: "Select" },
    { action: "back", label: "Back" },
  ]);

  const { data, isFetching } = useStorageUsage();
  const invalidate = useInvalidateStorageUsage();

  const totalBytes = data?.total_bytes ?? 0;
  const freeBytes = data?.free_bytes ?? 0;
  const breakdown = data ? storageBreakdown(data) : [];
  const usedBytes = totalBytes - freeBytes;

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col overflow-y-auto">
        <Header />
        <div className="px-16">
          <h1 className="mb-4 text-3xl font-bold">Storage</h1>

          <section className="space-y-3">
            <StorageBar totalBytes={totalBytes} breakdown={breakdown} />
            <p className="text-sm text-muted-foreground">
              {!data ? "Calculating..." : `${formatBytes(usedBytes)} used of ${formatBytes(totalBytes)}`}
            </p>
          </section>

          <section className="mt-6 space-y-2">
            <List>
              {breakdown.map(({ category, bytes }) => (
                <StorageLegendRow key={category} label={CATEGORY_META[category].label} colorClass={CATEGORY_META[category].colorClass} bytes={bytes} />
              ))}
              <StorageLegendRow label="Free" colorClass="bg-gray-700" bytes={freeBytes} />
            </List>
          </section>

          <section className="mt-6">
            <List>
              <ListRow label="Refresh" onSelect={() => invalidate()} value={isFetching ? "Refreshing..." : undefined} />
            </List>
            <p className="mt-2 text-sm text-muted-foreground">
              Games, BIOS, and Media are what's in ~/Relay -- System covers everything else on this drive (the OS, installed apps).
            </p>
          </section>
        </div>
      </div>
    </FocusContext.Provider>
  );
}

// Purely informational (no `to`/`onSelect`) -- same as ListRow's own "Version 0.1.1" precedent --
// so this doesn't register as a focusable/navigable row at all, just a value display.
function StorageLegendRow({ label, colorClass, bytes }: { label: string; colorClass: string; bytes: number }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded px-4 py-3 text-sm font-medium">
      <div className="flex items-center gap-3">
        <span className={cn("size-2.5 rounded-full", colorClass)} aria-hidden="true" />
        <span>{label}</span>
      </div>
      <span className="text-muted-foreground">{formatBytes(bytes)}</span>
    </div>
  );
}
