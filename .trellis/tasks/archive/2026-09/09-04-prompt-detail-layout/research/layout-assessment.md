# Assessment A — layout and design review

Visitor mode: Operate. Surface: Prompt detail overlay, 内容 tab.
Agent: explore `01a06c75-1f3c-7331-afb3-53b892d90341`. Isolated from detector.

## Spatial thesis

The overlay **shell** is a workbench (`min(1180px, 100%)` × `min(90vh, 56rem)`). The **内容 tab** is a centered reading column (`max-w-[68ch] mx-auto`, `gap-7`) stacking Identity → Definition → Organization → Media. Primary Operate work (edit body, copy, save) is the second section, inside ~34rem, while the card stays ~1180px. Gutters of roughly 18–20rem each side are unused.

Container queries key off the full-width form, not the 68ch column, so field-level two-column rules fire while the visible measure stays narrow.

## Heuristics (this surface)

| # | Heuristic | Score | Key issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | Dirty has no persistent mark |
| 2 | Match System / Real World | 3 | Reading-width column vs workbench |
| 3 | User Control and Freedom | 3 | Overlay size fixed; body cannot take spare width |
| 4 | Consistency and Standards | 1 | Conflicts with archived two-column and full-width PromptEditor |
| 5 | Error Prevention | 3 | Save/dirty guards exist |
| 6 | Recognition Rather Than Recall | 3 | Labels visible; evaluation.* copy in prompts |
| 7 | Flexibility and Efficiency | 2 | Shortcuts exist; still scrolls past Identity in 68ch |
| 8 | Aesthetic and Minimalist Design | 1 | Large empty gutters; all sections expanded |
| 9 | Error Recovery | 3 | Inline validation; dirty dialog |
| 10 | Help and Documentation | 2 | Kbd hints; no cue that the body is primary |
| **Total** | | **23/40** | Acceptable |

Cognitive load: 7/8 checklist items fail.

## Priority issues

- P0: 68ch centered form inside 1180px overlay (`PromptDetailModal.tsx:562-563`)
- P0: One scroll stack; body cannot own remaining height
- P1: Container queries target the form, not the visual column
- P1: Duplicate title/description and duplicate copy controls
- P2: Rhythm `gap-7`+`pt-5`; always-on empty variable/media chrome
- P3: `ContentTab.tsx` from 08-24 design was never composed

## Recommended model

Two-column workbench (archived `08-24-detail-modal`): left body flex-1 remaining height, right metadata rail. Below ~40rem: identity strip, definition flex-1, org/media after. Do not only delete 68ch and keep the stack.
