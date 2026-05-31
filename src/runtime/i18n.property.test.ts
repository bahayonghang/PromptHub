/**
 * Property-based tests for the Frontend Internationalization layer (Requirement 21).
 *
 * Implements the named properties from the design's Testing Strategy:
 *  - Property 41: Startup locale resolution (Req 21.5)
 *  - Property 42: Translation fallback chain (Req 21.6, 21.7)
 *  - Property 43: Locale persistence round-trip (Req 21.4)
 *  - Property 44: Locale key parity (Req 21.1)
 *
 * Unit-style examples for these same behaviours live in `./i18n.test.ts`; the
 * tests here exercise universal properties across generated inputs with
 * `fast-check`. Property 42 builds an isolated `i18next` instance with synthetic
 * resources so the fallback chain can be driven across a full presence matrix
 * without coupling to (or mutating) the shared singleton.
 */
import { beforeAll, describe, expect, it } from "vitest";
import fc from "fast-check";
import i18next from "i18next";
import i18n, {
  changeLocale,
  initI18n,
  normalizeLocale,
  resolveActiveLocale,
  SUPPORTED,
  type LocaleGateway,
  type SupportedLocale,
} from "./i18n";
import en from "../locales/en.json";

/** A controllable in-memory gateway standing in for the Runtime_Bridge. */
function makeGateway(overrides: Partial<LocaleGateway> = {}): LocaleGateway {
  const persisted = { value: null as string | null };
  return {
    loadPersistedLocale: async () => persisted.value,
    detectOsLocale: () => "en-US",
    persistLocale: async (locale) => {
      persisted.value = locale;
    },
    ...overrides,
  };
}

/** An arbitrary identifier free of i18next's default `.`/`:` separators. */
const alnumKey = fc
  .array(
    fc.constantFrom(
      ..."abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_".split(""),
    ),
    { minLength: 1, maxLength: 20 },
  )
  .map((chars) => chars.join(""));

// ---------------------------------------------------------------------------
// Property 41: Startup locale resolution (Req 21.5)
// ---------------------------------------------------------------------------

