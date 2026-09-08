import { useEffect } from "react";
import type { ErrorComponentProps } from "@tanstack/react-router";
import { Logo } from "@/components/logo";
import { List, ListRow } from "@/components/list";
import { FocusContext, useFocusable } from "@/lib/focus";

// Wired as the router's `defaultErrorComponent` (see src/main.tsx) -- the fallback every route
// gets for free unless it sets its own `errorComponent`, since TanStack Router wraps each matched
// route (root included) in its own catch boundary and bubbles up to the nearest one that defines
// this. Deliberately `flex-1` rather than `h-svh`: it renders in place of whatever the route was
// going to put inside __root.tsx's <Outlet/>, alongside HintBar/QuickMenu/PowerMenu, which is the
// whole point -- a crash in one page's content shouldn't take the rest of the kiosk chrome down
// with it, unlike the top-level ErrorBoundary in main.tsx (which only exists to catch what
// happens above/outside the router entirely). Logging the caught error is the router's job too --
// see `defaultOnCatch` alongside this in main.tsx -- this component only renders the recovery UI.
export function RouteErrorFallback({ reset }: ErrorComponentProps) {
  const { ref, focusKey, focusSelf } = useFocusable({ trackChildren: true });
  useEffect(() => {
    focusSelf();
    // Mount-only, matching ConfirmActionView's identical effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <FocusContext.Provider value={focusKey}>
      <div ref={ref} className="flex flex-1 flex-col items-center justify-center gap-6 text-white">
        <Logo className="w-20 opacity-60" />
        <p className="font-lexend-deca text-sm text-white/60">This page hit an error.</p>
        <List>
          <ListRow label="Try Again" onSelect={reset} />
          <ListRow label="Go Home" to="/" />
        </List>
      </div>
    </FocusContext.Provider>
  );
}
