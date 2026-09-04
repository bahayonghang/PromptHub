# Prompt references

A prompt body may reference another prompt with `@@` tokens. Two forms:

- `@@Title of the prompt@@` — explicit; the title may contain spaces
- `@@Title` to end of line — shorthand

The explicit form is scanned first on each `@@`. An empty title or `@@` at the
end of input is literal text and creates no edge.

Edges are stored by prompt id at save time. Renaming a target does not break a
resolved edge. Copy expands through `prompt.copy` before substituting
`{{placeholders}}`. Depth is 3. Cycles, missing, ambiguous, and locked targets
leave `@@Title` in the text. A successful clipboard write then calls
`prompt.incrementUsage`. Do not increment inside `prompt.copy`.

A prompt saved before this change has no edges until it is next saved. No
backfill runs: a private prompt's body is ciphertext, so a backfill would cover
public prompts only.
