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

import { Select, SettingRow, Switch } from "../../../components/ui";

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

  return (
    <div className="flex flex-col gap-6">
      <SettingRow
        title={t("settingsView.general.language")}
        hint={t("settingsView.general.languageHint")}
        layout="stacked"
        footer={
          <PreferenceStatus
            status={languageStatus}
            errorKey={languageError}
            onRetry={onRetryLanguage}
          />
        }
      >
        {({ titleId, hintId }) => (
          <Select
            aria-labelledby={titleId}
            aria-describedby={hintId}
            value={isSupportedLocale(settings?.language) ? settings.language : DEFAULT_LOCALE}
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
        )}
      </SettingRow>

      <SettingRow
        title={t("settingsView.general.autoSave")}
        hint={t("settingsView.general.autoSaveHint")}
      >
        {({ titleId, hintId }) => (
          <Switch
            checked={settings?.autoSave ?? false}
            onChange={(next) => onUpdate({ autoSave: next })}
            labelledBy={titleId}
            describedBy={hintId}
            disabled={settings == null}
          />
        )}
      </SettingRow>

      <SettingRow
        title={t("settingsView.general.launchAtStartup")}
        hint={t("settingsView.general.launchAtStartupHint")}
      >
        {({ titleId, hintId }) => (
          <Switch
            checked={settings?.launchAtStartup ?? false}
            onChange={(next) => onUpdate({ launchAtStartup: next })}
            labelledBy={titleId}
            describedBy={hintId}
            disabled={settings == null}
          />
        )}
      </SettingRow>
    </div>
  );
}
