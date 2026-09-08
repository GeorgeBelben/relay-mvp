import { useFocusable } from "@/lib/focus";
import { playSound } from "@/lib/sound";
import { useSetSoundVolume, useSoundVolume } from "@/hooks/use-settings";
import { cn } from "@/lib/cn";

const STEP = 10;

// A discrete stepped bar, not a draggable track -- there's no pointer on this device, only
// gamepad/keyboard, so Left/Right adjusting a fixed step is the actual interaction, same
// "onArrowPress consumes the press itself" idiom norigin expects for a custom horizontal control
// (return false to stop it also being treated as a request to move focus left/right; true for
// up/down so the row above/below is still reachable normally).
export function VolumeSlider() {
  const volume = useSoundVolume();
  const setVolume = useSetSoundVolume();

  const { ref, focused } = useFocusable({
    focusKey: "SOUND_VOLUME_SLIDER",
    onArrowPress: (direction) => {
      if (direction !== "left" && direction !== "right") return true;

      const next = Math.max(0, Math.min(100, volume + (direction === "right" ? STEP : -STEP)));
      if (next !== volume) {
        setVolume.mutate(next);
        playSound("nav"); // audible feedback for the level you just landed on
      }
      return false;
    },
  });

  return (
    <div
      ref={ref}
      className={cn("rounded bg-gray-800 px-4 py-3 transition-bounce", focused && "my-1 py-5")}
      aria-label={`Sound volume ${volume}%`}
    >
      <div className="flex items-center justify-between pb-2 text-sm font-medium">
        <span>Sound Volume</span>
        <span className="text-muted-foreground">{volume}%</span>
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-gray-600">
        <div className="h-full bg-white transition-[width]" style={{ width: `${volume}%` }} />
      </div>
    </div>
  );
}
