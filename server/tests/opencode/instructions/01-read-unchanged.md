# Smoke test: repeated unchanged read

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call the Loopwhole `read` tool for `src/lib.rs` with `offset: 1` and `limit: 200`.
2. Call the same Loopwhole `read` tool again with exactly the same path, offset, and limit.
3. Stop. Report whether the second result was exactly `NoC`.

Expected gateway decisions: `full`, then `unchanged`. The second call should report positive token savings in the gateway log.
