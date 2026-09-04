---
target: PromptDetailModal content tab
total_score: 23
max_score: 40
na_heuristics: 
p0_count: 2
p1_count: 2
target_identity: "file:D:\\Documents\\Code\\Rust\\Exp\\PromptHub\\src\\features\\prompts\\components\\detail\\PromptDetailModal.tsx"
target_fingerprint: "sha256:d95dc73108cd66ef90edb5d04be13696dcb1f83b5639b20711b71aaf11877ec9"
target_path: "D:\\Documents\\Code\\Rust\\Exp\\PromptHub\\src\\features\\prompts\\components\\detail\\PromptDetailModal.tsx"
timestamp: 2026-09-04T12-51-39Z
slug: es-prompts-components-detail-promptdetailmodal-tsx
---
# Prompt detail overlay content tab — critique

Visitor mode: Operate. Target: `src/features/prompts/components/detail/PromptDetailModal.tsx`.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | Dirty has no persistent mark; only the close/nav dialog |
| 2 | Match System / Real World | 3 | Reading-width column does not match a prompt workbench |
| 3 | User Control and Freedom | 3 | Escape/scrim/dirty guard exist; body cannot take spare 1180px |
| 4 | Consistency and Standards | 1 | Conflicts with archived two-column design and full-width PromptEditor |
| 5 | Error Prevention | 3 | Save disabled when invalid; dirty guard on close and nav |
| 6 | Recognition Rather Than Recall | 3 | Field labels visible; evaluation.* copy leaks into prompts |
| 7 | Flexibility and Efficiency | 2 | ⌘S / copy hints exist; still scrolls past Identity in 68ch |
| 8 | Aesthetic and Minimalist Design | 1 | Large empty gutters; all sections expanded |
| 9 | Error Recovery | 3 | Inline title/body errors; dirty dialog copy is explicit |
| 10 | Help and Documentation | 2 | Kbd hints; no cue that the body is the primary region |
| **Total** | | **23/40** | **Acceptable** |

## Design Specificity Verdict

**LLM assessment:** Mostly category-interchangeable. The shell (1180×90vh overlay, four tabs, dirty close, lock redaction, chat role/body/actions) is authored for PromptHub. The content tab is a CMS-style labeled form in a centered 68ch article measure. That does not encode “prompt body is the work surface.”

**Deterministic scan:** `detect.mjs --scope layout` exited 0 with `[]`. The regex engine did not fire `line-length` because `max-w-[68ch]` is the prose measure the rule treats as correct. The defect is applying that measure to an Operate workbench container, which the detector cannot see.

**Visual overlays:** not injected. Tauri overlay, no localhost surface.

## Overall Impression

The overlay chrome is the right object. The content tab wastes the chrome. A 1180px dialog hosts a ~34rem centered stack, so the user’s screenshot is mostly empty card.

## What's Working

1. Shell: centered overlay, scrim, focus trap, header/tabs/footer, 1180×90vh.
2. Chat row grammar at ≥40rem: role | body | actions, compact min-height for multiple messages.
3. Close/lock weighting: three-action dirty dialog; locked content withheld.

## Priority Issues

### [P0] 68ch centered form inside an 1180px overlay
- **What:** `PromptDetailModal.tsx:562-563` wraps Identity, Definition, Organization, and Media in `mx-auto max-w-[68ch]`.
- **Why it matters:** ~18–20rem empty gutter each side. The prompt body is the high-frequency work and sits in a reading column.
- **Fix:** Remove the reading-column clamp from the form. Let the body pane use remaining width.
- **Suggested command:** `$impeccable layout`

### [P0] One vertical stack; body cannot own remaining height
- **What:** Order is Identity → Definition → Organization → Media on one `overflow-y-auto` (`PromptDetailModal.tsx:564-610`).
- **Why it matters:** Catalog fields take first position. The body shares the scroll with attachments. Archived `08-24-detail-modal` specified left body / right metadata.
- **Fix:** Two-column workbench: body left `flex-1` remaining height; metadata rail right.
- **Suggested command:** `$impeccable layout`

### [P1] Container queries target the form, not the visual column
- **What:** `.prompt-editor` is full modal width (`PromptDetailModal.tsx:556`, `globals.css:338-341`). The 68ch wrapper is a child, so `@container (min-width: 40rem)` fires while content stays 68ch.
- **Why it matters:** Title/description split into two skinny fields while unused gutters grow.
- **Fix:** Query the pane that actually holds the fields, or drop the inner clamp so form width equals visual width.
- **Suggested command:** `$impeccable layout`

### [P1] Duplicate leading content and duplicate copy
- **What:** Header already shows title and description (`PromptDetailModal.tsx:400-411`). Identity repeats both. Copy exists in the header and on the definition heading.
- **Why it matters:** Extra scan cost before the body.
- **Fix:** Keep copy once in chrome. Keep identity fields in the metadata rail only.
- **Suggested command:** `$impeccable distill`

### [P2] Rhythm and always-on secondary chrome
- **What:** `gap-7` plus section `pt-5` vs DESIGN 4/8/12/16/24. Empty VariableEditor hint and empty media lists always occupy the same stack as the body.
- **Why it matters:** Secondary chrome inflates the scroll the body must share.
- **Fix:** After the two-pane split, collapse empty media; put variables under the body pane only.
- **Suggested command:** `$impeccable quieter`

## Persona Red Flags

**Alex (Power User):** Must tab/scroll through title, description, type, and private before the prompt body. Types in 68ch inside an 1180px window. Two copy buttons. No persistent dirty mark.

**Riley (Stress Tester):** Single long message is `min-height: 24rem` inside 68ch, so a one-line prompt still paints a 384px well. Many messages share one scroll with Identity and Media. More-actions menu is `absolute` inside `overflow-hidden` dialog.

**Sam (Keyboard / AT):** Focus trap and tablist arrows exist. Edit vs read is icon-only `aria-pressed`. Disabled version/run/references tabs on create/lock use `title` only.

## Minor Observations

- Confirm-dialog width/max-height classes concatenate onto Modal defaults with no `twMerge`.
- Inner flex column uses the same `min(90vh,56rem)` as the dialog `max-h`, so the 1px border can clip.
- `.prompt-editor__footer` compact rules never apply: the live footer is outside the form.
- `.prompt-editor__message-role` is unused.
- `ContentTab.tsx` from 08-24 design D2 does not exist.

## Questions to Consider

- What if the prompt body were the only full-height pane, and metadata were a rail?
- Does 68ch belong on preview prose only, not on the workbench form?
- What would a confident workbench do with empty attachments?
