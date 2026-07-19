# OpenCode isolation smoke tests

These prompts exercise the Loopwhole MCP tools through a real OpenCode agent while native read, write, edit, patch, and Bash tools are disabled.

The runner rebuilds the gateway and copies `fixture/` into the ignored `workspace/` before every scenario. Each run therefore starts with clean files and a fresh gateway session. It prepends `server/tests/context.md` and inlines the referenced `@docs/tools/` and manual documentation before every scenario so diagnosis does not create extra MCP read calls.

Detailed manual reproduction lives at `docs/tests/manual.md`; empirical findings and threats to validity live at `docs/controlled-experiments.md`.

## Requirements

- `cargo`
- `jq`
- `opencode` with a configured model/provider
- `python3` for the command-ID scenarios

## Run one scenario

```bash
server/tests/opencode/run-smoke.sh 01-read-unchanged
server/tests/opencode/run-smoke.sh 02-read-diff
server/tests/opencode/run-smoke.sh 03-write-edit
server/tests/opencode/run-smoke.sh 04-bash-unchanged
server/tests/opencode/run-smoke.sh 05-bash-diff
server/tests/opencode/run-smoke.sh 06-bash-id-reuse
server/tests/opencode/run-smoke.sh 07-bash-edit-id
```

## Run every scenario

```bash
server/tests/opencode/run-smoke.sh all
```

The script passes the selected Markdown file from `instructions/` to `opencode run`, injects an absolute local MCP configuration, and prints:

- one token-accounting row per tool call from `workspace/logs/<session>.log`, including actual/original input and input savings;
- cumulative totals from `workspace/.loopwhole/sessions/<session>.json`;
- automated metric and command-ID assertions for scenarios 06 and 07.

A useful successful row looks like:

```text
sequence  tool  mode       input  original_input  input_saved  original_output  intercepted_output  total_saved
2         bash  unchanged  10     58              48           900              1                   947
```

Token counts use the gateway's `ceil(characters / 4)` estimate, so `NoC` is one output token. Negative savings are possible and should be investigated rather than hidden.

## Instructions

- `01-read-unchanged.md` — first read versus an identical repeated read.
- `02-read-diff.md` — full read, exact edit, then compact changed read.
- `03-write-edit.md` — create-only write, overwrite rejection, exact edit, and unchanged read.
- `04-bash-unchanged.md` — execute a Cargo test command, then rerun it by ID.
- `05-bash-diff.md` — change a passing test to failing before rerunning by ID; returns a diff only when smaller than the canonical failure.
- `06-bash-id-reuse.md` — prove a real agent can reuse a Python command ID with positive input and output savings.
- `07-bash-edit-id.md` — prove a real agent can edit a stored Python command, receive a new ID, and reuse it with positive total savings.

## Direct log inspection

While a run is active or after it exits:

```bash
grep '^{' server/tests/opencode/workspace/logs/opencode-*.log | jq .
```

Each tool-call JSON line contains:

- delivery mode and reason;
- actual and counterfactual full-command input token estimates;
- input tokens saved;
- original-output and intercepted-output token estimates;
- saved tokens;
- context and output savings percentages;
- byte counts, duration, and baseline/current hashes.

The log intentionally excludes payload text. Full original and intercepted evidence remains available through the API and shutdown session dump.

## Notes

The command allowlist is a demo policy, not an operating-system sandbox. The generated `workspace/` is ignored and may be deleted at any time.
