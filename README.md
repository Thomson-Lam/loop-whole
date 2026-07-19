# Loop-Whole

Loop-Whole is an experimental, stateful MCP layer for coding agents. It remembers previously delivered file views and command results, then returns only relevant changes—or the one-token marker `NoC`—while preserving the current execution as evidence. Reusable command IDs and exact stored-command edits also reduce repeated Bash input.

> [!WARNING]
> Loop-Whole is a hackathon prototype, fresh out of the oven, and is **not ready for production use**. We have horrible telemetry, attached a flashy UI for hackathon purposes, and wrote a lot of JSONs for debugging. The commands that work with our tools via the allowed list of commands is just Rust based commands (yes, we were testing it on its own source code as we were building it, one of the beauties of dev tool development) and basic commands (no Python, JS/TS at all); and the polling API plus frontend child server are wired into the process for the demo purposes. If this were to be used as a dev tool, you would not want the UI and would want much better telemetry than this MVP. More coming soon!

We also did controlled OpenCode experiments to demonstrate the mechanisms behind our tooling design and its limits, including some negative and zero-savings cases. But we truly believe that this has potential to bring big gains, so more tweaks and evals will be done on this beyond HT6. See [`docs/controlled-experiments.md`](docs/controlled-experiments.md).

## Requirements

- Rust and (ideally) Cargo 
- A MCP client or coding-agent MCP extension that supports stdio servers

## Quick start - run one script and launch coding agent (Claude Code, Codex, OpenCode) 

For this dev repo, clone and build the Rust crate using absolute paths. Relative paths are easy to misresolve because the coding harness launches Loop-Whole with the UI and the HTTP server as a child process from the target workspace.

```bash
git clone https://github.com/Thomson-Lam/loop-whole.git
cd loop-whole
LOOPWHOLE_ROOT="$(pwd -P)"
cargo build --manifest-path "$LOOPWHOLE_ROOT/server/Cargo.toml" --release
```

From the workspace your agent will edit, run the setup script with the absolute gateway path:

```bash
cd /absolute/path/to/your/workspace
"$LOOPWHOLE_ROOT/scripts/setup-workspace.sh" \
  "$LOOPWHOLE_ROOT/server/target/release/warp-mcp-gateway"
```

The helper setup script currently supports only:

- **Claude Code** through `.mcp.json`;
- **Codex** through `.codex/config.toml`;
- **OpenCode** through `opencode.json`.

It preserves unrelated configuration, adds one `AGENTS.md` line explaining `NoC`, and refuses malformed or conflicting managed sections. The MCP tool descriptions also define `NoC` because not every harness reads `AGENTS.md`. Other MCP clients can still use Loop-Whole through manual stdio configuration below, but they are not covered by the setup script.

Start the supported coding harness from the configured workspace and ask it to use the Loop-Whole MCP tools. Re-run the release build after changing Rust code. Make sure to disable native read, write and Bash tools (including Grep and Glob for OpenCode).

The browser UI is optional and is only for the hackathon demo. If Node.js is available:

```bash
npm --prefix "$LOOPWHOLE_ROOT/web" install
npm --prefix "$LOOPWHOLE_ROOT/web" run dev
```

The MCP client normally starts Loop-Whole itself. Do not separately start another gateway process on the same API address.

## Configure an MCP client manually

Configure your MCP client to launch the binary over stdio. Use absolute paths when the client's working directory is uncertain.

```json
{
  "mcpServers": {
    "Loopwhole": {
      "command": "/absolute/path/to/loop-whole/server/target/release/warp-mcp-gateway",
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

The server reserves stdout for MCP protocol messages. Diagnostics are written to stderr and mirrored to `logs/<session-id>.log` under the configured workspace root.

## Options

```text
--root <PATH>                    Workspace boundary; defaults to current directory
--api-addr <HOST:PORT>           Polling API address; defaults to 127.0.0.1:8787
--session-id <ID>                Start a fresh session with this identifier
--resume-session <ID>            Resume .loopwhole/sessions/<ID>.json
--context-window-tokens <COUNT>  Optional model context-window size for UI percentages
```

Example:

```bash
server/target/release/warp-mcp-gateway \
  --root /path/to/project \
  --session-id demo-session \
  --context-window-tokens 200000
