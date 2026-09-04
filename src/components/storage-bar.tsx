import { cn } from "@/lib/cn";
import type { StorageUsage } from "@/hooks/use-storage";

type StorageCategory = "games" | "bios" | "media" | "saves" | "system";

// Colors are the only thing a caller can't get from CATEGORY_META's own label on its own --
// exported so storage.tsx's legend rows can reuse the exact same dot color as the bar segment
// they describe, rather than a second hardcoded copy of this mapping.
export const CATEGORY_META: Record<StorageCategory, { label: string; colorClass: string }> = {
  games: { label: "Games", colorClass: "bg-sky-400" },
  bios: { label: "BIOS", colorClass: "bg-violet-400" },
  media: { label: "Media", colorClass: "bg-rose-400" },
  saves: { label: "Saves", colorClass: "bg-emerald-400" },
  system: { label: "System", colorClass: "bg-gray-500" },
};

// StorageUsage carries these as flat per-category fields (games_bytes, bios_bytes, ...), not an
// array the way the Electron MVP's breakdown was -- the category set is fixed, so there's nothing
// to gain from a runtime list here. This is the one place that turns those fields back into
// {category, bytes} pairs, for storage.tsx's legend rows and this bar's own segments to share.
export function storageBreakdown(usage: StorageUsage): { category: StorageCategory; bytes: number }[] {
  return [
    { category: "games", bytes: usage.games_bytes },
    { category: "bios", bytes: usage.bios_bytes },
    { category: "media", bytes: usage.media_bytes },
    { category: "saves", bytes: usage.saves_bytes },
    { category: "system", bytes: usage.system_bytes },
  ];
}

type StorageBarProps = {
  totalBytes: number;
  breakdown: { category: StorageCategory; bytes: number }[];
  className?: string;
};

// One rounded track, one flex-basis-by-percentage div per category plus a trailing "free space"
// segment in the track's own background color (so it visually reads as "unfilled", not as its own
// distinct color needing a legend entry). totalBytes: 0 (storage.tsx's loading state) renders an
// empty track rather than dividing by zero.
export function StorageBar({ totalBytes, breakdown, className }: StorageBarProps) {
  const usedBytes = breakdown.reduce((sum, { bytes }) => sum + bytes, 0);
  const freeBytes = Math.max(0, totalBytes - usedBytes);

  return (
    <div className={cn("flex h-4 w-full overflow-hidden rounded-full bg-gray-800", className)}>
      {totalBytes > 0 &&
        breakdown
          .filter(({ bytes }) => bytes > 0)
          .map(({ category, bytes }) => (
            <div
              key={category}
              className={CATEGORY_META[category].colorClass}
              style={{ width: `${(bytes / totalBytes) * 100}%` }}
              aria-label={`${CATEGORY_META[category].label}: ${Math.round((bytes / totalBytes) * 100)}%`}
            />
          ))}
      {totalBytes > 0 && freeBytes > 0 && (
        <div className="bg-gray-800" style={{ width: `${(freeBytes / totalBytes) * 100}%` }} aria-hidden="true" />
      )}
    </div>
  );
}
