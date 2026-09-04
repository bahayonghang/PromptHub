import { describe, expect, it, vi } from "vitest";
import { createSettingsApi } from "./api";
import type { RuntimeBridge } from "../../runtime";
import type { ExportScope, S3Config, SettingsPatch, WebDavConfig } from "./types";

/** A bridge whose invoke records the command + args and echoes a value. */
function makeBridge(returnValue: unknown = null) {
  const invoke = vi.fn(async () => returnValue);
  const bridge: RuntimeBridge = {
    capabilities: () => ({
      appUpdate: true,
      dataRecovery: true,
      desktopWindowControls: true,
    }),
    invoke: invoke as RuntimeBridge["invoke"],
    on: vi.fn(() => () => {}),
  };
  return { bridge, invoke };
}

describe("createSettingsApi command contract (Req 3.1)", () => {
  it("routes settings get/update through the bridge (Req 19.1, 19.2)", async () => {
    const { bridge, invoke } = makeBridge({});
    const api = createSettingsApi(bridge);
    const patch: SettingsPatch = { theme: "light", language: "ja" };

    await api.getSettings();
    await api.updateSettings(patch);

    expect(invoke).toHaveBeenCalledWith("settings.get");
    expect(invoke).toHaveBeenCalledWith("settings.update", { patch });
  });

  it("routes system font discovery through the bridge", async () => {
    const { bridge, invoke } = makeBridge(["Inter", "Segoe UI"]);
    const api = createSettingsApi(bridge);

    await expect(api.listSystemFonts()).resolves.toEqual(["Inter", "Segoe UI"]);
    expect(invoke).toHaveBeenCalledWith("settings.list_system_fonts");
  });

  it("routes security commands through the bridge (Req 15)", async () => {
    const { bridge, invoke } = makeBridge(null);
    const api = createSettingsApi(bridge);

    await api.securityStatus();
    await api.setMasterPassword("password1");
    await api.changeMasterPassword("oldpassword", "newpassword");
    await api.unlock("password1");
    await api.lock();

    expect(invoke).toHaveBeenCalledWith("security.status");
    expect(invoke).toHaveBeenCalledWith("security.setMasterPassword", {
      password: "password1",
    });
    expect(invoke).toHaveBeenCalledWith("security.changeMasterPassword", {
      currentPassword: "oldpassword",
      newPassword: "newpassword",
    });
    expect(invoke).toHaveBeenCalledWith("security.unlock", { password: "password1" });
    expect(invoke).toHaveBeenCalledWith("security.lock");
  });

  it("routes data-path commands through the bridge (Req 19.3-19.10)", async () => {
    const { bridge, invoke } = makeBridge(null);
    const api = createSettingsApi(bridge);

    await api.getDataPath();
    await api.getDataStatus();
    await api.previewDataChange("/new/path");
    await api.applyDataChange("/new/path", "migrate", "tok-change");
    await api.recoveryScan();
    await api.recoveryPreview("/old/path");
    await api.recoveryApply("/old/path", "tok-recovery");

    expect(invoke).toHaveBeenCalledWith("data.getPath");
    expect(invoke).toHaveBeenCalledWith("data.getStatus");
    expect(invoke).toHaveBeenCalledWith("data.previewChange", { targetPath: "/new/path" });
    expect(invoke).toHaveBeenCalledWith("data.applyChange", {
      targetPath: "/new/path",
      action: "migrate",
      confirmToken: "tok-change",
    });
    expect(invoke).toHaveBeenCalledWith("data.recoveryScan");
    expect(invoke).toHaveBeenCalledWith("data.recoveryPreview", { sourcePath: "/old/path" });
    expect(invoke).toHaveBeenCalledWith("data.recoveryApply", {
      sourcePath: "/old/path",
      confirmToken: "tok-recovery",
    });
  });

  it("routes sync transport + backup commands through the bridge (Req 17)", async () => {
    const { bridge, invoke } = makeBridge(null);
    const api = createSettingsApi(bridge);
    const webdav: WebDavConfig = { url: "https://x", username: "u", password: "p" };
    const s3: S3Config = {
      endpoint: "https://s3",
      region: "us-east-1",
      bucket: "b",
      accessKeyId: "ak",
      secretAccessKey: "sk",
    };
    const scope: ExportScope = { data: true, media: false, rule: false };

    await api.testWebdav(webdav);
    await api.testS3(s3);
    await api.exportZip(scope);
    await api.exportCancel();
    await api.listBackups();
    await api.createBackup();
    await api.restoreBackup("b1");
    await api.deleteBackup("b1");

    expect(invoke).toHaveBeenCalledWith("webdav.test", { config: webdav });
    expect(invoke).toHaveBeenCalledWith("s3.test", { config: s3 });
    expect(invoke).toHaveBeenCalledWith("data.exportZip", { scope });
    expect(invoke).toHaveBeenCalledWith("data.exportCancel");
    expect(invoke).toHaveBeenCalledWith("backup.list");
    expect(invoke).toHaveBeenCalledWith("backup.create");
    expect(invoke).toHaveBeenCalledWith("backup.restore", { id: "b1" });
    expect(invoke).toHaveBeenCalledWith("backup.delete", { id: "b1" });
  });
});
