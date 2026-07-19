import { useState } from "react";
import LineWaves from "./LineWaves";

const TOOL_ORDER = ["read", "edit", "write", "bash", "bash_edit"];

const TOOL_STORIES = {
  read: {
    label: "Read",
    icon: "▤",
    statement: "See only what changed.",
    steps: [
      {
        column: "context",
        title: "Read settings.js again",
        detail: "The agent asks for a file it saw earlier.",
      },
      {
        column: "check",
        title: "Check the saved view",
        detail: "Has this exact version already been delivered?",
      },
      {
        column: "check",
        title: "Nothing changed",
        detail: "The file still matches the saved baseline.",
        tone: "decision",
      },
      {
        column: "return",
        title: "NoC",
        detail: "1 token delivered instead of 84 lines.",
        comparison: "Normally: the full file returns again",
        tone: "result",
      },
    ],
  },
  edit: {
    label: "Edit",
    icon: "✎",
    statement: "Change one exact match.",
    steps: [
      {
        column: "context",
        title: "Change one line",
        detail: "Rename timeout to requestTimeout.",
      },
      {
        column: "check",
        title: "Find the exact text",
        detail: "Loop-Whole looks for one proven match.",
      },
      {
        column: "check",
        title: "One match found",
        detail: "Only that location is changed.",
        tone: "decision",
      },
      {
        column: "return",
        title: "Edit applied",
        detail: "The next read receives only the resulting diff.",
        comparison: "No whole-file rewrite",
        tone: "result",
      },
    ],
  },
  write: {
    label: "Write",
    icon: "＋",
    statement: "Create without overwriting.",
    steps: [
      {
        column: "context",
        title: "Create notes.md",
        detail: "The agent asks to create a new file.",
      },
      {
        column: "check",
        title: "Check the path",
        detail: "The file must stay inside the workspace.",
      },
      {
        column: "check",
        title: "Confirm it is new",
        detail: "Existing work is never silently replaced.",
        tone: "decision",
      },
      {
        column: "return",
        title: "File created",
        detail: "A short confirmation returns to the agent.",
        comparison: "Existing file? Refuse to overwrite",
        tone: "result",
      },
    ],
  },
  bash: {
    label: "Bash",
    icon: "»_",
    statement: "Run again; skip repeated noise.",
    steps: [
      {
        column: "context",
        title: "Run the tests again",
        detail: "The agent repeats the same command.",
      },
      {
        column: "check",
        title: "Execute the command",
        detail: "Bash always runs. Nothing is skipped.",
      },
      {
        column: "check",
        title: "Compare the result",
        detail: "The relevant test result has not changed.",
        tone: "decision",
      },
      {
        column: "return",
        title: "NoC",
        detail: "1 token delivered instead of the repeated test log.",
        comparison: "Normally: the noisy log returns again",
        tone: "result",
      },
    ],
  },
  bash_edit: {
    label: "Command edit",
    icon: "⌁",
    statement: "Reuse a saved command.",
    steps: [
      {
        column: "context",
        title: "Reuse command #7",
        detail: "The agent starts from a command it already ran.",
      },
      {
        column: "check",
        title: "Change one argument",
        detail: "Replace old_test with new_test.",
      },
      {
        column: "check",
        title: "Run the updated command",
        detail: "The edited command executes normally.",
        tone: "decision",
      },
      {
        column: "return",
        title: "Result + new command ID",
        detail: "The updated command can be reused again.",
        comparison: "No need to resend the full command",
        tone: "result",
      },
    ],
  },
};

const COLUMNS = [
  { id: "context", label: "Agent context" },
  { id: "check", label: "Loop-Whole" },
  { id: "return", label: "Delivered to agent" },
];

