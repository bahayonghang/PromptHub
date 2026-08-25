// @vitest-environment jsdom
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import i18n, { ensureBundle } from "../../../../runtime/i18n";
import type { Prompt, ReferenceList } from "../../types";
import { ReferencesTab } from "./ReferencesTab";

vi.mock("../../api", () => ({
  promptApi: {
    listReferences: vi.fn(),
  },
}));

import { promptApi } from "../../api";

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
    usageCount: 0,
    createdAt: "2026-07-15T00:00:00Z",
    updatedAt: "2026-07-15T00:00:00Z",
  };
}

afterEach(() => {
  cleanup();
});

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
});

describe("ReferencesTab", () => {
  it("shows resolved, unresolved, and incoming rows with text reasons", async () => {
    const listed: ReferenceList = {
      outgoing: [
        {
          targetPromptId: "a",
          targetTitle: "Alpha",
          tokenTitle: "Alpha",
          resolution: "resolved",
        },
        {
          targetPromptId: null,
          targetTitle: null,
          tokenTitle: "Missing",
          resolution: "missing",
        },
      ],
      incoming: [
        {
          sourcePromptId: "s",
          sourceTitle: "Source",
          tokenTitle: "Target",
          resolution: "resolved",
        },
      ],
    };
    vi.mocked(promptApi.listReferences).mockResolvedValue(listed);
    render(
      <ReferencesTab
        prompt={prompt("t", "Target")}
        prompts={[prompt("t", "Target"), prompt("a", "Alpha")]}
        onInsert={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByText("Alpha")).toBeTruthy());
    expect(screen.getAllByText("Missing").length).toBeGreaterThan(0);
    expect(screen.getByText("Source")).toBeTruthy();
  });
});
