# Design — design tokens and typography baseline

## Finding that reshapes this task

The PRD assumed the token layer is `src/styles/globals.css`. It is not the layer
that decides what the user sees.

The live paint path is `createPreferenceAppearanceController` in
`src/appearance/preferences.ts:231-290`. `paint()` (`:251-270`) runs on every
startup from `src/main.tsx:10` and writes onto `document.documentElement`:

| Written                          | Source                                                 | Count |
| -------------------------------- | ------------------------------------------------------ | ----- |
| `.dark` class                    | `FLAVOR_BASE[variant]` (`preferences.ts:253-255`)      | —     |
| surface / text / border tokens   | `FLAVOR_OVERRIDES[variant]` (`preferences.ts:256`)     | 16    |
| accent tokens                    | `ACCENT_PALETTE[base][accent]` (`preferences.ts:257`)  | 5     |
| `--font-interface`               | `resolveInterfaceFontStack` (`preferences.ts:258-261`) | 1     |
| `--font-scale`, density          | `preferences.ts:262-263`                               | 3     |
| `color-scheme`, `data-*`, `lang` | `preferences.ts:264-269`                               | —     |

Inline styles on the root beat any `@layer base` rule. So the 21 color tokens
that `FLAVOR_OVERRIDES` and `ACCENT_PALETTE` cover are decided by the appearance
system, not by `globals.css`.

The out-of-box result is `themeFamily: "catppuccin"` + `catppuccinDarkVariant:
"mocha"` + `accentColor: "Blue"` (`preferences.ts:44-46`, `index.ts:112`), which
paints Catppuccin Mocha with a blue accent. Editing `globals.css` alone changes
none of it.

`globals.css` still matters. It defines the complete token set before the
controller runs, and it owns every token the controller does not write —
`--destructive`, `--shadow-*`, `--radius`, and anything new.

There is a second, older controller in `src/appearance/index.ts:490-600`
(`applyFlavor`, `applyDisplayFont`, `applyBodyFont`). Nothing outside
`src/appearance/**` imports it, and `src/theme/index.ts` has no importers
either. Both are superseded by `preferences.ts`. This task does not delete
them; it keeps their data tables correct, because `preferences.ts` imports
`FLAVOR_BASE`, `FLAVOR_OVERRIDES`, `ACCENT_PALETTE`, `DENSITY_RHYTHM`, and
`FONT_SCALE_PERCENT` from `index.ts:19`.

## Decisions

### D1 — The design palette enters as a new theme family

Add `themeFamily: "prompthub"` with two variants, `"PromptHub Light"` and
`"PromptHub Dark"`, and make it the default family.

Concretely:

- `Flavor` (`index.ts:22-29`) gains `"PromptHub Light" | "PromptHub Dark"`.
- `FLAVORS` (`index.ts:71-78`), `FLAVOR_SWATCHES` (`index.ts:238-286`),
  and `FLAVOR_BASE` (`index.ts:125`) gain matching entries.
- `ThemeFamily` (`preferences.ts:22`) gains `"prompthub"`; `THEME_FAMILIES`
  (`:37`) lists it; `DEFAULT_THEME_FAMILY` (`:44`) becomes `"prompthub"`.
- `resolveThemeVariant` (`preferences.ts:130-149`) returns the two new variants
  for that family.
- `LEGACY_FLAVOR_MIGRATION` (`preferences.ts:48-58`) is `Record<Flavor, …>`, so
  the compiler forces entries for the two new variants.

Rejected alternatives:

- **Overwrite the Catppuccin swatches in place.** Silently changes what the
  "Catppuccin" label means and removes a shipped user choice with no way back.
- **Stop the controller writing palette overrides.** Breaks the accent picker,
  the light/dark/system mode, and the OS-preference listener at
  `preferences.ts:277-284`.

`FlavorSwatches` (`index.ts:227-234`) carries six slots: `base`, `mantle`,
`surface0`, `surface1`, `text`, `subtext0`. `buildFlavorOverrides`
(`index.ts:289-311`) expands them into the 16 tokens. The design has more
distinct greys than six slots, so the mapping loses `--surf2` and `--faint`.
Chosen mapping:

