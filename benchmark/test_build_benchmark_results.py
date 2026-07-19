import argparse
import json
import tempfile
import unittest
from pathlib import Path

from benchmark.build_benchmark_results import (
    BridgeError,
    build_results,
    discover_sessions,
    load_report,
    run,
)


TASK_A = "django__django-11099"
TASK_B = "sympy__sympy-14248"


def session(task_id: str, *, legacy: bool = False, workspace_match: bool = True) -> dict:
    root = (
        f"/tmp/.swebench_codex/runs/{task_id}-abc/repo"
        if workspace_match
        else "/tmp/checkout"
    )
    data = {
        "session": {
            "id": f"session-{task_id.split('-')[-1]}",
            "startedAtMs": 1,
            "endedAtMs": 2,
            "workspaceRoot": root,
            "contextWindowTokens": 200000,
            "tokenCounter": "chars_div_4_v1",
        },
        "totals": {
            "toolInputTokens": 2,
            "originalOutputTokens": 10,
            "interceptedOutputTokens": 6,
            "withoutRuntimeTokens": 12,
            "withRuntimeTokens": 8,
            "savedTokens": 4,
            "savingsPercent": 33.3,
            "withoutRuntimeContextPercent": 0.01,
            "withRuntimeContextPercent": 0.0,
        },
        "toolCalls": [
            {
                "id": 1,
                "sequence": 1,
                "occurredAtMs": 1,
                "toolName": "read",
                "subjectPath": f"/tmp/.swebench_codex/runs/{task_id}-abc/repo/file.py",
                "status": "success",
                "durationMs": 1,
                "deliveryMode": "full",
                "decisionReason": "test",
                "baselineHash": None,
                "currentHash": "hash",
                "input": {"path": "file.py"},
                "original": {"text": "text", "bytes": 4, "tokens": 1},
                "intercepted": {"text": "text", "bytes": 4, "tokens": 1},
            },
            {
                "id": 2,
                "sequence": 2,
                "occurredAtMs": 2,
                "toolName": "bash",
                "subjectPath": ".",
                "status": "success",
                "durationMs": 1,
                "deliveryMode": "compressed",
                "decisionReason": "test",
                "baselineHash": None,
                "currentHash": "hash2",
                "input": {"program": "rg", "args": ["--files"], "cwd": "."},
                "original": {"text": "text", "bytes": 4, "tokens": 1},
                "intercepted": {"text": "text", "bytes": 4, "tokens": 1},
            },
        ],
        "baselines": {"reads": [], "commands": []},
    }
    if not legacy:
        data["formatVersion"] = 1
    return data


