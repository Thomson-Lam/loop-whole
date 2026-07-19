# Warp MCP Gateway index

Description: Navigation map for the Rust MCP gateway, its live observability API, frontend, and tests.
Purpose: Find the runtime boundary or integration point without treating this index as the source of truth.

## Backend

- `server/INDEX.md` — Rust MCP gateway, child Axum API, Cargo package, and backend test map.

## Frontend

- `web/INDEX.md` — Vite/React UI components and the live API integration seam.

## Documentation and tests

- `README.md` — setup, tool behavior, API overview, and persistence format.
- `docs/curl.md` — live API testing without a frontend.
- `docs/tools/` — delivery and token-reduction behavior by MCP tool.
- `server/tests/opencode/` — isolated OpenCode fixture, prompts, and smoke runner.
- `docs/tests/manual.md` — manual smoke-test and diagnosis workflow.

Rust unit tests live beside their implementations under `server/src/`.

## Runtime boundaries

- Stdout is reserved for MCP; diagnostics go to stderr and `logs/`.
- File tools and command working directories remain inside the configured workspace root.
- The HTTP API serves the live in-memory session; `.loopwhole/sessions/` is written on shutdown.
