"""Builds a realistic, self-consistent Loopey demo session dump.

Output conforms to .loopwhole.example/session.schema.json and mirrors the
warp-mcp-gateway session-dump shape (server/src/store.rs::PersistedSession).

Token counting matches the backend: tokens = ceil(char_count / 4)  ("chars_div_4_v1").
Byte counts use UTF-8 length. Totals are summed exactly like server/src/store.rs.
"""

import hashlib
import json
import math
from pathlib import Path

TOKEN_COUNTER = "chars_div_4_v1"
CONTEXT_WINDOW = 200_000
SESSION_ID = "loopey-demo"
START_MS = 1_774_267_200_000  # 2026-03-20T16:00:00Z, arbitrary stable base


def tokens(text: str) -> int:
    return math.ceil(len(text) / 4)


def nbytes(text: str) -> int:
    return len(text.encode("utf-8"))


def short_hash(text: str) -> str:
    return "sha256:" + hashlib.sha256(text.encode("utf-8")).hexdigest()[:12]


def compact_json(value) -> str:
    # Matches serde_json::to_string (no spaces) so input token counts are realistic.
    return json.dumps(value, separators=(",", ":"))


# --- Realistic file contents the agent "reads" ------------------------------

MAIN_RS = """use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use crate::gateway::Gateway;
use crate::store::SessionStore;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = ".")]
    root: std::path::PathBuf,
    #[arg(long, default_value = "127.0.0.1:8787")]
    api_addr: std::net::SocketAddr,
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long)]
    context_window_tokens: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let store = SessionStore::new(session_summary(&args));
    let gateway = Gateway::new(store.clone());
    serve_api(args.api_addr, store.clone()).await?;
    gateway.serve_stdio().await?;
    store.persist_on_shutdown().await?;
    Ok(())
}
""" * 3  # padded so the "large unchanged file" reads are genuinely sizable

STORE_RS = """use std::sync::{Arc, RwLock};

use crate::schema::{NewToolCall, SessionSnapshot, TokenTotals, ToolCallSummary};

#[derive(Debug, Clone)]
pub struct SessionStore {
    inner: Arc<RwLock<StoreData>>,
}

impl SessionStore {
    pub fn record(&self, call: NewToolCall) -> i64 {
        let mut store = self.inner.write().expect("lock poisoned");
        let id = store.next_id;
        store.next_id += 1;
        store.tool_calls.push(StoredToolCall { id, call });
        id
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let store = self.inner.read().expect("lock poisoned");
        let input = store.tool_calls.iter().map(|c| c.call.input_tokens).sum();
        let original = store.tool_calls.iter().map(|c| c.call.original_output_tokens).sum();
        let intercepted = store.tool_calls.iter().map(|c| c.call.intercepted_output_tokens).sum();
        SessionSnapshot::new(store.session.clone(), input, original, intercepted)
    }
}
""" * 3

SCHEMA_RS_V1 = """#[derive(Debug, Clone)]
pub struct NewToolCall {
    pub sequence: u64,
    pub occurred_at_ms: i64,
    pub tool_name: String,
    pub subject_path: Option<String>,
    pub status: String,
    pub delivery_mode: String,
    pub original_text: String,
    pub intercepted_text: String,
    pub input_tokens: u64,
    pub original_output_tokens: u64,
    pub intercepted_output_tokens: u64,
}
""" * 2

# After the agent's write: a `decision_reason` field is added.
SCHEMA_RS_V2 = SCHEMA_RS_V1.replace(
    "    pub delivery_mode: String,\n",
    "    pub delivery_mode: String,\n    pub decision_reason: Option<String>,\n",
    1,
)

DIFF_TEXT = """@@ schema.rs (baseline sha256:{old} -> current sha256:{new}) @@
 pub struct NewToolCall {{
     pub delivery_mode: String,
+    pub decision_reason: Option<String>,
     pub original_text: String,
[loopey] delivered 1 changed hunk; 41 unchanged lines suppressed"""

# After the agent's edit: a single unique line in store.rs gains a comment.
# `.replace(..., 1)` mutates only the first occurrence, so this stays a
# realistic one-hunk change even though the demo file is padded.
STORE_RS_V2 = STORE_RS.replace(
    "        store.next_id += 1;\n",
    "        store.next_id += 1; // ids are session-local\n",
    1,
)

STORE_DIFF_TEXT = """@@ store.rs (baseline sha256:{old} -> current sha256:{new}) @@
         let id = store.next_id;
-        store.next_id += 1;
+        store.next_id += 1; // ids are session-local
         store.tool_calls.push(StoredToolCall {{ id, call }});
[loopey] delivered 1 changed hunk; 71 unchanged lines suppressed"""

