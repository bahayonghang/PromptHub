import { useTranslation } from "react-i18next";
import { setTheme, type ThemeMode } from "../../../theme";
import { changeLocale, SUPPORTED, type SupportedLocale } from "../../../runtime/i18n";
import type { Settings } from "../types";

interface GeneralPanelProps {
  settings: Settings | null;
  /** Persists a partial settings update through the backend (Req 19.2). */
  onUpdate: (patch: Partial<Settings>) => void;
  /** Syncs the store's in-memory copy after theme/i18n persisted a change. */
  onLocalMerge: (patch: Partial<Settings>) => void;
}

/** The three theme modes offered in the appearance section (Req 22.x). */
const THEME_MODES: readonly ThemeMode[] = ["light", "dark", "system"];

/** Human-readable label key for each supported locale (Req 21.1). */
const LOCALE_LABEL_KEYS: Record<SupportedLocale, string> = {
  en: "settingsView.general.locale.en",
  zh: "settingsView.general.locale.zh",
  "zh-TW": "settingsView.general.locale.zh-TW",
  ja: "settingsView.general.locale.ja",
  fr: "settingsView.general.locale.fr",
  de: "settingsView.general.locale.de",
  es: "settingsView.general.locale.es",
};

/**
 * General preferences panel (Req 19.1, 19.2, 21, 22). Wires theme and language
 * switching to the existing `theme` and `runtime/i18n` modules — which already
 * persist their selection via `settings.update` — and exposes the auto-save and
 * startup preferences as direct settings updates. The theme/language controls
 * call the shared modules (so the change applies immediately without a restart)
 * and then merge the new value into the store's in-memory copy to avoid a
 * duplicate backend write.
 */
export function GeneralPanel({ settings, onUpdate, onLocalMerge }: GeneralPanelProps) {
  const { t, i18n } = useTranslation();

  const theme = (settings?.theme ?? "dark") as ThemeMode;
  const language = settings?.language ?? i18n.language;

  const handleTheme = (mode: ThemeMode) => {
    // setTheme applies the visual change and persists via settings.update.
    void setTheme(mode);
    onLocalMerge({ theme: mode });
  };

  const handleLanguage = (locale: SupportedLocale) => {
    // changeLocale switches the active locale and persists via settings.update.
    void changeLocale(locale);
    onLocalMerge({ language: locale });
  };

  const labelClass = "text-sm font-medium text-foreground";
  const hintClass = "text-xs text-muted-foreground";

  return (
    <div className="flex flex-col gap-6">
      {/* Appearance: theme */}
      <section className="flex flex-col gap-3">
        <div className="flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("settingsView.general.theme")}</h3>
          <p className={hintClass}>{t("settingsView.general.themeHint")}</p>
        </div>
        <div className="flex flex-wrap gap-2" role="group" aria-label={t("settingsView.general.theme")}>
          {THEME_MODES.map((mode) => (
            <button
              key={mode}
              type="button"
              aria-pressed={theme === mode}
              onClick={() => handleTheme(mode)}
              className={`rounded-md border px-4 py-2 text-sm transition-colors ${
                theme === mode
                  ? "border-primary bg-primary/15 text-foreground"
                  : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              {t(`settingsView.general.themeMode.${mode}`)}
            </button>
          ))}
        </div>
      </section>

      {/* Language */}
      <section className="flex flex-col gap-3">
        <div className="flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("settingsView.general.language")}</h3>
          <p className={hintClass}>{t("settingsView.general.languageHint")}</p>
        </div>
        <select
          value={SUPPORTED.includes(language as SupportedLocale) ? language : "en"}
          aria-label={t("settingsView.general.language")}
          onChange={(e) => handleLanguage(e.target.value as SupportedLocale)}
          className="w-full max-w-xs rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring"
        >
          {SUPPORTED.map((locale) => (
            <option key={locale} value={locale}>
              {t(LOCALE_LABEL_KEYS[locale])}
            </option>
          ))}
        </select>
      </section>

      {/* Editor: auto-save */}
      <section className="flex items-center justify-between gap-4">
        <div className="flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("settingsView.general.autoSave")}</h3>
          <p className={hintClass}>{t("settingsView.general.autoSaveHint")}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={settings?.autoSave ?? false}
          aria-label={t("settingsView.general.autoSave")}
          onClick={() => onUpdate({ autoSave: !(settings?.autoSave ?? false) })}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${
            settings?.autoSave ? "bg-primary" : "bg-input"
          }`}
        >
          <span
            className={`absolute top-0.5 h-5 w-5 rounded-full bg-background transition-transform ${
              settings?.autoSave ? "translate-x-5" : "translate-x-0.5"
            }`}
          />
        </button>
      </section>

      {/* Startup: launch at startup */}
      <section className="flex items-center justify-between gap-4">
        <div className="flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("settingsView.general.launchAtStartup")}</h3>
          <p className={hintClass}>{t("settingsView.general.launchAtStartupHint")}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={settings?.launchAtStartup ?? false}
          aria-label={t("settingsView.general.launchAtStartup")}
          onClick={() => onUpdate({ launchAtStartup: !(settings?.launchAtStartup ?? false) })}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${
            settings?.launchAtStartup ? "bg-primary" : "bg-input"
          }`}
        >
          <span
            className={`absolute top-0.5 h-5 w-5 rounded-full bg-background transition-transform ${
              settings?.launchAtStartup ? "translate-x-5" : "translate-x-0.5"
            }`}
          />
        </button>
      </section>
    </div>
  );
}
