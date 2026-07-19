import { useEffect, useState } from "react";

const STATES = [
  {
    label: "Call",
    heading: "tool calling",
    title: "The agent uses its normal tools.",
    example: "read src/main.rs",
    without: "The agent calls the repository tool directly.",
    withLoopWhole: "The agent uses the same tool and workflow.",
  },
  {
    label: "Recall",
    heading: "context recall",
    title: "We intercept the call",
    example: "baseline: call #12",
    without: "Every response is handled as a new result.",
    withLoopWhole: "Exact delivered views become comparison baselines.",
  },
  {
    label: "Compare",
    heading: "state comparison",
    title: "We manage the agent’s context state and check for edge cases.",
    example: "unseen · unchanged · changed",
    without: "Repeated searches return the full output again.",
    withLoopWhole: "Prior context is classified before delivery.",
  },
  {
    label: "Return",
    heading: "safe return",
    title: "Agent gets context without full ingestion.",
    example: "FULL · NoC · DIFF",
    without: "Raw tool output goes straight back to the model.",
    withLoopWhole: "The call is intercepted and only the safe result is returned.",
  },
];

export default function ContextTimeline() {
  const [active, setActive] = useState(0);
  const [paused, setPaused] = useState(false);
  const reducedMotion = window.matchMedia(
    "(prefers-reduced-motion: reduce)"
  ).matches;
  const state = STATES[active];

  useEffect(() => {
    if (paused || reducedMotion) return;
    const timer = window.setInterval(
      () => setActive((current) => (current + 1) % STATES.length),
      2400
    );
    return () => window.clearInterval(timer);
  }, [paused, reducedMotion]);

  return (
    <div className="state-machine reveal">
      <div className="state-track" role="tablist" aria-label="Tool-call lifecycle">
        <div className="state-line" aria-hidden="true">
          <span style={{ width: `${(active / (STATES.length - 1)) * 100}%` }} />
        </div>

        {STATES.map((item, index) => (
          <button
            className={`state-node${active === index ? " active" : ""}`}
            key={item.label}
            type="button"
            role="tab"
            aria-selected={active === index}
            onClick={() => setActive(index)}
            onPointerEnter={() => {
              setActive(index);
              setPaused(true);
            }}
            onPointerLeave={() => setPaused(false)}
            onFocus={() => {
              setActive(index);
              setPaused(true);
            }}
            onBlur={() => setPaused(false)}
          >
            <span className="state-dot">{index + 1}</span>
            <span className="state-label mono">{item.label}</span>
          </button>
        ))}
      </div>

      <div className="state-copy" role="tabpanel">
        <span className="mono">
          {String(active + 1).padStart(2, "0")} - {state.heading}
        </span>
        <h3>{state.title}</h3>
        <code>{state.example}</code>

        <div className="state-comparison">
          <article>
            <span className="mono">Without Loop-Whole</span>
            <p>{state.without}</p>
          </article>
          <article className="with-loop-whole">
            <span className="mono">With Loop-Whole</span>
            <p>{state.withLoopWhole}</p>
          </article>
        </div>
      </div>
    </div>
  );
}
