import { describe, expect, it } from "vitest";
import en from "../../locales/en.json";
import de from "../../locales/de.json";
import es from "../../locales/es.json";
import fr from "../../locales/fr.json";
import ja from "../../locales/ja.json";
import zh from "../../locales/zh.json";
import zhTW from "../../locales/zh-TW.json";

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
  "promptsView.pagination.summary",
  "promptsView.pagination.previous",
  "promptsView.pagination.next",
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
  "promptsView.pin",
  "promptsView.unpin",
  "promptsView.duplicatePrompt",
  "promptsView.deletePrompt",
  "promptsView.deletePromptConfirm",
  "promptsView.privatePrompt",
  "promptsView.privateLockedPreview",
  "promptsView.privateLockedTitle",
  "promptsView.privateLockedHint",
  "promptsView.bundle.export",
  "promptsView.bundle.import",
  "promptsView.bundle.exported",
  "promptsView.bundle.path",
  "promptsView.bundle.pathPlaceholder",
  "promptsView.bundle.conflictPolicy",
  "promptsView.bundle.skip",
  "promptsView.bundle.duplicate",
  "promptsView.bundle.replace",
  "promptsView.bundle.preview",
  "promptsView.bundle.previewSummary",
  "promptsView.bundle.privateKeyWarning",
  "promptsView.bundle.confirmImport",
  "promptsView.bundle.imported",
  "promptsView.batch.selectPrompt",
  "promptsView.batch.selected",
  "promptsView.batch.selectPage",
  "promptsView.batch.clear",
  "promptsView.batch.folder",
  "promptsView.batch.move",
  "promptsView.batch.tagPlaceholder",
  "promptsView.batch.addTag",
  "promptsView.batch.delete",
  "promptsView.batch.deleteConfirm",
  "promptsView.tags.manage",
  "promptsView.tags.rename",
  "promptsView.tags.renameValue",
  "promptsView.tags.delete",
  "promptsView.tags.deleteConfirm",
  "promptsView.editor.privatePrompt",
  "promptsView.editor.privatePromptHint",
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
  "promptsView.history.showDiff",
  "promptsView.history.noDiff",
  "promptsView.history.sources.create",
  "promptsView.history.sources.save",
  "promptsView.history.sources.manual",
  "promptsView.history.sources.rollback",
  "promptsView.history.sources.import",
  "promptsView.history.sources.replace",
  "promptsView.history.fields.title",
  "promptsView.history.fields.description",
  "promptsView.history.fields.promptType",
  "promptsView.history.fields.systemPrompt",
  "promptsView.history.fields.userPrompt",
  "promptsView.history.fields.variables",
  "promptsView.history.fields.tags",
  "promptsView.history.fields.folder",
  "promptsView.history.fields.images",
  "promptsView.history.fields.videos",
  "promptsView.history.fields.favorite",
  "promptsView.history.fields.pinned",
  "promptsView.history.fields.private",
  "promptsView.history.fields.source",
  "promptsView.history.fields.notes",
  "promptsView.history.fields.aiResponse",
];

const LOCALIZED_OPERATION_KEYS = [
  "promptsView.pagination.summary",
  "promptsView.pagination.previous",
  "promptsView.pagination.next",
  "promptsView.pin",
  "promptsView.unpin",
  "promptsView.duplicatePrompt",
  "promptsView.privatePrompt",
  "promptsView.editor.privatePrompt",
  "promptsView.bundle.export",
  "promptsView.bundle.import",
  "promptsView.bundle.preview",
  "promptsView.bundle.confirmImport",
  "promptsView.batch.selected",
  "promptsView.batch.move",
  "promptsView.batch.addTag",
  "promptsView.batch.delete",
  "promptsView.tags.manage",
  "promptsView.tags.rename",
  "promptsView.tags.delete",
];

describe("prompts view i18n keys (Req 21.3)", () => {
  it("resolves every rendered key to a non-empty string in the English bundle", () => {
    for (const key of KEYS) {
      const value = lookup(en, key);
      expect(typeof value, `missing key: ${key}`).toBe("string");
      expect((value as string).length, `empty key: ${key}`).toBeGreaterThan(0);
    }
  });

  it("localizes the new library operations in every supported bundle", () => {
    for (const [locale, bundle] of Object.entries({
      en,
      de,
      es,
      fr,
      ja,
      zh,
      "zh-TW": zhTW,
    })) {
      for (const key of LOCALIZED_OPERATION_KEYS) {
        const value = lookup(bundle, key);
        expect(typeof value, `${locale}: missing key ${key}`).toBe("string");
        expect((value as string).length, `${locale}: empty key ${key}`).toBeGreaterThan(0);
      }
    }
  });
});
