// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { CommandPalette } from "./CommandPalette";
import { usePaletteStore } from "../paletteStore";
import { usePromptStore } from "../promptStore";
import { promptApi } from "../api";
import type { Prompt } from "../types";

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    promptApi: {
      ...actual.promptApi,
      searchPrompts: vi.fn(),
    },
  };
});

function prompt(id: string, title: string): Prompt {
  return {
    id,
    title,
    promptType: "text",
    userPrompt: "body",
    messages: [],
    variables: [],
    tags: [],
    images: [],
    videos: [],
    isFavorite: false,
    isPinned: false,
    isPrivate: false,
    isLocked: false,
    currentVersion: 1,
    usageCount: 3,
    createdAt: "2026-07-15T00:00:00Z",
    updatedAt: "2026-07-15T00:00:00Z",
  };
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
  usePaletteStore.setState({ open: true });
  vi.mocked(promptApi.searchPrompts).mockResolvedValue({
    items: [prompt("off-page", "Off page prompt")],
    total: 1,
    limit: 5,
    offset: 0,
    hasMore: false,
  });
});

afterEach(() => {
  cleanup();
  usePaletteStore.setState({ open: false });
});

describe("CommandPalette", () => {
  it("finds a prompt that is not on the current library page", async () => {
    render(<CommandPalette />);
    await waitFor(() =>
      expect(screen.getByText("Off page prompt")).toBeTruthy(),
    );
    expect(usePromptStore.getState().prompts).toEqual([]);
  });

  it("calls requestSelectPrompt for a prompt row", async () => {
    const requestSelectPrompt = vi.fn(async () => true);
    usePromptStore.setState({ requestSelectPrompt });
    render(<CommandPalette />);
    await waitFor(() =>
      expect(screen.getByText("Off page prompt")).toBeTruthy(),
    );
    fireEvent.click(screen.getByText("Off page prompt"));
    await waitFor(() =>
      expect(requestSelectPrompt).toHaveBeenCalledWith("off-page"),
    );
    expect(usePaletteStore.getState().open).toBe(false);
  });

  it("keeps the palette open when navigation is cancelled", async () => {
    usePromptStore.setState({
      requestSelectPrompt: vi.fn(async () => false),
    });
    render(<CommandPalette />);
    await waitFor(() =>
      expect(screen.getByText("Off page prompt")).toBeTruthy(),
    );
    fireEvent.click(screen.getByText("Off page prompt"));
    await waitFor(() =>
      expect(usePromptStore.getState().requestSelectPrompt).toHaveBeenCalled(),
    );
    expect(usePaletteStore.getState().open).toBe(true);
    expect(
      (screen.getByRole("combobox") as HTMLInputElement).value,
    ).toBe("");
  });

  it("toggles view mode through the store action", async () => {
    const setViewMode = vi.fn();
    usePromptStore.setState({ viewMode: "list", setViewMode });
    render(<CommandPalette />);
    fireEvent.click(screen.getByText("Switch list and grid"));
    expect(setViewMode).toHaveBeenCalledWith("grid");
  });
});
