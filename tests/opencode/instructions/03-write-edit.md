# Smoke test: create-only write and exact edit

Use only the Loopwhole MCP tools. Do not use OpenCode's native filesystem or shell tools.

Perform exactly these steps:

1. Call Loopwhole `write` to create `notes.txt` with this exact content:

   ```text
   alpha
   ```

2. Call Loopwhole `write` again for `notes.txt` with content `replacement`. Confirm that overwriting is rejected.
3. Call Loopwhole `edit` for `notes.txt`, replacing exact text `alpha` with `beta`.
4. Call Loopwhole `read` for `notes.txt` with `offset: 1` and `limit: 20`.
5. Call the same read again with exactly the same arguments.
6. Stop and report whether create-only write, overwrite rejection, exact edit, and unchanged-read compaction all behaved as expected.

Expected decisions: write `passthrough`, write `error`, edit `passthrough`, read `full`, read `unchanged`.
