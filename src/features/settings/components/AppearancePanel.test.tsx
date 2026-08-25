// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import appearancePanelSrc from "./AppearancePanel.tsx?raw";
import preferencesSrc from "../../../appearance/preferences.ts?raw";
import { AppearancePanel } from "./AppearancePanel";
import { useSettingsStore, type PreferenceRuntime } from "../settingsStore";
import type { SettingsApi } from "../api";
import type { Settings, SettingsPatch } from "../types";

const baseSettings: Settings = {
  theme: "dark",
  language: "en",
  autoSave: true,
  themeFamily: "catppuccin",
  catppuccinDarkVariant: "mocha",
  interfaceFontStack: ["System"],
};

function makeApi(updateSettings?: (patch: SettingsPatch) => Promise<Settings>) {
  return {
    updateSettings:
      updateSettings ??
      vi.fn(async (patch: SettingsPatch) => ({ ...baseSettings, ...patch })),
    listSystemFonts: vi.fn(async () => ["Arial", "Microsoft YaHei UI"]),
  } as unknown as SettingsApi;
}

function makeRuntime(): PreferenceRuntime {
  return {
    currentLocale: () => "en",
    changeLocale: vi.fn(async () => {}),
    applyAppearance: vi.fn(),
  };
}

beforeEach(() => {
  useSettingsStore.setState({
    api: makeApi(),
    settings: baseSettings,
    error: null,
    systemFonts: [],
    systemFontsStatus: "idle",
    preferenceRuntime: makeRuntime(),
    preferenceStatus: {},
    preferenceErrors: {},
    pendingPreferences: {},
  });
});

afterEach(cleanup);

describe("AppearancePanel", () => {
  it("renders the independent theme, mode, accent, font, scale, and density controls", async () => {
    render(<AppearancePanel />);

    expect(within(screen.getByRole("group", { name: "Theme family" })).getAllByRole("button")).toHaveLength(3);
    expect(within(screen.getByRole("group", { name: "Color mode" })).getAllByRole("button")).toHaveLength(3);
    expect(within(screen.getByRole("group", { name: "Accent color" })).getAllByRole("button")).toHaveLength(15);
    expect(within(screen.getByRole("group", { name: "Font scale" })).getAllByRole("button")).toHaveLength(4);
    expect(within(screen.getByRole("group", { name: "Density" })).getAllByRole("button")).toHaveLength(3);
    expect(screen.getByRole("combobox", { name: "Catppuccin dark variant" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Primary font" })).toBeTruthy();
    await waitFor(() => expect(screen.getByRole("option", { name: "Arial" })).toBeTruthy());
  });

  it("migrates a legacy Claude flavor into the selected family and mode", () => {
    useSettingsStore.setState({
      settings: { ...baseSettings, themeFamily: null, flavor: "Claude Light" },
    });
    render(<AppearancePanel />);

    expect(screen.getByRole("button", { name: /Claude/ }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Light" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByRole("combobox", { name: "Catppuccin dark variant" })).toBeNull();
  });

  it("persists through the settings store and adopts the returned canonical settings", async () => {
    const update = vi.fn(async (patch: SettingsPatch) => ({
      ...baseSettings,
      ...patch,
      themeFamily: "claude",
    }));
    useSettingsStore.setState({ api: makeApi(update) });
    render(<AppearancePanel />);

    fireEvent.click(screen.getByRole("button", { name: /Claude/ }));

    await waitFor(() => expect(useSettingsStore.getState().preferenceStatus.themeFamily).toBe("saved"));
    expect(update).toHaveBeenCalledOnce();
    expect(useSettingsStore.getState().settings?.themeFamily).toBe("claude");
    expect(screen.getByText("Saved")).toBeTruthy();
  });

  it("keeps a failed preview and offers an inline retry", async () => {
    useSettingsStore.setState({
      api: makeApi(vi.fn(async () => { throw new Error("disk full"); })),
    });
    render(<AppearancePanel />);

    fireEvent.click(screen.getByRole("button", { name: /Claude/ }));

    await waitFor(() => expect(screen.getByText("Applied for this session, but not saved.")).toBeTruthy());
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
    expect(useSettingsStore.getState().settings?.themeFamily).toBe("claude");
  });

  it("keeps the Runtime Bridge boundary outside the component", () => {
    expect(appearancePanelSrc).not.toContain("@tauri-apps/api");
    expect(appearancePanelSrc).not.toContain("runtime.invoke");
    expect(preferencesSrc).not.toContain("settings.update");
  });
});
