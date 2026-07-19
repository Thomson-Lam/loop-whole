# Codex SWE-bench Prediction Runner

`generate_predictions.py` loads SWE-bench instances from Hugging Face, checks out
each repository at its specified base commit, asks the Codex CLI to implement the
fix, and saves the resulting Git diff in SWE-bench prediction format.

## Prerequisites

You need:

- Python 3.10 or newer
- Git
- Network access to Hugging Face and GitHub
- The `codex` CLI installed and available on `PATH`
- A working Codex login

Check your Codex login with:

```bash
codex login status
```

If necessary, authenticate with:

```bash
codex login
```

## Python setup

Create a virtual environment and install the local SWE-bench package, which also
installs the Hugging Face `datasets` dependency:

```bash
python3 -m venv venv
venv/bin/pip install -e ./SWE-bench
```

## Start with one prediction

Run one SWE-bench Verified instance as a smoke test:

```bash
venv/bin/python generate_predictions.py \
    --dataset SWE-bench/SWE-bench_Verified \
    --limit 1
```

The terminal prints the instance ID, repository and base commit, checkout path,
model, timeout, complete Codex command, and exact prompt sent to Codex.

Generating predictions may consume paid Codex usage. Start with `--limit 1`
before launching a larger batch.

## Common examples

Run the first two Verified instances:

```bash
venv/bin/python generate_predictions.py \
    --dataset SWE-bench/SWE-bench_Verified \
    --limit 2
```

Run SWE-bench Lite:

```bash
venv/bin/python generate_predictions.py \
    --dataset SWE-bench/SWE-bench_Lite
```

Run one specific instance:

```bash
venv/bin/python generate_predictions.py \
    --dataset SWE-bench/SWE-bench_Verified \
    --instance-id sympy__sympy-20590
```

Run several specific instances by repeating the option:

```bash
venv/bin/python generate_predictions.py \
    --instance-id sympy__sympy-20590 \
    --instance-id django__django-11099
```

Use four concurrent Codex runs and allow two hours per instance:

```bash
venv/bin/python generate_predictions.py \
    --workers 4 \
    --timeout 7200
```

Concurrency starts multiple Codex sessions and repository checkouts at once, so
increase `--workers` carefully.

Select a model explicitly:

```bash
venv/bin/python generate_predictions.py --model YOUR_CODEX_MODEL
```

If `--model` is omitted, the Codex CLI uses the model from its current
configuration. Use `--model-name` when the name recorded in the prediction file
should differ from the model argument:

```bash
venv/bin/python generate_predictions.py \
    --model YOUR_CODEX_MODEL \
    --model-name my-experiment
```

Include the dataset's optional hints in the prompt:

```bash
venv/bin/python generate_predictions.py --include-hints
```

## Outputs

By default, the runner creates:

- `predictions.jsonl`: successful predictions in SWE-bench format
- `predictions.errors.jsonl`: failures and their error messages
- `.swebench_codex/logs/<instance_id>.log`: the Git diff for each successful run
- `.swebench_codex/repos/`: cached Git repository mirrors
- `.swebench_codex/runs/`: temporary instance checkouts while runs are active

Each successful prediction has this shape:

```json
{
  "instance_id": "owner__repository-issue_number",
  "model_name_or_path": "codex-default",
  "model_patch": "diff --git ..."
}
```

Choose different output and workspace locations with:

```bash
venv/bin/python generate_predictions.py \
    --output results/verified.jsonl \
    --errors-output results/verified-errors.jsonl \
    --work-dir .runs/verified
```

## Resuming an interrupted run

The output file is resumable. When the command is run again with the same
`--output`, instances already present in that JSONL file are skipped. Failed
instances are not added to the prediction file, so they will be retried.

```bash
venv/bin/python generate_predictions.py \
    --dataset SWE-bench/SWE-bench_Verified \
    --output predictions.jsonl
```

`--start N` skips the first `N` selected dataset rows. `--limit N` then limits
the number of remaining, unfinished rows processed by that invocation.

## Repository cache and debugging

Repositories are mirrored under `.swebench_codex/repos` and reused between
instances. Existing mirrors are refreshed once per invocation. Pass
`--no-refresh-cache` to use them without fetching updates:

```bash
venv/bin/python generate_predictions.py --no-refresh-cache
```

Completed instance checkouts are normally deleted. Preserve them for inspection
with:

```bash
venv/bin/python generate_predictions.py --limit 1 --keep-worktrees
```

Preserved checkouts are placed under `.swebench_codex/runs`.

## Evaluate the predictions

After predictions have been generated, run the SWE-bench evaluation harness:

```bash
venv/bin/python -m swebench.harness.run_evaluation \
    --dataset_name SWE-bench/SWE-bench_Verified \
    --predictions_path predictions.jsonl \
    --max_workers 2 \
    --run_id codex-verified
```

On Apple Silicon or another ARM-based machine, SWE-bench may need to build its
evaluation images locally:

```bash
venv/bin/python -m swebench.harness.run_evaluation \
    --dataset_name SWE-bench/SWE-bench_Verified \
    --predictions_path predictions.jsonl \
    --max_workers 2 \
    --namespace '' \
    --run_id codex-verified
```

Evaluation requires Docker and can use substantial disk space and compute.

## All options

Show the complete command-line reference with:

```bash
venv/bin/python generate_predictions.py --help
```
