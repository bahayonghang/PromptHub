/**
 * View-state store for the system view (Req 20, 24). Owns the live window state
 * (maximized / fullscreen / visible, tracked from Window_Manager events), the
 * close-action and auto-launch settings, the registered keyboard shortcuts, the
 * updater lifecycle (check / download / install with byte progress), and the
 * runtime-paths / cache report. All backend access goes through an injectable
 * {@link SystemApi} (default: the live bridge-bound API) and all events through
 * an injectable {@link RuntimeBridge}, so the store can be driven in tests
 * without a backend or a live webview (Req 3.1).
 *
 * The window-control family is capability-gated by `desktopWindowControls` and
 * the updater family by `appUpdate` (Req 3.7); a gate rejection is surfaced as
 * the matching `*Unavailable` flag so the UI degrades gracefully rather than
 * crashing.
 */
import { create } from "zustand";
import { runtime, type RuntimeBridge } from "../../runtime";
import { systemApi, type SystemApi } from "./api";
import type {
  CloseAction,
  FullscreenChanged,
  RuntimePathsReport,
  Shortcut,
  ShortcutTriggered,
  UpdateCheckResult,
  UpdaterPhase,
  UpdaterStatus,
  VisibilityChanged,
} from "./types";

/** Tauri event names emitted by the Window_Manager and Updater (design event table). */
const EVENT_FULLSCREEN_CHANGED = "window:fullscreen-changed";
const EVENT_VISIBILITY_CHANGED = "window:visibility-changed";
const EVENT_CLOSE_REQUESTED = "window:close-requested";
const EVENT_SHORTCUT_TRIGGERED = "shortcut:triggered";
const EVENT_UPDATER_STATUS = "updater:status";

/** The backend `updater:status` phase that marks a finished download. */
const UPDATER_PHASE_DONE = "done";

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

interface SystemStoreState {
  /** Backend command surface; injectable so tests can supply a fake. */
  api: SystemApi;

  // --- Window state (Req 20.1-20.3) -------------------------------------
  /** Whether the window is currently maximized (tracked across toggles). */
  isMaximized: boolean;
  /** Whether the window is fullscreen, mirrored from the fullscreen event. */
  isFullscreen: boolean;
  /** Whether the window is visible, mirrored from the visibility event. */
  isVisible: boolean;
  /** True when an `ask`-action close was requested and awaits a decision (Req 20.4). */
  closeDialogOpen: boolean;

  // --- Settings wired to the Window_Manager (Req 20.4, 20.5) ------------
  /** The active close action; defaults to `ask` until set (Req 20.4). */
  closeAction: CloseAction;
  /** Whether launch-on-login is enabled (Req 20.5). */
  autoLaunch: boolean;

  // --- Keyboard shortcuts (Req 20.6, 20.11) -----------------------------
  /** The currently registered shortcuts (the last set the backend accepted). */
  shortcuts: Shortcut[];
  /** The action id of the most recently triggered shortcut, or `null`. */
  lastTriggeredAction: string | null;

  // --- Updater (Req 24.2-24.7) ------------------------------------------
  /** The updater UI lifecycle phase. */
  updaterPhase: UpdaterPhase;
  /** The latest update-check result, or `null` before a check. */
  updateCheck: UpdateCheckResult | null;
  /** Bytes downloaded so far during a download (Req 24.3). */
  downloaded: number | null;
  /** Total bytes to download, when known (Req 24.3). */
  total: number | null;
  /** The structured updater error message, or `null` (Req 24.7). */
  updaterError: string | null;

  // --- Runtime info (Req 20.8, 20.9, 24.6) ------------------------------
  /** The resolved runtime paths, or `null` before load (Req 20.9). */
  runtimePaths: RuntimePathsReport | null;
  /** The cache size in bytes, or `null` before load (Req 20.8). */
  cacheSize: number | null;
  /** The running application version, or `null` before load (Req 24.6). */
  version: string | null;
  /** The platform identifier (Windows|macOS|Linux), or `null` (Req 24.6). */
  platform: string | null;

  // --- Capability gating (Req 3.7) --------------------------------------
  /** True when window controls are gated off in this runtime. */
  windowControlsUnavailable: boolean;
  /** True when the updater is gated off in this runtime. */
  updaterUnavailable: boolean;

  /** A general error surfaced to the view (Req 3.5). */
  error: string | null;

  /**
   * Subscribes to Window_Manager and Updater events through the Runtime_Bridge
   * (Req 3.1) and returns an unsubscribe function detaching them all (Req 3.6).
   */
  initialize: (bridge?: RuntimeBridge) => () => void;

