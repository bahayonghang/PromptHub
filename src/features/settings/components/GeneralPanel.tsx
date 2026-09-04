import { useTranslation } from "react-i18next";
import {
  DEFAULT_LOCALE,
  isSupportedLocale,
  SUPPORTED,
  type SupportedLocale,
} from "../../../runtime/i18n";
import type { PreferenceSaveStatus } from "../settingsStore";
import type { Settings } from "../types";
import { PreferenceStatus } from "./PreferenceStatus";

import { Select } from "../../../components/ui";

interface GeneralPanelProps {
  settings: Settings | null;
  /** Persists a partial settings update through the backend (Req 19.2). */
  onUpdate: (patch: Partial<Settings>) => void;
  onLanguageChange: (locale: SupportedLocale) => void;
  onRetryLanguage: () => void;
  languageStatus?: PreferenceSaveStatus;
  languageError?: string;
}

/**
 * General preferences panel (Req 19.1, 19.2). Language uses the canonical
 * preference action, while auto-save and launch-at-startup remain direct
 * settings updates.
 */
export function GeneralPanel({
  settings,
  onUpdate,
  onLanguageChange,
  onRetryLanguage,
  languageStatus,
  languageError,
}: GeneralPanelProps) {
  const { t } = useTranslation();

  const labelClass = "text-body font-medium text-foreground";
  const hintClass = "text-label text-muted-foreground";

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <div className="min-w-0 flex flex-col gap-0.5">
          <label htmlFor="settings-language" className={labelClass}>
            {t("settingsView.general.language")}
          </label>
          <p id="settings-language-hint" className={hintClass}>
            {t("settingsView.general.languageHint")}
          </p>
        </div>
        <Select
          id="settings-language"
          value={isSupportedLocale(settings?.language) ? settings.language : DEFAULT_LOCALE}
          aria-describedby="settings-language-hint"
          disabled={settings == null || languageStatus === "saving"}
          onChange={(event) => onLanguageChange(event.target.value as SupportedLocale)}
          wrapperClassName="w-full max-w-sm"
        >
          {SUPPORTED.map((locale) => (
            <option key={locale} value={locale}>
              {t(`settingsView.general.locale.${locale}`)}
            </option>
          ))}
        </Select>
        <PreferenceStatus
          status={languageStatus}
          errorKey={languageError}
          onRetry={onRetryLanguage}
        />
      </section>

      {/* Editor: auto-save */}
      <section className="flex items-center justify-between gap-4">
        <div className="min-w-0 flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("settingsView.general.autoSave")}</h3>
          <p className={hintClass}>{t("settingsView.general.autoSaveHint")}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={settings?.autoSave ?? false}
          aria-label={t("settingsView.general.autoSave")}
          disabled={settings == null}
          onClick={() => onUpdate({ autoSave: !(settings?.autoSave ?? false) })}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors duration-fast ease-out disabled:cursor-not-allowed disabled:opacity-50 ${
            settings?.autoSave ? "bg-primary" : "bg-input"
          }`}
        >
          <span
            className={`absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform duration-base ease-spring ${
              settings?.autoSave ? "translate-x-5" : "translate-x-0"
            }`}
          />
        </button>
      </section>

      {/* Startup: launch at startup */}
      <section className="flex items-center justify-between gap-4">
        <div className="min-w-0 flex flex-col gap-0.5">
          <h3 className={labelClass}>{t("settingsView.general.launchAtStartup")}</h3>
          <p className={hintClass}>{t("settingsView.general.launchAtStartupHint")}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={settings?.launchAtStartup ?? false}
          aria-label={t("settingsView.general.launchAtStartup")}
          disabled={settings == null}
          onClick={() => onUpdate({ launchAtStartup: !(settings?.launchAtStartup ?? false) })}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors duration-fast ease-out disabled:cursor-not-allowed disabled:opacity-50 ${
            settings?.launchAtStartup ? "bg-primary" : "bg-input"
          }`}
        >
          <span
            className={`absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform duration-base ease-spring ${
              settings?.launchAtStartup ? "translate-x-5" : "translate-x-0"
            }`}
          />
        </button>
      </section>
    </div>
  );
}
