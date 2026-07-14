import { describe, expect, it, vi } from "vitest";
import type { Settings } from "./features/settings/types";
import { startApplication } from "./bootstrap";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

describe("startApplication", () => {
  it("reads settings once and mounts only after locale and appearance are ready", async () => {
    const settingsReady = deferred<Settings>();
    const localeReady = deferred<"zh">();
    const order: string[] = [];
    const settings = { theme: "system", language: "zh", autoSave: true } as Settings;

    const loadSettings = vi.fn(() => settingsReady.promise);
    const initializeLocale = vi.fn(() => localeReady.promise);
    const applyAppearance = vi.fn(() => order.push("appearance"));
    const hydrateSettings = vi.fn(() => order.push("hydrate"));
    const mount = vi.fn(() => order.push("mount"));

    const started = startApplication({
      loadSettings,
      initializeLocale,
      applyAppearance,
      hydrateSettings,
      mount,
    });

    expect(loadSettings).toHaveBeenCalledOnce();
    expect(mount).not.toHaveBeenCalled();

    settingsReady.resolve(settings);
    await Promise.resolve();
    expect(initializeLocale).toHaveBeenCalledWith("zh");
    expect(mount).not.toHaveBeenCalled();

    localeReady.resolve("zh");
    await started;

    expect(applyAppearance).toHaveBeenCalledWith(settings, "zh");
    expect(hydrateSettings).toHaveBeenCalledWith(settings);
    expect(order).toEqual(["appearance", "hydrate", "mount"]);
  });

  it("mounts with documented defaults when settings and locale startup fail", async () => {
    const applyAppearance = vi.fn();
    const hydrateSettings = vi.fn();
    const mount = vi.fn();

    await startApplication({
      loadSettings: vi.fn(async () => {
        throw new Error("backend unavailable");
      }),
      initializeLocale: vi.fn(async () => {
        throw new Error("bundle unavailable");
      }),
      applyAppearance,
      hydrateSettings,
      mount,
    });

    const defaults = { theme: "dark", language: "en", autoSave: true };
    expect(applyAppearance).toHaveBeenCalledWith(defaults, "en");
    expect(hydrateSettings).toHaveBeenCalledWith(defaults);
    expect(mount).toHaveBeenCalledOnce();
  });
});