  // Window controls (Req 20.1-20.3) -------------------------------------
  minimize: () => Promise<void>;
  /** Maximizes or restores the window based on the tracked state (Req 20.1). */
  toggleMaximize: () => Promise<void>;
  close: () => Promise<void>;
  toggleFullscreen: () => Promise<void>;
  toggleVisibility: () => Promise<void>;

  // Close action / auto-launch (Req 20.4, 20.5) -------------------------
  /** Sets the close action through the Window_Manager and tracks it (Req 20.4). */
  setCloseAction: (action: CloseAction) => Promise<boolean>;
  /** Sets launch-on-login through the Window_Manager and tracks it (Req 20.5). */
  setAutoLaunch: (enabled: boolean) => Promise<boolean>;
  /** Seeds the auto-launch toggle from persisted settings without a backend call. */
  setAutoLaunchLocal: (enabled: boolean) => void;
  /** Dismisses the ask-close dialog without exiting (Req 20.4). */
  dismissCloseDialog: () => void;
  /** Confirms an ask-close by terminating the window (Req 20.4). */
  confirmClose: () => Promise<void>;

  // Shortcuts (Req 20.6, 20.11) -----------------------------------------
  /**
   * Registers a full shortcut set. On success the set becomes the active
   * shortcuts; on a conflict the backend rejects and the prior set is left
   * unchanged (Req 20.11), surfaced via `error`.
   */
  registerShortcuts: (shortcuts: Shortcut[]) => Promise<boolean>;

  // Updater (Req 24.2-24.5) ---------------------------------------------
  checkUpdate: () => Promise<void>;
  downloadUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;

  // Notifications (Req 20.7, 20.13) -------------------------------------
  showNotification: (title: string, body: string) => Promise<boolean>;

  // Runtime info + cache (Req 20.8, 20.9, 24.6) -------------------------
  loadInfo: () => Promise<void>;
  clearCache: () => Promise<void>;
  openPath: (path: string) => Promise<void>;
}

