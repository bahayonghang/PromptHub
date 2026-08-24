// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../runtime/i18n";
import type { Prompt } from "../types";
import { PromptList } from "./PromptList";

function makePrompt(overrides: Partial<Prompt> = {}): Prompt {
  return {
    id: "prompt-1",
    title: "Steelman",
    description: null,
    promptType: "text",
    systemPrompt: "Be terse.",
    userPrompt: "Argue both sides.",
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

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

afterEach(cleanup);

describe("PromptList preview", () => {
  it("shows the description instead of the prompt body", () => {
    render(
      <PromptList
        prompts={[
          makePrompt({
            description: "A short card summary",
            userPrompt: "This body must not appear in the list",
          }),
        ]}
        promptTypeDefinitions={[]}
        selectedPromptId={null}
        selectedPromptIds={[]}
        onSelect={vi.fn()}
        onToggleSelection={vi.fn()}
      />,
    );

    expect(screen.getByText("A short card summary")).toBeTruthy();
    expect(
      screen.queryByText("This body must not appear in the list"),
    ).toBeNull();
  });

  it("shows a placeholder when the description is empty", () => {
    render(
      <PromptList
        prompts={[makePrompt({ description: "  ", userPrompt: "Hidden body" })]}
        promptTypeDefinitions={[]}
        selectedPromptId={null}
        selectedPromptIds={[]}
        onSelect={vi.fn()}
        onToggleSelection={vi.fn()}
      />,
    );

    expect(screen.getByText("No description")).toBeTruthy();
    expect(screen.queryByText("Hidden body")).toBeNull();
  });
});

describe("PromptList copy control", () => {
  it("exposes one copy control per row and copies without selecting", async () => {
    const onSelect = vi.fn();
    const onToggleSelection = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    const first = makePrompt();
    const second = makePrompt({
      id: "prompt-2",
      title: "Curiosity",
      userPrompt: "Pick a field.",
      systemPrompt: "",
    });

    render(
      <PromptList
        prompts={[first, second]}
        promptTypeDefinitions={[]}
        selectedPromptId={null}
        selectedPromptIds={[]}
        onSelect={onSelect}
        onToggleSelection={onToggleSelection}
        writeText={writeText}
      />,
    );

    expect(screen.getByRole("button", { name: "Copy Steelman" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Copy Curiosity" })).toBeTruthy();

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy Steelman" }));
    });

    expect(writeText).toHaveBeenCalledWith(
      "[System]\nBe terse.\n\n[User]\nArgue both sides.",
    );
    expect(onSelect).not.toHaveBeenCalled();
    expect(onToggleSelection).not.toHaveBeenCalled();
  });

  it("selects from the title target without copying", async () => {
    const onSelect = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    render(
      <PromptList
        prompts={[makePrompt()]}
        promptTypeDefinitions={[]}
        selectedPromptId={null}
        selectedPromptIds={[]}
        onSelect={onSelect}
        onToggleSelection={vi.fn()}
        writeText={writeText}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Steelman" }));
    expect(onSelect).toHaveBeenCalledWith("prompt-1");
    expect(writeText).not.toHaveBeenCalled();
  });

  it("disables copy on a locked private prompt", () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    render(
      <PromptList
        prompts={[
          makePrompt({
            isPrivate: true,
            isLocked: true,
            userPrompt: "",
          }),
        ]}
        promptTypeDefinitions={[]}
        selectedPromptId={null}
        selectedPromptIds={[]}
        onSelect={vi.fn()}
        onToggleSelection={vi.fn()}
        writeText={writeText}
      />,
    );

    const copy = screen.getByRole("button", {
      name: "Unlock the prompt library to copy private content",
    });
    expect((copy as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(copy);
    expect(writeText).not.toHaveBeenCalled();
  });
});
