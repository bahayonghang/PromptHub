import { describe, expect, it } from "vitest";
import en from "../../locales/en.json";
import de from "../../locales/de.json";
import es from "../../locales/es.json";
import fr from "../../locales/fr.json";
import ja from "../../locales/ja.json";
import zh from "../../locales/zh.json";
import zhTW from "../../locales/zh-TW.json";

type LocaleTree = Record<string, unknown>;

function flattenLeaves(
  tree: LocaleTree,
  prefix = "promptsView",
): Map<string, unknown> {
  const leaves = new Map<string, unknown>();

  for (const [key, value] of Object.entries(tree)) {
    const path = `${prefix}.${key}`;
    if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      for (const [nestedPath, nestedValue] of flattenLeaves(
        value as LocaleTree,
        path,
      )) {
        leaves.set(nestedPath, nestedValue);
      }
    } else {
      leaves.set(path, value);
    }
  }

  return leaves;
}

const BUNDLES = {
  en,
  de,
  es,
  fr,
  ja,
  zh,
  "zh-TW": zhTW,
} as const;

describe("prompts view i18n keys (Req 21.3)", () => {
  const canonical = flattenLeaves(en.promptsView);
  const canonicalKeys = [...canonical.keys()].sort();

  it("defines a non-empty English string for every Prompts key", () => {
    for (const [key, value] of canonical) {
      expect(typeof value, `${key} must be a string`).toBe("string");
      expect((value as string).trim(), `${key} must not be empty`).not.toBe("");
    }
  });

  it("keeps every supported bundle in complete Prompts key parity", () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const localized = flattenLeaves(bundle.promptsView);
      expect([...localized.keys()].sort(), `${locale} Prompts keys`).toEqual(
        canonicalKeys,
      );

      for (const [key, value] of localized) {
        expect(typeof value, `${locale}: ${key} must be a string`).toBe("string");
        expect(
          (value as string).trim(),
          `${locale}: ${key} must not be empty`,
        ).not.toBe("");
      }
    }
  });
});
