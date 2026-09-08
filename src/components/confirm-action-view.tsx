import { useEffect } from "react";
import { List, ListRow } from "./list";
import { FocusContext, useBackHandler, useFocusable } from "@/lib/focus";

// Own sub-view rather than an inline conditional inside the caller's menu view -- same reasoning
// as AchievementsView: a distinct focus subtree needs its own FocusContext + focusSelf() so the
// gamepad lands on it rather than wherever focus was left in the previous view. Shared by
// QuickMenu's Quit/Restart (REL-148/REL-150) and PowerMenu's Quit/Restart/Shut Down/Reboot -- same
// confirm shape everywhere, only the copy and the mutation that fires differ, which the caller
// supplies rather than this needing to know about any of them.
export function ConfirmActionView({
  title,
  description,
  confirmLabel,
  onConfirm,
  onBack,
}: {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void;
  onBack: () => void;
}) {
  useBackHandler(onBack);

  const { ref, focusKey, focusSelf } = useFocusable({
    trackChildren: true,
    saveLastFocusedChild: true,
  });
  useEffect(() => {
    focusSelf();
    // Mount-only -- nothing async to wait for here, unlike AchievementsView's own focusSelf
    // effect keyed on its data arriving.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref}>
        <h2 className="px-4 pb-2 text-base font-semibold">{title}</h2>
        <p className="px-4 pb-3 text-sm text-muted-foreground">{description}</p>
        <List>
          <ListRow label={confirmLabel} onSelect={onConfirm} />
          <ListRow label="Back" onSelect={onBack} />
        </List>
      </div>
    </FocusContext.Provider>
  );
}
