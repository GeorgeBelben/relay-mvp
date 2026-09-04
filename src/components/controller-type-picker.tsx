import { FocusContext, useFocusable } from "@/lib/focus";
import { useControllerType, useSetControllerType, type ControllerType } from "@/hooks/use-settings";
import { getIconUrl } from "@/lib/icons/iconSet";
import { cn } from "@/lib/cn";

const OPTIONS: { value: ControllerType; label: string }[] = [
  { value: "xbox", label: "Xbox" },
  { value: "playstation", label: "PlayStation" },
  { value: "switch", label: "Switch" },
  { value: "generic", label: "Generic" },
];

// The four icon sets differ subtly (which symbol sits on which face button) and there's no other
// way to see that before committing to one -- the actual result only ever shows up later, in the
// hint bar, on whatever screen you happened to navigate to next. Same card shape as
// wallpaper-picker.tsx's Option (border highlight + focus ring), swapping its image thumbnail for
// a live render of this brand's own confirm/back glyphs via the same getIconUrl lookup the hint
// bar itself uses, so the preview can never drift out of sync with what players actually see.
function Option({
  value,
  label,
  selected,
  onSelect,
}: {
  value: ControllerType;
  label: string;
  selected: boolean;
  onSelect: (value: ControllerType) => void;
}) {
  const { ref, focused } = useFocusable({ onEnterPress: () => onSelect(value) });

  return (
    <div
      ref={ref}
      className={cn(
        "flex w-28 shrink-0 flex-col items-center gap-2 rounded-md border-2 bg-gray-800 py-3 transition-bounce",
        focused && "bg-gray-700 scale-110",
        selected ? "border-primary" : "border-transparent",
      )}
    >
      <div className="flex items-center gap-3">
        <img src={getIconUrl("gamepad", value, "confirm")} alt="" className="h-7 w-7" />
        <img src={getIconUrl("gamepad", value, "back")} alt="" className="h-7 w-7" />
      </div>
      <span className={cn("text-sm font-medium", selected ? "text-white" : "text-muted-foreground")}>{label}</span>
    </div>
  );
}

// Nav-compliant controller-brand picker -- self-contained, ready for Settings to render as-is.
export function ControllerTypePicker() {
  const controllerType = useControllerType();
  const setControllerType = useSetControllerType();
  const { ref, focusKey } = useFocusable({ focusKey: "CONTROLLER_TYPE_PICKER" });

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex gap-4">
        {OPTIONS.map((option) => (
          <Option
            key={option.value}
            value={option.value}
            label={option.label}
            selected={controllerType === option.value}
            onSelect={(value) => setControllerType.mutate(value)}
          />
        ))}
      </div>
    </FocusContext.Provider>
  );
}
