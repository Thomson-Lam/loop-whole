# Loop-Whole demo manual

This is the copy-and-paste setup guide for the Loop-Whole demo. It assumes this local layout:

```text
/Users/tlam/
├── loop-whole/           # Gateway backend and frontend
└── loop-whole-sandbox/   # OpenCode demo workspace
```

The frontend has two relevant routes:

```text
http://localhost:5173/#/app        # live gateway session
http://localhost:5173/#/benchmarks # bundled benchmark results
```

`#/app` requires the gateway API running under OpenCode. `#/benchmarks` reads bundled JSON and does not require the gateway API.

## 1. Set the demo paths and session ID

Run this in every terminal used by the demo:

```bash
export LOOP_WHOLE="/Users/tlam/loop-whole"
export SANDBOX="/Users/tlam/loop-whole-sandbox"
export SESSION="pitch-demo"
```

Change `SESSION` when you want a separate gateway session.

> **Required setup:** Both repositories must exist as sibling directories at the paths above. This can be set up with `git clone https://github.com/Thomson-Lam/loop-whole.git "$LOOP_WHOLE"` and `git clone https://github.com/Thomson-Lam/loop-whole-demo-sandbox.git "$SANDBOX"`.

Check the directories:

```bash
test -d "$LOOP_WHOLE/server"
test -d "$LOOP_WHOLE/web"
test -d "$SANDBOX/scripts"
```

## 2. Check required programs

```bash
command -v cargo
command -v npm
command -v node
command -v opencode
command -v curl
command -v lsof
```

> **Required setup:** OpenCode must have a working model/provider configuration before the demo. Configure it in OpenCode's normal configuration location and confirm it by running `opencode` once outside the demo workflow.

## 3. Prepare benchmark data and the frontend

The repository includes benchmark data at:

```text
/Users/tlam/loop-whole/web/src/data/benchmark-results.json
```

Check it and install the frontend dependencies:

```bash
cd "$LOOP_WHOLE"
test -s web/src/data/benchmark-results.json
npm --prefix web ci
npm --prefix web run lint
npm --prefix web run build
```

> **Required setup:** `web/src/data/benchmark-results.json` must contain either the committed mock benchmark or generated evaluator results. If it is missing, create the deterministic mock data by running `python3 scripts/build_mock_benchmark.py` from `$LOOP_WHOLE`. Real benchmark data can be generated with `benchmark/build_benchmark_results.py` as documented in [`../../benchmark/README.md`](../../benchmark/README.md).

Do not use `/benchmark`. The frontend route is hash-based and plural:

```text
http://localhost:5173/#/benchmarks
```

## 4. Build the gateway

The sandbox configuration uses the debug binary:

```bash
cargo build --manifest-path "$LOOP_WHOLE/server/Cargo.toml"
test -x "$LOOP_WHOLE/server/target/debug/warp-mcp-gateway"
```

> **Required setup:** The Rust gateway must be rebuilt after backend changes. This can be done by running `cargo build --manifest-path "$LOOP_WHOLE/server/Cargo.toml"`.

A frontend-only change does not require rebuilding the gateway.

## 5. Prepare a fresh named gateway session

Use this section when creating a new demo checkpoint. Do not use it when resuming an existing checkpoint.

```bash
cd "$SANDBOX"
scripts/reset-demo.sh
rm -f ".loopwhole/sessions/$SESSION.json"
rm -f "logs/$SESSION.log"
scripts/configure.sh --session-id "$SESSION"
```

Check the generated OpenCode configuration:

```bash
test -s opencode.json
grep -F -- "--session-id" opencode.json
grep -F -- "$SESSION" opencode.json
grep -F -- "127.0.0.1:8787" opencode.json
grep -F -- "$SANDBOX" opencode.json
```

> **Required setup:** `opencode.json` must launch `$LOOP_WHOLE/server/target/debug/warp-mcp-gateway` with `--root "$SANDBOX"`, `--api-addr 127.0.0.1:8787`, and `--session-id "$SESSION"`. This can be configured by running `scripts/configure.sh --session-id "$SESSION"` from `$SANDBOX`.

