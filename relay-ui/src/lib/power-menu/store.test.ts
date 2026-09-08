import { describe, expect, it } from "vitest";
import { usePowerMenuStore } from "./store";

describe("usePowerMenuStore", () => {
  it("starts closed", () => {
    expect(usePowerMenuStore.getState().open).toBe(false);
  });

  it("openMenu/closeMenu set open directly", () => {
    usePowerMenuStore.getState().openMenu();
    expect(usePowerMenuStore.getState().open).toBe(true);

    usePowerMenuStore.getState().closeMenu();
    expect(usePowerMenuStore.getState().open).toBe(false);
  });

  it("toggleMenu flips open/closed each call", () => {
    usePowerMenuStore.setState({ open: false });

    usePowerMenuStore.getState().toggleMenu();
    expect(usePowerMenuStore.getState().open).toBe(true);

    usePowerMenuStore.getState().toggleMenu();
    expect(usePowerMenuStore.getState().open).toBe(false);
  });
});
