import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import type { UseFocusableConfig } from "@noriginmedia/norigin-spatial-navigation-react";
import { TextFieldRow } from "./text-field-row";
import { initFocusEngine } from "@/lib/focus";

initFocusEngine();

// Mirrors bridge.test.ts's own mocking boundary (it mocks SpatialNavigation itself) -- this app's
// real gamepad path calls straight into SpatialNavigation.onEnterPress, which looks up whichever
// onEnterPress callback this hook most recently registered for the currently-focused component and
// invokes that directly, with no regard for which JSX branch the component happens to be rendering.
// Capturing that callback here and calling it ourselves is what actually exercises the regression
// this file guards against -- a real Enter keydown on the <input> instead would only ever exercise
// the (separately correct) physical-keyboard path, which bypasses this callback entirely via
// stopPropagation.
let latestOnEnterPress: (() => void) | undefined;
vi.mock("@noriginmedia/norigin-spatial-navigation-react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@noriginmedia/norigin-spatial-navigation-react")>();
  return {
    ...actual,
    useFocusable: (config?: UseFocusableConfig) => {
      latestOnEnterPress = config?.onEnterPress as (() => void) | undefined;
      return actual.useFocusable(config);
    },
  };
});

// Calling the captured callback directly, like SpatialNavigation itself does, happens outside
// React's own event system -- act() is what flushes the resulting state update before the next
// assertion/query runs.
function pressConfirm() {
  act(() => latestOnEnterPress?.());
}

describe("TextFieldRow", () => {
  it("opens the field on the first gamepad confirm, and saves the typed draft on the second", () => {
    const onCommit = vi.fn();
    render(<TextFieldRow label="Web API Key" value="" onCommit={onCommit} secret placeholder="Paste your key" />);

    pressConfirm();
    const input = screen.getByPlaceholderText("Paste your key");
    fireEvent.change(input, { target: { value: "pasted-key-123" } });

    pressConfirm();

    expect(onCommit).toHaveBeenCalledExactlyOnceWith("pasted-key-123");
  });

  it("does not reset the draft back to the committed value on the second confirm", () => {
    const onCommit = vi.fn();
    render(<TextFieldRow label="Web API Key" value="" onCommit={onCommit} secret placeholder="Paste your key" />);

    pressConfirm();
    const input = screen.getByPlaceholderText("Paste your key");
    fireEvent.change(input, { target: { value: "pasted-key-123" } });

    pressConfirm();

    expect(onCommit).not.toHaveBeenCalledWith("");
  });

  it("commits nothing when the draft never changed from the committed value", () => {
    const onCommit = vi.fn();
    render(<TextFieldRow label="Name" value="George" onCommit={onCommit} />);

    pressConfirm(); // open
    pressConfirm(); // "save" with an unedited draft

    expect(onCommit).not.toHaveBeenCalled();
  });
});
