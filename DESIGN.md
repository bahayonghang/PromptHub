---
name: PromptHub
description: A compact, precise prompt workbench for a local-first knowledge base.
colors:
  signal-blue-light: "#1f66f4"
  signal-blue-dark: "#89b5fa"
  on-primary-light: "#ffffff"
  on-primary-dark: "#1e1e2e"
  latte-canvas: "#eff1f5"
  latte-surface: "#e6e9ef"
  latte-ink: "#4b4e68"
  latte-muted: "#6c6f84"
  latte-divider: "#bcc0cd"
  mocha-canvas: "#1e1e2e"
  mocha-surface: "#181825"
  mocha-elevated: "#313244"
  mocha-ink: "#cdd6f4"
  mocha-muted: "#a6adc9"
  mocha-divider: "#454759"
  claude-light-canvas: "#faf9f5"
  claude-light-ink: "#2b2a26"
  claude-dark-canvas: "#282725"
  claude-dark-ink: "#eae8e1"
  destructive-light: "#c05959"
  destructive-dark: "#dd3c3c"
typography:
  title:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 600
    lineHeight: 1.75
    letterSpacing: "0"
  body:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.25
    letterSpacing: "0"
  label:
    fontFamily: "ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: 1
    letterSpacing: "0"
  mono:
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.25
    letterSpacing: "0"
rounded:
  sm: "8px"
  md: "10px"
  lg: "12px"
  full: "9999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
components:
  button-primary-dark:
    backgroundColor: "{colors.signal-blue-dark}"
    textColor: "{colors.on-primary-dark}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-primary-light:
    backgroundColor: "{colors.signal-blue-light}"
    textColor: "{colors.on-primary-light}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "8px 16px"
  button-secondary-dark:
    backgroundColor: "{colors.mocha-canvas}"
    textColor: "{colors.mocha-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  input-dark:
    backgroundColor: "{colors.mocha-canvas}"
    textColor: "{colors.mocha-ink}"
    typography: "{typography.body}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
    height: "38px"
  navigation-active-dark:
    backgroundColor: "{colors.signal-blue-dark}"
    textColor: "{colors.on-primary-dark}"
    typography: "{typography.body}"
    rounded: "{rounded.lg}"
    padding: "8px 12px"
  chip-dark:
    backgroundColor: "{colors.mocha-elevated}"
    textColor: "{colors.mocha-muted}"
    typography: "{typography.label}"
    rounded: "{rounded.full}"
    padding: "2px 8px"
  panel-dark:
    backgroundColor: "{colors.mocha-surface}"
    textColor: "{colors.mocha-ink}"
    rounded: "{rounded.lg}"
    padding: "16px"
  switch-on-dark:
    backgroundColor: "{colors.signal-blue-dark}"
    textColor: "{colors.on-primary-dark}"
    rounded: "{rounded.full}"
    width: "44px"
    height: "24px"
---

# Design System: PromptHub

## Overview

**Creative North Star: "The Prompt Workbench"**

PromptHub is a compact, rigorous, tool-like workspace. It should feel ready for
real work immediately: dense enough for repeated editing and comparison, calm
enough for long sessions, and precise enough that users trust every saved state.
Its visual authority comes from hierarchy, alignment, and predictable feedback.

The workbench supports two named theme families. Catppuccin retains its official
Latte, Frappé, Macchiato, and Mocha character; Claude provides a warmer light and
dark alternative. Theme personality changes, but semantic component roles and
interaction behavior do not.

PromptHub explicitly rejects the generic SaaS dashboard: no oversized card
grids, gradients, badge piles, decorative metrics, or marketing composition in
operational screens.

**Key Characteristics:**

- Compact, precise controls with stable dimensions.
- Tonal surface hierarchy before shadow.
- One restrained signal accent for action, selection, and focus.
- Consistent desktop navigation and form vocabulary.
- Fast state transitions with no decorative choreography.

## Colors

Official Catppuccin and Claude names are preserved. Semantic roles stay stable
across families; Signal Blue is the action language, not a decorative brand wash.

### Primary

- **Signal Blue:** `colors.signal-blue-light` on light variants and
  `colors.signal-blue-dark` on dark variants. Use it for primary actions,
  current selection, progress, and keyboard focus.
- **On Signal:** `colors.on-primary-light` on the light Signal Blue and
  `colors.on-primary-dark` on the dark Signal Blue. These are contrast pairs,
  not interchangeable white text defaults.

### Neutral

- **Catppuccin Latte:** `colors.latte-canvas`, `colors.latte-surface`,
  `colors.latte-ink`, `colors.latte-muted`, and `colors.latte-divider` form the
  light workbench hierarchy.
- **Catppuccin Mocha:** `colors.mocha-canvas`, `colors.mocha-surface`,
  `colors.mocha-elevated`, `colors.mocha-ink`, `colors.mocha-muted`, and
  `colors.mocha-divider` form the default dark hierarchy. Frappé and Macchiato
  use the same semantic roles with their official palette values.
- **Claude:** `colors.claude-light-canvas` with `colors.claude-light-ink`, and
  `colors.claude-dark-canvas` with `colors.claude-dark-ink`, provide the second
  family without changing component semantics.

### Functional

- **Destructive:** `colors.destructive-light` and `colors.destructive-dark` are
  reserved for irreversible actions and blocking errors.

**The Signal, Not Decoration Rule.** Signal Blue should occupy no more than the
interactive emphasis needed on a screen. If inactive surfaces turn blue, the
signal has lost its meaning.

**The Family and Mode Rule.** Theme family and light/dark/system mode are
independent. An OS mode change may swap a variant; it must never change the
stored family, accent, or Catppuccin dark variant.

