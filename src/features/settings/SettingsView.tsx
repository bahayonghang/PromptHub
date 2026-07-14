import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  DatabaseIcon,
  HardDriveIcon,
  type LucideIcon,
  MonitorCogIcon,
  PaletteIcon,
  RefreshCwIcon,
  ShieldIcon,
  SlidersHorizontalIcon,
} from "lucide-react";
import { useSettingsStore } from "./settingsStore";
import { AppearancePanel } from "./components/AppearancePanel";
import { GeneralPanel } from "./components/GeneralPanel";
import { SecurityPanel } from "./components/SecurityPanel";
import { DataPathPanel } from "./components/DataPathPanel";
import { SyncPanel } from "./components/SyncPanel";
import { SystemPanel } from "../system/components/SystemPanel";

/** The settings sections selectable from the left rail. */
type SettingsSection = "appearance" | "general" | "security" | "sync" | "dataPath" | "system";

interface SectionEntry {
  id: SettingsSection;
  labelKey: string;
  icon: LucideIcon;
}

/** The settings sections in display order (Req 22.3). */
const SECTIONS: readonly SectionEntry[] = [
  { id: "general", labelKey: "settingsView.sections.general", icon: SlidersHorizontalIcon },
  { id: "appearance", labelKey: "settingsView.sections.appearance", icon: PaletteIcon },
  { id: "security", labelKey: "settingsView.sections.security", icon: ShieldIcon },
  { id: "sync", labelKey: "settingsView.sections.sync", icon: DatabaseIcon },
  { id: "dataPath", labelKey: "settingsView.sections.dataPath", icon: HardDriveIcon },
  { id: "system", labelKey: "settingsView.sections.system", icon: MonitorCogIcon },
];

/**
 * The settings view (Req 22.3). A left rail switches between the general,
 * security, sync, and data-path sections; each renders a panel wired to the
 * settings store, which routes every backend call through the Runtime_Bridge
 * (Req 3.1). A persistent banner surfaces when an applied change (data-path
 * apply, master-password change, or backup restore) requires a restart to take
 * effect (Req 19.5, 19.7, 17.7).
 */
