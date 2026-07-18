# About

Maintain a state management system to optimize:

1. Tool calls (using a diff system rather than full tool calls).
2. File reads and state management.

This works on two axes:

1. Compaction (compaction is trying to fit less into the context window by using what you already have, such as diffs) vs. compression (truncation is an example of it).
2. Examples: an agent might choose to read the same code snippet or file again. The context runtime engine checks: a) did this file change? and only shows diffs. This also enables humans to edit files agents are working on, and humans and agents can work together more easily.

Attached below is the planning agent's feedback and advice:

## Turn 1

### User

I am building a project to improve developer experience with coding agents and instead of targeting a specific workflow implementation, I am trying to see if the results of tool calling can be condensed/compacted, not using a LLM, cheaply. Check Hypa: https://github.com/Hypabolic/Hypa

### Agent

I checked Hypa, and I think your original intuition points to a potentially more interesting layer than Hypa’s current headline feature.

Hypa today is primarily doing **pre-context reduction**:

```text
tool executes
    ↓
raw result
    ↓
deterministic parser/reducer
    ↓
compact result
    ↓
agent sees compact result
```

It has parser tiers, deterministic filters, token accounting, and raw-output recovery through artifacts. It also has `hypa_read` modes such as `outline`, `signatures`, `pruned`, and `smart`, and its Pi integration can intercept shell calls before their output reaches the agent.

This is a good foundation, but there are actually **two separate compression problems**.

#### 1. Compress an individual result

This is what Hypa mostly does now:

```text
dotnet test
    ↓
4,000 lines
    ↓ deterministic reducer
FAIL MyTest
  Foo.cs:43
  expected 5, got 4
124 tests, 1 failed
exit 1
```

This is relatively straightforward because the semantics of many developer tools are structured enough to reduce without an LLM. Hypa explicitly prefers parsing structured formats into canonical DTOs where possible, falling back through degraded parsing and ultimately safe truncation. ([GitHub][1])

#### 2. Compress results **across time**

This is where your earlier file/diff observation becomes much more interesting.

Consider:

```text
read foo.ts V0
apply patch A
→ success
apply patch B
→ success
read foo.ts V2
grep foo
→ returns lines from V2
apply patch C
→ success
```

Most agent transcripts treat these as six independent historical events.

But semantically, they form a graph:

```text
foo.ts
V0
 ↓ A
V1
 ↓ B
V2
 ↓ C
V3
```

There is a lot of **superseded information** here.

That suggests a different concept from output compression:

> **Deterministic context normalization.**

You could maintain an external event store:

```text
RAW EVENTS
read(foo.ts, V0)
patch(foo.ts, A)
patch(foo.ts, B)
read(foo.ts, V2)
grep(result based on V2)
patch(foo.ts, C)
```

and derive a compact projection:

```text
CURRENT CONTEXT PROJECTION
foo.ts:
  observed at V0
  modified A+B+C
  current hash: abc123
  relevant symbols: Foo, Bar
  full current state available: artifact://abc123
```

The distinction is important:

```text
Compression:
4,000 lines → 30 lines

Compaction:
[event A, event B, event C] → equivalent current state S
```

I think **compaction is the larger opportunity**.

---

#### You can do surprisingly much without an LLM

There are several deterministic operations that are almost universally valid.

##### Supersession

```text
git status at T1
git status at T2
git status at T3
```

Usually only `T3` matters as current state.

You can represent:

```text
git-status current=T3
previous observations superseded=2
```

The raw history remains externally recoverable.

##### Deduplication

```text
grep "Foo"
grep "Foo"
grep "Foo"
```

If the underlying files haven't changed:

```text
same query
same workspace version
same result
```

The later results don't need to consume context again.

This becomes easy with content hashes.

##### Delta coalescing

```text
patch A
patch B
patch C
```

can potentially become:

```text
foo.ts changed V0 → V3
combined delta available
```

You could mechanically compose patches or simply calculate:

```text
diff(V0, V3)
```

No model required.

##### Result invalidation

This is particularly interesting.

Suppose:

```text
grep Foo → result R1
patch foo.ts
grep Foo → result R2
```

You know mechanically that certain observations in `R1` are now stale.

So your context runtime could track:

```text
R1:
  derived_from:
    foo.ts@hash123
    bar.ts@hash456
```

