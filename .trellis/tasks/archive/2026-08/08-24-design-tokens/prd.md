# Design tokens and typography baseline

Child of `08-24-ui-refactor`. Owns parent requirement R15.

## Goal

Replace the current palette with the design concept's token set in both themes,
and add the concept's two type families, without breaking the Tailwind utility
layer, the `.dark` toggle, or the runtime appearance overrides.

## Ordering

This child lands first. Every other child in the tree styles against these
tokens.

## Background

- `src/styles/globals.css:18-73` declares the light scope and `:85-126` the dark
  scope. Values are channel-only HSL, for example `--background: 220 14% 96%`.
- `tailwind.config.js:12-53` maps every color utility to `hsl(var(--token))`.
  The channel-only form is what makes `bg-primary/20` work. Any hex value written
  into a token breaks alpha variants.
- Theme switching toggles a `.dark` class on `document.documentElement`
  (`src/theme/index.ts`). The design file instead uses
  `:root[data-theme="light"]`. The class mechanism stays.
- The appearance system, not `globals.css`, decides 21 of these tokens. The live
  paint path is `src/appearance/preferences.ts:251-270`, called from
  `src/main.tsx:10` on every startup. It writes `FLAVOR_OVERRIDES[variant]`
  (16 surface/text/border tokens) and `ACCENT_PALETTE[base][accent]` (5 accent
  tokens) as inline styles on the root, which beat any `@layer base` rule.
- The out-of-box appearance is `themeFamily: "catppuccin"` +
  `catppuccinDarkVariant: "mocha"` + `accentColor: "Blue"`
  (`preferences.ts:44-46`, `index.ts:112`). Editing `globals.css` alone changes
  none of the 21 tokens. See `design.md` for how the design palette is routed
  in.
- The controller in `src/appearance/index.ts:490-600` and `src/theme/index.ts`
  have no importers on the startup path. They are superseded by
  `preferences.ts`, but `preferences.ts` imports their data tables
  (`FLAVOR_BASE`, `FLAVOR_OVERRIDES`, `ACCENT_PALETTE`, `DENSITY_RHYTHM`,
  `FONT_SCALE_PERCENT`), so those tables must stay correct.
- `--font-interface`, `--font-scale`, and the density variables are written by
  the live controller (`preferences.ts:258-263`). These overrides must keep
  working. `--font-display` and `--font-body` are written only by the superseded
  controller, so they currently resolve to the system fallback.
- `globals.css:185-290` defines the `prompt-workspace` / `prompt-editor`
  container queries. They are layout, not color, and stay.

## Design token source

From `PromptHub.dc.html`:

