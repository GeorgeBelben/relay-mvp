import { create } from "zustand";

type PowerMenuState = {
  open: boolean;
  openMenu: () => void;
  closeMenu: () => void;
  toggleMenu: () => void;
};

export const usePowerMenuStore = create<PowerMenuState>((set) => ({
  open: false,
  openMenu: () => set({ open: true }),
  closeMenu: () => set({ open: false }),
  toggleMenu: () => set((state) => ({ open: !state.open })),
}));
