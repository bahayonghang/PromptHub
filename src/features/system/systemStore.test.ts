import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { downloadProgressPercent, useSystemStore } from "./systemStore";
import type { SystemApi } from "./api";
import type { RuntimeBridge } from "../../runtime";
import type { Shortcut, UpdateCheckResult } from "./types";

/** A controllable fake SystemApi. Each method is a vi mock with a default. */
function makeApi(overrides: Partial<SystemApi> = {}): SystemApi {
  return {
    minimizeWindow: vi.fn(async () => undefined),
    maximizeWindow: vi.fn(async () => undefined),
    restoreWindow: vi.fn(async () => undefined),
    closeWindow: vi.fn(async () => undefined),
    quitWindow: vi.fn(async () => undefined),
    hideWindow: vi.fn(async () => undefined),
    toggleVisibility: vi.fn(async () => undefined),
    enterFullscreen: vi.fn(async () => undefined),
    exitFullscreen: vi.fn(async () => undefined),
    toggleFullscreen: vi.fn(async () => undefined),
    setCloseAction: vi.fn(async () => undefined),
    setAutoLaunch: vi.fn(async () => undefined),
    registerShortcuts: vi.fn(async () => undefined),
    showNotification: vi.fn(async () => undefined),
    getCacheSize: vi.fn(async () => 0),
    clearCache: vi.fn(async () => 0),
    getRuntimePaths: vi.fn(async () => ({
      data: "/data",
      database: "/data/prompthub.db",
      media: "/data/media",
      rule: "/data/rule",
      backup: "/data/backup",
      log: "/data/log",
    })),
    openPath: vi.fn(async () => undefined),
    getVersion: vi.fn(async () => "1.0.0"),
    getPlatform: vi.fn(async () => "Windows"),
    checkUpdate: vi.fn(
      async (): Promise<UpdateCheckResult> => ({
        available: false,
        currentVersion: "1.0.0",
      }),
    ),
    downloadUpdate: vi.fn(async () => undefined),
    installUpdate: vi.fn(async () => undefined),
    ...overrides,
  };
}

function resetStore(api: SystemApi) {
  useSystemStore.setState({
    api,
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
  });
}

const SHORTCUT: Shortcut = {
  action: "toggle-window",
  accelerator: "CmdOrCtrl+Shift+K",
  mode: "global",
};

afterEach(() => vi.restoreAllMocks());

