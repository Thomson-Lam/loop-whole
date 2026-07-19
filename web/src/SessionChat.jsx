import { useCallback, useEffect, useRef, useState } from "react";
import { sendMessage } from "./backboardApi";

const STORAGE_KEY = "loopwhole_bb_thread";
const ASSISTANT_KEY = "loopwhole_bb_assistant";

// Hardcoded for hackathon — in production this would come from env / config.
const API_KEY = "espr_0_lmpXLY5VCdBQj5kic7wRo4-TI2QMtAr9gv63_msHQ";

function getStored(key) {
  try {
    return localStorage.getItem(key) || null;
  } catch {
    return null;
  }
}

function setStored(key, value) {
  try {
    if (value) localStorage.setItem(key, value);
  } catch {
    /* noop */
  }
}

export default function SessionChat({ session }) {
  const [open, setOpen] = useState(false);
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [threadId, setThreadId] = useState(() => getStored(STORAGE_KEY));
  const [assistantId, setAssistantId] = useState(() =>
    getStored(ASSISTANT_KEY)
  );
  const scrollRef = useRef(null);

  // Auto-scroll to bottom when messages change.
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, loading]);

  const buildContext = useCallback(() => {
    if (!session) return "";
    const t = session.totals;
    const calls = session.toolCalls || [];
    return [
      `[Current session context]`,
      `Session: ${session.session?.id || "unknown"}`,
      `Tool calls: ${calls.length}`,
      `Tokens saved: ${t?.savedTokens ?? 0} (${t?.savingsPercent ?? 0}%)`,
      `Without LoopWhole: ${t?.withoutRuntimeTokens ?? 0} tokens`,
      `With LoopWhole: ${t?.withRuntimeTokens ?? 0} tokens`,
    ].join("\n");
  }, [session]);

  const handleSend = async () => {
    const text = input.trim();
    if (!text || loading) return;

    const userMsg = { role: "user", text };
    setMessages((prev) => [...prev, userMsg]);
    setInput("");
    setLoading(true);

    try {
      const contextPrefix = buildContext();
      const fullContent = contextPrefix
        ? `${contextPrefix}\n\nUser question: ${text}`
        : text;

      const res = await sendMessage(API_KEY, fullContent, {
        threadId,
        assistantId,
      });

      if (res.thread_id && res.thread_id !== threadId) {
        setThreadId(res.thread_id);
        setStored(STORAGE_KEY, res.thread_id);
      }
      if (res.assistant_id && res.assistant_id !== assistantId) {
        setAssistantId(res.assistant_id);
        setStored(ASSISTANT_KEY, res.assistant_id);
      }

      setMessages((prev) => [
        ...prev,
        { role: "assistant", text: res.content || "(no response)" },
      ]);
    } catch (err) {
      setMessages((prev) => [
        ...prev,
        { role: "error", text: `Error: ${err.message}` },
      ]);
    } finally {
      setLoading(false);
    }
  };

  const onKeyDown = (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const clearThread = () => {
    setMessages([]);
    setThreadId(null);
    setAssistantId(null);
    try {
      localStorage.removeItem(STORAGE_KEY);
      localStorage.removeItem(ASSISTANT_KEY);
    } catch {
      /* noop */
    }
  };

  return (
    <>
      {/* Floating trigger button */}
      <button
        className="chat-fab"
        onClick={() => setOpen((s) => !s)}
        title="Ask about sessions"
      >
        {open ? "✕" : "✦"}
      </button>

      {/* Chat panel */}
      {open && (
        <div className="chat-panel">
          <div className="chat-header">
            <div className="chat-header-left">
              <span className="chat-logo">✦</span>
              <span className="chat-title">Session Analyst</span>
              <span className="chat-badge">Backboard</span>
            </div>
            <button className="chat-clear" onClick={clearThread} title="New conversation">
              ↻
            </button>
          </div>

          <div className="chat-messages" ref={scrollRef}>
            {messages.length === 0 && (
              <div className="chat-empty">
                <p className="chat-empty-title">Ask me anything about your sessions</p>
                <div className="chat-suggestions">
                  {[
                    "How many tokens did my sessions save?",
                    "Which files were read the most?",
                    "Summarize my session history",
                  ].map((q) => (
                    <button
                      key={q}
                      className="chat-suggestion"
                      onClick={() => {
                        setInput(q);
                      }}
                    >
                      {q}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {messages.map((msg, i) => (
              <div key={i} className={`chat-msg chat-msg-${msg.role}`}>
                <div className="chat-msg-bubble">{msg.text}</div>
              </div>
            ))}

            {loading && (
              <div className="chat-msg chat-msg-assistant">
                <div className="chat-msg-bubble chat-typing">
                  <span className="dot" />
                  <span className="dot" />
                  <span className="dot" />
                </div>
              </div>
            )}
          </div>

          <div className="chat-input-bar">
            <input
              className="chat-input"
              type="text"
              placeholder="Ask about your sessions…"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={onKeyDown}
              disabled={loading}
              autoFocus
            />
            <button
              className="chat-send"
              onClick={handleSend}
              disabled={!input.trim() || loading}
            >
              ▶
            </button>
          </div>
        </div>
      )}
    </>
  );
}
