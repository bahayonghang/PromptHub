import type { Settings } from "../features/settings/types";
import type { SupportedLocale } from "../runtime/i18n";
import {
  ACCENT_COLORS,
  ACCENT_PALETTE,
  DEFAULT_ACCENT,
  DEFAULT_DENSITY,
  DEFAULT_FONT_SCALE,
  DENSITIES,
  DENSITY_RHYTHM,
  FLAVOR_BASE,
  FLAVOR_OVERRIDES,
  FONT_SCALE_PERCENT,
  FONT_SCALES,
  type AccentColor,
  type Density,
  type Flavor,
  type FontScale,
} from "./index";

export type ColorMode = "light" | "dark" | "system";
export type ThemeFamily = "catppuccin" | "claude";
export type CatppuccinDarkVariant = "frappe" | "macchiato" | "mocha";
export type HostPlatform = "windows" | "macos" | "linux";

export interface AppearancePreferences {
  theme: ColorMode;
  themeFamily: ThemeFamily;
  catppuccinDarkVariant: CatppuccinDarkVariant;
  accentColor: AccentColor;
  interfaceFontStack: string[];
  fontScale: FontScale;
  density: Density;
}

export const COLOR_MODES: readonly ColorMode[] = ["light", "dark", "system"];
export const THEME_FAMILIES: readonly ThemeFamily[] = ["catppuccin", "claude"];
export const CATPPUCCIN_DARK_VARIANTS: readonly CatppuccinDarkVariant[] = [
  "frappe",
  "macchiato",
  "mocha",
];
export const MAX_INTERFACE_FONT_FAMILIES = 4;
export const DEFAULT_THEME_FAMILY: ThemeFamily = "catppuccin";
export const DEFAULT_COLOR_MODE: ColorMode = "dark";
export const DEFAULT_CATPPUCCIN_DARK_VARIANT: CatppuccinDarkVariant = "mocha";

const LEGACY_FLAVOR_MIGRATION: Record<
  Flavor,
  Pick<AppearancePreferences, "themeFamily" | "theme" | "catppuccinDarkVariant">
> = {
  Latte: { themeFamily: "catppuccin", theme: "light", catppuccinDarkVariant: "mocha" },
  Frappé: { themeFamily: "catppuccin", theme: "dark", catppuccinDarkVariant: "frappe" },
  Macchiato: { themeFamily: "catppuccin", theme: "dark", catppuccinDarkVariant: "macchiato" },
  Mocha: { themeFamily: "catppuccin", theme: "dark", catppuccinDarkVariant: "mocha" },
  "Claude Light": { themeFamily: "claude", theme: "light", catppuccinDarkVariant: "mocha" },
  "Claude Dark": { themeFamily: "claude", theme: "dark", catppuccinDarkVariant: "mocha" },
};

function catalogValue<T extends string>(value: unknown, catalog: readonly T[], fallback: T): T {
  return catalog.includes(value as T) ? (value as T) : fallback;
}

function legacyMigration(value: unknown) {
  return typeof value === "string" && value in LEGACY_FLAVOR_MIGRATION
    ? LEGACY_FLAVOR_MIGRATION[value as Flavor]
    : null;
}

export function normalizeInterfaceFontStack(
  value: unknown,
  legacyBodyFont?: unknown,
  legacyDisplayFont?: unknown,
): string[] {
  const source = Array.isArray(value)
    ? value
    : typeof legacyBodyFont === "string" && legacyBodyFont.trim() !== ""
      ? [legacyBodyFont]
      : typeof legacyDisplayFont === "string" && legacyDisplayFont.trim() !== ""
        ? [legacyDisplayFont]
        : ["System"];
  const result: string[] = [];
  const seen = new Set<string>();

  for (const item of source) {
    if (typeof item !== "string") continue;
    const family = item.trim();
    const key = family.toLocaleLowerCase("en-US");
    if (family === "" || family.length > 128 || /[\u0000-\u001f\u007f]/.test(family) || seen.has(key)) {
      continue;
    }
    result.push(family);
    seen.add(key);
    if (result.length === MAX_INTERFACE_FONT_FAMILIES) break;
  }

  return result.length > 0 ? result : ["System"];
}

