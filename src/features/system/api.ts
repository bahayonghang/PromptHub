/**
 * Thin command wrappers for the system view (Req 20, 24): window controls,
 * close-action, auto-launch, keyboard shortcuts, notifications, runtime paths /
 * cache, and the in-app updater. Every call is routed through the Runtime_Bridge
 * (Req 3.1); none touches `@tauri-apps/api` directly. Command names follow the
 * design's `domain.action` convention and argument/field names use the camelCase
 * DTO shapes the backend returns.
 *
 * The window-control family (`window.*`) is capability-gated by the
 * Runtime_Bridge (`desktopWindowControls`) and the updater family (`updater.*`)
 * by `appUpdate`; when unavailable the bridge rejects with a
 * `CAPABILITY_UNAVAILABLE` {@link BridgeError} without calling the backend
 * (Req 3.7), which the store surfaces gracefully.
 */
import { runtime, type RuntimeBridge } from "../../runtime";
import type {
  CloseAction,
  RuntimePathsReport,
  Shortcut,
  UpdateCheckResult,
} from "./types";

/** The backend command surface the system view depends on, grouped for injection. */
export interface SystemApi {
  // Window controls (Req 20.1-20.4) — gated by `desktopWindowControls`
  minimizeWindow(): Promise<void>;
  maximizeWindow(): Promise<void>;
  restoreWindow(): Promise<void>;
  closeWindow(): Promise<void>;
  toggleVisibility(): Promise<void>;
  enterFullscreen(): Promise<void>;
  exitFullscreen(): Promise<void>;
  toggleFullscreen(): Promise<void>;
  setCloseAction(action: CloseAction): Promise<void>;

  // Auto-launch (Req 20.5)
  setAutoLaunch(enabled: boolean): Promise<void>;

  // Keyboard shortcuts (Req 20.6, 20.11)
  registerShortcuts(shortcuts: Shortcut[]): Promise<void>;

  // Notifications (Req 20.7, 20.13)
  showNotification(title: string, body: string): Promise<void>;

  // Cache + runtime paths (Req 20.8, 20.9)
  getCacheSize(): Promise<number>;
  clearCache(): Promise<number>;
  getRuntimePaths(): Promise<RuntimePathsReport>;

  // Open path in system shell (Req 20.10, 20.12)
  openPath(path: string): Promise<void>;

  // Version + platform (Req 24.6)
  getVersion(): Promise<string>;
  getPlatform(): Promise<string>;

  // Updater (Req 24.2-24.5) — gated by `appUpdate`
  checkUpdate(): Promise<UpdateCheckResult>;
  downloadUpdate(): Promise<void>;
  installUpdate(): Promise<void>;
}

/**
 * Builds the {@link SystemApi} bound to a Runtime_Bridge (the live `runtime` by
 * default). Tests inject a fake bridge to drive the view without a backend.
 */
export function createSystemApi(bridge: RuntimeBridge = runtime): SystemApi {
  return {
    minimizeWindow: () => bridge.invoke<void>("window.minimize"),
    maximizeWindow: () => bridge.invoke<void>("window.maximize"),
    restoreWindow: () => bridge.invoke<void>("window.restore"),
    closeWindow: () => bridge.invoke<void>("window.close"),
    toggleVisibility: () => bridge.invoke<void>("window.toggleVisibility"),
    enterFullscreen: () => bridge.invoke<void>("window.enterFullscreen"),
    exitFullscreen: () => bridge.invoke<void>("window.exitFullscreen"),
    toggleFullscreen: () => bridge.invoke<void>("window.toggleFullscreen"),
    setCloseAction: (action) =>
      bridge.invoke<void>("window.setCloseAction", { action }),

    setAutoLaunch: (enabled) => bridge.invoke<void>("app.setAutoLaunch", { enabled }),

    registerShortcuts: (shortcuts) =>
      bridge.invoke<void>("shortcut.register", { shortcuts }),

    showNotification: (title, body) =>
      bridge.invoke<void>("app.showNotification", { title, body }),

    getCacheSize: () => bridge.invoke<number>("app.getCacheSize"),
    clearCache: () => bridge.invoke<number>("app.clearCache"),
    getRuntimePaths: () => bridge.invoke<RuntimePathsReport>("app.getRuntimePaths"),

    openPath: (path) => bridge.invoke<void>("app.openPath", { path }),

    getVersion: () => bridge.invoke<string>("app.getVersion"),
    getPlatform: () => bridge.invoke<string>("app.getPlatform"),

    checkUpdate: () => bridge.invoke<UpdateCheckResult>("updater.check"),
    downloadUpdate: () => bridge.invoke<void>("updater.download"),
    installUpdate: () => bridge.invoke<void>("updater.install"),
  };
}

/** The production system API bound to the live Runtime_Bridge. */
export const systemApi: SystemApi = createSystemApi();
