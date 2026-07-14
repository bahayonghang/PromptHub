# Improve settings appearance and localization

## Goal

Unify PromptHub's language and appearance preferences so the packaged desktop
application starts in a deterministic state, applies changes immediately, and
restores them after restart. Preserve the existing Catppuccin and Claude theme
work, add an explicit font fallback contract, and remove settings that appear to
work but do not affect the real application.

## Confirmed Facts

- The supplied 2026-07-14 screenshot shows `简体中文` selected while most settings
  content remains English, and it has no Appearance entry in the settings rail.
- Current `main` does have an Appearance entry and panel
  (`src/features/settings/SettingsView.tsx:31`,
  `src/features/settings/SettingsView.tsx:138`). The local release executable
  predates the appearance commits, so the screenshot is most consistent with a
  stale packaged build. This must be verified with a rebuilt desktop artifact;
  the source alone does not reproduce the reported screen.
- Current source already includes Catppuccin Latte/Frappé/Macchiato/Mocha and
  Claude Light/Dark palettes (`src/appearance/index.ts:23`,
  `src/appearance/index.ts:228`). The task must consolidate and validate them,
  not add duplicate theme definitions.
- Current language selection delegates to `changeLocale()` and then updates the
  settings store (`src/features/settings/components/AppearancePanel.tsx:158`).
  All seven locale bundles have settings keys, and the existing i18n/appearance
  unit tests pass.
- Startup is not deterministic: `main.tsx` starts appearance and i18n without
  awaiting either (`src/main.tsx:17`, `src/main.tsx:22`), while `App.tsx` starts
  a separate legacy theme initializer (`src/App.tsx:16`).
- The legacy theme controller and appearance controller both own the root
  `.dark` class (`src/theme/index.ts:83`, `src/appearance/index.ts:455`). A
  completion-order or OS-theme event can therefore make color mode disagree
  with the selected flavor.
- The UI exposes display and body fonts, but `font-display` is only used by the
  settings preview. The actual application inherits `font-body`; the display
  font setting has no product-wide effect. Arbitrary fonts also receive only a
  hard-coded system tail (`src/appearance/index.ts:397`), not a user-controlled
  fallback chain.
- Settings are stored as one JSON document and merged by top-level key, so new
  optional preference fields do not require a SQLite schema migration
  (`src-tauri/src/services/settings.rs:4`,
  `src-tauri/src/services/settings.rs:82`).
- Planning baseline on 2026-07-14: `just build` passed, `just test` passed with
  222 tests, targeted i18n/appearance tests passed with 41 tests, and
  `cargo test settings --manifest-path src-tauri/Cargo.toml` passed with 13 tests.

## Product Decision

- Approved on 2026-07-14: Catppuccin is a theme family controlled by the global
  `light`/`dark`/`system` mode. Its light variant is Latte; its dark variant is
  selectable among Frappé, Macchiato, and Mocha. Claude remains the second
  family with light and dark variants. The current six fixed-mode flavor buttons
  will be migrated to this family-and-mode model.

## Requirements

### R1. One deterministic preferences bootstrap

- One owner must load persisted preferences, resolve the effective locale and
  appearance, subscribe to OS color-mode changes, and apply the result before
  mounting React.
- Only one controller may mutate root appearance state. The legacy theme and
  appearance controllers must not race or independently toggle `.dark`.
- Failure to read preferences must fall back to documented defaults without a
  blank screen, raw translation keys, or an unhandled rejection.

### R2. Reliable live language switching

- Language belongs in General settings, separate from visual appearance.
- Selecting any supported locale must update all mounted shell and settings
  strings immediately, set the document language, and persist the choice.
- Startup must await i18next initialization and bundle loading before the first
  application render, preventing an English flash or mixed-language screen.
- A persistence failure must be visible as a localized "applied for this
  session, not saved" state instead of silently diverging from stored settings.

### R3. Separate theme family from color mode

- Persisted color mode remains `light`, `dark`, or `system`; named theme family
  is a separate preference.