export function normalizeAppearancePreferences(raw: Partial<Settings>): AppearancePreferences {
  const migrated = legacyMigration(raw.flavor);
  const hasNewFamily = THEME_FAMILIES.includes(raw.themeFamily as ThemeFamily);
  const themeFamily = hasNewFamily
    ? (raw.themeFamily as ThemeFamily)
    : (migrated?.themeFamily ?? DEFAULT_THEME_FAMILY);
  const theme = hasNewFamily
    ? catalogValue(raw.theme, COLOR_MODES, DEFAULT_COLOR_MODE)
    : (migrated?.theme ?? catalogValue(raw.theme, COLOR_MODES, DEFAULT_COLOR_MODE));
  const catppuccinDarkVariant = catalogValue(
    raw.catppuccinDarkVariant,
    CATPPUCCIN_DARK_VARIANTS,
    migrated?.catppuccinDarkVariant ?? DEFAULT_CATPPUCCIN_DARK_VARIANT,
  );

  return {
    theme,
    themeFamily,
    catppuccinDarkVariant,
    accentColor: catalogValue(raw.accentColor, ACCENT_COLORS, DEFAULT_ACCENT),
    interfaceFontStack: normalizeInterfaceFontStack(
      raw.interfaceFontStack,
      raw.bodyFont,
      raw.displayFont,
    ),
    fontScale: catalogValue(raw.fontScale, FONT_SCALES, DEFAULT_FONT_SCALE),
    density: catalogValue(raw.density, DENSITIES, DEFAULT_DENSITY),
  };
}

export function resolveThemeVariant(
  preferences: AppearancePreferences,
  systemPrefersDark: boolean,
): Flavor {
  const dark =
    preferences.theme === "dark" ||
    (preferences.theme === "system" && systemPrefersDark);
  if (preferences.themeFamily === "claude") {
    return dark ? "Claude Dark" : "Claude Light";
  }
  if (!dark) return "Latte";
  switch (preferences.catppuccinDarkVariant) {
    case "frappe":
      return "Frappé";
    case "macchiato":
      return "Macchiato";
    case "mocha":
      return "Mocha";
  }
}

const LOCALE_FALLBACKS: Partial<Record<SupportedLocale, readonly string[]>> = {
  zh: ["Microsoft YaHei UI", "PingFang SC", "Noto Sans CJK SC"],
  "zh-TW": ["Microsoft JhengHei UI", "PingFang TC", "Noto Sans CJK TC"],
  ja: ["Yu Gothic UI", "Hiragino Sans", "Noto Sans CJK JP"],
};

const PLATFORM_FALLBACKS: Record<HostPlatform, readonly string[]> = {
  windows: ["Segoe UI"],
  macos: ["-apple-system", "BlinkMacSystemFont"],
  linux: ["Ubuntu", "Cantarell", "Noto Sans"],
};

const SYMBOL_FALLBACKS = ["Segoe UI Symbol", "Apple Symbols", "Noto Sans Symbols 2"] as const;
const CSS_GENERIC_FAMILIES = new Set(["sans-serif", "system-ui", "ui-sans-serif", "-apple-system"]);

function hostPlatform(): HostPlatform {
  if (typeof navigator === "undefined") return "windows";
  const platform = `${navigator.platform} ${navigator.userAgent}`.toLowerCase();
  if (platform.includes("mac")) return "macos";
  if (platform.includes("linux")) return "linux";
  return "windows";
}

function serializeFamily(family: string): string {
  if (CSS_GENERIC_FAMILIES.has(family)) return family;
  const escaped = family
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\//g, "\\/")
    .replace(/[\u0000-\u001f\u007f]/g, "");
  return `"${escaped}"`;
}

export function resolveInterfaceFontStack(
  stack: readonly string[],
  locale: SupportedLocale,
  platform: HostPlatform = hostPlatform(),
): string {
  const families: string[] = [];
  const seen = new Set<string>();
  const append = (family: string) => {
    if (family === "System") return;
    const key = family.toLocaleLowerCase("en-US");
    if (seen.has(key)) return;
    families.push(family);
    seen.add(key);
  };

  normalizeInterfaceFontStack(stack).forEach(append);
  (LOCALE_FALLBACKS[locale] ?? []).forEach(append);
  PLATFORM_FALLBACKS[platform].forEach(append);
  SYMBOL_FALLBACKS.forEach(append);
  append("system-ui");
  append("sans-serif");
  return families.map(serializeFamily).join(", ");
}

