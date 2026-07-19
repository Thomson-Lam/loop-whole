# Warp MCP Gateway

A Rust MCP server that exposes context-aware `read`, create-only `write`, exact `edit`, and allowlisted command tools, keeps tool-call evidence and runtime baselines in memory, and writes a session JSON dump on shutdown for observability.

Repeated reads and commands return unchanged markers or progressive diffs. Command output is bounded and known `cargo test` output is projected into a compact result.

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

Disable the agent's equivalent native tools so reads, file mutations, and demo commands use the MCP gateway.

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
- An identical repeated request returns an unchanged marker or a diff from its previous result.
- Paths are restricted to the configured workspace, including symlink checks.
- Detailed reduction behavior: `docs/tools/read.md`.

### `write`

```json
{
  "path": "src/example.rs",
  "content": "fn main() {}\n"
}
```

- Creates missing parent directories.
- Atomically refuses to overwrite an existing path; use `edit` instead.
- Serializes concurrent writes to the same path within the server process.
- Paths are restricted to the configured workspace.
- Detailed accounting behavior: `docs/tools/write.md`.

### `edit`

```json
{
  "path": "src/example.rs",
  "old_text": "fn old() {}",
  "new_text": "fn new() {}"
}
```

- Replaces one exact, unique occurrence in an existing UTF-8 file.
- Rejects missing or ambiguous matches.
- Serializes concurrent edits to the same path within the server process.
- Detailed accounting behavior: `docs/tools/edit.md`.

### `bash`

```json
{
  "program": "cargo",
  "args": ["test", "--workspace"],
  "cwd": "."
}
```

- Executes programs directly without shell expansion, pipes, redirects, or chaining.
- Allows selected `cargo`, `npm`, read-only `git`, `grep`, and `rg` commands.
- Rejects executable paths, unsupported command families, absolute path arguments, and parent traversal.
- Times out after 120 seconds and retains at most 256KB from the head and tail of each output stream while hashing the complete drained output.
- Repeated exact commands are always executed, then compared with the previous result for unchanged or progressive-diff delivery.
- The allowlist is a demo policy, not an operating-system sandbox; allowed programs and build scripts retain the process user's permissions.
- Detailed reduction behavior: `docs/tools/bash.md`.

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

Original payloads contain the bounded result of the current tool execution. Intercepted payloads contain the exact full, compressed, unchanged, or diff result delivered to the model.

## Current implementation status

Implemented:

- MCP stdio lifecycle and tool discovery;
- context-aware `read`, create-only `write`, exact `edit`, and allowlisted `bash` tools;
- workspace path enforcement for file tools and command working directories;
- bounded read and command output;
- in-memory read-view and repeated-command baselines;
- unchanged and progressive-diff delivery;
- generic command normalization and a conservative `cargo test` projection;
- session-scoped original/intercepted payload storage;
- automatic session JSON dump on shutdown;
- token-estimate totals;
- polling and detail API endpoints.

Not implemented yet:

- overlapping read-range reasoning;
- broader command DTO adapters;
- operating-system command sandboxing;
- filesystem change watching or prompt injection;
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
      "decisionReason": "no_read_baseline",
      "baselineHash": null,
      "currentHash": "8fb09291be4f2042",
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

Every recorded tool call also emits one JSON line containing its delivery mode, decision reason, duration, hashes, byte counts, input/original/intercepted token estimates, saved tokens, and context/output savings percentages. Payload text is excluded from logs and remains available through the API and session dump.

```bash
grep '^{' logs/<session-id>.log | jq 'select(.event == "tool_call")'
```

The `logs/` directory is gitignored. This is the easiest way to inspect server activity when an MCP host hides child-process stderr.

## OpenCode smoke tests

An isolated fixture and instruction-driven runner live in `tests/opencode/`:

```bash
tests/opencode/run-smoke.sh 01-read-unchanged
tests/opencode/run-smoke.sh 04-bash-unchanged
tests/opencode/run-smoke.sh all
```

Each scenario resets an ignored fixture workspace, runs OpenCode with native filesystem and Bash tools disabled, and prints per-call log metrics plus shutdown session totals. See `tests/opencode/README.md`.

## Development checks

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
