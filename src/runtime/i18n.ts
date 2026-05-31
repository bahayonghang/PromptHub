/**
 * Frontend Internationalization layer (Requirement 21).
 *
 * Wraps `i18next` + `react-i18next` with:
 *  - the seven shipped locale bundles (`en`, `zh`, `zh-TW`, `ja`, `fr`, `de`, `es`);
 *  - an English (`en`) fallback chain (21.6, 21.7);
 *  - OS-locale normalization to a supported locale (21.5);
 *  - startup resolution (persisted -> normalized OS -> English) and persistence
 *    of the selected locale to the backend via `settings.update` (21.4).
 *
 * Only the English bundle is loaded eagerly; the other six are imported lazily
 * the first time their locale becomes active (lazy resource loading).
 */
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../locales/en.json";
import { runtime, type RuntimeBridge } from "./index";

/** The seven locales the Frontend ships translations for (Req 21.1). */
export const SUPPORTED = ["en", "zh", "zh-TW", "ja", "fr", "de", "es"] as const;

/** One of the seven supported locale identifiers. */
export type SupportedLocale = (typeof SUPPORTED)[number];

/** The canonical/reference locale and ultimate fallback (Req 21.5, 21.6). */
export const DEFAULT_LOCALE: SupportedLocale = "en";

/** The i18next namespace every bundle is registered under. */
const NS = "translation";

/**
 * Maps an arbitrary OS/BCP-47 locale string onto one of the seven supported
 * locales (Req 21.5). `zh-Hant`/`zh-TW` map to Traditional Chinese, any other
 * `zh*` to Simplified Chinese, and `ja`/`fr`/`de`/`es` by language prefix;
 * everything else falls back to English. Every supported locale is a fixed
 * point of this mapping, so normalizing an already-supported value is a no-op.
 */
export function normalizeLocale(raw: string): SupportedLocale {
  const l = (raw || "").toLowerCase();
  if (l === "zh-tw" || l === "zh-hant") return "zh-TW";
  if (l.startsWith("zh")) return "zh";
  if (l.startsWith("ja")) return "ja";
  if (l.startsWith("fr")) return "fr";
  if (l.startsWith("de")) return "de";
  if (l.startsWith("es")) return "es";
  return "en";
}

/** Type guard: is `value` one of the seven supported locale identifiers? */
export function isSupportedLocale(value: unknown): value is SupportedLocale {
  return typeof value === "string" && (SUPPORTED as readonly string[]).includes(value);
}

/**
 * Resolves the active locale at startup (Req 21.5): a previously persisted
 * locale wins; otherwise the normalized OS locale is used; both ultimately fall
 * back to English. `persisted` is treated as absent when `null`/`undefined`/`""`.
 */
export function resolveActiveLocale(
  persisted: string | null | undefined,
  osLocale: string | null | undefined,
): SupportedLocale {
  if (persisted != null && persisted !== "") {
    return normalizeLocale(persisted);
  }
  return normalizeLocale(osLocale ?? "");
}

/**
 * Lazy loaders for each locale bundle. English is bundled eagerly; the rest are
 * dynamically imported the first time they are needed.
 */
const LOADERS: Record<SupportedLocale, () => Promise<Record<string, unknown>>> = {
  en: async () => en as Record<string, unknown>,
  zh: async () => (await import("../locales/zh.json")).default as Record<string, unknown>,
  "zh-TW": async () => (await import("../locales/zh-TW.json")).default as Record<string, unknown>,
  ja: async () => (await import("../locales/ja.json")).default as Record<string, unknown>,
  fr: async () => (await import("../locales/fr.json")).default as Record<string, unknown>,
  de: async () => (await import("../locales/de.json")).default as Record<string, unknown>,
  es: async () => (await import("../locales/es.json")).default as Record<string, unknown>,
};

/** Loads a locale's resource bundle into i18next once, if not already present. */
export async function ensureBundle(locale: SupportedLocale): Promise<void> {
  if (i18n.hasResourceBundle(locale, NS)) return;
  const resources = await LOADERS[locale]();
  i18n.addResourceBundle(locale, NS, resources, true, true);
}

let initialized = false;

/**
 * Initializes the shared i18next instance once, with English loaded eagerly and
 * `fallbackLng: 'en'`. A key missing in the active locale resolves to the
 * English string (21.6); a key missing in both resolves to the key identifier
 * itself (21.7) — both are i18next defaults left untouched here.
 */
export function ensureI18nInitialized(): typeof i18n {
  if (initialized) return i18n;
  void i18n.use(initReactI18next).init({
    resources: { en: { [NS]: en } },
    lng: DEFAULT_LOCALE,
    fallbackLng: DEFAULT_LOCALE,
    defaultNS: NS,
    interpolation: { escapeValue: false },
  });
  initialized = true;
  return i18n;
}

/**
 * The side channels i18n needs from the backend, isolated behind an interface so
 * startup logic can be unit-tested without a live Runtime_Bridge.
 */
export interface LocaleGateway {
  /** Returns the previously persisted locale, or `null` if none is set. */
  loadPersistedLocale(): Promise<string | null>;
  /** Returns the host operating system's locale string. */
  detectOsLocale(): string;
  /** Persists the selected locale so it is active on the next launch (21.4). */
  persistLocale(locale: SupportedLocale): Promise<void>;
}

/** Builds the default {@link LocaleGateway} bound to the Runtime_Bridge. */
function bridgeGateway(bridge: RuntimeBridge): LocaleGateway {
  return {
    async loadPersistedLocale() {
      try {
        const settings = await bridge.invoke<{ language?: string }>("settings.get");
        const lang = settings?.language;
        return typeof lang === "string" && lang !== "" ? lang : null;
      } catch {
        // A failed read must not block startup; fall through to OS detection.
        return null;
      }
    },
    detectOsLocale() {
      return typeof navigator !== "undefined" ? navigator.language : "";
    },
    async persistLocale(locale) {
      await bridge.invoke("settings.update", { patch: { language: locale } });
    },
  };
}

/**
 * Startup entry point (Req 21.5): initializes i18next, resolves the active
 * locale from the persisted setting or the OS locale, loads its bundle, and
 * applies it. Returns the resolved locale.
 */
export async function initI18n(
  gateway: LocaleGateway = bridgeGateway(runtime),
): Promise<SupportedLocale> {
  ensureI18nInitialized();
  const persisted = await gateway.loadPersistedLocale();
  const active = resolveActiveLocale(persisted, gateway.detectOsLocale());
  await ensureBundle(active);
  if (i18n.language !== active) {
    await i18n.changeLanguage(active);
  }
  return active;
}

/**
 * Switches the active locale at runtime (Req 21.2) and persists the selection
 * (Req 21.4). Loads the target bundle lazily before switching so the change
 * applies without a restart.
 */
export async function changeLocale(
  locale: SupportedLocale,
  gateway: LocaleGateway = bridgeGateway(runtime),
): Promise<void> {
  await ensureBundle(locale);
  await i18n.changeLanguage(locale);
  await gateway.persistLocale(locale);
}

// Initialize eagerly on import so `useTranslation()` works for any component
// that renders before {@link initI18n} completes its async locale resolution.
ensureI18nInitialized();

export default i18n;
