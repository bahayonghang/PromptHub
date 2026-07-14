import { create } from "zustand";
import { runtime, type RuntimeBridge } from "../runtime";

/**
 * The major navigable views in the application shell (Req 22.3). Each maps to a
 * sidebar navigation entry and a content region. The concrete view bodies are
 * implemented by tasks 22.2–22.5; this shell renders placeholders for them.
 */
export type AppView = "prompts" | "settings";

/** All views in display order, used by the navigation and for validation. */
export const APP_VIEWS: readonly AppView[] = ["prompts", "settings"];

/** The view shown when the application first opens. */
export const DEFAULT_VIEW: AppView = "prompts";

/** Mirror of the backend `AppStatus` payload from the `app_status` command. */
interface AppStatus {
  ready: boolean;
  initError?: string;
}

/** The fatal startup-failure event emitted by the backend (Req 4.7 / 23.3). */
const INIT_FAILED_EVENT = "app:init-failed";

interface AppState {
  /** The currently active major view (Req 22.3). */
  activeView: AppView;
  /** Switches the active view. */
  setActiveView: (view: AppView) => void;

  /** Whether the navigation sidebar is collapsed to its icon rail. */
  sidebarCollapsed: boolean;
  /** Toggles the sidebar between expanded and collapsed. */
  toggleSidebar: () => void;
  /** Sets the sidebar collapsed state explicitly. */
  setSidebarCollapsed: (collapsed: boolean) => void;

  /** Whether the backend reports itself ready to accept commands (Req 23.1). */
  ready: boolean;
  setReady: (ready: boolean) => void;

  /** Fatal startup error reported by the backend (Req 4.7 / 23.3), if any. */
  initError: string | null;
  setInitError: (initError: string | null) => void;

  /**
   * Bootstraps app-level state through the Runtime_Bridge (Req 3.1): subscribes
   * to the fatal init-failure event and reads the backend readiness status. All
   * backend access is routed through the bridge rather than `@tauri-apps/api`.
   * Resolves with an unsubscribe function for the event subscription.
   */
  initialize: (bridge?: RuntimeBridge) => Promise<() => void>;
}

export const useAppStore = create<AppState>((set) => ({
  activeView: DEFAULT_VIEW,
  setActiveView: (activeView) => set({ activeView }),

  sidebarCollapsed: false,
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),

  ready: false,
  setReady: (ready) => set({ ready }),

  initError: null,
  setInitError: (initError) => set({ initError }),

  initialize: async (bridge = runtime) => {
    // Subscribe first so a failure emitted after this point is never missed.
    const unsubscribe = bridge.on<string>(INIT_FAILED_EVENT, (message) => {
      set({ initError: message, ready: false });
    });

    // Poll readiness in case the failure was recorded before we subscribed.
    try {
      const status = await bridge.invoke<AppStatus>("app_status");
      set({ ready: status.ready });
      if (status.initError) {
        set({ initError: status.initError });
      }
    } catch {
      // `app_status` is best-effort; the event subscription is the primary path.
    }

    return unsubscribe;
  },
}));
