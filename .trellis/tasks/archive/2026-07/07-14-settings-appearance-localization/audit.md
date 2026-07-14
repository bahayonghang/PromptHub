# Impeccable settings audit

Date: 2026-07-14
Target: `src/features/settings` plus its startup, appearance, theme, and shell dependencies
Mode: read-only source and supplied-release-screenshot audit

## Scorecard

| Dimension | Score | Main reason |
| --- | ---: | --- |
| Accessibility | 61/100 | Semantic controls exist, but focus treatment and asynchronous save feedback are incomplete, and one dark primary pair fails contrast. |
| Performance | 58/100 | Startup performs redundant settings reads, while system-font discovery repeats on panel remount and has no observable state. |
| Theming | 42/100 | Family and color mode are conflated, two controllers own the root dark class, and dark accents retain a white primary foreground. |
| Responsive design | 55/100 | The inner settings rail is fixed-width inside another fixed-width app rail, with no narrow-window structural adaptation or wide-content constraint. |
| Anti-patterns | 46/100 | The page duplicates its own selections, exposes a preview-only display font, and splits persistence ownership across panel helpers and the store. |

Overall: **52/100**. There are no P0 blockers, but the P1 items must be resolved
before the new appearance system can be considered release-ready.

## Findings

### P1 - Dark primary actions fail WCAG contrast

- Evidence: `src/appearance/index.ts:336-350` defines light Catppuccin accent
  values for dark mode, while `src/styles/globals.css:100` keeps
  `--primary-foreground` fixed at white. The Signal Blue pair is approximately
  `2.09:1`; using Mocha base ink `#1e1e2e` is approximately `7.85:1`.
- Impact: primary buttons and selected controls can become unreadable in dark
  Catppuccin themes.
- Required response: make foreground part of every resolved accent/theme token
  set and add automated AA contrast tests for every family, mode, and variant.

### P1 - Startup preference ownership races before first paint

- Evidence: `src/main.tsx:11-24` starts default theme, appearance, and locale
  initialization without awaiting either async path, then mounts React.
  `src/App.tsx:12-23` starts a second legacy theme initialization after mount.
  Both `src/theme/index.ts:102-130` and `src/appearance/index.ts:455-461` mutate
  the root `.dark` class.
- Impact: the last async completion wins, so persisted locale or appearance can
  flash, be overwritten, or disagree with the selected family after an OS mode
  event.
- Required response: one awaited bootstrap and one appearance controller must
  own root class, tokens, and the system color-scheme subscription.

### P1 - The persisted model cannot express the approved theme behavior

- Evidence: `src/appearance/index.ts:124-132` binds each flavor directly to a
  light or dark base, and `src/features/settings/components/AppearancePanel.tsx:165-188`
  presents those flavors as one selection group. Claude light/dark and the four
  Catppuccin flavors therefore mix family, mode, and dark variant in one value.
- Impact: selecting `system` independently from Catppuccin or Claude is
  impossible, and Catppuccin's dark variant cannot survive a light-mode switch.
- Required response: persist family, color mode, and Catppuccin dark variant as
  independent fields, with compatibility migration from every legacy flavor.

### P1 - Language switching is not proven at the application-shell level

- Evidence: the supplied release screenshot shows `简体中文` selected while
  the shell and settings copy remain English. Source code does call
  `i18n.changeLanguage` at `src/runtime/i18n.ts:169-175`, but `src/main.tsx:19-24`
  mounts before startup locale resolution, and current tests cover the i18n
  module/settings panel rather than an awaited AppShell bootstrap.
- Impact: a source-level helper can pass while the packaged user workflow still
  fails or ships from a stale artifact.
- Required response: add mounted shell integration coverage and compare the old
  executable with a freshly packaged binary before closing the defect.

### P1 - Settings become too narrow at the supported minimum window

