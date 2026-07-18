# Warp MCP Gateway

A Rust MCP server that exposes workspace-scoped `read` and `write` tools, keeps tool-call payloads in memory during the active session, and writes a session JSON dump on shutdown for observability.

This is currently the gateway backbone. It does **not** yet compact, compress, deduplicate, or diff tool results.

## Requirements

- Rust and Cargo
- An MCP client or coding-agent MCP extension that supports stdio servers

## Quick start

First, build the executable:

```bash
cargo build --release
```

The binary is created at:

```text
target/release/warp-mcp-gateway
```

Next, configure your MCP client to launch that binary. You normally do **not** start the gateway separately: the client starts it as a child process, sends MCP requests through its stdin, reads MCP responses from its stdout, and stops it when the client session ends.

Re-run `cargo build --release` after changing the Rust code.

## Configure an MCP client

Configure your MCP client to launch the binary over stdio. Use absolute paths when the client's working directory is uncertain.

```json
{
  "mcpServers": {
    "warp": {
      "command": "/absolute/path/to/warp/target/release/warp-mcp-gateway",
      "args": [
        "--root",
        "/absolute/path/to/workspace",
        "--api-addr",
        "127.0.0.1:8787"
      ]
    }
  }
}
```

Disable the agent's native read and write tools so repository operations use the MCP tools.

The server reserves stdout for MCP protocol messages. Diagnostics are written to stderr and mirrored to `logs/<session-id>.log` in the repository root.

## Options

```text
--root <PATH>                    Workspace boundary; defaults to current directory
--api-addr <HOST:PORT>           Polling API address; defaults to 127.0.0.1:8787
--session-id <ID>                Optional external session identifier
--context-window-tokens <COUNT>  Optional model context-window size for UI percentages
```

Example:

```bash
target/release/warp-mcp-gateway \
  --root /path/to/project \
  --session-id demo-session \
  --context-window-tokens 200000
```

This is the command the MCP client launches; it is shown for clarity rather than as a separate startup step. While the client-managed MCP process is running, the same process exposes the HTTP API at the configured address. When the MCP process ends, it saves a session dump to `.loopwhole/sessions/<session-id>.json`.

Expected startup order:

1. Build with `cargo build --release`.
2. Configure the MCP client with the resulting binary path and arguments.
3. Start the MCP client; it launches the gateway automatically.
4. Open the frontend while the client session is running.

## MCP tools

### `read`

```json
{
  "path": "src/main.rs",
  "offset": 1,
  "limit": 200
}
```

- Reads UTF-8 text files.
- `offset` is optional and one-indexed.
- `limit` is optional.
- Results are limited to 2,000 lines or 50KB.
- Paths are restricted to the configured workspace, including symlink checks.

### `write`

```json
{
  "path": "src/example.rs",
  "content": "fn main() {}\n"
}
```

- Creates missing parent directories.
- Creates or overwrites the complete file.
- Serializes concurrent writes to the same path within the server process.
- Paths are restricted to the configured workspace.

## Dashboard API

The API is read-only and intended for a Vite/TanStack Query frontend. It reads from the process's in-memory session store, so the frontend can query it concurrently without a database. When the MCP process exits, the in-memory state is dumped to `.loopwhole/sessions/<session-id>.json` and then discarded.

### Health

```http
GET /health
```

### Current session and tool-call summaries

```http
GET /api/v1/sessions/current
```

Returns:

- session metadata;
- cumulative original and intercepted token estimates;
- context-window percentages when configured;
- ordered lightweight tool-call summaries for timeline navigation.

This is the endpoint to poll.

### Tool-call detail

```http
GET /api/v1/tool-calls/{id}
```

Returns the selected call's:

- input;
- original tool result;
- intercepted tool result;
- byte and estimated-token counts;
- delivery decision metadata.

Fetch this endpoint when the selected timeline item changes rather than polling every large payload.

## Token accounting

Counts currently use the explicit approximation:

```text
estimated tokens = ceil(character count / 4)
```

The session comparison covers tool arguments and tool results only. It excludes system prompts, repository instruction files, user messages, assistant prose, and reasoning.

At this stage, original and intercepted outputs are identical, so expected savings are zero.

## Current implementation status

Implemented:

- MCP stdio lifecycle and tool discovery;
- Rust `read` and `write` tools;
- workspace path enforcement;
- in-memory session and tool-call store;
- session-scoped original/intercepted payload storage;
- automatic session JSON dump on shutdown;
- token-estimate totals;
- polling and detail API endpoints.

Not implemented yet:

- observed file baselines;
- content hashes;
- unchanged-read suppression;
- changed-file diffs;
- compression or compaction strategies;
- filesystem change watching;
- silent-failure signals;
- frontend UI.

## Session dump schema

Each session writes one JSON file on shutdown:

```text
.loopwhole/sessions/<session-id>.json
```

Shape:

```json
{
  "session": {
    "id": "demo-session",
    "startedAtMs": 1774267200000,
    "endedAtMs": 1774267265000,
    "workspaceRoot": "/abs/path/to/repo",
    "contextWindowTokens": 200000,
    "tokenCounter": "chars_div_4_v1"
  },
  "totals": {
    "toolInputTokens": 1200,
    "originalOutputTokens": 12420,
    "interceptedOutputTokens": 4610,
    "withoutRuntimeTokens": 13620,
    "withRuntimeTokens": 5810,
    "savedTokens": 7810,
    "savingsPercent": 57.34,
    "withoutRuntimeContextPercent": 6.81,
    "withRuntimeContextPercent": 2.91
  },
  "toolCalls": [
    {
      "id": 1,
      "sequence": 1,
      "occurredAtMs": 1774267201000,
      "toolName": "read",
      "subjectPath": "src/main.rs",
      "status": "success",
      "durationMs": 4,
      "deliveryMode": "full",
      "decisionReason": "state_optimization_not_enabled",
      "baselineHash": null,
      "currentHash": null,
      "input": { "path": "src/main.rs" },
      "original": { "text": "...", "bytes": 1234, "tokens": 309 },
      "intercepted": { "text": "...", "bytes": 1234, "tokens": 309 }
    }
  ]
}
```

A committed example schema lives at `.loopwhole.example/session.schema.json`.

## Logs

Each run writes diagnostics to:

```text
logs/<session-id>.log
```

The `logs/` directory is gitignored. This is the easiest way to inspect server activity when an MCP host hides child-process stderr.

## Development checks

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