| Swatch     | Dark                      | Light                     | Feeds                                                                                                         |
| ---------- | ------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `base`     | `225 27% 6%` (`#0b0d13`)  | `220 37% 97%` (`#f4f6fa`) | `--background`                                                                                                |
| `mantle`   | `225 25% 9%` (`#12151e`)  | `0 0% 100%` (`#ffffff`)   | `--card`, `--popover`, `--sidebar`                                                                            |
| `surface0` | `224 24% 15%` (`#1e2331`) | `227 36% 95%` (`#eef0f7`) | `--muted`, `--secondary`, `--sidebar-accent`                                                                  |
| `surface1` | `223 22% 19%` (`#252b3a`) | `225 27% 91%` (`#e3e6ef`) | `--border`, `--input`, `--sidebar-border`                                                                     |
| `text`     | `225 33% 93%` (`#e7eaf3`) | `223 24% 11%` (`#161a24`) | `--foreground`, `--card-foreground`, `--popover-foreground`, `--secondary-foreground`, `--sidebar-foreground` |
| `subtext0` | `223 16% 65%` (`#98a0b5`) | `222 13% 41%` (`#5b6376`) | `--muted-foreground`                                                                                          |

The two design values the swatch set cannot carry are added as their own
tokens in `globals.css`, outside the flavor override set:

- `--surface-inset` — dark `224 25% 12%` (`#171b26`), light `228 45% 98%`
  (`#f7f8fc`). The design's `--surf2`, used for inset panels and textareas.
- `--muted-foreground-subtle` — dark `222 11% 47%` (`#6a7285`), light
  `222 13% 60%` (`#8b93a6`). The design's `--faint`, used for counts,
  timestamps, and column headers.
- `--border-strong` — dark `223 21% 23%` (`#2f3648`), light `224 28% 86%`
  (`#d3d8e6`). The design's `--line2`.

These three are not flavor-scoped, so under a Catppuccin or Claude family they
keep the design's values. That is a known cosmetic inconsistency, recorded here
rather than fixed, because widening `FlavorSwatches` would force six new values
for all six existing flavors — work this task does not own.

### D2 — The design accent enters as a new accent color

Add `AccentColor` `"Violet"` and make it `DEFAULT_ACCENT`.

- dark: `[248, 89, 73]` (`#8b7bf7`)
- light: `[247, 67, 59]` (`#6151dd`)

`ACCENT_HSL` (`index.ts:322-358`) is
`Record<AppearanceBase, Record<AccentColor, [number, number, number]>>`, so
adding the union member forces both bases to be filled. The values then run
through `buildAccentSet` (`index.ts:409-448`), which already derives
`--primary-foreground` by comparing contrast ratios and lifts `--ring` when the
accent fails a 3:1 check against the base surface. Adding the accent this way
inherits those guarantees instead of re-deriving them.

The design's `--ac2` hover value is not a token. Hover states use
`hsl(var(--primary) / <alpha>)` or a `brightness` filter, matching how the
design's own `style-hover="filter:brightness(1.08)"` works.

### D3 — Status, diff, and elevation tokens live only in `globals.css`

`FLAVOR_OVERRIDES` does not carry them, so declaring them under `:root` and
`.dark` makes them follow whichever base the active flavor selected. That is
correct for all three families.

| New token        | Dark          | Light         | Design source                    |
| ---------------- | ------------- | ------------- | -------------------------------- |
| `--success`      | `158 49% 56%` | `159 54% 40%` | `--ok`                           |
| `--warning`      | `40 67% 62%`  | `39 63% 44%`  | `--wn`                           |
| `--accent-alt`   | `332 64% 72%` | `331 48% 55%` | `--pk`                           |
| `--diff-added`   | `158 49% 56%` | `159 54% 40%` | `--add` fill, applied with alpha |
| `--diff-removed` | `352 67% 66%` | `352 58% 55%` | `--del` fill, applied with alpha |

`--destructive` (`globals.css:56`, `:109`) is re-pointed to the design's `--rd`:
dark `352 67% 66%`, light `352 58% 55%`. It is not flavor-overridden, so one
edit covers every family.

The design's `--add` / `--del` are 12% alpha fills. They are not separate
tokens; callers write `hsl(var(--diff-added) / 0.12)`, which is why every value
stays a channel-only triplet.

Elevation: `--shadow-lg` (`globals.css:73`, `:126`) is re-pointed to the
design's overlay shadow — dark `0 24px 80px rgba(0, 0, 0, .62)`, light
`0 24px 70px rgba(20, 25, 45, .16)`. `--shadow-sm` and `--shadow` keep their
current values; the design does not specify them.