# --- Bash: same command always executes; only delivery is compacted ---------
# The MCP `bash` tool spawns an allowlisted program directly (no shell). The
# original pane is the real bounded stdout/stderr; the intercepted pane is the
# canonical Cargo-test projection (first run) or an unchanged marker (repeat).

CARGO_TEST_RAW = """   Compiling warp-mcp-gateway v0.1.0 (/home/dev/warp-mcp-gateway)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.21s
     Running unittests src/lib.rs (target/debug/deps/warp_mcp_gateway-9f2c1a3b7d)

running 2 tests
test tools::tests::read_rejects_zero_offset ... ok
test commands::tests::cargo_test_projection_parses ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

[Exit code: 0]"""

CARGO_TEST_CANONICAL = """Cargo test: PASSED
2 passed; 0 failed
[Exit code: 0]"""


def suppressed_stub(seq_first_seen: int, h: str) -> str:
    return (
        f"[loopey] file unchanged since seq {seq_first_seen} "
        f"(baseline {h}); 0 bytes re-sent to the model"
    )


def command_unchanged_stub(seq_first_seen: int, label: str) -> str:
    return (
        f"[loopey] command output unchanged since seq {seq_first_seen} "
        f"({label}); 0 bytes re-sent to the model"
    )


# --- Build the tool-call ledger ---------------------------------------------

calls = []


def add_call(seq, tool, path, status, mode, reason, baseline_hash, current_hash,
             input_obj, original_text, intercepted_text, duration_ms):
    input_text = compact_json(input_obj)
    calls.append({
        "id": seq,
        "sequence": seq,
        "occurredAtMs": START_MS + seq * 1000,
        "toolName": tool,
        "subjectPath": path,
        "status": status,
        "durationMs": duration_ms,
        "deliveryMode": mode,
        "decisionReason": reason,
        "baselineHash": baseline_hash,
        "currentHash": current_hash,
        "input": input_obj,
        "original": {
            "text": original_text,
            "bytes": nbytes(original_text),
            "tokens": tokens(original_text),
        },
        "intercepted": {
            "text": intercepted_text,
            "bytes": nbytes(intercepted_text),
            "tokens": tokens(intercepted_text),
        },
        "_input_tokens": tokens(input_text),  # stripped before writing
    })


h_main = short_hash(MAIN_RS)
h_store = short_hash(STORE_RS)
h_store_v2 = short_hash(STORE_RS_V2)
h_schema_v1 = short_hash(SCHEMA_RS_V1)
h_schema_v2 = short_hash(SCHEMA_RS_V2)
h_cargo = short_hash(CARGO_TEST_CANONICAL)

# 1: first read of main.rs -> unseen, full delivery
add_call(1, "read", "src/main.rs", "success", "full", "no_baseline_observed",
         None, h_main,
         {"path": "src/main.rs"}, MAIN_RS, MAIN_RS, 5)

# 2: first read of store.rs -> unseen, full delivery
add_call(2, "read", "src/store.rs", "success", "full", "no_baseline_observed",
         None, h_store,
         {"path": "src/store.rs"}, STORE_RS, STORE_RS, 6)

# 3: first read of schema.rs -> unseen, full delivery
add_call(3, "read", "src/schema.rs", "success", "full", "no_baseline_observed",
         None, h_schema_v1,
         {"path": "src/schema.rs"}, SCHEMA_RS_V1, SCHEMA_RS_V1, 4)

# 4: re-read main.rs (unchanged) -> suppressed
add_call(4, "read", "src/main.rs", "success", "unchanged", "hash_match_since_seq_1",
         h_main, h_main,
         {"path": "src/main.rs"}, MAIN_RS, suppressed_stub(1, h_main), 3)

# 5: re-read store.rs (unchanged) -> suppressed
add_call(5, "read", "src/store.rs", "success", "unchanged", "hash_match_since_seq_2",
         h_store, h_store,
         {"path": "src/store.rs"}, STORE_RS, suppressed_stub(2, h_store), 3)

# 6: write schema.rs (adds decision_reason field) -> passthrough
write_confirmation = f"Successfully wrote {nbytes(SCHEMA_RS_V2)} bytes to src/schema.rs"
add_call(6, "write", "src/schema.rs", "success", "passthrough", "state_optimization_not_enabled",
         None, h_schema_v2,
         {"path": "src/schema.rs", "content": SCHEMA_RS_V2},
         write_confirmation, write_confirmation, 8)

