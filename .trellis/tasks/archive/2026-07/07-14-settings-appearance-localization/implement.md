# Settings appearance and localization implementation plan

## Success Criteria

Implementation is complete only when the rebuilt desktop application satisfies
AC1-AC8 in `prd.md`, the full local gate passes, and restart persistence is
verified against the packaged binary rather than an old target artifact.

## Ordered Checklist

### 1. Lock the failing integration contracts

- [x] Add a mounted AppShell/SettingsView test that starts in English, selects
  Simplified Chinese, and proves shell plus active settings strings rerender.
- [x] Add a startup-order test with delayed settings/bundle promises that fails
  if React mounts before locale and appearance are ready.
- [x] Add a combined-controller regression that proves an OS color-mode event
  cannot override the selected theme family or restore a legacy mode.
- [x] Record a native reproduction from the current release executable and from
  a freshly built executable to distinguish stale packaging from source behavior.

### 2. Extend the persisted settings contract

- [x] Add `themeFamily`, `catppuccinDarkVariant`, and `interfaceFontStack` to the
  Rust and TypeScript settings mirrors as optional compatibility fields.
- [x] Add backend validation for bounded font arrays and string field types,
  preserving validate-before-write behavior.
- [x] Add round-trip, partial-update isolation, invalid-input, and legacy JSON
  tests in the settings service/storage test surfaces.
- [x] Add pure frontend migration/normalization tests for every legacy flavor and
  display/body font combination before changing runtime application.

### 3. Unify startup and appearance ownership

- [x] Introduce one awaited preference bootstrap that reads settings once, loads
  the selected locale bundle, applies appearance, and then mounts React.
- [x] Move `prefers-color-scheme` subscription and all root `.dark`/token writes
  into the appearance controller.
- [x] Remove `initializeTheme()` from `App.tsx` and retire the separate runtime
  theme controller only after compatibility tests pass.
- [x] Convert the theme token catalog to family + effective-mode resolution while
  preserving Catppuccin dark variants and current Claude palettes.
- [x] Add token completeness and WCAG AA contrast checks for every variant.

### 4. Make preference actions canonical and observable

- [x] Move language and appearance persistence orchestration into typed settings
  store actions backed by `SettingsApi`.
- [x] Apply immediate session previews, track per-control saving state, and
  reconcile success with the full `Settings` returned by the backend.
- [x] Represent known failures with localized keys and a retry path; retain the
  session preview but label it unsaved on rejection.
- [x] Set `document.documentElement.lang` and recompute locale-aware font
  fallbacks whenever the active locale changes.

### 5. Implement the font fallback contract

- [x] Replace display/body preferences with normalized `interfaceFontStack`
  application and migration.
- [x] Safely serialize family names, append locale/platform/generic fallbacks,
  and test missing-font behavior for Latin, zh, zh-TW, ja, and symbols.
- [x] Keep system-font enumeration behind the Runtime Bridge and expose explicit
  loading, empty, and error states instead of silently swallowing failures.
- [x] Remove preview-only display-font plumbing and keep prompt-editor monospace
  behavior unchanged.

### 6. Refine the settings information architecture

- [x] Move Language back to General and keep appearance-only controls in the
  Appearance section.
- [x] Add compact family tiles, color-mode segmented control, conditional
  Catppuccin dark variant, accent swatches, ordered font fallback controls, font
  scale, density, and one multilingual specimen.
- [x] Remove the redundant summary strip and constrain the settings content
  width; verify the 800 px minimum window and wide-window alignment.
- [x] Add accessible names, keyboard order, focus-visible, disabled, saving,
  saved, and error states to every changed control.
- [x] Update all seven locale bundles and the settings key-coverage test.

### 7. Verification and packaged smoke test

- [x] Run focused frontend tests while iterating:
  `npx vitest run src/runtime/i18n.test.ts src/runtime/i18n.property.test.ts src/appearance src/features/settings`.
- [x] Run focused backend tests:
  `cargo test settings --manifest-path src-tauri/Cargo.toml`.
- [x] Run `just build` and `just test` after frontend stabilization.
- [x] Run `just ci` for the final cross-boundary gate.
- [x] Run `just tauri-build`, launch the newly produced package, and verify
  locale/theme/font persistence across restart.
- [x] Capture and inspect 800x600, 1200x800, and wide-window screenshots in both
  Catppuccin and Claude families, including Simplified Chinese and a Latin locale.
- [x] Run `git diff --check` and review that no generated `dist/`, target output,
  read-only `ref/`, or unrelated Trellis runtime changes entered the implementation diff.

## Risky Files and Boundaries

- `src/main.tsx`: render timing; preserve a visible fallback on bootstrap failure.
- `src/runtime/i18n.ts`: singleton initialization and lazy-resource ordering.
- `src/appearance/index.ts` and `src/theme/index.ts`: shared root DOM ownership;
  change behind regressions, then remove the duplicate owner.
- `src/features/settings/settingsStore.ts`: canonical settings versus optimistic
  session state; do not duplicate backend writes.
- `src-tauri/src/models/settings.rs` and `src-tauri/src/services/settings.rs`:
  JSON compatibility and validate-before-write guarantees.
- `src/locales/*.json`: seven-bundle parity and stale contradictory copy.

## Review Gates

- Product gate: complete; theme family + color-mode semantics were approved on
  2026-07-14.
- Design-context gate: follow root `PRODUCT.md` and `DESIGN.md`; preserve the
  Prompt Workbench direction and the documented Signal Blue contrast pairs.
- Contract gate: approve legacy-field migration before removing any control.
- UX gate: review rebuilt desktop screenshots, not only jsdom output.
- Release gate: packaged restart persistence must pass before the old executable
  can be treated as replaced.
