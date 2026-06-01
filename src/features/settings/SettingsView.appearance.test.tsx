// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { SettingsView } from "./SettingsView";
import { useSettingsStore } from "./settingsStore";
import type { SettingsApi } from "./api";
import type { Settings } from "./types";

/**
 * Settings_View appearance section (Req 1.1, 1.2, 1.6). SettingsView's mount-time
 * `load()` is made stable by seeding a fake api + settings on the store so the
 * view renders without a backend.
 */
const fakeSettings = { theme: "dark", language: "en", autoSave: false } as Settings;
const fakeApi = {
  getSettings: async () => fakeSettings,
  securityStatus: async () => ({ hasMasterPassword: false, isLocked: false }),
  getDataStatus: async () => ({ activePath: "", restartRequired: false }),
  listBackups: async () => [],
} as unknown as SettingsApi;

beforeEach(() => {
  useSettingsStore.setState({ api: fakeApi, settings: fakeSettings, error: null });
});

afterEach(cleanup);

describe("SettingsView appearance section (Req 1.1, 1.2, 1.6)", () => {
  it("presents an Appearance rail entry with an i18n label and a Lucide icon", () => {
    render(<SettingsView />);

    const entry = screen.getByRole("button", { name: "Appearance" });
    // The label is routed through i18n (resolves to "Appearance", not a key).
    expect(entry.textContent).toContain("Appearance");
    // The entry renders a Lucide icon (svg).
    expect(entry.querySelector("svg")).toBeTruthy();
  });

  it("renders the AppearancePanel with all seven controls and marks the entry active", () => {
    render(<SettingsView />);

    const entry = screen.getByRole("button", { name: "Appearance" });
    fireEvent.click(entry);

    // The entry becomes the active rail item.
    expect(entry.getAttribute("aria-current")).toBe("page");

    // All seven appearance controls are present.
    expect(screen.getByRole("group", { name: "Flavor" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Accent color" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Font scale" })).toBeTruthy();
    expect(screen.getByRole("group", { name: "Density" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Display font" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Body font" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Language" })).toBeTruthy();
  });

  it("replaces the previously displayed panel when Appearance is selected", () => {
    render(<SettingsView />);

    // The General panel is shown by default; the appearance Flavor group is not.
    expect(screen.getByRole("switch", { name: "Auto save" })).toBeTruthy();
    expect(screen.queryByRole("group", { name: "Flavor" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Appearance" }));

    // The Appearance panel replaces the General panel.
    expect(screen.getByRole("group", { name: "Flavor" })).toBeTruthy();
    expect(screen.queryByRole("switch", { name: "Auto save" })).toBeNull();
  });
});
