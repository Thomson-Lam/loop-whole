# Smoke test: changed Cargo test result

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `bash` with `program: cargo`, `args: ["test", "--quiet"]`, and `cwd: .`.
2. Call Loopwhole `edit` on `src/lib.rs`, replacing the exact text:

   ```text
   assert_eq!(status(), "ready");
   ```

   with:

   ```text
   assert_eq!(status(), "broken");
   ```

3. Call Loopwhole `bash` again using only the `command_id` returned by step 1.
4. Stop. Report whether the second command returned a progressive change showing the newly failing test.

Expected decisions: bash `compressed`, edit `passthrough`, bash `diff`. The second Cargo invocation must execute and fail; the diff should preserve the failure information.
