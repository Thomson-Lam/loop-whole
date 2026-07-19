# Testing the observability API with curl

The gateway exposes a read-only HTTP API while its MCP process is running. A frontend is not required to inspect the current session or full tool-call evidence.

## Prerequisites

Start the gateway through OpenCode or directly, and keep that process running. The examples assume the default API address and require `curl`; `jq` is optional but used for readable output.

```bash
API=http://127.0.0.1:8787
```

If the gateway was started with `--api-addr`, use that address instead.

## Check health

```bash
curl -fsS "$API/health" | jq
```

Expected response:

```json
{
  "status": "ok"
}
```

A connection failure usually means the MCP process is not running or is listening on a different address. An `address already in use` error in the gateway log means another process already owns the configured port.

## Inspect the current session

```bash
curl -fsS "$API/api/v1/sessions/current" | jq
```

This returns session metadata, cumulative token totals, and lightweight summaries of all recorded tool calls. The data is read from the live in-memory session store.

Show only cumulative totals:

```bash
curl -fsS "$API/api/v1/sessions/current" | jq '.totals'
```

Show a compact tool-call timeline:

```bash
curl -fsS "$API/api/v1/sessions/current" | jq -r '
  .toolCalls[] |
  [.id, .sequence, .toolName, .status, .deliveryMode, .savedTokens] |
  @tsv
'
```

Get the latest tool-call ID:

```bash
ID=$(curl -fsS "$API/api/v1/sessions/current" | jq -r '.toolCalls[-1].id')
printf 'latest tool-call ID: %s\n' "$ID"
```

If no calls have been recorded, the expression returns `null`.

## Inspect one tool call

Use an ID from the current-session response:

```bash
ID=1
curl -fsS "$API/api/v1/tool-calls/$ID" | jq
```

The detail response includes:

- the exact tool input;
- delivery mode, reason, and comparison hashes;
- the bounded original result;
- the exact intercepted result delivered to the model;
- byte and estimated-token counts.

Inspect only the optimization decision:

```bash
curl -fsS "$API/api/v1/tool-calls/$ID" | jq '.decision'
```

Compare original and intercepted payloads:

```bash
curl -fsS "$API/api/v1/tool-calls/$ID" |
  jq '{original: .original, intercepted: .intercepted}'
```

Requesting an unknown ID returns HTTP `404`:

```bash
curl -i "$API/api/v1/tool-calls/999999"
```

## Poll while the agent runs

The current-session endpoint is intended for polling. For a simple terminal view:

```bash
while true; do
  clear
  curl -fsS "$API/api/v1/sessions/current" | jq '{totals, toolCalls}'
  sleep 2
done
```

Press Ctrl-C to stop polling.

## Live API versus files

The API is available only while the gateway process is alive. Per-call metric lines are also appended and flushed during the session at:

```text
<workspace>/logs/<session-id>.log
```

The complete persisted session is written on a normal MCP disconnect or handled shutdown signal at:

```text
<workspace>/.loopwhole/sessions/<session-id>.json
```

The live detail endpoint and shutdown dump include original and intercepted payload text. The live log contains benchmark metadata but intentionally excludes payload text.
