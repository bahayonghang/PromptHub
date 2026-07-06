# Hook Guidelines

> How hooks are used in this project.

---

## Overview

The codebase currently has no dedicated `hooks/` directory and no custom hook
files. Stateful shared behavior is represented by Zustand store hooks such as
`usePromptStore`, `useSettingsStore`, `useSkillStore`, `useSystemStore`, and
`useAppStore`. Component-local React hooks are used for effects, memoized
derived values, refs, and temporary UI drafts.

Document reality: do not introduce a custom-hook layer unless it removes real
duplication or matches an existing local pattern.

---

## Built-In React Hook Patterns

Use `useEffect` for initial loads and subscriptions:

```tsx
const load = usePromptStore((s) => s.load);

useEffect(() => {
  void load();
}, [load]);
```

Use `useMemo` when a render-only derived value is non-trivial and pure. Existing
example from `src/features/prompts/components/FolderTree.tsx`:

```tsx
const tree = useMemo(() => buildFolderTree(folders), [folders]);
```

Use `useState` for transient UI state only:
- open/closed filters
- inline create/rename text
- form drafts before submit
- local spinner/result flags
- currently dragged/drop-target ids

Do not store backend-backed data in component `useState` when a feature store
already owns that domain.

---

## Zustand Hook Patterns

Stores are exported as hooks but are not treated like custom React hooks. They
live in store modules and are created with `zustand/create`:

```ts
export const usePromptStore = create<PromptStoreState>((set, get) => ({
  api: promptApi,
  prompts: [],
  selectedPromptId: null,
  error: null,
  load: async () => {
    const { api } = get();
    set({ loading: true, error: null });
    // ...
  },
}));
```

Components select only the slices they need:

```tsx
const prompts = usePromptStore((s) => s.prompts);
const selectedPrompt = usePromptStore(selectSelectedPrompt);
const selectPrompt = usePromptStore((s) => s.selectPrompt);
```

Avoid reading a whole store object in a component unless the component truly
needs every field. Narrow selectors keep renders more predictable.

---

## Data Fetching

There is no React Query, SWR, or custom fetch hook. Data access follows this
path:

```text
component -> Zustand action -> feature api.ts -> Runtime Bridge -> Tauri command
```

Feature APIs accept an injectable bridge:

```ts
export function createSettingsApi(bridge: RuntimeBridge = runtime): SettingsApi {
  return {
    getSettings: () => bridge.invoke<Settings>("settings.get"),
    updateSettings: (patch) => bridge.invoke<Settings>("settings.update", { patch }),
  };
}
```

Tests inject fake APIs into stores or fake bridge primitives into
`createRuntimeBridge(deps)`. Do not mock `@tauri-apps/api` in component tests.

---

## Future Custom Hooks

If a custom hook becomes necessary:

- Name it `useSomething`.
- Keep it near the feature that owns the behavior unless multiple features
  already use it.
- Do not hide backend command names inside a hook; backend access still belongs
  in `api.ts` and store actions.
- Return stable values/actions with clear names rather than a mixed bag object.
- Test the pure helper/store behavior underneath the hook first.

Good candidates are repeated UI-only behavior across multiple components.
Backend orchestration, command routing, and cross-component domain state are not
good candidates in this codebase.

---

## Common Mistakes

- Creating `src/hooks/` for one local behavior.
- Calling `runtime.invoke` inside a hook used by components.
- Putting long-lived domain state in `useState` instead of the feature store.
- Forgetting `void` for intentionally unawaited async work inside event handlers
  or effects.
- Omitting effect dependencies to silence reruns instead of making the dependency
  stable or moving logic into the store.
