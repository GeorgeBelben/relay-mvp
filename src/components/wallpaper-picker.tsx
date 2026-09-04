import { FocusContext, useFocusable } from "@/lib/focus";
import { resolveWallpaperUrl, useLibraryRoot } from "@/lib/media";
import { useSetWallpaper, useWallpaper, useWallpaperOptions } from "@/hooks/use-settings";
import { cn } from "@/lib/cn";

function Option({
  label,
  thumbnailUrl,
  selected,
  onSelect,
}: {
  label: string;
  thumbnailUrl: string | null;
  selected: boolean;
  onSelect: () => void;
}) {
  const { ref, focused } = useFocusable({ onEnterPress: onSelect });

  return (
    <div
      ref={ref}
      className={cn(
        "relative h-20 w-32 shrink-0 overflow-hidden opacity-60 border-2 border-transparent rounded-md bg-gray-800 transition-bounce",
        focused && "scale-110 opacity-100 bg-gray-700",
        selected ? "border-primary" : "border-transparent",
      )}
    >
      {thumbnailUrl ? (
        <img src={thumbnailUrl} alt={label} className="h-full w-full object-cover" />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-xs">None</div>
      )}
    </div>
  );
}

// Nav-compliant wallpaper picker -- self-contained, ready for Settings to render as-is. Options
// come from whatever image files are sitting in ~/Relay/wallpapers; drop new ones in over Samba/
// Finder and they show up here.
export function WallpaperPicker() {
  const wallpaper = useWallpaper();
  const setWallpaper = useSetWallpaper();
  const options = useWallpaperOptions();
  const libraryRoot = useLibraryRoot();
  const { ref, focusKey } = useFocusable({ focusKey: "WALLPAPER_PICKER" });

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex gap-4 pb-1">
        <Option
          label="None"
          thumbnailUrl={null}
          selected={wallpaper === ""}
          onSelect={() => setWallpaper.mutate("")}
        />
        {libraryRoot &&
          options.map((filename) => (
            <Option
              key={filename}
              label={filename}
              thumbnailUrl={resolveWallpaperUrl(libraryRoot, filename)}
              selected={wallpaper === filename}
              onSelect={() => setWallpaper.mutate(filename)}
            />
          ))}
      </div>
      {options.length === 0 && (
        <p className="mt-2 text-sm text-muted-foreground">No wallpapers found -- add image files to ~/Relay/wallpapers.</p>
      )}
    </FocusContext.Provider>
  );
}
