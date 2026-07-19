# `bash` token reduction

## Purpose

`bash` is an allowlisted direct command runner. It does not invoke a shell, so shell pipes, redirects, substitution, and chaining remain unsupported.

Every accepted command executes. Stored commands and previous outputs reduce context; they never skip validation or execution.

## Inputs and command IDs

Run a full command once:

```json
{
  "program": "cargo",
  "args": ["test", "--workspace"],
  "cwd": "."
}
```

A completed full call returns a content-addressed command ID:

```text
[Command ID: cmd-...]
```

Rerun the exact stored command with the smaller input:

```json
{ "command_id": "cmd-..." }
```

Command IDs cover the program, exact arguments, canonical working directory, and optional stdin. They resolve only commands already stored in the current or resumed gateway session.

### Reusable Python stdin

One-time Python heredoc-style work is represented without a shell:

```json
{
  "program": "python3",
  "args": ["-"],
  "cwd": ".",
  "stdin": "from pathlib import Path\nprint(len(list(Path('.').rglob('*.rs'))))\n"
}
```

`stdin` is accepted only for `python3` with `args: ["-"]`. Python retains the process user's permissions; the allowlist is demo policy, not an operating-system sandbox.

## `bash_edit`

`bash_edit` reuses a stored command, replaces one exact unique occurrence across its arguments and stdin, executes the edited command, and returns the edited command's new ID:

```json
{
  "command_id": "cmd-...",
  "old_text": "*.rs",
  "new_text": "*.toml"
}
```

It does not edit the executable or working directory. The edited command is validated against the same allowlist before execution.

## Execution and bounded evidence

A command baseline is keyed by:

```text
program + exact argument list + canonical working directory + stdin
```

Stdout and stderr are drained while their complete byte streams are hashed. The retained original keeps at most a bounded head and tail for each stream, includes omission markers, and always includes completion status. Commands time out after 120 seconds.

`original` is the bounded output from the current execution. `intercepted` is exactly what the model receives after normalization, adapter projection, comparison, and command-ID metadata.

## Canonicalization and DTO adapters

The current pipeline is:

1. execute the real command;
2. capture stdout and stderr separately, hash the complete streams, and retain bounded head/tail evidence;
3. build the generic DTO by removing ANSI control sequences, normalizing carriage returns, preserving stderr, and attaching exit/timeout status;
4. for untruncated `cargo test`, attempt a conservative Cargo-test projection;
5. compare the current result with the immediately previous baseline for that exact command;
6. return an unchanged marker, a smaller progressive diff, or the current canonical output.

The Cargo adapter returns pass/fail, test summaries, exit status, and failure details. Warnings, uncertain parsing, and relevant errors fall back to generic output. No broad npm, pytest, TypeScript, or JSON adapters exist yet.

## Delivery decisions

### First exact command

```text
deliveryMode: compressed
decisionReason: no_command_baseline
```

The current canonical result is returned, plus a command ID for full command inputs.

### Exact repeated output

The command executes again. Equal complete raw-output hash and exit code produce:

```text
deliveryMode: unchanged
decisionReason: command_output_unchanged
```

### Same relevant canonical result

If raw presentation differs but the untruncated canonical result and exit code match:

```text
deliveryMode: unchanged
decisionReason: canonical_command_output_unchanged
```

### Changed result

The gateway computes a bounded unified diff from the immediately previous canonical result. It sends the diff only when the diff token estimate is smaller than the current canonical output; otherwise it sends the current output.

```text
deliveryMode: diff
decisionReason: command_output_changed
```

or:

```text
deliveryMode: compressed
decisionReason: command_diff_not_smaller_than_current_output
```

Comparison is progressive, not permanently anchored to the first run.

## Token calculation

Counts use `ceil(characters / 4)`.

For normal calls, original and delivered input sizes are equal. For command-ID and `bash_edit` calls, the original-side input estimate is the equivalent full command DTO and the delivered-side input estimate is the actual smaller tool request.

```text
saved input  = originalInputTokens - inputTokens
saved output = originalOutputTokens - interceptedOutputTokens
saved total  = saved input + saved output
```

The API exposes both input counts. Session totals include command-input reuse as well as output reduction.

## Assessment of `docs/handoff/bash-tool-strategy-discussion.md`

Implemented and useful:

- complete-stream hashing with bounded retained output;
- stable generic normalization before comparison;
- a conservative command-specific DTO adapter for `cargo test`;
- exit status and stderr preservation;
- progressive unchanged/diff delivery;
- fallback to current output when a diff is not smaller.

Partially implemented:

- failure-first projection exists for Cargo tests, but not for other command families;
- deterministic-output handling strips presentation noise, but does not inject flags or environment settings beyond disabling ripgrep config.

Not implemented, intentionally for the MVP:

- broad adapters for npm, pytest, TypeScript, JSON, and diagnostics;
- semantic sorting or path/timing normalization;
- silent-failure detection;
- an OS sandbox.

This is better than a simple cached-output check because commands still execute, complete raw streams are compared even when retained output is truncated, noisy-but-equivalent Cargo results can collapse semantically, and changed output can be reduced progressively. A cache that skips execution can return stale validation; an execute-then-raw-diff implementation misses normalization and can send a diff larger than the current result.

The next adapter should be added only after recorded sessions show a noisy command family with meaningful token volume. Do not build a generic adapter framework first.

## Diagnosis

Expected command-ID sequence:

```text
full cargo test call: compressed + Command ID
bash by command_id: unchanged + smaller input
```

Expected edit sequence:

```text
full python3 stdin call: compressed + Command ID
bash_edit one script literal: executes + new Command ID
bash by new command_id: unchanged
```

If an ID cannot be resolved, verify that the same gateway session stayed alive or was resumed from its session dump.
