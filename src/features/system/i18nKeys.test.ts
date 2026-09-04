import { describe, expect, it } from "vitest";
import en from "../../locales/en.json";
import zh from "../../locales/zh.json";
import zhTW from "../../locales/zh-TW.json";
import ja from "../../locales/ja.json";
import fr from "../../locales/fr.json";
import de from "../../locales/de.json";
import es from "../../locales/es.json";

/** Resolves a dotted i18n key against a bundle, returning undefined if absent. */
function lookup(bundle: unknown, key: string): unknown {
  return key.split(".").reduce<unknown>((node, part) => {
    if (node !== null && typeof node === "object") {
      return (node as Record<string, unknown>)[part];
    }
    return undefined;
  }, bundle);
}

/** Every i18n key the system view renders (Req 21.3). */
const KEYS = [
  // Settings section label
  "settingsView.sections.system",

  // Window controls (Req 20.1, 20.2)
  "systemView.window.minimize",
  "systemView.window.maximize",
  "systemView.window.restore",
  "systemView.window.close",
  "systemView.window.enterFullscreen",
  "systemView.window.exitFullscreen",

  // Close action + auto-launch (Req 20.4, 20.5)
  "systemView.window.closeActionTitle",
  "systemView.window.closeActionHint",
  "systemView.window.closeAction.ask",
  "systemView.window.closeAction.minimize",
  "systemView.window.closeAction.exit",
  "systemView.window.autoLaunch",
  "systemView.window.autoLaunchHint",

  // Close dialog (Req 20.4)
  "systemView.close.title",
  "systemView.close.message",
  "systemView.close.cancel",
  "systemView.close.minimize",
  "systemView.close.confirm",

  // Shortcuts (Req 20.6, 20.11)
  "systemView.shortcuts.title",
  "systemView.shortcuts.hint",
  "systemView.shortcuts.empty",
  "systemView.shortcuts.action",
  "systemView.shortcuts.actionPlaceholder",
  "systemView.shortcuts.accelerator",
  "systemView.shortcuts.acceleratorPlaceholder",
  "systemView.shortcuts.mode",
  "systemView.shortcuts.modeGlobal",
  "systemView.shortcuts.modeLocal",
  "systemView.shortcuts.add",
  "systemView.shortcuts.remove",
  "systemView.shortcuts.save",
  "systemView.shortcuts.lastTriggered",

  // Notifications (Req 20.7, 20.13)
  "systemView.notifications.title",
  "systemView.notifications.hint",
  "systemView.notifications.titleLabel",
  "systemView.notifications.bodyLabel",
  "systemView.notifications.send",
  "systemView.notifications.sent",

  // Updater (Req 24.2-24.7)
  "systemView.updater.title",
  "systemView.updater.hint",
  "systemView.updater.currentVersion",
  "systemView.updater.check",
  "systemView.updater.checking",
  "systemView.updater.upToDate",
  "systemView.updater.available",
  "systemView.updater.download",
  "systemView.updater.downloading",
  "systemView.updater.downloadingPercent",
  "systemView.updater.downloadingBytes",
  "systemView.updater.downloaded",
  "systemView.updater.install",
  "systemView.updater.installing",
  "systemView.updater.unavailable",

  // Runtime paths + cache (Req 20.8, 20.9, 20.10)
  "systemView.paths.title",
  "systemView.paths.platform",
  "systemView.paths.data",
  "systemView.paths.database",
  "systemView.paths.media",
  "systemView.paths.rule",
  "systemView.paths.backup",
  "systemView.paths.log",
  "systemView.paths.open",
  "systemView.paths.cache",
  "systemView.paths.clearCache",
  "systemView.paths.unavailable",
];

const BUNDLES: Record<string, unknown> = { en, zh, "zh-TW": zhTW, ja, fr, de, es };

describe("system view i18n keys (Req 21.3)", () => {
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