- Evidence: the app shell uses a `w-60` (240px) primary sidebar when expanded at
  `src/components/layout/Sidebar.tsx:56-59`; settings adds a second fixed `w-56`
  (224px) rail at `src/features/settings/SettingsView.tsx:85-118`, followed by
  24px content padding at `src/features/settings/SettingsView.tsx:137`.
- Impact: an 800px window leaves roughly 288px for settings content before its
  own control gaps, while wide windows leave controls and switches visually
  disconnected across an unconstrained surface.
- Required response: add a narrow-window section navigation pattern and a
  deliberate content max-width/alignment strategy, verified at 800x600.

### P2 - Font settings promise behavior they do not deliver

- Evidence: display and body selectors are separate at
  `src/features/settings/components/AppearancePanel.tsx:227-275`, but the only
  production use of `.font-display` is the settings specimen at
  `src/features/settings/components/SpecimenCard.tsx:34`; the app body uses only
  `--font-body` at `src/styles/globals.css:145-166`. Arbitrary fonts receive a
  fixed fallback in `src/appearance/index.ts:387-400`.
- Impact: users can change a display font that does not affect the application,
  and cannot define the ordered fallback stack requested for multilingual UI.
- Required response: replace both fields with one normalized interface stack,
  append locale/platform/symbol fallbacks safely, and preserve editor monospace.

### P2 - System-font discovery is expensive and opaque

- Evidence: `src/features/settings/components/AppearancePanel.tsx:117-122`
  enumerates fonts on every panel mount and silently catches every failure.
- Impact: revisiting Appearance can repeat a native system scan, while users see
  neither loading, empty, nor error state.
- Required response: cache the discovery result behind the Runtime Bridge and
  expose explicit loading, empty, and retryable error states.

### P2 - Preference persistence has split ownership and weak feedback

- Evidence: appearance controls call field-specific setters and immediately
  merge local state at `src/features/settings/components/AppearancePanel.tsx:138-160`,
  while other settings use the canonical store action at
  `src/features/settings/settingsStore.ts:182-198`. Rejections become one raw
  global message and there is no per-control saving or unsaved state.
- Impact: overlapping writes can reconcile out of order, backend canonical
  values are not consistently adopted, and users cannot tell which preview is
  still unsaved.
- Required response: centralize language and appearance actions in the settings
  store, reconcile the full returned Settings object, and localize saving,
  saved, unsaved, and retry states.

### P2 - Information architecture and summary add avoidable cognitive load

- Evidence: language is placed inside Appearance at
  `src/features/settings/components/AppearancePanel.tsx:191-205`, while General
  explicitly excludes it at `src/features/settings/components/GeneralPanel.tsx:10-14`.
  `src/features/settings/components/AppearancePanel.tsx:319-327` then renders a
  preview and a summary of selections already visible immediately above.
- Impact: localization is harder to find and the summary consumes vertical
  space without helping a decision.
- Required response: return Language to General, retain one multilingual
  specimen, and remove the redundant summary strip.

### P2 - Interactive-state coverage is inconsistent

- Evidence: selection buttons and swatches at
  `src/features/settings/components/AppearancePanel.tsx:167-223` define hover and
  selected styling but no explicit `focus-visible`, disabled, saving, or error
  treatment. General switches at
  `src/features/settings/components/GeneralPanel.tsx:30-69` remain actionable
  before settings load and also omit explicit keyboard focus styling.
- Impact: keyboard location and asynchronous state are less legible than pointer
  state, especially during initial load or slow persistence.
- Required response: define consistent focus-visible, disabled, saving, saved,
  and error states for every changed control and verify keyboard order.

## Positive baseline

- Native `button`, `select`, section headings, `aria-pressed`, `aria-checked`,
  `aria-current`, and alert/status roles provide a usable semantic foundation.
- Locale bundles are lazy-loaded and English fallback behavior is explicit.
- Existing palette, normalization, and controller tests make the model migration
  safer than replacing the system without a regression harness.
- The implementation checklist already covers every required response above;
  no additional product scope is introduced by this audit.