export default function ToolReplay() {
  const [tool, setTool] = useState("read");
  const [step, setStep] = useState(0);
  const story = TOOL_STORIES[tool];
  const atStart = step === 0;
  const atEnd = step === story.steps.length - 1;

  const selectTool = (nextTool) => {
    setTool(nextTool);
    setStep(0);
  };

  return (
    <section className="replay" id="replay" style={{ position: "relative" }}>
      <div
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          right: 0,
          height: "400px",
          zIndex: 0,
          opacity: 0.6,
          pointerEvents: "none",
        }}
      >
        <LineWaves
          speed={0.5}
          innerLineCount={21}
          outerLineCount={36}
          warpIntensity={0.5}
          rotation={-45}
          edgeFadeWidth={0}
          colorCycleSpeed={1}
          brightness={0.1}
          color1="#eaff00"
          color2="#8b8b93"
          color3="#c8da12"
          enableMouseInteraction={false}
          mouseInfluence={2}
        />
      </div>

      <div className="wrap" style={{ position: "relative", zIndex: 1 }}>
        <div className="section-head reveal in">
          <h2>Example</h2>
        </div>

        <div className="replay-tabs" role="tablist" aria-label="Tool demonstrations">
          {TOOL_ORDER.map((toolId) => {
            const item = TOOL_STORIES[toolId];
            const active = tool === toolId;
            return (
              <button
                key={toolId}
                type="button"
                role="tab"
                aria-selected={active}
                className={`replay-tab${active ? " active" : ""}`}
                onClick={() => selectTool(toolId)}
              >
                <span className="replay-tab-top">
                  <span className="replay-tab-icon mono" aria-hidden="true">
                    {item.icon}
                  </span>
                  <span className="replay-tab-name">{item.label}</span>
                </span>
                <span className="replay-tab-stmt">{item.statement}</span>
              </button>
            );
          })}
        </div>

        {tool === "edit" ? (
          <div className="replay-stage storyboard-placeholder" role="tabpanel">
            no changes made to edit tool :)
          </div>
        ) : (
          <div className="replay-stage storyboard" role="tabpanel">
          <div className="storyboard-head">
            <span className="mono">{story.label} walkthrough</span>
            <span className="mono">
              Step {step + 1} / {story.steps.length}
            </span>
          </div>

          <div className="storyboard-grid">
            {COLUMNS.map((column, columnIndex) => {
              const visibleSteps = story.steps
                .slice(0, step + 1)
                .map((item, index) => ({ ...item, index }))
                .filter((item) => item.column === column.id);

              return (
                <div className="storyboard-column-wrap" key={column.id}>
                  <div className={`storyboard-column ${column.id}`}>
                    <span className="storyboard-column-label mono">{column.label}</span>

                    {column.id === "context" && (
                      <div className="context-stack" aria-hidden="true">
                        <span>System instructions</span>
                        <span>Current task</span>
                      </div>
                    )}

                    <div className="storyboard-layers">
                      {visibleSteps.map((item) => (
                        <article
                          className={`storyboard-card ${item.tone || ""}${
                            item.index === step ? " is-new" : ""
                          }`}
                          key={`${tool}-${item.index}`}
                        >
                          <strong>{item.title}</strong>
                          <p>{item.detail}</p>
                          {item.comparison && <small>{item.comparison}</small>}
                        </article>
                      ))}

                      {visibleSteps.length === 0 && (
                        <div className="storyboard-empty">Next step</div>
                      )}
                    </div>
                  </div>

                  {columnIndex < COLUMNS.length - 1 && (
                    <span className="storyboard-arrow" aria-hidden="true">→</span>
                  )}
                </div>
              );
            })}
          </div>

          <div className="storyboard-foot">
            <div className="storyboard-progress" aria-label={`Step ${step + 1} of ${story.steps.length}`}>
              {story.steps.map((item, index) => (
                <span
                  className={index <= step ? "complete" : ""}
                  key={item.title}
                />
              ))}
            </div>

            <div className="storyboard-controls">
              <button
                className="btn btn-ghost"
                type="button"
                disabled={atStart}
                onClick={() => setStep((current) => Math.max(0, current - 1))}
              >
                ← Previous
              </button>
              <button
                className="btn btn-primary"
                type="button"
                onClick={() =>
                  setStep((current) =>
                    atEnd ? 0 : Math.min(story.steps.length - 1, current + 1)
                  )
                }
              >
                {atEnd ? `Restart ${story.label}` : `Next: ${story.steps[step + 1].title}`} →
              </button>
            </div>
          </div>
          </div>
        )}

        <p className="replay-fine mono">
          Token counts vary by tokenizer. Model history stays untouched.
        </p>

        <div className="cta-row">
          <a className="btn btn-primary" href="#/app">
            Launch →
          </a>
        </div>
      </div>
    </section>
  );
}
