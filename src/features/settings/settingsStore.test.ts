import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSettingsStore } from "./settingsStore";
import type { SettingsApi } from "./api";
import type {
  ApplyResult,
  BackupEntry,
  DataPathStatus,
  PreviewResult,
  RecoverySource,
  SecurityStatus,
  Settings,
} from "./types";

function makeSettings(partial: Partial<Settings> = {}): Settings {
  return {
    theme: "dark",
    language: "en",
    autoSave: true,
    ...partial,
  };
}

function makeStatus(partial: Partial<SecurityStatus> = {}): SecurityStatus {
  return { hasMasterPassword: false, isLocked: true, ...partial };
}

function makeDataStatus(partial: Partial<DataPathStatus> = {}): DataPathStatus {
  return { activePath: "/data", restartRequired: false, ...partial };
}

/** A controllable fake SettingsApi. Each method is a vi mock with a default. */
function makeApi(overrides: Partial<SettingsApi> = {}): SettingsApi {
  return {
    getSettings: vi.fn(async () => makeSettings()),
    updateSettings: vi.fn(async () => makeSettings()),
    securityStatus: vi.fn(async () => makeStatus()),
    setMasterPassword: vi.fn(async () => undefined),
    changeMasterPassword: vi.fn(async () => undefined),
    unlock: vi.fn(async () => undefined),
    lock: vi.fn(async () => undefined),
    getDataPath: vi.fn(async () => "/data"),
    getDataStatus: vi.fn(async () => makeDataStatus()),
    previewDataChange: vi.fn(
      async (): Promise<PreviewResult> => ({
        targetPath: "/new",
        exists: true,
        hasPromptHubData: false,
        isCurrent: false,
        recommendedAction: "migrate",
        markers: [],
      }),
    ),
    applyDataChange: vi.fn(
      async (): Promise<ApplyResult> => ({ restartRequired: true, configuredPath: "/new" }),
    ),
    recoveryScan: vi.fn(async () => [] as RecoverySource[]),
    recoveryPreview: vi.fn(async () => ({
      sourcePath: "/old",
      exists: true,
      hasPromptHubData: true,
      markers: [],
    })),
    recoveryApply: vi.fn(
      async (): Promise<ApplyResult> => ({ restartRequired: true, configuredPath: "/data" }),
    ),
    testWebdav: vi.fn(async () => ({ success: true, message: "ok" })),
    testS3: vi.fn(async () => ({ success: true, message: "ok" })),
    exportZip: vi.fn(async () => ({ canceled: false, filePath: "/out.zip" })),
    exportCancel: vi.fn(async () => undefined),
    listBackups: vi.fn(async () => [] as BackupEntry[]),
    createBackup: vi.fn(async () => ({ id: "b1", createdAt: "2024-01-01T00:00:00.000Z" })),
    restoreBackup: vi.fn(async () => ({ id: "b1", restartRequired: true })),
    deleteBackup: vi.fn(async () => undefined),
    ...overrides,
  };
}

function resetStore(api: SettingsApi) {
  useSettingsStore.setState({
    api,
    settings: null,
    securityStatus: null,
    dataStatus: null,
    preview: null,
    recoverySources: [],
    recoveryUnavailable: false,
    backups: [],
    restartRequired: false,
    loading: false,
    error: null,
  });
}

afterEach(() => vi.restoreAllMocks());

