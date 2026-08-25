# Frontend Guidelines

This file governs `src/**`. Root guidance still applies; this file adds the
frontend-specific rules and overrides for the React/TypeScript app.

For this subtree, start with `src/code_map.md` before broad grep. If that file is
missing, fall back to `../code_map.md`.

## Stack

React 18 + TypeScript 5, built by Vite 6. State: Zustand. Styling: TailwindCSS 3
with CSS-variable design tokens. i18n: i18next + react-i18next (7 locales).

## Commands

All run from the repository root (the frontend shares the root `package.json`):

- `npm run dev` — Vite dev server on `http://localhost:5173`.
- `npm run build` — `tsc` typecheck + Vite build; this is the static gate.
- `npm run test` — Vitest (`src/**/*.test.ts`), Node environment.
- `npx vitest run <path>` — run a single test file while iterating.

## Hard Rules

- Never import `@tauri-apps/api` outside `src/runtime`. All backend calls go
  through `runtime.invoke(command, args)` and all events through `runtime.on(...)`.
  Components must not call the backend directly.
- Call backend commands by their wire name: `domain.action` (e.g.
  `"prompt.create"`, `"settings.update"`). `invoke` rejects with a `BridgeError`
  carrying the backend `code` + `message`; handle failures, never assume success.
- Capability-gated commands (see `CAPABILITY_GATES` in `runtime/index.ts`)
  short-circuit with `CAPABILITY_UNAVAILABLE` when unavailable — do not work
  around the gate.
- `strict` TypeScript with `noUnusedLocals`/`noUnusedParameters` is on; clean up
  orphaned imports/locals your change creates.

## Conventions

- All user-facing strings use i18n: `const { t } = useTranslation();` with
  dot-notation keys. Each feature has an `i18nKeys.test.ts` that checks keys
  resolve across locales — keep `src/locales/*.json` aligned when adding keys.
- Styling uses Tailwind design tokens only (`bg-card`, `text-muted-foreground`,
  `border-border`, …); theme is a single `.dark` class toggle (`src/theme`). No
  other icon library than `lucide-react`.
- Feature modules under `src/features/<name>/` follow `api.ts` (bridge calls) +
  `<name>Store.ts` (Zustand) + `types.ts` + `components/`, each with colocated
  `*.test.ts`. Keep new feature code in that shape.

## Local Testing

- Bridge-dependent code is tested by injecting fake Tauri primitives via
  `createRuntimeBridge(deps)` / gateway interfaces — no webview needed. Follow
  that dependency-injection pattern instead of mocking modules.
- Run `npm run test` plus `npm run build` before claiming a frontend change is done.