class ReportTests(unittest.TestCase):
    def test_loads_aggregate_and_record_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            aggregate = root / "aggregate.json"
            aggregate.write_text(
                json.dumps(
                    {
                        "resolved_ids": [TASK_A],
                        "completed_ids": [TASK_A, TASK_B],
                    }
                )
            )
            records = root / "records.jsonl"
            records.write_text(
                "\n".join(
                    [
                        json.dumps({"instance_id": TASK_A, "resolved": True}),
                        json.dumps({"instanceId": TASK_B, "resolved": False}),
                    ]
                )
            )
            self.assertEqual(load_report(aggregate), {TASK_A: True, TASK_B: False})
            self.assertEqual(load_report(records), {TASK_A: True, TASK_B: False})

    def test_counts_submitted_errors_and_empty_patches_as_unresolved(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            path.write_text(
                json.dumps(
                    {
                        "submitted_ids": [TASK_A, TASK_B, "pytest-dev__pytest-7432"],
                        "resolved_ids": [TASK_A],
                        "error_ids": [TASK_B],
                        "empty_patch_ids": ["pytest-dev__pytest-7432"],
                    }
                )
            )
            self.assertEqual(
                load_report(path),
                {
                    TASK_A: True,
                    TASK_B: False,
                    "pytest-dev__pytest-7432": False,
                },
            )

    def test_rejects_conflicting_outcomes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            path.write_text(
                json.dumps(
                    {"resolved_ids": [TASK_A], "unresolved_ids": [TASK_A]}
                )
            )
            with self.assertRaisesRegex(BridgeError, "conflicting"):
                load_report(path)


class SessionTests(unittest.TestCase):
    def test_discovers_current_and_legacy_sessions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a.json").write_text(json.dumps(session(TASK_A)))
            (root / "b.json").write_text(json.dumps(session(TASK_B, legacy=True)))
            found = discover_sessions(root, {TASK_A: True, TASK_B: False})
            self.assertEqual(set(found), {TASK_A, TASK_B})

    def test_falls_back_to_subject_path(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "session.json").write_text(
                json.dumps(session(TASK_A, workspace_match=False))
            )
            found = discover_sessions(root, {TASK_A: True})
            self.assertIn(TASK_A, found)

    def test_ignores_unrelated_json_and_unselected_sessions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "broken.json").write_text("{not json")
            (root / "selected.json").write_text(json.dumps(session(TASK_A)))
            (root / "other.json").write_text(json.dumps(session(TASK_B)))
            found = discover_sessions(root, {TASK_A: True})
            self.assertEqual(set(found), {TASK_A})

    def test_rejects_duplicate_sessions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            payload = json.dumps(session(TASK_A))
            (root / "one.json").write_text(payload)
            (root / "two.json").write_text(payload)
            with self.assertRaisesRegex(BridgeError, "multiple sessions"):
                discover_sessions(root, {TASK_A: True})


class BridgeTests(unittest.TestCase):
    def test_builds_compact_paired_results(self):
        baseline = {TASK_A: True, TASK_B: False}
        mcp = {TASK_A: True, TASK_B: True}
        baseline_sessions = {task: session(task) for task in baseline}
        mcp_sessions = {task: session(task) for task in mcp}
        result = build_results(
            baseline,
            mcp,
            baseline_sessions,
            mcp_sessions,
            "SWE-bench Verified",
            "model",
            "description",
        )
        self.assertFalse(result["mock"])
        self.assertEqual([item["id"] for item in result["instances"]], [TASK_A, TASK_B])
        run = result["instances"][0]["mcp"]
        self.assertEqual(run["toolCalls"], 2)
        self.assertEqual(run["toolContextTokens"], 8)
        self.assertEqual(run["toolCounts"], {"bash": 1, "read": 1})
        self.assertNotIn("snapshot", run)

    def test_rejects_different_task_sets(self):
        with self.assertRaisesRegex(BridgeError, "different task IDs"):
            build_results(
                {TASK_A: True},
                {TASK_B: True},
                {},
                {},
                "benchmark",
                "model",
                "description",
            )

    def test_cli_flow_writes_frontend_result(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            baseline_dir = root / "baseline"
            mcp_dir = root / "mcp"
            baseline_dir.mkdir()
            mcp_dir.mkdir()
            (baseline_dir / "session.json").write_text(json.dumps(session(TASK_A)))
            (mcp_dir / "session.json").write_text(json.dumps(session(TASK_A)))
            baseline_report = root / "baseline.json"
            mcp_report = root / "mcp.json"
            baseline_report.write_text(json.dumps({"resolved_ids": [TASK_A]}))
            mcp_report.write_text(json.dumps({TASK_A: {"resolved": True}}))
            output = root / "results.json"
            args = argparse.Namespace(
                baseline_sessions=baseline_dir,
                mcp_sessions=mcp_dir,
                baseline_report=baseline_report,
                mcp_report=mcp_report,
                output=output,
                benchmark_name="SWE-bench Verified",
                model="model",
                description="description",
            )
            self.assertEqual(run(args), 0)
            result = json.loads(output.read_text())
            self.assertFalse(result["mock"])
            self.assertEqual(len(result["instances"]), 1)


if __name__ == "__main__":
    unittest.main()