After modifying `foo.ts`:

```text
R1 confidence:
  partially stale
```

That is something ordinary context windows don't express at all.

---

#### The architecture I would pursue

Hypa already talks about provenance, canonical DTOs, evidence storage, compact tool surfaces, and keeping raw results recoverable. ([GitHub][2]) Extend that into a generic intermediate representation:

```text
Tool Call
   ↓
Tool Adapter
   ↓
Canonical Event
   ↓
State Engine
   ├── raw artifact store
   ├── dependency/provenance graph
   └── current projections
           ↓
Context Renderer
           ↓
Agent
```

For example:

```json
{
  "kind": "file.patch",
  "subject": "src/foo.ts",
  "before": "sha256:aaa",
  "after": "sha256:bbb",
  "changedRanges": [[41, 58]],
  "raw": "artifact://event-8291"
}
```

Or:

```json
{
  "kind": "test.run",
  "workspaceVersion": "git:abc123",
  "passed": 143,
  "failed": 2,
  "failures": [...],
  "raw": "artifact://run-9823"
}
```

Then you aren't writing a compressor for arbitrary text.

You're building something closer to:

> **a database execution plan for agent context.**

Raw tool calls are the event log.

The context window is a **materialized view**.

That framing gives you mechanical rules like:

```text
file.read        → cacheable by content hash
file.patch       → advances file version
grep/search      → dependent on matching file versions
test.run         → dependent on workspace version
git.status       → supersedes previous git status
build            → supersedes previous build for same workspace version
ls               → cacheable by directory state
```

No LLM is necessary for any of those.

---

#### The main constraint is where you sit in the stack

There is a major practical distinction.

Hypa can currently reduce a result **before the agent ever sees it**, which is ideal. Its Pi integration does this by intercepting/replacing tools, and its architecture also targets MCP and host-specific adapters. ([GitHub][3])

But once output has entered a hosted agent's transcript:

```text
read V0
patch A
patch B
patch C
```

an external utility generally cannot magically replace those previous context items with:

```text
checkpoint V3
```

unless it controls the agent's transcript assembly.

So I see three levels of integration:

```text
Level 1 — Tool output reduction
Works almost anywhere.
tool → Hypa → compact output → agent

Level 2 — Stateful tool virtualization
Hypa owns reads/search/shell/etc.
agent → Hypa tool
          ↓
      state/cache
          ↓
     minimal result

Level 3 — Transcript compaction
Hypa participates in building each model request.
history → Hypa → normalized history → model
```

**Level 2 may be the sweet spot for a workflow-independent product.**

You don't have to control Claude Code, Codex, Pi, or another agent's entire loop. You make the tools themselves stateful.

For example, the agent asks to reread the same file:

```text
hypa_read("foo.ts")
```

Instead of blindly returning 8,000 tokens:

```text
foo.ts unchanged since your previous read
hash: 918ac...
```

Or after two patches:

```text
foo.ts changed since your previous read:
  lines 38–61
  lines 201–214
[those ranges]
```

The agent already has the rest.

That is **context-aware result generation**, not summarization.

And Hypa's existing `smart` read concept is already heading in this direction, while its code-intelligence architecture explicitly aims to reduce repeated large-file reads through local maps. ([GitHub][4])

---

#### My honest take

I would shift the conceptual pitch from:

> "Hypa compresses noisy coding-agent output."

Toward something like:

> **"Hypa is a context-aware I/O layer for coding agents. It knows what the agent has already observed and only sends what changed or matters."**

The first is mostly a collection of good reducers.

The second becomes a genuinely new runtime abstraction.

The technically fascinating next problem would be defining the **safe deterministic compaction algebra** for each tool-result type: which events supersede others, which can be coalesced, which depend on workspace versions, and when a full checkpoint must be reintroduced. That connects almost exactly to the `file → diff → diff → diff` idea you started with.

[1]: https://github.com/Hypabolic/Hypa/blob/main/docs/architecture/compression-pipeline.md "Hypa/docs/architecture/compression-pipeline.md at main · Hypabolic/Hypa · GitHub"
[2]: https://github.com/Hypabolic/Hypa/tree/main/docs/architecture "Hypa/docs/architecture at main · Hypabolic/Hypa · GitHub"
[3]: https://github.com/Hypabolic/Hypa/tree/main/packages/pi-hypa "Hypa/packages/pi-hypa at main · Hypabolic/Hypa · GitHub"
[4]: https://github.com/Hypabolic/Hypa/blob/main/docs/architecture/mcp-and-tools.md "Hypa/docs/architecture/mcp-and-tools.md at main · Hypabolic/Hypa · GitHub"

