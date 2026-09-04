# Frontend Guidelines

This file adds the rules for `src/**`. The root `AGENTS.md` still applies,
including the Runtime Bridge boundary and the verification gates.

Start with `src/code_map.md` before a broad search in `src/**`.

## Non-obvious stack facts

- Styling uses TailwindCSS with CSS-variable design tokens. The theme is one
  `.dark` class toggle (`src/theme`).
- i18n uses i18next with 7 locale bundles in `src/locales/`.
- Vitest runs in the Node environment and includes `src/**/*.test.{ts,tsx}`. A
  component test opts into a DOM with a per-file `// @vitest-environment jsdom`
  docblock.

## Hard rules

- Call a backend command by its wire name: `domain.action` (for example
  `"prompt.create"`, `"settings.update"`). `invoke` rejects with a `BridgeError`
  that carries the backend `code` and `message`. Handle the failure. Do not
  assume success.
- Capability-gated commands (see `CAPABILITY_GATES` in `runtime/index.ts`)
  short-circuit with `CAPABILITY_UNAVAILABLE`. Do not work around the gate.
- Remove the orphaned imports and locals that your change creates. `strict`,
  `noUnusedLocals`, and `noUnusedParameters` are on.

## Conventions

- All user-facing strings use i18n: `const { t } = useTranslation();` with
  dot-notation keys. Add each new key to all 7 bundles in `src/locales/`. Each
  feature has an `i18nKeys.test.ts` that checks the keys resolve.
- Use the Tailwind token classes (`bg-card`, `text-muted-foreground`,
  `border-border`, …). Use no icon library other than `lucide-react`.
- A feature module in `src/features/<name>/` holds `api.ts` (bridge calls),
  `<name>Store.ts` (Zustand), `types.ts`, and `components/`, each with a
  colocated `*.test.ts`. Keep new feature code in that shape.
- Test bridge-dependent code with injected fake Tauri primitives through
  `createRuntimeBridge(deps)` or a gateway interface. Do not mock modules.

Detailed frontend conventions live in `.trellis/spec/frontend/`. Read the index
there before large component, store, or design-token work.
