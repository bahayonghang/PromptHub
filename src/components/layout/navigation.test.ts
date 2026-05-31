import { describe, expect, it } from "vitest";
import { FOOTER_NAV, NAV_ENTRIES, PRIMARY_NAV } from "./navigation";
import { APP_VIEWS } from "../../store/appStore";
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

describe("navigation entries (Req 22.3)", () => {
  it("has exactly one entry per major view", () => {
    expect(NAV_ENTRIES).toHaveLength(APP_VIEWS.length);
    const views = NAV_ENTRIES.map((entry) => entry.view).sort();
    expect(views).toEqual([...APP_VIEWS].sort());
  });

  it("splits into primary entries plus a settings footer", () => {
    expect(PRIMARY_NAV.map((e) => e.view)).toEqual(["prompts", "skills"]);
    expect(FOOTER_NAV.map((e) => e.view)).toEqual(["settings"]);
    expect(PRIMARY_NAV.length + FOOTER_NAV.length).toBe(NAV_ENTRIES.length);
  });

  it("provides an icon component for every entry (Req 22.4)", () => {
    for (const entry of NAV_ENTRIES) {
      expect(entry.icon).toBeDefined();
    }
  });

  it("uses translation keys that resolve in the English bundle (Req 21.3)", () => {
    for (const entry of NAV_ENTRIES) {
      const value = lookup(en, entry.labelKey);
      expect(typeof value).toBe("string");
      expect((value as string).length).toBeGreaterThan(0);
    }
  });

  it("resolves the shell's own labels in the English bundle (Req 21.3)", () => {
    const keys = [
      "shell.primaryNav",
      "shell.collapseSidebar",
      "shell.expandSidebar",
      "shell.startupFailedTitle",
      "shell.promptsPlaceholder",
      "shell.skillsPlaceholder",
      "shell.settingsPlaceholder",
    ];
    for (const key of keys) {
      const value = lookup(en, key);
      expect(typeof value).toBe("string");
      expect((value as string).length).toBeGreaterThan(0);
    }
  });
});
