import { useEffect, useMemo, useRef, useState } from "react";
import session from "./data/demo-session.json";

const CALL_MS = 2800;

const TOOL_ORDER = ["read", "edit", "write", "bash"];

const TOOL_META = {
  read: {
    label: "Read",
    icon: "▤",
    statement: "Return only what changed — or say nothing changed.",
    note: "Read remembers the exact view it delivered. Re-reads collapse to a marker or a minimal diff; a first, unseen view is always sent in full.",
  },
  edit: {
    label: "Edit",
    icon: "✎",
    statement: "Replace one exact match without rewriting the whole file.",
    note: "Edit changes one proven location. Its confirmation is already tiny, so it saves no output tokens here — the payoff appears on the next read of the file.",
  },
  write: {
    label: "Write",
    icon: "＋",
    statement: "Create safely; never silently overwrite existing work.",
    note: "Write is create-only. The confirmation is identical on both sides — its value is safety, not token savings.",
  },
  bash: {
    label: "Bash",
    icon: "»_",
    statement: "Execute again, then return only the relevant output changes.",
    note: "Bash always executes — it is never cached or skipped. Loop-Whole canonicalizes the output, then compares it with the previous run.",
  },
};

const MODE_LABEL = {
  full: "FULL",
  unchanged: "UNCHANGED",
  diff: "DIFF",
  passthrough: "PASSTHROUGH",
  compressed: "COMPRESSED",
  error: "ERROR",
};

// Per-call, mode-accurate caption. Kept honest: never claims savings that the
// backend does not produce (write/edit confirmations are identical both sides).
function captionFor(call) {
  const { toolName, deliveryMode } = call;
  if (deliveryMode === "full")
    return "First time seen — full result delivered, and a baseline is stored for next time.";
  if (deliveryMode === "unchanged" && toolName === "bash")
    return "Command ran again and returned the identical result — only a short marker is delivered.";
  if (deliveryMode === "unchanged")
    return "Already delivered identically — Loop-Whole returns a one-line marker instead of re-sending the file.";
  if (deliveryMode === "diff")
    return "Changed since the stored baseline — delivered as a minimal diff of just the changed lines.";
  if (deliveryMode === "compressed")
    return "First run of this command — noisy output is projected to its canonical result.";
  if (deliveryMode === "passthrough" && toolName === "edit")
    return "One exact replacement applied — confirmation passes through unchanged; the diff shows up on the next read.";
  if (deliveryMode === "passthrough")
    return "Create-only — confirmation passes through unchanged. Safety, not token savings.";
  return "";
}

function classifyLine(line) {
  if (line.startsWith("@@")) return "rl-hunk";
  if (line.startsWith("[loop-whole]")) return "rl-loop-whole";
  if (line.startsWith("+")) return "rl-add";
  if (line.startsWith("-")) return "rl-del";
  return "rl-ctx";
}

function usePrefersReducedMotion() {
  const [reduced, setReduced] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setReduced(mq.matches);
    update();
    mq.addEventListener("change", update);
    return () => mq.removeEventListener("change", update);
  }, []);
  return reduced;
}

