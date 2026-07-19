# `write` token accounting

## Purpose

The `write` tool creates a new UTF-8 file. It deliberately refuses to overwrite an existing path.

```text
file absent  → create
file present → error; use edit
```

Creation uses create-new semantics so existence checking and creation are one operation.

## Original and intercepted payloads

`write` does not currently perform result compression or temporal compaction:

```text
original == intercepted
```

Successful output is already a short confirmation containing the path and byte count. Errors are also passed through unchanged.

Expected successful decision:

```text
deliveryMode: passthrough
decisionReason: create_only_write
```

The resulting file hash is recorded as `currentHash`.

## Token calculation

The complete file content is part of the tool input, so it contributes to `inputTokens`. Only output differences count as saved output tokens:

```text
savedTokens = originalOutputTokens - interceptedOutputTokens
```

For `write`, this should normally be zero. That is intentional: its value is safe mutation and evidence, not output reduction.

## Diagnosis

Expected smoke sequence:

```text
first write to new path: passthrough success
second write to path:   error; existing file remains untouched
```

Unexpected positive or negative output savings would indicate that original and intercepted error/confirmation payloads diverged.

An unrestricted command can still bypass create-only policy; the command allowlist is a demo policy rather than an operating-system sandbox.

See `@docs/tests/manual.md` and `@tests/opencode/instructions/03-write-edit.md`.
