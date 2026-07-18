# Proposed product specification direction

## Product one-liner

> **A repository-state runtime that gives coding agents only the context they need and surfaces evidence of silent execution failures—without changing how developers prompt or work.**

## Primary audience

**Hackathon audience:** developer-tooling and agent-platform engineers evaluating whether this runtime belongs inside an agentic development environment.
**End user:** developers already using MCP-capable coding agents who want lower context consumption and greater confidence in autonomous work.

## Core user journey

1. The developer connects the runtime and continues using their agent normally.
2. The agent reads and writes through familiar tools.
3. The ledger tracks what the agent observed and how the repository subsequently changed.
4. Responses contain the minimum correct context: full state, unchanged status, or changes from a known baseline.
5. The runtime surfaces evidence when tool activity suggests stale work, unresolved failure, or ineffective repetition.
6. The developer inspects savings and evidence in the observability UI.

No planning schema, special prompt syntax, architecture diagram, or new communication modality is introduced.

1. **Shared backbone:** the demo presents one runtime and one MCP tooling 
2. **Token UI:** Show raw eligible content, content delivered to the agent, savings, and successful task continuity.
3. **Soft warnings:** Correct framing. Evidence first, confidence second, failure label last.
4. **Realistic fixtures:** Essential; proposed below.
5. **Seven-failure taxonomy:** Good as an internal soft evaluation framework to point the user to where they need to be looking without relying on LLM calls and subagents, not a product menu.
6. **MCP portability:** Credible as an architectural direction. Pitch it as **MCP-native and agent-agnostic**, demonstrated deeply through one agent.


**core requirements**: MCP server, Rust based.

---

# MVP feature manifold

## Feature 1: Transparent agent tool gateway

**Requirement:** Existing agent workflows can route repository operations through the product without changing how developers describe tasks.

**Success criteria:**

- One real coding agent completes a realistic task through the gateway.
- Familiar read/write behavior remains recognizable.
- Developers do not need to author structured contracts or special prompts.

**Antirequisites:**

- No custom agent loop.
- No proprietary prompting workflow.
- No claim of supporting hosts that cannot route tools through MCP.
- No multi-agent orchestration features.

---

## Feature 2: Repository state ledger

**Requirement:** Maintain a durable account of repository state, agent observations, external changes, and tool outcomes throughout a run.

**Success criteria:**

- The runtime knows whether the agent has seen the current version of a file.
- Human and external edits are reflected in the ledger.
- Every optimized response and warning can cite its supporting state history.
- Original evidence remains inspectable.

**Antirequisites:**

- No attempt to own or rewrite the full agent transcript.
- No generalized semantic model of the entire codebase.
- No production-scale collaboration or distributed-state requirements for the hackathon.

---

## Feature 3: Context-efficient repository responses

**Requirement:** Return the smallest representation that is correct relative to what the agent has already observed.

**Required behavior:**

- Unseen state returns complete usable context.
- Unchanged state is not resent.
- Changed state returns only relevant changes when the agent has the required baseline.
- Missing or uncertain baselines fall back to complete context.
- Raw and delivered token consumption are measured separately.

**Success criteria:**

- The demo shows a meaningful reduction in delivered context.
- The agent continues correctly using the reduced representation.
- Savings never depend on lossy LLM summarization.

**Antirequisites:**

- No delta without a confirmed baseline.
- No token-savings claim without task-continuity evidence.
- No optimization that conceals information required for correctness.

---

## Feature 4: Evidence-based execution signals

**Requirement:** Surface possible silent failures derived from observable repository and tool history.

Each signal should include:

- what was observed;
- when it happened;
- why it may matter;
- the supporting evidence;
- confidence or severity;
- whether later activity resolved it.

### MVP signal families

1. **Stale-state risk**  
   Relevant repository state changed after the agent observed it.

2. **External-change conflict**  
   A human or another process changed a file while the agent was working from an older state.

3. **Unresolved failure**  
   A failed test, build, or relevant operation has no subsequent successful evidence.

4. **Repeated ineffective action**  
   Substantially identical activity repeats without a relevant state change or improved outcome.

The seven silent-failure categories may classify these signals internally, but should not appear as seven promised detectors.

**Antirequisites:**

- No authoritative declaration that the agent failed.
- No generic hallucination or intent detection.
- No opaque warning without repository evidence.
- No claim that absence of a warning proves correctness.

---

## Feature 5: Demo observability UI

**Requirement:** Make the otherwise invisible runtime behavior immediately understandable to judges.

### Primary display

- chronological tool and repository-state timeline;
- original response versus response delivered to the agent;
- original and delivered token counts;
- cumulative reduction;
- ledger-backed warnings with evidence;
- whether warnings were later resolved.

**Success criteria:**

A tooling engineer should understand within 30 seconds:

1. what the agent requested;
2. what the runtime knew;
3. what the agent received;
4. what context was saved;
5. what suspicious behavior was identified.

**Antirequisites:**

- No general-purpose monitoring dashboard.
- No complex configuration interface.
- No speculative analytics.
- No UI feature that does not appear in the demo narrative.

---

# Realistic demo scenarios

## Scenario A: Repeated repository exploration

The agent rereads a large unchanged file and later rereads it after a small external edit.

Demonstrates:

- complete initial context;
- suppression of unchanged content;
- minimal changed context;
- continued task correctness;
- measurable context reduction.

## Scenario B: Human-agent concurrent edit

The agent reads a file, the developer changes it, and the agent later attempts work based on its prior observation.

Demonstrates:

- external-change tracking;
- stale-state evidence;
- safer human-agent collaboration;
- why the ledger is more than a token counter.

## Scenario C: Unresolved test failure

The agent runs a realistic test, receives a failure, makes changes, but finishes without obtaining successful verification.

Demonstrates:

- evidence persistence across tool calls;
- unresolved-failure warning;
- distinction between completed actions and completed work.

## Scenario D: Ineffective repetition

The agent repeats the same operation after receiving the same result, with no relevant intervening change.

Demonstrates:

- wasted tool activity;
- repeated-failure evidence;
- connection between context efficiency and execution integrity.

Stale specifications can appear naturally in these repositories, but should be supporting conditions—not obviously planted traps.

---

## Assessment: GTM message

The strongest positioning is not “seven failure detectors plus token optimization.” It is:

> **Your coding agent does not need more repository context—it needs the right state.**

Supporting pitch:

> Existing tools treat every read, write, and test as an isolated event. Our repository ledger remembers what the agent has already observed, returns only what changed, and preserves evidence when repository activity stops matching apparent progress.

### Hackathon proof points

- meaningful reduction in context delivered during a real task;
- no change to developer prompting behavior;
- one deeply working MCP integration;
- multiple warnings grounded in visible evidence;
- correct handling of external repository changes.

## Decisions needed before finalizing the spec

1. Will shell, test, and build results pass through the observable runtime, or only file reads and writes?
2. Is the MVP strictly observational, or may it pause a conflicting write?
3. Which one agent will be the primary demonstrated integration?
4. How much hackathon time remains?

The first answer determines whether unresolved integration failures belong in the MVP or must be cut.
