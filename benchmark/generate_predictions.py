#!/usr/bin/env python3
"""Generate SWE-bench patch predictions with Codex or OpenCode.

Each benchmark instance is checked out in an isolated clone, handed to the
selected coding-agent CLI, and written to a JSONL file in the format expected by
the SWE-bench evaluation harness.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import threading
from pathlib import Path
from typing import Any, Iterable

DEFAULT_DATASET = "SWE-bench/SWE-bench_Verified"
DEFAULT_OUTPUT = Path("predictions.jsonl")
DEFAULT_WORK_DIR = Path(".swebench_codex")
DEFAULT_OPENCODE_CONFIG = Path(__file__).resolve().with_name("opencode.json")

_repo_locks: dict[str, threading.Lock] = {}
_repo_locks_guard = threading.Lock()
_refreshed_repos: set[str] = set()
_console_lock = threading.Lock()


class PredictionError(RuntimeError):
    """An error tied to one SWE-bench instance."""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Load a SWE-bench dataset from Hugging Face and ask Codex or OpenCode "
            "to solve it."
        )
    )
    parser.add_argument("--dataset", default=DEFAULT_DATASET, help="Hugging Face dataset name")
    parser.add_argument("--split", default="test", help="Dataset split (default: test)")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT, help="Prediction JSONL path")
    parser.add_argument(
        "--errors-output",
        type=Path,
        help="Failure JSONL path (default: <output>.errors.jsonl)",
    )
    parser.add_argument("--work-dir", type=Path, default=DEFAULT_WORK_DIR)
    parser.add_argument(
        "--backend",
        choices=("codex", "opencode"),
        default="codex",
        help="Coding-agent CLI to run (default: codex)",
    )
    parser.add_argument(
        "--model",
        help="Model passed to the selected backend; defaults to its current config",
    )
    parser.add_argument(
        "--model-name",
        help="Name recorded in predictions (default: --model or <backend>-default)",
    )
    parser.add_argument("--codex-bin", default="codex", help="Codex CLI executable")
    parser.add_argument(
        "--codex-arg",
        action="append",
        default=[],
        help="Extra argument passed to `codex exec` (repeatable)",
    )
    parser.add_argument("--opencode-bin", default="opencode", help="OpenCode CLI executable")
    parser.add_argument(
        "--opencode-config",
        type=Path,
        default=DEFAULT_OPENCODE_CONFIG,
        help="OpenCode config file (default: opencode.json beside this script)",
    )
    parser.add_argument(
        "--opencode-arg",
        action="append",
        default=[],
        help="Extra argument passed to `opencode run` (repeatable)",
    )
    parser.add_argument("--workers", type=int, default=1, help="Concurrent agent runs")
    parser.add_argument("--timeout", type=int, default=3600, help="Seconds allowed per agent run")
    parser.add_argument("--limit", type=int, help="Process at most this many pending instances")
    parser.add_argument("--start", type=int, default=0, help="Skip this many selected instances")
    parser.add_argument(
        "--instance-id",
        action="append",
        dest="instance_ids",
        help="Only run this instance ID (repeatable)",
    )
    parser.add_argument(
        "--include-hints",
        action="store_true",
        help="Include hints_text from the dataset in the agent prompt",
    )
    parser.add_argument(
        "--keep-worktrees",
        action="store_true",
        help="Keep instance repositories after each run for debugging",
    )
    parser.add_argument(
        "--no-refresh-cache",
        action="store_true",
        help="Do not fetch updates for repositories already in the local mirror cache",
    )
    args = parser.parse_args()

    if args.workers < 1:
        parser.error("--workers must be at least 1")
    if args.timeout < 1:
        parser.error("--timeout must be at least 1")
    if args.start < 0:
        parser.error("--start cannot be negative")
    if args.limit is not None and args.limit < 0:
        parser.error("--limit cannot be negative")
    return args


def run_command(
    command: list[str],
    *,
    cwd: Path | None = None,
    timeout: int | None = None,
    check: bool = True,
) -> subprocess.CompletedProcess[str]:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise PredictionError(f"command timed out after {timeout}s: {shlex.join(command)}") from exc
    if check and result.returncode != 0:
        tail = result.stdout[-4000:].strip()
        raise PredictionError(
            f"command failed with exit code {result.returncode}: {shlex.join(command)}"
            + (f"\n{tail}" if tail else "")
        )
    return result


def safe_name(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value)


def repo_lock(repo: str) -> threading.Lock:
    with _repo_locks_guard:
        return _repo_locks.setdefault(repo, threading.Lock())


def ensure_repo_mirror(repo: str, cache_dir: Path, refresh: bool) -> Path:
    """Create/update one bare mirror, serialized when workers share a repository."""
    mirror = cache_dir / f"{safe_name(repo)}.git"
    with repo_lock(repo):
        if not mirror.exists():
            run_command(
                ["git", "clone", "--mirror", f"https://github.com/{repo}.git", str(mirror)]
            )
            _refreshed_repos.add(repo)
        elif refresh and repo not in _refreshed_repos:
            run_command(["git", "fetch", "--prune", "origin"], cwd=mirror)
            _refreshed_repos.add(repo)
    return mirror


def make_checkout(instance: dict[str, Any], args: argparse.Namespace) -> tuple[Path, Path]:
    repo = str(instance["repo"])
    instance_id = str(instance["instance_id"])
    base_commit = str(instance["base_commit"])
    cache_dir = args.work_dir / "repos"
    runs_dir = args.work_dir / "runs"
    cache_dir.mkdir(parents=True, exist_ok=True)
    runs_dir.mkdir(parents=True, exist_ok=True)

    mirror = ensure_repo_mirror(repo, cache_dir, not args.no_refresh_cache)
    run_root = Path(tempfile.mkdtemp(prefix=f"{safe_name(instance_id)}-", dir=runs_dir))
    checkout = run_root / "repo"
    try:
        run_command(["git", "clone", "--no-checkout", "--shared", str(mirror), str(checkout)])
        result = run_command(
            ["git", "checkout", "--detach", base_commit], cwd=checkout, check=False
        )
        if result.returncode != 0:
            # A stale mirror may not contain the requested commit. Fetch it once explicitly.
            with repo_lock(repo):
                run_command(["git", "fetch", "origin", base_commit], cwd=mirror)
            run_command(["git", "fetch", "origin", base_commit], cwd=checkout)
            run_command(["git", "checkout", "--detach", base_commit], cwd=checkout)
    except Exception:
        shutil.rmtree(run_root, ignore_errors=True)
        raise
    return run_root, checkout


def build_prompt(instance: dict[str, Any], include_hints: bool) -> str:
    prompt = f"""You are solving SWE-bench instance {instance['instance_id']}.

