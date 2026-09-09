import { renderHook, waitFor } from "@testing-library/react";
import { act } from "react";
import { describe, expect, it, vi } from "vitest";
import { useConnectedControllers } from "./hooks";
import type { BackendGamepadEvent } from "./backendEvent";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

type Listener = (event: { payload: BackendGamepadEvent }) => void;
let capturedListener: Listener | undefined;
const unlistenMock = vi.fn();
const listenMock = vi.fn((_event: string, handler: Listener) => {
  capturedListener = handler;
  return Promise.resolve(unlistenMock);
});
vi.mock("@tauri-apps/api/event", () => ({
  listen: (event: string, handler: Listener) => listenMock(event, handler),
}));

function emit(event: BackendGamepadEvent) {
  act(() => capturedListener!({ payload: event }));
}

describe("useConnectedControllers", () => {
  it("reads whatever's already connected on mount, without waiting for a connect event", async () => {
    invokeMock.mockResolvedValue([0, 2]);

    const { result } = renderHook(() => useConnectedControllers());

    await waitFor(() => expect(result.current).toEqual([0, 2]));
    expect(invokeMock).toHaveBeenCalledWith("list_connected_gamepads");
  });

  it("picks up a hot-plugged controller via the connected event", async () => {
    invokeMock.mockResolvedValue([]);
    const { result } = renderHook(() => useConnectedControllers());
    await waitFor(() => expect(result.current).toEqual([]));

    emit({ type: "connected", id: 0, name: "Test Pad" });

    expect(result.current).toEqual([0]);
  });

  it("drops a controller via the disconnected event", async () => {
    invokeMock.mockResolvedValue([0, 1]);
    const { result } = renderHook(() => useConnectedControllers());
    await waitFor(() => expect(result.current).toEqual([0, 1]));

    emit({ type: "disconnected", id: 1 });

    expect(result.current).toEqual([0]);
  });

  it("sorts ids low to high regardless of connect order", async () => {
    invokeMock.mockResolvedValue([]);
    const { result } = renderHook(() => useConnectedControllers());
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());

    emit({ type: "connected", id: 3, name: "Pad 3" });
    emit({ type: "connected", id: 1, name: "Pad 1" });

    expect(result.current).toEqual([1, 3]);
  });

  it("ignores button/axis events -- presence only changes on connect/disconnect", async () => {
    invokeMock.mockResolvedValue([0]);
    const { result } = renderHook(() => useConnectedControllers());
    await waitFor(() => expect(result.current).toEqual([0]));

    emit({ type: "button-pressed", id: 0, button: "south" });

    expect(result.current).toEqual([0]);
  });

  it("unlistens on unmount", async () => {
    invokeMock.mockResolvedValue([]);
    const { unmount } = renderHook(() => useConnectedControllers());
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());

    unmount();

    expect(unlistenMock).toHaveBeenCalled();
  });
});