// rAF interpolation toward `target`. Skips animation when reduced-motion is on
// or before the section has started, so the value is always readable.
function useAnimatedNumber(target, active, reduced, duration = 650) {
  const [val, setVal] = useState(target);
  const fromRef = useRef(target);
  const rafRef = useRef(0);
  useEffect(() => {
    if (reduced || !active) {
      setVal(target);
      fromRef.current = target;
      return;
    }
    const from = fromRef.current;
    const start = performance.now();
    cancelAnimationFrame(rafRef.current);
    const tick = (now) => {
      const t = Math.min(1, (now - start) / duration);
      const eased = 1 - Math.pow(1 - t, 3);
      setVal(from + (target - from) * eased);
      if (t < 1) rafRef.current = requestAnimationFrame(tick);
      else fromRef.current = target;
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [target, active, reduced, duration]);
  return val;
}

export default function ToolReplay() {
  const calls = useMemo(
    () => [...session.toolCalls].sort((a, b) => a.sequence - b.sequence),
    []
  );

  const reduced = usePrefersReducedMotion();
  const [index, setIndex] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [started, setStarted] = useState(false);
  const sectionRef = useRef(null);

  // Autoplay once when the section scrolls into view.
  useEffect(() => {
    const el = sectionRef.current;
    if (!el) return;
    const io = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            setStarted(true);
            if (!reduced) setPlaying(true);
            io.unobserve(e.target);
          }
        });
      },
      { threshold: 0.35 }
    );
    io.observe(el);
    return () => io.disconnect();
  }, [reduced]);

  // Advance the tape while playing; stop on the final frame.
  useEffect(() => {
    if (!playing || reduced) return;
    if (index >= calls.length - 1) {
      setPlaying(false);
      return;
    }
    const id = setTimeout(
      () => setIndex((i) => Math.min(calls.length - 1, i + 1)),
      CALL_MS
    );
    return () => clearTimeout(id);
  }, [playing, index, calls.length, reduced]);

  const call = calls[index];
  const origTok = call.original.tokens;
  const intTok = call.intercepted.tokens;
  const savedTok = origTok - intTok;
  const savedPct = origTok > 0 ? Math.round((savedTok / origTok) * 100) : 0;
  const keptRatio = origTok > 0 ? Math.max(intTok / origTok, 0.03) : 1;

  // Cumulative tool-output savings through the current call (output tokens only,
  // matching original.tokens - intercepted.tokens; labelled as such in the UI).
  const cum = useMemo(() => {
    let o = 0;
    let it = 0;
    for (let i = 0; i <= index; i++) {
      o += calls[i].original.tokens;
      it += calls[i].intercepted.tokens;
    }
    return o > 0 ? ((o - it) / o) * 100 : 0;
  }, [index, calls]);

  const animInt = useAnimatedNumber(intTok, started, reduced);
  const animCum = useAnimatedNumber(cum, started, reduced);

  const atStart = index === 0;
  const atEnd = index === calls.length - 1;

  const go = (i) => {
    setIndex(Math.max(0, Math.min(calls.length - 1, i)));
  };
  const restart = () => {
    setIndex(0);
    if (!reduced) setPlaying(true);
  };
  const jumpTo = (tool) => {
    const i = calls.findIndex((c) => c.toolName === tool);
    if (i >= 0) {
      setPlaying(false);
      setIndex(i);
    }
  };

  const origLines = call.original.text.replace(/\n$/, "").split("\n");
  const intLines = call.intercepted.text.replace(/\n$/, "").split("\n");

  return (
    <section className="replay" id="replay" ref={sectionRef}>
      <div className="wrap">
        <div className="section-head reveal in">
          <span className="mono kicker">Context-aware tools</span>
          <h2>Your tools remember what the agent has already seen.</h2>
          <p>
            Loop-Whole runs the real operation, keeps the original evidence, and
            sends the smallest safe result back to the model. Watch a real
            session replay — <b>left is what the tool returned</b>,{" "}
            <b>right is what the model received</b>.
          </p>
        </div>

        <div
          className="replay-tabs"
          role="tablist"
          aria-label="Tool demonstrations"
        >
          {TOOL_ORDER.map((tool) => {
            const m = TOOL_META[tool];
            if (!m) return null;
            const active = call.toolName === tool;
            return (
              <button
                key={tool}
                role="tab"
                aria-selected={active}
                className={`replay-tab${active ? " active" : ""}`}
                onClick={() => jumpTo(tool)}
              >
                <span className="replay-tab-top">
                  <span className="replay-tab-icon mono" aria-hidden="true">
                    {m.icon}
                  </span>
                  <span className="replay-tab-name">{m.label}</span>
                </span>
                <span className="replay-tab-stmt">{m.statement}</span>
              </button>
            );
          })}
        </div>

        <div className="replay-stage">
          <div className="replay-bar">
            <div className="replay-bar-left">
              <span className={`badge badge-${call.deliveryMode}`}>
                {MODE_LABEL[call.deliveryMode] ||
                  call.deliveryMode.toUpperCase()}
              </span>
              <span className="replay-tool mono">{call.toolName}</span>
              <span className="replay-subject mono">
                {call.subjectPath || "—"}
              </span>
            </div>
            <div className="replay-bar-right mono">
              Step {index + 1} / {calls.length}
            </div>
          </div>

          <div className="replay-split">
            <div className="replay-pane">
              <div className="replay-pane-head">
                <span className="mono">Original · agent → tool</span>
                <span className="replay-tok mono">{origTok} tok</span>
              </div>
              <pre className="replay-code" key={`o-${index}`}>
                {origLines.map((line, i) => (
                  <span
                    key={i}
                    className="replay-line"
                    style={
                      reduced
                        ? undefined
                        : { animationDelay: `${Math.min(i * 12, 620)}ms` }
                    }
                  >
                    {line || " "}
                  </span>
                ))}
              </pre>
            </div>

            <div className="replay-pane replay-pane-int">
              <div className="replay-pane-head">
                <span className="mono">Delivered to the model</span>
                <span className="replay-tok mono">
                  {Math.round(animInt)} tok
                  {savedTok > 0 && (
                    <span className="replay-reduction"> −{savedPct}%</span>
                  )}
                </span>
              </div>
              <pre
                className={`replay-code replay-code-int${
                  reduced ? "" : " delayed"
                }`}
                key={`i-${index}`}
              >
                {intLines.map((line, i) => (
                  <span key={i} className={`replay-line ${classifyLine(line)}`}>
                    {line || " "}
                  </span>
                ))}
              </pre>
            </div>
          </div>

          <div className="replay-rail">
            <div className="replay-track" aria-hidden="true">
              <div className="replay-fill-orig" />
              <div
                className="replay-fill-kept"
                style={{ width: `${keptRatio * 100}%` }}
              />
            </div>
            <div className="replay-rail-meta mono">
              {savedTok > 0 ? (
                <span>
                  {origTok} → {intTok} output tokens ·{" "}
                  <span className="replay-reduction">
                    {savedTok} saved on this call
                  </span>
                </span>
              ) : (
                <span>
                  {origTok} → {intTok} output tokens ·{" "}
                  <span className="replay-flat">0 saved on this call</span>
                </span>
              )}
            </div>
          </div>

          <p className="replay-caption">{captionFor(call)}</p>

          <div className="replay-foot">
            <div className="replay-controls">
              <button
                className="replay-ctrl"
                onClick={restart}
                aria-label="Restart"
                title="Restart"
              >
                ⟲
              </button>
              <button
                className="replay-ctrl"
                onClick={() => {
                  setPlaying(false);
                  go(index - 1);
                }}
                disabled={atStart}
                aria-label="Previous step"
                title="Previous"
              >
                ◀
              </button>
              <button
                className="replay-ctrl replay-ctrl-play"
                onClick={() => {
                  if (atEnd) restart();
                  else setPlaying((p) => !p);
                }}
                aria-label={playing ? "Pause" : "Play"}
                title={playing ? "Pause" : "Play"}
              >
                {playing ? "❚❚" : "▶"}
              </button>
              <button
                className="replay-ctrl"
                onClick={() => {
                  setPlaying(false);
                  go(index + 1);
                }}
                disabled={atEnd}
                aria-label="Next step"
                title="Next"
              >
                ▶
              </button>
            </div>

            <div className="replay-cum">
              <span className="replay-cum-num">{animCum.toFixed(0)}%</span>
              <span className="replay-cum-lbl mono">
                tool output saved · session so far
              </span>
            </div>
          </div>
        </div>

        <p className="replay-fine mono">
          Illustrative replay from the smoke fixture. Token counts are estimates
          (⌈characters ÷ 4⌉), not model-tokenizer tokens. Loop-Whole reduces future
          tool responses; it does not rewrite existing model history.
        </p>
      </div>
    </section>
  );
}

