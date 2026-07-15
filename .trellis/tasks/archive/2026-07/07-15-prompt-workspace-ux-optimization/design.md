# Design: Prompt Workspace UX and Organization

## Product Boundary

The target is the existing local-first Prompts workbench. Preserve compact
desktop density, direct manipulation, current semantic tokens, and all existing
prompt lifecycle capabilities. Improve structure and interaction rather than
turning the page into a dashboard or a collection of cards.

## Workspace Structure

Use a three-region workbench with explicit responsive states:

```text
folder navigation | prompt discovery | active prompt workspace
```

- Wide/default: show all three regions with bounded navigation tracks and a
  `minmax(0, 1fr)` editor.
- Constrained: collapse folder navigation behind a standard labeled control.
- Minimum supported width: show discovery or detail as the primary region;
  switching regions must preserve selection, filter, and unsaved draft state.
- History and evaluation are secondary workspaces and must not reduce the editor
  below its usable minimum.

Do not add resizable splitters in this scope. Stable structural states solve the
supported 800-1200 px range with less state and less testing burden.

## Editor Hierarchy

Keep one scroll owner and the existing persistent footer action area. Divide the
form into unframed sections with headings/dividers:

1. identity and privacy;
2. organization (base/custom type, folder, tags);
3. prompt definition and variables;
4. media and provenance.

Fields may form two columns only when their container has sufficient inline
space. At constrained widths they stack. Cards must not be nested around these
sections.

## Filter Interaction

Use a non-modal anchored filter surface rendered outside the scrolling result
region. The surface has a stable bounded width, vertical field groups, long-label
wrapping only at semantic boundaries, and a clear applied-state summary. The
trigger exposes `aria-expanded` and `aria-controls`; Escape and outside click
close it and focus returns to the trigger.

## Organization Contracts

Folder creation reuses `folder.create` and the existing store action. The editor
owns only temporary input/open/busy state. A successful response updates the
folder list and draft selection atomically from the user's perspective.

Custom prompt types use two concepts:

- `PromptType` remains the stable base format: `text`, `image`, or `video`.
- `PromptTypeDefinition` supplies a unique user-facing name and required base
  format. Built-in formats remain virtual default choices; custom definitions
  are persisted and referenced by prompts/revisions/bundles.

Execution and evaluation branch only on the base format. A custom definition is
organizational metadata and cannot register behavior.

## Localization and Accessibility

English remains the fallback locale, but tests require every rendered Prompts
key in all seven bundles. Controls retain visible labels, predictable focus,
non-color state cues, and keyboard parity. Test text scaling separately from
viewport resizing because the appearance controller changes the root font size.

## Compatibility

- Existing prompts with no custom definition continue to render their built-in
  base format and require no destructive rewrite.
- Type-definition migrations are additive and transactional.
- Revisions and portable manifests include the definition reference and enough
  definition data to preserve meaning across machines.
- Folder behavior and ids remain unchanged.
- All outbound/evaluation behavior continues through existing backend policies.

## Rollback

Each child is independently revertible. The layout, filter, and folder children
do not change persisted prompt data. The type child lands last and requires a
pre-migration disposable backup/restore rehearsal before parent integration.
