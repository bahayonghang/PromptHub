/**
 * View-state store for the settings view (Req 15, 17, 19). Holds the loaded
 * settings, the security/lock status, the active data-path status and change
 * preview, the recovery candidates, and the upgrade-backup list. All backend
 * access goes through an injectable {@link SettingsApi} (default: the live
 * bridge-bound API) so the store can be driven in tests without a backend
 * (Req 3.1).
 *
 * The recovery family is capability-gated (Req 3.7); the store surfaces a gate
 * rejection as `recoveryUnavailable` so the UI degrades gracefully rather than
 * crashing.
 *
 * Master-password values are passed straight to the backend and never stored in
 * this state or logged (security note in task 22.4).
 */
import { create } from "zustand";
import {
  applyAppearancePreferences,
  normalizeAppearancePreferences,
  normalizeInterfaceFontStack,
} from "../../appearance/preferences";
import i18n, {
  changeLocale as applyLocale,
  DEFAULT_LOCALE,
  isSupportedLocale,
  type LocaleGateway,
  type SupportedLocale,
} from "../../runtime/i18n";
import { settingsApi, type SettingsApi } from "./api";
import type {
  ApplyResult,
  BackupEntry,
  ConnectionTestResult,
  DataPathAction,
  DataPathStatus,
  ExportResult,
  ExportScope,
  PreviewResult,
  RecoverySource,
  S3Config,
  SecurityStatus,
  Settings,
  SettingsPatch,
  WebDavConfig,
} from "./types";

/** A `BridgeError`-shaped failure surfaced to the view (Req 3.5). */
function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    return String((err as { message: unknown }).message);
  }
  return String(err);
}

/** The error code carried by a `BridgeError`, or `null` when not present. */
function errorCode(err: unknown): string | null {
  if (err && typeof err === "object" && "code" in err) {
    return String((err as { code: unknown }).code);
  }
  return null;
}

export const PREFERENCE_KEYS = [
  "language",
  "theme",
  "themeFamily",
  "catppuccinDarkVariant",
  "accentColor",
  "interfaceFontStack",
  "fontScale",
  "density",
] as const;

export type PreferenceKey = (typeof PREFERENCE_KEYS)[number];
export type PreferenceValue = string | string[];
export type PreferenceSaveStatus = "idle" | "saving" | "saved" | "unsaved";
export type SystemFontsStatus = "idle" | "loading" | "ready" | "empty" | "error";

export interface PreferenceRuntime {
  currentLocale(): SupportedLocale;
  changeLocale(locale: SupportedLocale): Promise<void>;
  applyAppearance(settings: Settings, locale: SupportedLocale): void;
}

const sessionOnlyLocaleGateway: LocaleGateway = {
  loadPersistedLocale: async () => null,
  detectOsLocale: () => "",
  persistLocale: async () => {},
};

const defaultPreferenceRuntime: PreferenceRuntime = {
  currentLocale: () => (isSupportedLocale(i18n.language) ? i18n.language : DEFAULT_LOCALE),
  changeLocale: (locale) => applyLocale(locale, sessionOnlyLocaleGateway),
  applyAppearance: applyAppearancePreferences,
};

function preferencePatch(
  current: Settings,
  key: PreferenceKey,
  value: PreferenceValue,
): SettingsPatch {
  if (key === "language") return { language: String(value) };

  const migrated = normalizeAppearancePreferences(current);
  const normalizedValue =
    key === "interfaceFontStack" ? normalizeInterfaceFontStack(value) : value;
  return {
    themeFamily: current.themeFamily ?? migrated.themeFamily,
    catppuccinDarkVariant:
      current.catppuccinDarkVariant ?? migrated.catppuccinDarkVariant,
    interfaceFontStack: current.interfaceFontStack ?? migrated.interfaceFontStack,
    [key]: normalizedValue,
  };
}

let preferenceWriteTail: Promise<void> = Promise.resolve();

async function serializePreferenceWrite<T>(operation: () => Promise<T>): Promise<T> {
  const previous = preferenceWriteTail;
  let release!: () => void;
  preferenceWriteTail = new Promise<void>((resolve) => {
    release = resolve;
  });
  await previous;
  try {
    return await operation();
  } finally {
    release();
  }
}

function overlayPendingPreferences(
  canonical: Settings,
  completedKey: PreferenceKey,
  pending: Partial<Record<PreferenceKey, PreferenceValue>>,
  statuses: Partial<Record<PreferenceKey, PreferenceSaveStatus>>,
): Settings {
  let result = canonical;
  for (const key of PREFERENCE_KEYS) {
    const value = pending[key];
    if (
      key === completedKey ||
      value === undefined ||
      !["saving", "unsaved"].includes(statuses[key] ?? "idle")
    ) {
      continue;
    }
    result = { ...result, ...preferencePatch(result, key, value) };
  }
  return result;
}