# 7: re-read schema.rs (changed since seq 3) -> diff delivery
diff = DIFF_TEXT.format(old=h_schema_v1.split(":")[1], new=h_schema_v2.split(":")[1])
add_call(7, "read", "src/schema.rs", "success", "diff", "changed_since_seq_3",
         h_schema_v1, h_schema_v2,
         {"path": "src/schema.rs"}, SCHEMA_RS_V2, diff, 5)

# 8: edit store.rs (one exact unique replacement) -> passthrough
# Edit is a mutation-safety tool: its confirmation is already tiny, so
# original == intercepted and this call saves no output tokens directly.
edit_confirmation = "Edited src/store.rs"
add_call(8, "edit", "src/store.rs", "success", "passthrough", "exact_text_replaced",
         h_store, h_store_v2,
         {"path": "src/store.rs",
          "old_text": "        store.next_id += 1;",
          "new_text": "        store.next_id += 1; // ids are session-local"},
         edit_confirmation, edit_confirmation, 7)

# 9: re-read store.rs (changed since seq 5 by the edit) -> diff delivery
# This is the downstream payoff of the edit: the prior read baseline still
# exists, so the next identical read can be delivered as a compact diff.
store_diff = STORE_DIFF_TEXT.format(old=h_store.split(":")[1], new=h_store_v2.split(":")[1])
add_call(9, "read", "src/store.rs", "success", "diff", "changed_since_seq_5",
         h_store, h_store_v2,
         {"path": "src/store.rs"}, STORE_RS_V2, store_diff, 4)

# 10: run `cargo test` (no baseline yet) -> executes, canonical projection
# Bash always executes; the first run has no baseline so Loopey returns the
# canonical projection instead of the full noisy output.
add_call(10, "bash", "cargo test", "success", "compressed", "no_command_baseline",
         None, h_cargo,
         {"program": "cargo", "args": ["test"]},
         CARGO_TEST_RAW, CARGO_TEST_CANONICAL, 4213)

# 11: run identical `cargo test` again -> executes again, output unchanged
# The command runs a second time (not skipped); the canonical result and exit
# code match the baseline, so only a short unchanged marker is delivered.
add_call(11, "bash", "cargo test", "success", "unchanged", "command_output_unchanged",
         h_cargo, h_cargo,
         {"program": "cargo", "args": ["test"]},
         CARGO_TEST_RAW, command_unchanged_stub(10, "cargo test"), 3980)


# --- Totals (mirrors store.rs) ----------------------------------------------

input_tokens_total = sum(c["_input_tokens"] for c in calls)
original_total = sum(c["original"]["tokens"] for c in calls)
intercepted_total = sum(c["intercepted"]["tokens"] for c in calls)
without_runtime = input_tokens_total + original_total
with_runtime = input_tokens_total + intercepted_total
saved = without_runtime - with_runtime
savings_percent = round(saved * 100.0 / without_runtime, 2) if without_runtime else 0.0
without_ctx_pct = round(without_runtime * 100.0 / CONTEXT_WINDOW, 2)
with_ctx_pct = round(with_runtime * 100.0 / CONTEXT_WINDOW, 2)

for c in calls:
    del c["_input_tokens"]

session = {
    "formatVersion": 1,
    "session": {
        "id": SESSION_ID,
        "startedAtMs": START_MS,
        "endedAtMs": START_MS + (len(calls) + 1) * 1000,
        "workspaceRoot": "/home/dev/warp-mcp-gateway",
        "contextWindowTokens": CONTEXT_WINDOW,
        "tokenCounter": TOKEN_COUNTER,
    },
    "totals": {
        "toolInputTokens": input_tokens_total,
        "originalOutputTokens": original_total,
        "interceptedOutputTokens": intercepted_total,
        "withoutRuntimeTokens": without_runtime,
        "withRuntimeTokens": with_runtime,
        "savedTokens": saved,
        "savingsPercent": savings_percent,
        "withoutRuntimeContextPercent": without_ctx_pct,
        "withRuntimeContextPercent": with_ctx_pct,
    },
    "toolCalls": calls,
    "baselines": {"reads": [], "commands": []},
}

repo_root = Path(__file__).resolve().parent.parent
serialized = json.dumps(session, indent=2) + "\n"

# The canonical reference dump plus the copy the web landing/dashboard imports.
out_paths = [
    repo_root / ".loopwhole.example" / "demo-session.json",
    repo_root / "web" / "src" / "data" / "demo-session.json",
]
for out_path in out_paths:
    if out_path.parent.exists():
        out_path.write_text(serialized, encoding="utf-8")
        print(f"wrote {out_path}")

print(f"  tool calls:        {len(calls)}")
print(f"  without runtime:   {without_runtime} tokens")
print(f"  with runtime:      {with_runtime} tokens")
print(f"  saved:             {saved} tokens ({savings_percent}%)")
