// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import { toLibraryItem } from "../libraryItem";
import type { Prompt } from "../types";
import { usePromptStore } from "../promptStore";
import { PromptGrid } from "./PromptGrid";

function makePrompt(overrides: Partial<Prompt> = {}): Prompt {
  return {
    id: "prompt-1",
    title: "Steelman",
    description: "A short card summary",
    promptType: "text",
    systemPrompt: "Be terse.",
    userPrompt: "SECRET BODY",
    messages: [],
    variables: [],
    tags: ["thinking"],
    folderId: null,
    images: [],
    videos: [],
    isFavorite: false,
    isPinned: false,
    isPrivate: false,
    isLocked: false,
    currentVersion: 1,
    usageCount: 0,
    source: null,
    notes: null,
    lastAiResponse: null,
    createdAt: "2026-08-24T00:00:00Z",
    updatedAt: "2026-08-24T00:00:00Z",
    ...overrides,
  };
}

function itemsFrom(prompts: Prompt[]) {
  return prompts.map((prompt) => toLibraryItem(prompt, [], (key) => i18n.t(key)));
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

afterEach(cleanup);

describe("PromptGrid", () => {
  beforeEach(() => {
    usePromptStore.setState({ incrementUsage: async () => null });
  });

  it("does not open the prompt from checkbox, favorite, or copy", async () => {
    const onSelect = vi.fn();
    const onToggleSelection = vi.fn();
    const onToggleFavorite = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    const incrementUsage = vi.fn(async () => null);
    usePromptStore.setState({ incrementUsage });
    render(
      <PromptGrid
        items={itemsFrom([makePrompt()])}
        selectedPromptId={null}
        selectedPromptIds={[]}
        batchMode
        onSelect={onSelect}
        onToggleSelection={onToggleSelection}
        onToggleFavorite={onToggleFavorite}
        writeText={writeText}
        copyPrompt={async () => ({
          systemPrompt: "Be terse.",
          userPrompt: "SECRET BODY",
          messages: [],
          unexpanded: [],
        })}
      />,
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "Select Steelman" }));
    fireEvent.click(screen.getByRole("button", { name: "Favorite" }));
    const copy = screen.getByRole("button", { name: "Copy Steelman" });
    expect(copy.nextElementSibling?.textContent).toContain("Steelman");
    expect(
      copy.closest("article")?.querySelector(".mt-auto")?.querySelector("button"),
    ).toBeNull();
    await act(async () => {
      fireEvent.click(copy);
    });
    expect(onSelect).not.toHaveBeenCalled();
    expect(onToggleSelection).toHaveBeenCalledWith("prompt-1");
    expect(onToggleFavorite).toHaveBeenCalledWith("prompt-1", true);
    expect(writeText).toHaveBeenCalled();
    expect(incrementUsage).toHaveBeenCalledWith("prompt-1");
  });

  it("hides body-derived preview text on a locked card", () => {
    render(
      <PromptGrid
        items={itemsFrom([
          makePrompt({ isLocked: true, isPrivate: true, description: "should not show" }),
        ])}
        selectedPromptId={null}
        selectedPromptIds={[]}
        onSelect={vi.fn()}
        onToggleSelection={vi.fn()}
        onToggleFavorite={vi.fn()}
      />,
    );
    expect(screen.getByText("Private content is locked")).toBeTruthy();
    expect(screen.queryByText("should not show")).toBeNull();
    expect(screen.queryByText("SECRET BODY")).toBeNull();
  });
});