interface SettingsStoreState {
  /** Backend command surface; injectable so tests can supply a fake. */
  api: SettingsApi;

  /** The loaded application settings (Req 19.1), or `null` before load. */
  settings: Settings | null;
  /** Master-password / lock status (Req 15.1), or `null` before load. */
  securityStatus: SecurityStatus | null;
  /** Active data-path status (Req 19.3), or `null` before load. */
  dataStatus: DataPathStatus | null;
  /** The latest data-path change preview (Req 19.4), or `null`. */
  preview: PreviewResult | null;
  /** Recovery candidates found by the last scan (Req 19.6). */
  recoverySources: RecoverySource[];
  /** True when the recovery capability is gated off in this runtime (Req 3.7). */
  recoveryUnavailable: boolean;
  /** Upgrade backups (Req 17.8), most-recent first as returned by the backend. */
  backups: BackupEntry[];
  /** Cached system font families for the appearance controls. */
  systemFonts: string[];
  systemFontsStatus: SystemFontsStatus;

  /** DOM/i18n preference application boundary; injectable for store tests. */
  preferenceRuntime: PreferenceRuntime;
  preferenceStatus: Partial<Record<PreferenceKey, PreferenceSaveStatus>>;
  preferenceErrors: Partial<Record<PreferenceKey, string>>;
  pendingPreferences: Partial<Record<PreferenceKey, PreferenceValue>>;

  /**
   * True once an operation that needs a restart to take effect has been applied
   * (data-path apply, master-password change, or backup restore). Surfaced to
   * the user as a restart indicator (Req 19.5, 19.7-restart, 17.7).
   */
  restartRequired: boolean;

  loading: boolean;
  error: string | null;

  /** Loads settings, security status, data status, and backups (Req 15, 17, 19). */
  load: () => Promise<void>;
  /** Seeds the settings loaded by the awaited application bootstrap. */
  hydrateSettings: (settings: Settings) => void;
  loadSystemFonts: () => Promise<void>;
  retrySystemFonts: () => Promise<void>;

  // General preferences ----------------------------------------------------
  /** Persists a partial settings update and stores the result (Req 19.2). */
  updateSettings: (patch: SettingsPatch) => Promise<Settings | null>;
  /**
   * Merges fields into the in-memory settings *without* a backend write. Used
   * after the theme/i18n modules have already persisted a change via
   * `settings.update`, to keep this store's copy in sync without a duplicate
   * write.
   */
  mergeLocalSettings: (patch: SettingsPatch) => void;
  setPreference: (
    key: PreferenceKey,
    value: PreferenceValue,
  ) => Promise<boolean>;
  retryPreference: (key: PreferenceKey) => Promise<boolean>;

  // Security (Req 15) -------------------------------------------------------
  refreshSecurityStatus: () => Promise<void>;
  /** Sets the initial master password and refreshes status (Req 15.2). */
  setMasterPassword: (password: string) => Promise<boolean>;
  /** Re-keys to a new master password; reports restart-required (Req 15.4). */
  changeMasterPassword: (
    currentPassword: string,
    newPassword: string,
  ) => Promise<boolean>;
  /** Unlocks the app and refreshes status (Req 15.6). */
  unlock: (password: string) => Promise<boolean>;
  /** Locks the app and refreshes status (Req 15.8). */
  lock: () => Promise<void>;

  // Data path (Req 19.3-19.10) ---------------------------------------------
  refreshDataStatus: () => Promise<void>;
  /** Previews a prospective data-path change, read-only (Req 19.4). */
  previewDataChange: (targetPath: string) => Promise<PreviewResult | null>;
  /** Clears the active preview without touching the backend. */
  clearPreview: () => void;
  /** Applies a data-path change; reports restart-required (Req 19.5). */
  applyDataChange: (
    targetPath: string,
    action: DataPathAction,
  ) => Promise<ApplyResult | null>;
  /** Scans for recoverable data sources (Req 19.6). */
  recoveryScan: () => Promise<void>;
  /** Recovers from a candidate source; reports restart-required (Req 19.8). */
  recoveryApply: (sourcePath: string) => Promise<ApplyResult | null>;