Repository: {instance['repo']}
Base commit: {instance['base_commit']}

Issue:
{instance['problem_statement']}

Inspect the repository, implement the smallest complete fix for the issue, and run relevant tests.
Work directly in the current checkout. Do not only describe the solution: edit the files.
Do not commit your changes. Do not modify or add tests unless the issue explicitly requires it.
Leave the working tree containing exactly the implementation changes that should be submitted.
"""
    hints = str(instance.get("hints_text") or "").strip()
    if include_hints and hints:
        prompt += f"\nAdditional hints from the benchmark:\n{hints}\n"
    return prompt


def collect_patch(checkout: Path) -> str:
    # Intent-to-add makes new text files appear in `git diff` without staging contents.
    run_command(["git", "add", "--intent-to-add", "--all"], cwd=checkout)
    patch = run_command(["git", "diff", "--binary", "HEAD"], cwd=checkout).stdout
    return patch.strip() + ("\n" if patch.strip() else "")


def build_agent_command(
    args: argparse.Namespace, checkout: Path, prompt: str, last_message: Path
) -> tuple[list[str], str | None]:
    """Build a backend command and return any text that should be sent on stdin."""
    if args.backend == "opencode":
        command = [args.opencode_bin, "run", "--dir", str(checkout)]
        if args.model:
            command.extend(["--model", args.model])
        command.extend(args.opencode_arg)
        command.append(prompt)
        return command, None

    command = [args.codex_bin, "-a", "never", "exec", "--ephemeral", "--color", "never"]
    command.extend(["--sandbox", "workspace-write", "--cd", str(checkout)])
    if args.model:
        command.extend(["--model", args.model])
    command.extend(args.codex_arg)
    command.extend(["--output-last-message", str(last_message), "-"])
    return command, prompt


def generate_one(instance: dict[str, Any], args: argparse.Namespace) -> dict[str, str]:
    instance_id = str(instance["instance_id"])
    with _console_lock:
        print(
            f"[checkout] {instance_id}: {instance['repo']}@{instance['base_commit']}",
            flush=True,
        )
    run_root, checkout = make_checkout(instance, args)
    log_dir = args.work_dir / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    log_path = log_dir / f"{safe_name(instance_id)}.log"
    last_message = run_root / "last_message.txt"

    prompt = build_prompt(instance, args.include_hints)
    command, command_input = build_agent_command(args, checkout, prompt, last_message)
    backend_label = args.backend.capitalize()
    command_env = None
    if args.backend == "opencode":
        command_env = os.environ.copy()
        command_env["OPENCODE_CONFIG"] = str(args.opencode_config)

    run_details = [
        f"[{args.backend}] Starting {instance_id}",
        f"[{args.backend}] Checkout: {checkout}",
        f"[{args.backend}] Model: {args.model or f'{backend_label} config default'}",
    ]
    if args.backend == "opencode":
        run_details.append(f"[opencode] Config: {args.opencode_config}")
    run_details.extend(
        [
            f"[{args.backend}] Timeout: {args.timeout} seconds",
            f"[{args.backend}] Command: {shlex.join(command)}",
            f"[{args.backend}] Prompt:",
            "---------------- PROMPT ----------------",
            prompt.rstrip(),
            "-------------- END PROMPT --------------",
        ]
    )
    with _console_lock:
        print("\n".join(run_details), flush=True)

    try:
        try:
            result = subprocess.run(
                command,
                input=command_input,
                env=command_env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                timeout=args.timeout,
                check=False,
            )
        except subprocess.TimeoutExpired as exc:
            raise PredictionError(f"{backend_label} timed out after {args.timeout}s") from exc
        if result.returncode != 0:
            tail = result.stdout[-4000:].strip()
            raise PredictionError(
                f"{backend_label} exited with code {result.returncode}"
                + (f"\n{tail}" if tail else "")
            )
        patch = collect_patch(checkout)
        if not patch:
            raise PredictionError(f"{backend_label} produced no patch")
        log_path.write_text(patch, encoding="utf-8")
        return {
            "instance_id": instance_id,
            "model_name_or_path": (
                args.model_name or args.model or f"{args.backend}-default"
            ),
            "model_patch": patch,
        }
    finally:
        if not args.keep_worktrees:
            shutil.rmtree(run_root, ignore_errors=True)


def read_completed_ids(output: Path) -> set[str]:
    if not output.exists():
        return set()
    completed: set[str] = set()
    with output.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
                completed.add(str(record["instance_id"]))
            except (json.JSONDecodeError, KeyError, TypeError) as exc:
                raise PredictionError(
                    f"invalid JSONL in {output} at line {line_number}: {exc}"
                ) from exc
    return completed


def append_jsonl(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, ensure_ascii=False) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


def select_instances(dataset: Iterable[dict[str, Any]], args: argparse.Namespace) -> list[dict[str, Any]]:
    requested = set(args.instance_ids or [])
    selected = [row for row in dataset if not requested or row["instance_id"] in requested]
    if requested:
        found = {row["instance_id"] for row in selected}
        missing = sorted(requested - found)
        if missing:
            raise PredictionError(f"instance IDs not found in dataset: {', '.join(missing)}")
    selected = selected[args.start :]
    completed = read_completed_ids(args.output)
    selected = [row for row in selected if row["instance_id"] not in completed]
    if args.limit is not None:
        selected = selected[: args.limit]
    return selected


def main() -> int:
    args = parse_args()
    agent_bin = args.codex_bin if args.backend == "codex" else args.opencode_bin
    if shutil.which(agent_bin) is None:
        print(
            f"error: {args.backend.capitalize()} executable not found: {agent_bin}",
            file=sys.stderr,
        )
        return 2
    if args.backend == "opencode":
        args.opencode_config = args.opencode_config.resolve()
        if not args.opencode_config.is_file():
            print(
                f"error: OpenCode config file not found: {args.opencode_config}",
                file=sys.stderr,
            )
            return 2
    if shutil.which("git") is None:
        print("error: git executable not found", file=sys.stderr)
        return 2

    args.work_dir = args.work_dir.resolve()
    args.output = args.output.resolve()
    errors_output = (args.errors_output or args.output.with_suffix(".errors.jsonl")).resolve()

    # Import lazily so `--help` and argument validation do not initialize the
    # relatively heavy Hugging Face multiprocessing stack.
    from datasets import load_dataset

    print(f"Loading {args.dataset!r} split {args.split!r} from Hugging Face...")
    dataset = load_dataset(args.dataset, split=args.split)
    instances = select_instances(dataset, args)
    if not instances:
        print("No pending instances selected.")
        return 0

    print(
        f"Running {len(instances)} instance(s) with {args.workers} worker(s); "
        f"predictions -> {args.output}"
    )
    failures = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(generate_one, row, args): row for row in instances}
        for future in concurrent.futures.as_completed(futures):
            instance_id = str(futures[future]["instance_id"])
            try:
                prediction = future.result()
            except Exception as exc:  # Keep the batch running and make failures retryable.
                failures += 1
                append_jsonl(errors_output, {"instance_id": instance_id, "error": str(exc)})
                print(f"[failed] {instance_id}: {exc}", file=sys.stderr)
            else:
                append_jsonl(args.output, prediction)
                print(f"[done] {instance_id}")

    print(f"Finished: {len(instances) - failures} succeeded, {failures} failed.")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
