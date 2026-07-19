# `edit` token accounting

## Purpose

The `edit` tool replaces one exact, unique occurrence in an existing UTF-8 file:

```text
path + old_text + new_text
```

It rejects empty, missing, ambiguous, and overlapping matches. The updated content is staged and atomically replaces the previous file where supported by the operating system.

## Original and intercepted payloads

`edit` does not currently compress its short result:

```text
original == intercepted
```

Expected successful decision:

```text
deliveryMode: passthrough
decisionReason: exact_text_replaced
```

The previous and resulting file hashes are recorded as `baselineHash` and `currentHash`.

## Relationship to read compaction

Editing does not erase an earlier read baseline. Therefore this sequence is useful for testing:

```text
read V1 → edit V1 to V2 → read same view
```

The final read compares its current result with the stored V1 read result and can return a compact V1-to-V2 diff.

## Token calculation

Both `old_text` and `new_text` are tool input and contribute to `inputTokens`. The concise edit confirmation normally produces zero saved output tokens:

```text
savedTokens = originalOutputTokens - interceptedOutputTokens = 0
```

Token reduction appears on a later repeated read, not on the edit confirmation itself.

## Diagnosis

Expected cases:

```text
one exact match:       passthrough success
no exact match:        error
multiple/overlap match:error
read after edit:       diff when smaller than current view
```

Never use fuzzy replacement to force a stale edit through; an error preserves the current file for diagnosis.

See `@docs/tests/manual.md`, `@server/tests/opencode/instructions/02-read-diff.md`, and `@server/tests/opencode/instructions/03-write-edit.md`.
