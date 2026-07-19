const BACKBOARD_BASE = "https://app.backboard.io/api";

/**
 * Send a message to Backboard and return the complete response.
 * Memory is always enabled so the assistant accumulates session knowledge.
 */
export async function sendMessage(apiKey, content, opts = {}) {
  const res = await fetch(`${BACKBOARD_BASE}/threads/messages`, {
    method: "POST",
    headers: {
      "X-API-Key": apiKey,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      content,
      ...(opts.assistantId && { assistant_id: opts.assistantId }),
      ...(opts.threadId && { thread_id: opts.threadId }),
      memory: "Auto",
      stream: false,
      llm_provider: "google",
      model_name: "gemini-2.5-flash",
    }),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Backboard ${res.status}: ${text}`);
  }

  return res.json();
}

/**
 * List memories stored for a given assistant.
 */
export async function listMemories(apiKey, assistantId) {
  const res = await fetch(
    `${BACKBOARD_BASE}/assistants/${assistantId}/memories`,
    {
      headers: { "X-API-Key": apiKey },
    }
  );

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Backboard ${res.status}: ${text}`);
  }

  return res.json();
}
