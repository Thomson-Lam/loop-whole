# Warp MCP Gateway

A Rust MCP gateway exposing workspace-scoped `read` and `write` tools over stdio, retaining tool-call evidence in memory for the active session, serving a polling API for a future observability UI, and dumping one session JSON file to `.loopwhole/` on shutdown. Repository baselines, hashing, compaction, compression, and diffs are not implemented yet.

## Project rules

- Reserve stdout exclusively for MCP; write diagnostics to stderr.
- Keep filesystem access inside the configured workspace root, including symlink checks.
- Keep original and intercepted payloads separate even while they are identical.
- Prefer the smallest direct implementation needed for the hackathon demo.
- Whenever a file is added, removed, or changes purpose, update the index in this `AGENTS.md` in the same change.

## Index

- `src/main.rs` — CLI configuration and MCP/HTTP server lifecycle.
- `src/mcp.rs` — MCP tools, token accounting, and session recording.
- `src/tools.rs` — Read/write behavior, truncation, path enforcement, and write locking.
- `src/store.rs` — Concurrent in-memory session records, token-summary projections, and shutdown JSON serialization.
- `src/api.rs` — Health, session polling, and tool-call detail endpoints.
- `src/logging.rs` — Repo-local diagnostics mirrored to stderr and `logs/`.
- `src/schema.rs` — MCP inputs and frontend API response types.
- `README.md` — Setup, MCP configuration, API usage, session dump schema, and implementation status.
- `.loopwhole.example/session.schema.json` — Committed reference schema for persisted session dumps.
- `docs/planning/` — Feature, optimization, frontend, and silent-failure specifications.

Tests currently live beside the implementation in `src/mcp.rs` and `src/tools.rs`.

## Validation

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
