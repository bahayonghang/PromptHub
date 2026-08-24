/**
 * Appearance_Controller (Requirements 2, 3, 4, 5, 6, 11).
 *
 * Modeled on `src/theme/index.ts`: pure, dependency-injected, and unit-testable
 * in a non-browser environment. It separates normalization (pure functions) from
 * DOM application (a side-effecting controller) so property tests target the pure
 * layer while the apply layer is driven through an injected fake root.
 *
 * Like the Theme controller, the visual switch layers on top of the existing
 * token system (`src/styles/globals.css`): a flavor selects the light/dark base
 * (the `.dark` class) and writes a palette-override variable set onto the root;
 * the accent, fonts, font scale, and density write their own CSS variables. The
 * base scopes in globals.css still define the complete token set under both
 * bases — these overrides only re-point a known subset.
 */
import { runtime, type RuntimeBridge } from "../runtime";

// ===========================================================================
// Field value unions (Task 3.1)
// ===========================================================================

/** Named theme flavor; each maps to exactly one light/dark base (Req 2.1, 2.2). */
export type Flavor =
  | "Latte"
  | "Frappé"
  | "Macchiato"
  | "Mocha"
  | "Claude Light"
  | "Claude Dark"
  | "PromptHub Light"
  | "PromptHub Dark";

/** Named accent color (Req 3.1). */
export type AccentColor =
  | "Rosewater"
  | "Flamingo"
  | "Pink"
  | "Mauve"
  | "Red"
  | "Maroon"
  | "Peach"
  | "Yellow"
  | "Green"
  | "Teal"
  | "Sky"
  | "Sapphire"
  | "Blue"
  | "Lavender"
  | "Violet";

/** Font family from the fixed Font_Catalog (Req 4.1). */
export type FontFamilyName = "System" | "Inter" | "Space Grotesk" | "JetBrains Mono";

/** Discrete font scale preset (Req 5.1). */
export type FontScale = "Small" | "Default" | "Large" | "Extra Large";

/** Discrete spacing density preset (Req 6.1). */
export type Density = "Compact" | "Default" | "Comfortable";

/** The light/dark base a flavor renders under. */
export type AppearanceBase = "light" | "dark";

/** The fully-resolved appearance, every field a valid value. */
export interface Appearance {
  flavor: Flavor;
  accentColor: AccentColor;
  displayFont: string;
  bodyFont: string;
  fontScale: FontScale;
  density: Density;
}

// ===========================================================================
// Catalogs & defaults (Task 3.1)
// ===========================================================================

export const FLAVORS: readonly Flavor[] = [
  "Latte",
  "Frappé",
  "Macchiato",
  "Mocha",
  "Claude Light",
  "Claude Dark",
  "PromptHub Light",
  "PromptHub Dark",
];

export const ACCENT_COLORS: readonly AccentColor[] = [
  "Rosewater",
  "Flamingo",
  "Pink",
  "Mauve",
  "Red",
  "Maroon",
  "Peach",
  "Yellow",
  "Green",
  "Teal",
  "Sky",
  "Sapphire",
  "Blue",
  "Lavender",
  "Violet",
];

export const FONT_CATALOG: readonly FontFamilyName[] = [
  "System",
  "Inter",
  "Space Grotesk",
  "JetBrains Mono",
];

export const FONT_SCALES: readonly FontScale[] = ["Small", "Default", "Large", "Extra Large"];

export const DENSITIES: readonly Density[] = ["Compact", "Default", "Comfortable"];

export const DEFAULT_ACCENT: AccentColor = "Violet";
export const DEFAULT_FONT: FontFamilyName = "System";
export const DEFAULT_FONT_SCALE: FontScale = "Default";
export const DEFAULT_DENSITY: Density = "Default";

/** Percentage applied to the base text size for each preset (Req 5.1). */
export const FONT_SCALE_PERCENT: Record<FontScale, number> = {
  Small: 90,
  Default: 100,
  Large: 110,
  "Extra Large": 125,
};

