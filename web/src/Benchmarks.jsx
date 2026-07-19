import { useMemo, useState } from "react";
import benchmarkData from "./data/benchmark-results.json";

const TOOL_ORDER = ["read", "bash", "edit", "write"];

function fmt(value) {
  return Number(value).toLocaleString();
}

function median(values) {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2
    ? sorted[middle]
    : (sorted[middle - 1] + sorted[middle]) / 2;
}

function summarizeRun(run) {
  const snapshot = run.snapshot;
  const calls = snapshot?.toolCalls ?? [];
  const compactCounts = run.toolCounts ?? {};
  const toolCounts = Object.fromEntries(
    TOOL_ORDER.map((tool) => [
      tool,
      compactCounts[tool] ?? calls.filter((call) => call.toolName === tool).length,
    ]),
  );

  return {
    resolved: Boolean(run.resolved),
    toolCalls: run.toolCalls ?? calls.length,
    toolContextTokens:
      run.toolContextTokens ?? snapshot?.totals?.withRuntimeTokens ?? 0,
    toolCounts,
  };
}

function normalizeInstances(instances) {
  return instances.map((instance) => ({
    id: instance.id,
    baseline: summarizeRun(instance.baseline),
    mcp: summarizeRun(instance.mcp),
  }));
}

function Outcome({ resolved }) {
  return (
    <span className={`benchmark-outcome ${resolved ? "resolved" : "unresolved"}`}>
      {resolved ? "Resolved" : "Unresolved"}
    </span>
  );
}

function RunDetail({ label, run, accent = false }) {
  return (
    <article className={`benchmark-run-detail ${accent ? "accent" : ""}`}>
      <div className="benchmark-run-head">
        <span className="mono">{label}</span>
        <Outcome resolved={run.resolved} />
      </div>
      <div className="benchmark-run-numbers">
        <div>
          <strong>{run.toolCalls}</strong>
          <span>tool calls</span>
        </div>
        <div>
          <strong>{fmt(run.toolContextTokens)}</strong>
          <span>tool-context tokens</span>
        </div>
      </div>
      <div className="benchmark-tool-mix" aria-label={`${label} tool-call breakdown`}>
        {TOOL_ORDER.map((tool) => (
          <span key={tool}>
            {tool} <b>{run.toolCounts[tool]}</b>
          </span>
        ))}
      </div>
    </article>
  );
}

