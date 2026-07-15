# Fix prompt filter panel presentation

## Goal

Replace the cramped inline filter disclosure with a stable, localized, keyboard-
operable interaction that fits the discovery pane at every supported width and
font scale.

## Confirmed Defects

- The disclosure is rendered inline inside a fixed 320 px discovery pane.
- At `SearchBar.tsx:92`, a flex label uses `ml-auto` while containing its text
  plus sort-field and sort-order selects; the combined intrinsic width forces
  the label to wrap and pushes controls against the panel edge.
- The trigger uses `aria-pressed` instead of disclosure semantics
  (`SearchBar.tsx:61`) and is 36 px (`SearchBar.tsx:63`).
- There is no component test for open/close, keyboard behavior, long labels,
  tag selection, clearing, or non-default sort state.

## Requirements

- R1: Present favorites, sort field, sort direction, and tag filters in a stable
  field hierarchy with no horizontal competition between labels and controls.
- R2: Use a non-modal anchored surface that does not consume result-list height
  or become clipped by the list scroll container.
- R3: Expose `aria-expanded`, `aria-controls`, an accessible surface name, and
  predictable focus. Escape and outside click close; focus returns to the trigger.
- R4: Preserve current conjunctive tag behavior, favorite behavior, sort values,
  active count, default values, and store-driven filtering.
- R5: Make applied state legible without relying only on the accent color. Clear
  resets tag/favorite filters and leaves the documented default sort behavior.
- R6: Handle zero tags, many tags, long tag names, all locales, 200% text scale,
  loading/filter updates, and the 800 px application window.
- R7: Use existing tokens and Lucide icons; do not add a popover dependency or a
  generic overlay framework for this single control.

## Acceptance Criteria

- [x] AC1: The filter surface never wraps sort labels into isolated words, clips
  controls, or produces horizontal scrolling at supported widths/text scales.
- [x] AC2: Trigger, surface, checkbox, selects, tags, and clear action are fully
  keyboard operable with correct expanded/pressed/checked semantics.
- [x] AC3: Escape, outside click, and a second trigger click close the surface;
  focus restoration is tested.
- [x] AC4: Tag/favorite filters, sort field/order, active count, and clear behavior
  produce the same `PromptFilters` patches as before.
- [x] AC5: Empty tags, 30 tags, long labels, Chinese, German, and 200% text scale
  have deterministic visual snapshots with no overlap.
- [x] AC6: Targeted SearchBar tests, `just build`, and `just test` pass.

## Out of Scope

- New filter dimensions, saved filters, query syntax, backend search changes, or
  changing folder filtering in the folder tree.
- The parent workspace restructure or custom type/folder creation controls.
