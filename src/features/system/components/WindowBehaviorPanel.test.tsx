// @vitest-environment jsdom
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import windowBehaviorPanelSrc from "./WindowBehaviorPanel.tsx?raw";
import { WindowBehaviorPanel } from "./WindowBehaviorPanel";
import { useSystemStore } from "../systemStore";
import i18n, { ensureBundle } from "../../../runtime/i18n";

const initialSystem = useSystemStore.getState();

afterEach(() => {
  cleanup();
  useSystemStore.setState(initialSystem, true);
});

describe("WindowBehaviorPanel (Req 20.4, 20.5)", () => {
  it("does not import @tauri-apps/api", () => {
    expect(windowBehaviorPanelSrc).not.toContain("@tauri-apps/api");
  });

  it("keeps the launch-at-login switch visually off when autoLaunch is false", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    useSystemStore.setState({ autoLaunch: false });
    render(<WindowBehaviorPanel />);

    const toggle = screen.getByRole("switch", { name: "Launch at login" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(toggle.className).toContain("bg-input");
    expect(toggle.className).not.toContain("bg-primary");
    const thumb = toggle.querySelector("span");
    expect(thumb?.className).toContain("left-0.5");
    expect(thumb?.className).toContain("translate-x-0");
    expect(thumb?.className).not.toContain("translate-x-5");
    expect(thumb?.className).not.toContain("translate-x-0.5");
  });

  it("moves the launch-at-login thumb and uses the primary track when autoLaunch is true", async () => {
    await ensureBundle("en");
    await i18n.changeLanguage("en");
    useSystemStore.setState({ autoLaunch: true });
    render(<WindowBehaviorPanel />);

    const toggle = screen.getByRole("switch", { name: "Launch at login" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    expect(toggle.className).toContain("bg-primary");
    const thumb = toggle.querySelector("span");
    expect(thumb?.className).toContain("translate-x-5");
  });
});
