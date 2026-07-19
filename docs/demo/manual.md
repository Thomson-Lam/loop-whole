# Loop-Whole sandbox manual

This [example depository](https://github.com/Thomson-Lam/loop-whole-demo-sandbox) used in the demo video is a deterministic development workload for demonstrating Loop-Whole's context-aware MCP tools. The gateway runs from the sibling `loop-whole` repository but treats this sandbox as its workspace root.

## What the demo shows

One logical OpenCode/gateway session, optionally split across two processes with durable resume, demonstrates:

- a full file read followed by an unchanged marker;
- a changed file delivered as a compact progressive diff;
- repeated deterministic commands delivered as unchanged;
- Cargo test output changing from passing to failing to passing;
- create-only write safety when the same file is written twice;
- live token totals and original/intercepted payloads in the frontend.

The duplicate write is a safety demonstration, not a token-saving case. `write` confirmations and errors are intentionally passed through unchanged.

## Repository locations

The expected local layout is:

```text
/Users/tlam/
├── loop-whole/           # Gateway backend and observability frontend
└── loop-whole-sandbox/   # This simulated development repository
```

The generated OpenCode configuration launches:

```text
/Users/tlam/loop-whole/server/target/debug/warp-mcp-gateway
```

with:

```text
--root /Users/tlam/loop-whole-sandbox
--api-addr 127.0.0.1:8787
--context-window-tokens 200000
```

Pass a different gateway executable to `scripts/configure.sh` if the repositories are not siblings.

## Prerequisites

Confirm these commands are available:

```bash
command -v cargo
command -v npm
command -v opencode
command -v python3
```

OpenCode also needs a configured model/provider.

## One-time gateway build

Build the MCP gateway whenever it does not exist or its Rust source has changed:

```bash
cargo build --manifest-path /Users/tlam/loop-whole/server/Cargo.toml
```

A frontend-only change does not require rebuilding the gateway.

## Prepare a clean demo baseline

From the sandbox repository:

```bash
cd /Users/tlam/loop-whole-sandbox
scripts/reset-demo.sh
rm -f .loopwhole/sessions/pitch-demo.json logs/pitch-demo.log
scripts/configure.sh --session-id pitch-demo
```

`reset-demo.sh` restores tracked files to the current sandbox commit and removes untracked, non-ignored demo files. It intentionally preserves:

- `opencode.json`;
- `target/`;
- `logs/`;
- `.loopwhole/`.

The generated `opencode.json` contains absolute paths for this checkout and is gitignored.

To configure a gateway at another location:

```bash
scripts/configure.sh /absolute/path/to/warp-mcp-gateway --session-id pitch-demo
```

Use a stable gateway session ID when preparing a recording. Calling `scripts/configure.sh` without a session option retains the old behavior of generating a random ID.

## Start the demo

Use two terminals.

### Terminal 1: observability frontend

```bash
cd /Users/tlam/loop-whole-sandbox
npm --prefix ../loop-whole/web run dev
```

Open the Vite URL and navigate to the dashboard. The frontend proxies `/api` and `/health` to `127.0.0.1:8787`. It may initially report that the API is unavailable because OpenCode has not launched the gateway yet.

### Terminal 2: OpenCode and gateway

```bash
cd /Users/tlam/loop-whole-sandbox
opencode
```

OpenCode reads `opencode.json` and launches the gateway as its local `Loopwhole` MCP server. Do not manually start another gateway on port `8787`.

The expected startup relationship is:

```text
OpenCode
  └── warp-mcp-gateway
        ├── MCP over stdio
        └── dashboard API on 127.0.0.1:8787

Vite frontend
  └── proxies API requests to 127.0.0.1:8787
```

Verify the live API from another shell if needed:

```bash
curl -fsS http://127.0.0.1:8787/health
curl -fsS http://127.0.0.1:8787/api/v1/sessions/current | python3 -m json.tool
```

## Run or pre-run the workflow

Follow the 18 prompts in [`DEMO.md`](DEMO.md), one at a time and in order. You can keep one OpenCode process alive, or use the durable checkpoint workflow below. In either case, preserve exact request arguments and do not reset the sandbox between workflow segments. The sequence is:

1. list repository files twice with the exact same `rg --files` call;
2. read `docs/architecture.md` twice with the same offset and limit;
3. read `reservation.rs` twice with the same offset and limit;
4. run the exact workspace test command twice;
5. create a zero-quantity regression test and retry the identical write;
6. run tests to observe the new failure;
7. apply the documented one-line quantity guard edit;
8. reread the same source view for a diff, then reread it unchanged;
9. run tests for failure-to-pass output, then run them unchanged;
10. list repository files again for a new-file diff, then unchanged.

Do not alter tool arguments, path spelling, offsets, limits, or working directories between paired calls. Baseline keys are exact:

```text
read: canonical path + offset + limit
bash: program + exact argument list + canonical working directory
```

The sandbox uses `rg --files --sort path` instead of `ls` because `ls` is not currently in the gateway command allowlist.

## Expected delivery progression

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

## Session JSON and logs

Because the generated gateway command uses this sandbox as `--root`, evidence is written inside this repository, not inside `loop-whole`:

```text
/Users/tlam/loop-whole-sandbox/logs/<session-id>.log
/Users/tlam/loop-whole-sandbox/.loopwhole/sessions/<session-id>.json
```

The session JSON is written when the gateway shuts down normally, usually when OpenCode exits or disconnects from the MCP child. While the session is running, use the live API instead. A force kill or crash may prevent the final JSON dump, although flushed log lines may still exist.

The gateway session ID comes from `--session-id`/`--resume-session` when configured, or is generated automatically when neither option is present. It is separate from OpenCode's `ses_...` conversation ID.

If the gateway is launched with another `--root`, both logs and `.loopwhole` move under that root. The frontend's location does not determine persistence.

Inspect the newest session:

```bash
ls -lt .loopwhole/sessions
python3 -m json.tool .loopwhole/sessions/<session-id>.json
```

Inspect totals only when `jq` is available:

```bash
jq '.totals' .loopwhole/sessions/<session-id>.json
```

## Pre-run checkpoint for a pitch or video

The recommended split is after Prompt 12. That leaves six short live calls while preserving two compelling cross-process comparisons: the source reread becomes a diff against its preloaded baseline, and tests change from the preloaded failure to passing.

1. Prepare the fresh named session:

   ```bash
   scripts/reset-demo.sh
   rm -f .loopwhole/sessions/pitch-demo.json logs/pitch-demo.log
   scripts/configure.sh --session-id pitch-demo
   opencode
   ```

2. Record the OpenCode conversation ID (the `ses_...` value shown by `opencode session list`) and run Prompts 1–12.
3. Exit OpenCode normally. Do **not** run `reset-demo.sh`. Verify the new dump has exactly 12 calls:

   ```bash
   python3 - <<'PY'
   import json
   state = json.load(open(".loopwhole/sessions/pitch-demo.json"))
   assert len(state["toolCalls"]) == 12
   assert "baselines" in state
   PY
   ```
4. Generate the resumed MCP command:

   ```bash
   scripts/configure.sh --resume-session pitch-demo
   ```

5. During the recording, start the frontend and resume OpenCode's separate conversation:

   ```bash
   opencode -s <ses_...>
   ```

The new gateway child loads the first 12 calls before serving the API. The unchanged frontend immediately shows those calls and appends Prompts 13–18 live. A resume error is fatal rather than silently starting an empty session.

The dump contains the gateway ledger and comparison baselines; OpenCode owns the model conversation. Both must be resumed, and the sandbox must remain at the Prompt 12 filesystem state. Only a normal gateway shutdown creates the checkpoint.

## End and reset

Exit OpenCode normally first so the gateway atomically updates the cumulative session JSON. Stop Vite separately when finished.

To restore source files for another run while retaining evidence:

```bash
cd /Users/tlam/loop-whole-sandbox
scripts/reset-demo.sh
```

To remove prior evidence manually:

```bash
rm -rf logs .loopwhole
```

Before another fresh run, remove or choose a new gateway session ID and regenerate `opencode.json` with `scripts/configure.sh --session-id <new-id>`. The reset preserves the prior `--resume-session` config, so do not launch OpenCode before regenerating it. Do not reset between the pre-run and resumed halves.

## Troubleshooting

### `MCP error -32000: Connection closed`

Check that the generated paths exist:

```bash
test -x /Users/tlam/loop-whole/server/target/debug/warp-mcp-gateway
test -d /Users/tlam/loop-whole-sandbox
```

Regenerate configuration and restart OpenCode:

```bash
scripts/configure.sh
```

### Dashboard reports API unavailable

OpenCode must be running and must show the `Loopwhole` MCP server as connected. Confirm that exactly one process owns port `8787`:

```bash
lsof -nP -iTCP:8787 -sTCP:LISTEN
```

### A repeated call does not become unchanged

Confirm the calls used exactly identical arguments and `cwd` values. Across a process restart, also confirm the gateway was launched with `--resume-session` and the expected dump contains a `baselines` object.

### Session JSON is missing

Exit OpenCode normally and check again. The gateway persists the JSON during shutdown, not after every tool call.
