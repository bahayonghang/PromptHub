# Design Tokens

PromptHub paints through CSS custom properties. Values that represent color are
channel-only HSL triplets (`H S% L%`) so Tailwind alpha utilities such as
`bg-primary/20` resolve to `hsl(var(--primary) / 0.2)`. Do not write a hex
string or an `hsl()` wrapper into a color token.

Theme switching is a `.dark` class on `document.documentElement`. Do not add a
`data-theme` attribute as the paint switch.

## Ownership

The appearance controller in `src/appearance/preferences.ts` overrides **21**
tokens at runtime as inline styles on the root. Inline styles beat `@layer
base`, so those 21 values always come from the active flavor and accent:

| Source | Tokens | Count |
| ------ | ------ | ----- |
| `FLAVOR_OVERRIDES[variant]` | `--background`, `--foreground`, `--card`, `--card-foreground`, `--popover`, `--popover-foreground`, `--muted`, `--muted-foreground`, `--secondary`, `--secondary-foreground`, `--border`, `--input`, `--sidebar`, `--sidebar-foreground`, `--sidebar-accent`, `--sidebar-border` | 16 |
| `ACCENT_PALETTE[base][accent]` | `--primary`, `--primary-foreground`, `--ring`, `--accent`, `--accent-foreground` | 5 |

`src/styles/globals.css` owns every other token. Both the `:root` (light) and
`.dark` scopes must declare the identical set.

The controller also writes `--font-interface`, `--font-scale`,
`--density-padding`, `--density-gap`, `color-scheme`, and `data-*` attributes.
Those are not part of the 21 color overrides.

## Tokens owned by `globals.css`

| Token | Role | Tailwind utility |
| ----- | ---- | ---------------- |
| `--surface-inset` | Inset panel / textarea (`--surf2`) | `bg-surface-inset` |
| `--muted-foreground-subtle` | Counts, timestamps, column headers (`--faint`) | `text-muted-foreground-subtle` |
| `--border-strong` | Strong divider (`--line2`) | `border-border-strong` |
| `--success` | Positive / ok (`--ok`) | `text-success`, `bg-success/15` |
| `--warning` | Warning (`--wn`) | `text-warning` |
| `--accent-alt` | Secondary accent (`--pk`) | `text-accent-alt` |
| `--diff-added` | Diff add fill; apply with alpha | `bg-diff-added/12` |
| `--diff-removed` | Diff delete fill; apply with alpha | `bg-diff-removed/12` |
| `--destructive` | Destructive / error (`--rd`) | `text-destructive` |
| `--shadow-lg` | Overlay elevation | `shadow-lg` |
| `--font-mono` | IBM Plex Mono stack | `font-mono` |
| `--radius` | Corner radius | `rounded-lg` |
| `--state-hover` | Hover surface for rows/controls | `bg-state-hover` |
| `--state-selected` | Selected row/card fill | `bg-state-selected` |
| `--state-pressed` | Pressed surface | `bg-state-pressed` |
| `--favorite` | Favourite star (never a second accent) | `text-favorite` |
| `--tag-1` … `--tag-8` | Global tag palette | `text-tag-3`, `bg-tag-3/15` |
| `--control-xs\|sm\|md\|lg` | Control heights (24/28/32/36px) | `h-control-md` |
| `--hairline` | Edge highlight (dark) / bottom inset (light) | `shadow-hairline` |
| `--shadow-overlay` | Dialog / popover elevation | `shadow-overlay` |
| `--shadow-panel` | Docked side panel | `shadow-panel` |
| `--dur-fast` / `--dur-base` | 120ms / 180ms | `duration-fast` |
| `--ease-out` / `--ease-spring` | Standard easings | `ease-out`, `ease-spring` |

## Type scale

Six steps, each carrying its own line-height and letter-spacing. Use these
instead of pairing `text-xs`/`text-sm` with a hand-picked `leading-*`, and never
write an arbitrary value such as `text-[11px]`.

| Utility | Size / line-height | Role |
| ------- | ------------------ | ---- |
| `text-micro` | 10 / 14 | Counters, corner badges |
| `text-meta` | 11 / 16 | Timestamps, column headers, versions |
| `text-label` | 12 / 16 | Field labels, chips, secondary buttons |
| `text-body` | 13 / 20 | **Primary interface size** |
| `text-title` | 15 / 22 | Panel and card headings |
| `text-display` | 19 / 26 | View titles |
| `text-hero` | 24 / 32 | Page headers |

## Alpha modifiers

Tailwind's default opacity scale is `0, 5, 10, … 100`. An off-scale modifier
such as `bg-tag-1/14` is **silently dropped at build time**, leaving the element
with no fill. Always snap to a multiple of 5.

## Tag colours

Tags are user-authored free text, so hues are derived, not authored:
`src/components/ui/tagColor.ts` hashes the tag name (FNV-1a) onto one of the
eight `--tag-*` slots. The mapping is pure and deterministic, so a tag keeps the
same hue in the sidebar, filter bar, list, cards, and detail panel. Slot classes
are written as complete literals so Tailwind's scanner retains them.

`--font-display` and `--font-body` default to `var(--font-interface)` so the
legacy utilities follow the live interface stack.

## Default appearance

A new or unset install uses `themeFamily: "prompthub"`, color mode `dark`, and
accent `Violet`. A stored `themeFamily` of `catppuccin` or `claude` is kept.

`Noto Sans SC` is the first default interface family when the OS has it. It is
not bundled. `IBM Plex Mono` is bundled (latin 400 and 500).