The gateway session ID is separate from OpenCode's `ses_...` conversation ID.

## 6. Start OpenCode and the gateway

First check that an old gateway is not occupying the API port:

```bash
lsof -nP -iTCP:8787 -sTCP:LISTEN || true
```

In terminal 1:

```bash
export LOOP_WHOLE="/Users/tlam/loop-whole"
export SANDBOX="/Users/tlam/loop-whole-sandbox"
export SESSION="pitch-demo"
cd "$SANDBOX"
opencode
```

OpenCode reads `$SANDBOX/opencode.json` and launches the gateway as its MCP child. Do not manually start another gateway on port `8787`.

Expected process relationship:

```text
OpenCode
  └── warp-mcp-gateway
        ├── MCP over stdio
        └── dashboard API on 127.0.0.1:8787
```

> **Required setup:** OpenCode must show the local `Loopwhole` MCP server as connected. This is set up by running `scripts/configure.sh --session-id "$SESSION"` before starting OpenCode.

## 7. Start the frontend

In terminal 2:

```bash
export LOOP_WHOLE="/Users/tlam/loop-whole"
cd "$LOOP_WHOLE"
npm --prefix web run dev
```

Open:

```text
http://localhost:5173/#/app
http://localhost:5173/#/benchmarks
```

The Vite frontend proxies `/api` and `/health` to `127.0.0.1:8787`.

> **Required setup:** Port `5173` must be available and frontend dependencies must be installed under `$LOOP_WHOLE/web/node_modules`. This can be set up by running `npm --prefix "$LOOP_WHOLE/web" ci` and then `npm --prefix "$LOOP_WHOLE/web" run dev`.

## 8. Verify the live stack

In terminal 3:

```bash
export SESSION="pitch-demo"
lsof -nP -iTCP:8787 -sTCP:LISTEN
lsof -nP -iTCP:5173 -sTCP:LISTEN
pgrep -fl warp-mcp-gateway
curl -fsS http://127.0.0.1:8787/health
curl -fsS http://127.0.0.1:8787/api/v1/sessions/current -o /tmp/loop-whole-session.json
grep -F -- "$SESSION" /tmp/loop-whole-session.json
```

The health response should be:

```json
{"status":"ok"}
```

The current-session response should contain the selected gateway session ID and should gain tool calls as OpenCode uses the MCP tools.

## 9. Run the demo workflow

Follow the prompts in `$SANDBOX/DEMO.md` exactly. Repeated operations must use identical paths, offsets, limits, arguments, and working directories because those values form the comparison-baseline keys.

The intended progression is:

```text
Repository listing       compressed → unchanged
Architecture read        full → unchanged
Reservation source read  full → unchanged
Baseline tests           compressed → unchanged
Duplicate write          passthrough success → passthrough error
Tests with regression    diff to failing
Reservation edit         passthrough
Source reread             diff → unchanged
Tests after repair        diff to passing → unchanged
Repository relisting     diff → unchanged
```

For the resumable demo, stop after Prompt 12. Record the OpenCode `ses_...` conversation ID before exiting.

## 10. Create the persistence checkpoint

Exit OpenCode normally. Do not kill the gateway process directly. The gateway writes its session dump only during normal shutdown.

Check the persisted files after OpenCode exits:

```bash
export SANDBOX="/Users/tlam/loop-whole-sandbox"
export SESSION="pitch-demo"
cd "$SANDBOX"
test -s ".loopwhole/sessions/$SESSION.json"
test -s "logs/$SESSION.log"
ls -lh ".loopwhole/sessions/$SESSION.json"
ls -lh "logs/$SESSION.log"
```

> **Required setup:** A resumable dump must exist at `$SANDBOX/.loopwhole/sessions/$SESSION.json`, and the sandbox filesystem must remain at the checkpoint state. This is created by starting with `scripts/configure.sh --session-id "$SESSION"`, making the pre-run calls, and exiting OpenCode normally.

