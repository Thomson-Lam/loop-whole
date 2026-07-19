"""Build deterministic mock SWE-bench comparison data for the frontend.

Each baseline/MCP run embeds a format-v1 Loop-Whole session snapshot so the UI
fixture stays aligned with the gateway's persisted session shape. Replace this
fixture with evaluated run data before presenting real benchmark results.
"""

import json
from pathlib import Path

START_MS = 1_774_267_200_000
TOOLS = ("read", "bash", "read", "edit", "read", "bash", "write")
TASKS = (
    ("django__django-11099", True, True, 24, 22),
    ("sympy__sympy-14248", True, True, 31, 29),
    ("scikit-learn__scikit-learn-10297", True, False, 19, 20),
    ("pytest-dev__pytest-7432", False, True, 22, 21),
    ("matplotlib__matplotlib-18869", True, True, 27, 25),
    ("astropy__astropy-12907", True, True, 18, 18),
    ("pallets__flask-4045", False, False, 20, 19),
    ("sphinx-doc__sphinx-10451", False, False, 25, 23),
)


def payload(label: str, tokens: int) -> dict:
    seed = label + " "
    text = (seed * (tokens * 4 // len(seed) + 1))[: tokens * 4]
    return {
        "text": text,
        "bytes": len(text.encode("utf-8")),
        "tokens": tokens,
    }


def build_calls(task_id: str, variant: str, count: int) -> list[dict]:
    run_id = f"{task_id}-{variant}-mock"
    root = f"/tmp/.swebench_codex/runs/{run_id}/repo"
    calls = []
    seen = set()

    for index in range(count):
        sequence = index + 1
        tool = TOOLS[index % len(TOOLS)]
        subject = (
            root
            if tool == "bash"
            else f"{root}/src/{task_id.split('__', 1)[-1].split('-', 1)[0]}.py"
        )
        key = (tool, subject)
        repeated = key in seen
        seen.add(key)
        original_tokens = 22 + (index % 5) * 4

        if variant == "baseline":
            mode = "full" if tool == "read" else "passthrough"
            intercepted_tokens = original_tokens
        elif repeated and tool in {"read", "bash"}:
            mode = "unchanged" if index % 3 else "diff"
            intercepted_tokens = 1 if mode == "unchanged" else 13
        else:
            mode = "full" if tool == "read" else "passthrough"
            intercepted_tokens = original_tokens

        if tool == "bash":
            input_value = {
                "program": "rg",
                "args": ["--files", "--sort", "path"],
                "cwd": ".",
            }
        elif tool == "write":
            input_value = {"path": subject, "content": "mock content\n"}
        elif tool == "edit":
            input_value = {"path": subject, "old_text": "old", "new_text": "new"}
        else:
            input_value = {"path": subject}
        original = payload(f"{tool} result", original_tokens)
        intercepted = (
            {"text": "NoC", "bytes": 3, "tokens": 1}
            if mode == "unchanged"
            else payload(f"{mode} result", intercepted_tokens)
        )
        calls.append(
            {
                "id": sequence,
                "sequence": sequence,
                "occurredAtMs": START_MS + sequence * 1000,
                "toolName": tool,
                "subjectPath": subject,
                "status": "success",
                "durationMs": 20 + index * 3,
                "deliveryMode": mode,
                "decisionReason": "mock_benchmark_fixture",
                "baselineHash": None if not repeated else f"mock-{index - 1:04d}",
                "currentHash": f"mock-{index:04d}",
                "input": input_value,
                "original": original,
                "intercepted": intercepted,
                "_inputTokens": (
                    len(json.dumps(input_value, separators=(",", ":"))) + 3
                )
                // 4,
            }
        )

    return calls


def build_snapshot(task_id: str, variant: str, count: int, task_index: int) -> dict:
    calls = build_calls(task_id, variant, count)
    input_tokens = sum(call.pop("_inputTokens") for call in calls)
    original_tokens = sum(call["original"]["tokens"] for call in calls)
    intercepted_tokens = sum(call["intercepted"]["tokens"] for call in calls)
    without_runtime = input_tokens + original_tokens
    with_runtime = input_tokens + intercepted_tokens
    saved = without_runtime - with_runtime
    run_id = f"{task_id}-{variant}-mock"

    return {
        "formatVersion": 1,
        "session": {
            "id": f"mock-{variant}-{task_index + 1:02d}",
            "startedAtMs": START_MS,
            "endedAtMs": START_MS + (count + 1) * 1000,
            "workspaceRoot": f"/tmp/.swebench_codex/runs/{run_id}/repo",
            "contextWindowTokens": 200000,
            "tokenCounter": "chars_div_4_v1",
        },
        "totals": {
            "toolInputTokens": input_tokens,
            "originalOutputTokens": original_tokens,
            "interceptedOutputTokens": intercepted_tokens,
            "withoutRuntimeTokens": without_runtime,
            "withRuntimeTokens": with_runtime,
            "savedTokens": saved,
            "savingsPercent": round(saved * 100 / without_runtime, 2),
            "withoutRuntimeContextPercent": round(without_runtime * 100 / 200000, 2),
            "withRuntimeContextPercent": round(with_runtime * 100 / 200000, 2),
        },
        "toolCalls": calls,
        "baselines": {"reads": [], "commands": []},
    }


def main() -> None:
    instances = []
    for index, (task_id, baseline_resolved, mcp_resolved, baseline_calls, mcp_calls) in enumerate(TASKS):
        instances.append(
            {
                "id": task_id,
                "baseline": {
                    "resolved": baseline_resolved,
                    "snapshot": build_snapshot(task_id, "baseline", baseline_calls, index),
                },
                "mcp": {
                    "resolved": mcp_resolved,
                    "snapshot": build_snapshot(task_id, "mcp", mcp_calls, index),
                },
            }
        )

    output = {
        "mock": True,
        "benchmark": {
            "name": "SWE-bench Verified",
            "model": "Matched model configuration",
            "description": "Coding-performance non-regression check",
        },
        "instances": instances,
    }
    path = Path(__file__).resolve().parent.parent / "web/src/data/benchmark-results.json"
    path.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path}")


if __name__ == "__main__":
    main()