Dark (the design's default presentation):

| Concept var                 | Value                                   | Role              |
| --------------------------- | --------------------------------------- | ----------------- |
| `--bg`                      | `#0b0d13`                               | app background    |
| `--surf`                    | `#12151e`                               | panel / card      |
| `--surf2`                   | `#171b26`                               | inset surface     |
| `--surf3`                   | `#1e2331`                               | chip / muted fill |
| `--line`                    | `#252b3a`                               | default border    |
| `--line2`                   | `#2f3648`                               | strong border     |
| `--tx`                      | `#e7eaf3`                               | foreground        |
| `--dim`                     | `#98a0b5`                               | secondary text    |
| `--faint`                   | `#6a7285`                               | tertiary text     |
| `--ac`                      | `#8b7bf7`                               | accent            |
| `--ac2`                     | `#a99bff`                               | accent hover      |
| `--pk` `--ok` `--wn` `--rd` | `#e58ab5` `#57c69d` `#dfb45f` `#e3707f` | status            |

Light:

| Concept var                 | Value                                   |
| --------------------------- | --------------------------------------- |
| `--bg`                      | `#f4f6fa`                               |
| `--surf`                    | `#ffffff`                               |
| `--surf2`                   | `#f7f8fc`                               |
| `--surf3`                   | `#eef0f7`                               |
| `--line`                    | `#e3e6ef`                               |
| `--line2`                   | `#d3d8e6`                               |
| `--tx`                      | `#161a24`                               |
| `--dim`                     | `#5b6376`                               |
| `--faint`                   | `#8b93a6`                               |
| `--ac`                      | `#6151dd`                               |
| `--ac2`                     | `#5241cf`                               |
| `--pk` `--ok` `--wn` `--rd` | `#c4568c` `#2f9d76` `#b8862a` `#cf4c5e` |

Type: `Noto Sans SC` (300/400/500/700) for interface and body,
`IBM Plex Mono` (400/500) for identifiers, counts, timestamps, versions, and
diff text.

## Requirements

- R1: Every design color is expressed as a channel-only HSL triplet, in both the
  `:root` and the `.dark` scope and in the appearance system's palette tables.
  No token value is a hex string or an `rgba()` literal.
- R1b: The design palette reaches the screen through the appearance system, not
  only through `globals.css`. The out-of-box appearance renders the design
  palette. Every appearance option that ships today stays selectable and keeps
  working: theme family, color mode including `system`, accent color, interface
  font stack, font scale, and density.
- R1c: An install that already persisted an appearance preference keeps it. Only
  a new or unset install gets the new default.
- R2: The mapping from concept var to existing token is explicit and complete.
  `--ac` maps to `--primary`. `--bg`/`--surf`/`--surf2`/`--surf3` map onto
  `--background`, `--card`/`--popover`, `--secondary`, and `--muted`.
  `--line`/`--line2` map onto `--border` and `--input`. `--tx`/`--dim`/`--faint`
  map onto `--foreground` and `--muted-foreground`. Sidebar tokens
  (`globals.css:65-68`, `:118-121`) get the concept's panel values.
- R3: Status colors (`--ok`, `--wn`, `--rd`, `--pk`) are added as named tokens
  with Tailwind utilities, because the design uses them for diff, score, and
  destructive states. `--rd` maps onto `--destructive`.
- R4: Diff-line fills (`--add`, `--del`) and the overlay shadow (`--shadow`)
  from the design exist as tokens. The version-history diff and the detail
  overlay consume them.
- R5: Dark is the presentation the design specifies. The existing default-theme
  resolution (system / light / dark) keeps working; no theme is removed.
- R6 (amended by `design.md` D4): No font is fetched from
  `fonts.googleapis.com`. The app renders both families correctly with no
  network. `IBM Plex Mono` is bundled, because it is Latin-only and small.
  `Noto Sans SC` is not bundled, because a CJK webfont costs tens of megabytes
  in a desktop bundle; it is preferred in the default interface stack when
  installed, and the existing locale fallbacks in `preferences.ts:151-155`
  cover the offline case with OS fonts. Implement step 5 measures the bundle
  cost before this is final.
- R7: The mono family is available as a token and a Tailwind utility. The
  appearance module's `--font-display` / `--font-body` overrides still take
  precedence when the user sets a font.
- R8: `--radius` and the density rhythm keep their current names and their
  runtime overrides.

## Acceptance criteria

- [ ] AC1: `src/styles/globals.css` declares the same token names in both
      scopes. Reading either scope shows no missing token.
- [ ] AC2: `bg-primary/20` and every other alpha utility still resolve. A
      regression test or a rendered check confirms alpha still applies.
- [ ] AC3: Toggling the theme repaints every existing view with no restart and
      no unstyled flash.
- [ ] AC4: Setting a custom accent in appearance settings still overrides
      `--primary`, and `src/appearance/index.palette.test.ts` passes.
- [ ] AC4b: A fresh install renders the design palette with no user action.
- [ ] AC4c: An install holding `themeFamily: "catppuccin"` still renders
      Catppuccin after the change, verified by a test over
      `normalizeAppearancePreferences`.
- [ ] AC4d: `system` color mode still follows the OS preference and repaints on
      change under the new default family.
- [ ] AC5 (R6): With the network disabled, no request goes to
      `fonts.googleapis.com` or `fonts.gstatic.com`, and `IBM Plex Mono` renders
      from the bundled file. Asserted on the network log, not by eye.
- [ ] AC5b (R6): With the network disabled and `Noto Sans SC` **not installed**,
      CJK text renders through the locale fallback chain
      (`preferences.ts:151-155`) with no missing-glyph boxes. The computed
      `font-family` contains the fallback stack.
- [ ] AC5c (R6): With `Noto Sans SC` installed, it is the family the browser
      resolves for CJK text.

AC5 previously read "both font families render", which no build can satisfy
deterministically: `design.md` D4 states that `Noto Sans SC` is not bundled,
because a CJK family across four weights is tens of megabytes. On a machine
without it installed, the app renders CJK through OS fallbacks. The same build
would pass or fail with the host's font set, so the criterion is split into the
three testable statements above. The host precondition — Noto installed or not —
is named in each.
- [ ] AC6: `just build` and `just test` pass.
- [ ] AC7: A short token table lands in `.trellis/spec/frontend/` so later
      children pick tokens from one place.

## Out of scope

- Restyling any component. This child changes the token layer only; components
  that already use token utilities inherit the new palette.
- The `data-theme` attribute mechanism from the design file.
- Removing or renaming the appearance module's user-facing font and density
  options.
