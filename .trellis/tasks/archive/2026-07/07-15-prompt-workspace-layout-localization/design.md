# Design: Prompt Workspace Layout and Localization

## Component Boundary

`PromptsView` remains the store-backed orchestrator. Extract only focused visual
regions when this reduces its current mixed layout/action responsibility; child
components receive values and callbacks and do not read the store directly.

`PromptEditor` retains local draft ownership. Split section rendering only where
it clarifies identity/organization, definition, and supporting metadata. Do not
create a generic form framework.

## Pane States

- Wide (approximately 1440+ content px): folder, discovery, and detail visible;
  optional history may appear beside detail.
- Default (approximately 1000-1439): all primary panes visible, history overlays
  or replaces a secondary region instead of shrinking the editor excessively.
- Constrained (approximately 800-999): folder navigation collapses behind a
  labeled control; discovery and detail use a master/detail transition.

Use CSS/container behavior rather than JavaScript window listeners where
possible. The selected prompt and draft stay mounted or are lifted so structural
visibility changes never silently reset work.

## Editor Layout

Use plain section headings and dividers, not cards. The definition section gets
the strongest spatial priority. Metadata fields use a responsive grid that
stacks when their minimum width cannot be maintained. Keep the action footer at
the stable bottom edge and the form body as the only vertical scroll owner.

## Localization Contract

Treat English as the canonical key shape. Recursively assert key parity and
non-empty string leaves for `promptsView` in all supported bundles. Preserve
i18next fallback for runtime resilience, but tests make fallback unnecessary for
this feature during normal use.

Translations must keep product terminology consistent: Prompt remains the
product object name where established, while actions, labels, hints, empty
states, and accessibility names are fully localized.

## Accessibility and Motion

Use native landmarks/controls where available, stable focus when panes change,
and visible text labels for pane-switch controls. State transitions are 150-200
ms opacity/transform at most and become immediate under reduced motion. Content
is visible without animation initialization.
