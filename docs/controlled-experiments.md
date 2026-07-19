# Controlled experiments

## Purpose

These experiments test whether Loop-Whole's context-aware tools behave as designed when a real coding-agent harness invokes them. They are mechanism checks, not evidence of production-wide token savings.

The latest run used OpenCode as the harness with its configured `build` agent and `gpt-5.5` model. The environment was OpenCode `1.17.7`, Rust/Cargo `1.94.1`, Python `3.14.5`, and Darwin `24.6.0 arm64`, based on Git commit `e2622c4` plus the documented working-tree changes. Provider-side sampling settings were not captured, which is itself a reproducibility limitation.

The complete suite was launched from the repository root with:

```bash
server/tests/opencode/run-smoke.sh all 2>&1 | tee controlled-experiments.txt
```

## Experimental controls

For every scenario, the runner:

1. rebuilt the current debug gateway;
2. deleted and recreated `server/tests/opencode/workspace/` from one committed fixture;
3. started a fresh Loop-Whole session rooted at that isolated workspace;
4. disabled OpenCode's native read, write, edit, patch, and Bash tools;
5. injected the same tool documentation and diagnostic context;
6. instructed the agent to make an exact, bounded sequence of Loop-Whole calls;
7. recorded the bounded original result, the result delivered to the model, decision metadata, and estimated tokens;
8. persisted the shutdown session dump for inspection.

The estimate is deliberately simple:

```text
estimated tokens = ceil(character count / 4)
```

`without runtime` combines the counterfactual full-command input and original tool output. `with runtime` combines the actual compact input and intercepted output. For command-ID calls, the original input is the equivalent full command DTO. Prompts, injected documentation, model reasoning, assistant prose, MCP envelopes, and the harness's own context are excluded.

## Results

Each scenario was run once in the latest full-suite execution. Percentages below are paired comparisons within that controlled session, not population estimates.

| Scenario | Controlled comparison and observed result | Session result | Significance and why it happened | Potential bias / why real agentic development may differ |
| --- | --- | ---: | --- | --- |
| `01-read-unchanged` | First read was `full`; the identical second read was `unchanged`. On the repeated call, output fell from **391 to 1 token** (`NoC`), saving **390 tokens / 99.74%**. | **804 → 414**, saving **390 / 48.51%** | Confirms exact read-view baselines and the one-token unchanged marker. The large repeated file view was already in context, so retransmission carried no new evidence. | The prompt forced an identical path, offset, and limit. Natural agents may change ranges, avoid rereading, or reread only a few lines; any of those lowers the opportunity or absolute savings. The fixture is also more repetitive than many source files. |
| `02-read-diff` | A full read was followed by one exact edit and the same read. The changed read returned a unified diff: **392 → 56 output tokens**, saving **336 / 85.71%**. | **849 → 513**, saving **336 / 39.58%** | Confirms progressive state: the new view replaced the old baseline, and only the changed guard-sized region was delivered. | The experiment changed one compact, known location. Broad formatting, generated files, line movement, or many scattered edits can make a unified diff as large as the current view, causing full delivery and little or no saving. |
| `03-write-edit` | Create-only write succeeded, overwrite was rejected, exact edit succeeded, then a tiny file was read twice. The repeated two-token read became one-token `NoC`, saving **1 token / 50%** on that call. | **90 → 89**, saving **1 / 1.11%** | Demonstrates that the optimization does not manufacture impressive savings where little redundancy exists. Write/edit confirmations remain passthrough; only the final unchanged read was eligible. | This purposefully tiny file creates a denominator effect: 50% per-call output reduction is only one token. In real work, tool instructions, assistant text, and protocol overhead may dominate and make this saving immaterial. |
| `04-bash-unchanged` | Cargo tests executed twice; the second call used the returned command ID and received `NoC`. First-run command-ID metadata cost **4 extra output tokens**. The repeated call reduced input **14 → 10** and output **61 → 1**, saving **64 total tokens** on that call. | **150 → 90**, saving **60 / 40.00%** | Confirms commands are re-executed rather than cached, while both repeated input and unchanged output can be compacted. It also exposes the first-run metadata cost instead of hiding it. | Cargo output was short and stable, and build artifacts were warm. Real commands may emit timestamps, nondeterministic ordering, warnings, progress output, or different exit details; normalization may not classify them as unchanged. Agents may also vary flags or working directories, creating different baseline keys. |
| `05-bash-diff` | Passing Cargo tests ran, a test was changed to fail, then the command was rerun by ID. The changed failure was delivered in full canonical form because its diff was not smaller: final output **131 → 131**, while ID input saved **4 tokens**. First-run metadata cost the same **4 tokens**, producing net zero. | **254 → 254**, saving **0 / 0.00%** | This is an important negative result. Loop-Whole preserved the failure and refused to send a larger diff merely to claim compression. Correctness and diagnostic evidence won over token reduction. | The fixture has one small failure whose canonical output is already compact. A large real test suite with one changed failure may benefit much more; conversely, widespread failures may produce no reduction. One handcrafted mutation cannot estimate either distribution. |
| `06-bash-id-reuse` | An 80-line deterministic Python command ran twice. The first result paid **9 extra output tokens** for command-ID metadata. The ID rerun reduced input **68 → 10** and output **1,094 → 1**, saving **1,151 tokens / 99.91% output reduction** on the repeated call. | **2,324 → 1,182**, saving **1,142 / 49.14%** | Directly validates the intended high-opportunity case: a long one-time script can become a compact reusable command, and stable repeated output collapses to `NoC`. | The script was intentionally deterministic, verbose, and repeated immediately. Real ad-hoc scripts often run once, include changing repository state, or produce small output. This scenario therefore measures mechanism capacity, not natural opportunity frequency. |
| `07-bash-edit-id` | A stored Python script ran, `bash_edit` changed `before` to `after`, and the edited command was rerun by its new ID. `bash_edit` reduced input **57 → 19**; despite **9 tokens** of new-ID output overhead, it saved **29 total tokens**. The final rerun reduced input **57 → 10** and output **619 → 1**, saving **665 tokens**. | **2,044 → 1,359**, saving **685 / 33.51%** | Confirms ID lineage, exact command editing, new-ID generation, execution of the edited script, and reuse of the new command. It also shows that input reuse can offset metadata overhead before any unchanged-output win. | The edit was a unique five-character substitution in a deliberately long script, and the edited command was guaranteed to run again. Real edits may be ambiguous, may change most of a script, or may never be reused. New commands also do not inherit old output baselines, so the edit call itself may not compress output. |

