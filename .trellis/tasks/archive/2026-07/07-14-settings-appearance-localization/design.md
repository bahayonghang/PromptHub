# Settings appearance and localization design

## Status

Theme semantics approved by the user on 2026-07-14: theme family is independent
from color mode, and Catppuccin retains a dark-variant choice. Implementation
started on 2026-07-14 after the read-only Impeccable audit was recorded.

## Current Data Flow and Failure Modes

```text
main.tsx
  -> applyTheme(default) synchronously
  -> initializeAppearance() fire-and-forget -> settings.get -> root tokens/.dark
  -> initI18n() fire-and-forget -> settings.get -> bundle/changeLanguage
  -> render React immediately

App mount
  -> initializeTheme() fire-and-forget -> settings.get -> root .dark + OS listener

AppearancePanel
  -> appearance set*() -> immediate DOM update -> settings.update
  -> changeLocale() -> bundle/changeLanguage -> settings.update
  -> optimistic mergeLocalSettings()
```

This has four concrete defects:

1. First render is not ordered after locale and appearance resolution.
2. Two controllers write the same `.dark` state and install independent behavior.
3. UI state is merged optimistically instead of being reconciled with the
   canonical `Settings` returned by the backend.
4. `displayFont` is only demonstrated by the specimen and has no application
   consumer, while fallback behavior is fixed inside `fontStack()`.

## Target Architecture

```text
bootstrapPreferences()
  -> settings.get once
  -> await i18n initialization + selected bundle
  -> resolve normalized AppearancePreferences
  -> AppearanceController.apply(resolved)
  -> install one prefers-color-scheme listener when mode=system
  -> set document.documentElement.lang
  -> mount React

SettingsStore preference actions
  -> normalize proposed value
  -> apply immediate session preview
  -> mark field saving
  -> settings.update through SettingsApi
  -> replace local state with canonical backend Settings
  -> clear saving / show localized unsaved state on failure
```

The Runtime Bridge remains the only backend boundary. Controllers become DOM
application modules; the settings store/API owns persistence and error state.

## Preference Contract

Keep the existing wire field `theme` as color mode to avoid needless persisted
data churn. Add optional fields to the JSON settings document:

```ts
type ColorMode = "light" | "dark" | "system";
type ThemeFamily = "catppuccin" | "claude";
type CatppuccinDarkVariant = "frappe" | "macchiato" | "mocha";

interface AppearancePreferences {
  theme: ColorMode;
  themeFamily: ThemeFamily;
  catppuccinDarkVariant: CatppuccinDarkVariant;
  accentColor: AccentColor;
  interfaceFontStack: string[];
  fontScale: FontScale;
  density: Density;
}
```

Rust mirrors the three new optional fields using `Option<String>` and
`Option<Vec<String>>`. No SQLite DDL changes are needed because settings remain
one JSON value. Backend validation is structural and bounded; frontend
normalizers remain total so corrupt or older settings cannot block rendering.

### Theme resolution

1. Resolve effective mode from explicit `light`/`dark`, or the media query for
   `system`.
2. Resolve variant from family + effective mode:
   - Catppuccin light -> Latte.
   - Catppuccin dark -> selected Frappé/Macchiato/Mocha, default Mocha.
   - Claude light/dark -> existing Claude Light/Claude Dark token sets.
3. Apply complete surface, text, border, focus, semantic status, and native
   `color-scheme` tokens from one catalog.
4. Apply accent overrides after base tokens.

Only `AppearanceController` may toggle `.dark`, write theme CSS variables, or
subscribe to `prefers-color-scheme`. Remove the runtime use of the separate
theme controller after compatibility tests pass.

### Font stack

`interfaceFontStack` stores family names, not CSS. Normalize it as follows:

- Trim, remove empty entries, de-duplicate case-insensitively, and cap at four.
- Allow built-in sentinels and names returned by system-font discovery.
- Escape quotes, slashes, and control characters before producing CSS values.
- Append a locale-aware fallback set for zh/zh-TW/ja, then the platform UI
  families and generic `sans-serif` tail.
- Recompute the effective CSS stack when either locale or font preference
  changes.

The global body consumes `--font-interface`. Remove the display-font control and
`font-display` preview-only contract. Keep the prompt editor's existing
`font-mono` behavior out of this task.

## UI Structure

### General

- Language selector with localized language names and inline save status.
- Auto-save and startup behavior switches.

### Appearance

- Compact theme-family radio tiles with representative token swatches.
- Light/Dark/System segmented control with familiar icons and text.
- Catppuccin dark-variant menu shown only for that family.
- Accent swatches.
- Primary font select plus an ordered fallback list (maximum three additional
  entries), with add/remove/reorder controls and loading/error state for system
  font discovery.
- Font scale and density segmented controls.
- One multilingual live specimen; remove the redundant summary strip.

Use an unframed, width-constrained settings column. The live specimen is the one
genuinely framed preview. At the 800 px minimum width, keep labels and controls
inside the content area and avoid full-row `justify-between` spacing that pushes
switches to the far window edge.

## I18n Lifecycle

- Replace the boolean `initialized` flag with a shared initialization promise.
- Await i18next initialization before adding lazy bundles or changing language.
- Await preference bootstrap before React mount while keeping synchronous base
  colors as a no-blank fallback.
- Move language handling to a settings-store action and set the root `lang`
  attribute after a successful in-memory switch.
- Store error codes/translation keys, not backend English messages, for known
  preference failures. Preserve diagnostic details outside visible copy.

## Compatibility and Migration

When the new fields are absent, resolve legacy values without writing during
startup:

| Legacy value | New family | Mode | Dark variant |
| --- | --- | --- | --- |
| Latte | catppuccin | light | mocha default |
| Frappé | catppuccin | dark | frappe |
| Macchiato | catppuccin | dark | macchiato |
| Mocha | catppuccin | dark | mocha |
| Claude Light | claude | light | unchanged/ignored |
| Claude Dark | claude | dark | unchanged/ignored |

`bodyFont` becomes the first `interfaceFontStack` entry. If it is missing,
`displayFont` is a last-resort legacy source; otherwise System is used. Keep the
legacy optional fields readable for one compatibility cycle. The first explicit
preference save writes the new fields and may clear legacy fields with `null`.

Rollback remains safe: older builds ignore unknown optional JSON fields and can
still read the retained legacy fields. Do not remove legacy fields in the same
release that introduces the new contract.

## Verification Design

- Unit tests: locale/theme/font normalizers, safe CSS serialization, theme token
  completeness and contrast, legacy migration, system event handling.
- Component integration: render AppShell + SettingsView, switch language, and
  assert shell plus active panel text changes; verify save/error/retry states.
- Backend: settings patch round trips for the new fields, invalid arrays rejected
  before write, legacy JSON remains readable.
- Startup integration: delayed promises prove React mounts only after selected
  locale and appearance are applied; no second controller overwrites them.
- Native smoke: build and launch the packaged app, change locale/theme/font,
  restart, and verify the same state. Capture 800x600, 1200x800, and wide-window
  screenshots for layout and overlap review.

## Rollback Points

- Commit contract and migration tests before removing any legacy runtime path.
- Keep the old theme fields readable until the packaged smoke test passes.
- If native system-font discovery is unstable, ship built-in + System choices
  with the same fallback contract; do not fall back to raw CSS input.