Do not run `scripts/reset-demo.sh` between checkpoint creation and resumption.

## 11. Configure `opencode.json` to resume the session

```bash
cd "$SANDBOX"
scripts/configure.sh --resume-session "$SESSION"
```

Check the generated configuration:

```bash
test -s opencode.json
grep -F -- "--resume-session" opencode.json
grep -F -- "$SESSION" opencode.json
grep -F -- "127.0.0.1:8787" opencode.json
grep -F -- "$SANDBOX" opencode.json
```

The command in `opencode.json` must include:

```text
--root /Users/tlam/loop-whole-sandbox
--api-addr 127.0.0.1:8787
--context-window-tokens 200000
--resume-session pitch-demo
```

It must not contain `--session-id` for the resumed launch:

```bash
if grep -F -- "--session-id" opencode.json; then
  echo "ERROR: opencode.json is configured for a fresh session"
  exit 1
fi
```

> **Required setup:** The selected session dump must already exist and must match the same workspace root and context-window setting. This is configured by running `scripts/configure.sh --resume-session "$SESSION"` from `$SANDBOX` after the prior OpenCode process exits normally.

Changing `opencode.json` does not change a running gateway. Exit OpenCode first, regenerate the configuration, and then launch OpenCode again.

## 12. Resume OpenCode and verify persistence

List the OpenCode conversations:

```bash
cd "$SANDBOX"
opencode session list
```

Resume the matching OpenCode conversation in terminal 1:

```bash
cd "$SANDBOX"
opencode -s <ses_...>
```

OpenCode launches a new gateway child with `--resume-session "$SESSION"`. Verify it in terminal 3:

```bash
export SESSION="pitch-demo"
pgrep -fl "warp-mcp-gateway.*--resume-session $SESSION"
curl -fsS http://127.0.0.1:8787/health
curl -fsS http://127.0.0.1:8787/api/v1/sessions/current -o /tmp/loop-whole-resumed-session.json
grep -F -- "$SESSION" /tmp/loop-whole-resumed-session.json
```

Reload `http://localhost:5173/#/app`. The dashboard should immediately show the calls restored from the checkpoint and append new calls from the resumed process.

Continue with Prompts 13–18 from `$SANDBOX/DEMO.md`. Exit OpenCode normally again to update the same session dump.

## 13. Reset for another demo

To restore the sandbox files while preserving logs, dumps, generated OpenCode configuration, and build output:

```bash
cd "$SANDBOX"
scripts/reset-demo.sh
```

Before starting another fresh demo, choose a new session ID or remove the old evidence and regenerate the configuration:

```bash
export SESSION="pitch-demo-2"
rm -f ".loopwhole/sessions/$SESSION.json"
rm -f "logs/$SESSION.log"
scripts/configure.sh --session-id "$SESSION"
```

Never start a fresh demo while `opencode.json` still contains `--resume-session` for an old checkpoint.

## Troubleshooting

### Dashboard says the API is unavailable

```bash
lsof -nP -iTCP:8787 -sTCP:LISTEN
curl -fsS http://127.0.0.1:8787/health
pgrep -fl warp-mcp-gateway
```

OpenCode must be running with the `Loopwhole` MCP server connected.

### Session dump is missing

Exit OpenCode normally, then check:

```bash
ls -la "$SANDBOX/.loopwhole/sessions"
ls -la "$SANDBOX/logs"
```

A force kill or crash may prevent the final session dump.

### Resume fails

```bash
test -s "$SANDBOX/.loopwhole/sessions/$SESSION.json"
grep -F -- "--resume-session" "$SANDBOX/opencode.json"
grep -F -- "$SESSION" "$SANDBOX/opencode.json"
```

The persisted session ID, canonical workspace root, and context-window value must match the resumed command.

### Repeated calls do not become unchanged

Use exactly the same tool arguments and working directory. Across a process restart, confirm that `opencode.json` uses `--resume-session` and that the correct checkpoint was not reset.