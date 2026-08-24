// @vitest-environment jsdom
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useGlobalShortcuts } from "./useGlobalShortcuts";
import { usePaletteStore } from "../features/prompts/paletteStore";
import { usePromptStore } from "../features/prompts/promptStore";

function Host() {
  useGlobalShortcuts();
  return null;
}

afterEach(() => {
  cleanup();
  usePaletteStore.setState({ open: false });
  usePromptStore.getState().registerDetailActions(null);
});

describe("useGlobalShortcuts", () => {
  it("toggles the palette on modifier+K and ignores it while typing", () => {
    render(<Host />);
    fireKey("k", { ctrlKey: true, metaKey: true });
    expect(usePaletteStore.getState().open).toBe(true);

    const input = document.createElement("textarea");
    document.body.appendChild(input);
    input.focus();
    usePaletteStore.setState({ open: false });
    fireKey("k", { ctrlKey: true, metaKey: true, target: input });
    expect(usePaletteStore.getState().open).toBe(false);
    input.remove();
  });

  it("fires save from a textarea when detail actions are registered", () => {
    const save = vi.fn(async () => ({ ok: true as const }));
    usePromptStore.getState().registerDetailActions({
      save,
      copy: vi.fn(async () => undefined),
    });
    render(<Host />);
    const input = document.createElement("textarea");
    document.body.appendChild(input);
    input.focus();
    fireKey("s", { ctrlKey: true, metaKey: true, target: input });
    expect(save).toHaveBeenCalledOnce();
    input.remove();
  });

  it("removes the listener on unmount", () => {
    const { unmount } = render(<Host />);
    unmount();
    usePaletteStore.setState({ open: false });
    fireKey("k", { ctrlKey: true, metaKey: true });
    expect(usePaletteStore.getState().open).toBe(false);
  });
});

function fireKey(
  key: string,
  init: KeyboardEventInit & { target?: EventTarget },
): void {
  const event = new KeyboardEvent("keydown", {
    key,
    bubbles: true,
    cancelable: true,
    ...init,
  });
  const target = init.target ?? document;
  target.dispatchEvent(event);
}
