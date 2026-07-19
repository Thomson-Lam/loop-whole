# Server index

Description: Navigation map for the Rust MCP gateway and its child observability HTTP server.
Purpose: Locate backend runtime, safety, persistence, and test boundaries.

## Runtime and modules

- `Cargo.toml` — Rust package definition; run Cargo commands from this directory or pass `--manifest-path server/Cargo.toml`.
- `src/main.rs` — CLI entry point; starts MCP over stdio and Axum, then persists the session on shutdown.
- `src/mcp.rs` — MCP handlers, command-ID reuse/edit orchestration, delivery decisions, diffs, token accounting, and evidence recording.
- `src/tools.rs` — workspace-scoped read, create-only write, exact edit, truncation, and path safety.
- `src/commands.rs` — command allowlist, optional Python stdin, execution limits, exact stored-command edits, normalization, and Cargo test projection.
- `src/store.rs` — in-memory calls, command lookup, baselines, API snapshots, and shutdown serialization.
- `src/api.rs` — read-only health, current-session, and tool-call detail routes.
- `src/schema.rs` — MCP inputs and camelCase HTTP response contracts; authoritative frontend data boundary.
- `src/logging.rs` — diagnostics to stderr and per-session metric logs.

## Tests

- `tests/context.md` — shared controlled-diagnostic instructions.
- `tests/opencode/` — isolated OpenCode fixture, scenario prompts, configuration, and smoke runner; scenarios 06–07 assert command-ID reuse, stored-command editing, and positive token savings.
- Unit tests live beside implementations in `src/commands.rs`, `src/mcp.rs`, `src/store.rs`, and `src/tools.rs`.

`target/` and `tests/opencode/workspace/` are generated and must not be indexed or committed.

## Related indexes

- `../INDEX.md` — repository navigation.
- `../web/INDEX.md` — frontend and live API integration boundary.
