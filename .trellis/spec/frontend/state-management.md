# State Management

> How state is managed in this project.

---

## Overview

Frontend state uses Zustand. Each major feature owns a store that wraps its
backend-backed state and async actions. Components keep only temporary UI state.
There is no React Query/SWR server-state library; feature stores load, cache,
refresh, and surface errors directly.

Existing stores:
- `src/store/appStore.ts` - app shell readiness, active view, sidebar state
- `src/features/prompts/promptStore.ts` - prompts, folders, tags, filters,
  selection, versions
- `src/features/settings/settingsStore.ts` - settings, security, data path,
  recovery, sync, backups
- `src/features/system/systemStore.ts` - window state, updater, shortcuts,
  runtime paths, events

---

## State Categories

Use component-local state for:
- form drafts and inputs before submit
- inline create/rename/edit fields
- expanded/collapsed UI affordances
- temporary busy/result flags inside a panel
- drag/drop ids and hover targets

Use a Zustand store for:
- data loaded from backend commands
- selected entity ids shared by multiple child components
- overlay-open flags shared by library, palette, and shortcuts
- active filters that affect backend search
- capability-unavailable flags
- durable error/loading/restart indicators
- event-driven state from `runtime.on(...)`

Use pure helpers for derived transformations that should be tested without
React, such as `buildFolderTree`, `wouldCreateCycle`, and
`downloadProgressPercent`.

---

## Store Shape

Stores use one interface containing the injected API, state fields, and actions:

```ts
interface PromptStoreState {
  api: PromptApi;
  prompts: Prompt[];
  selectedPromptId: string | null;
  loading: boolean;
  error: string | null;

  load: () => Promise<void>;
  selectPrompt: (id: string | null) => Promise<void>;
}
```

Actions read dependencies with `get()`, update state with `set()`, catch bridge
failures, and return nullable values or booleans when the caller needs to know
whether the operation succeeded.

```ts
createPrompt: async (input) => {
  const { api } = get();
  set({ error: null });
  try {
    const prompt = await api.createPrompt(input);
    await get().refreshPrompts();
    await get().selectPrompt(prompt.id);
    get().closeDetail();
    return prompt;
  } catch (err) {
    set({ error: errorMessage(err) });
    return null;
  }
}
```

---

## Backend State and APIs

Backend access belongs in feature `api.ts` wrappers. Store modules depend on the
feature API interface, not raw Tauri calls.

```text
component -> useFeatureStore action -> feature api.ts -> RuntimeBridge.invoke
```

Feature API wrappers:
- define an interface such as `PromptApi` or `SettingsApi`
- expose `create<Feature>Api(bridge: RuntimeBridge = runtime)`
- export a production singleton such as `promptApi`
- call commands by wire names like `prompt.search`, `settings.update`,
  `window.minimize`

This pattern keeps tests simple: stores can swap `api` with a fake using
`useFeatureStore.setState({ api: fakeApi })`.

---

## Derived State

Prefer exported selector functions for derived store state that multiple callers
or tests need:

```ts
export function selectSelectedPrompt(state: PromptStoreState): Prompt | null {
  if (state.selectedPromptId == null) return null;
  return state.prompts.find((p) => p.id === state.selectedPromptId) ?? null;
}
```

Prefer pure helpers for derived algorithms:
- `src/features/prompts/folderTree.ts`
- `src/features/prompts/promptText.ts`
- `src/features/settings/validation.ts`
- `src/features/system/systemStore.ts` for `downloadProgressPercent`

When derived state depends on current backend state and needs persistence, keep
it inside a store action rather than recomputing in multiple components.

---

## Prompt detail overlay

`detailOpen` is independent of `selectedPromptId`. `CommandPalette` is a sibling
of `PromptsView` and opens a Prompt through `requestSelectPrompt`, so overlay
visibility cannot live only in the view.

```text
open = creating || (detailOpen && selectedPrompt != null)
```

`creating` stays local in `PromptsView` (a create draft is not a selected row).

| Action | Selection | Overlay |
|---|---|---|
| `selectPrompt(id)` | loads the row | unchanged |
| `requestSelectPrompt(id)` when `id != null` | `selectPrompt(id)` after the nav guard | `detailOpen = true` |
| `requestSelectPrompt(null)` | clears selection after the nav guard | `detailOpen = false` |
| `createPrompt` | persist, refresh, `selectPrompt(newId)` | `closeDetail()` |
| `closeDetail()` | unchanged | `detailOpen = false` |
| view dismiss (`onClose`) | `selectPrompt(null)` | `closeDetail()` plus local `creating = false` |

Create-mode **Create**, `Ctrl+S`, and dirty **Save and close** must not call
`onClose`. That callback deselects. After a successful create the library keeps
the new row highlighted and the overlay stays closed until
`requestSelectPrompt(id)` runs again.

---

## Error and Capability Patterns

Bridge failures are turned into strings through local `errorMessage(err)`
helpers. Stores expose `error: string | null` for views to render.

Capability-gated command failures are detected by `err.code ===
"CAPABILITY_UNAVAILABLE"` and surfaced as feature-specific flags:
- `recoveryUnavailable` in `settingsStore.ts`
- `windowControlsUnavailable` and `updaterUnavailable` in `systemStore.ts`

Best-effort side loads should not block primary state. `settingsStore.load()`
loads backups after the main settings/security/data status load and treats
backup failure as non-fatal.

---

## Common Mistakes

- Calling `runtime.invoke` from components instead of using the feature API and
  store.
- Keeping selected ids, filters, or backend result lists in a child component.
- Updating optimistic UI state before the backend accepts an operation when the
  current store waits for success, such as shortcut registration.
- Dropping capability failures into a generic error when existing UI expects a
  dedicated unavailable flag.
- Duplicating selector logic in JSX instead of exporting a selector or pure
  helper.
- Deriving the Prompt detail overlay from `selectedPrompt != null`.
  `createPrompt` selects the new row (Req 6.1); that derivation keeps the
  overlay open as an editor after **Create**. Use `detailOpen` instead.
