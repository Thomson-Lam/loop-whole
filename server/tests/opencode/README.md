# OpenCode isolation smoke tests

These prompts exercise the Loopwhole MCP tools through a real OpenCode agent while native read, write, edit, patch, and Bash tools are disabled.

The runner rebuilds the gateway and copies `fixture/` into the ignored `workspace/` before every scenario. Each run therefore starts with clean files and a fresh gateway session. It prepends `server/tests/context.md` and inlines the referenced `@docs/tools/` and manual documentation before every scenario so diagnosis does not create extra MCP read calls.

Detailed manual reproduction and diagnosis instructions live at `docs/tests/manual.md`.

## Requirements

- `cargo`
- `jq`
- `opencode` with a configured model/provider

## Run one scenario

```bash
server/tests/opencode/run-smoke.sh 01-read-unchanged
server/tests/opencode/run-smoke.sh 02-read-diff
server/tests/opencode/run-smoke.sh 03-write-edit
server/tests/opencode/run-smoke.sh 04-bash-unchanged
server/tests/opencode/run-smoke.sh 05-bash-diff
```

## Run every scenario

```bash
server/tests/opencode/run-smoke.sh all
```

The script passes the selected Markdown file from `instructions/` to `opencode run`, injects an absolute local MCP configuration, and prints:

- one token-accounting row per tool call from `workspace/logs/<session>.log`;
- cumulative totals from `workspace/.loopwhole/sessions/<session>.json`.

A useful successful row looks like:

```text
sequence  tool  mode       original  intercepted  saved  output_savings_pct
2         read  unchanged  400       24           376    94.0
```

Token counts use the gateway's `ceil(characters / 4)` estimate. Negative savings are possible and should be investigated rather than hidden.

## Instructions

- `01-read-unchanged.md` — first read versus an identical repeated read.
- `02-read-diff.md` — full read, exact edit, then compact changed read.
- `03-write-edit.md` — create-only write, overwrite rejection, exact edit, and unchanged read.
- `04-bash-unchanged.md` — execute the same Cargo test command twice.
- `05-bash-diff.md` — change a passing test to failing between identical Cargo commands.

## Direct log inspection

While a run is active or after it exits:

```bash
grep '^{' server/tests/opencode/workspace/logs/opencode-*.log | jq .
```

Each tool-call JSON line contains:

- delivery mode and reason;
- input, original-output, and intercepted-output token estimates;
- saved tokens;
- context and output savings percentages;
- byte counts, duration, and baseline/current hashes.

The log intentionally excludes payload text. Full original and intercepted evidence remains available through the API and shutdown session dump.

## Notes

The command allowlist is a demo policy, not an operating-system sandbox. The generated `workspace/` is ignored and may be deleted at any time.
