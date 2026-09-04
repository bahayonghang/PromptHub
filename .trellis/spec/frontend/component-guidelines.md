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

### Async Inline Creation

When an editor creates a related entity inline, inject the existing store action
as an async callback that returns the authoritative entity or `null`:

```tsx
onCreateFolder: (input: CreateFolderInput) => Promise<Folder | null>;

const folder = await onCreateFolder({ name: name.trim(), parentId: null });
if (!folder) return; // Keep the input open for correction.
onChange(folder.id);
```

- Keep the temporary name, validation, busy state, and focus refs local to the
  focused child component; do not duplicate backend-backed lists locally.
- Mirror simple backend validation for immediate feedback, while keeping the
  store/backend authoritative. Empty or overlong input must not call the action.
- Disable every submit path while awaiting the callback. On `null`, retain the
  name and existing selection; on success, select only the returned id.
- Enter submits without bubbling to the parent form, Escape cancels, cancel
  returns focus to the disclosure trigger, and success focuses the picker.
- Component tests must cover validation, duplicate-submit prevention, failure
  retention, focus restoration, and preservation of the surrounding form draft.

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

### Prompt copy control

Library list, library grid, the detail overlay header, and the Prompt content
heading share `CopyPromptButton`. The control sits immediately before the title
(or the Prompt content heading), as a sibling of the title activator, never
nested inside it. Hit target is `h-9 w-9`; the `ClipboardCopy` / `Check` icon
is `h-5 w-5`.

Success is the in-control `Check` for about 1.5 s plus a `ToastHost` success
toast with `replaceGroup: "prompt-copy"`. Failure stays on the control and may
push a danger toast in the same group. Persist usage only after `writeText`
succeeds; see `.trellis/spec/cross-layer/prompt-library-foundations.md`
(Recording a clipboard copy as usage).

### Responsive Container Layouts

Feature-internal panes and forms should respond to their content region rather
than the full window. The checked-in Tailwind configuration does not provide a
container-query plugin, so do not assume `@container` or `@[...]:` utility
classes will emit CSS. Use a named native container and semantic child classes
in `src/styles/globals.css`:

```css
.feature-workspace {
  container: feature-workspace / inline-size;
}

@container feature-workspace (min-width: 56rem) {
  .feature-workspace__navigation {
    display: flex;
  }
}
```

Keep these override rules outside Tailwind's `components` layer when they must
win over utilities such as `hidden`, `invisible`, `absolute`, or `w-*`.
Component-layer rules have lower cascade priority than utilities even when they
appear later in the source.

Wrong:

```tsx
<div className="@container/workspace">
  <aside className="hidden @[56rem]/workspace:flex" />
</div>
```

Correct:

```tsx
<div className="feature-workspace">
  <aside className="feature-workspace__navigation hidden" />
</div>
```

After changing a container layout, run the production build and verify in a
browser that `getComputedStyle(container).containerType === "inline-size"` at
the required viewport sizes. Closed off-canvas regions must also disappear from
the accessibility tree, not only move outside the visible canvas.

### Overlay Tools in Clipped Workspaces

Workbench roots such as `prompt-workspace` use `overflow-hidden` to contain
off-canvas panes. A dropdown or filter surface positioned inside that root will
be clipped even when its own z-index is correct. Render non-modal overlay tools
through a fixed-position portal, measure against the viewport, and flip above
the trigger when the space below is smaller.

```tsx
const surface = createPortal(
  <div
    role="region"
    style={{ position: "fixed", left, width, top, maxHeight }}
    className="overflow-y-auto"
  />,
  document.body,
);
```

- Keep at least 16 px between the surface and viewport edges.
- Bound the height to the available side and make overflowing content scroll.
- Recompute on resize, captured scroll, and owner-size changes.
- A portal changes DOM tab order. Focus the first surface control when it opens,
  return focus to the trigger on Escape, and keep outside-click checks scoped to
  the trigger plus the portaled surface.
- Test the geometry in a real browser at the minimum viewport and 200% text
  scale; component tests should also assert the portal parent and flip direction.

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
- Calling the backend from a component instead of the feature `api.ts`.
- Using raw color classes when a token class exists.
- Adding an icon from a library other than `lucide-react`.
- Letting a component grow both backend orchestration and detailed rendering;
  split panels/children or move orchestration into the store.