describe("system store (Req 3.1, 20, 24)", () => {
  beforeEach(() => resetStore(makeApi()));

  // --- window controls (Req 20.1) ---------------------------------------
  it("toggleMaximize() maximizes then restores, tracking the state (Req 20.1)", async () => {
    const api = makeApi();
    resetStore(api);

    await useSystemStore.getState().toggleMaximize();
    expect(api.maximizeWindow).toHaveBeenCalled();
    expect(useSystemStore.getState().isMaximized).toBe(true);

    await useSystemStore.getState().toggleMaximize();
    expect(api.restoreWindow).toHaveBeenCalled();
    expect(useSystemStore.getState().isMaximized).toBe(false);
  });

  it("window controls degrade gracefully when the capability is gated (Req 3.7)", async () => {
    resetStore(
      makeApi({
        minimizeWindow: vi.fn(async () => {
          throw { code: "CAPABILITY_UNAVAILABLE", message: "no window controls" };
        }),
      }),
    );

    await useSystemStore.getState().minimize();

    const state = useSystemStore.getState();
    expect(state.windowControlsUnavailable).toBe(true);
    expect(state.error).toBeNull();
  });

  // --- close action / auto-launch (Req 20.4, 20.5) ----------------------
  it("setCloseAction() persists and tracks the action (Req 20.4)", async () => {
    const api = makeApi();
    resetStore(api);

    const ok = await useSystemStore.getState().setCloseAction("exit");

    expect(ok).toBe(true);
    expect(api.setCloseAction).toHaveBeenCalledWith("exit");
    expect(useSystemStore.getState().closeAction).toBe("exit");
  });

  it("setAutoLaunch() persists and tracks the enabled state (Req 20.5)", async () => {
    const api = makeApi();
    resetStore(api);

    const ok = await useSystemStore.getState().setAutoLaunch(true);

    expect(ok).toBe(true);
    expect(api.setAutoLaunch).toHaveBeenCalledWith(true);
    expect(useSystemStore.getState().autoLaunch).toBe(true);
  });

  // --- close dialog flow (Req 20.4) -------------------------------------
  it("confirmClose() quits the process and dismisses the dialog (Req 20.4)", async () => {
    const api = makeApi();
    resetStore(api);
    useSystemStore.setState({ closeDialogOpen: true });

    await useSystemStore.getState().confirmClose();

    expect(api.quitWindow).toHaveBeenCalled();
    expect(api.closeWindow).not.toHaveBeenCalled();
    expect(useSystemStore.getState().closeDialogOpen).toBe(false);
  });

  it("hideToTray() hides via window.hide and dismisses the dialog (Req 20.4)", async () => {
    const api = makeApi();
    resetStore(api);
    useSystemStore.setState({ closeDialogOpen: true });

    await useSystemStore.getState().hideToTray();

    expect(api.hideWindow).toHaveBeenCalled();
    expect(api.toggleVisibility).not.toHaveBeenCalled();
    expect(useSystemStore.getState().closeDialogOpen).toBe(false);
  });

  it("setCloseActionLocal() seeds the close action without a backend call", () => {
    const api = makeApi();
    resetStore(api);

    useSystemStore.getState().setCloseActionLocal("minimize");

    expect(api.setCloseAction).not.toHaveBeenCalled();
    expect(useSystemStore.getState().closeAction).toBe("minimize");
  });

  // --- shortcuts (Req 20.6, 20.11) --------------------------------------
  it("registerShortcuts() adopts the set on success (Req 20.6)", async () => {
    const api = makeApi();
    resetStore(api);

    const ok = await useSystemStore.getState().registerShortcuts([SHORTCUT]);

    expect(ok).toBe(true);
    expect(api.registerShortcuts).toHaveBeenCalledWith([SHORTCUT]);
    expect(useSystemStore.getState().shortcuts).toEqual([SHORTCUT]);
  });

  it("registerShortcuts() leaves the prior set unchanged on conflict (Req 20.11)", async () => {
    resetStore(
      makeApi({
        registerShortcuts: vi.fn(async () => {
          throw { code: "CONFLICT", message: "shortcut conflicts with an existing one" };
        }),
      }),
    );
    // Seed an already-registered set.
    useSystemStore.setState({ shortcuts: [SHORTCUT] });

    const conflicting: Shortcut = {
      action: "other",
      accelerator: "CmdOrCtrl+Shift+K",
      mode: "local",
    };
    const ok = await useSystemStore.getState().registerShortcuts([SHORTCUT, conflicting]);

    expect(ok).toBe(false);
    // The previously registered set is preserved (Req 20.11).
    expect(useSystemStore.getState().shortcuts).toEqual([SHORTCUT]);
    expect(useSystemStore.getState().error).toContain("conflict");
  });

  // --- updater lifecycle (Req 24.2-24.7) --------------------------------
  it("checkUpdate() reports an available update (Req 24.2)", async () => {
    resetStore(
      makeApi({
        checkUpdate: vi.fn(async () => ({
          available: true,
          version: "1.2.0",
          currentVersion: "1.0.0",
        })),
      }),
    );

    await useSystemStore.getState().checkUpdate();

    const state = useSystemStore.getState();
    expect(state.updaterPhase).toBe("available");
    expect(state.updateCheck?.version).toBe("1.2.0");
  });

  it("checkUpdate() reports up-to-date when no update is available (Req 24.2)", async () => {
    resetStore(makeApi());

    await useSystemStore.getState().checkUpdate();

    expect(useSystemStore.getState().updaterPhase).toBe("upToDate");
  });

  it("checkUpdate() degrades gracefully when the updater is gated (Req 3.7)", async () => {
    resetStore(
      makeApi({
        checkUpdate: vi.fn(async () => {
          throw { code: "CAPABILITY_UNAVAILABLE", message: "no updater" };
        }),
      }),
    );

    await useSystemStore.getState().checkUpdate();

    const state = useSystemStore.getState();
    expect(state.updaterUnavailable).toBe(true);
    expect(state.updaterPhase).toBe("idle");
  });

  it("installUpdate() surfaces a signature failure and leaves the phase in error (Req 24.5, 24.7)", async () => {
    resetStore(
      makeApi({
        installUpdate: vi.fn(async () => {
          throw { code: "SIGNATURE", message: "signature verification failed" };
        }),
      }),
    );

    await useSystemStore.getState().installUpdate();

    const state = useSystemStore.getState();
    expect(state.updaterPhase).toBe("error");
    expect(state.updaterError).toContain("signature");
  });

  // --- notifications (Req 20.7, 20.13) ----------------------------------
  it("showNotification() surfaces a permission denial as an error (Req 20.13)", async () => {
    resetStore(
      makeApi({
        showNotification: vi.fn(async () => {
          throw { code: "UNAUTHORIZED", message: "notifications are not permitted" };
        }),
      }),
    );

    const ok = await useSystemStore.getState().showNotification("t", "b");

    expect(ok).toBe(false);
    expect(useSystemStore.getState().error).toContain("not permitted");
  });

  // --- runtime info + cache (Req 20.8, 20.9, 24.6) ----------------------
  it("loadInfo() loads version, platform, paths, and cache size (Req 20.9, 24.6)", async () => {
    resetStore(
      makeApi({
        getVersion: vi.fn(async () => "2.0.0"),
        getPlatform: vi.fn(async () => "macOS"),
        getCacheSize: vi.fn(async () => 2048),
      }),
    );

    await useSystemStore.getState().loadInfo();

    const state = useSystemStore.getState();
    expect(state.version).toBe("2.0.0");
    expect(state.platform).toBe("macOS");
    expect(state.runtimePaths?.database).toBe("/data/prompthub.db");
    expect(state.cacheSize).toBe(2048);
  });

  // --- event subscriptions (Req 20.2-20.4, 20.6, 24.3) ------------------
  it("initialize() subscribes to events and updates state from payloads", () => {
    const api = makeApi();
    resetStore(api);

    type Handler = (payload: unknown) => void;
    const handlers = new Map<string, Handler>();
    const bridge: RuntimeBridge = {
      capabilities: () => ({
        appUpdate: true,
        dataRecovery: true,
        desktopWindowControls: true,
      }),
      invoke: vi.fn() as RuntimeBridge["invoke"],
      on: ((event: string, handler: Handler) => {
        handlers.set(event, handler);
        return () => handlers.delete(event);
      }) as RuntimeBridge["on"],
    };

    const unsubscribe = useSystemStore.getState().initialize(bridge);

    handlers.get("window:fullscreen-changed")?.({ fullscreen: true });
    handlers.get("window:visibility-changed")?.({ visible: false });
    handlers.get("window:close-requested")?.({});
    handlers.get("shortcut:triggered")?.({ action: "toggle-window" });
    handlers.get("updater:status")?.({ phase: "downloading", downloaded: 50, total: 100 });

    let state = useSystemStore.getState();
    expect(state.isFullscreen).toBe(true);
    expect(state.isVisible).toBe(false);
    expect(state.closeDialogOpen).toBe(true);
    expect(state.lastTriggeredAction).toBe("toggle-window");
    expect(state.updaterPhase).toBe("downloading");
    expect(state.downloaded).toBe(50);
    expect(state.total).toBe(100);

    // The terminal `done` event marks the download finished (Req 24.3).
    handlers.get("updater:status")?.({ phase: "done", downloaded: 100, total: 100 });
    expect(useSystemStore.getState().updaterPhase).toBe("downloaded");

    unsubscribe();
    expect(handlers.size).toBe(0);
  });
});

describe("downloadProgressPercent (Req 24.3)", () => {
  it("returns null when the total is unknown or non-positive", () => {
    expect(downloadProgressPercent(50, null)).toBeNull();
    expect(downloadProgressPercent(null, 100)).toBeNull();
    expect(downloadProgressPercent(50, 0)).toBeNull();
  });

  it("computes a clamped 0-100 percentage", () => {
    expect(downloadProgressPercent(0, 100)).toBe(0);
    expect(downloadProgressPercent(50, 100)).toBe(50);
    expect(downloadProgressPercent(100, 100)).toBe(100);
    // Over-reporting is clamped to 100.
    expect(downloadProgressPercent(150, 100)).toBe(100);
  });
});
