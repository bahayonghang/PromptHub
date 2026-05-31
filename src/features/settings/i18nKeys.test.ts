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

/** Every i18n key the settings view renders (Req 21.3). */
const KEYS = [
  "settingsView.title",
  "settingsView.restartRequired",
  "settingsView.sections.general",
  "settingsView.sections.security",
  "settingsView.sections.sync",
  "settingsView.sections.dataPath",

  // General
  "settingsView.general.theme",
  "settingsView.general.themeHint",
  "settingsView.general.themeMode.light",
  "settingsView.general.themeMode.dark",
  "settingsView.general.themeMode.system",
  "settingsView.general.language",
  "settingsView.general.languageHint",
  "settingsView.general.locale.en",
  "settingsView.general.locale.zh",
  "settingsView.general.locale.zh-TW",
  "settingsView.general.locale.ja",
  "settingsView.general.locale.fr",
  "settingsView.general.locale.de",
  "settingsView.general.locale.es",
  "settingsView.general.autoSave",
  "settingsView.general.autoSaveHint",
  "settingsView.general.launchAtStartup",
  "settingsView.general.launchAtStartupHint",

  // Security
  "settingsView.security.statusConfigured",
  "settingsView.security.statusNotConfigured",
  "settingsView.security.statusNotConfiguredHint",
  "settingsView.security.statusLocked",
  "settingsView.security.statusUnlocked",
  "settingsView.security.passwordLength",
  "settingsView.security.passwordMismatch",
  "settingsView.security.passwordRequired",
  "settingsView.security.currentPasswordRequired",
  "settingsView.security.setTitle",
  "settingsView.security.setHint",
  "settingsView.security.newPassword",
  "settingsView.security.confirmPassword",
  "settingsView.security.setButton",
  "settingsView.security.password",
  "settingsView.security.unlockTitle",
  "settingsView.security.unlockButton",
  "settingsView.security.lockButton",
  "settingsView.security.changeTitle",
  "settingsView.security.changeRestartHint",
  "settingsView.security.currentPassword",
  "settingsView.security.changeButton",

  // Data path
  "settingsView.dataPath.activeLabel",
  "settingsView.dataPath.restartPending",
  "settingsView.dataPath.changeTitle",
  "settingsView.dataPath.changeHint",
  "settingsView.dataPath.targetPlaceholder",
  "settingsView.dataPath.previewButton",
  "settingsView.dataPath.previewExists",
  "settingsView.dataPath.previewHasData",
  "settingsView.dataPath.previewIsCurrent",
  "settingsView.dataPath.previewRecommended",
  "settingsView.dataPath.previewYes",
  "settingsView.dataPath.previewNo",
  "settingsView.dataPath.previewAlreadyCurrent",
  "settingsView.dataPath.action.migrate",
  "settingsView.dataPath.action.switch",
  "settingsView.dataPath.applyAction.migrate",
  "settingsView.dataPath.applyAction.switch",
  "settingsView.dataPath.applyAction.overwrite",
  "settingsView.dataPath.applyConfirm",
  "settingsView.dataPath.recoveryTitle",
  "settingsView.dataPath.recoveryHint",
  "settingsView.dataPath.recoveryScan",
  "settingsView.dataPath.recoveryUnavailable",
  "settingsView.dataPath.recoveryApply",
  "settingsView.dataPath.recoveryEmpty",
  "settingsView.dataPath.recoveryConfirm",

  // Sync
  "settingsView.sync.testConnection",
  "settingsView.sync.testing",
  "settingsView.sync.webdav.title",
  "settingsView.sync.webdav.url",
  "settingsView.sync.webdav.username",
  "settingsView.sync.webdav.password",
  "settingsView.sync.webdav.urlRequired",
  "settingsView.sync.webdav.urlScheme",
  "settingsView.sync.s3.title",
  "settingsView.sync.s3.endpoint",
  "settingsView.sync.s3.region",
  "settingsView.sync.s3.bucket",
  "settingsView.sync.s3.accessKey",
  "settingsView.sync.s3.secretKey",
  "settingsView.sync.s3.endpointRequired",
  "settingsView.sync.s3.endpointScheme",
  "settingsView.sync.s3.regionRequired",
  "settingsView.sync.s3.bucketRequired",
  "settingsView.sync.s3.accessKeyRequired",
  "settingsView.sync.s3.secretKeyRequired",
  "settingsView.sync.export.title",
  "settingsView.sync.export.hint",
  "settingsView.sync.export.data",
  "settingsView.sync.export.media",
  "settingsView.sync.export.skill",
  "settingsView.sync.export.rule",
  "settingsView.sync.export.button",
  "settingsView.sync.export.exporting",
  "settingsView.sync.export.done",
  "settingsView.sync.backup.title",
  "settingsView.sync.backup.hint",
  "settingsView.sync.backup.create",
  "settingsView.sync.backup.empty",
  "settingsView.sync.backup.restore",
  "settingsView.sync.backup.delete",
  "settingsView.sync.backup.restoreConfirm",
  "settingsView.sync.backup.deleteConfirm",
];

describe("settings view i18n keys (Req 21.3)", () => {
  it("resolves every rendered key to a non-empty string in the English bundle", () => {
    for (const key of KEYS) {
      const value = lookup(en, key);
      expect(typeof value, `missing key: ${key}`).toBe("string");
      expect((value as string).length, `empty key: ${key}`).toBeGreaterThan(0);
    }
  });
});
