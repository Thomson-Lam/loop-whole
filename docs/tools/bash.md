# `bash` token reduction

## Purpose

The MCP tool named `bash` is an allowlisted direct command runner. It does not invoke a shell, and therefore does not support pipes, redirects, command substitution, or chaining.

Every accepted command executes. Previous output is only a comparison baseline; it is never used to skip execution.

A command baseline is identified by:

```text
program + exact argument list + canonical working directory
```

## Bounded original output

Stdout and stderr are drained while their complete byte streams are hashed. The retained response keeps a bounded head and tail for each stream, includes omission markers, and always includes completion status. Commands time out after 120 seconds.

`original` is this bounded current execution result. It is not an unbounded copy of operating-system output.

## Canonicalization and adapters

Generic canonicalization removes ANSI control sequences and normalizes carriage returns.

A conservative Cargo-test adapter recognizes untruncated `cargo test` output and projects stable test summaries and failure details. If parsing is uncertain, it falls back to generic canonical output.

## Delivery decisions

### First exact command

```text
deliveryMode: compressed
decisionReason: no_command_baseline
```

The intercepted result is the current canonical representation.

### Exact repeated output

The command executes again. If its complete raw-output hash and exit code match:

```text
deliveryMode: unchanged
decisionReason: command_output_unchanged
```

### Same relevant canonical result

If raw presentation changed but the untruncated canonical result and exit code did not:

```text
deliveryMode: unchanged
decisionReason: canonical_command_output_unchanged
```

### Changed result

The gateway returns a bounded unified diff from the immediately previous canonical result to the current canonical result:

```text
deliveryMode: diff
decisionReason: command_output_changed
```

The current result then becomes the next baseline. Comparison is progressive; it is not permanently anchored to the first run.

If the adapter changes or the previous representation is incompatible, the current canonical result is returned with `command_adapter_changed`.

## Token calculation

```text
savedTokens = originalOutputTokens - interceptedOutputTokens
```

A compact first projection may save tokens. An unchanged marker should save more. A changed diff can produce smaller, zero, or negative savings depending on how much output changed; negative savings remain visible for diagnosis.

## Diagnosis

Expected unchanged smoke sequence:

```text
cargo test #1: compressed
cargo test #2: unchanged
```

Expected changed smoke sequence:

```text
cargo test passing → edit test → same cargo command failing → diff
```

If no baseline matches, verify that program, argument order, arguments, and working directory are exactly identical and that the same gateway session stayed alive.

The allowlist is not an operating-system sandbox. See `@docs/tests/manual.md`, `@server/tests/opencode/instructions/04-bash-unchanged.md`, and `@server/tests/opencode/instructions/05-bash-diff.md`.
