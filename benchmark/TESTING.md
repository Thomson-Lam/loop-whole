## Generate prediction from coding harness

Codex (default):

```bash
venv/bin/python generate_predictions.py \
    --dataset SWE-bench/SWE-bench_Verified \
    --limit 2
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
    --run_id test2
```
