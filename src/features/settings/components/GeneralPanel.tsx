import { useTranslation } from "react-i18next";
import type { Settings } from "../types";

interface GeneralPanelProps {
  settings: Settings | null;
  /** Persists a partial settings update through the backend (Req 19.2). */
  onUpdate: (patch: Partial<Settings>) => void;
}

/**
 * General preferences panel (Req 19.1, 19.2). Theme and language live solely in
 * the Appearance section (flavor selects the light/dark base, and the locale
 * picker switches the UI language), so this panel exposes only the auto-save and
 * launch-at-startup preferences as direct settings updates.
 */
export function GeneralPanel({ settings, onUpdate }: GeneralPanelProps) {
  const { t } = useTranslation();

  const labelClass = "text-sm font-medium text-foreground";
  const hintClass = "text-xs text-muted-foreground";

  return (
    <div className="flex flex-col gap-6">
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
            className={`absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${
              settings?.autoSave ? "translate-x-5" : "translate-x-0"
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
            className={`absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform ${
              settings?.launchAtStartup ? "translate-x-5" : "translate-x-0"
            }`}
          />
        </button>
      </section>
    </div>
  );
}
