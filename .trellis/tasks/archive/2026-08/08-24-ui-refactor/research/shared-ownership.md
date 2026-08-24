# Shared ownership — one owner per shared thing

Several children touch the same file, the same store field, or the same
primitive. Without a named owner, two children perform the same split and
conflict, or both assume the other did it and neither does. This table is the
parent's decision. A child may not renegotiate it at implementation time.

## Ownership table

| Shared thing                                                       | Owner                                   | Consumers                                                  | Ordering                                                                             |
| ------------------------------------------------------------------ | --------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `PromptList.tsx` loading / empty branch split (`:58-77`)           | `08-24-library-toolbar` (D7, step 8)    | `08-24-library-views`                                      | toolbar lands first                                                                  |
| `PromptGrid.tsx` and the shared item projection `libraryItem.ts`   | `08-24-library-views` (D1)              | —                                                          | after toolbar                                                                        |
| `viewMode` store field                                             | `08-24-library-toolbar` (D8)            | `library-views`, `command-palette`                         | toolbar lands first                                                                  |
| `activeView` / library scope state                                 | `08-24-shell-sidebar` (D3, D4)          | `library-toolbar`, `command-palette`                       | shell lands first                                                                    |
| `libraryCounts` slice                                              | `08-24-shell-sidebar` (D5)              | —                                                          | —                                                                                    |
| `src/components/ui/Modal.tsx` primitive and its stack              | `08-24-detail-modal` (D1, step 3)       | `08-24-command-palette` (D3)                               | detail-modal lands first; if it does not, palette builds it and records the transfer |
| The global shortcut layer and its binding table                    | `08-24-command-palette` (D2)            | `detail-modal` footer hints                                | palette lands last; detail-modal prints no hint it did not implement                 |
| `⌘S` / `⌘Enter` bindings                                           | `08-24-command-palette`                 | `detail-modal` supplies the actions through the registry   | see D2b                                                                              |
| The detail action registry (`save`, `copy`, `requestNavigation`)   | `08-24-detail-modal` (D10)              | `command-palette`                                          | detail-modal lands first                                                             |
| Guarded navigation on a dirty draft                                | `08-24-detail-modal` (D10)              | `command-palette` (D8), `library-views`, `library-toolbar` | every caller of `selectPrompt` routes through it                                     |
| Migrating `CopyPromptButton` to `prompt.copy`                      | `08-24-prompt-references` (D4, step 9b) | `library-views`, `detail-modal` reuse the migrated control | references lands before both                                                         |
| `transferMessage` removal from `PromptsView.tsx`                   | `08-24-command-palette` (step 1)        | —                                                          | after `library-toolbar` reworks that file                                            |
| `promptsView.editor.sections.references` → `...attachments` rename | `08-24-detail-modal` (R7b)              | —                                                          | all 7 locale bundles in one commit                                                   |

## Landing order

```
design-tokens → shell-sidebar → library-toolbar → library-views
                                       │
              prompt-references ───────┴──→ detail-modal → command-palette
```

`prompt-references` has no frontend dependency and may run in parallel with the
first four. It must land before `detail-modal`, because that task's references
tab consumes `reference.list`, and before either library child verifies copy
expansion end to end.

## Rule

If a child finds its owned shared thing already built by a child that landed
earlier out of order, it records the transfer in its own `implement.md` step 0
and does not build a second one. Building it twice is the defect this file
exists to prevent.