```

FYI: This is the command the MCP client launches under the hood. While the client-managed MCP process is running, the same process exposes the HTTP API at the configured address. When the MCP process ends, it writes a session dump to `.loopwhole/sessions/<session-id>.json`; we have no persistent DB because we only handle tool calls and not full agent traces, so there was no point in using SQLite.

Session IDs may contain ASCII letters, numbers, `.`, `_`, and `-`. To continue that logical gateway session in a later MCP-client process, replace `--session-id demo-session` with `--resume-session demo-session`, which restores prior calls, token totals, read baselines, command baselines, and ID/sequence counters before serving MCP or HTTP. The configured workspace and context-window value must match the dump. Resume the coding agent's own conversation separately; those are two independent session IDs.

Expected startup order:

1. Build with `cargo build --manifest-path server/Cargo.toml --release`.
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
- An identical repeated request returns `NoC` or a diff from its previous result.
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
- Allows selected `cargo`, `npm`, read-only `git`, `grep`, and `rg` commands, plus `python3` scripts supplied as stdin with `args: ["-"]`.
- A completed full call returns a command ID; rerun it with `{ "command_id": "cmd-..." }` to avoid resending the command.
- Rejects executable paths, unsupported command families, absolute path arguments, and parent traversal.
- Times out after 120 seconds and retains at most 256KB from the head and tail of each output stream while hashing the complete drained output.
- Repeated exact commands are always executed, then compared with the previous result; unchanged relevant output returns exactly `NoC`.
- The allowlist is a demo policy, not an operating-system sandbox; allowed programs, Python scripts, and build scripts retain the process user's permissions.
- Detailed reduction behavior: `docs/tools/bash.md`.

### `bash_edit`

```json
{
  "command_id": "cmd-...",
  "old_text": "*.rs",
  "new_text": "*.toml"
}
```

- Replaces one exact unique occurrence across a stored command's arguments and stdin.
- Executes the edited command under the same allowlist and returns its new reusable command ID.

## Dashboard API

The read-only polling API and Vite/React frontend are bundled into the gateway lifecycle for the hackathon demonstration. This is not the intended production boundary: the UI observer, telemetry, and MCP execution path should be separated before real deployment. The API reads the process's in-memory session store without a database; resumed sessions seed that store from `.loopwhole/sessions/<session-id>.json`. During development, Vite proxies `/api` and `/health` to `127.0.0.1:8787`.

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

The hackathon frontend hydrates every summary through this endpoint on each poll so both the dashboard and landing-page replay stay live. Cache details by ID if demo sessions become large.

## Token accounting

Counts currently use the explicit approximation because we are unable to get exact values based on the model tokenizers:

```text
estimated tokens = ceil(character count / 4)
```

The session comparison covers tool arguments and tool results only. It excludes system prompts, repository instruction files, user messages, assistant prose, and reasoning since we were purely obsessed with minimzing both the inputs and outputs required of tool calls without degrading agent performance.

For command-ID and `bash_edit` calls, the without-runtime input estimate is the equivalent full command DTO while the with-runtime input estimate is the actual compact request. Original payloads contain the bounded result of the current tool execution without the DTO, and intercepted payloads mean that they contain the more compact, compressed, unchanged, or diff result delivered to the model.

## Session dump schema

Each session writes one JSON file on shutdown:

```text
.loopwhole/sessions/<session-id>.json
```

Shape:

```json
{
  "formatVersion": 1,
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
    "originalToolInputTokens": 1200,
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
      "originalInputTokens": 7,
      "original": { "text": "...", "bytes": 1234, "tokens": 309 },
      "intercepted": { "text": "...", "bytes": 1234, "tokens": 309 }
    }
  ],
  "baselines": {
    "reads": [],
    "commands": []
  }
}
```

A committed example schema lives at `.loopwhole.example/session.schema.json`.

## Logs

Development and demo runs write diagnostics to:

```text
logs/<session-id>.log
```

Every recorded tool call also emits one JSON line containing its delivery mode, decision reason, duration, hashes, byte counts, input/original/intercepted token estimates, saved tokens, and context/output savings percentages. Payload text is excluded from logs and remains available through the API and session dump.

```bash
grep '^{' logs/<session-id>.log | jq 'select(.event == "tool_call")'
```

The `logs/` directory is gitignored. These logs are a development diagnostic surface, not production telemetry: they have no remote collection, retention policy, access controls, alerting, or durable query layer. They are useful when an MCP host hides child-process stderr.

## Controlled tests

The isolated OpenCode suite verifies tool mechanics without claiming that its purpose-built repetition rate represents normal agentic development:

```bash
server/tests/opencode/run-smoke.sh 01-read-unchanged
server/tests/opencode/run-smoke.sh 04-bash-unchanged
server/tests/opencode/run-smoke.sh 06-bash-id-reuse
server/tests/opencode/run-smoke.sh 07-bash-edit-id
server/tests/opencode/run-smoke.sh all
```

Each scenario resets an ignored fixture workspace, runs OpenCode with native filesystem and Bash tools disabled, and prints per-call plus shutdown totals. Command-ID scenarios fail automatically unless the agent uses returned IDs correctly and produces positive measured savings. Results, negative cases, methodology, and threats to validity are documented in [`docs/controlled-experiments.md`](docs/controlled-experiments.md). Runner details live in `server/tests/opencode/README.md`.

## Development

```bash
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo clippy --manifest-path server/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path server/Cargo.toml
npm --prefix web run lint
npm --prefix web run build
```