/** Maps each flavor to its light/dark base; total over the six flavors (Req 2.2). */
export const FLAVOR_BASE: Record<Flavor, AppearanceBase> = {
  Latte: "light",
  Frappé: "dark",
  Macchiato: "dark",
  Mocha: "dark",
  "Claude Light": "light",
  "Claude Dark": "dark",
  "PromptHub Light": "light",
  "PromptHub Dark": "dark",
};

// ===========================================================================
// Normalization (pure, total — never throws; Task 3.2, Req 11)
// ===========================================================================

export function normalizeAccent(value: unknown): AccentColor {
  return (ACCENT_COLORS as readonly string[]).includes(value as string)
    ? (value as AccentColor)
    : DEFAULT_ACCENT;
}

/** Shared by both the display and body font fields. Accepts any non-empty string
 * (catalog values and arbitrary OS family names); defaults only on empty/invalid. */
export function normalizeFont(value: unknown): string {
  if (typeof value === "string" && value.trim() !== "") {
    return value;
  }
  return DEFAULT_FONT;
}

export function normalizeFontScale(value: unknown): FontScale {
  return (FONT_SCALES as readonly string[]).includes(value as string)
    ? (value as FontScale)
    : DEFAULT_FONT_SCALE;
}

export function normalizeDensity(value: unknown): Density {
  return (DENSITIES as readonly string[]).includes(value as string)
    ? (value as Density)
    : DEFAULT_DENSITY;
}

/**
 * A valid persisted flavor wins; otherwise the base is derived from the
 * Legacy_Theme: `light` -> Latte, anything else -> Mocha (Req 2.6-2.9).
 */
export function normalizeFlavor(value: unknown, legacyTheme?: unknown): Flavor {
  if ((FLAVORS as readonly string[]).includes(value as string)) {
    return value as Flavor;
  }
  return legacyTheme === "light" ? "Latte" : "Mocha";
}

/** Normalizes a raw (possibly partial/invalid) slice into a fully valid Appearance. */
export function normalizeAppearance(
  raw: Partial<Record<keyof Appearance, unknown>>,
  legacyTheme?: unknown,
): Appearance {
  return {
    flavor: normalizeFlavor(raw.flavor, legacyTheme),
    accentColor: normalizeAccent(raw.accentColor),
    displayFont: normalizeFont(raw.displayFont),
    bodyFont: normalizeFont(raw.bodyFont),
    fontScale: normalizeFontScale(raw.fontScale),
    density: normalizeDensity(raw.density),
  };
}

// ===========================================================================
// Value maps consumed during DOM application (Task 4.1)
// ===========================================================================

/** The accent token variables written for the active accent color. */
export const ACCENT_TOKENS = [
  "--primary",
  "--primary-foreground",
  "--ring",
  "--accent",
  "--accent-foreground",
] as const;

/** Surface/text/border tokens every flavor override set re-points. */
export const FLAVOR_PALETTE_TOKENS = [
  "--background",
  "--foreground",
  "--card",
  "--card-foreground",
  "--popover",
  "--popover-foreground",
  "--muted",
  "--muted-foreground",
  "--secondary",
  "--secondary-foreground",
  "--border",
  "--input",
  "--sidebar",
  "--sidebar-foreground",
  "--sidebar-accent",
  "--sidebar-border",
] as const;

/** A small set of palette swatches, given as HSL channel triplets `H S% L%`. */
interface FlavorSwatches {
  base: string;
  mantle: string;
  surface0: string;
  surface1: string;
  text: string;
  subtext0: string;
}

/**
 * Catppuccin Latte/Frappé/Macchiato/Mocha palettes converted to HSL; Claude
 * Light/Claude Dark use a tasteful warm-neutral light/dark palette.
 */
