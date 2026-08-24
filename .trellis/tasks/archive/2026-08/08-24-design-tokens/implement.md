# Implement — design tokens and typography baseline

Execution plan for the decisions in `design.md`. Steps are ordered. Each gate
must pass before the next step starts.

Frontend only. No file under `src-tauri/` changes in this task.

## Step 0 — Baseline

- [ ] `git status` is clean; branch from the task's base branch.
- [ ] Run `just build` and `just test` and record that both pass before any
      edit. A pre-existing failure must be reported, not absorbed into this
      task's diff.
- [ ] Capture the current look for comparison: start the app and screenshot the
      prompts view in the default appearance (Catppuccin Mocha + Blue) in both
      light and dark mode.

Gate: baseline recorded. Without it, "no visual regression" is unverifiable.

## Step 1 — Add the PromptHub flavor variants

File: `src/appearance/index.ts`

- [ ] Extend the `Flavor` union (`:22-29`) with `"PromptHub Light"` and
      `"PromptHub Dark"`.
- [ ] Add both to `FLAVORS` (`:71-78`).
- [ ] Add both to `FLAVOR_BASE` (`:125`): light -> `"light"`, dark -> `"dark"`.
- [ ] Add both swatch sets to `FLAVOR_SWATCHES` (`:238-286`) using the D1 table
      in `design.md`.
- [ ] Add both to `FLAVOR_OVERRIDES` (`:313-320`) via `buildFlavorOverrides`.

Do not touch the four Catppuccin or two Claude swatch sets.

Gate: `npx tsc --noEmit` passes. Every `Record<Flavor, …>` is exhaustive, so a
missing entry fails here rather than at runtime.

## Step 2 — Add the Violet accent

File: `src/appearance/index.ts`

- [ ] Extend the `AccentColor` union (`:31-45`) with `"Violet"`.
- [ ] Add it to `ACCENT_COLORS` (`:80-95`).
- [ ] Add `Violet: [248, 89, 73]` under `dark` and `Violet: [247, 67, 59]` under
      `light` in `ACCENT_HSL` (`:322-358`).
- [ ] Change `DEFAULT_ACCENT` (`:112`) to `"Violet"`.

Do not hand-write `--primary-foreground`, `--ring`, `--accent`, or
`--accent-foreground`. `buildAccentSet` (`:409-448`) derives all four.

Gate: a unit assertion that `ACCENT_PALETTE.dark.Violet["--primary"]` is
`"248 89% 73%"`, and that the derived `--primary-foreground` clears 4.5:1
against the accent. Extend `src/appearance/index.palette.test.ts`; it already
tests this derivation for other accents.

## Step 3 — Add the PromptHub theme family

File: `src/appearance/preferences.ts`

- [ ] Extend `ThemeFamily` (`:22`) with `"prompthub"` and add it to
      `THEME_FAMILIES` (`:37-42`), listed first.
- [ ] Set `DEFAULT_THEME_FAMILY` (`:44`) to `"prompthub"`.
- [ ] Extend `resolveThemeVariant` (`:130-149`) to return `"PromptHub Dark"` /
      `"PromptHub Light"` for that family, before the Catppuccin branch.
- [ ] Add `LEGACY_FLAVOR_MIGRATION` entries (`:48-58`) for the two new variants,
      each mapping to `themeFamily: "prompthub"` and its own color mode.

Verify by reading, not by assuming: `normalizeAppearancePreferences`
(`:100-128`) must still prefer a stored valid `themeFamily` over the new
default. An install that already chose Catppuccin keeps Catppuccin.

Gate: extend `src/appearance/preferences.test.ts` with three cases —

1. empty settings resolve to `themeFamily: "prompthub"` and
   `effectiveVariant() === "PromptHub Dark"`;
2. `{ themeFamily: "catppuccin", catppuccinDarkVariant: "mocha" }` still
   resolves to `"Mocha"`;
3. `theme: "system"` under the new family still attaches the media listener and
   repaints on change (`:277-284`).

## Step 4 — Rewrite the token layer

File: `src/styles/globals.css`

- [ ] Re-point the light scope (`:39-73`) and the dark scope (`:92-126`) to the
      design values, using the D1 and D3 tables. Every value stays a
      channel-only HSL triplet, `H S% L%`, with no `hsl()` wrapper and no hex.
- [ ] Add `--surface-inset`, `--muted-foreground-subtle`, and `--border-strong`
      to both scopes.
- [ ] Add `--success`, `--warning`, `--accent-alt`, `--diff-added`, and
      `--diff-removed` to both scopes.
- [ ] Re-point `--destructive` in both scopes.
- [ ] Re-point `--shadow-lg` in both scopes to the design's overlay shadow.
- [ ] Add `--font-mono` to both scopes.
- [ ] Re-point the `--font-display` and `--font-body` defaults
      (`:33-34`, `:86-87`) to `var(--font-interface)`.
