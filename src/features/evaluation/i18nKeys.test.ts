import { describe, expect, it } from "vitest";
import de from "../../locales/de.json";
import en from "../../locales/en.json";
import es from "../../locales/es.json";
import fr from "../../locales/fr.json";
import ja from "../../locales/ja.json";
import zh from "../../locales/zh.json";
import zhTW from "../../locales/zh-TW.json";

describe("evaluation i18n keys", () => {
  it("keeps the complete evaluation namespace in every supported locale", () => {
    const expected = Object.keys(en.evaluation).sort();
    for (const [locale, bundle] of Object.entries({
      en,
      de,
      es,
      fr,
      ja,
      zh,
      "zh-TW": zhTW,
    })) {
      expect(Object.keys(bundle.evaluation).sort(), locale).toEqual(expected);
      for (const key of expected) {
        expect(
          (bundle.evaluation as Record<string, string>)[key].length,
          `${locale}: empty evaluation.${key}`,
        ).toBeGreaterThan(0);
      }
    }
  });
});