const FLAVOR_SWATCHES: Record<Flavor, FlavorSwatches> = {
  Latte: {
    base: "220 23% 95%",
    mantle: "220 22% 92%",
    surface0: "223 16% 83%",
    surface1: "225 14% 77%",
    text: "234 16% 35%",
    subtext0: "233 10% 37%",
  },
  Frappé: {
    base: "229 19% 23%",
    mantle: "231 19% 20%",
    surface0: "230 16% 30%",
    surface1: "227 15% 37%",
    text: "227 70% 87%",
    subtext0: "228 29% 75%",
  },
  Macchiato: {
    base: "232 23% 18%",
    mantle: "233 23% 15%",
    surface0: "230 19% 26%",
    surface1: "231 16% 34%",
    text: "227 68% 88%",
    subtext0: "227 27% 72%",
  },
  Mocha: {
    base: "240 21% 15%",
    mantle: "240 21% 12%",
    surface0: "237 16% 23%",
    surface1: "234 13% 31%",
    text: "226 64% 88%",
    subtext0: "228 24% 72%",
  },
  "Claude Light": {
    base: "48 33% 97%",
    mantle: "0 0% 100%",
    surface0: "48 16% 90%",
    surface1: "45 13% 82%",
    text: "40 6% 16%",
    subtext0: "45 5% 40%",
  },
  "Claude Dark": {
    base: "40 4% 15%",
    mantle: "40 4% 18%",
    surface0: "40 4% 22%",
    surface1: "45 4% 28%",
    text: "48 17% 90%",
    subtext0: "45 6% 64%",
  },
  "PromptHub Light": {
    base: "220 37% 97%",
    mantle: "0 0% 100%",
    surface0: "227 36% 95%",
    surface1: "225 27% 91%",
    text: "223 24% 11%",
    subtext0: "222 13% 41%",
  },
  "PromptHub Dark": {
    base: "225 27% 6%",
    mantle: "225 25% 9%",
    surface0: "224 24% 15%",
    surface1: "223 22% 19%",
    text: "225 33% 93%",
    subtext0: "223 16% 65%",
  },
};

/** Builds the complete flavor palette-override set from its swatches. */
function buildFlavorOverrides(s: FlavorSwatches): Record<string, string> {
  return {
    "--background": s.base,
    "--foreground": s.text,
    "--card": s.mantle,
    "--card-foreground": s.text,
    "--popover": s.mantle,
    "--popover-foreground": s.text,
    "--muted": s.surface0,
    "--muted-foreground": s.subtext0,
    "--secondary": s.surface0,
    "--secondary-foreground": s.text,
    "--border": s.surface1,
    "--input": s.surface1,
    "--sidebar": s.mantle,
    "--sidebar-foreground": s.text,
    "--sidebar-accent": s.surface0,
    "--sidebar-border": s.surface1,
  };
}

/** Flavor -> palette override variables (re-pointed surface/text/border tokens). */
export const FLAVOR_OVERRIDES: Record<Flavor, Record<string, string>> = {
  Latte: buildFlavorOverrides(FLAVOR_SWATCHES.Latte),
  Frappé: buildFlavorOverrides(FLAVOR_SWATCHES.Frappé),
  Macchiato: buildFlavorOverrides(FLAVOR_SWATCHES.Macchiato),
  Mocha: buildFlavorOverrides(FLAVOR_SWATCHES.Mocha),
  "Claude Light": buildFlavorOverrides(FLAVOR_SWATCHES["Claude Light"]),
  "Claude Dark": buildFlavorOverrides(FLAVOR_SWATCHES["Claude Dark"]),
  "PromptHub Light": buildFlavorOverrides(FLAVOR_SWATCHES["PromptHub Light"]),
  "PromptHub Dark": buildFlavorOverrides(FLAVOR_SWATCHES["PromptHub Dark"]),
};

/**
 * Accent hue/saturation/lightness per base: Latte values under the light base,
 * Mocha values under the dark base (HSL channels `[h, s, l]`).
 */