export default function Benchmarks() {
  const instances = useMemo(
    () => normalizeInstances(benchmarkData.instances ?? []),
    [],
  );
  const [selectedId, setSelectedId] = useState(instances[0]?.id ?? null);
  const selected =
    instances.find((instance) => instance.id === selectedId) ?? instances[0];

  const summary = useMemo(() => {
    const baselineResolved = instances.filter(
      (instance) => instance.baseline.resolved,
    ).length;
    const mcpResolved = instances.filter((instance) => instance.mcp.resolved).length;
    const agreement = instances.filter(
      (instance) => instance.baseline.resolved === instance.mcp.resolved,
    ).length;

    return {
      agreement,
      baselineMedianCalls: median(
        instances.map((instance) => instance.baseline.toolCalls),
      ),
      baselineResolved,
      mcpMedianCalls: median(instances.map((instance) => instance.mcp.toolCalls)),
      mcpResolved,
      resolvedDelta: mcpResolved - baselineResolved,
      total: instances.length,
    };
  }, [instances]);

  return (
    <div className="dash benchmark-page">
      <header className="dash-top">
        <div className="dash-left">
          <a className="brand" href="#/" title="Back to home">
            <span className="mark">✳</span> Loop-Whole
          </a>
          <nav className="tabs" aria-label="Dashboard views">
            <a className="tab" href="#/">
              Home
            </a>
            <a className="tab active" href="#/benchmarks" aria-current="page">
              Benchmark
            </a>
          </nav>
        </div>
        <div className="benchmark-count">
          <div className="count">{summary.total}</div>
          <div className="mono">evaluated tasks</div>
        </div>
      </header>

      <main className="benchmark-main">
        <section className="benchmark-hero">
          <div className="benchmark-copy">
            {!benchmarkData.mock && (
              <span className="mono benchmark-eyebrow">
                {benchmarkData.benchmark.name} · non-regression
              </span>
            )}
            <h1>
              {summary.resolvedDelta >= 0
                ? "Task completion held."
                : "Task completion declined."}
              <em>Less overhead.</em>
            </h1>
            <p>
              The tool replay demonstrates context efficiency. {benchmarkData.benchmark.name}
              checks how aggregate coding performance compares under matched conditions.
            </p>
          </div>

          <div className="benchmark-score" aria-label="Loop-Whole tasks resolved">
            <div className="benchmark-score-number">
              <strong>{summary.mcpResolved}</strong>
              <span>/{summary.total}</span>
            </div>
            <p>SWE-bench tasks resolved with Loop-Whole</p>
            <div className="benchmark-score-baseline">
              <span>Baseline</span>
              <b>
                {summary.baselineResolved}/{summary.total}
              </b>
              <span>
                {summary.resolvedDelta === 0
                  ? "Matched"
                  : `${summary.resolvedDelta > 0 ? "+" : ""}${summary.resolvedDelta}`}
              </span>
            </div>
          </div>
        </section>

        <section className="benchmark-proof" aria-label="Benchmark summary">
          <article>
            <span className="mono">Resolved-task delta</span>
            <strong>
              {summary.resolvedDelta > 0 ? "+" : ""}
              {summary.resolvedDelta}
            </strong>
            <p>
              {summary.resolvedDelta === 0
                ? "No aggregate coding-performance loss in this evaluated set."
                : summary.resolvedDelta > 0
                  ? "More tasks resolved with Loop-Whole in this evaluated set."
                  : "Fewer tasks resolved with Loop-Whole in this evaluated set."}
            </p>
          </article>
          <article>
            <span className="mono">Outcome agreement</span>
            <strong>
              {summary.agreement}/{summary.total}
            </strong>
            <p>Tasks where baseline and Loop-Whole reached the same result.</p>
          </article>
          <article>
            <span className="mono">Median tool calls</span>
            <strong>
              {summary.baselineMedianCalls} <i>→</i> {summary.mcpMedianCalls}
            </strong>
            <p>Baseline versus Loop-Whole trajectories under matched conditions.</p>
          </article>
        </section>

        <section className="benchmark-ledger-section">
          <div className="benchmark-section-head">
            <div>
              <span className="mono">Paired evidence</span>
              <h2>Task ledger</h2>
            </div>
            <p>Select an instance to inspect outcomes and tool-call shape.</p>
          </div>

          <div className="benchmark-ledger">
            {instances.map((instance, index) => (
              <button
                className={`benchmark-task ${selected?.id === instance.id ? "active" : ""}`}
                key={instance.id}
                onClick={() => setSelectedId(instance.id)}
                type="button"
                aria-pressed={selected?.id === instance.id}
              >
                <span className="benchmark-task-index">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <strong>{instance.id}</strong>
                <span className="benchmark-task-results">
                  <span>
                    Base <Outcome resolved={instance.baseline.resolved} />
                  </span>
                  <span>
                    MCP <Outcome resolved={instance.mcp.resolved} />
                  </span>
                </span>
                <span className="benchmark-task-calls">
                  {instance.baseline.toolCalls} → {instance.mcp.toolCalls} calls
                </span>
              </button>
            ))}
          </div>

          {selected && (
            <div className="benchmark-inspector" aria-live="polite">
              <div className="benchmark-inspector-head">
                <div>
                  <span className="mono">Selected instance</span>
                  <h3>{selected.id}</h3>
                </div>
                <span className="benchmark-inspector-delta">
                  {selected.mcp.toolCalls - selected.baseline.toolCalls > 0 ? "+" : ""}
                  {selected.mcp.toolCalls - selected.baseline.toolCalls} tool calls
                </span>
              </div>
              <div className="benchmark-run-grid">
                <RunDetail label="Baseline" run={selected.baseline} />
                <RunDetail label="Loop-Whole MCP" run={selected.mcp} accent />
              </div>
            </div>
          )}
        </section>
      </main>
    </div>
  );
}
