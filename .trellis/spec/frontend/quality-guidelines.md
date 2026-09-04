# Quality Guidelines

> Code quality standards for frontend development.

---

## Overview

Frontend quality is enforced by TypeScript, Vitest, Testing Library, property
tests, i18n-key coverage, and architectural tests that protect the Runtime Bridge
boundary. The default local gates for frontend work are `just build` and
`just test` from the repository root.

---

## Forbidden Patterns

- Importing `@tauri-apps/api` outside `src/runtime`.
- Calling backend commands directly from components.
- Bypassing `RuntimeBridge` capability gates.
- Hardcoding user-facing strings in JSX.
- Adding locale keys to only one language bundle.
- Using icon libraries other than `lucide-react`.
- Styling with raw one-off colors where a Tailwind token exists.
- Clamping an Operate overlay form to a reading measure (`max-w-[68ch]` or
  equivalent). `DESIGN.md` 65–75ch applies to prose, not the prompt workbench
  container. See [Component Guidelines](./component-guidelines.md)
  (Prompt detail workbench).
- Editing generated output or the read-only `ref/PromptHub/**` tree.
- Swallowing all async failures silently when the store/view has an error
  channel.

Existing test pattern for import boundaries from
`src/features/settings/components/AppearancePanel.test.tsx`:

```ts
import appearancePanelSrc from "./AppearancePanel.tsx?raw";

expect(appearancePanelSrc).not.toContain("@tauri-apps/api");
```

---

## Required Patterns

- Backend calls go through feature `api.ts` wrappers and `RuntimeBridge`.
- Events go through `runtime.on(...)` or an injected `RuntimeBridge`.
- Components use `useTranslation()` for visible text and accessible labels.
- Tailwind uses token classes: `bg-background`, `bg-card`,
  `text-muted-foreground`, `border-border`, `focus:ring-ring`.
- Stores expose `loading`, `error`, and capability-unavailable flags where the
  UI needs them.
- Async actions catch `unknown`, surface stable messages, and return
  `null`/`false` when the caller needs a result.
- Pure helper logic is tested directly.
- Existing `Req N.M` / `Requirement N` comments are preserved when editing
  nearby code.

---

## Testing Requirements

Use the smallest meaningful test while iterating, then run the required gate for
the touched layer.

Common frontend commands:

```bash
npx vitest run src/features/prompts/folderTree.test.ts
npx vitest run src/features/settings/components/AppearancePanel.test.tsx
just build
just test
```

Test styles in this repo:
- Unit tests for pure helpers: `folderTree.test.ts`, `validation.test.ts`,
  `promptText.test.ts`.
- Property tests with `fast-check` for invariant-heavy helpers and controllers:
  `runtime/i18n.property.test.ts`, `appearance/index.normalize.test.ts`,
  `features/prompts/folderTree.test.ts`.
- Store tests that inject fake APIs and inspect store state:
  `promptStore.test.ts`, `settingsStore.test.ts`, `systemStore.test.ts`.
- Runtime Bridge tests that inject fake Tauri primitives through
  `createRuntimeBridge(deps)`.
- Component tests in jsdom using Testing Library and accessible queries:
  `AppearancePanel.test.tsx`, `SummaryStrip.test.tsx`, `SpecimenCard.test.tsx`.
- Per-feature i18n key tests that assert rendered keys resolve.

Do not use a real Tauri webview for ordinary frontend unit tests.

---

## I18n Quality

When adding UI text:

1. Add dot-notation keys to all seven locale bundles in `src/locales/`.
2. Use `const { t } = useTranslation();` in the component.
3. Add the rendered keys to the feature's `i18nKeys.test.ts`, and assert
   every key in **all seven** locale bundles. Checking only English lets
   missing translations fall back to `en` in the UI.
4. Prefer accessible queries in component tests so missing labels are caught.

Current locale bundles: `en`, `zh`, `zh-TW`, `ja`, `fr`, `de`, and `es`.

---

## Code Review Checklist

Review frontend changes for:

- Runtime Bridge boundary: no raw Tauri imports outside `src/runtime`.
- Command names: wire names match backend `domain.action` strings.
- Types: DTO fields match backend camelCase payloads and nullability.
- State: durable state lives in the feature store; local state is only
  temporary UI state.
- Errors: bridge failures are caught and surfaced consistently.
- Capability gates: `CAPABILITY_UNAVAILABLE` maps to existing unavailable flags.
- I18n: no hardcoded user-facing strings; all locale bundles and key tests
  updated.
- Styling: token classes and Lucide icons match existing UI.
- Accessibility: icon buttons, grouped controls, selections, and inputs are
  labeled.
- Tests: targeted tests cover new logic, and `just build` / `just test` pass for
  frontend work.

---

## Common Mistakes

- Running only component tests after changing types or stores; run `just build`
  too because strict TypeScript catches unused and mismatched code.
- Adding a test that only proves the test setup, not behavior. If deleting the
  feature under test would still pass, the test is not useful.
- Mocking modules when the local pattern is dependency injection through
  `createRuntimeBridge(deps)` or store `api` fields.
- Treating all backend failures the same when a capability-gated path should
  degrade gracefully.
- Calling `window.close` from the ask-close confirm path. CloseAction is still
  `ask`, so that re-emits `window:close-requested`. Use `window.quit`. Hide
  from that dialog must use `window.hide` (tray-aware), not
  `window.toggleVisibility`. See `.trellis/spec/cross-layer/window-close-tray.md`.
