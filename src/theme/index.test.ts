import { describe, expect, it, vi } from "vitest";
import {
  createThemeController,
  DEFAULT_THEME,
  initializeTheme,
  isDarkMode,
  normalizeThemeMode,
  setTheme,
  type ClassListLike,
  type MediaQuery,
  type ThemeMode,
} from "./index";

/** A fake class list that records the current token set. */
function makeClassList() {
  const tokens = new Set<string>();
  const classList: ClassListLike = {
    add: (t) => tokens.add(t),
    remove: (t) => tokens.delete(t),
  };
  return { classList, has: (t: string) => tokens.has(t) };
}

/** A controllable fake media query for `prefers-color-scheme`. */
function makeMediaQuery(initialMatches: boolean) {
  let matches = initialMatches;
  const listeners = new Set<(e: { matches: boolean }) => void>();
  const mq: MediaQuery = {
    get matches() {
      return matches;
    },
    addEventListener: (_type, listener) => listeners.add(listener),
    removeEventListener: (_type, listener) => listeners.delete(listener),
  };
  return {
    mq,
    listenerCount: () => listeners.size,
    /** Simulate an OS theme change. */
    emit: (next: boolean) => {
      matches = next;
      for (const l of listeners) l({ matches: next });
    },
  };
}

describe("normalizeThemeMode (Req 22.5)", () => {
  it("passes through valid modes", () => {
    expect(normalizeThemeMode("light")).toBe("light");
    expect(normalizeThemeMode("dark")).toBe("dark");
    expect(normalizeThemeMode("system")).toBe("system");
  });

  it("defaults unknown/missing values to dark", () => {
    expect(normalizeThemeMode(undefined)).toBe("dark");
    expect(normalizeThemeMode(null)).toBe("dark");
    expect(normalizeThemeMode("")).toBe("dark");
    expect(normalizeThemeMode("solarized")).toBe("dark");
    expect(normalizeThemeMode(42)).toBe("dark");
    expect(DEFAULT_THEME).toBe("dark");
  });
});

describe("isDarkMode", () => {
  it("is explicit for light/dark regardless of OS preference", () => {
    expect(isDarkMode("light", true)).toBe(false);
    expect(isDarkMode("light", false)).toBe(false);
    expect(isDarkMode("dark", false)).toBe(true);
    expect(isDarkMode("dark", true)).toBe(true);
  });

  it("follows the OS preference for system mode", () => {
    expect(isDarkMode("system", true)).toBe(true);
    expect(isDarkMode("system", false)).toBe(false);
  });
});

describe("createThemeController.apply (Req 22.2)", () => {
  it("adds the dark class for dark mode and removes it for light mode", () => {
    const { classList, has } = makeClassList();
    const controller = createThemeController({
      root: { classList },
      matchMedia: () => makeMediaQuery(false).mq,
    });

    controller.apply("dark");
    expect(has("dark")).toBe(true);
    expect(controller.current()).toBe("dark");

    controller.apply("light");
    expect(has("dark")).toBe(false);
    expect(controller.current()).toBe("light");
  });

  it("system mode paints from the OS preference and reacts to OS changes", () => {
    const { classList, has } = makeClassList();
    const media = makeMediaQuery(true); // OS currently prefers dark
    const controller = createThemeController({
      root: { classList },
      matchMedia: () => media.mq,
    });

    controller.apply("system");
    expect(has("dark")).toBe(true);

    // OS switches to light -> class removed without re-applying.
    media.emit(false);
    expect(has("dark")).toBe(false);

    // OS switches back to dark.
    media.emit(true);
    expect(has("dark")).toBe(true);
  });

  it("detaches the system subscription when switching away, and on dispose", () => {
    const { classList } = makeClassList();
    const media = makeMediaQuery(false);
    const controller = createThemeController({
      root: { classList },
      matchMedia: () => media.mq,
    });

    controller.apply("system");
    expect(media.listenerCount()).toBe(1);

    // Switching to an explicit mode drops the subscription.
    controller.apply("light");
    expect(media.listenerCount()).toBe(0);

    // Re-subscribe then dispose.
    controller.apply("system");
    expect(media.listenerCount()).toBe(1);
    controller.dispose();
    expect(media.listenerCount()).toBe(0);
  });

  it("does not leak listeners when re-applying system mode", () => {
    const { classList } = makeClassList();
    const media = makeMediaQuery(false);
    const controller = createThemeController({
      root: { classList },
      matchMedia: () => media.mq,
    });

    controller.apply("system");
    controller.apply("system");
    expect(media.listenerCount()).toBe(1);
  });
});

describe("initializeTheme (Req 22.5)", () => {
  function controllerWithSpy() {
    const applied: ThemeMode[] = [];
    const controller = {
      apply: (m: ThemeMode) => applied.push(m),
      current: () => applied[applied.length - 1] ?? DEFAULT_THEME,
      dispose: () => {},
    };
    return { controller, applied };
  }

  it("applies the persisted theme from settings", async () => {
    const invoke = vi.fn(async () => ({ theme: "light" }));
    const { controller, applied } = controllerWithSpy();
    const mode = await initializeTheme(invoke as never, controller);
    expect(mode).toBe("light");
    expect(applied).toEqual(["light"]);
    expect(invoke).toHaveBeenCalledWith("settings.get");
  });

  it("defaults to dark when settings carry no valid theme", async () => {
    const invoke = vi.fn(async () => ({}));
    const { controller, applied } = controllerWithSpy();
    const mode = await initializeTheme(invoke as never, controller);
    expect(mode).toBe("dark");
    expect(applied).toEqual(["dark"]);
  });

  it("falls back to dark when the settings read fails", async () => {
    const invoke = vi.fn(async () => {
      throw new Error("backend unavailable");
    });
    const { controller, applied } = controllerWithSpy();
    const mode = await initializeTheme(invoke as never, controller);
    expect(mode).toBe("dark");
    expect(applied).toEqual(["dark"]);
  });
});

describe("setTheme (Req 22.2)", () => {
  it("applies the mode and persists it via settings.update", async () => {
    const applied: ThemeMode[] = [];
    const controller = {
      apply: (m: ThemeMode) => applied.push(m),
      current: () => applied[applied.length - 1] ?? DEFAULT_THEME,
      dispose: () => {},
    };
    const invoke = vi.fn(async () => undefined);

    await setTheme("system", invoke as never, controller);

    expect(applied).toEqual(["system"]);
    expect(invoke).toHaveBeenCalledWith("settings.update", { patch: { theme: "system" } });
  });
});
