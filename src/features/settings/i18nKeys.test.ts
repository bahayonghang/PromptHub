import { describe, expect, it } from "vitest";
import en from "../../locales/en.json";
import zh from "../../locales/zh.json";
import zhTW from "../../locales/zh-TW.json";
import ja from "../../locales/ja.json";
import fr from "../../locales/fr.json";
import de from "../../locales/de.json";
import es from "../../locales/es.json";

/** Every shipped locale bundle, keyed by its locale id (Req 21.1, 21.3). */
const BUNDLES: Record<string, unknown> = { en, zh, "zh-TW": zhTW, ja, fr, de, es };

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
  "settingsView.preferences.saving",
  "settingsView.preferences.saved",
  "settingsView.preferences.unsaved",
  "settingsView.preferences.retry",
  "settingsView.sections.appearance",
  "settingsView.sections.general",
  "settingsView.sections.security",
  "settingsView.sections.sync",
  "settingsView.sections.dataPath",

  // Appearance
  "settingsView.appearance.flavor",
  "settingsView.appearance.themeFamily",
  "settingsView.appearance.themeFamilyHint",
  "settingsView.appearance.familyOption.prompthub",
  "settingsView.appearance.familyOption.catppuccin",
  "settingsView.appearance.familyOption.claude",
  "settingsView.appearance.familyDescription.prompthub",
  "settingsView.appearance.familyDescription.catppuccin",
  "settingsView.appearance.familyDescription.claude",
  "settingsView.appearance.colorMode",
  "settingsView.appearance.catppuccinDarkVariant",
  "settingsView.appearance.darkVariantOption.frappe",
  "settingsView.appearance.darkVariantOption.macchiato",
  "settingsView.appearance.darkVariantOption.mocha",
  "settingsView.appearance.interfaceFonts",
  "settingsView.appearance.interfaceFontsHint",
  "settingsView.appearance.primaryFont",
  "settingsView.appearance.fallbackFont",
  "settingsView.appearance.addFallback",
  "settingsView.appearance.moveFontUp",
  "settingsView.appearance.moveFontDown",
  "settingsView.appearance.removeFont",
  "settingsView.appearance.systemFontsLoading",
  "settingsView.appearance.systemFontsEmpty",
  "settingsView.appearance.systemFontsError",
  "settingsView.appearance.specimenMultilingual",
  "settingsView.appearance.language",
  "settingsView.appearance.accentColor",
  "settingsView.appearance.displayFont",
  "settingsView.appearance.bodyFont",
  "settingsView.appearance.fontScale",
  "settingsView.appearance.density",
  "settingsView.appearance.preview",
  "settingsView.appearance.summary",
  "settingsView.appearance.specimenDisplay",
  "settingsView.appearance.specimenBody",
  "settingsView.appearance.specimenAction",
  "settingsView.appearance.flavorOption.Latte",
  "settingsView.appearance.flavorOption.Frappé",
  "settingsView.appearance.flavorOption.Macchiato",
  "settingsView.appearance.flavorOption.Mocha",
  "settingsView.appearance.flavorOption.Claude Light",
  "settingsView.appearance.flavorOption.Claude Dark",
  "settingsView.appearance.accentOption.Rosewater",
  "settingsView.appearance.accentOption.Flamingo",
  "settingsView.appearance.accentOption.Pink",
  "settingsView.appearance.accentOption.Mauve",
  "settingsView.appearance.accentOption.Red",
  "settingsView.appearance.accentOption.Maroon",
  "settingsView.appearance.accentOption.Peach",
  "settingsView.appearance.accentOption.Yellow",
  "settingsView.appearance.accentOption.Green",
  "settingsView.appearance.accentOption.Teal",
  "settingsView.appearance.accentOption.Sky",
  "settingsView.appearance.accentOption.Sapphire",
  "settingsView.appearance.accentOption.Blue",
  "settingsView.appearance.accentOption.Lavender",
  "settingsView.appearance.accentOption.Violet",
  "settingsView.appearance.fontOption.System",
  "settingsView.appearance.fontOption.Inter",
  "settingsView.appearance.fontOption.Space Grotesk",
  "settingsView.appearance.fontOption.JetBrains Mono",
  "settingsView.appearance.fontGroup.builtin",
  "settingsView.appearance.fontGroup.system",
  "settingsView.appearance.fontScaleOption.Small",
  "settingsView.appearance.fontScaleOption.Default",
  "settingsView.appearance.fontScaleOption.Large",
  "settingsView.appearance.fontScaleOption.Extra Large",
  "settingsView.appearance.densityOption.Compact",
  "settingsView.appearance.densityOption.Default",
  "settingsView.appearance.densityOption.Comfortable",
  "settingsView.appearance.locale.en",
  "settingsView.appearance.locale.zh",
  "settingsView.appearance.locale.zh-TW",
  "settingsView.appearance.locale.ja",
  "settingsView.appearance.locale.fr",
  "settingsView.appearance.locale.de",
  "settingsView.appearance.locale.es",

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
  // Titles for the themed ConfirmDialog that replaced window.confirm.
  "settingsView.sync.backup.restoreTitle",
  "settingsView.sync.backup.deleteTitle",
  "settingsView.dataPath.applyTitle",
  "settingsView.dataPath.recoveryTitle",
  "settingsView.sync.backup.deleteConfirm",
];

describe("settings view i18n keys (Req 21.3)", () => {
  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    it(`resolves every rendered key to a non-empty string in the ${locale} bundle`, () => {
      for (const key of KEYS) {
        const value = lookup(bundle, key);
        expect(typeof value, `[${locale}] missing key: ${key}`).toBe("string");
        expect((value as string).length, `[${locale}] empty key: ${key}`).toBeGreaterThan(0);
      }
    });
  }
});