- The initial family catalog remains scoped to the already implemented
  Catppuccin and Claude themes. Catppuccin keeps Latte for light and preserves a
  selectable Frappé/Macchiato/Mocha dark variant; Claude supplies light and dark
  variants.
- `system` must follow `prefers-color-scheme` without changing the selected
  family or accent.
- Each theme variant must define the complete semantic token set consumed by
  the app and meet WCAG AA contrast for settings text, controls, and focus.

### R4. Replace placebo font controls with a real fallback stack

- Replace the display/body split with one interface font stack because the
  current display font does not affect application headings outside the preview.
- Users may select one primary family and a small ordered set of fallback
  families from built-in and detected system fonts. Raw CSS input is not
  allowed.
- The controller must safely quote family names, remove empty/duplicate entries,
  cap the list, and append locale-aware system/CJK fallbacks plus a generic
  `sans-serif` tail.
- Missing fonts must fall through without blank text, broken layout, or startup
  failure. The stack, font scale, and density must apply immediately and persist.

### R5. Coherent, accessible settings UX

- General contains language and behavior settings. Appearance contains theme
  family, color mode, optional Catppuccin dark variant, accent, interface font
  stack, font scale, density, and one live multilingual specimen.
- Use compact, familiar controls: segmented mode selection, theme radio tiles
  with real swatches, color swatches, selects/reorder controls for fonts, and
  switches for binary preferences.
- Constrain content width so switches do not sit at the far edge of a wide
  window. Keep the existing settings rail, but make the layout usable at the
  repository's 800 px minimum window width.
- Interactive controls need keyboard-visible focus, selected, disabled,
  loading, saved, and save-error states. Errors must be localized and associated
  with the affected control.

### R6. Compatibility and packaged-app verification

- Existing persisted `flavor`, `displayFont`, and `bodyFont` values must migrate
  deterministically to the new preference shape without resetting the user's
  effective appearance on first launch.
- Persistence continues through the Runtime Bridge and `settings.update`; no
  frontend component may import Tauri APIs directly.
- Completion requires a rebuilt Tauri application smoke test, not only Vite and
  unit tests, because the reported screenshot is likely from a stale package.

## Acceptance Criteria

- [ ] AC1: A rebuilt desktop app launched with persisted `zh` renders the shell
  and active settings panel in Simplified Chinese on first paint, with no mixed
  English settings labels.
- [ ] AC2: Switching `en -> zh -> ja -> en` updates all mounted shell/settings
  assertions immediately, persists each successful selection, updates the
  document language, and requires no refresh or restart.
- [ ] AC3: A failed locale or appearance write keeps the session preview but
  shows a localized unsaved state; retry stores the canonical backend result.
- [ ] AC4: Theme family and `light`/`dark`/`system` mode can be changed
  independently. An OS-mode event changes the effective variant only and never
  changes the stored family, accent, or Catppuccin dark variant.
- [ ] AC5: Legacy Latte/Frappé/Macchiato/Mocha and Claude Light/Dark settings
  migrate to the equivalent family, effective mode, and variant; invalid values
  normalize to documented defaults.
- [ ] AC6: A 1-4 family interface stack round-trips through the backend. Missing
  primary/CJK families fall through to readable Latin, Simplified Chinese,
  Traditional Chinese, Japanese, symbols, and generic sans-serif samples.
- [ ] AC7: The settings view is keyboard operable, focus-visible, non-overlapping,
  and readable at 800x600, 1200x800, and a wide desktop viewport. Theme token
  contrast checks pass for every supported effective variant.
- [ ] AC8: Targeted integration tests cover combined bootstrap order, a mounted
  locale switch, system-mode events, migration, font-stack normalization, and
  persistence failure/retry. `just ci` and `just tauri-build` pass, followed by
  a packaged-app restart smoke test.

## Out of Scope

- Downloading fonts or themes from remote sources.
- Arbitrary CSS, a user-authored theme editor, or plugin theme loading.
- Per-prompt typography or changing the prompt editor's monospace font.
- Adding theme families beyond Catppuccin and Claude in this task.
- Editing the read-only Electron reference implementation.
