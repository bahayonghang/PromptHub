// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import appearancePanelSrc from "./AppearancePanel.tsx?raw";
import specimenCardSrc from "./SpecimenCard.tsx?raw";
import summaryStripSrc from "./SummaryStrip.tsx?raw";
import appearanceIndexSrc from "../../../appearance/index.ts?raw";
import { AppearancePanel, type AppearancePanelProps } from "./AppearancePanel";
import { useSettingsStore } from "../settingsStore";
import type { Settings } from "../types";
import { createAppearanceController } from "../../../appearance";
import { BridgeError } from "../../../runtime";
import i18n from "../../../runtime/i18n";

/** A resolved-bridge invoke spy typed for the panel prop. */
function okInvoke() {
  return vi.fn().mockResolvedValue(undefined) as unknown as AppearancePanelProps["invoke"];
}

function group(name: string) {
  return within(screen.getByRole("group", { name }));
}

function combobox(name: string): HTMLSelectElement {
  return screen.getByRole("combobox", { name }) as HTMLSelectElement;
}

beforeEach(() => {
  useSettingsStore.setState({
    settings: { theme: "dark", language: "en", autoSave: false } as Settings,
    error: null,
  });
});

afterEach(cleanup);

describe("AppearancePanel controls (Req 1-7)", () => {
  it("offers exactly each control's catalog and uses Lucide icons", () => {
    const { container } = render(<AppearancePanel invoke={okInvoke()} changeLocaleFn={vi.fn()} />);

    expect(group("Flavor").getAllByRole("button")).toHaveLength(6);
    expect(group("Accent color").getAllByRole("button")).toHaveLength(14);
    expect(group("Font scale").getAllByRole("button")).toHaveLength(4);
    expect(group("Density").getAllByRole("button")).toHaveLength(3);
    expect(within(combobox("Display font")).getAllByRole("option")).toHaveLength(4);
    expect(within(combobox("Body font")).getAllByRole("option")).toHaveLength(4);
    expect(within(combobox("Language")).getAllByRole("option")).toHaveLength(7);

    // Every control section renders a Lucide icon (svg).
    expect(container.querySelectorAll("svg").length).toBeGreaterThanOrEqual(8);
  });

  it("pre-selects the documented defaults when no appearance is persisted", () => {
    render(<AppearancePanel invoke={okInvoke()} changeLocaleFn={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Mocha" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Blue" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Default (100%)" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Default" }).getAttribute("aria-pressed")).toBe("true");
    expect(combobox("Display font").value).toBe("System");
    expect(combobox("Body font").value).toBe("System");
    expect(combobox("Language").value).toBe("en");
  });

  it("pre-selects the applied value when appearance is persisted", () => {
    useSettingsStore.setState({
      settings: {
        theme: "dark",
        language: "en",
        autoSave: false,
        flavor: "Latte",
        accentColor: "Red",
        displayFont: "Inter",
        bodyFont: "JetBrains Mono",
        fontScale: "Large",
        density: "Compact",
      } as Settings,
    });
    render(<AppearancePanel invoke={okInvoke()} changeLocaleFn={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Latte" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Red" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Large (110%)" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Compact" }).getAttribute("aria-pressed")).toBe("true");
    expect(combobox("Display font").value).toBe("Inter");
    expect(combobox("Body font").value).toBe("JetBrains Mono");
  });
});

describe("AppearancePanel persistence (Req 1.5, 2.5)", () => {
  it("routes persistence through the injected bridge and syncs the store", () => {
    const invoke = okInvoke();
    render(<AppearancePanel invoke={invoke} changeLocaleFn={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Latte" }));

    expect(invoke).toHaveBeenCalledWith("settings.update", { patch: { flavor: "Latte" } });
    expect(useSettingsStore.getState().settings?.flavor).toBe("Latte");
  });

  it("keeps the applied value and sets the store error when persistence is rejected (Req 3.8)", async () => {
    const invoke = vi
      .fn()
      .mockRejectedValue(new BridgeError("INTERNAL", "nope")) as unknown as AppearancePanelProps["invoke"];
    const controller = createAppearanceController({ root: document.createElement("div") });
    render(<AppearancePanel invoke={invoke} controller={controller} changeLocaleFn={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Latte" }));

    await waitFor(() => expect(useSettingsStore.getState().error).toBeTruthy());
    // The applied value is kept for the session despite the failed persist.
    expect(controller.current().flavor).toBe("Latte");
  });
});

describe("AppearancePanel language delegation (Req 7)", () => {
  it("delegates language selection to changeLocale and reflects the active locale", () => {
    const changeLocaleFn = vi.fn().mockResolvedValue(undefined);
    render(
      <AppearancePanel
        invoke={okInvoke()}
        changeLocaleFn={changeLocaleFn as AppearancePanelProps["changeLocaleFn"]}
      />,
    );

    const select = combobox("Language");
    expect(select.value).toBe(i18n.language); // reflects the active locale

    fireEvent.change(select, { target: { value: "ja" } });
    expect(changeLocaleFn).toHaveBeenCalledWith("ja");
  });
});

describe("AppearancePanel routing constraint (Req 1.5)", () => {
  it("does not import @tauri-apps/api in any appearance module", () => {
    const sources: Record<string, string> = {
      "AppearancePanel.tsx": appearancePanelSrc,
      "SpecimenCard.tsx": specimenCardSrc,
      "SummaryStrip.tsx": summaryStripSrc,
      "appearance/index.ts": appearanceIndexSrc,
    };
    for (const [name, source] of Object.entries(sources)) {
      expect(source, `${name} must not import @tauri-apps/api`).not.toContain("@tauri-apps/api");
    }
  });
});
