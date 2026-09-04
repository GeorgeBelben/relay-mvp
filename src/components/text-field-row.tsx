import { useEffect, useRef, useState } from "react";
import { pushBackHandler, useFocusable } from "@/lib/focus";
import { useActionHints } from "@/lib/hints";
import { cn } from "@/lib/cn";

type TextFieldRowProps = {
  label: string;
  value: string;
  onCommit: (value: string) => void;
  secret?: boolean;
  placeholder?: string;
};

// A List-style row that becomes a real, natively-focused <input> when selected -- everything
// else in this app is virtual-focus-only (see lib/focus), but free text entry needs the real
// thing (cursor movement, selection, paste) to work at all. The input's own onKeyDown stops
// Escape/Enter/arrows from bubbling to window, where the spatial-nav engine and our own
// back-handler listener both intercept those keys unconditionally -- without that, typing in this
// field would also drive navigation underneath it.
export function TextFieldRow({ label, value, onCommit, secret, placeholder }: TextFieldRowProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(value);
  const inputRef = useRef<HTMLInputElement>(null);

  const { ref, focused, focusSelf } = useFocusable({
    onEnterPress: () => {
      setDraft(value);
      setEditing(true);
    },
  });

  useEffect(() => {
    if (editing) inputRef.current?.focus();
  }, [editing]);

  // Shadows the parent screen's own hints while editing (same stack-based override every
  // modal/drawer already relies on) -- otherwise the hint bar keeps showing the screen's
  // generic "Confirm: Edit" even mid-edit, which doesn't tell you Enter is what actually saves.
  useActionHints(editing ? [{ action: "confirm", label: "Save" }, { action: "back", label: "Cancel" }] : null);

  // Reclaiming focus here (both on cancel and on save, below) matters for the same reason as
  // search-field.tsx: norigin's autoRestoreFocus only fires on unmount, and this row never
  // unmounts while editing (it just swaps its own returned JSX) -- without this, focus is left
  // stuck on whichever now-hidden on-screen-keyboard key was last pressed.
  //
  // Deliberately does NOT save the draft -- this is the "give up on this edit" path (Escape,
  // gamepad Back, or the input blurring for any other reason). onBlur calling the save function
  // unconditionally would mean the blur that naturally follows Escape/Back setting `editing`
  // false silently saves whatever was in the draft anyway, including an accidental edit the user
  // never meant to keep.
  const cancelEditing = () => {
    setEditing(false);
    focusSelf();
  };

  useEffect(() => {
    if (!editing) return;
    return pushBackHandler(cancelEditing);
  }, [editing, cancelEditing]);

  // The only path that actually persists the draft -- Enter (including a future on-screen
  // keyboard's Enter key, which would dispatch a real Enter keydown at whatever field is focused).
  const save = () => {
    setEditing(false);
    focusSelf();
    if (draft !== value) onCommit(draft);
  };

  if (editing) {
    return (
      <div className="flex items-center justify-between gap-3 rounded bg-gray-700 px-4 py-3 text-sm font-medium">
        <span className="shrink-0">{label}</span>
        <input
          ref={inputRef}
          data-osk
          value={draft}
          placeholder={placeholder}
          type={secret ? "password" : "text"}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            // "m" needs the same stopPropagation treatment as Enter/Escape/arrows -- it's the
            // global menu shortcut (lib/input/keyboard.ts), and that listener's preventDefault()
            // would otherwise eat the letter instead of typing it.
            if (e.key === "Enter" || e.key === "Escape" || e.key === "m" || e.key.startsWith("Arrow")) e.stopPropagation();
            if (e.key === "Enter") save();
            if (e.key === "Escape") cancelEditing();
          }}
          onBlur={cancelEditing}
          className="min-w-0 flex-1 bg-transparent text-right text-muted-foreground outline-none"
        />
      </div>
    );
  }

  return (
    <div
      ref={ref}
      className={cn(
        "flex items-center justify-between gap-3 rounded px-4 py-3 text-sm font-medium transition-bounce",
        focused ? "my-1 py-4 bg-gray-700" : "bg-gray-800",
      )}
    >
      <span>{label}</span>
      <span className="text-muted-foreground">{value ? (secret ? "••••••••" : value) : "Not set"}</span>
    </div>
  );
}
