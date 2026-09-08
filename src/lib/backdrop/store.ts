import { create } from "zustand";
import type { LibraryGame } from "@/hooks/use-library";

// The game whose backdrop should currently be showing behind the app -- pure client UI state
// (never routed through Tauri, see CLAUDE.md), bridging Home's carousel focus (routes/index.tsx)
// to BackdropLayer, which is mounted at the root layout (__root.tsx) and therefore outside Home's
// own component tree. `null` means "nothing to show" -- BackdropLayer treats that as "keep
// whatever's already showing" rather than clearing it, so navigating focus off the carousel
// (or away from Home entirely) doesn't flash the background back to blank.
type BackdropState = {
  game: LibraryGame | null;
  setBackdropGame: (game: LibraryGame | null) => void;
};

export const useBackdropStore = create<BackdropState>((set) => ({
  game: null,
  setBackdropGame: (game) => set({ game }),
}));
