import { afterEach, describe, expect, it, vi } from "vitest";
import fc from "fast-check";
import {
  APP_VIEWS,
  DEFAULT_VIEW,
  useAppStore,
  type AppView,
} from "./appStore";
import type { RuntimeBridge } from "../runtime";

/** Resets the store to its initial state between tests. */
function resetStore() {
  useAppStore.setState({
    activeView: DEFAULT_VIEW,
    sidebarCollapsed: false,
    ready: false,
    initError: null,
  });
}

/** Builds a fake Runtime_Bridge with controllable invoke/event behavior. */
function makeBridge(
  overrides: Partial<{
    invoke: RuntimeBridge["invoke"];
    on: RuntimeBridge["on"];
  }> = {},
): RuntimeBridge {
  return {
    capabilities: () => ({
      appUpdate: true,
      dataRecovery: true,
      desktopWindowControls: true,
    }),
    invoke: overrides.invoke ?? (vi.fn(async () => ({ ready: true })) as RuntimeBridge["invoke"]),
    on: overrides.on ?? (vi.fn(() => () => {}) as RuntimeBridge["on"]),
  };
}

afterEach(() => {
  resetStore();
  vi.restoreAllMocks();
});

describe("app store defaults (Req 22.3)", () => {
  it("starts on the default view, expanded, and not ready", () => {
    const state = useAppStore.getState();
    expect(state.activeView).toBe(DEFAULT_VIEW);
    expect(DEFAULT_VIEW).toBe("prompts");
    expect(state.sidebarCollapsed).toBe(false);
    expect(state.ready).toBe(false);
    expect(state.initError).toBeNull();
  });
});

describe("setActiveView (Req 22.3)", () => {
  it("switches to any of the major views", () => {
    fc.assert(
      fc.property(fc.constantFrom<AppView>(...APP_VIEWS), (view) => {
        useAppStore.getState().setActiveView(view);
        expect(useAppStore.getState().activeView).toBe(view);
      }),
    );
  });
});

describe("sidebar state", () => {
  it("toggles between expanded and collapsed", () => {
    const { toggleSidebar } = useAppStore.getState();
    expect(useAppStore.getState().sidebarCollapsed).toBe(false);
    toggleSidebar();
    expect(useAppStore.getState().sidebarCollapsed).toBe(true);
    toggleSidebar();
    expect(useAppStore.getState().sidebarCollapsed).toBe(false);
  });

  it("sets the collapsed state explicitly", () => {
    useAppStore.getState().setSidebarCollapsed(true);
    expect(useAppStore.getState().sidebarCollapsed).toBe(true);
  });
});

describe("initialize wires backend access through the Runtime_Bridge (Req 3.1)", () => {
  it("subscribes to the init-failure event and reads readiness via the bridge", async () => {
    const invoke = vi.fn(async () => ({ ready: true }));
    const on = vi.fn(() => () => {});
    const bridge = makeBridge({
      invoke: invoke as RuntimeBridge["invoke"],
      on: on as RuntimeBridge["on"],
    });

    await useAppStore.getState().initialize(bridge);

    expect(on).toHaveBeenCalledWith("app:init-failed", expect.any(Function));
    expect(invoke).toHaveBeenCalledWith("app_status");
    expect(useAppStore.getState().ready).toBe(true);
    expect(useAppStore.getState().initError).toBeNull();
  });

  it("records the init error reported by the status payload", async () => {
    const bridge = makeBridge({
      invoke: vi.fn(async () => ({
        ready: false,
        initError: "data directory not writable",
      })) as RuntimeBridge["invoke"],
    });

    await useAppStore.getState().initialize(bridge);

    expect(useAppStore.getState().ready).toBe(false);
    expect(useAppStore.getState().initError).toBe("data directory not writable");
  });

  it("surfaces a fatal init failure delivered via the event channel (Req 23.3)", async () => {
    let emit: ((message: string) => void) | undefined;
    const bridge = makeBridge({
      on: ((_event: string, handler: (payload: string) => void) => {
        emit = handler;
        return () => {};
      }) as RuntimeBridge["on"],
    });

    await useAppStore.getState().initialize(bridge);
    emit?.("media directory not writable");

    expect(useAppStore.getState().initError).toBe("media directory not writable");
    expect(useAppStore.getState().ready).toBe(false);
  });

  it("tolerates an app_status failure without throwing", async () => {
    const unsubscribe = vi.fn();
    const bridge = makeBridge({
      invoke: vi.fn(async () => {
        throw new Error("backend unavailable");
      }) as RuntimeBridge["invoke"],
      on: (() => unsubscribe) as RuntimeBridge["on"],
    });

    const off = await useAppStore.getState().initialize(bridge);
    expect(useAppStore.getState().initError).toBeNull();

    off();
    expect(unsubscribe).toHaveBeenCalledOnce();
  });
});