const ACCENT_HSL: Record<AppearanceBase, Record<AccentColor, [number, number, number]>> = {
  light: {
    Rosewater: [11, 59, 67],
    Flamingo: [0, 60, 67],
    Pink: [316, 73, 69],
    Mauve: [266, 85, 58],
    Red: [347, 87, 44],
    Maroon: [355, 76, 59],
    Peach: [22, 99, 52],
    Yellow: [35, 77, 49],
    Green: [109, 58, 40],
    Teal: [183, 74, 35],
    Sky: [197, 97, 46],
    Sapphire: [189, 70, 42],
    Blue: [220, 91, 54],
    Lavender: [231, 97, 72],
    Violet: [247, 67, 59],
  },
  dark: {
    Rosewater: [10, 56, 91],
    Flamingo: [0, 59, 88],
    Pink: [316, 72, 86],
    Mauve: [267, 84, 81],
    Red: [343, 81, 75],
    Maroon: [350, 65, 77],
    Peach: [23, 92, 75],
    Yellow: [41, 86, 83],
    Green: [115, 54, 76],
    Teal: [170, 57, 73],
    Sky: [189, 71, 73],
    Sapphire: [199, 76, 69],
    Blue: [217, 92, 76],
    Lavender: [232, 97, 85],
    Violet: [248, 89, 73],
  },
};

