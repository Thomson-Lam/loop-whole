# Smoke test: repeated unchanged Cargo test

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `bash` with:
   - `program: cargo`
   - `args: ["test", "--quiet"]`
   - `cwd: .`
2. Call Loopwhole `bash` again with exactly the same program, arguments, and working directory.
3. Stop. Report whether the second result said that the command output or relevant normalized result was unchanged.

Do not skip the second execution. Expected gateway decisions: first `compressed`, then `unchanged`. The second command should report positive token savings.