- [ ] Update the file's header comment (`:5-17`) to state that the appearance
      controller in `src/appearance/preferences.ts` overrides 21 of these
      tokens at runtime, and that this file owns the rest. The current comment
      is now misleading and would send the next reader down the wrong path.

Do not touch the container-query blocks (`:180-304`) or the reduced-motion block
(`:311-320`).

File: `tailwind.config.js`

- [ ] Add `success`, `warning`, `accent-alt`, `diff-added`, `diff-removed`, and
      `border-strong` to `extend.colors`, each as `hsl(var(--token))`.
- [ ] Add `extend.fontFamily.mono` reading `var(--font-mono)`.

Check each new key against Tailwind's built-in palette before adding it. A name
collision silently shadows the built-in color for the whole project.

Gate: `just build` passes, and a rendered check confirms an alpha utility still
resolves — `bg-primary/20` must produce a translucent fill, not a solid one.
That is the regression this token format exists to prevent.

## Step 5 — Fonts

- [x] Measure first. `@fontsource-variable/noto-sans-sc@5.3.0` unpacked size is
      4.89 MB (4.7 MB tarball; unicode-range woff2 slices for CJK). That is
      above the 3 MB reopen threshold, so D4 stands: do not bundle Noto Sans SC.
      Baseline `dist/` before this child: 1.03 MB.
- [ ] Add the IBM Plex Mono dependency (weights 400 and 500, woff2 only) and
      import it so Vite bundles it. Do not add a `fonts.googleapis.com` link to
      `index.html`.
- [ ] Point `--font-mono` at `"IBM Plex Mono", ui-monospace, SFMono-Regular,
    Menlo, Consolas, monospace`.
- [ ] Prepend `"Noto Sans SC"` to the default interface stack. The default lives
      in `normalizeInterfaceFontStack` (`preferences.ts:70-98`), which falls back
      to `["System"]`. Change the default, not the sanitizer: the length,
      control-character, and de-duplication rules stay as they are.

Gate: with the network blocked, launch the app and confirm mono text renders in
IBM Plex Mono and Latin plus CJK interface text renders without a fallback box.

## Step 6 — Appearance panel and locales

- [ ] `src/features/settings/components/AppearancePanel.tsx` iterates
      `THEME_FAMILIES` (`:149`) and resolves
      `settingsView.appearance.familyOption.prompthub` and
      `settingsView.appearance.familyDescription.prompthub` (`:172`, `:175`).
      Add both keys to all 7 bundles under `src/locales/`.
- [ ] Confirm the Catppuccin-only dark-variant section (`:214-230`) still hides
      correctly under the new family.

Gate: `src/features/settings/i18nKeys.test.ts` passes.

## Step 7 — Spec note

- [ ] Write a short token reference into `.trellis/spec/frontend/`: the token
      list, which 21 tokens the appearance controller owns, which tokens
      `globals.css` owns, and the rule that token values are channel-only HSL.
      The five later children in this tree pick tokens from that page.
- [ ] Add the new file to `.trellis/spec/frontend/index.md`.

## Step 8 — Full check

- [ ] `just build`
- [ ] `just test`
- [ ] `just ci`
- [ ] Screenshot comparison against the Step 0 baseline, in both color modes,
      and under one Catppuccin flavor to confirm the existing families still
      paint correctly.

## Review gates

| After step | Gate                                                               |
| ---------- | ------------------------------------------------------------------ |
| 1          | Type check passes; no existing flavor edited                       |
| 2          | Violet accent derives a contrast-passing foreground and ring       |
| 3          | A stored Catppuccin preference survives the default change         |
| 4          | Alpha utilities still resolve; no hex or `rgba()` in a color token |
| 5          | Offline launch renders both families                               |
| 6          | i18n key test passes across all 7 bundles                          |
| 8          | `just ci` green; existing families unchanged                       |

## Rollback points

- After step 4, the change is still additive and reversible per file.
- The cheapest revert at any point is `DEFAULT_THEME_FAMILY = "catppuccin"` and
  `DEFAULT_ACCENT = "Blue"`. That restores the previous out-of-box look and
  leaves every addition in place and selectable.
- Steps 1 through 3 are independently revertible. Step 4 is the only step that
  changes what every existing component renders, so it is the one to revert
  first if the diff has to shrink.

## Known follow-ups, not in this task

- `src/theme/index.ts` and the controller in `src/appearance/index.ts:490-600`
  have no importers on the startup path. Retiring them is separate work; this
  task keeps their data tables correct because `preferences.ts` imports them.
- `--surface-inset`, `--muted-foreground-subtle`, and `--border-strong` are not
  flavor-scoped, so they keep the design's values under a Catppuccin or Claude
  family. Widening `FlavorSwatches` would fix it and is not owned here.
- Components that still carry literal colors are re-styled by the later
  children in this tree.
