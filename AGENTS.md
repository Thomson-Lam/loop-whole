# Warp MCP Gateway

A Rust MCP gateway exposing context-aware file tools and an allowlisted developer-command runner over stdio. It keeps tool-call evidence and comparison baselines in memory, serves a read-only polling API, and writes a session JSON file under the configured workspace on shutdown. A Vite/React frontend displays the live session.

## Project rules

- Reserve stdout exclusively for MCP; write diagnostics to stderr and workspace-local logs.
- Keep filesystem access inside the configured workspace root, including symlink checks.
- Keep original and intercepted payloads separate even when they are identical.
- Treat the command allowlist as demo policy, not an operating-system sandbox.
- Prefer the smallest direct implementation needed for the hackathon demo.
- Keep this navigation map current whenever files are added, removed, or repurposed.
- Remind the user to run cargo build after file changes: output "test (run cargo build)?"

## Repository navigation

### Root and documentation

- `README.md` — build and MCP-client setup, tool contracts, API endpoints, persistence, logs, and development checks.
- `server/Cargo.toml` and `web/package.json` — backend and frontend manifests; `web/vite.config.js` proxies `/api` and `/health` to the gateway during development.
- `.env.example` — local secret template; `.loopwhole.example/` contains the persisted-session schema and an example session.
- `docs/tools/` — behavior and diagnosis guides for `read`, `write`, `edit`, and `bash`; `docs/curl.md` contains live API recipes.
- `docs/tests/manual.md` — manual OpenCode token-reduction reproduction; `docs/demo/manual.md` — the external sandbox demo workflow and runtime topology.
- `docs/planning/` — active, deferred, and archived product/optimization specifications.
- `scripts/build_demo_session.py` — generates the committed example session fixture.
- `benchmark/` — SWE-bench submodule boundary plus prediction-generation and benchmark testing notes.

### Backend

- `server/src/main.rs` — CLI parsing, workspace canonicalization, logging setup, MCP/HTTP lifecycle, and shutdown persistence.
- `server/src/mcp.rs` — MCP handlers, delivery-mode decisions, diffing, token estimation, and evidence recording.
- `server/src/tools.rs` — bounded UTF-8 reads, create-only writes, exact edits, path enforcement, and process-local mutation locks.
- `server/src/commands.rs` — command allowlist, workspace-scoped working directories, bounded process capture, normalization, and Cargo test projection.
- `server/src/store.rs` — concurrent in-memory calls and baselines, API snapshots/details, totals, and persisted session shape.
- `server/src/api.rs` — health, current-session summary, and tool-call detail routes.
- `server/src/schema.rs` — MCP request schemas and API response models.
- `server/src/logging.rs` — diagnostics mirrored to stderr and `logs/<session-id>.log`.

Unit tests are colocated in `server/src/commands.rs`, `server/src/mcp.rs`, `server/src/store.rs`, and `server/src/tools.rs`. `server/tests/context.md` supplies smoke-test instructions; `server/tests/opencode/` contains the isolated fixture, MCP configuration, scenario prompts, and `run-smoke.sh` runner.

### Frontend

- `web/src/main.jsx` and `web/src/App.jsx` — React bootstrap and hash-based landing/dashboard routing.
- `web/src/Landing.jsx` and `web/src/ToolReplay.jsx` — marketing page and live current-session replay.
- `web/src/Dashboard.jsx` — per-call original/intercepted comparison and cumulative context metrics.
- `web/src/api.js` and `web/src/useLiveSession.js` — current-session/detail hydration and polling.
- `web/src/Antigravity.jsx` — Three.js hero effect; `web/src/index.css` contains shared landing, replay, and dashboard styles.
- `web/index.html`, `web/eslint.config.js`, and `web/vite.config.js` — browser entry, linting, build, and development-server configuration.

## Runtime boundaries

- File paths and command working directories must resolve within `--root`; file operations also reject symlink escapes.
- `bash` executes allowlisted programs directly without shell expansion. Output is bounded, but allowed programs and build scripts retain the process user's permissions.
- Read baselines are keyed by canonical path, offset, and limit. Command baselines are keyed by program, exact arguments, and canonical working directory; commands execute again before comparison.
- Session evidence and baselines are process-local and in memory. The HTTP API exposes the current process only; shutdown writes `.loopwhole/sessions/<session-id>.json` under the workspace root.
- Token counts approximate `ceil(characters / 4)` and cover serialized tool arguments and tool results, not full model context.

## Validation

```bash
cargo fmt --manifest-path server/Cargo.toml -- --check
cargo clippy --manifest-path server/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path server/Cargo.toml
npm --prefix web run lint
npm --prefix web run build
```
