# Directory Structure

> How frontend code is organized in this project.

---

## Overview

The frontend lives under `src/**`. It is organized around a small set of
cross-cutting modules plus feature modules under `src/features/<name>/`.
Feature modules own their bridge wrappers, Zustand store, DTO types, view, child
components, and tests.

Before doing broad search in `src/**`, read `src/code_map.md` and use its
anchors (`runtime.invoke(`, `runtime.on(`, `useTranslation`, `i18nKeys.test`,
`create(`) to jump to the right layer.

---

## Directory Layout

```text
src/
  main.tsx                    # React bootstrap, theme + i18n initialization
  App.tsx                     # app readiness and fatal-error shell
  runtime/                    # Runtime Bridge and i18n setup
  store/                      # app-level Zustand store
  theme/                      # light/dark/system theme controller
  appearance/                 # appearance token controller and catalogs
  styles/globals.css          # Tailwind layers and CSS-variable tokens
  locales/*.json              # seven translation bundles
  components/
    layout/                   # app shell, sidebar slot, header, nav config
                              # Sidebar does not import the prompt store; it
                              # renders PromptLibraryNav as a child slot.
    views/                    # shared top-level view placeholders
  features/
    prompts/
      api.ts                  # typed prompt/folder/version bridge calls
      promptStore.ts          # prompt view Zustand store
      types.ts                # prompt DTO mirrors
      folderTree.ts           # pure tree/reorder helpers
      components/
      *.test.ts
      i18nKeys.test.ts
    settings/
    system/
```

Generated or local-only files are not source: `dist/`, `src-tauri/target/`,
`src-tauri/gen/schemas/`, `node_modules/`, and `ref/` must not be edited for
frontend work.

---

## Module Organization

Use feature modules for business behavior. Existing examples:

- `src/features/prompts/` owns prompt CRUD, folder tree behavior, prompt version
  history, prompt editor UI, and prompt i18n key coverage.
- `src/features/settings/` owns settings bridge calls, settings store, validation
  helpers, settings panels, appearance-panel tests, and settings i18n keys.
- `src/features/system/` owns window controls, updater state, shortcuts,
  notifications, runtime-path panels, and system event subscriptions.

Keep each feature in this shape when adding behavior:

```text
src/features/<feature>/
  api.ts                  # create<Feature>Api(bridge = runtime)
  <feature>Store.ts       # use<Feature>Store = create<...>((set, get) => ...)
  types.ts                # frontend mirrors of backend DTOs
  <Feature>View.tsx       # state orchestration and page layout
  components/             # presentational and focused interaction components
  *.test.ts[x]            # colocated unit/component tests
  i18nKeys.test.ts        # rendered-key coverage when the feature has UI text
```

Put reusable pure logic in a plain `.ts` helper next to the feature, not inside a
component. `src/features/prompts/folderTree.ts` and
`src/features/settings/validation.ts` are the local examples.

---

## Naming Conventions

- Component files use PascalCase: `PromptList.tsx`, `AppearancePanel.tsx`,
  `WindowControls.tsx`.
- Feature stores use `use<Feature>Store` exported from `<feature>Store.ts`, such
  as `usePromptStore`, `useSettingsStore`, and `useSystemStore`.
- Feature bridge factories use `create<Feature>Api`, with a production singleton
  such as `promptApi`.
- Types live in `types.ts`, use exported interfaces for DTOs and props, and use
  literal unions for bounded values.
- Tests are colocated with the module under test: `folderTree.test.ts`,
  `AppearancePanel.test.tsx`, `runtime/index.test.ts`.
- Locale-key tests are named `i18nKeys.test.ts`.

---

## Examples

Bridge-wrapper module shape from `src/features/prompts/api.ts`:

```ts
export interface PromptApi {
  listPrompts(): Promise<Prompt[]>;
  createPrompt(input: CreatePromptInput): Promise<Prompt>;
}

export function createPromptApi(bridge: RuntimeBridge = runtime): PromptApi {
  return {
    listPrompts: () => bridge.invoke<Prompt[]>("prompt.list"),
    createPrompt: (input) => bridge.invoke<Prompt>("prompt.create", { input }),
  };
}
```

Pure helper placement from `src/features/prompts/folderTree.ts`:

```ts
export function wouldCreateCycle(
  folders: readonly Folder[],
  folderId: string,
  targetParentId: string | null,
): boolean {
  if (targetParentId == null) return false;
  return collectSubtreeIds(folders, folderId).has(targetParentId);
}
```

Shared layout belongs outside features when it is truly app-wide:
`src/components/layout/Sidebar.tsx`, `src/components/layout/Header.tsx`, and
`src/components/layout/AppShell.tsx`.

---

## Common Mistakes

- Do not put feature backend calls directly in components. Add or extend the
  feature's `api.ts`, then call through the store or an injected prop.
- Do not create a new top-level folder under `src/` for a one-feature concern.
  Keep feature-specific code under `src/features/<feature>/`.
- Do not duplicate pure helper logic inside multiple components. Extract a local
  helper next to the feature and test it directly.
- Do not add user-facing strings without updating every file in `src/locales/`
  and the relevant `i18nKeys.test.ts`.
