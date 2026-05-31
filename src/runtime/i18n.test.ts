import { describe, expect, it, vi } from "vitest";
import {
  changeLocale,
  initI18n,
  isSupportedLocale,
  normalizeLocale,
  resolveActiveLocale,
  SUPPORTED,
  type LocaleGateway,
  type SupportedLocale,
} from "./i18n";

/** A controllable in-memory gateway standing in for the Runtime_Bridge. */
function makeGateway(overrides: Partial<LocaleGateway> = {}): {
  gateway: LocaleGateway;
  persisted: { value: string | null };
} {
  const persisted = { value: null as string | null };
  const gateway: LocaleGateway = {
    loadPersistedLocale: async () => persisted.value,
    detectOsLocale: () => "en-US",
    persistLocale: async (locale) => {
      persisted.value = locale;
    },
    ...overrides,
  };
  return { gateway, persisted };
}

describe("normalizeLocale (Req 21.5)", () => {
  it("maps Traditional Chinese variants to zh-TW", () => {
    expect(normalizeLocale("zh-TW")).toBe("zh-TW");
    expect(normalizeLocale("zh-Hant")).toBe("zh-TW");
    expect(normalizeLocale("ZH-HANT")).toBe("zh-TW");
  });

  it("maps other Chinese variants to zh", () => {
    expect(normalizeLocale("zh")).toBe("zh");
    expect(normalizeLocale("zh-CN")).toBe("zh");
    expect(normalizeLocale("zh-Hans")).toBe("zh");
  });

  it("maps ja/fr/de/es by language prefix", () => {
    expect(normalizeLocale("ja-JP")).toBe("ja");
    expect(normalizeLocale("fr-FR")).toBe("fr");
    expect(normalizeLocale("de-DE")).toBe("de");
    expect(normalizeLocale("es-ES")).toBe("es");
  });

  it("falls back to English for unsupported or empty locales", () => {
    expect(normalizeLocale("en-US")).toBe("en");
    expect(normalizeLocale("ko-KR")).toBe("en");
    expect(normalizeLocale("")).toBe("en");
  });

  it("is idempotent on every supported locale (fixed point)", () => {
    for (const locale of SUPPORTED) {
      expect(normalizeLocale(locale)).toBe(locale);
    }
  });
});

describe("isSupportedLocale", () => {
  it("accepts the seven supported locales and rejects others", () => {
    for (const locale of SUPPORTED) {
      expect(isSupportedLocale(locale)).toBe(true);
    }
    expect(isSupportedLocale("ko")).toBe(false);
    expect(isSupportedLocale(null)).toBe(false);
    expect(isSupportedLocale(42)).toBe(false);
  });
});

describe("resolveActiveLocale (Req 21.5)", () => {
  it("prefers a persisted locale over the OS locale", () => {
    expect(resolveActiveLocale("ja", "fr-FR")).toBe("ja");
  });

  it("normalizes a persisted locale", () => {
    expect(resolveActiveLocale("zh-Hant", "en-US")).toBe("zh-TW");
  });

  it("falls back to the normalized OS locale when none is persisted", () => {
    expect(resolveActiveLocale(null, "de-DE")).toBe("de");
    expect(resolveActiveLocale("", "fr-CA")).toBe("fr");
  });

  it("falls back to English when neither is supported", () => {
    expect(resolveActiveLocale(null, "ko-KR")).toBe("en");
    expect(resolveActiveLocale(undefined, undefined)).toBe("en");
  });
});

describe("initI18n (Req 21.5)", () => {
  it("resolves and applies the persisted locale", async () => {
    const { gateway } = makeGateway({
      loadPersistedLocale: async () => "ja",
      detectOsLocale: () => "fr-FR",
    });
    const active = await initI18n(gateway);
    expect(active).toBe("ja");
  });

  it("resolves the normalized OS locale when nothing is persisted", async () => {
    const { gateway } = makeGateway({
      loadPersistedLocale: async () => null,
      detectOsLocale: () => "de-DE",
    });
    const active = await initI18n(gateway);
    expect(active).toBe("de");
  });
});

describe("changeLocale (Req 21.2, 21.4)", () => {
  it("persists the selected locale via the gateway", async () => {
    const persist = vi.fn(async () => {});
    const { gateway } = makeGateway({ persistLocale: persist });
    await changeLocale("es", gateway);
    expect(persist).toHaveBeenCalledWith("es");
  });

  it("round-trips: a persisted selection resolves back on next launch (Req 21.4)", async () => {
    const { gateway } = makeGateway({ detectOsLocale: () => "en-US" });
    for (const locale of SUPPORTED) {
      await changeLocale(locale, gateway);
      const next = await initI18n(gateway);
      expect(next).toBe(locale);
    }
  });
});

describe("translation fallback chain (Req 21.6, 21.7)", () => {
  it("returns the key identifier for a key absent in every bundle", async () => {
    const { gateway } = makeGateway({ loadPersistedLocale: async () => "en" });
    const i18n = (await import("./i18n")).default;
    await initI18n(gateway);
    const missing = "this.key.does.not.exist" as const;
    expect(i18n.t(missing)).toBe(missing);
  });

  it("resolves a real English key", async () => {
    const { gateway } = makeGateway({ loadPersistedLocale: async () => "en" });
    const i18n = (await import("./i18n")).default;
    await initI18n(gateway);
    expect(i18n.t("app.name")).toBe("PromptHub");
  });
});

// Type-level sanity: SupportedLocale stays in sync with SUPPORTED.
const _localeTypeCheck: SupportedLocale = SUPPORTED[0];
void _localeTypeCheck;
