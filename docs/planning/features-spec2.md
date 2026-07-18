# Product Specification 2.0: AI Agent Compaction & Verification Runtime

## 1. Product Formulation & Pitch
**One-Liner:** An enterprise-grade, agent-agnostic repository runtime that reduces AI compute costs and surfaces silent execution failures—powered by Gemini and orchestrated via Backboard.

**The Problem:** Coding agents are token-inefficient (re-reading unchanged files) and difficult to supervise (silently failing, skipping work, or drifting out of scope).
**The Solution:** We have built a proxy layer and SaaS dashboard that intercepts agent tool calls, compacts context deterministically (The Green AI Engine), and runs a flight-recorder heuristic to catch agent failures before they commit.

### Target Hackathon Tracks:
*   **Warp (Best Dev Tool):** Improves the core developer experience by making coding agents cheaper, faster, and actually verifiable.
*   **Deloitte (Green AI):** The Context Compaction Engine prevents the Gemini API from needlessly processing millions of redundant tokens, directly reducing GPU compute cycles and carbon emissions.
*   **Base44 (Venture Builder):** Packaged as a B2B Enterprise SaaS dashboard that visualizes cost and compute savings for development teams.
*   **Backboard.io:** Uses Backboard as the core orchestration infrastructure to route tool calls.
*   **MLH (Gemini API):** Relies on Gemini 1.5 as the underlying intelligence powering the AI coding agent.

---

## 2. Core Architecture
No planning schema or special prompt syntax is required by the end-user developer.
*   **The Orchestrator (Backboard):** Handles the agent loop and tool-call routing.
*   **The Brain (Gemini API):** Interprets the user's intent and generates code modifications.
*   **The Proxy (Rust MCP Server):** Sits between Backboard and the file system. It intercepts reads/writes, calculates diffs, and evaluates execution signals.
*   **The UI (Base44):** A web dashboard that reads the proxy's ledger to display token savings, carbon offset, and failure warnings.

---

## 3. MVP Feature Manifold

### Feature 1: The Backboard-to-MCP Gateway
**Requirement:** Existing agent workflows must route repository operations through the product without changing how developers prompt.
**Success Criteria:**
*   Backboard successfully orchestrates a Gemini-powered agent to complete a realistic task through the Rust MCP gateway.
*   Familiar read/write behavior remains recognizable.
**Antirequisites:**
*   No proprietary prompting workflow.
*   No custom LLM models; strictly rely on Gemini API.

### Feature 2: "Green AI" Context Compaction
**Requirement:** Return the smallest representation of a file that is correct relative to what the agent has already observed to minimize compute energy.
**Required Behavior:**
*   **Unseen State:** Returns complete usable file context.
*   **Unchanged State:** Suppresses content and returns a cache hash.
*   **Changed State:** Returns only relevant diffs if the agent has the required baseline.
**Success Criteria:**
*   The demo shows a massive reduction in delivered context (tokens).
*   The system translates token reduction into estimated Compute/Carbon savings (Deloitte alignment).
*   The agent continues its task correctly without lossy LLM summarizations.

### Feature 3: Evidence-Based Execution Signals
**Requirement:** Surface possible silent failures derived from observable repository and tool history using Tree-sitter, AST, and exact-search pipelines.
**MVP Signal Families:**
1.  **Unresolved Failure:** A test or build failed, and the agent claimed completion without a subsequent successful retry.
2.  **Retry Loop:** The agent attempts the same action 3+ times with the exact same failure and no meaningful file changes.
3.  **Constraint Breach:** A user invariant (e.g., "Do not change public API") is violated according to an AST diff.
4.  **Scope Drift:** The agent modifies files with no dependency-graph or semantic proximity to the task.
**Antirequisites:**
*   No authoritative declarations that the agent failed; frame as "evidence of failure".
*   No generic hallucination detection (only verifiable claims).

### Feature 4: Base44 Enterprise Observability Dashboard
**Requirement:** A SaaS portal that visualizes runtime behavior, ROI, and Green AI metrics for enterprise teams.
**Primary Display:**
*   Chronological tool and repository-state timeline.
*   Original response vs. Response delivered to the agent (diff view).
*   **Green AI & ROI Widget:** Displays Cumulative Tokens Saved, Estimated API Costs Saved ($), and Compute Energy/Carbon emissions prevented.
*   Ledger-backed warnings with specific code-state evidence.
**Success Criteria:**
*   A judge understands within 30 seconds what the agent requested, what context was saved, what energy was saved, and what suspicious behavior was flagged.

---

## 4. Realistic Demo Scenarios (Pre-Recorded)

### Scenario A: The Green AI Optimization
*   **Action:** The Gemini agent reads a massive, unchanged 20,000 token file multiple times while exploring the repo.
*   **Demonstrates:** The proxy intercepts the duplicate reads, returning only the cache hash. The Base44 dashboard dynamically updates to show thousands of tokens saved and the corresponding carbon/cost reduction.

### Scenario B: The Retry Loop
*   **Action:** The agent runs a test file, it fails. The agent gets stuck and runs the exact same test command 3 times without modifying any source files.
*   **Demonstrates:** The execution monitor catches the repetition fingerprint and flags a "Probable Retry Loop (96% Confidence)" on the dashboard before the developer loses money.

### Scenario C: Unresolved Test Failure & Scope Drift
*   **Action:** The agent attempts to fix a bug in `render.ts`, but inexplicably edits `auth/session.ts` and breaks a test. The agent then claims "Done."
*   **Demonstrates:** The dashboard flags two things: 1) A "Scope Drift" warning indicating `auth/session.ts` has zero dependency relation to `render.ts`, and 2) an "Unresolved Failure" showing the agent lied about completion because the test suite is currently red.