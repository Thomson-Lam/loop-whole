#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEST_ROOT="$ROOT/tests/opencode"
FIXTURE="$TEST_ROOT/fixture"
WORKSPACE="$TEST_ROOT/workspace"
CONFIG="$TEST_ROOT/opencode.json"

SCENARIOS=(
  01-read-unchanged
  02-read-diff
  03-write-edit
  04-bash-unchanged
  05-bash-diff
)

DOCUMENTS=(
  docs/tools/read.md
  docs/tools/write.md
  docs/tools/edit.md
  docs/tools/bash.md
  docs/tests/manual.md
)

reset_workspace() {
  rm -rf "$WORKSPACE"
  mkdir -p "$WORKSPACE"
  cp "$FIXTURE/Cargo.toml" "$WORKSPACE/"
  cp -R "$FIXTURE/src" "$WORKSPACE/"
}

print_results() {
  local session_id="$1"
  local log="$WORKSPACE/logs/$session_id.log"
  local dump="$WORKSPACE/.loopwhole/sessions/$session_id.json"

  echo
  echo "Tool-call benchmark log:"
  if [[ -f "$log" ]]; then
    printf 'sequence\ttool\tmode\toriginal\tintercepted\tsaved\toutput_savings_pct\n'
    { grep '^{' "$log" || true; } | jq -r '
      select(.event == "tool_call") |
      [.sequence, .toolName, .deliveryMode, .originalOutputTokens,
       .interceptedOutputTokens, .savedTokens,
       (.outputSavingsPercent | tostring)] | @tsv
    '
  else
    echo "missing: $log"
  fi

  echo
  echo "Session totals:"
  if [[ -f "$dump" ]]; then
    jq '.totals' "$dump"
  else
    echo "missing: $dump"
  fi
}

run_one() {
  local scenario="$1"
  local instruction="$TEST_ROOT/instructions/$scenario.md"
  local session_id="opencode-$scenario"

  if [[ ! -f "$instruction" ]]; then
    echo "Unknown scenario: $scenario" >&2
    exit 2
  fi

  reset_workspace
  local config_content
  local prompt
  config_content="$(jq -c \
    --arg binary "$ROOT/target/debug/warp-mcp-gateway" \
    --arg workspace "$WORKSPACE" \
    --arg session "$session_id" \
    '.mcp.Loopwhole.command = [
      $binary,
      "--root", $workspace,
      "--api-addr", "127.0.0.1:8787",
      "--session-id", $session
    ]' "$CONFIG")"
  prompt="$(
    cat "$ROOT/tests/context.md"
    for document in "${DOCUMENTS[@]}"; do
      printf '\n\n--- BEGIN %s ---\n\n' "$document"
      cat "$ROOT/$document"
      printf '\n\n--- END %s ---\n' "$document"
    done
    printf '\n\n--- BEGIN SCENARIO ---\n\n'
    cat "$instruction"
  )"

  echo
  echo "=== $scenario ==="
  (
    cd "$ROOT"
    OPENCODE_CONFIG_CONTENT="$config_content" \
      opencode run "$prompt"
  )
  print_results "$session_id"
}

command -v jq >/dev/null || {
  echo "jq is required" >&2
  exit 1
}
command -v opencode >/dev/null || {
  echo "opencode is required" >&2
  exit 1
}

cargo build --manifest-path "$ROOT/Cargo.toml"

if [[ ${1:-all} == "all" ]]; then
  for scenario in "${SCENARIOS[@]}"; do
    run_one "$scenario"
  done
else
  run_one "$1"
fi
