# Smoke test: reusable Bash command ID

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `bash` exactly once with:
   - `program: python3`
   - `args: ["-"]`
   - `cwd: .`
   - `stdin` set to this exact text:

     ```python
     prefix = "stable-agent-smoke"
     alphabet = "abcdefghijklmnopqrstuvwxyz"
     for index in range(1, 81):
         parity = "even" if index % 2 == 0 else "odd"
         print(f"{prefix}:{index:03}:{parity}:{alphabet}")
     ```

2. Call Loopwhole `bash` exactly once using only the `command_id` returned by step 1. Do not resend program, args, cwd, or stdin.
3. Stop. Report whether the second call executed, returned an unchanged marker, and used fewer input and output tokens.

Expected gateway decisions: `compressed`, then `unchanged`. The runner will fail unless the second call has positive input-token savings, positive total savings, and references the exact ID returned by the first call.