Across these seven purpose-built scenarios, totals were **6,515 tokens without Loop-Whole versus 3,901 with it**, a reduction of **2,614 / 40.12%**. This aggregate must not be treated as an expected production saving: the suite intentionally over-samples repeated calls to exercise the optimization, gives every scenario equal weight, and contains only one run per scenario.

## What the experiments establish

The controlled evidence supports these narrow claims:

- OpenCode can discover and invoke the Loop-Whole MCP tools with native equivalents disabled.
- Exact repeated reads return `NoC` while changed reads preserve the change.
- Repeated commands execute again and can reuse a command ID.
- A stored command can be edited by ID, executed, assigned a new ID, and reused.
- First-run command-ID metadata can produce negative savings.
- Changed results can legitimately produce zero savings when a diff is not smaller.
- Under deliberately eligible repeated workloads, both input and output token estimates fall substantially.

It does **not** establish a production-average reduction, lower model billing, faster wall-clock completion, better task success, or compatibility beyond the tested harness and environment.

## Threats to validity

### Construct validity

- The character-count estimator is not a provider tokenizer.
- The gateway measures serialized tool arguments and results, not total conversation context or billed tokens.
- The `original` side is an in-process paired counterfactual, not a separately executed untreated agent trajectory.
- Injected documentation teaches the model how to interpret `NoC` but is excluded from measured totals.

### Internal validity

- Each scenario has one observation and no confidence interval.
- The model was instructed to issue exact calls, so agent choice and discovery behavior were largely removed.
- Cargo's local build cache and deterministic fixture can reduce output variability.
- The same implementation produces the decision and telemetry, so an independent tokenizer or transcript audit is still desirable.

### External validity

- Only OpenCode was exercised in these smoke runs.
- The fixture is a small Rust crate, not a long-lived, concurrent, multilingual repository.
- Real agents change arguments, working directories, read ranges, and command order.
- Real command output can contain timestamps, nondeterministic ordering, environment-specific paths, progress bars, or flaky failures.
- Session interruption, stale state, concurrent agents, large baseline stores, and long-duration resume behavior were not stressed.
- The allowlist is demo policy rather than an operating-system sandbox.

## Required follow-up experiments

1. **Natural-trajectory opportunity study:** measure repeated-key and unchanged-output frequency in unmodified coding-agent trajectories before claiming an expected saving.
2. **Paired task evaluation:** run the same tasks, model, harness, prompt, and repository state with Loop-Whole enabled and disabled; compare task resolution, full transcript tokens, tool tokens, latency, and cost.
3. **Replication:** repeat each condition across seeds, models, harnesses, repositories, languages, and command families; report distributions and confidence intervals.
4. **Independent accounting:** tokenize complete exported transcripts with the provider tokenizer and reconcile them with gateway estimates.
5. **Statefulness tests:** exercise resume, interruption, stale baselines, command-ID lifetime, concurrent calls, and large session stores.
6. **Failure-preservation tests:** add flaky, timed-out, truncated, warning-heavy, and nondeterministic command outputs and verify that compression never removes actionable diagnostics.
7. **Operational testing:** separate or disable the demo HTTP/UI child server, introduce production telemetry, and evaluate security boundaries before real-world deployment.

## Reproduction and evidence

- Runner: `server/tests/opencode/run-smoke.sh`
- Scenario prompts: `server/tests/opencode/instructions/`
- Fixture: `server/tests/opencode/fixture/`
- Current-scenario generated logs: `server/tests/opencode/workspace/logs/`
- Current-scenario generated session dump: `server/tests/opencode/workspace/.loopwhole/sessions/`
- Full-suite stdout capture when using the reproduction command above: `controlled-experiments.txt`
- Detailed mechanics: `docs/tools/read.md` and `docs/tools/bash.md`

The generated workspace is reset before every scenario and is intentionally ignored by Git.