export const useSystemStore = create<SystemStoreState>((set, get) => ({
  api: systemApi,

  isMaximized: false,
  isFullscreen: false,
  isVisible: true,
  closeDialogOpen: false,

  closeAction: "ask",
  autoLaunch: false,

  shortcuts: [],
  lastTriggeredAction: null,

  updaterPhase: "idle",
  updateCheck: null,
  downloaded: null,
  total: null,
  updaterError: null,

  runtimePaths: null,
  cacheSize: null,
  version: null,
  platform: null,

  windowControlsUnavailable: false,
  updaterUnavailable: false,

  error: null,

  initialize: (bridge = runtime) => {
    const unsubscribers = [
      bridge.on<FullscreenChanged>(EVENT_FULLSCREEN_CHANGED, (payload) => {
        set({ isFullscreen: payload.fullscreen });
      }),
      bridge.on<VisibilityChanged>(EVENT_VISIBILITY_CHANGED, (payload) => {
        set({ isVisible: payload.visible });
      }),
      bridge.on<unknown>(EVENT_CLOSE_REQUESTED, () => {
        // The `ask` close action emits this; surface the decision dialog (Req 20.4).
        set({ closeDialogOpen: true });
      }),
      bridge.on<ShortcutTriggered>(EVENT_SHORTCUT_TRIGGERED, (payload) => {
        set({ lastTriggeredAction: payload.action });
      }),
      bridge.on<UpdaterStatus>(EVENT_UPDATER_STATUS, (payload) => {
        set({
          downloaded: payload.downloaded ?? null,
          total: payload.total ?? null,
          // The terminal `done` event marks the download finished (Req 24.3).
          updaterPhase:
            payload.phase === UPDATER_PHASE_DONE ? "downloaded" : "downloading",
        });
      }),
    ];

    return () => {
      for (const unsubscribe of unsubscribers) unsubscribe();
    };
  },

  minimize: async () => {
    const { api } = get();
    try {
      await api.minimizeWindow();
    } catch (err) {
      handleWindowError(set, err);
    }
  },

  toggleMaximize: async () => {
    const { api, isMaximized } = get();
    try {
      if (isMaximized) {
        await api.restoreWindow();
        set({ isMaximized: false });
      } else {
        await api.maximizeWindow();
        set({ isMaximized: true });
      }
    } catch (err) {
      handleWindowError(set, err);
    }
  },

  close: async () => {
    const { api } = get();
    try {
      await api.closeWindow();
    } catch (err) {
      handleWindowError(set, err);
    }
  },

  toggleFullscreen: async () => {
    const { api } = get();
    try {
      await api.toggleFullscreen();
    } catch (err) {
      handleWindowError(set, err);
    }
  },

  toggleVisibility: async () => {
    const { api } = get();
    try {
      await api.toggleVisibility();
    } catch (err) {
      handleWindowError(set, err);
    }
  },

  setCloseAction: async (action) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.setCloseAction(action);
      set({ closeAction: action });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  setAutoLaunch: async (enabled) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.setAutoLaunch(enabled);
      set({ autoLaunch: enabled });
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  setAutoLaunchLocal: (enabled) => set({ autoLaunch: enabled }),

  dismissCloseDialog: () => set({ closeDialogOpen: false }),

  confirmClose: async () => {
    set({ closeDialogOpen: false });
    await get().close();
  },

  registerShortcuts: async (shortcuts) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.registerShortcuts(shortcuts);
      // Only adopt the new set once the backend accepted it (Req 20.11).
      set({ shortcuts });
      return true;
    } catch (err) {
      // A conflict leaves the previously registered set unchanged (Req 20.11).
      set({ error: errorMessage(err) });
      return false;
    }
  },

  checkUpdate: async () => {
    const { api } = get();
    set({ updaterPhase: "checking", updaterError: null });
    try {
      const result = await api.checkUpdate();
      set({
        updateCheck: result,
        updaterPhase: result.available ? "available" : "upToDate",
      });
    } catch (err) {
      if (errorCode(err) === "CAPABILITY_UNAVAILABLE") {
        set({ updaterUnavailable: true, updaterPhase: "idle" });
      } else {
        set({ updaterPhase: "error", updaterError: errorMessage(err) });
      }
    }
  },

  downloadUpdate: async () => {
    const { api } = get();
    set({
      updaterPhase: "downloading",
      updaterError: null,
      downloaded: null,
      total: null,
    });
    try {
      await api.downloadUpdate();
      // Progress + the terminal `done` arrive via `updater:status`; ensure the
      // phase reflects completion even if the event was missed.
      if (get().updaterPhase === "downloading") {
        set({ updaterPhase: "downloaded" });
      }
    } catch (err) {
      set({ updaterPhase: "error", updaterError: errorMessage(err) });
    }
  },

  installUpdate: async () => {
    const { api } = get();
    set({ updaterPhase: "installing", updaterError: null });
    try {
      await api.installUpdate();
      // On success the plugin applies the update on the next restart (Req 24.4).
    } catch (err) {
      // A signature failure leaves the installed version intact (Req 24.5, 24.7).
      set({ updaterPhase: "error", updaterError: errorMessage(err) });
    }
  },

  showNotification: async (title, body) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.showNotification(title, body);
      return true;
    } catch (err) {
      set({ error: errorMessage(err) });
      return false;
    }
  },

  loadInfo: async () => {
    const { api } = get();
    try {
      const [version, platform] = await Promise.all([
        api.getVersion(),
        api.getPlatform(),
      ]);
      set({ version, platform });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
    // Runtime paths + cache size are window-gated; load best-effort (Req 3.7).
    try {
      const [runtimePaths, cacheSize] = await Promise.all([
        api.getRuntimePaths(),
        api.getCacheSize(),
      ]);
      set({ runtimePaths, cacheSize, windowControlsUnavailable: false });
    } catch (err) {
      if (errorCode(err) === "CAPABILITY_UNAVAILABLE") {
        set({ windowControlsUnavailable: true });
      } else {
        set({ error: errorMessage(err) });
      }
    }
  },

  clearCache: async () => {
    const { api } = get();
    set({ error: null });
    try {
      await api.clearCache();
      set({ cacheSize: await api.getCacheSize() });
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },

  openPath: async (path) => {
    const { api } = get();
    set({ error: null });
    try {
      await api.openPath(path);
    } catch (err) {
      set({ error: errorMessage(err) });
    }
  },
}));

/**
 * Classifies a window-control failure: a capability gate flips the
 * `windowControlsUnavailable` flag so the title bar can hide its controls
 * (Req 3.7); anything else surfaces as a normal error.
 */
function handleWindowError(
  set: (partial: Partial<SystemStoreState>) => void,
  err: unknown,
): void {
  if (errorCode(err) === "CAPABILITY_UNAVAILABLE") {
    set({ windowControlsUnavailable: true });
  } else {
    set({ error: errorMessage(err) });
  }
}

/** Returns the download progress as a 0-100 percentage, or `null` when unknown. */
export function downloadProgressPercent(
  downloaded: number | null,
  total: number | null,
): number | null {
  if (downloaded == null || total == null || total <= 0) return null;
  const ratio = downloaded / total;
  const clamped = ratio < 0 ? 0 : ratio > 1 ? 1 : ratio;
  return Math.round(clamped * 100);
}
