# Smoke test: repeated unchanged Cargo test

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `bash` with:
   - `program: cargo`
   - `args: ["test", "--quiet"]`
   - `cwd: .`
2. Call Loopwhole `bash` again using only the `command_id` returned by step 1. Do not resend the program, arguments, or working directory.
3. Stop. Report whether the second result was exactly `NoC`.

Do not skip the second execution. Expected gateway decisions: first `compressed`, then `unchanged`. The second command should report both input and output token savings.
