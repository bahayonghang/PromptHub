# Prompt Workspace UX Audit

## Scope and Evidence

This audit covers the Prompts management and editing workspace shown in the four
2026-07-15 screenshots. Evidence was verified against `PromptsView.tsx`,
`SearchBar.tsx`, `PromptEditor.tsx`, `FolderTree.tsx`, prompt locale tests, the
prompt/folder services, storage migrations, `PRODUCT.md`, and `DESIGN.md`.

## Audit Health Score

| Dimension | Score | Key finding |
| --- | ---: | --- |
| Accessibility | 2/4 | Good labels exist, but the filter disclosure semantics, compact targets, and text-scaling behavior need work. |
| Performance | 3/4 | Store selectors and local draft state are sound; no expensive visual effects were found. |
| Responsive design | 1/4 | Fixed 224 px and 320 px panes leave an unusable editor at the supported 800 px minimum. |
| Theming | 4/4 | Prompt surfaces consistently use semantic Tailwind tokens and inherit the existing theme system. |
| Anti-patterns | 3/4 | The UI is product-like and restrained, but the editor hierarchy is flat and the filter is framed too tightly. |
| **Total** | **13/20** | **Acceptable: significant focused work is needed.** |

## Anti-Patterns Verdict

Pass. The page does not look like a generic AI-generated SaaS dashboard: it has
no gradients, glass effects, decorative metrics, oversized cards, or gratuitous
motion. The problems are conventional product-UI defects: rigid geometry,
incomplete localization, weak information hierarchy, and insufficient state
coverage.

## Findings

### P1: Filter controls overflow the list pane

- Location: `SearchBar.tsx:82-118`, especially `ml-auto` at line 92.
- Impact: the "Sort by" label wraps and both selects compete for a 320 px pane;
  translated or scaled labels make the layout less usable.
- Recommendation: use a stable anchored filter surface with vertically grouped
  fields, explicit disclosure semantics, and viewport/list-pane constraints.

### P1: The shell has no narrow-window structure

- Location: `PromptsView.tsx:123`, `PromptsView.tsx:139`; supported minimum at
  `tauri.conf.json:18`.
- Impact: at 800 px the two fixed navigation panes consume 544 px before borders
  and padding, leaving the editor too narrow for its fixed two-column fields.
- Recommendation: define structural pane states for minimum, default, and wide
  widths; collapse secondary panes instead of squeezing the editor.

### P1: Locale fallback creates a mixed-language workbench

- Location: `i18nKeys.test.ts:156-202`, `i18n.ts:96-107`.
- Impact: a Simplified Chinese session visibly mixes Chinese and English labels,
  reducing trust and scanability.
- Recommendation: require every rendered Prompts key in every shipped bundle and
  reserve English fallback for resilience, not normal UI rendering.

### P1: Custom type requests conflict with a storage invariant

- Location: `types.ts:10-13`, `models/enums.rs:10-20`, `prompt.rs:151-170`,
  `storage/mod.rs:427` and `storage/mod.rs:458`.
- Impact: changing the select to accept free text would fail backend validation
  and break revision, bundle, mapping, and evaluation assumptions.
- Recommendation: keep the base format enum and add a separate persisted type
  definition with a user name and one required base format.

### P2: Folder creation is available but contextually distant

- Location: `FolderTree.tsx:26-74`, `PromptEditor.tsx:296-312`,
  `promptStore.ts:451-461`.
- Impact: users must leave the editor, create in the tree, then return and find
  the new value; this interrupts draft flow.
- Recommendation: add a focused inline create affordance that reuses the store
  action and selects the returned folder only on success.

### P2: Editor fields form one long, flat sequence

- Location: `PromptEditor.tsx:242-615`.
- Impact: metadata, definition, variables, media, provenance, and notes have
  equal visual weight, slowing repeated authoring and review.
- Recommendation: use plain section bands, headings, dividers, and responsive
  field groups; keep cards out of the form and retain the sticky action footer.

### P2: Filter disclosure semantics and target sizing are incomplete

- Location: `SearchBar.tsx:57-71`.
- Impact: `aria-pressed` does not describe a disclosure relationship, and the
  36 px trigger plus 10 px count badge fall below the documented component ramp.
- Recommendation: add `aria-expanded`/`aria-controls`, stable focus restoration,
  and documented compact control dimensions.

### P2: Component tests do not cover these interactions

- Location: `src/features/prompts/**`; no SearchBar or PromptEditor component
  test currently exercises filter geometry/semantics or inline creation.
- Impact: locale, keyboard, failure, and narrow-width regressions can pass the
  current store/API suite.
- Recommendation: add Testing Library behavior tests plus browser screenshots at
  the supported window sizes and text scales.

### P3: Two 10 px badges sit outside the design typography ramp

- Location: `SearchBar.tsx:71`, `PromptList.tsx:113`.
- Impact: small status text becomes fragile under scaling.
- Recommendation: use the documented label step or a non-text status cue.

## Positive Findings

- The existing interface uses semantic theme tokens and Lucide icons consistently.
- Search fields and editor controls have associated labels; selected tags use
  `aria-pressed`, and major icon-only prompt actions have accessible names.
- Prompt data access already follows component -> Zustand -> API -> Runtime
  Bridge -> command, so folder inline creation needs no new backend command.
- Ordered migrations, complete revisions, portable bundles, and evaluation links
  already provide the compatibility seams required by custom type definitions.

## Recommended Order

1. Establish the responsive workspace and locale completeness baseline.
2. Implement and verify the filter surface inside that layout.
3. Add inline folder creation through the existing folder action.
4. Add custom type definitions as the only cross-layer/schema child.
5. Run a parent-level accessibility, visual, native, and `just ci` review.
