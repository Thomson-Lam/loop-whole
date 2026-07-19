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
    --instance-id astropy__astropy-13398 scikit-learn__scikit-learn-12682 django__django-14631 pydata__xarray-3095 sympy__sympy-14248 \
    --workers 3
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