export interface PreferenceMediaQuery {
  readonly matches: boolean;
  addEventListener(type: "change", listener: (event: { matches: boolean }) => void): void;
  removeEventListener(type: "change", listener: (event: { matches: boolean }) => void): void;
}

export interface PreferenceAppearanceDeps {
  root: {
    classList: { add(token: string): void; remove(token: string): void };
    style: { setProperty(name: string, value: string): void };
    setAttribute(name: string, value: string): void;
  };
  matchMedia: (query: string) => PreferenceMediaQuery;
  platform: HostPlatform;
}

export interface PreferenceAppearanceController {
  apply(preferences: AppearancePreferences, locale: SupportedLocale): void;
  current(): AppearancePreferences;
  effectiveVariant(): Flavor;
  dispose(): void;
}

export function createPreferenceAppearanceController(
  deps: Partial<PreferenceAppearanceDeps> = {},
): PreferenceAppearanceController {
  const root = deps.root ?? document.documentElement;
  const matchMedia = deps.matchMedia ?? ((query: string) => window.matchMedia(query));
  const platform = deps.platform ?? hostPlatform();
  let preferences = normalizeAppearancePreferences({});
  let locale: SupportedLocale = "en";
  let variant = resolveThemeVariant(preferences, true);
  let mediaQuery: PreferenceMediaQuery | null = null;
  let mediaListener: ((event: { matches: boolean }) => void) | null = null;

  const setVars = (values: Record<string, string>) => {
    for (const [name, value] of Object.entries(values)) root.style.setProperty(name, value);
  };
  const detach = () => {
    if (mediaQuery && mediaListener) mediaQuery.removeEventListener("change", mediaListener);
    mediaQuery = null;
    mediaListener = null;
  };
  const paint = (systemPrefersDark: boolean) => {
    variant = resolveThemeVariant(preferences, systemPrefersDark);
    const dark = FLAVOR_BASE[variant] === "dark";
    if (dark) root.classList.add("dark");
    else root.classList.remove("dark");
    setVars(FLAVOR_OVERRIDES[variant]);
    setVars(ACCENT_PALETTE[dark ? "dark" : "light"][preferences.accentColor]);
    root.style.setProperty(
      "--font-interface",
      resolveInterfaceFontStack(preferences.interfaceFontStack, locale, platform),
    );
    root.style.setProperty("--font-scale", String(FONT_SCALE_PERCENT[preferences.fontScale] / 100));
    setVars(DENSITY_RHYTHM[preferences.density]);
    root.style.setProperty("color-scheme", dark ? "dark" : "light");
    root.setAttribute("data-density", preferences.density);
    root.setAttribute("data-theme-family", preferences.themeFamily);
    root.setAttribute("data-theme-variant", variant);
    root.setAttribute("data-color-mode", preferences.theme);
    root.setAttribute("lang", locale);
  };

  return {
    apply(next, nextLocale) {
      detach();
      preferences = { ...next, interfaceFontStack: [...next.interfaceFontStack] };
      locale = nextLocale;
      if (preferences.theme === "system") {
        mediaQuery = matchMedia("(prefers-color-scheme: dark)");
        paint(mediaQuery.matches);
        mediaListener = (event) => paint(event.matches);
        mediaQuery.addEventListener("change", mediaListener);
      } else {
        paint(preferences.theme === "dark");
      }
    },
    current: () => ({ ...preferences, interfaceFontStack: [...preferences.interfaceFontStack] }),
    effectiveVariant: () => variant,
    dispose: detach,
  };
}

let defaultController: PreferenceAppearanceController | undefined;

export function getPreferenceAppearanceController(): PreferenceAppearanceController {
  if (!defaultController) defaultController = createPreferenceAppearanceController();
  return defaultController;
}

export function applyAppearancePreferences(settings: Partial<Settings>, locale: SupportedLocale): void {
  getPreferenceAppearanceController().apply(normalizeAppearancePreferences(settings), locale);
}