## Turn 2

### User

"You could maintain an external event store: and derive a compact projection:" clarify the idea of this to me: this is for tool calls compaction? An example use case of this is starting a new session, then giving the agent this diff rather than the current state of the file? I am lost on this point.

### Agent

The key point I blurred is this:

> The “compact projection” is **not** necessarily something you give to a brand-new agent instead of the current file.

It is mainly a way to compact the **history of tool interactions** inside a running session.

Suppose the agent does this:

```text
1. read foo.ts        -> sees V0
2. patch A            -> file becomes V1
3. patch B            -> file becomes V2
4. read foo.ts again  -> sees V2
5. grep Foo           -> gets result based on V2
6. patch C            -> file becomes V3
```

A normal transcript may retain all of that:

```text
[file V0]
[patch A]
[success]
[patch B]
[success]
[file V2]
[grep result]
[patch C]
[success]
```

My proposed projection was saying:

```text
Instead of keeping all historical tool results active forever,
we can record externally that:

foo.ts
- agent originally saw V0
- current workspace state is V3
- changes A/B/C happened
- some old observations are stale
```

Then, if you control context assembly, you might eventually replace the noisy history with something like:

```text
foo.ts is currently at hash abc123.
You previously inspected this file.
It has since been modified in these regions: ...
Current full file is available if needed.
```

That is **tool-history compaction**.

---

#### It is not primarily this

Starting a fresh session:

```text
Here is the diff from V0 to V3.
Good luck.
```

That would often be bad because the new agent never saw `V0`.

A diff only makes sense relative to a baseline.

For a fresh session, usually the best source of truth is simply:

```text
read current foo.ts V3
```

not:

```text
read old V0
+ replay patches A
+ replay patches B
+ replay patches C
```

So:

```text
Existing session:
V0 + A + B + C
can sometimes be compacted into
"current state is V3 + small summary of how we got here"
```

Whereas:

```text
New session:
just start from V3
```

unless the history itself matters.

---

#### Where the diff idea actually helps

Imagine the agent already saw this 20,000-token file:

```text
foo.ts V0
```

Then another tool modifies 30 lines.

If the agent asks to read `foo.ts` again, returning the full 20,000-token V1 is wasteful.

Hypa could know:

```text
agent has seen:
foo.ts @ hash AAA

current file:
foo.ts @ hash BBB
```

Then mechanically compute:

```diff
@@ function processUser @@
- old code
+ new code
```

and return:

```text
Since your last read, foo.ts changed here:

[diff]

The rest of the file is unchanged.
```

That is the strongest version of the idea.

The diff is useful because the **same agent already has the baseline in context**.

```text
Agent context:
V0

Tool result:
V0 → V1 delta
```

Now the agent can reconstruct V1 cheaply.

For a new session:

```text
Agent context:
nothing

Tool result:
V0 → V1 delta
```

That is insufficient because V0 is missing.

So the distinction is:

| Situation | Good representation |
| --- | --- |
| Agent has never seen file | Current file |
| Agent saw previous version | Diff since previous version |
| Agent made the patch itself | Possibly just patch success |
| Many old reads/patches clutter context | Compact them into current-state metadata/checkpoint |
| New session | Current state + relevant task history |

The external event store I mentioned is therefore mostly there so Hypa can answer:

> **What has this particular agent already seen, and what is the cheapest correct thing to show it now?**

For example:

```text
Agent reads V0
        ↓
Hypa remembers: agent knows hash AAA

File changes to V1
        ↓
Agent reads again

Hypa compares:
known hash AAA
current hash BBB

        ↓

return delta AAA → BBB
instead of full V1
```

That, to me, is much more directly relevant to Hypa than giving a new session a chain of historical diffs.

The broader compaction idea only comes later: if you control the conversation history itself, you can also remove old tool-call clutter and replace it with a checkpoint saying, essentially, **“the current workspace is here; these are the few historical facts still relevant.”**