  // Sync transports (Req 17.1, 17.3) ---------------------------------------
  /** Tests a WebDAV connection, returning the explicit pass/fail (Req 17.1). */
  testWebdav: (config: WebDavConfig) => Promise<ConnectionTestResult | null>;
  /** Tests an S3 connection, returning the explicit pass/fail (Req 17.3). */
  testS3: (config: S3Config) => Promise<ConnectionTestResult | null>;

  // Export + backups (Req 17.5-17.8, 17.11, 17.12) -------------------------
  /** Exports the selected scope to a ZIP and returns the result (Req 17.5). */
  exportZip: (scope: ExportScope) => Promise<ExportResult | null>;
  /** Requests cancellation of an in-progress export (Req 17.11). */
  exportCancel: () => Promise<void>;
  refreshBackups: () => Promise<void>;
  /** Creates an upgrade backup and refreshes the list (Req 17.6). */
  createBackup: () => Promise<BackupEntry | null>;
  /** Restores a backup; reports restart-required (Req 17.7). */
  restoreBackup: (id: string) => Promise<boolean>;
  /** Deletes a backup and refreshes the list (Req 17.8). */
  deleteBackup: (id: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsStoreState>((set, get) => ({
  api: settingsApi,

  settings: null,
  securityStatus: null,
  dataStatus: null,
  preview: null,
  recoverySources: [],
  recoveryUnavailable: false,
  backups: [],
  systemFonts: [],
  systemFontsStatus: "idle",
  preferenceRuntime: defaultPreferenceRuntime,
  preferenceStatus: {},
  preferenceErrors: {},
  pendingPreferences: {},
  restartRequired: false,
  loading: false,
  error: null,

  hydrateSettings: (settings) => set({ settings }),

  load: async () => {
    const { api } = get();
    set({ loading: true, error: null });
    try {
      const [settings, securityStatus, dataStatus] = await Promise.all([
        get().settings == null ? api.getSettings() : Promise.resolve(get().settings as Settings),
        api.securityStatus(),
        api.getDataStatus(),
      ]);
      set({
        settings,
        securityStatus,
        dataStatus,
        // A configured-but-not-yet-active path means a restart is pending.
        restartRequired: dataStatus.restartRequired,
        loading: false,
      });
    } catch (err) {
      set({ error: errorMessage(err), loading: false });
    }
    // Backups load best-effort so a failure there never blocks the panels.
    await get().refreshBackups();
  },

  loadSystemFonts: async () => {
    const { api, systemFontsStatus } = get();
    if (["loading", "ready", "empty"].includes(systemFontsStatus)) return;
    set({ systemFontsStatus: "loading" });
    try {
      const systemFonts = await api.listSystemFonts();
      set({
        systemFonts,
        systemFontsStatus: systemFonts.length > 0 ? "ready" : "empty",
      });
    } catch {
      set({ systemFonts: [], systemFontsStatus: "error" });
    }
  },

  retrySystemFonts: async () => {
    set({ systemFontsStatus: "idle" });
    await get().loadSystemFonts();
  },

  updateSettings: async (patch) => {
    const { api } = get();
    set({ error: null });
    try {
      const settings = await api.updateSettings(patch);
      set({ settings });
      return settings;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  mergeLocalSettings: (patch) => {
    const current = get().settings;
    if (current == null) return;
    set({ settings: { ...current, ...patch } });
  },

  setPreference: async (key, value) => {
    const { api, preferenceRuntime } = get();
    const current = get().settings;
    if (current == null) return false;

    const patch = preferencePatch(current, key, value);
    const preview = { ...current, ...patch };
    const locale =
      key === "language" && isSupportedLocale(value)
        ? value
        : preferenceRuntime.currentLocale();
    set((state) => ({
      settings: preview,
      error: null,
      preferenceStatus: { ...state.preferenceStatus, [key]: "saving" },
      preferenceErrors: { ...state.preferenceErrors, [key]: undefined },
      pendingPreferences: { ...state.pendingPreferences, [key]: value },
    }));

    try {
      if (key === "language") await preferenceRuntime.changeLocale(locale);
      preferenceRuntime.applyAppearance(preview, locale);
      const canonical = await serializePreferenceWrite(() => api.updateSettings(patch));
      const state = get();
      const settings = overlayPendingPreferences(
        canonical,
        key,
        state.pendingPreferences,
        state.preferenceStatus,
      );
      const canonicalLocale = isSupportedLocale(settings.language)
        ? settings.language
        : locale;
      if (canonicalLocale !== preferenceRuntime.currentLocale()) {
        await preferenceRuntime.changeLocale(canonicalLocale);
      }
      preferenceRuntime.applyAppearance(settings, canonicalLocale);
      set((state) => ({
        settings,
        preferenceStatus: { ...state.preferenceStatus, [key]: "saved" },
        preferenceErrors: { ...state.preferenceErrors, [key]: undefined },
        pendingPreferences: { ...state.pendingPreferences, [key]: undefined },
      }));
      return true;
    } catch {
      set((state) => ({
        preferenceStatus: { ...state.preferenceStatus, [key]: "unsaved" },
        preferenceErrors: {
          ...state.preferenceErrors,
          [key]: "settingsView.preferences.unsaved",
        },
      }));
      return false;
    }
  },

  retryPreference: async (key) => {
    const value = get().pendingPreferences[key];
    return value === undefined ? false : get().setPreference(key, value);
  },

  refreshSecurityStatus: async () => {
    const { api } = get();
    try {
      set({ securityStatus: await api.securityStatus() });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  setMasterPassword: async (password) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.setMasterPassword(password);
      await get().refreshSecurityStatus();
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  changeMasterPassword: async (currentPassword, newPassword) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.changeMasterPassword(currentPassword, newPassword);
      await get().refreshSecurityStatus();
      set({ restartRequired: true });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  unlock: async (password) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.unlock(password);
      await get().refreshSecurityStatus();
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  lock: async () => {
    const { api } = get();
    set({ error: null });
    try {
      await api.lock();
      await get().refreshSecurityStatus();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  refreshDataStatus: async () => {
    const { api } = get();
    try {
      const dataStatus = await api.getDataStatus();
      set({ dataStatus, restartRequired: dataStatus.restartRequired });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  previewDataChange: async (targetPath) => {
    const { api } = get();
    set({ error: null });
    try {
      const preview = await api.previewDataChange(targetPath);
      set({ preview });
      return preview;
    } catch (err) {
      set({ error: errorMessage(err), preview: null });
      return null;
    }
  },

  clearPreview: () => set({ preview: null }),

  applyDataChange: async (targetPath, action) => {
    const { api, preview } = get();
    set({ error: null });
    try {
      const confirmToken =
        preview && preview.targetPath === targetPath ? preview.confirmToken : "";
      const result = await api.applyDataChange(targetPath, action, confirmToken);
      set({ preview: null });
      // Refresh the status first, then assert restart-required from the apply
      // result so the authoritative apply signal is not clobbered (Req 19.5).
      await get().refreshDataStatus();
      set({ restartRequired: result.restartRequired });
      return result;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  recoveryScan: async () => {
    const { api } = get();
    set({ error: null });
    try {
      const recoverySources = await api.recoveryScan();
      set({ recoverySources, recoveryUnavailable: false });
    } catch (err) {
      if (errorCode(err) === "CAPABILITY_UNAVAILABLE") {
        set({ recoverySources: [], recoveryUnavailable: true });
      } else {
        set({ error: errorMessage(err) });
      }
    }
  },

  recoveryApply: async (sourcePath) => {
    const { api } = get();
    set({ error: null });
    try {
      const preview = await api.recoveryPreview(sourcePath);
      const result = await api.recoveryApply(sourcePath, preview.confirmToken);
      // Refresh the status first, then assert restart-required from the apply
      // result so the authoritative recovery signal is not clobbered (Req 19.8).
      await get().refreshDataStatus();
      set({ restartRequired: result.restartRequired });
      return result;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  testWebdav: async (config) => {
    const { api } = get();
    set({ error: null });
    try {
      return await api.testWebdav(config);
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  testS3: async (config) => {
    const { api } = get();
    set({ error: null });
    try {
      return await api.testS3(config);
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  exportZip: async (scope) => {
    const { api } = get();
    set({ error: null });
    try {
      return await api.exportZip(scope);
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  exportCancel: async () => {
    const { api } = get();
    try {
      await api.exportCancel();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  refreshBackups: async () => {
    const { api } = get();
    try {
      set({ backups: await api.listBackups() });
    } catch {
      // Backups are best-effort; a failure here must not block other panels.
      set({ backups: [] });
    }
  },

  createBackup: async () => {
    const { api } = get();
    set({ error: null });
    try {
      const entry = await api.createBackup();
      await get().refreshBackups();
      return entry;
    } catch (err) {
      set({ error: errorMessage(err) });
      return null;
    }
  },

  restoreBackup: async (id) => {
    const { api } = get();
    set({ error: null });
    try {
      const result = await api.restoreBackup(id);
      set({ restartRequired: result.restartRequired });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  deleteBackup: async (id) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.deleteBackup(id);
      await get().refreshBackups();
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },
}));
