import { describe, expect, it } from "vitest";
import { toLibraryItem } from "./libraryItem";
import type {
  AssertPromptListItemHasNoBodies,
  PromptListItem,
  PromptTypeDefinition,
} from "./types";

const _listItemHasNoBodies: AssertPromptListItemHasNoBodies = true;
void _listItemHasNoBodies;

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

function makeListItem(partial: Partial<PromptListItem> = {}): PromptListItem {
  return {
    id: "p1",
    title: "Steelman",
    description: "A short card summary",
    promptType: "text",
    tags: ["a", "b", "c", "d"],
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
    const item = toLibraryItem(makeListItem(), [], t);
    expect(item.tags).toEqual(["a", "b", "c"]);
    expect(item.overflowTagCount).toBe(1);
  });

  it("redacts the description of a locked prompt and keeps metadata", () => {
    const item = toLibraryItem(
      makeListItem({ isLocked: true, isPrivate: true, title: "Hidden" }),
      [],
      t,
    );
    expect(item.description).toBe("Private content is locked");
    expect(item.description).not.toContain("SECRET");
    expect(item.title).toBe("Hidden");
    expect(item.tags).toHaveLength(3);
    expect(item.source.userPrompt).toBe("");
  });

  it("does not depend on userPrompt in the list payload", () => {
    const prompt = makeListItem();
    expect("userPrompt" in prompt).toBe(false);
    expect("systemPrompt" in prompt).toBe(false);
    expect("messages" in prompt).toBe(false);
    const item = toLibraryItem(prompt, [], t);
    expect(item.source.userPrompt).toBe("");
    expect(item.source.messages).toEqual([]);
    expect(JSON.stringify(item)).not.toContain("SECRET");
  });

  it("resolves a custom type name", () => {
    const item = toLibraryItem(
      makeListItem({ typeDefinitionId: "type-1", promptType: "image" }),
      [storyboard],
      t,
    );
    expect(item.typeLabel).toBe("Storyboard");
    expect(item.typeKind).toBe("image");
  });

  it("uses the empty-description notice when description is missing", () => {
    const item = toLibraryItem(makeListItem({ description: "  " }), [], t);
    expect(item.description).toBe("No description");
    expect(item.versionLabel).toBe("v2");
    expect(item.updatedLabel).toBe("2026-08-24");
  });
});
