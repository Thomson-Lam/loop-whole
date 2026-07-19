# Smoke test: edit and reuse a stored Bash command

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `bash` exactly once with:
   - `program: python3`
   - `args: ["-"]`
   - `cwd: .`
   - `stdin` set to this exact text:

     ```python
     prefix = "before"
     alphabet = "abcdefghijklmnopqrstuvwxyz"
     for index in range(1, 61):
         width = index * 3
         print(f"{prefix}:{index:03}:{width:03}:{alphabet}")
     ```

2. Call Loopwhole `bash_edit` exactly once using the command ID returned by step 1, with `old_text: before` and `new_text: after`.
3. Call Loopwhole `bash` exactly once using only the new command ID returned by step 2. Do not resend program, args, cwd, or stdin.
4. Stop. Report whether `bash_edit` ran the edited script and the final call returned exactly `NoC`.

Expected gateway decisions: `compressed`, `compressed`, then `unchanged`. The runner will fail unless `bash_edit` has positive input-token savings, returns a different command ID, the final call references that new ID, and the scenario has positive total savings.
