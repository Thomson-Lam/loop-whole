import { useEffect, useMemo, useState } from "react";
import useLiveSession from "./useLiveSession";

const MODE_LABEL = {
  full: "FULL",
  unchanged: "UNCHANGED",
  diff: "DIFF",
  passthrough: "PASSTHROUGH",
  compressed: "COMPRESSED",
  error: "ERROR",
};

function fmt(n) {
  return Number(n).toLocaleString();
}

export default function Dashboard() {
  const { session, error } = useLiveSession();
  const calls = useMemo(
    () =>
      [...(session?.toolCalls ?? [])].sort(
        (a, b) => a.sequence - b.sequence
      ),
    [session]
  );

  const [index, setIndex] = useState(0);
  const [showContext, setShowContext] = useState(false);

  const call = calls[index];

  const prev = () => setIndex((i) => Math.max(0, i - 1));
  const next = () =>
    setIndex((i) => Math.max(0, Math.min(calls.length - 1, i + 1)));

  useEffect(() => {
    const onKey = (e) => {
      if (e.key === "ArrowLeft") prev();
      else if (e.key === "ArrowRight") next();
      else if (e.key === "c" || e.key === "C") setShowContext((s) => !s);
      else if (e.key === "Escape") setShowContext(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [calls.length]);

  useEffect(() => {
    setIndex((i) => Math.max(0, Math.min(i, calls.length - 1)));
  }, [calls.length]);

  if (!session) {
    return (
      <div className="dash pane-body">
        {error ? `API unavailable: ${error.message}` : "Loading session…"}
      </div>
    );
  }
  if (!call) {
    return <div className="dash pane-body">Waiting for tool calls…</div>;
  }

  const totals = session.totals;
  const meta = session.session;
  const origTok = call.original.tokens;
  const intTok = call.intercepted.tokens;
  const savedTok = origTok - intTok;
  const savedPct = origTok > 0 ? Math.round((savedTok / origTok) * 100) : 0;

  const ctx = meta.contextWindowTokens || 0;
  const withoutPct = ctx ? (totals.withoutRuntimeTokens / ctx) * 100 : 0;
  const withPct = ctx ? (totals.withRuntimeTokens / ctx) * 100 : 0;

  return (
    <div className="dash">
      <header className="dash-top">
        <div className="dash-left">
          <a className="brand" href="#/" title="Back to home">
            <span className="mark">✳</span> Loopey
          </a>
          <div className="tabs">
            <button className="tab active">Token</button>
            <button className="tab disabled" disabled title="Coming soon">
              Silent Failures
            </button>
          </div>
        </div>
        <div className="savings">
          <div className="pct">{totals.savingsPercent}%</div>
          <div className="saved mono">{fmt(totals.savedTokens)} tokens saved</div>
        </div>
      </header>

      <div className="callbar">
        <div className="callbar-left">
          <span className={`badge badge-${call.deliveryMode}`}>
            {MODE_LABEL[call.deliveryMode] || call.deliveryMode.toUpperCase()}
          </span>
          <span className="tool mono">{call.toolName}</span>
          <span className="subject">{call.subjectPath || "—"}</span>
        </div>
        <div className="callbar-right mono">
          Call {index + 1} / {calls.length}
        </div>
      </div>

      <div className="split">
        <section className="pane">
          <div className="pane-head">
            <span className="mono">Original (agent → tool)</span>
            <span className="tok">{fmt(origTok)} tok</span>
          </div>
          <pre className="pane-body">{call.original.text}</pre>
        </section>

        <section className="pane">
          <div className="pane-head">
            <span className="mono">Intercepted (Loopey runtime)</span>
            <span className="tok">
              {fmt(intTok)} tok
              {savedTok > 0 && (
                <span className="reduction">
                  {" "}
                  −{fmt(savedTok)} ({savedPct}%)
                </span>
              )}
            </span>
          </div>
          <pre className="pane-body">{call.intercepted.text}</pre>
        </section>
      </div>

      <div className="dash-foot">
        <span className="hint mono">
          Press <kbd>C</kbd> for context window · <kbd>◀</kbd> <kbd>▶</kbd> to
          cycle
        </span>
        <div className="arrows">
          <button className="arrow-btn" onClick={prev} disabled={index === 0}>
            ◀
          </button>
          <button
            className="arrow-btn"
            onClick={next}
            disabled={index === calls.length - 1}
          >
            ▶
          </button>
        </div>
      </div>

      {showContext && (
        <div className="overlay" onClick={() => setShowContext(false)}>
          <div className="popup" onClick={(e) => e.stopPropagation()}>
            <div className="popup-head">
              <span className="mono">Context window usage</span>
              <button className="x" onClick={() => setShowContext(false)}>
                ✕
              </button>
            </div>
            <p className="popup-sub mono">
              Model window: {fmt(ctx)} tokens · tool calls &amp; file reads only
            </p>

            <div className="ctx-row">
              <div className="ctx-label mono">Without Loopey</div>
              <div className="ctx-track">
                <div
                  className="ctx-fill without"
                  style={{ width: `${Math.max(withoutPct, 1.5)}%` }}
                />
              </div>
              <div className="ctx-val mono">
                {fmt(totals.withoutRuntimeTokens)} · {withoutPct.toFixed(2)}%
              </div>
            </div>

            <div className="ctx-row">
              <div className="ctx-label mono">With Loopey</div>
              <div className="ctx-track">
                <div
                  className="ctx-fill with"
                  style={{ width: `${Math.max(withPct, 1.5)}%` }}
                />
              </div>
              <div className="ctx-val mono">
                {fmt(totals.withRuntimeTokens)} · {withPct.toFixed(2)}%
              </div>
            </div>

            <div className="popup-foot">
              <span className="reduction">
                {fmt(totals.savedTokens)} tokens saved ({totals.savingsPercent}%)
              </span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
