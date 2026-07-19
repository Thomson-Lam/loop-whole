#!/usr/bin/env bash
set -euo pipefail

LOOPWHOLE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_ROOT="$(pwd -P)"
GATEWAY="${LOOPWHOLE_GATEWAY:-}"

usage() {
  cat <<'EOF'
Usage: /path/to/loop-whole/scripts/setup-workspace.sh [GATEWAY]

Run from the workspace to configure. Writes an idempotent AGENTS.md instruction
and project-scoped Claude Code, Codex, and OpenCode MCP configuration.
Set LOOPWHOLE_GATEWAY or pass GATEWAY to override the bundled binary path.
EOF
}

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
  "") ;;
  *)
    if [[ $# -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    GATEWAY="$1"
    ;;
esac

if [[ -z "$GATEWAY" ]]; then
  if [[ -x "$LOOPWHOLE_ROOT/server/target/release/warp-mcp-gateway" ]]; then
    GATEWAY="$LOOPWHOLE_ROOT/server/target/release/warp-mcp-gateway"
  else
    GATEWAY="$LOOPWHOLE_ROOT/server/target/debug/warp-mcp-gateway"
  fi
fi
if [[ "$GATEWAY" != /* ]]; then
  GATEWAY="$(cd "$(dirname "$GATEWAY")" 2>/dev/null && pwd)/$(basename "$GATEWAY")"
fi
if [[ ! -x "$GATEWAY" ]]; then
  printf 'Gateway is not executable: %s\n' "$GATEWAY" >&2
  printf 'Build it with: cargo build --manifest-path %s/server/Cargo.toml --release\n' "$LOOPWHOLE_ROOT" >&2
  exit 1
fi
command -v python3 >/dev/null || {
  echo "python3 is required to merge project configuration safely" >&2
  exit 1
}

python3 - "$TARGET_ROOT" "$GATEWAY" <<'PY'
import json
import pathlib
import re
import sys

target = pathlib.Path(sys.argv[1]).resolve()
gateway = str(pathlib.Path(sys.argv[2]).resolve())
args = ["--root", str(target), "--api-addr", "127.0.0.1:8787"]
instruction = "- Use Loopwhole MCP tools; `NoC` = no relevant change since the prior matching call, and Bash still ran."


def write_json(path, update):
    if path.exists():
        try:
            value = json.loads(path.read_text())
        except json.JSONDecodeError as error:
            raise SystemExit(f"Refusing to overwrite invalid JSON in {path}: {error}")
        if not isinstance(value, dict):
            raise SystemExit(f"Refusing to overwrite non-object JSON in {path}")
    else:
        value = {}
    update(value)
    path.write_text(json.dumps(value, indent=2) + "\n")


agents = target / "AGENTS.md"
existing = agents.read_text() if agents.exists() else ""
if instruction not in existing.splitlines():
    agents.write_text(existing.rstrip("\n") + ("\n" if existing else "") + instruction + "\n")


def update_claude(config):
    servers = config.setdefault("mcpServers", {})
    if not isinstance(servers, dict):
        raise SystemExit("Refusing to replace non-object mcpServers in .mcp.json")
    servers["Loopwhole"] = {"type": "stdio", "command": gateway, "args": args}


write_json(target / ".mcp.json", update_claude)


def update_opencode(config):
    config.setdefault("$schema", "https://opencode.ai/config.json")
    servers = config.setdefault("mcp", {})
    if not isinstance(servers, dict):
        raise SystemExit("Refusing to replace non-object mcp in opencode.json")
    servers["Loopwhole"] = {
        "type": "local",
        "command": [gateway, *args],
        "enabled": True,
        "timeout": 120000,
    }


write_json(target / "opencode.json", update_opencode)

codex = target / ".codex" / "config.toml"
codex.parent.mkdir(parents=True, exist_ok=True)
begin = "# BEGIN LOOPWHOLE MCP"
end = "# END LOOPWHOLE MCP"
block = "\n".join(
    [
        begin,
        "[mcp_servers.Loopwhole]",
        f"command = {json.dumps(gateway)}",
        f"args = {json.dumps(args)}",
        end,
    ]
)
existing = codex.read_text() if codex.exists() else ""
if begin in existing or end in existing:
    pattern = re.compile(re.escape(begin) + r".*?" + re.escape(end), re.DOTALL)
    if len(pattern.findall(existing)) != 1:
        raise SystemExit(f"Refusing to edit malformed Loopwhole block in {codex}")
    updated = pattern.sub(block, existing)
elif re.search(r"^\[mcp_servers\.Loopwhole\]\s*$", existing, re.MULTILINE):
    raise SystemExit(f"Refusing to replace unmanaged [mcp_servers.Loopwhole] in {codex}")
else:
    updated = existing.rstrip("\n") + ("\n\n" if existing else "") + block + "\n"
codex.write_text(updated if updated.endswith("\n") else updated + "\n")
PY

printf 'Configured Loopwhole for workspace: %s\n' "$TARGET_ROOT"
printf 'Gateway: %s\n' "$GATEWAY"
printf 'Updated: AGENTS.md, .mcp.json, .codex/config.toml, opencode.json\n'
