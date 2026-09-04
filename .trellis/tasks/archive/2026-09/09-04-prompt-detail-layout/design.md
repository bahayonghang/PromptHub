# Design — prompt detail content-tab workbench

## Decision

Restore the `08-24-detail-modal` two-pane content tab. Remove the reading-column clamp. Keep header, tabs, footer, draft ownership, and both copy buttons.

## Topology

`PromptDetailModal` content tab (not `PromptEditor.tsx`):

```
form.prompt-editor (container-type: inline-size, unchanged)
  .prompt-editor__workspace
    .prompt-editor__meta-pane
      IdentitySection
      OrganizationSection
      MediaSection
    .prompt-editor__body-pane
      DefinitionSection
      fill-and-copy (when variables exist)
```

### Wide (`@container prompt-editor (min-width: 40rem)`)

- Workspace is a two-column grid: `minmax(0, 1fr) minmax(16rem, 20rem)`, one row `minmax(0, 1fr)`, `overflow: hidden`.
- `grid-template-areas: "body meta"`.
- Meta pane is a scrolling flex column with a left hairline (`border-border`).
- Body pane scrolls independently. Prompt body is the wider column and uses remaining overlay height.
- Field-level two-column queries still key off `.prompt-editor` (full overlay width). Inside the rail, `__two-column` and `__organization` stay one column so title/description/tags are not squeezed into ~10rem halves.

### Narrow (default)

- Meta pane is `display: contents` so its children join the workspace grid.
- Areas: identity, body, org, media (top to bottom). Full width, no 68ch.
- Workspace is the single scroller. Body row uses `minmax(16rem, 1fr)` so the editor stays on screen under Identity; Organization and Media remain reachable below.

## CSS ownership

New rules live in `src/styles/globals.css` next to the existing `@container prompt-editor` block. Field-level queries (`__two-column`, `__message`, `__organization`, `__preview-grid`) stay. The `min-width: 40rem` padding rule on `.prompt-editor__body` continues to serve `PromptEditor.tsx` only.

Do not put `max-w-[68ch]` (or any reading-measure max-width) on the content-tab form or workspace.

Section roots get area classes:

- Identity: `prompt-editor__identity`
- Definition: `prompt-editor__definition`
- Organization section: `prompt-editor__organization-section` (inner grid keeps `__organization`)
- Media: `prompt-editor__media`

`.prompt-editor__body-pane .prompt-editor__definition` drops the top hairline so the body pane does not start with a stray divider. `PromptEditor` still stacks sections and keeps Definition’s `border-t`.

## Unchanged

- Draft, save, validation, dirty dialog, lock placeholder, tabs, shortcuts, i18n.
- Two copy buttons (header + definition). `PromptDetailModal.test.tsx` asserts both.
- Version / run / references tabs.
- `PromptEditor.tsx` layout (tests only; no 68ch there today).

## Compatibility

`display: contents` on the meta pane is a layout-only wrapper, not a landmark. Headings stay on the sections. DOM order is body pane then meta pane so Tab reaches the prompt body before metadata. Grid areas still paint Identity above the body on the narrow stack.

## Rollback

Revert the content-tab markup in `PromptDetailModal.tsx`, the four section class names, and the workspace rules in `globals.css`.