**The Contrast Pair Rule.** Never assume white is valid on every primary color.
Use the paired on-primary token and verify WCAG 2.2 AA contrast.

## Typography

**Display Font:** The interface system sans stack; PromptHub has no separate
display face.

**Body Font:** The same interface system sans stack with an ordered,
locale-aware fallback chain.

**Label/Mono Font:** System sans for labels; the system monospace stack only for
prompt source, structured content, and code-like values.

**Character:** One technical sans voice carries navigation, headings, forms, and
data. Weight and spacing create hierarchy; switching to a decorative display
font would make the workbench feel less trustworthy.

### Hierarchy

- **Title** (600, 1.125rem, 1.75 line-height): active view titles and major panel
  headings.
- **Body** (400, 0.875rem, 1.25 line-height): controls, descriptions, and primary
  application copy. Prose should remain within 65-75 characters per line.
- **Label** (500, 0.75rem, 1 line-height): field labels, metadata, and compact
  status copy. Keep normal case and zero letter spacing.
- **Mono** (400, 0.875rem, 1.25 line-height): editable prompt bodies, previews,
  paths, and structured technical values.

**The One Working Voice Rule.** Use one interface family across product UI.
Never introduce a second display family merely to make a settings heading feel
special.

## Elevation

PromptHub is layered, not lifted. Canvas, sidebar, panels, selected rows, and
popovers separate primarily through tonal surfaces and one-pixel dividers.
Shadows are reserved for overlays or an explicitly raised state.

### Shadow Vocabulary

- **Light low** (`0 1px 2px rgba(0, 0, 0, 0.04)`): for a selected or raised control
  that cannot rely on tone alone.
- **Light overlay** (`0 12px 32px rgba(0, 0, 0, 0.08)`): for dialogs and popovers.
- **Dark low** (`0 2px 4px rgba(0, 0, 0, 0.4)`): for the same limited raised state
  under dark themes.
- **Dark overlay** (`0 16px 48px rgba(0, 0, 0, 0.6)`): for modal surfaces only.

**The Tonal-First Rule.** A resting card or section must not use a wide shadow.
If a one-pixel border and a shadow are both decorative, remove the shadow.

## Components

Components feel compact, precise, and decisive. Their shape vocabulary is
limited to gently curved controls and panels; state changes are fast and direct.

### Buttons

- **Shape:** Gently curved controls (`rounded.md`, 10px), normally 38-40px high.
- **Primary:** Signal Blue with its paired on-primary color, medium body weight,
  and 8px by 16px padding.
- **Hover / Focus:** A small tonal change at 150ms; keyboard focus uses a clear
  two-pixel equivalent ring. Disabled state reduces emphasis and removes pointer
  affordance without hiding the label.
- **Secondary:** Transparent or canvas-toned with a one-pixel input divider;
  hover moves to the semantic accent surface.

### Chips

- **Style:** Compact metadata only, using `rounded.full`, a tonal muted surface,
  2px by 8px padding, and label typography.
- **State:** Selected filter chips require a non-color cue such as a check or
  explicit pressed state. Do not turn ordinary labels into chips.

### Cards / Containers

- **Corner Style:** Gently curved panels (`rounded.lg`, 12px).
- **Background:** Theme surface tokens, never an unrelated tinted card palette.
- **Shadow Strategy:** Flat by default; overlay shadows only as defined above.
- **Border:** One-pixel semantic divider when adjacent tones are insufficient.
- **Internal Padding:** 12-16px for repeated items and 16-24px for framed tools.

### Inputs / Fields

- **Style:** Canvas background, one-pixel input divider, 10px radius, 8px by
  12px padding, and body typography.
- **Focus:** Signal Blue ring with sufficient area and contrast; preserve stable
  dimensions so focus never shifts layout.
- **Error / Disabled:** Error copy sits adjacent to the field and uses an icon or
  text in addition to destructive color. Disabled fields remain readable.

### Navigation

The application uses a fixed desktop sidebar, compact icon-plus-label rows, and
a plain header title. Default items are muted, hover uses the sidebar accent
surface, and the active item uses a strong semantic selection with
`aria-current`. Sidebar collapse is structural and takes 200ms; reduced-motion
preference makes it immediate.

### Switches

Switches are reserved for binary preferences. The track uses semantic inactive
and primary states, the thumb keeps a stable 20px size, and `aria-checked` is the
source of truth. A label remains visible beside every switch.

## Do's and Don'ts

### Do:

- **Do** use semantic tokens so Catppuccin and Claude remain behaviorally
  identical even when their palettes differ.
- **Do** keep primary actions, selection, and focus on Signal Blue and use the
  correct on-primary contrast partner.
- **Do** use 10px control radii, 12px panel radii, and the 4/8/12/16/24px spacing
  rhythm consistently.
- **Do** constrain long settings and editor copy to readable line lengths while
  allowing lists and data surfaces to remain dense.
- **Do** expose hover, focus, active, disabled, loading, error, and selected
  states, with status never conveyed by color alone.
- **Do** keep state motion between 150-200ms and provide an immediate
  reduced-motion path.

### Don't:

- **Don't** resemble a generic SaaS dashboard: no oversized cards, gradients,
  badge-heavy layouts, decorative metrics, or marketing composition in the app.
- **Don't** nest cards or turn full-width page sections into floating cards.
- **Don't** pair a decorative one-pixel border with a wide shadow; resting
  surfaces use tone and dividers.
- **Don't** use display fonts, gradient text, glassmorphism, decorative grids, or
  giant rounded containers to manufacture personality.
- **Don't** hide common commands behind modals or unfamiliar controls when a
  standard inline product pattern exists.
- **Don't** use white text on dark-theme Signal Blue; use
  `colors.on-primary-dark` so the control remains readable.
