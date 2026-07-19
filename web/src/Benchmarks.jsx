import { useMemo } from "react";
import {
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Tooltip,
} from "chart.js";
import { Bar } from "react-chartjs-2";

ChartJS.register(CategoryScale, LinearScale, BarElement, Tooltip, Legend);

const benchmarkModules = import.meta.glob("../benchmarks/*.json", {
  eager: true,
  import: "default",
});

function formatTokens(value) {
  return Number(value).toLocaleString();
}

function runIdFromSnapshot(snapshot) {
  for (const call of snapshot?.toolCalls ?? []) {
    const match = call.subjectPath?.match(/\.swebench_codex\/runs\/([^/]+)\/repo/);
    if (match) return match[1];
  }
  return null;
}

function canonicalTaskId(runId) {
  return runId?.match(/^(.+__.+-\d+)-[^/]+$/)?.[1] ?? runId;
}

function loadBenchmarks() {
  const invalidFiles = [];
  const runs = [];

  for (const [file, snapshot] of Object.entries(benchmarkModules)) {
    const withoutMcp = snapshot?.totals?.withoutRuntimeTokens;
    const withMcp = snapshot?.totals?.withRuntimeTokens;
    const runId = runIdFromSnapshot(snapshot);

    if (
      !runId ||
      !Number.isFinite(withoutMcp) ||
      !Number.isFinite(withMcp)
    ) {
      invalidFiles.push(file.split("/").pop());
      continue;
    }

    runs.push({
      canonicalId: canonicalTaskId(runId),
      file,
      runId,
      sessionId: snapshot.session?.id ?? file.split("/").pop().replace(/\.json$/, ""),
      withMcp,
      withoutMcp,
    });
  }

  runs.sort((a, b) =>
    a.canonicalId.localeCompare(b.canonicalId) ||
    a.sessionId.localeCompare(b.sessionId)
  );

  const counts = new Map();
  for (const run of runs) {
    counts.set(run.canonicalId, (counts.get(run.canonicalId) ?? 0) + 1);
  }

  return {
    invalidFiles,
    runs: runs.map((run) => ({
      ...run,
      label:
        counts.get(run.canonicalId) > 1
          ? `${run.canonicalId} · ${run.sessionId.slice(0, 8)}`
          : run.canonicalId,
    })),
  };
}

export default function Benchmarks() {
  const { invalidFiles, runs } = useMemo(loadBenchmarks, []);
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const data = useMemo(
    () => ({
      labels: runs.map((run) => run.label),
      datasets: [
        {
          label: "Without MCP",
          data: runs.map((run) => run.withoutMcp),
          backgroundColor: "rgba(139, 139, 147, 0.55)",
          borderColor: "#8b8b93",
          borderWidth: 1,
        },
        {
          label: "With MCP",
          data: runs.map((run) => run.withMcp),
          backgroundColor: "rgba(234, 255, 0, 0.78)",
          borderColor: "#eaff00",
          borderWidth: 1,
        },
      ],
    }),
    [runs]
  );

  const options = useMemo(
    () => ({
      animation: reduceMotion ? false : undefined,
      maintainAspectRatio: false,
      responsive: true,
      interaction: {
        intersect: false,
        mode: "index",
      },
      plugins: {
        legend: {
          align: "end",
          labels: {
            boxHeight: 10,
            boxWidth: 10,
            color: "#cfcfd4",
            font: { family: "JetBrains Mono", size: 11 },
            padding: 20,
          },
        },
        tooltip: {
          backgroundColor: "#0e0e10",
          borderColor: "rgba(255, 255, 255, 0.16)",
          borderWidth: 1,
          bodyColor: "#f7f7f8",
          callbacks: {
            label: (context) =>
              ` ${context.dataset.label}: ${formatTokens(context.parsed.y)} tokens`,
          },
          displayColors: true,
          padding: 12,
          titleColor: "#eaff00",
        },
      },
      scales: {
        x: {
          grid: { display: false },
          ticks: {
            autoSkip: false,
            color: "#8b8b93",
            font: { family: "JetBrains Mono", size: 10 },
            maxRotation: 35,
            minRotation: 0,
          },
        },
        y: {
          beginAtZero: true,
          border: { display: false },
          grid: { color: "rgba(255, 255, 255, 0.08)" },
          ticks: {
            color: "#8b8b93",
            font: { family: "JetBrains Mono", size: 10 },
            callback: (value) => formatTokens(value),
          },
          title: {
            color: "#8b8b93",
            display: true,
            font: { family: "JetBrains Mono", size: 10 },
            text: "TOOL-CONTEXT TOKENS",
          },
        },
      },
    }),
    [reduceMotion]
  );

  return (
    <div className="dash benchmark-page">
      <header className="dash-top">
        <div className="dash-left">
          <a className="brand" href="#/" title="Back to home">
            <span className="mark">✳</span> Loop-Whole
          </a>
          <nav className="tabs" aria-label="Dashboard views">
            <a className="tab" href="#/app">
              Token
            </a>
            <a className="tab active" href="#/benchmarks" aria-current="page">
              Benchmark
            </a>
            <button className="tab disabled" disabled title="Coming soon">
              Silent Failures
            </button>
          </nav>
        </div>
        <div className="benchmark-count">
          <div className="count">{runs.length}</div>
          <div className="mono">benchmark {runs.length === 1 ? "run" : "runs"}</div>
        </div>
      </header>

      <main className="benchmark-main">
        <div className="benchmark-heading">
          <div>
            <span className="mono">SWE-Bench · token comparison</span>
            <h1>Runtime benchmark</h1>
          </div>
          <p>
            Tool-context tokens delivered to agents with the Loop-Whole MCP
            runtime versus the projected baseline without it.
          </p>
        </div>

        {runs.length > 0 ? (
          <section className="benchmark-panel" aria-labelledby="chart-title">
            <div className="benchmark-panel-head">
              <div>
                <h2 id="chart-title">Tokens by instance</h2>
                <p>Hover the chart to inspect exact token totals.</p>
              </div>
              {invalidFiles.length > 0 && (
                <span className="benchmark-warning">
                  {invalidFiles.length} invalid {invalidFiles.length === 1 ? "file" : "files"} skipped
                </span>
              )}
            </div>

            <div className="benchmark-chart-scroll">
              <div
                className="benchmark-chart"
                style={{ minWidth: `${Math.max(760, runs.length * 170)}px` }}
              >
                <Bar
                  data={data}
                  options={options}
                  role="img"
                  aria-label="Grouped bar chart comparing tokens with and without the MCP runtime for each SWE-Bench run"
                />
              </div>
            </div>

            <table className="sr-only">
              <caption>Exact SWE-Bench tool-context token totals</caption>
              <thead>
                <tr>
                  <th>Instance</th>
                  <th>Without MCP</th>
                  <th>With MCP</th>
                </tr>
              </thead>
              <tbody>
                {runs.map((run) => (
                  <tr key={run.file}>
                    <th>{run.label}</th>
                    <td>{run.withoutMcp}</td>
                    <td>{run.withMcp}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        ) : (
          <section className="benchmark-empty">
            <span className="mono">No benchmark data</span>
            <h2>Add a benchmark JSON file to begin.</h2>
            <p>
              Expected files in <code>benchmarks/</code> with runtime token
              totals and a SWE-Bench run path.
            </p>
            {invalidFiles.length > 0 && (
              <p className="benchmark-warning">
                {invalidFiles.length} existing {invalidFiles.length === 1 ? "file is" : "files are"} missing required data.
              </p>
            )}
          </section>
        )}
      </main>
    </div>
  );
}