/** Derives the four accent token values for one accent under one base. */
function relativeLuminance([h, s, l]: [number, number, number]): number {
  const hue = h / 360;
  const saturation = s / 100;
  const lightness = l / 100;
  const channel = (p: number, q: number, value: number) => {
    let t = value;
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  const rgb: [number, number, number] = saturation === 0
    ? [lightness, lightness, lightness]
    : (() => {
        const q = lightness < 0.5
          ? lightness * (1 + saturation)
          : lightness + saturation - lightness * saturation;
        const p = 2 * lightness - q;
        return [
          channel(p, q, hue + 1 / 3),
          channel(p, q, hue),
          channel(p, q, hue - 1 / 3),
        ];
      })();
  const linear = rgb.map((value) =>
    value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
  );
  return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
}

function contrastRatio(a: [number, number, number], b: [number, number, number]): number {
  const [lighter, darker] = [relativeLuminance(a), relativeLuminance(b)].sort((x, y) => y - x);
  return (lighter + 0.05) / (darker + 0.05);
}

function buildAccentSet(base: AppearanceBase, color: [number, number, number]): Record<string, string> {
  const [h, s, l] = color;
  const primary = `${h} ${s}% ${l}%`;
  const darkForeground: [number, number, number] = [240, 21, 15];
  const lightForeground: [number, number, number] = [0, 0, 100];
  const darkContrast = contrastRatio(color, darkForeground);
  const lightContrast = contrastRatio(color, lightForeground);
  let primaryForeground = darkContrast >= lightContrast
    ? "240 21% 15%"
    : "0 0% 100%";
  if (Math.max(darkContrast, lightContrast) < 4.5) {
    primaryForeground = "240 21% 5%";
  }
  const focusSurface: [number, number, number] = base === "light" ? [220, 23, 95] : [229, 19, 23];
  const ringColor: [number, number, number] = contrastRatio(color, focusSurface) >= 3
    ? color
    : base === "light"
      ? [h, Math.max(s, 40), 30]
      : [h, s, 75];
  const ring = `${ringColor[0]} ${ringColor[1]}% ${ringColor[2]}%`;
  if (base === "light") {
    return {
      "--primary": primary,
      "--primary-foreground": primaryForeground,
      "--ring": ring,
      "--accent": `${h} ${Math.round(s * 0.4)}% 92%`,
      "--accent-foreground": `${h} ${s}% 35%`,
    };
  }
  return {
    "--primary": primary,
    "--primary-foreground": primaryForeground,
    "--ring": ring,
    "--accent": `${h} ${Math.round(s * 0.3)}% 18%`,
    "--accent-foreground": `${h} ${s}% 85%`,
  };
}

function buildAccentPalette(base: AppearanceBase): Record<AccentColor, Record<string, string>> {
  const out = {} as Record<AccentColor, Record<string, string>>;
  for (const accent of ACCENT_COLORS) {
    out[accent] = buildAccentSet(base, ACCENT_HSL[base][accent]);
  }
  return out;
}

/** Accent color x base -> accent token variables (Req 3.2). Every entry present. */
export const ACCENT_PALETTE: Record<AppearanceBase, Record<AccentColor, Record<string, string>>> = {
  light: buildAccentPalette("light"),
  dark: buildAccentPalette("dark"),
};

/** Font family -> CSS font stack. `System` maps to the OS UI font stack. */
export const FONT_STACK: Record<FontFamilyName, string> = {
  System:
    'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
  Inter: '"Inter", ui-sans-serif, system-ui, sans-serif',
  "Space Grotesk": '"Space Grotesk", ui-sans-serif, system-ui, sans-serif',
  "JetBrains Mono": '"JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
};

/** Resolves a font name to its CSS font-family stack (known or arbitrary). */
export function fontStack(name: string): string {
  if (name in FONT_STACK) return FONT_STACK[name as FontFamilyName];
  return `"${name}", ui-sans-serif, system-ui, sans-serif`;
}

/** Density -> spacing rhythm variables (Req 6.2). */
export const DENSITY_RHYTHM: Record<Density, { "--density-padding": string; "--density-gap": string }> = {
  Compact: { "--density-padding": "0.375rem", "--density-gap": "0.5rem" },
  Default: { "--density-padding": "0.5rem", "--density-gap": "0.75rem" },
  Comfortable: { "--density-padding": "0.75rem", "--density-gap": "1rem" },
};

// ===========================================================================
// DOM application (side-effecting controller; Task 5.1)
// ===========================================================================

/** The class toggled on the root element to activate the dark token scope. */
const DARK_CLASS = "dark";

/** The appearance applied before settings load and as the controller's initial state. */
export const DEFAULT_APPEARANCE: Appearance = normalizeAppearance({});

/** Minimal subset of an element's class list the controller relies on. */
export interface ClassListLike {
  add(token: string): void;
  remove(token: string): void;
}

/** Minimal CSS custom-property target the controller relies on. */
export interface CssVarTarget {
  setProperty(name: string, value: string): void;
}

/** Injectable DOM primitives. Defaults bind to `document.documentElement`. */
export interface AppearanceDeps {
  root: { classList: ClassListLike; style: CssVarTarget; setAttribute(name: string, value: string): void };
}

/** Applies appearance values to the DOM, one field at a time or all at once. */
export interface AppearanceController {
  /** Applies a fully-normalized appearance (base class + CSS variables). */
  apply(appearance: Appearance): void;
  /** Applies a single field, leaving the others untouched. */
  applyField<K extends keyof Appearance>(field: K, value: Appearance[K]): void;
  /** The appearance most recently applied. */
  current(): Appearance;
}

export function createAppearanceController(deps: Partial<AppearanceDeps> = {}): AppearanceController {
  const root = deps.root ?? document.documentElement;
  let current: Appearance = { ...DEFAULT_APPEARANCE };

  function setVars(vars: Record<string, string>): void {
    for (const [name, value] of Object.entries(vars)) {
      root.style.setProperty(name, value);
    }
  }

  function applyFlavor(flavor: Flavor): void {
    if (FLAVOR_BASE[flavor] === "dark") {
      root.classList.add(DARK_CLASS);
    } else {
      root.classList.remove(DARK_CLASS);
    }
    setVars(FLAVOR_OVERRIDES[flavor]);
  }

  function applyAccent(accent: AccentColor): void {
    // The accent palette depends on the active base, derived from the flavor.
    setVars(ACCENT_PALETTE[FLAVOR_BASE[current.flavor]][accent]);
  }

  function applyDisplayFont(font: string): void {
    root.style.setProperty("--font-display", fontStack(font));
  }

  function applyBodyFont(font: string): void {
    root.style.setProperty("--font-body", fontStack(font));
  }

  function applyFontScale(scale: FontScale): void {
    // Unitless multiplier, e.g. Large (110%) -> "1.1".
    root.style.setProperty("--font-scale", String(FONT_SCALE_PERCENT[scale] / 100));
  }

  function applyDensity(density: Density): void {
    const rhythm = DENSITY_RHYTHM[density];
    root.style.setProperty("--density-padding", rhythm["--density-padding"]);
    root.style.setProperty("--density-gap", rhythm["--density-gap"]);
    root.setAttribute("data-density", density);
  }

  function apply(appearance: Appearance): void {
    current = { ...appearance };
    applyFlavor(appearance.flavor);
    applyAccent(appearance.accentColor);
    applyDisplayFont(appearance.displayFont);
    applyBodyFont(appearance.bodyFont);
    applyFontScale(appearance.fontScale);
    applyDensity(appearance.density);
  }

  function applyField<K extends keyof Appearance>(field: K, value: Appearance[K]): void {
    const next = { ...current };
    next[field] = value;
    current = next;
    switch (field) {
      case "flavor":
        applyFlavor(current.flavor);
        // The base may have flipped, so re-point the base-dependent accent tokens.
        applyAccent(current.accentColor);
        break;
      case "accentColor":
        applyAccent(current.accentColor);
        break;
      case "displayFont":
        applyDisplayFont(current.displayFont);
        break;
      case "bodyFont":
        applyBodyFont(current.bodyFont);
        break;
      case "fontScale":
        applyFontScale(current.fontScale);
        break;
      case "density":
        applyDensity(current.density);
        break;
    }
  }

  return {
    apply,
    applyField,
    current: () => ({ ...current }),
  };
}

// ===========================================================================
// Production entry points (bind to live DOM + Runtime_Bridge; Task 6.1)
// ===========================================================================

/** Just the `invoke` slice of the Runtime_Bridge that persistence needs. */
type Invoke = RuntimeBridge["invoke"];

/** The settings slice the appearance fields are read from at startup. */
interface AppearanceSettingsSlice {
  theme?: unknown;
  flavor?: unknown;
  accentColor?: unknown;
  displayFont?: unknown;
  bodyFont?: unknown;
  fontScale?: unknown;
  density?: unknown;
}

let defaultController: AppearanceController | undefined;
function getDefaultController(): AppearanceController {
  if (!defaultController) {
    defaultController = createAppearanceController();
  }
  return defaultController;
}

/**
 * Startup hook: read persisted settings, normalize each field (with the legacy
 * theme as the flavor-base fallback), and apply. On a read failure the full
 * default appearance is applied so the UI is never left unstyled (best-effort,
 * never throws). Returns the applied appearance.
 */
export async function initializeAppearance(
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<Appearance> {
  let appearance: Appearance;
  try {
    const settings = await invoke<AppearanceSettingsSlice>("settings.get");
    appearance = normalizeAppearance(
      {
        flavor: settings?.flavor,
        accentColor: settings?.accentColor,
        displayFont: settings?.displayFont,
        bodyFont: settings?.bodyFont,
        fontScale: settings?.fontScale,
        density: settings?.density,
      },
      settings?.theme,
    );
  } catch {
    appearance = { ...DEFAULT_APPEARANCE };
  }
  controller.apply(appearance);
  return appearance;
}

/**
 * Applies a single field to the controller first (instant visual change), then
 * best-effort persists it. A rejecting bridge does not roll back the applied
 * value and no exception escapes (Req 5.6); the panel/store surfaces the failure.
 */
async function applyAndPersist<K extends keyof Appearance>(
  field: K,
  value: Appearance[K],
  invoke: Invoke,
  controller: AppearanceController,
): Promise<void> {
  controller.applyField(field, value);
  try {
    await invoke<unknown>("settings.update", { patch: { [field]: value } });
  } catch {
    // Persistence failed: keep the applied value for the session; swallow so no
    // unhandled error escapes the set* flow.
  }
}

export function setFlavor(
  value: Flavor,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<void> {
  return applyAndPersist("flavor", value, invoke, controller);
}

export function setAccentColor(
  value: AccentColor,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<void> {
  return applyAndPersist("accentColor", value, invoke, controller);
}

export function setDisplayFont(
  value: string,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<void> {
  return applyAndPersist("displayFont", value, invoke, controller);
}

export function setBodyFont(
  value: string,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<void> {
  return applyAndPersist("bodyFont", value, invoke, controller);
}

export function setFontScale(
  value: FontScale,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<void> {
  return applyAndPersist("fontScale", value, invoke, controller);
}

export function setDensity(
  value: Density,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: AppearanceController = getDefaultController(),
): Promise<void> {
  return applyAndPersist("density", value, invoke, controller);
}
