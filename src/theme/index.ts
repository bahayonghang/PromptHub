/**
 * Theme module: light/dark/system theme application and persistence
 * (Requirement 22.1, 22.2, 22.4, 22.5).
 *
 * The visual switch is a single `.dark` class toggle on `document.documentElement`.
 * Because the design tokens in `src/styles/globals.css` are declared under both a
 * default (light) scope and a `.dark` scope, toggling that one class re-points
 * every token and repaints all rendered views without a restart (22.2).
 *
 * Like the Runtime_Bridge, the controller takes its DOM primitives via dependency
 * injection so it can be unit-tested in a non-browser environment. The production
 * entry points (`applyTheme`, `initializeTheme`, `setTheme`) bind to the live
 * `document`/`window` and the Runtime_Bridge.
 */
import { runtime, type RuntimeBridge } from "../runtime";

/** The user-selectable theme modes (mirrors `Settings.theme`). */
export type ThemeMode = "light" | "dark" | "system";

/** The media query used to detect the OS dark-mode preference for `system` mode. */
const PREFERS_DARK_QUERY = "(prefers-color-scheme: dark)";

/** The class toggled on the root element to activate the dark token scope. */
const DARK_CLASS = "dark";

/**
 * The theme applied before settings load and whenever a stored selection is
 * missing or unrecognized (Requirement 22.5).
 */
export const DEFAULT_THEME: ThemeMode = "dark";

/** The set of valid theme modes, used to validate values coming from settings. */
const THEME_MODES: readonly ThemeMode[] = ["light", "dark", "system"];

/** Narrows an arbitrary value to a {@link ThemeMode}, defaulting to dark (22.5). */
export function normalizeThemeMode(value: unknown): ThemeMode {
  return THEME_MODES.includes(value as ThemeMode) ? (value as ThemeMode) : DEFAULT_THEME;
}

/**
 * Resolves whether the dark token scope should be active for a given mode.
 * `light`/`dark` are explicit; `system` follows the OS preference.
 */
export function isDarkMode(mode: ThemeMode, systemPrefersDark: boolean): boolean {
  switch (mode) {
    case "light":
      return false;
    case "dark":
      return true;
    case "system":
      return systemPrefersDark;
  }
}

/** Minimal subset of `MediaQueryList` the controller relies on (DI-friendly). */
export interface MediaQuery {
  matches: boolean;
  addEventListener(type: "change", listener: (event: { matches: boolean }) => void): void;
  removeEventListener(type: "change", listener: (event: { matches: boolean }) => void): void;
}

/** Minimal subset of an element's class list the controller relies on. */
export interface ClassListLike {
  add(token: string): void;
  remove(token: string): void;
}

/** Injectable DOM primitives. Defaults bind to the live document/window. */
export interface ThemeDeps {
  /** The element whose class list carries the `.dark` scope. */
  root: { classList: ClassListLike };
  /** Resolves a media query (used to observe `prefers-color-scheme`). */
  matchMedia: (query: string) => MediaQuery;
}

/** Applies theme modes and manages the `system`-mode media subscription. */
export interface ThemeController {
  /**
   * Applies `mode`: toggles the `.dark` class to match, and (for `system`)
   * subscribes to `prefers-color-scheme` so OS changes re-toggle the class.
   * Re-applying replaces any previous `system` subscription.
   */
  apply(mode: ThemeMode): void;
  /** The mode most recently passed to {@link apply}. */
  current(): ThemeMode;
  /** Detaches any active `system`-mode media subscription. */
  dispose(): void;
}

/**
 * Creates a {@link ThemeController}. DOM primitives default to the live
 * `document.documentElement` and `window.matchMedia`; tests inject fakes.
 */
export function createThemeController(deps: Partial<ThemeDeps> = {}): ThemeController {
  const root = deps.root ?? document.documentElement;
  const matchMedia = deps.matchMedia ?? ((query: string) => window.matchMedia(query));

  let mode: ThemeMode = DEFAULT_THEME;
  let mediaQuery: MediaQuery | undefined;
  let mediaListener: ((event: { matches: boolean }) => void) | undefined;

  function paint(dark: boolean): void {
    if (dark) {
      root.classList.add(DARK_CLASS);
    } else {
      root.classList.remove(DARK_CLASS);
    }
  }

  function detach(): void {
    if (mediaQuery && mediaListener) {
      mediaQuery.removeEventListener("change", mediaListener);
    }
    mediaQuery = undefined;
    mediaListener = undefined;
  }

  function apply(next: ThemeMode): void {
    mode = next;
    // Drop any prior system-mode subscription before re-evaluating.
    detach();

    if (next === "system") {
      mediaQuery = matchMedia(PREFERS_DARK_QUERY);
      paint(mediaQuery.matches);
      mediaListener = (event) => paint(event.matches);
      mediaQuery.addEventListener("change", mediaListener);
    } else {
      paint(isDarkMode(next, false));
    }
  }

  return {
    apply,
    current: () => mode,
    dispose: detach,
  };
}

/** Lazily-created controller bound to the live DOM for the production entry points. */
let defaultController: ThemeController | undefined;
function getDefaultController(): ThemeController {
  if (!defaultController) {
    defaultController = createThemeController();
  }
  return defaultController;
}

/**
 * Applies `mode` to the live document (Requirement 22.2). For `system` mode this
 * also subscribes to `prefers-color-scheme` so OS theme changes are reflected
 * without a restart.
 */
export function applyTheme(mode: ThemeMode): void {
  getDefaultController().apply(mode);
}

/** Just the `invoke` slice of the Runtime_Bridge that theme persistence needs. */
type Invoke = RuntimeBridge["invoke"];

/**
 * Reads the persisted theme selection from Settings, defaulting to dark when no
 * (valid) selection exists (Requirement 22.5). Never throws on a missing/odd
 * value; it normalizes to {@link DEFAULT_THEME}.
 */
export async function loadThemeMode(invoke: Invoke): Promise<ThemeMode> {
  const settings = await invoke<{ theme?: unknown }>("settings.get");
  return normalizeThemeMode(settings?.theme);
}

/**
 * Persists the theme selection through `settings.update` (Requirement 22.2's
 * "stored in Settings"). The backend `Settings_Service.update` merges this
 * partial over the stored settings.
 */
export async function saveThemeMode(invoke: Invoke, mode: ThemeMode): Promise<void> {
  await invoke<unknown>("settings.update", { patch: { theme: mode } });
}

/**
 * Startup hook: load the persisted theme and apply it (Requirement 22.5). If the
 * settings read fails, the default (dark) theme is applied so the UI is never
 * left unthemed. Returns the applied mode.
 */
export async function initializeTheme(
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: ThemeController = getDefaultController(),
): Promise<ThemeMode> {
  let mode: ThemeMode;
  try {
    mode = await loadThemeMode(invoke);
  } catch {
    mode = DEFAULT_THEME;
  }
  controller.apply(mode);
  return mode;
}

/**
 * Applies a new theme selection and persists it (Requirement 22.2). The visual
 * change is applied first so the UI updates immediately; persistence follows.
 */
export async function setTheme(
  mode: ThemeMode,
  invoke: Invoke = runtime.invoke.bind(runtime),
  controller: ThemeController = getDefaultController(),
): Promise<void> {
  controller.apply(mode);
  await saveThemeMode(invoke, mode);
}
