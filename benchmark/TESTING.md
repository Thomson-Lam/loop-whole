## Generate prediction from coding harness

Codex (default):

```bash
venv/bin/python generate_predictions.py \
    --instance-id astropy__astropy-13398 scikit-learn__scikit-learn-12682 django__django-14631 pydata__xarray-3095 sympy__sympy-14248 \
    --dataset SWE-bench/SWE-bench_Verified \
    --workers 2

venv/bin/python generate_predictions.py \
    --backend opencode \
    --dataset SWE-bench/SWE-bench_Verified \
    --instance-id sympy__sympy-14248 \
    --workers 1


scikit-learn__scikit-learn-12682 django__django-14631 pydata__xarray-3095 sympy__sympy-14248 \
--work-dir .swebench-shard-1
```

OpenCode:

```bash
venv/bin/python generate_predictions.py \
    --backend opencode \
    --dataset SWE-bench/SWE-bench_Verified \
    --limit 2
```

## Run SWE-BENCH
```bash
python -m swebench.harness.run_evaluation \
    --dataset_name SWE-bench/SWE-bench_Verified \
    --predictions_path predictions.jsonl \
    --max_workers 2 \
    --namespace '' \
    --run_id more_tests
```

## Build and test frontend evidence

Run the standard-library bridge tests from the repository root:

```bash
python3 -m unittest benchmark/test_build_benchmark_results.py
```

Build a compact frontend artifact from matched evaluator reports and preserved
session roots:

```bash
python3 benchmark/build_benchmark_results.py \
    --baseline-sessions /path/to/baseline/sessions \
    --mcp-sessions /path/to/mcp/sessions \
    --baseline-report /path/to/baseline-report.json \
    --mcp-report /path/to/mcp-report.json \
    --output web/src/data/benchmark-results.json
```

Then validate the consumer:

```bash
npm --prefix web run lint
npm --prefix web run build
```
