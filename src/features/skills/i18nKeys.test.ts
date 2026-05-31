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

/** Every i18n key the skill-management view renders (Req 21.3). */
const KEYS = [
  "skillsView.searchPlaceholder",
  "skillsView.newSkill",
  "skillsView.loading",
  "skillsView.noSkills",
  "skillsView.noSkillsHint",
  "skillsView.selectSkillTitle",
  "skillsView.selectSkillHint",
  "skillsView.untitled",
  "skillsView.favorite",
  "skillsView.unfavorite",
  "skillsView.deleteSkill",
  "skillsView.deleteSkillConfirm",
  "skillsView.editor.name",
  "skillsView.editor.namePlaceholder",
  "skillsView.editor.nameRequired",
  "skillsView.editor.description",
  "skillsView.editor.descriptionPlaceholder",
  "skillsView.editor.version",
  "skillsView.editor.versionPlaceholder",
  "skillsView.editor.author",
  "skillsView.editor.authorPlaceholder",
  "skillsView.editor.content",
  "skillsView.editor.contentPlaceholder",
  "skillsView.editor.tags",
  "skillsView.editor.addTagPlaceholder",
  "skillsView.editor.addTag",
  "skillsView.editor.save",
  "skillsView.editor.cancel",
  "skillsView.editor.create",
  "skillsView.preview.title",
  "skillsView.history.title",
  "skillsView.history.empty",
  "skillsView.history.emptyHint",
  "skillsView.history.saveVersion",
  "skillsView.history.notePlaceholder",
  "skillsView.history.versionLabel",
  "skillsView.history.restore",
  "skillsView.history.restoreConfirm",
  "skillsView.history.delete",
  "skillsView.platform.title",
  "skillsView.platform.unavailable",
  "skillsView.platform.empty",
  "skillsView.platform.detected",
  "skillsView.platform.notDetected",
  "skillsView.platform.detectedCount",
  "skillsView.platform.installed",
  "skillsView.platform.install",
  "skillsView.platform.uninstall",
  "skillsView.safety.title",
  "skillsView.safety.scan",
  "skillsView.safety.scanning",
  "skillsView.safety.empty",
  "skillsView.safety.noFindings",
  "skillsView.safety.score",
  "skillsView.safety.level.safe",
  "skillsView.safety.level.warn",
  "skillsView.safety.level.high-risk",
  "skillsView.safety.level.blocked",
  "skillsView.safety.severity.info",
  "skillsView.safety.severity.warn",
  "skillsView.safety.severity.high",
];

describe("skills view i18n keys (Req 21.3)", () => {
  it("resolves every rendered key to a non-empty string in the English bundle", () => {
    for (const key of KEYS) {
      const value = lookup(en, key);
      expect(typeof value, `missing key: ${key}`).toBe("string");
      expect((value as string).length, `empty key: ${key}`).toBeGreaterThan(0);
    }
  });
});
