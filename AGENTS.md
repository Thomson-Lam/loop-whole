# Warp MCP Gateway

A Rust MCP gateway exposing context-aware file tools and an allowlisted developer-command runner over stdio, retaining tool-call evidence and runtime comparison baselines in memory, serving a polling API for a future observability UI, and dumping one session JSON file to `.loopwhole/` on shutdown. Repeated read views and commands support unchanged or progressive-diff delivery, with bounded command output and an initial Cargo test projection.

## Project rules

- Reserve stdout exclusively for MCP; write diagnostics to stderr.
- Keep filesystem access inside the configured workspace root, including symlink checks.
- Keep original and intercepted payloads separate even while they are identical.
- Prefer the smallest direct implementation needed for the hackathon demo.
- Whenever a file is added, removed, or changes purpose, update the index in this `AGENTS.md` in the same change.
- Remind the user to run cargo build after file changes: output "test (run cargo build)?"

## Index

- `src/main.rs` — CLI configuration and MCP/HTTP server lifecycle.
- `src/mcp.rs` — MCP tools, context-delivery decisions, token accounting, and session recording.
- `src/tools.rs` — Read/create/edit behavior, truncation, path enforcement, and write locking.
- `src/commands.rs` — Allowlisted command execution, bounded output capture, normalization, and Cargo test projection.
- `src/store.rs` — Concurrent in-memory evidence and runtime baselines, token-summary projections, and shutdown JSON serialization.
- `src/api.rs` — Health, session polling, and tool-call detail endpoints.
- `src/logging.rs` — Repo-local diagnostics and per-call benchmark lines mirrored to stderr and `logs/`.
- `src/schema.rs` — MCP inputs and frontend API response types.
- `README.md` — Setup, MCP configuration, API usage, session dump schema, and implementation status.
- `.loopwhole.example/session.schema.json` — Committed reference schema for persisted session dumps.
- `tests/context.md` — Entry point for local MCP smoke testing.
- `tests/opencode/` — Isolated OpenCode configuration, fixture, instruction prompts, and smoke runner.
- `docs/tools/` — Per-tool delivery, token-reduction, and diagnosis documentation.
- `docs/tests/manual.md` — Manual OpenCode reproduction, measurement, and troubleshooting guide.
- `docs/planning/` — Feature, optimization, frontend, and silent-failure specifications.

Tests currently live beside the implementation in `src/commands.rs`, `src/mcp.rs`, `src/store.rs`, and `src/tools.rs`.

## Validation

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
