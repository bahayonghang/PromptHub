// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../runtime/i18n";
import { useGlobalShortcuts } from "./useGlobalShortcuts";
import { usePaletteStore } from "../features/prompts/paletteStore";
import { usePromptStore } from "../features/prompts/promptStore";
import { CommandPalette } from "../features/prompts/components/CommandPalette";

vi.mock("../features/prompts/api", async () => {
  const actual = await vi.importActual<typeof import("../features/prompts/api")>(
    "../features/prompts/api",
  );
  return {
    ...actual,
    promptApi: {
      ...actual.promptApi,
      searchPrompts: vi.fn(async () => ({
        items: [],
        total: 0,
        limit: 5,
        offset: 0,
        hasMore: false,
      })),
    },
  };
});

function Host() {
  useGlobalShortcuts();
  return null;
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

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

  it("still toggles Cmd/Ctrl+K while the palette query input is focused", () => {
    usePaletteStore.setState({ open: true });
    render(
      <>
        <Host />
        <CommandPalette />
      </>,
    );
    const query = screen.getByRole("combobox", {
      name: "Search prompts and actions",
    });
    query.focus();
    expect(document.activeElement).toBe(query);
    fireKey("k", { ctrlKey: true, metaKey: true, target: query });
    expect(usePaletteStore.getState().open).toBe(false);
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

  it("fires copy from a textarea on Cmd/Ctrl+Enter", () => {
    const copy = vi.fn(async () => undefined);
    usePromptStore.getState().registerDetailActions({
      save: vi.fn(async () => ({ ok: true as const })),
      copy,
    });
    render(<Host />);
    const input = document.createElement("textarea");
    document.body.appendChild(input);
    input.focus();
    fireKey("Enter", { ctrlKey: true, metaKey: true, target: input });
    expect(copy).toHaveBeenCalledOnce();
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
