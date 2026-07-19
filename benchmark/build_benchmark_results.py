#!/usr/bin/env python3
"""Pair SWE-bench outcomes with Loop-Whole sessions for the frontend."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = REPO_ROOT / "web/src/data/benchmark-results.json"


class BridgeError(Exception):
    """Expected input or validation failure."""


def add_outcome(outcomes: dict[str, bool], instance_id: Any, resolved: Any) -> None:
    if not isinstance(instance_id, str) or not instance_id:
        raise BridgeError("evaluation outcome has a missing instance ID")
    if not isinstance(resolved, bool):
        raise BridgeError(f"evaluation outcome for {instance_id} is not boolean")
    previous = outcomes.get(instance_id)
    if previous is not None and previous != resolved:
        raise BridgeError(f"evaluation report has conflicting outcomes for {instance_id}")
    outcomes[instance_id] = resolved


def load_report(path: Path) -> dict[str, bool]:
    try:
        if path.suffix == ".jsonl":
            data: Any = [
                json.loads(line)
                for line in path.read_text(encoding="utf-8").splitlines()
                if line.strip()
            ]
        else:
            data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BridgeError(f"failed to read evaluation report {path}: {error}") from error

    outcomes: dict[str, bool] = {}
    parse_report_value(data, outcomes)
    if not outcomes:
        raise BridgeError(f"evaluation report {path} has no per-instance outcomes")
    return outcomes


def parse_report_value(data: Any, outcomes: dict[str, bool]) -> None:
    if isinstance(data, list):
        for record in data:
            if not isinstance(record, dict):
                continue
            instance_id = record.get("instance_id", record.get("instanceId"))
            if instance_id is not None and "resolved" in record:
                add_outcome(outcomes, instance_id, record["resolved"])
        return

    if not isinstance(data, dict):
        return

    record_id = data.get("instance_id", data.get("instanceId"))
    if record_id is not None and "resolved" in data:
        add_outcome(outcomes, record_id, data["resolved"])

    resolved_ids = data.get("resolved_ids", data.get("resolvedIds", []))
    unresolved_ids = data.get("unresolved_ids", data.get("unresolvedIds", []))
    completed_ids = data.get("completed_ids", data.get("completedIds", []))
    submitted_ids = data.get("submitted_ids", data.get("submittedIds", []))
    error_ids = data.get("error_ids", data.get("errorIds", []))
    empty_patch_ids = data.get("empty_patch_ids", data.get("emptyPatchIds", []))
    resolved_set = set(resolved_ids) if isinstance(resolved_ids, list) else set()
    for instance_id in resolved_set:
        add_outcome(outcomes, instance_id, True)
    for values in (unresolved_ids, error_ids, empty_patch_ids):
        if isinstance(values, list):
            for instance_id in values:
                add_outcome(outcomes, instance_id, False)
    denominator_ids: set[str] = set()
    for values in (completed_ids, submitted_ids):
        if isinstance(values, list):
            denominator_ids.update(values)
    for instance_id in denominator_ids - resolved_set:
        add_outcome(outcomes, instance_id, False)

    for wrapper in ("instances", "results"):
        if wrapper in data:
            parse_report_value(data[wrapper], outcomes)

    for instance_id, value in data.items():
        if isinstance(value, dict) and "resolved" in value and "__" in instance_id:
            add_outcome(outcomes, instance_id, value["resolved"])
        elif isinstance(value, bool) and "__" in instance_id:
            add_outcome(outcomes, instance_id, value)


def validate_session(path: Path, data: Any) -> dict[str, Any] | None:
    if not isinstance(data, dict):
        return None
    session_keys = {"session", "totals", "toolCalls"}
    if not session_keys.intersection(data):
        return None
    if not session_keys.issubset(data):
        raise BridgeError(f"session-like JSON is incomplete: {path}")
    if data.get("formatVersion") not in (None, 1):
        raise BridgeError(f"unsupported session format in {path}")

    session = data["session"]
    totals = data["totals"]
    calls = data["toolCalls"]
    if not isinstance(session, dict) or not isinstance(session.get("workspaceRoot"), str):
        raise BridgeError(f"session workspaceRoot is invalid: {path}")
    tokens = totals.get("withRuntimeTokens") if isinstance(totals, dict) else None
    if isinstance(tokens, bool) or not isinstance(tokens, int) or tokens < 0:
        raise BridgeError(f"session withRuntimeTokens is invalid: {path}")
    if not isinstance(calls, list):
        raise BridgeError(f"session toolCalls is not an array: {path}")
    for index, call in enumerate(calls):
        if not isinstance(call, dict) or not isinstance(call.get("toolName"), str):
            raise BridgeError(f"session tool call {index} is invalid: {path}")
        subject = call.get("subjectPath")
        if subject is not None and not isinstance(subject, str):
            raise BridgeError(f"session tool call {index} has invalid subjectPath: {path}")
    return data


def match_instance(
    session: dict[str, Any], known_ids: set[str], path: Path
) -> str | None:
    candidates = [session["session"]["workspaceRoot"]]
    candidates.extend(
        call["subjectPath"]
        for call in session["toolCalls"]
        if isinstance(call.get("subjectPath"), str)
    )
    matches = {
        instance_id
        for instance_id in known_ids
        for candidate in candidates
        if re.search(
            rf"(?<![A-Za-z0-9]){re.escape(instance_id)}(?![A-Za-z0-9])",
            candidate,
        )
    }
    if not matches:
        return None
    if len(matches) > 1:
        raise BridgeError(f"session {path} matches multiple task IDs: {sorted(matches)}")
    return next(iter(matches))


def discover_sessions(root: Path, outcomes: dict[str, bool]) -> dict[str, dict[str, Any]]:
    sessions: dict[str, dict[str, Any]] = {}
    for path in sorted(root.rglob("*.json")):
        try:
            raw = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            continue
        session = validate_session(path, raw)
        if session is None:
            continue
        instance_id = match_instance(session, set(outcomes), path)
        if instance_id is None:
            continue
        if instance_id in sessions:
            raise BridgeError(f"multiple sessions found for {instance_id} under {root}")
        sessions[instance_id] = session

    missing = set(outcomes) - set(sessions)
    if missing:
        raise BridgeError(f"missing sessions under {root} for: {', '.join(sorted(missing))}")
    return sessions


def compact_run(resolved: bool, session: dict[str, Any]) -> dict[str, Any]:
    calls = session["toolCalls"]
    counts = Counter(call["toolName"] for call in calls)
    return {
        "resolved": resolved,
        "toolCalls": len(calls),
        "toolContextTokens": session["totals"]["withRuntimeTokens"],
        "toolCounts": dict(sorted(counts.items())),
    }


def build_results(
    baseline_outcomes: dict[str, bool],
    mcp_outcomes: dict[str, bool],
    baseline_sessions: dict[str, dict[str, Any]],
    mcp_sessions: dict[str, dict[str, Any]],
    benchmark_name: str,
    model: str,
    description: str,
) -> dict[str, Any]:
    if set(baseline_outcomes) != set(mcp_outcomes):
        only_baseline = sorted(set(baseline_outcomes) - set(mcp_outcomes))
        only_mcp = sorted(set(mcp_outcomes) - set(baseline_outcomes))
        raise BridgeError(
            "baseline/MCP reports contain different task IDs "
            f"(baseline only: {only_baseline}; MCP only: {only_mcp})"
        )

    instances = []
    for instance_id in sorted(baseline_outcomes):
        instances.append(
            {
                "id": instance_id,
                "baseline": compact_run(
                    baseline_outcomes[instance_id], baseline_sessions[instance_id]
                ),
                "mcp": compact_run(mcp_outcomes[instance_id], mcp_sessions[instance_id]),
            }
        )
    return {
        "formatVersion": 1,
        "mock": False,
        "benchmark": {
            "name": benchmark_name,
            "model": model,
            "description": description,
        },
        "instances": instances,
    }


def write_atomic(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            "w", encoding="utf-8", dir=path.parent, prefix=f".{path.name}.", delete=False
        ) as file:
            temporary = Path(file.name)
            json.dump(data, file, indent=2)
            file.write("\n")
            file.flush()
            os.fsync(file.fileno())
        os.replace(temporary, path)
    finally:
        if temporary is not None and temporary.exists():
            temporary.unlink()


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build compact frontend evidence from SWE-bench reports and sessions."
    )
    parser.add_argument("--baseline-sessions", type=Path, required=True)
    parser.add_argument("--mcp-sessions", type=Path, required=True)
    parser.add_argument("--baseline-report", type=Path, required=True)
    parser.add_argument("--mcp-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--benchmark-name", default="SWE-bench Verified")
    parser.add_argument("--model", default="Matched model configuration")
    parser.add_argument("--description", default="Coding-performance non-regression check")
    return parser.parse_args(argv)


def run(args: argparse.Namespace) -> int:
    for label in ("baseline_sessions", "mcp_sessions"):
        path = getattr(args, label)
        if not path.is_dir():
            raise BridgeError(f"session directory does not exist: {path}")
    for label in ("baseline_report", "mcp_report"):
        path = getattr(args, label)
        if not path.is_file():
            raise BridgeError(f"evaluation report does not exist: {path}")

    baseline_outcomes = load_report(args.baseline_report)
    mcp_outcomes = load_report(args.mcp_report)
    if set(baseline_outcomes) != set(mcp_outcomes):
        build_results(
            baseline_outcomes,
            mcp_outcomes,
            {},
            {},
            args.benchmark_name,
            args.model,
            args.description,
        )
    baseline_sessions = discover_sessions(args.baseline_sessions, baseline_outcomes)
    mcp_sessions = discover_sessions(args.mcp_sessions, mcp_outcomes)
    results = build_results(
        baseline_outcomes,
        mcp_outcomes,
        baseline_sessions,
        mcp_sessions,
        args.benchmark_name,
        args.model,
        args.description,
    )
    write_atomic(args.output, results)
    print(f"wrote {args.output} ({len(results['instances'])} paired tasks)")
    return 0


def main(argv: list[str] | None = None) -> int:
    try:
        return run(parse_args(argv))
    except BridgeError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