describe("settings store (Req 3.1, 15, 17, 19)", () => {
  beforeEach(() => resetStore(makeApi()));

  it("load() fetches settings, security status, data status, and backups", async () => {
    const api = makeApi({
      getSettings: vi.fn(async () => makeSettings({ theme: "light" })),
      securityStatus: vi.fn(async () => makeStatus({ hasMasterPassword: true, isLocked: false })),
      getDataStatus: vi.fn(async () => makeDataStatus({ activePath: "/here" })),
      listBackups: vi.fn(async () => [{ id: "b1", createdAt: "2024-01-01T00:00:00.000Z" }]),
    });
    resetStore(api);

    await useSettingsStore.getState().load();

    const state = useSettingsStore.getState();
    expect(state.settings?.theme).toBe("light");
    expect(state.securityStatus?.hasMasterPassword).toBe(true);
    expect(state.dataStatus?.activePath).toBe("/here");
    expect(state.backups).toHaveLength(1);
    expect(state.loading).toBe(false);
  });

  it("load() marks restartRequired when a configured change is pending (Req 19.3)", async () => {
    resetStore(
      makeApi({
        getDataStatus: vi.fn(async () =>
          makeDataStatus({ restartRequired: true, configuredPath: "/pending" }),
        ),
      }),
    );

    await useSettingsStore.getState().load();

    expect(useSettingsStore.getState().restartRequired).toBe(true);
  });

  it("updateSettings() persists and stores the returned settings (Req 19.2)", async () => {
    const updated = makeSettings({ theme: "system" });
    const update = vi.fn(async () => updated);
    resetStore(makeApi({ updateSettings: update }));

    const result = await useSettingsStore.getState().updateSettings({ theme: "system" });

    expect(update).toHaveBeenCalledWith({ theme: "system" });
    expect(result).toEqual(updated);
    expect(useSettingsStore.getState().settings).toEqual(updated);
  });

  it("mergeLocalSettings() updates the in-memory copy without a backend write", () => {
    const update = vi.fn();
    resetStore(makeApi({ updateSettings: update }));
    useSettingsStore.setState({ settings: makeSettings({ theme: "dark" }) });

    useSettingsStore.getState().mergeLocalSettings({ theme: "light" });

    expect(useSettingsStore.getState().settings?.theme).toBe("light");
    expect(update).not.toHaveBeenCalled();
  });

  it("setMasterPassword() refreshes status on success (Req 15.2)", async () => {
    const status = vi.fn(async () => makeStatus({ hasMasterPassword: true, isLocked: false }));
    resetStore(makeApi({ securityStatus: status }));

    const ok = await useSettingsStore.getState().setMasterPassword("password1");

    expect(ok).toBe(true);
    expect(useSettingsStore.getState().securityStatus?.hasMasterPassword).toBe(true);
  });

  it("setMasterPassword() surfaces a BridgeError and returns false (Req 3.5, 15.3)", async () => {
    resetStore(
      makeApi({
        setMasterPassword: vi.fn(async () => {
          throw { code: "VALIDATION", message: "master password must be 8 to 128 characters" };
        }),
      }),
    );

    const ok = await useSettingsStore.getState().setMasterPassword("short");

    expect(ok).toBe(false);
    expect(useSettingsStore.getState().error).toContain("8 to 128");
  });

  it("changeMasterPassword() marks restartRequired on success (Req 15.4)", async () => {
    resetStore(makeApi());

    const ok = await useSettingsStore
      .getState()
      .changeMasterPassword("oldpassword", "newpassword");

    expect(ok).toBe(true);
    expect(useSettingsStore.getState().restartRequired).toBe(true);
  });

  it("unlock() returns false and keeps state on wrong password (Req 15.7)", async () => {
    resetStore(
      makeApi({
        unlock: vi.fn(async () => {
          throw { code: "UNAUTHORIZED", message: "master password is incorrect" };
        }),
      }),
    );

    const ok = await useSettingsStore.getState().unlock("wrong");

    expect(ok).toBe(false);
    expect(useSettingsStore.getState().error).toBe("master password is incorrect");
  });

  it("previewDataChange() stores the read-only preview (Req 19.4)", async () => {
    const preview: PreviewResult = {
      targetPath: "/new",
      exists: true,
      hasPromptHubData: true,
      isCurrent: false,
      recommendedAction: "switch",
      markers: [{ name: "data", path: "/new/data", kind: "directory" }],
    };
    resetStore(makeApi({ previewDataChange: vi.fn(async () => preview) }));

    const result = await useSettingsStore.getState().previewDataChange("/new");

    expect(result).toEqual(preview);
    expect(useSettingsStore.getState().preview).toEqual(preview);
  });

  it("applyDataChange() reports restartRequired and clears the preview (Req 19.5)", async () => {
    resetStore(makeApi());
    useSettingsStore.setState({
      preview: {
        targetPath: "/new",
        exists: false,
        hasPromptHubData: false,
        isCurrent: false,
        recommendedAction: "migrate",
        markers: [],
      },
    });

    const result = await useSettingsStore.getState().applyDataChange("/new", "migrate");

    expect(result?.restartRequired).toBe(true);
    expect(useSettingsStore.getState().restartRequired).toBe(true);
    expect(useSettingsStore.getState().preview).toBeNull();
  });

  it("recoveryScan() degrades gracefully when the capability is gated (Req 3.7)", async () => {
    resetStore(
      makeApi({
        recoveryScan: vi.fn(async () => {
          throw { code: "CAPABILITY_UNAVAILABLE", message: "no recovery" };
        }),
      }),
    );

    await useSettingsStore.getState().recoveryScan();

    const state = useSettingsStore.getState();
    expect(state.recoveryUnavailable).toBe(true);
    expect(state.recoverySources).toEqual([]);
    expect(state.error).toBeNull();
  });

  it("testWebdav() returns the explicit pass/fail result (Req 17.1)", async () => {
    resetStore(
      makeApi({
        testWebdav: vi.fn(async () => ({ success: false, message: "401 Unauthorized" })),
      }),
    );

    const result = await useSettingsStore
      .getState()
      .testWebdav({ url: "https://x", username: "u", password: "p" });

    expect(result).toEqual({ success: false, message: "401 Unauthorized" });
  });

  it("createBackup() refreshes the backup list (Req 17.6)", async () => {
    const entry = { id: "b2", createdAt: "2024-02-02T00:00:00.000Z" };
    resetStore(
      makeApi({
        createBackup: vi.fn(async () => entry),
        listBackups: vi.fn(async () => [entry]),
      }),
    );

    const result = await useSettingsStore.getState().createBackup();

    expect(result).toEqual(entry);
    expect(useSettingsStore.getState().backups).toEqual([entry]);
  });

  it("restoreBackup() marks restartRequired on success (Req 17.7)", async () => {
    resetStore(makeApi());

    const ok = await useSettingsStore.getState().restoreBackup("b1");

    expect(ok).toBe(true);
    expect(useSettingsStore.getState().restartRequired).toBe(true);
  });

  it("deleteBackup() refreshes the list after removing (Req 17.8)", async () => {
    const list = vi.fn(async () => [] as BackupEntry[]);
    const del = vi.fn(async () => undefined);
    resetStore(makeApi({ deleteBackup: del, listBackups: list }));

    await useSettingsStore.getState().deleteBackup("b1");

    expect(del).toHaveBeenCalledWith("b1");
    expect(list).toHaveBeenCalled();
  });
});
