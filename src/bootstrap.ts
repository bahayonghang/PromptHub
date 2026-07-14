import { applyAppearancePreferences } from "./appearance/preferences";
import { settingsApi } from "./features/settings/api";
import { useSettingsStore } from "./features/settings/settingsStore";
import type { Settings } from "./features/settings/types";
import { DEFAULT_LOCALE, initI18n, type SupportedLocale } from "./runtime/i18n";

export interface ApplicationStartDeps {
  loadSettings(): Promise<Settings>;
  initializeLocale(persistedLocale: string | null | undefined): Promise<SupportedLocale>;
  applyAppearance(settings: Settings, locale: SupportedLocale): void;
  hydrateSettings(settings: Settings): void;
  mount(): void;
}

export const DEFAULT_BOOTSTRAP_SETTINGS: Settings = {
  theme: "dark",
  language: "en",
  autoSave: true,
};

async function initializePersistedLocale(
  persistedLocale: string | null | undefined,
): Promise<SupportedLocale> {
  return initI18n({
    loadPersistedLocale: async () => persistedLocale ?? null,
    detectOsLocale: () => (typeof navigator === "undefined" ? "" : navigator.language),
    persistLocale: async () => {},
  });
}

const defaultDeps: Omit<ApplicationStartDeps, "mount"> = {
  loadSettings: () => settingsApi.getSettings(),
  initializeLocale: initializePersistedLocale,
  applyAppearance: applyAppearancePreferences,
  hydrateSettings: (settings) => useSettingsStore.getState().hydrateSettings(settings),
};

export async function startApplication(deps: ApplicationStartDeps): Promise<void> {
  let settings: Settings;
  try {
    settings = await deps.loadSettings();
  } catch {
    settings = { ...DEFAULT_BOOTSTRAP_SETTINGS };
  }

  let locale: SupportedLocale;
  try {
    locale = await deps.initializeLocale(settings.language);
  } catch {
    locale = DEFAULT_LOCALE;
  }

  deps.applyAppearance(settings, locale);
  deps.hydrateSettings(settings);
  deps.mount();
}

export function startDefaultApplication(mount: () => void): Promise<void> {
  return startApplication({ ...defaultDeps, mount });
}
