// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SettingsView } from "./SettingsView";
import { useSettingsStore } from "./settingsStore";
import type { SettingsApi } from "./api";
import type { Settings } from "./types";

const fakeSettings = {
  theme: "dark",
  language: "en",
  autoSave: false,
  themeFamily: "catppuccin",
  catppuccinDarkVariant: "mocha",
  interfaceFontStack: ["System"],
} as Settings;

const fakeApi = {
  getSettings: async () => fakeSettings,
  securityStatus: async () => ({ hasMasterPassword: false, isLocked: false }),
  getDataStatus: async () => ({ activePath: "", restartRequired: false }),
  listBackups: async () => [],
  listSystemFonts: async () => [],
} as unknown as SettingsApi;

beforeEach(() => {
  useSettingsStore.setState({
    api: fakeApi,
    settings: fakeSettings,
    error: null,
    systemFontsStatus: "idle",
    preferenceStatus: {},
    preferenceErrors: {},
  });
});

afterEach(cleanup);

describe("SettingsView appearance and general sections", () => {
  it("starts in General with language and behavior controls", () => {
    render(<SettingsView />);

    expect(screen.getByRole("button", { name: "General" }).getAttribute("aria-current")).toBe("page");
    expect(screen.getByRole("combobox", { name: "Language" })).toBeTruthy();
    expect(screen.getByRole("switch", { name: "Auto save" })).toBeTruthy();
  });

  it("switches to the new Appearance controls and replaces General", () => {
    render(<SettingsView />);
    fireEvent.click(screen.getByRole("button", { name: "Appearance" }));

    expect(screen.getByRole("group", { name: "Theme family" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Color mode" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Accent color" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Primary font" })).toBeTruthy();
    expect(screen.queryByRole("combobox", { name: "Language" })).toBeNull();
    expect(screen.queryByText("Current selections")).toBeNull();
  });
});
