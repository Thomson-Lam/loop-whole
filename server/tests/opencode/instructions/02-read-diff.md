# Smoke test: changed read view

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `read` for `src/lib.rs` with `offset: 1` and `limit: 200`.
2. Call Loopwhole `edit` on `src/lib.rs`, replacing the exact text:

   ```text
   pub fn status() -> &'static str {
       "ready"
   }
   ```

   with:

   ```text
   pub fn status() -> &'static str {
       "changed"
   }
   ```

3. Call Loopwhole `read` again with exactly `path: src/lib.rs`, `offset: 1`, and `limit: 200`.
4. Stop. Report whether the final read returned only the compact change.

Expected gateway decisions: read `full`, edit `passthrough`, read `diff`. The final read should report positive token savings.
