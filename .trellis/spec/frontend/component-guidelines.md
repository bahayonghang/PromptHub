# Component Guidelines

> How React components are built in this project.

---

## Overview

Components are named function components written in TypeScript. They use local
props interfaces, `useTranslation()` for all rendered text, Tailwind token
classes for styling, and `lucide-react` for icons. Feature views orchestrate
stores and data loading; child components receive values and callbacks.

Existing examples to follow:
- `src/features/prompts/components/PromptList.tsx`
- `src/features/prompts/components/FolderTree.tsx`
- `src/features/settings/components/AppearancePanel.tsx`
- `src/features/system/components/WindowControls.tsx`

---

## Component Structure

Keep component files in this order:

1. React, i18n, icon, and local imports.
2. Local props interfaces and small local helper components.
3. Local class-name constants when repeated.
4. Exported component.
5. Small private helper functions only when they are component-specific.

Example from `src/features/prompts/components/PromptList.tsx`:

```tsx
interface PromptListProps {
  prompts: Prompt[];
  selectedPromptId: string | null;
  loading: boolean;
  onSelect: (id: string) => void;
}

export function PromptList({
  prompts,
  selectedPromptId,
  loading,
  onSelect,
}: PromptListProps) {
  const { t } = useTranslation();
  // render states...
}
```

Small private subcomponents are fine when they make the exported component
clearer. `TypeBadge` in `PromptList.tsx` maps prompt kind to Lucide icons without
forcing the caller to know that mapping.

---

## Props Conventions

- Define props as `interface <ComponentName>Props` unless the props are tiny and
  truly private.
- Use domain types from the feature's `types.ts`; do not redefine DTO shapes in
  components.
- Pass state down as values and mutations up as callbacks. Child components do
  not reach into stores unless they are intentionally store-backed panels.
- Prefer explicit callback names: `onSelectFolder`, `onCreateFolder`,
  `onRenameFolder`, `onReorder`, `onReparent`.
- Use injectable props for external dependencies that tests need to replace.
  `AppearancePanel` accepts `invoke`, `controller`, and `changeLocaleFn`.

Example from `src/features/settings/components/AppearancePanel.tsx`:

```tsx
export interface AppearancePanelProps {
  invoke?: RuntimeBridge["invoke"];
  controller?: AppearanceController;
  changeLocaleFn?: (locale: SupportedLocale) => Promise<void>;
}
```

---

## Composition Patterns

Feature views should connect stores and panels. For example,
`src/features/prompts/PromptsView.tsx` selects prompt store slices, triggers
`load()` in an effect, and passes the resulting values/callbacks into prompt
list, folder tree, editor, and history components.

Presentational children should keep only short-lived UI state:
- inline edit values in `FolderTree.tsx`
- filter expansion in `SearchBar.tsx`
- form drafts in editor/panel components

Move durable, cross-component, backend-backed, or view-wide state into the
feature store.

---

## Styling Patterns

Styling uses Tailwind classes backed by CSS-variable design tokens. Prefer token
classes over raw colors:

```tsx
className={`rounded-md border px-3 py-2 text-sm transition-colors ${
  active
    ? "border-primary bg-primary/15 text-foreground"
    : "border-input text-muted-foreground hover:bg-accent hover:text-foreground"
}`}
```

Use local class constants when the same class string appears repeatedly:

```tsx
const labelClass = "text-sm font-medium text-foreground";
const selectClass =
  "w-full max-w-xs rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground outline-none focus:ring-1 focus:ring-ring";
```

Icons come from `lucide-react`. Decorative icons use `aria-hidden="true"`;
action icons need an accessible label on the button or icon.

---

## Accessibility

- Every `<button>` has `type="button"` unless it intentionally submits a form.
- Icon-only buttons need `aria-label` and often `title` too.
- Selected navigation/list rows use `aria-current` or `aria-pressed`, matching
  existing components.
- Grouped option controls use `role="group"` with an i18n-backed `aria-label`.
- Form controls need labels or `aria-label` sourced from i18n.
- Hidden hover actions must still be keyboard reachable when visible focus
  behavior is added; avoid mouse-only workflows for new controls.

Examples in the current code:
- `PromptList.tsx` uses `aria-current` for selected rows.
- `AppearancePanel.tsx` uses `aria-pressed` for option buttons and `role="group"`
  for segmented option sets.
- `FolderTree.tsx` gives folder action buttons i18n `title` and `aria-label`.

---

## User-Facing Text

All text that a user can see or hear goes through i18n:

```tsx
const { t } = useTranslation();

<button aria-label={t("promptsView.newFolder")}>
  <FolderPlusIcon aria-hidden="true" />
</button>
```

When adding or renaming keys:
- update every file in `src/locales/*.json`
- update the relevant `src/features/<feature>/i18nKeys.test.ts`
- use dot-notation keys such as `settingsView.appearance.fontScale`

---

## Common Mistakes

- Hardcoding English labels in JSX.
- Importing `@tauri-apps/api` or calling the backend from a component.
- Using raw color classes when a token class exists.
- Adding an icon from a library other than `lucide-react`.
- Letting a component grow both backend orchestration and detailed rendering;
  split panels/children or move orchestration into the store.