export function SettingsView() {
  const { t } = useTranslation();

  const settings = useSettingsStore((s) => s.settings);
  const securityStatus = useSettingsStore((s) => s.securityStatus);
  const dataStatus = useSettingsStore((s) => s.dataStatus);
  const preview = useSettingsStore((s) => s.preview);
  const recoverySources = useSettingsStore((s) => s.recoverySources);
  const recoveryUnavailable = useSettingsStore((s) => s.recoveryUnavailable);
  const backups = useSettingsStore((s) => s.backups);
  const restartRequired = useSettingsStore((s) => s.restartRequired);
  const error = useSettingsStore((s) => s.error);
  const preferenceStatus = useSettingsStore((s) => s.preferenceStatus);
  const preferenceErrors = useSettingsStore((s) => s.preferenceErrors);

  const load = useSettingsStore((s) => s.load);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const setPreference = useSettingsStore((s) => s.setPreference);
  const retryPreference = useSettingsStore((s) => s.retryPreference);
  const setMasterPassword = useSettingsStore((s) => s.setMasterPassword);
  const changeMasterPassword = useSettingsStore((s) => s.changeMasterPassword);
  const unlock = useSettingsStore((s) => s.unlock);
  const lock = useSettingsStore((s) => s.lock);
  const previewDataChange = useSettingsStore((s) => s.previewDataChange);
  const clearPreview = useSettingsStore((s) => s.clearPreview);
  const applyDataChange = useSettingsStore((s) => s.applyDataChange);
  const recoveryScan = useSettingsStore((s) => s.recoveryScan);
  const recoveryApply = useSettingsStore((s) => s.recoveryApply);
  const testWebdav = useSettingsStore((s) => s.testWebdav);
  const testS3 = useSettingsStore((s) => s.testS3);
  const exportZip = useSettingsStore((s) => s.exportZip);
  const createBackup = useSettingsStore((s) => s.createBackup);
  const restoreBackup = useSettingsStore((s) => s.restoreBackup);
  const deleteBackup = useSettingsStore((s) => s.deleteBackup);

  const [section, setSection] = useState<SettingsSection>("general");

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="flex h-full w-full">
      {/* Section rail */}
      <nav
        aria-label={t("settingsView.title")}
        className="flex w-56 shrink-0 flex-col gap-1 border-r border-border p-3 max-[900px]:w-16 max-[900px]:px-2"
      >
        <h2 className="px-2 pb-2 text-sm font-semibold text-foreground max-[900px]:sr-only">
          {t("settingsView.title")}
        </h2>
        {SECTIONS.map((entry) => {
          const Icon = entry.icon;
          const active = section === entry.id;
          return (
            <button
              key={entry.id}
              type="button"
              aria-label={t(entry.labelKey)}
              title={t(entry.labelKey)}
              aria-current={active ? "page" : undefined}
              onClick={() => setSection(entry.id)}
              className={`flex items-center gap-2 rounded-md px-3 py-2 text-left text-sm transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring max-[900px]:justify-center max-[900px]:px-2 ${
                active
                  ? "bg-primary/15 font-medium text-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-foreground"
              }`}
            >
              <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span className="max-[900px]:sr-only">{t(entry.labelKey)}</span>
            </button>
          );
        })}
      </nav>

      {/* Active section */}
      <section className="flex min-w-0 flex-1 flex-col">
        {restartRequired && (
          <div
            role="status"
            className="flex items-center gap-2 border-b border-primary/40 bg-primary/10 px-4 py-2 text-sm text-foreground"
          >
            <RefreshCwIcon className="h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
            {t("settingsView.restartRequired")}
          </div>
        )}
        {error && (
          <div
            role="alert"
            className="border-b border-destructive/40 bg-destructive/10 px-4 py-2 text-sm text-destructive"
          >
            {error}
          </div>
        )}

        <div className="min-h-0 flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-3xl p-4 sm:p-6">
          {section === "appearance" && <AppearancePanel />}
          {section === "general" && (
            <GeneralPanel
              settings={settings}
              onUpdate={(patch) => void updateSettings(patch)}
              onLanguageChange={(locale) => void setPreference("language", locale)}
              onRetryLanguage={() => void retryPreference("language")}
              languageStatus={preferenceStatus.language}
              languageError={preferenceErrors.language}
            />
          )}
          {section === "security" && (
            <SecurityPanel
              status={securityStatus}
              onSetMasterPassword={setMasterPassword}
              onChangeMasterPassword={changeMasterPassword}
              onUnlock={unlock}
              onLock={() => void lock()}
            />
          )}
          {section === "sync" && (
            <SyncPanel
              settings={settings}
              backups={backups}
              onTestWebdav={testWebdav}
              onTestS3={testS3}
              onExport={exportZip}
              onCreateBackup={() => void createBackup()}
              onRestoreBackup={(id) => {
                if (window.confirm(t("settingsView.sync.backup.restoreConfirm"))) {
                  void restoreBackup(id);
                }
              }}
              onDeleteBackup={(id) => {
                if (window.confirm(t("settingsView.sync.backup.deleteConfirm"))) {
                  void deleteBackup(id);
                }
              }}
            />
          )}
          {section === "dataPath" && (
            <DataPathPanel
              status={dataStatus}
              preview={preview}
              recoverySources={recoverySources}
              recoveryUnavailable={recoveryUnavailable}
              onPreview={(targetPath) => void previewDataChange(targetPath)}
              onClearPreview={clearPreview}
              onApply={(targetPath, action) => {
                if (window.confirm(t("settingsView.dataPath.applyConfirm"))) {
                  void applyDataChange(targetPath, action);
                }
              }}
              onRecoveryScan={() => void recoveryScan()}
              onRecoveryApply={(sourcePath) => {
                if (window.confirm(t("settingsView.dataPath.recoveryConfirm"))) {
                  void recoveryApply(sourcePath);
                }
              }}
            />
          )}
          {section === "system" && <SystemPanel launchAtStartup={settings?.launchAtStartup} />}
          </div>
        </div>
      </section>
    </div>
  );
}
