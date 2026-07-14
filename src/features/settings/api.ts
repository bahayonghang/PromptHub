/**
 * Thin command wrappers for the settings view (Req 15, 17, 19). Every call is
 * routed through the Runtime_Bridge (Req 3.1); none touches `@tauri-apps/api`
 * directly. Command names follow the design's `domain.action` convention and
 * argument/field names use the camelCase DTO shapes the backend returns.
 *
 * The recovery family (`data.recovery*`) is capability-gated by the
 * Runtime_Bridge (`dataRecovery`); when unavailable the bridge rejects with a
 * `CAPABILITY_UNAVAILABLE` {@link BridgeError} without calling the backend
 * (Req 3.7), which the store surfaces as a normal error.
 */
import { runtime, type RuntimeBridge } from "../../runtime";
import type {
  ApplyResult,
  BackupEntry,
  ConnectionTestResult,
  DataPathAction,
  DataPathStatus,
  ExportResult,
  ExportScope,
  PreviewResult,
  RecoveryPreview,
  RecoverySource,
  RestoreResult,
  S3Config,
  SecurityStatus,
  Settings,
  SettingsPatch,
  WebDavConfig,
} from "./types";

/** The backend command surface the settings view depends on, grouped for injection. */
export interface SettingsApi {
  // Settings (Req 19.1, 19.2)
  getSettings(): Promise<Settings>;
  updateSettings(patch: SettingsPatch): Promise<Settings>;
  listSystemFonts(): Promise<string[]>;

  // Security (Req 15)
  securityStatus(): Promise<SecurityStatus>;
  setMasterPassword(password: string): Promise<void>;
  changeMasterPassword(currentPassword: string, newPassword: string): Promise<void>;
  unlock(password: string): Promise<void>;
  lock(): Promise<void>;

  // Data path (Req 19.3–19.10)
  getDataPath(): Promise<string>;
  getDataStatus(): Promise<DataPathStatus>;
  previewDataChange(targetPath: string): Promise<PreviewResult>;
  applyDataChange(targetPath: string, action: DataPathAction): Promise<ApplyResult>;
  recoveryScan(): Promise<RecoverySource[]>;
  recoveryPreview(sourcePath: string): Promise<RecoveryPreview>;
  recoveryApply(sourcePath: string): Promise<ApplyResult>;

  // Sync transports (Req 17.1, 17.3, 17.13)
  testWebdav(config: WebDavConfig): Promise<ConnectionTestResult>;
  testS3(config: S3Config): Promise<ConnectionTestResult>;

  // Export + backups (Req 17.5–17.8, 17.11, 17.12)
  exportZip(scope: ExportScope): Promise<ExportResult>;
  exportCancel(): Promise<void>;
  listBackups(): Promise<BackupEntry[]>;
  createBackup(): Promise<BackupEntry>;
  restoreBackup(id: string): Promise<RestoreResult>;
  deleteBackup(id: string): Promise<void>;
}

/**
 * Builds the {@link SettingsApi} bound to a Runtime_Bridge (the live `runtime`
 * by default). Tests inject a fake bridge to drive the view without a backend.
 */
export function createSettingsApi(bridge: RuntimeBridge = runtime): SettingsApi {
  return {
    getSettings: () => bridge.invoke<Settings>("settings.get"),
    updateSettings: (patch) => bridge.invoke<Settings>("settings.update", { patch }),
    listSystemFonts: () => bridge.invoke<string[]>("settings.list_system_fonts"),

    securityStatus: () => bridge.invoke<SecurityStatus>("security.status"),
    setMasterPassword: (password) =>
      bridge.invoke<void>("security.setMasterPassword", { password }),
    changeMasterPassword: (currentPassword, newPassword) =>
      bridge.invoke<void>("security.changeMasterPassword", {
        currentPassword,
        newPassword,
      }),
    unlock: (password) => bridge.invoke<void>("security.unlock", { password }),
    lock: () => bridge.invoke<void>("security.lock"),

    getDataPath: () => bridge.invoke<string>("data.getPath"),
    getDataStatus: () => bridge.invoke<DataPathStatus>("data.getStatus"),
    previewDataChange: (targetPath) =>
      bridge.invoke<PreviewResult>("data.previewChange", { targetPath }),
    applyDataChange: (targetPath, action) =>
      bridge.invoke<ApplyResult>("data.applyChange", { targetPath, action }),
    recoveryScan: () => bridge.invoke<RecoverySource[]>("data.recoveryScan"),
    recoveryPreview: (sourcePath) =>
      bridge.invoke<RecoveryPreview>("data.recoveryPreview", { sourcePath }),
    recoveryApply: (sourcePath) =>
      bridge.invoke<ApplyResult>("data.recoveryApply", { sourcePath }),

    testWebdav: (config) =>
      bridge.invoke<ConnectionTestResult>("webdav.test", { config }),
    testS3: (config) => bridge.invoke<ConnectionTestResult>("s3.test", { config }),

    exportZip: (scope) => bridge.invoke<ExportResult>("data.exportZip", { scope }),
    exportCancel: () => bridge.invoke<void>("data.exportCancel"),
    listBackups: () => bridge.invoke<BackupEntry[]>("backup.list"),
    createBackup: () => bridge.invoke<BackupEntry>("backup.create"),
    restoreBackup: (id) => bridge.invoke<RestoreResult>("backup.restore", { id }),
    deleteBackup: (id) => bridge.invoke<void>("backup.delete", { id }),
  };
}

/** The production settings API bound to the live Runtime_Bridge. */
export const settingsApi: SettingsApi = createSettingsApi();
