import { describe, expect, it, vi } from "vitest";
import { createSystemApi } from "./api";
import type { RuntimeBridge } from "../../runtime";
import type { Shortcut } from "./types";

/** A bridge whose invoke records the command + args and echoes a value. */
function makeBridge(returnValue: unknown = null) {
  const invoke = vi.fn(async () => returnValue);
  const bridge: RuntimeBridge = {
    capabilities: () => ({
      appUpdate: true,
      dataRecovery: true,
      desktopWindowControls: true,
      skillDistribution: true,
      skillFileEditing: true,
      skillLocalScan: true,
      skillPlatformIntegration: true,
      skillStore: true,
    }),
    invoke: invoke as RuntimeBridge["invoke"],
    on: vi.fn(() => () => {}),
  };
  return { bridge, invoke };
}

describe("createSystemApi command contract (Req 3.1)", () => {
  it("routes window controls through the bridge (Req 20.1-20.4)", async () => {
    const { bridge, invoke } = makeBridge();
    const api = createSystemApi(bridge);

    await api.minimizeWindow();
    await api.maximizeWindow();
    await api.restoreWindow();
    await api.closeWindow();
    await api.toggleVisibility();
    await api.enterFullscreen();
    await api.exitFullscreen();
    await api.toggleFullscreen();
    await api.setCloseAction("minimize");

    expect(invoke).toHaveBeenCalledWith("window.minimize");
    expect(invoke).toHaveBeenCalledWith("window.maximize");
    expect(invoke).toHaveBeenCalledWith("window.restore");
    expect(invoke).toHaveBeenCalledWith("window.close");
    expect(invoke).toHaveBeenCalledWith("window.toggleVisibility");
    expect(invoke).toHaveBeenCalledWith("window.enterFullscreen");
    expect(invoke).toHaveBeenCalledWith("window.exitFullscreen");
    expect(invoke).toHaveBeenCalledWith("window.toggleFullscreen");
    expect(invoke).toHaveBeenCalledWith("window.setCloseAction", { action: "minimize" });
  });

  it("routes auto-launch, shortcuts, and notifications through the bridge (Req 20.5-20.7)", async () => {
    const { bridge, invoke } = makeBridge();
    const api = createSystemApi(bridge);
    const shortcuts: Shortcut[] = [
      { action: "toggle-window", accelerator: "CmdOrCtrl+Shift+K", mode: "global" },
    ];

    await api.setAutoLaunch(true);
    await api.registerShortcuts(shortcuts);
    await api.showNotification("Title", "Body");

    expect(invoke).toHaveBeenCalledWith("app.setAutoLaunch", { enabled: true });
    expect(invoke).toHaveBeenCalledWith("shortcut.register", { shortcuts });
    expect(invoke).toHaveBeenCalledWith("app.showNotification", {
      title: "Title",
      body: "Body",
    });
  });

  it("routes cache, runtime paths, and open-path through the bridge (Req 20.8-20.10)", async () => {
    const { bridge, invoke } = makeBridge();
    const api = createSystemApi(bridge);

    await api.getCacheSize();
    await api.clearCache();
    await api.getRuntimePaths();
    await api.openPath("/data/log");

    expect(invoke).toHaveBeenCalledWith("app.getCacheSize");
    expect(invoke).toHaveBeenCalledWith("app.clearCache");
    expect(invoke).toHaveBeenCalledWith("app.getRuntimePaths");
    expect(invoke).toHaveBeenCalledWith("app.openPath", { path: "/data/log" });
  });

  it("routes version, platform, and updater commands through the bridge (Req 24.2-24.6)", async () => {
    const { bridge, invoke } = makeBridge();
    const api = createSystemApi(bridge);

    await api.getVersion();
    await api.getPlatform();
    await api.checkUpdate();
    await api.downloadUpdate();
    await api.installUpdate();

    expect(invoke).toHaveBeenCalledWith("app.getVersion");
    expect(invoke).toHaveBeenCalledWith("app.getPlatform");
    expect(invoke).toHaveBeenCalledWith("updater.check");
    expect(invoke).toHaveBeenCalledWith("updater.download");
    expect(invoke).toHaveBeenCalledWith("updater.install");
  });
});