describe("Property 41: Startup locale resolution (Req 21.5)", () => {
  // OS-locale tags that MUST normalize to each supported locale.
  const SUPPORTED_TAG_SAMPLES: Record<SupportedLocale, string[]> = {
    en: ["en", "en-US", "EN-gb"],
    zh: ["zh", "zh-CN", "zh-Hans", "ZH-cn"],
    "zh-TW": ["zh-TW", "zh-Hant", "ZH-TW", "zh-hant"],
    ja: ["ja", "ja-JP", "JA"],
    fr: ["fr", "fr-FR", "fr-CA"],
    de: ["de", "de-DE", "DE-at"],
    es: ["es", "es-ES", "es-MX"],
  };

  // Tags whose language is not one of the seven, so resolution falls to English.
  const UNSUPPORTED_TAGS = ["ko-KR", "ru", "it-IT", "pt-BR", "ar", "xx", "hello", "123", ""];

  it("resolves an OS locale to its supported form when nothing is persisted", () => {
    const supportedCase = fc
      .constantFrom(...SUPPORTED)
      .chain((loc) =>
        fc.constantFrom(...SUPPORTED_TAG_SAMPLES[loc]).map((tag) => ({ tag, expected: loc })),
      );
    fc.assert(
      fc.property(supportedCase, fc.constantFrom(null, undefined, ""), ({ tag, expected }, empty) => {
        expect(resolveActiveLocale(empty, tag)).toBe(expected);
      }),
      { numRuns: 200 },
    );
  });

  it("falls back to English for OS locales outside the seven", () => {
    fc.assert(
      fc.property(
        fc.constantFrom(...UNSUPPORTED_TAGS),
        fc.constantFrom(null, undefined, ""),
        (tag, empty) => {
          expect(resolveActiveLocale(empty, tag)).toBe("en");
        },
      ),
    );
  });

  it("always yields a supported locale and ignores empty persisted values", () => {
    fc.assert(
      fc.property(fc.string(), (os) => {
        const expected = normalizeLocale(os);
        // The result is always one of the seven supported locales.
        expect(SUPPORTED).toContain(expected);
        // null / undefined / "" persisted are all treated as "nothing persisted".
        expect(resolveActiveLocale(null, os)).toBe(expected);
        expect(resolveActiveLocale(undefined, os)).toBe(expected);
        expect(resolveActiveLocale("", os)).toBe(expected);
      }),
      { numRuns: 300 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 42: Translation fallback chain (Req 21.6, 21.7)
// ---------------------------------------------------------------------------

describe("Property 42: Translation fallback chain (Req 21.6, 21.7)", () => {
  const NS = "translation";
  const RESERVED = ["both", "enOnly", "localeOnly"];
  // Isolated instance mirroring the production init config (fallbackLng: 'en'),
  // with synthetic resources spanning the key/locale presence matrix.
  const inst = i18next.createInstance();

  beforeAll(async () => {
    const resources: Record<string, { translation: Record<string, string> }> = {
      en: { translation: { both: "en_both", enOnly: "en_only" } },
    };
    for (const loc of SUPPORTED) {
      if (loc === "en") continue;
      resources[loc] = { translation: { both: `${loc}_both`, localeOnly: `${loc}_localeOnly` } };
    }
    await inst.init({
      resources,
      lng: "en",
      fallbackLng: "en",
      defaultNS: NS,
      interpolation: { escapeValue: false },
    });
  });

  it("returns the selected locale's string, then the English string, by presence", () => {
    const nonEn = SUPPORTED.filter((l) => l !== "en");
    const cases: { key: string; expected: (loc: SupportedLocale) => string }[] = [
      // Present in the selected locale -> selected locale's string.
      { key: "both", expected: (loc) => `${loc}_both` },
      { key: "localeOnly", expected: (loc) => `${loc}_localeOnly` },
      // Missing in the selected locale but present in English -> English (21.6).
      { key: "enOnly", expected: () => "en_only" },
    ];
    fc.assert(
      fc.property(fc.constantFrom(...nonEn), fc.constantFrom(...cases), (loc, c) => {
        expect(inst.getFixedT(loc)(c.key)).toBe(c.expected(loc));
      }),
    );
  });

  it("returns the key identifier when it is absent in both the locale and English (21.7)", () => {
    fc.assert(
      fc.property(
        fc.constantFrom(...SUPPORTED),
        alnumKey.filter((k) => !RESERVED.includes(k)),
        (loc, key) => {
          expect(inst.getFixedT(loc)(key)).toBe(key);
        },
      ),
      { numRuns: 200 },
    );
  });
});

// ---------------------------------------------------------------------------
// Property 43: Locale persistence round-trip (Req 21.4)
// ---------------------------------------------------------------------------

describe("Property 43: Locale persistence round-trip (Req 21.4)", () => {
  it("a persisted selection resolves back as the active locale on next launch", async () => {
    await fc.assert(
      fc.asyncProperty(fc.constantFrom(...SUPPORTED), async (locale) => {
        const gateway = makeGateway();
        await changeLocale(locale, gateway);
        const active = await initI18n(gateway);
        expect(active).toBe(locale);
      }),
    );
  });
});

// ---------------------------------------------------------------------------
// Property 44: Locale key parity (Req 21.1)
// ---------------------------------------------------------------------------

describe("Property 44: Locale key parity (Req 21.1)", () => {
  /** Flattens a nested resource bundle into its dotted leaf-key paths. */
  function flattenKeys(obj: Record<string, unknown>, prefix = ""): string[] {
    const keys: string[] = [];
    for (const [k, v] of Object.entries(obj)) {
      const full = prefix ? `${prefix}.${k}` : k;
      if (v !== null && typeof v === "object" && !Array.isArray(v)) {
        keys.push(...flattenKeys(v as Record<string, unknown>, full));
      } else {
        keys.push(full);
      }
    }
    return keys;
  }

  const EN_KEYS = flattenKeys(en as Record<string, unknown>);

  beforeAll(async () => {
    // Every supported bundle must load without throwing (no missing-bundle failure).
    for (const locale of SUPPORTED) {
      await import("./i18n").then((m) => m.ensureBundle(locale));
    }
  });

  it("resolves every English key in all seven locales (at minimum via the English fallback)", () => {
    expect(EN_KEYS.length).toBeGreaterThan(0);
    fc.assert(
      fc.property(fc.constantFrom(...SUPPORTED), (locale) => {
        const t = i18n.getFixedT(locale);
        // A key that resolves to itself means the bundle/fallback failed for it.
        const unresolved = EN_KEYS.filter((key) => {
          const value = t(key);
          return typeof value !== "string" || value === key;
        });
        expect(unresolved).toEqual([]);
      }),
      { numRuns: 30 },
    );
  });
});
