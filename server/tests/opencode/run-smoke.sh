#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SERVER_ROOT="$ROOT/server"
TEST_ROOT="$SERVER_ROOT/tests/opencode"
FIXTURE="$TEST_ROOT/fixture"
WORKSPACE="$TEST_ROOT/workspace"
CONFIG="$TEST_ROOT/opencode.json"

SCENARIOS=(
  01-read-unchanged
  02-read-diff
  03-write-edit
  04-bash-unchanged
  05-bash-diff
  06-bash-id-reuse
  07-bash-edit-id
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
    printf 'sequence\ttool\tmode\tinput\toriginal_input\tinput_saved\toriginal_output\tintercepted_output\ttotal_saved\toutput_savings_pct\n'
    { grep '^{' "$log" || true; } | jq -r '
      select(.event == "tool_call") |
      [.sequence, .toolName, .deliveryMode, .inputTokens,
       .originalInputTokens, .inputSavedTokens, .originalOutputTokens,
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
    return 1
  fi
}

assert_results() {
  local scenario="$1"
  local session_id="$2"
  local log="$WORKSPACE/logs/$session_id.log"
  local dump="$WORKSPACE/.loopwhole/sessions/$session_id.json"
  local assertion

  case "$scenario" in
    06-bash-id-reuse)
      assertion='[
        .[] | select(.event == "tool_call")
      ] as $calls |
      ($calls | length) == 2 and
      $calls[0].toolName == "bash" and
      $calls[0].deliveryMode == "compressed" and
      $calls[1].toolName == "bash" and
      $calls[1].deliveryMode == "unchanged" and
      $calls[1].inputSavedTokens > 0 and
      $calls[1].outputSavingsPercent > 0 and
      $calls[1].savedTokens > 0'
      ;;
    07-bash-edit-id)
      assertion='[
        .[] | select(.event == "tool_call")
      ] as $calls |
      ($calls | length) == 3 and
      $calls[0].toolName == "bash" and
      $calls[0].deliveryMode == "compressed" and
      $calls[1].toolName == "bash_edit" and
      $calls[1].deliveryMode == "compressed" and
      $calls[1].inputSavedTokens > 0 and
      $calls[2].toolName == "bash" and
      $calls[2].deliveryMode == "unchanged" and
      $calls[2].inputSavedTokens > 0 and
      $calls[2].outputSavingsPercent > 0 and
      ([$calls[].savedTokens] | add) > 0'
      ;;
    *)
      return 0
      ;;
  esac

  if ! { grep '^{' "$log" || true; } | jq -s -e "$assertion" >/dev/null; then
    echo "Scenario metric assertions failed: $scenario" >&2
    return 1
  fi

  case "$scenario" in
    06-bash-id-reuse)
      assertion='def command_id:
        . | capture("\\[Command ID: (?<id>cmd-[0-9a-f]+)\\]").id;
        .toolCalls as $calls |
        ($calls[0].intercepted.text | command_id) as $first_id |
        $calls[1].input.command_id == $first_id'
      ;;
    07-bash-edit-id)
      assertion='def command_id:
        . | capture("\\[Command ID: (?<id>cmd-[0-9a-f]+)\\]").id;
        .toolCalls as $calls |
        ($calls[0].intercepted.text | command_id) as $first_id |
        ($calls[1].intercepted.text | command_id) as $edited_id |
        $first_id != $edited_id and
        $calls[1].input.command_id == $first_id and
        $calls[2].input.command_id == $edited_id and
        ($calls[0].original.text | contains("before")) and
        ($calls[1].original.text | contains("after"))'
      ;;
  esac

  if ! jq -e "$assertion" "$dump" >/dev/null; then
    echo "Scenario command-ID assertions failed: $scenario" >&2
    return 1
  fi
  echo "Scenario assertions: PASS"
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
    --arg binary "$SERVER_ROOT/target/debug/warp-mcp-gateway" \
    --arg workspace "$WORKSPACE" \
    --arg session "$session_id" \
    '.mcp.Loopwhole.command = [
      $binary,
      "--root", $workspace,
      "--api-addr", "127.0.0.1:0",
      "--session-id", $session
    ]' "$CONFIG")"
  prompt="$(
    cat "$SERVER_ROOT/tests/context.md"
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
  assert_results "$scenario" "$session_id"
}

command -v jq >/dev/null || {
  echo "jq is required" >&2
  exit 1
}
command -v opencode >/dev/null || {
  echo "opencode is required" >&2
  exit 1
}
if [[ ${1:-all} == "all" || ${1:-} == "06-bash-id-reuse" || ${1:-} == "07-bash-edit-id" ]]; then
  command -v python3 >/dev/null || {
    echo "python3 is required for Bash command-ID scenarios" >&2
    exit 1
  }
fi

cargo build --manifest-path "$SERVER_ROOT/Cargo.toml"

if [[ ${1:-all} == "all" ]]; then
  for scenario in "${SCENARIOS[@]}"; do
    run_one "$scenario"
  done
else
  run_one "$1"
fi
