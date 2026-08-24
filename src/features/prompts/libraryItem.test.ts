import { describe, expect, it } from "vitest";
import { toLibraryItem } from "./libraryItem";
import type { Prompt, PromptTypeDefinition } from "./types";

function t(key: string): string {
  const labels: Record<string, string> = {
    "promptsView.untitled": "Untitled prompt",
    "promptsView.noDescription": "No description",
    "promptsView.privateLockedPreview": "Private content is locked",
    "promptsView.editor.typeText": "Text",
    "promptsView.editor.typeImage": "Image",
    "promptsView.editor.typeVideo": "Video",
  };
  return labels[key] ?? key;
}

function makePrompt(partial: Partial<Prompt> = {}): Prompt {
  return {
    id: "p1",
    title: "Steelman",
    description: "A short card summary",
    promptType: "text",
    userPrompt: "SECRET BODY",
    messages: [],
    variables: [],
    tags: ["a", "b", "c", "d"],
    images: [],
    videos: [],
    isFavorite: false,
    isPinned: false,
    isPrivate: false,
    isLocked: false,
    currentVersion: 2,
    usageCount: 9,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-08-24T00:00:00Z",
    ...partial,
  };
}

const storyboard: PromptTypeDefinition = {
  id: "type-1",
  name: "Storyboard",
  baseKind: "image",
  createdAt: "2026-01-01T00:00:00Z",
};

describe("toLibraryItem", () => {
  it("clamps tags to three and reports overflow", () => {
    const item = toLibraryItem(makePrompt(), [], t);
    expect(item.tags).toEqual(["a", "b", "c"]);
    expect(item.overflowTagCount).toBe(1);
  });

  it("redacts the description of a locked prompt and keeps metadata", () => {
    const item = toLibraryItem(
      makePrompt({ isLocked: true, isPrivate: true, title: "Hidden" }),
      [],
      t,
    );
    expect(item.description).toBe("Private content is locked");
    expect(item.description).not.toContain("SECRET");
    expect(item.title).toBe("Hidden");
    expect(item.tags).toHaveLength(3);
    expect(item.source.userPrompt).toBe("SECRET BODY");
  });

  it("resolves a custom type name", () => {
    const item = toLibraryItem(
      makePrompt({ typeDefinitionId: "type-1", promptType: "image" }),
      [storyboard],
      t,
    );
    expect(item.typeLabel).toBe("Storyboard");
    expect(item.typeKind).toBe("image");
  });

  it("uses the empty-description notice when description is missing", () => {
    const item = toLibraryItem(makePrompt({ description: "  " }), [], t);
    expect(item.description).toBe("No description");
    expect(item.versionLabel).toBe("v2");
    expect(item.updatedLabel).toBe("2026-08-24");
  });
});
