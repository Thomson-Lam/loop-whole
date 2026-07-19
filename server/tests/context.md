# OpenCode agent context: Loopwhole MCP smoke test

You are running a controlled diagnostic of the Loopwhole MCP gateway. The goal is to execute the requested tool calls exactly and verify that the model-visible `intercepted` result is smaller than the counterfactual `original` result when an optimization applies.

The smoke runner injects the contents of these implementation references after this context:

- @docs/tools/read.md
- @docs/tools/write.md
- @docs/tools/edit.md
- @docs/tools/bash.md
- @docs/tests/manual.md

Do not call any tool to fetch these `@` paths. Read their injected contents directly so documentation lookup does not pollute the tool-call benchmark.

## Rules

1. Use only Loopwhole MCP tools for read, write, edit, and command operations.
2. Do not substitute OpenCode native filesystem, patch, or Bash tools.
3. Follow the scenario's paths, offsets, limits, programs, arguments, and working directories exactly. Baseline matching uses exact request keys.
4. Repeated Bash commands must execute again; the prior result is only a comparison baseline.
5. Do not make unrelated changes or attempt to improve the fixture.
6. After the requested calls, report the observed delivery behavior and stop.

## Interpretation

For each tool call:

- `original` means the bounded result the current invocation would normally return;
- `intercepted` means the exact result delivered to you;
- `savedTokens` includes both full-command input reuse and output reduction; `outputSavingsPercent` isolates output reduction;
- these instructions and linked documentation are not included in gateway token totals;
- `full` and `passthrough` commonly save zero;
- `unchanged`, `diff`, and `compressed` are expected to reduce output when their compact representation is smaller.

Write and edit are mutation-safety tools and normally use passthrough output. Their changes establish scenarios that allow later reads or commands to demonstrate compaction.

If behavior differs from the scenario expectation, report the exact tool arguments, returned text, delivery behavior if visible, and likely diagnosis from the linked tool documentation. Do not hide zero or negative savings.
