import { describe, expect, it } from "vitest";
import en from "../../locales/en.json";

/** Resolves a dotted i18n key against a bundle, returning undefined if absent. */
function lookup(bundle: unknown, key: string): unknown {
  return key.split(".").reduce<unknown>((node, part) => {
    if (node !== null && typeof node === "object") {
      return (node as Record<string, unknown>)[part];
    }
    return undefined;
  }, bundle);
}

/** Every i18n key the prompt-editing view renders (Req 21.3). */
const KEYS = [
  "promptsView.searchPlaceholder",
  "promptsView.newPrompt",
  "promptsView.filters",
  "promptsView.favoritesOnly",
  "promptsView.sortBy",
  "promptsView.sortUpdated",
  "promptsView.sortCreated",
  "promptsView.sortTitle",
  "promptsView.sortUsage",
  "promptsView.sortAsc",
  "promptsView.sortDesc",
  "promptsView.clearFilters",
  "promptsView.filterByTags",
  "promptsView.noTags",
  "promptsView.allFolders",
  "promptsView.folders",
  "promptsView.newFolder",
  "promptsView.renameFolder",
  "promptsView.deleteFolder",
  "promptsView.folderNamePlaceholder",
  "promptsView.deleteFolderConfirm",
  "promptsView.emptyFolders",
  "promptsView.noPrompts",
  "promptsView.noPromptsHint",
  "promptsView.loading",
  "promptsView.selectPromptTitle",
  "promptsView.selectPromptHint",
  "promptsView.untitled",
  "promptsView.favorite",
  "promptsView.unfavorite",
  "promptsView.deletePrompt",
  "promptsView.deletePromptConfirm",
  "promptsView.editor.title",
  "promptsView.editor.titlePlaceholder",
  "promptsView.editor.titleRequired",
  "promptsView.editor.description",
  "promptsView.editor.type",
  "promptsView.editor.typeText",
  "promptsView.editor.typeImage",
  "promptsView.editor.typeVideo",
  "promptsView.editor.folder",
  "promptsView.editor.noFolder",
  "promptsView.editor.systemPrompt",
  "promptsView.editor.userPrompt",
  "promptsView.editor.userPromptRequired",
  "promptsView.editor.variables",
  "promptsView.editor.noVariables",
  "promptsView.editor.variableRequired",
  "promptsView.editor.tags",
  "promptsView.editor.addTag",
  "promptsView.editor.images",
  "promptsView.editor.videos",
  "promptsView.editor.source",
  "promptsView.editor.notes",
  "promptsView.editor.preview",
  "promptsView.editor.save",
  "promptsView.editor.cancel",
  "promptsView.editor.create",
  "promptsView.history.title",
  "promptsView.history.empty",
  "promptsView.history.emptyHint",
  "promptsView.history.saveVersion",
  "promptsView.history.notePlaceholder",
  "promptsView.history.noteTooLong",
  "promptsView.history.versionLabel",
  "promptsView.history.restore",
  "promptsView.history.restoreConfirm",
  "promptsView.history.delete",
];

describe("prompts view i18n keys (Req 21.3)", () => {
  it("resolves every rendered key to a non-empty string in the English bundle", () => {
    for (const key of KEYS) {
      const value = lookup(en, key);
      expect(typeof value, `missing key: ${key}`).toBe("string");
      expect((value as string).length, `empty key: ${key}`).toBeGreaterThan(0);
    }
  });
});
