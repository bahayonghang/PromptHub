# Design: Prompt Filter Panel

## Interaction Model

Keep the keyword input and filter trigger in the discovery toolbar. Open a
controlled, non-modal surface anchored to the trigger and rendered outside the
scrolling results region. Use React's existing portal support only if the final
workspace stacking/overflow contexts require it.

The surface uses vertical groups:

1. favorites checkbox;
2. sort field and direction as two labeled full-width controls;
3. tag toggle list;
4. applied-state summary and clear action when relevant.

This avoids the current single flex line and keeps translation growth local.

## State and Ownership

Open/close and focus refs remain transient component state. Filter values remain
owned by the prompt store and flow through the existing `onChange`,
`onToggleTag`, and `onClear` callbacks. Do not duplicate filter state in the
popover.

## Accessibility

The trigger names the action and owns `aria-expanded`/`aria-controls`. The
surface has an accessible heading/name but is non-modal, so users can move to
the search field or results without a focus trap. Escape closes and restores the
trigger; normal Tab order reaches every control. Tag toggles retain
`aria-pressed`; the favorite remains a native checkbox.

## Geometry

Use a bounded width that fits the discovery pane and viewport, vertical spacing
from the documented 4/8/12/16 rhythm, a semantic overlay z-index, and one tonal
surface/border. It is an overlay tool, not a nested content card. Long tags wrap
inside their own buttons without resizing the toolbar.
