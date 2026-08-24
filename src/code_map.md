# `src` Code Map

Use this map for `src/**` navigation. Behavioral rules and commands live in
`src/AGENTS.md` (or the root `AGENTS.md`).

## Subtree Responsibility

The React/TypeScript frontend: UI, app state, i18n, theming, and the single
Runtime Bridge through which all backend access flows.

## Internal Routing

- `runtime/` — Runtime Bridge (`invoke`/`on`/capabilities) and i18n setup; start
  here for anything crossing to the backend.
- `features/` — business modules (`prompts/`, `settings/`, `system/`),
  each self-contained; start here for feature behavior.
- `components/` — shared layout (`layout/`) and top-level views (`views/`).
  The sidebar rail is app-wide; library saved views, folders, and tags live in
  `features/prompts/components/PromptLibraryNav.tsx`.
- `store/` — app-level Zustand store (readiness, fatal init error).
- `theme/` — light/dark/system theme application and persistence.
- `appearance/` — theme family catalogs, accent palettes, and the live paint
  controller (`preferences.ts`). New installs default to the PromptHub family.
- `locales/` — the 7 translation bundles (`en`, `zh`, `zh-TW`, `ja`, `fr`, `de`, `es`).
- `styles/globals.css` — Tailwind layers + design-token CSS variables. Color
  tokens are channel-only HSL; `src/appearance/preferences.ts` overrides 21 of
  them at runtime. Token list: `.trellis/spec/frontend/design-tokens.md`.

## Key Files

- `main.tsx` — entry: applies theme, inits i18n, mounts `<App/>`.
- `App.tsx` — bootstraps app store; renders `AppShell` or the fatal-error surface.
- `runtime/index.ts` — `createRuntimeBridge`, `BridgeError`, `CommandResult`,
  `RuntimeCapabilities`, `CAPABILITY_GATES`.
- `runtime/i18n.ts` — locale resolution/normalization, lazy bundle loading.
- `store/appStore.ts` — `initialize()` and init-failure subscription.
- `features/<name>/api.ts` — that feature's typed bridge calls.
- `features/<name>/<name>Store.ts` — that feature's Zustand store.
- `features/prompts/PromptsView.tsx` — prompt paging, portable bundle controls,
  tag management, and batch-action orchestration.
- `features/prompts/libraryItem.ts` — shared projection for grid and list
  library items.
- `features/prompts/promptText.ts` — clipboard formatting for `prompt.copy`;
  `@@Title` expansion lives in the backend (`reference.list` / `prompt.copy`).
- `features/prompts/versionDiff.ts` — structured immutable-revision diffing.
- `features/evaluation/` — typed bridge API, Zustand orchestration, and the
  playground/matrix/history workbench.

## Upstream and Downstream Boundaries

- Downstream of the backend: every `api.ts` calls `runtime.invoke("<domain>.<action>")`
  whose names/payloads are defined in `src-tauri/src/commands/**` and registered
  in `src-tauri/src/lib.rs`.
- Event payloads come from `src-tauri/src/commands/events.rs` (`ai:stream-*`,
  `updater:status`, `window:*`, `shortcut:triggered`).

## Local Search Anchors

- `runtime.invoke(` — every backend call site.
- `runtime.on(` — every event subscription.
- `useTranslation` / `t(` — i18n usage.
- `i18nKeys.test` — per-feature locale-key coverage tests.
- `create(` (Zustand) — store definitions.
- `prompt.bundle` — portable bundle preview/export/import bridge calls.
- `evaluation.` — profile, run, test-set, evaluator, matrix, and label bridge calls.
- `PromptPage` — counted prompt paging contract and store pagination state.

## Generated or Ignored Local Paths

- `vite-env.d.ts` — Vite type shim; not hand-authored content.
- Build output lives in the repo-root `dist/`, not under `src/`.
