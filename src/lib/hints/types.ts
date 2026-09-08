import type { NavAction } from "@/lib/input";

// Excludes "power" -- it's a global, always-available input (see lib/power-menu), not a
// screen-contextual one, so nothing should ever advertise it as a per-screen hint.
export type Hint = { action: Exclude<NavAction, "power">; label: string };