Tailwind utility names must not shadow a built-in palette entry. `success`,
`warning`, `accent-alt`, `diff-added`, and `diff-removed` are not Tailwind
color names, so `extend.colors` adds them without hiding anything. `rose` and
`pink` would have shadowed built-ins; that is why `--accent-alt` is not named
after its color.

### D4 — Fonts: bundle the mono family, do not bundle the CJK family

Two families, two different answers.

**IBM Plex Mono — bundled.** Latin, Greek, and Cyrillic only. Two weights
(400, 500) in woff2 are roughly 100 KB total. It ships as a dependency, is
imported from CSS so Vite fingerprints and inlines it into `dist/`, and needs
no network.

New token `--font-mono`, plus a Tailwind `fontFamily.mono` entry so `font-mono`
resolves to it. The design uses mono for counts, timestamps, version tags,
shortcut hints, diff bodies, and the prompt body — every place the current UI
has no mono family at all.

**Noto Sans SC — not bundled.** It is a CJK face. The full family across four
weights is tens of megabytes, and this is a desktop bundle, not a page load.

Instead:

- `"Noto Sans SC"` is prepended to the default `interfaceFontStack`, so it is
  used when the user has it installed.
- `resolveInterfaceFontStack` (`preferences.ts:184-206`) already appends
  locale-aware CJK fallbacks — `Microsoft YaHei UI`, `PingFang SC`,
  `Noto Sans CJK SC` for `zh` (`preferences.ts:151-155`) — then platform
  fallbacks, then symbol fallbacks. Those are OS-installed, so offline
  rendering is already correct.

This amends PRD R6, which asked for both families to be bundled. The offline
requirement behind R6 is met either way; the bundle-size cost is not
justified. Implement step 5 measures the real size before this is final, so the
call is data-backed rather than assumed.

**Legacy font variables.** `--font-display` and `--font-body`
(`globals.css:33-34`, `:86-87`) are written only by the superseded controller
(`index.ts:533-537`). Nothing on the startup path sets them, so the
`.font-display` / `.font-body` utilities (`globals.css:163-169`) always resolve
to the system fallback today. Both defaults are re-pointed to
`var(--font-interface)` so the utilities follow the user's real font choice.

## Contracts and boundaries

- No backend change. `theme_family` is `Option<String>`
  (`src-tauri/src/models/settings.rs:26`) and `validate`
  (`src-tauri/src/services/settings.rs:120-142`) checks only
  `interface_font_stack`. A `"prompthub"` value persists as is.
- No Runtime Bridge change. The settings command surface is untouched.
- No component change. Components that already use token utilities inherit the
  palette. Components with literal colors are the later children's problem.

## Compatibility and migration

- An install that already persisted `themeFamily: "catppuccin"` keeps it.
  `normalizeAppearancePreferences` (`preferences.ts:100-128`) prefers a stored
  valid family over the default. Only new or unset installs get the new look.
- An install still holding a legacy `flavor` string migrates through
  `legacyMigration` (`preferences.ts:64-68`) unchanged.
- `accentColor` is normalized by `catalogValue` against `ACCENT_COLORS`
  (`preferences.ts:119`). A stored `"Blue"` stays `"Blue"`; only an unset or
  invalid value falls to the new `"Violet"` default.
- The appearance panel iterates `THEME_FAMILIES` (`AppearancePanel.tsx:149`) and
  resolves `familyOption.${family}` / `familyDescription.${family}`
  (`:172`, `:175`). Two new keys per locale are required in all 7 bundles, or
  the family renders as a raw key and `i18nKeys.test.ts` fails.

## Test impact

These suites assert current palette or normalization behavior and will need
updating together with the change:

- `src/appearance/index.palette.test.ts` — accent derivation.
- `src/appearance/index.normalize.test.ts` — flavor and accent normalization.
- `src/appearance/index.apply.test.ts` — the superseded controller's apply path.
- `src/appearance/preferences.test.ts` — family resolution and defaults.
- `src/features/settings/components/AppearancePanel.test.tsx`,
  `src/features/settings/SettingsView.appearance.test.tsx` — the family control.
- `src/features/settings/i18nKeys.test.ts` — the two new locale keys.

## Rollout and rollback

Rollout is a single default flip; every existing family stays selectable.

Rollback is one line: set `DEFAULT_THEME_FAMILY` back to `"catppuccin"` and
`DEFAULT_ACCENT` back to `"Blue"`. The PromptHub family and the Violet accent
remain available to anyone who picked them, and no persisted value becomes
invalid. The token additions in `globals.css` are additive and need no
rollback.
