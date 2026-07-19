# Manual token-reduction reproduction

This guide reproduces Loopwhole tool behavior through an isolated OpenCode agent without the frontend.

## Prerequisites

```bash
command -v cargo
command -v jq
command -v opencode
```

OpenCode must have a configured model/provider.

## Fast path

From the repository root, run one scenario:

```bash
server/tests/opencode/run-smoke.sh 01-read-unchanged
server/tests/opencode/run-smoke.sh 02-read-diff
server/tests/opencode/run-smoke.sh 03-write-edit
server/tests/opencode/run-smoke.sh 04-bash-unchanged
server/tests/opencode/run-smoke.sh 05-bash-diff
server/tests/opencode/run-smoke.sh 06-bash-id-reuse
server/tests/opencode/run-smoke.sh 07-bash-edit-id
```

Run all scenarios:

```bash
server/tests/opencode/run-smoke.sh all
```

The runner performs these steps for every scenario:

1. builds `server/target/debug/warp-mcp-gateway`;
2. resets `server/tests/opencode/workspace/` from the committed fixture;
3. disables OpenCode's native read/write/edit/patch/Bash tools;
4. starts one gateway session rooted at the fixture workspace;
5. gives OpenCode `server/tests/context.md` followed by the selected instruction;
6. prints per-call JSON log measurements and shutdown session totals.

## Expected scenario behavior

| Scenario | Expected delivery modes |
| --- | --- |
| `01-read-unchanged` | `full`, `unchanged` |
| `02-read-diff` | `full`, `passthrough`, `diff` |
| `03-write-edit` | `passthrough`, `error`, `passthrough`, `full`, `unchanged` |
| `04-bash-unchanged` | `compressed`, `unchanged` |
| `05-bash-diff` | `compressed`, `passthrough`, then `diff` or `compressed` when the diff is not smaller |
| `06-bash-id-reuse` | `compressed`, `unchanged` |
| `07-bash-edit-id` | `compressed`, `compressed`, `unchanged` |

Illustrative measurements from the fixture are:

```text
repeated read:       391 original → 1 intercepted token (`NoC`)
repeated cargo test:  61 original → 1 intercepted token (`NoC`)
```

`NoC` means no relevant changes from the previous matching call; Bash commands still execute. Exact original counts may change with compiler output or fixture changes. Delivery modes, the one-token `NoC` estimate, and positive unchanged-call savings are the stable assertions. Gateway totals cover serialized tool arguments and tool-result text only; OpenCode prompts, linked documentation, model reasoning, and protocol wrappers are excluded.

## Inspect the current scenario log

The generated workspace is reset before each scenario, so its log contains only the current isolated run:

```bash
LOG=server/tests/opencode/workspace/logs/opencode-04-bash-unchanged.log
grep '^{' "$LOG" | jq 'select(.event == "tool_call")'
```

Compact table:

```bash
grep '^{' "$LOG" | jq -r '
  select(.event == "tool_call") |
  [.sequence, .toolName, .deliveryMode,
   .originalOutputTokens, .interceptedOutputTokens,
   .savedTokens, .outputSavingsPercent] | @tsv
'
```

Each line excludes payload text but includes the fields needed to explain the decision.

## Inspect the shutdown dump

```bash
DUMP=server/tests/opencode/workspace/.loopwhole/sessions/opencode-04-bash-unchanged.json
jq '.totals' "$DUMP"
```

Per-call evidence:

```bash
jq '.toolCalls[] | {
  sequence,
  toolName,
  deliveryMode,
  decisionReason,
  originalTokens: .original.tokens,
  interceptedTokens: .intercepted.tokens,
  savedTokens: (.original.tokens - .intercepted.tokens)
}' "$DUMP"
```

## Inspect the live API

While an interactive OpenCode session keeps the MCP child running, use another terminal:

```bash
curl -s http://127.0.0.1:8787/health | jq
curl -s http://127.0.0.1:8787/api/v1/sessions/current | jq '.totals, .toolCalls'
```

Fetch one selected call's original and intercepted payloads:

```bash
curl -s http://127.0.0.1:8787/api/v1/tool-calls/1 | jq
```

## Manual OpenCode invocation

The runner is preferred because it creates absolute paths dynamically. Its equivalent prompt is:

```bash
PROMPT="$(
  cat server/tests/context.md
  for document in \
    docs/tools/read.md docs/tools/write.md docs/tools/edit.md \
    docs/tools/bash.md docs/tests/manual.md
  do
    printf '\n\n--- BEGIN %s ---\n\n' "$document"
    cat "$document"
    printf '\n\n--- END %s ---\n' "$document"
  done
  cat server/tests/opencode/instructions/01-read-unchanged.md
)"
OPENCODE_CONFIG_CONTENT="$(jq -c . server/tests/opencode/opencode.json)" \
  opencode run "$PROMPT"
```

The committed configuration contains paths for this local checkout. `run-smoke.sh` overrides the binary, workspace, and session paths and is therefore less error-prone.

## How to diagnose failures

### No JSON tool-call lines

- rebuild the gateway;
- confirm OpenCode launched `server/target/debug/warp-mcp-gateway`;
- confirm the MCP process uses the expected session ID;
- inspect non-JSON startup/error lines in the same log.

### Repeated read remains `full`

- verify path, offset, and limit are exactly identical;
- verify both calls occurred in one live session;
- inspect `decisionReason`;
- see `@docs/tools/read.md`.

### Repeated command remains `compressed`

- verify program, argument order, argument values, and working directory are identical;
- confirm the first process completed and established a baseline;
- see `@docs/tools/bash.md`.

### Missing shutdown dump

The MCP process may still be running or may have been terminated without graceful shutdown. The live API and log remain the first diagnostic sources.

### Savings are zero or negative

Inspect `originalOutputTokens`, `interceptedOutputTokens`, and `decisionReason`. Passthrough write/edit calls should normally save zero. A changed command diff may legitimately be larger than the current bounded output.

## Tool references

- `@docs/tools/read.md`
- `@docs/tools/write.md`
- `@docs/tools/edit.md`
- `@docs/tools/bash.md`
