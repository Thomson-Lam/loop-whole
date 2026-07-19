use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{logging::log_line, store::PersistedSession};

const BACKBOARD_BASE: &str = "https://app.backboard.io/api";

#[derive(Debug, Serialize)]
struct SendMessageRequest {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    assistant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_id: Option<String>,
    memory: String,
    stream: bool,
    llm_provider: String,
    model_name: String,
}

#[derive(Debug, Deserialize)]
pub struct BackboardResponse {
    pub content: Option<String>,
    pub thread_id: Option<String>,
    pub assistant_id: Option<String>,
}

/// Push a natural-language session summary to Backboard with persistent memory enabled.
/// Errors are logged but never propagated — this must not block server shutdown.
pub async fn push_session_summary(api_key: &str, session: &PersistedSession) -> Option<BackboardResponse> {
    let summary = format_session_summary(session);
    log_line(format!(
        "backboard: pushing session summary ({} chars) for session {}",
        summary.len(),
        session.session.id,
    ));

    let client = reqwest::Client::new();
    let request = SendMessageRequest {
        content: summary,
        assistant_id: None,
        thread_id: None,
        memory: "Auto".to_string(),
        stream: false,
        llm_provider: "google".to_string(),
        model_name: "gemini-2.5-flash".to_string(),
    };

    match client
        .post(format!("{BACKBOARD_BASE}/threads/messages"))
        .header("X-API-Key", api_key)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<BackboardResponse>().await {
                    Ok(body) => {
                        log_line(format!(
                            "backboard: session summary stored (assistant={}, thread={})",
                            body.assistant_id.as_deref().unwrap_or("auto"),
                            body.thread_id.as_deref().unwrap_or("auto"),
                        ));
                        Some(body)
                    }
                    Err(e) => {
                        log_line(format!("backboard: failed to parse response: {e:#}"));
                        None
                    }
                }
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                log_line(format!("backboard: API returned {status}: {body}"));
                None
            }
        }
        Err(e) => {
            log_line(format!("backboard: request failed: {e:#}"));
            None
        }
    }
}

fn format_session_summary(session: &PersistedSession) -> String {
    let meta = &session.session;
    let totals = &session.totals;
    let calls = &session.tool_calls;

    let duration_s = if meta.ended_at_ms > meta.started_at_ms {
        (meta.ended_at_ms - meta.started_at_ms) as f64 / 1000.0
    } else {
        0.0
    };

    // Count tool calls by type.
    let mut by_tool: HashMap<&str, usize> = HashMap::new();
    for call in calls {
        *by_tool.entry(call.tool_name.as_str()).or_default() += 1;
    }

    // Count delivery modes.
    let mut by_mode: HashMap<&str, usize> = HashMap::new();
    for call in calls {
        *by_mode.entry(call.delivery_mode.as_str()).or_default() += 1;
    }

    // Top 5 most-accessed files.
    let mut file_counts: HashMap<&str, usize> = HashMap::new();
    for call in calls {
        if let Some(ref path) = call.subject_path {
            *file_counts.entry(path.as_str()).or_default() += 1;
        }
    }
    let mut top_files: Vec<(&&str, &usize)> = file_counts.iter().collect();
    top_files.sort_by(|a, b| b.1.cmp(a.1));
    top_files.truncate(5);

    let mut summary = String::with_capacity(1024);
    summary.push_str(&format!(
        "LoopWhole MCP session completed. Please remember this session data for future questions.\n\n"
    ));
    summary.push_str(&format!("Session ID: {}\n", meta.id));
    summary.push_str(&format!("Workspace: {}\n", meta.workspace_root));
    summary.push_str(&format!("Duration: {duration_s:.1}s\n"));
    summary.push_str(&format!("Total tool calls: {}\n", calls.len()));

    if !by_tool.is_empty() {
        summary.push_str("Tool breakdown: ");
        let parts: Vec<String> = by_tool.iter().map(|(k, v)| format!("{k}={v}")).collect();
        summary.push_str(&parts.join(", "));
        summary.push('\n');
    }

    summary.push_str(&format!(
        "\nToken accounting:\n  Without LoopWhole: {} tokens\n  With LoopWhole: {} tokens\n  Saved: {} tokens ({:.1}%)\n",
        totals.without_runtime_tokens,
        totals.with_runtime_tokens,
        totals.saved_tokens,
        totals.savings_percent,
    ));

    if !by_mode.is_empty() {
        summary.push_str("\nDelivery modes: ");
        let parts: Vec<String> = by_mode.iter().map(|(k, v)| format!("{k}={v}")).collect();
        summary.push_str(&parts.join(", "));
        summary.push('\n');
    }

    if !top_files.is_empty() {
        summary.push_str("\nMost accessed files:\n");
        for (path, count) in &top_files {
            summary.push_str(&format!("  {path}: {count} accesses\n"));
        }
    }

    if let Some(ctx) = meta.context_window_tokens {
        summary.push_str(&format!(
            "\nContext window: {} tokens\n  Without LoopWhole: {:.2}%\n  With LoopWhole: {:.2}%\n",
            ctx,
            totals.without_runtime_context_percent.unwrap_or(0.0),
            totals.with_runtime_context_percent.unwrap_or(0.0),
        ));
    }

    summary
}

#[cfg(test)]
mod tests {
    use crate::schema::ToolPayload;
    use crate::store::{PersistedSession, PersistedSessionMeta, PersistedToolCall};
    use crate::schema::TokenTotals;

    use super::*;

    #[test]
    fn format_session_summary_covers_all_sections() {
        let session = PersistedSession {
            session: PersistedSessionMeta {
                id: "test-session".to_string(),
                started_at_ms: 1000,
                ended_at_ms: 11000,
                workspace_root: "/tmp/project".to_string(),
                context_window_tokens: Some(200_000),
                token_counter: "chars_div_4_v1".to_string(),
            },
            totals: TokenTotals {
                tool_input_tokens: 100,
                original_output_tokens: 5000,
                intercepted_output_tokens: 2000,
                without_runtime_tokens: 5100,
                with_runtime_tokens: 2100,
                saved_tokens: 3000,
                savings_percent: 58.82,
                without_runtime_context_percent: Some(2.55),
                with_runtime_context_percent: Some(1.05),
            },
            tool_calls: vec![
                PersistedToolCall {
                    id: 1,
                    sequence: 1,
                    occurred_at_ms: 2000,
                    tool_name: "read".to_string(),
                    subject_path: Some("src/main.rs".to_string()),
                    status: "success".to_string(),
                    duration_ms: 5,
                    delivery_mode: "full".to_string(),
                    decision_reason: None,
                    baseline_hash: None,
                    current_hash: None,
                    input: serde_json::json!({"path": "src/main.rs"}),
                    original: ToolPayload { text: "x".to_string(), bytes: 100, tokens: 25 },
                    intercepted: ToolPayload { text: "x".to_string(), bytes: 100, tokens: 25 },
                },
            ],
        };

        let text = format_session_summary(&session);
        assert!(text.contains("test-session"));
        assert!(text.contains("10.0s"));
        assert!(text.contains("src/main.rs"));
        assert!(text.contains("58.8"));
    }
}
