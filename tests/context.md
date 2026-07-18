# MCP server test context

Use this when opening an OpenCode agent to test the Rust MCP server in this repository.

## Goal

Validate that the MCP server can be launched by an MCP client over stdio, exposes `read` and `write`, serves the frontend polling API, and writes a session dump on shutdown.

## Binary

Build first:

```bash
cargo build --release
```

Binary:

```text
target/release/warp-mcp-gateway
```

## Suggested MCP server config

Use absolute paths.

```json
{
  "mcpServers": {
    "loopwhole": {
      "command": "/absolute/path/to/warp/target/release/warp-mcp-gateway",
      "args": [
        "--root",
        "/absolute/path/to/warp",
        "--api-addr",
        "127.0.0.1:8787",
        "--session-id",
        "opencode-test"
      ]
    }
  }
}
```

Disable native read/write tools in the agent if possible so calls go through MCP.

## What to test

1. MCP client can initialize the server.
2. `tools/list` shows `read` and `write`.
3. `write` can create a test file.
4. `read` can read it back.
5. Frontend API responds while MCP session is active:
   - `GET /health`
   - `GET /api/v1/sessions/current`
   - `GET /api/v1/tool-calls/{id}`
6. On session shutdown, a dump is written to:

```text
.loopwhole/sessions/opencode-test.json
```

## Important stdout/stderr note

Yes: you should assume you **cannot** use stdout for ordinary server logs.

- stdout is reserved for MCP JSON-RPC over stdio;
- writing logs to stdout would corrupt the MCP transport;
- this server writes diagnostics to stderr instead.

So if the MCP server is launched as a child process by the MCP client:

- MCP protocol travels on stdin/stdout;
- normal logs should be visible only if the client surfaces child-process stderr;
- if the client hides stderr, use the HTTP API and session dump as your primary observability.

## Practical implication

If you want to inspect runtime behavior while testing through a real MCP client, prefer:

- polling `http://127.0.0.1:8787/api/v1/sessions/current`;
- polling `http://127.0.0.1:8787/api/v1/tool-calls/{id}`;
- inspecting `.loopwhole/sessions/opencode-test.json` after shutdown.

## Manual smoke test fallback

If MCP client stderr is hidden, you can still test the server manually with a small stdio client script or the existing local smoke flow, then inspect the API and dump file.
