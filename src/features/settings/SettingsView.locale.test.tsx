// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AppShell } from "../../components/layout/AppShell";
import i18n, { ensureBundle } from "../../runtime/i18n";
import { useAppStore } from "../../store/appStore";
import { useSystemStore } from "../system/systemStore";
import type { SettingsApi } from "./api";
import { useSettingsStore } from "./settingsStore";
import type { Settings } from "./types";

const baseSettings = { theme: "dark", language: "en", autoSave: true } as Settings;
const initializeSystem = useSystemStore.getState().initialize;

function makeApi() {
  let current = baseSettings;
  return {
    getSettings: vi.fn(async () => current),
    updateSettings: vi.fn(async (patch: Partial<Settings>) => {
      current = { ...current, ...patch };
      return current;
    }),
    securityStatus: vi.fn(async () => ({ hasMasterPassword: false, isLocked: false })),
    getDataStatus: vi.fn(async () => ({ activePath: "", restartRequired: false })),
    listBackups: vi.fn(async () => []),
  } as unknown as SettingsApi;
}

beforeEach(async () => {
  await ensureBundle("en");
  await i18n.changeLanguage("en");
  document.documentElement.lang = "en";
  useAppStore.setState({ activeView: "settings", sidebarCollapsed: true });
  useSystemStore.setState({ initialize: () => () => {} });
  useSettingsStore.setState({
    api: makeApi(),
    settings: baseSettings,
    error: null,
    loading: false,
  });
});

afterEach(async () => {
  cleanup();
  useSystemStore.setState({ initialize: initializeSystem });
  await i18n.changeLanguage("en");
});

describe("live settings localization", () => {
  it("rerenders the mounted shell and General panel when Simplified Chinese is selected", async () => {
    render(<AppShell />);

    expect(screen.getAllByText("Settings").length).toBeGreaterThan(0);
    expect(screen.getByText("General")).toBeTruthy();
    expect(screen.getByText("Auto save")).toBeTruthy();

    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), {
      target: { value: "zh" },
    });

    await waitFor(() => {
      expect(screen.getAllByText("设置").length).toBeGreaterThan(0);
      expect(screen.getByText("通用")).toBeTruthy();
      expect(screen.getByText("自动保存")).toBeTruthy();
    });
    expect(document.documentElement.lang).toBe("zh");
  });
});
