import { describe, expect, it } from "vitest";
import {
  createPreferenceAppearanceController,
  normalizeAppearancePreferences,
  normalizeInterfaceFontStack,
  resolveInterfaceFontStack,
} from "./preferences";

function makeMediaQuery(initial: boolean) {
  let matches = initial;
  const listeners = new Set<(event: { matches: boolean }) => void>();
  return {
    query: {
      get matches() {
        return matches;
      },
      addEventListener: (_type: "change", listener: (event: { matches: boolean }) => void) => {
        listeners.add(listener);
      },
      removeEventListener: (_type: "change", listener: (event: { matches: boolean }) => void) => {
        listeners.delete(listener);
      },
    },
    emit(next: boolean) {
      matches = next;
      for (const listener of listeners) listener({ matches: next });
    },
  };
}

function makeRoot() {
  const classes = new Set<string>();
  const variables = new Map<string, string>();
  const attributes = new Map<string, string>();
  return {
    classes,
    variables,
    attributes,
    root: {
      classList: {
        add: (token: string) => void classes.add(token),
        remove: (token: string) => void classes.delete(token),
      },
      style: {
        setProperty: (name: string, value: string) => void variables.set(name, value),
      },
      setAttribute: (name: string, value: string) => void attributes.set(name, value),
    },
  };
}

describe("appearance preference migration", () => {
  it.each([
    ["Latte", "catppuccin", "light", "mocha"],
    ["Frappé", "catppuccin", "dark", "frappe"],
    ["Macchiato", "catppuccin", "dark", "macchiato"],
    ["Mocha", "catppuccin", "dark", "mocha"],
    ["Claude Light", "claude", "light", "mocha"],
    ["Claude Dark", "claude", "dark", "mocha"],
    ["PromptHub Light", "prompthub", "light", "mocha"],
    ["PromptHub Dark", "prompthub", "dark", "mocha"],
  ] as const)("migrates legacy %s without changing its effective appearance", (flavor, family, mode, variant) => {
    expect(normalizeAppearancePreferences({ flavor, bodyFont: "Inter" })).toMatchObject({
      themeFamily: family,
      theme: mode,
      catppuccinDarkVariant: variant,
      interfaceFontStack: ["Inter"],
    });
  });
});

describe("interface font fallback", () => {
  it("trims, de-duplicates case-insensitively, and caps persisted families at four", () => {
    expect(normalizeInterfaceFontStack([" Inter ", "inter", "Microsoft YaHei", "Noto Sans", "Arial", "Extra"])).toEqual([
      "Inter",
      "Microsoft YaHei",
      "Noto Sans",
      "Arial",
    ]);
  });

  it("safely quotes families and appends locale, platform, symbol, and generic fallbacks", () => {
    const stack = resolveInterfaceFontStack(["A\\B\"C"], "zh", "windows");
    expect(stack).toContain('"A\\\\B\\\"C"');
    expect(stack).toContain('"Microsoft YaHei UI"');
    expect(stack).toContain('"Segoe UI Symbol"');
    expect(stack).toContain('"Segoe UI"');
    expect(stack.endsWith("sans-serif")).toBe(true);
  });

  it.each([
    ["en", "Segoe UI"],
    ["zh", "Microsoft YaHei UI"],
    ["zh-TW", "Microsoft JhengHei UI"],
    ["ja", "Yu Gothic UI"],
  ] as const)("keeps missing-font fallback readable for %s", (locale, expectedFamily) => {
    const stack = resolveInterfaceFontStack(["Definitely Missing Font"], locale, "windows");
    expect(stack.startsWith('"Definitely Missing Font"')).toBe(true);
    expect(stack).toContain(`"${expectedFamily}"`);
    expect(stack).toContain('"Segoe UI Symbol"');
    expect(stack.endsWith("sans-serif")).toBe(true);
  });
});

describe("PromptHub family defaults", () => {
  it("resolves empty settings to PromptHub Dark", () => {
    const preferences = normalizeAppearancePreferences({});
    expect(preferences.themeFamily).toBe("prompthub");
    expect(preferences.accentColor).toBe("Violet");
    expect(preferences.interfaceFontStack).toEqual(["Noto Sans SC", "System"]);

    const fake = makeRoot();
    const controller = createPreferenceAppearanceController({
      root: fake.root,
      matchMedia: () => makeMediaQuery(true).query,
      platform: "windows",
    });
    controller.apply(preferences, "en");
    expect(controller.effectiveVariant()).toBe("PromptHub Dark");
    expect(fake.classes.has("dark")).toBe(true);
  });

  it("keeps a stored Catppuccin mocha preference", () => {
    const preferences = normalizeAppearancePreferences({
      themeFamily: "catppuccin",
      catppuccinDarkVariant: "mocha",
    });
    expect(preferences.themeFamily).toBe("catppuccin");

    const fake = makeRoot();
    const controller = createPreferenceAppearanceController({
      root: fake.root,
      matchMedia: () => makeMediaQuery(true).query,
      platform: "windows",
    });
    controller.apply(preferences, "en");
    expect(controller.effectiveVariant()).toBe("Mocha");
  });

  it("attaches the media listener for system mode under the PromptHub family", () => {
    const media = makeMediaQuery(false);
    const fake = makeRoot();
    const controller = createPreferenceAppearanceController({
      root: fake.root,
      matchMedia: () => media.query,
      platform: "windows",
    });
    const preferences = normalizeAppearancePreferences({
      theme: "system",
      themeFamily: "prompthub",
    });

    controller.apply(preferences, "en");
    expect(controller.effectiveVariant()).toBe("PromptHub Light");
    expect(fake.classes.has("dark")).toBe(false);

    media.emit(true);
    expect(controller.effectiveVariant()).toBe("PromptHub Dark");
    expect(controller.current()).toEqual(preferences);
    expect(fake.classes.has("dark")).toBe(true);

    media.emit(false);
    expect(controller.effectiveVariant()).toBe("PromptHub Light");
    expect(controller.current().themeFamily).toBe("prompthub");
  });
});

describe("system color mode ownership", () => {
  it("changes only the effective variant when the OS mode changes", () => {
    const media = makeMediaQuery(false);
    const fake = makeRoot();
    const controller = createPreferenceAppearanceController({
      root: fake.root,
      matchMedia: () => media.query,
      platform: "windows",
    });
    const preferences = normalizeAppearancePreferences({
      theme: "system",
      themeFamily: "catppuccin",
      catppuccinDarkVariant: "mocha",
      interfaceFontStack: ["Inter"],
    });

    controller.apply(preferences, "en");
    expect(controller.effectiveVariant()).toBe("Latte");
    expect(fake.classes.has("dark")).toBe(false);

    media.emit(true);
    expect(controller.effectiveVariant()).toBe("Mocha");
    expect(controller.current()).toEqual(preferences);
    expect(fake.classes.has("dark")).toBe(true);

    media.emit(false);
    expect(controller.effectiveVariant()).toBe("Latte");
    expect(controller.current().themeFamily).toBe("catppuccin");
    expect(controller.current().catppuccinDarkVariant).toBe("mocha");
  });
});
